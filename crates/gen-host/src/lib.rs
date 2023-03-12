#![allow(clippy::must_use_candidate)]

use heck::{ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use std::{fmt::Write as _, mem};
use tauri_bindgen_core::{
    postprocess, uwrite, uwriteln, Files, InterfaceGenerator as _, Source, TypeInfo, Types,
    WorldGenerator,
};
use tauri_bindgen_gen_rust::{BorrowMode, FnSig, RustFlagsRepr, RustGenerator};
use wit_parser::{Docs, Flags, Function, Interface, Results, Type, TypeId};

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not `rustfmt` is executed to format generated code.
    #[cfg_attr(feature = "clap", clap(long))]
    pub rustfmt: bool,

    /// Whether or not to emit `tracing` macro calls on function entry/exit.
    #[cfg_attr(feature = "clap", clap(long))]
    pub tracing: bool,

    /// Whether or not to use async rust functions and traits.
    #[cfg_attr(feature = "clap", clap(long = "async"))]
    pub async_: bool,
}

impl Opts {
    pub fn build(self) -> Box<dyn WorldGenerator> {
        Box::new(Host {
            opts: self,
            ..Default::default()
        })
    }
}

#[derive(Debug, Default)]
struct Host {
    src: Source,
    opts: Opts,
    imports: Vec<String>,
}

impl WorldGenerator for Host {
    fn import(&mut self, name: &str, iface: &Interface, _files: &mut Files, world_hash: &str) {
        let mut gen = InterfaceGenerator::new(self, iface, BorrowMode::Owned);
        gen.types();
        gen.print_trait(name);
        gen.print_invoke_handler(name);

        let snake = name.to_snake_case();
        let module = &gen.src[..];

        uwriteln!(
            self.src,
            "#[allow(clippy::all, unused)]
            #[rustfmt::skip]
            pub mod {snake} {{
                use serde::Serialize;
                use serde::Deserialize;
                use tauri::InvokeError;
                use ::tauri_bindgen_host::tauri_bindgen_abi;
                use crate::web_bridge::DatabaseState;
                pub const WORLD_HASH: &str = \"{world_hash}\";
                {module}
            }}"
        );

        self.imports.push(snake);
    }

    fn finish(&mut self, name: &str, files: &mut Files, _world_hash: &str) {
        let mut src = mem::take(&mut self.src);
        if self.opts.rustfmt {
            postprocess(src.as_mut_string(), "rustfmt", ["--edition=2018"])
                .expect("failed to run `rustfmt`");
        }

        files.push(&format!("{name}.rs"), src.as_bytes());
    }
}

struct InterfaceGenerator<'a> {
    src: Source,
    gen: &'a mut Host,
    iface: &'a Interface,
    default_param_mode: BorrowMode,
    types: Types,
}

impl<'a> InterfaceGenerator<'a> {
    fn new(
        gen: &'a mut Host,
        iface: &'a Interface,
        default_param_mode: BorrowMode,
    ) -> InterfaceGenerator<'a> {
        let mut types = Types::default();
        types.analyze(iface);
        InterfaceGenerator {
            src: Source::default(),
            gen,
            iface,
            types,
            default_param_mode,
        }
    }

    // pub(crate) fn generate_invoke_handler(&mut self, name: &str) {
    //     self.print_trait(name);
    // }

    fn print_trait(&mut self, name: &str) {
        if self.gen.opts.async_ {
            uwriteln!(self.src, "#[::tauri_bindgen_host::async_trait]");
        }

        uwriteln!(
            self.src,
            "pub trait {}: Sized {{",
            name.to_upper_camel_case()
        );

        for func in &self.iface.functions {
            let fnsig = FnSig {
                async_: self.gen.opts.async_,
                private: true,
                self_arg: Some("&self".to_string()),
                ..Default::default()
            };

            self.print_docs_and_params(func, BorrowMode::Owned, &fnsig);
            self.push_str(" -> ");

            self.push_str("::tauri_bindgen_host::anyhow::Result<");
            self.print_result_ty(&func.results, BorrowMode::Owned);
            self.push_str(">;\n");
        }

        uwriteln!(self.src, "}}");
    }

    fn print_invoke_handler(&mut self, name: &str) {
        uwriteln!(
            self.src,
            "
                pub fn invoke_handler<U, R>(ctx: U) -> impl Fn(::tauri_bindgen_host::tauri::Invoke<R>)
                where
                    U: {} + Send + Sync + 'static,
                    R: ::tauri_bindgen_host::tauri::Runtime + 'static
                {{

                move |invoke| {{
            ",
            name.to_upper_camel_case()
        );

        if self.gen.opts.tracing {
            uwriteln!(
                self.src,
                r#"let span = ::tauri_bindgen_host::tracing::span!(
                    ::tauri_bindgen_host::tracing::Level::TRACE,
                    "tauri-bindgen invoke handler",
                    module = "{name}", function = invoke.message.command(), payload = ?invoke.message.payload()
                );
                let _enter = span.enter();
               "#
            );
        }

        uwriteln!(self.src, "match invoke.message.command() {{");

        for func in &self.iface.functions {
            self.print_handler_match(func);
        }

        uwriteln!(self.src, "_ => todo!(),");

        uwriteln!(self.src, "}}");
        uwriteln!(self.src, "}}");
        uwriteln!(self.src, "}}");
    }

    fn print_handler_match(&mut self, func: &Function) {
        uwrite!(self.src, "\"{}\" => {{", &func.name);

        // extract message and resolver
        uwriteln!(
            self.src,
            "
            #[allow(unused_variables)]
            let ::tauri_bindgen_host::tauri::Invoke {{
                message: __tauri_message__,
                resolver: __tauri_resolver__,
            }} = invoke;"
        );

        self.print_param_struct(func);

        // decode param from message
        uwriteln!(
            self.src,
            r#"
            let message: String = ::tauri_bindgen_host::tauri::command::CommandArg::from_command(::tauri_bindgen_host::tauri::command::CommandItem {{
                name: "{}",
                key: "encoded",
                message: &__tauri_message__,
            }}).unwrap();
            let message = ::tauri_bindgen_host::decode_base64(&message);
            let message = String::from_utf8(message.clone()).unwrap();  
            let params : Params = serde_json::from_str(&message).unwrap();
        "#,
            func.name
        );

        // call method
        uwriteln!(self.src, "let result = ctx.{}(", func.name.to_snake_case());

        for (param, _) in &func.params {
            self.src.push_str("params.");
            self.src.push_str(&param.to_snake_case());
            self.src.push_str(", ");
        }

        uwriteln!(
            self.src,
            r#"
            match ::tauri::command::CommandArg::from_command(
                ::tauri::command::CommandItem {{
                    name: "{}",
                    key: "state",
                    message: &__tauri_message__,
                }},
            ) {{
                Ok(arg) => arg,
                Err(err) => return __tauri_resolver__.invoke_error(err),
            }},);"#,
            func.name.to_snake_case()
        );

        // serialize and encode result
        uwriteln!(self.src, "
            __tauri_resolver__.respond(result
                .map(|ref val| ::tauri_bindgen_host::encode_base64(serde_json::to_string(val).unwrap().as_bytes()))
                .map_err(|ref err| InvokeError::from(serde_json::to_string(&err.to_string()).unwrap()))
            );");

        uwriteln!(self.src, "}},");
    }

    fn print_param_struct(&mut self, func: &Function) {
        // let lifetime = func.params.iter().any(|(_, ty)| self.needs_lifetime(ty));

        self.push_str("#[derive(tauri_bindgen_abi::Readable, Serialize, Deserialize)]\n");
        // self.push_str("#[serde(rename_all = \"camelCase\")]\n");
        self.src.push_str("struct Params");
        // self.print_generics(lifetime.then(|| "'a"));
        self.src.push_str(" {\n");

        for (param, ty) in &func.params {
            self.src.push_str(&param.to_snake_case());
            self.src.push_str(" : ");
            self.print_ty(ty, BorrowMode::Owned);
            self.push_str(",\n");
        }

        self.src.push_str("}\n");
    }

    //     for func in &self.iface.functions {

    //         for (param, _) in &func.params {
    //             let func_name = &func.name;

    //             uwriteln!(
    //                 self.src,
    //                 r#"let {snake_param} = match ::tauri_bindgen_host::tauri::command::CommandArg::from_command(::tauri_bindgen_host::tauri::command::CommandItem {{
    //                     name: "{func_name}",
    //                     key: "{camel_param}",
    //                     message: &__tauri_message__,
    //                 }}) {{
    //                     Ok(arg) => arg,
    //                     Err(err) => {{"#,
    //                 snake_param = param.to_snake_case(),
    //                 camel_param = param.to_lower_camel_case()
    //             );

    //             uwriteln!(
    //                 self.src,
    //                 r#"return __tauri_resolver__.invoke_error(err);
    //                     }},
    //                 }};
    //                 "#
    //             );
    //         }

    //         if self.gen.opts.async_ {
    //             uwriteln!(
    //                 self.src,
    //                 "
    //             __tauri_resolver__
    //             .respond_async(async move {{
    //             "
    //             );
    //         }

    //         uwriteln!(self.src, "let result = ctx.{}(", func.name.to_snake_case());

    //         for (param, _) in &func.params {
    //             self.src.push_str(&param.to_snake_case());
    //             self.src.push_str(", ");
    //         }

    //         uwriteln!(self.src, ");");

    //         if self.gen.opts.async_ {
    //             uwriteln!(
    //                 self.src,
    //                 "
    //                 result.await.map_err(::tauri_bindgen_host::tauri::InvokeError::from_anyhow)
    //                 }});
    //             "
    //             );
    //         } else {
    //             uwriteln!(self.src, "
    //                 __tauri_resolver__.respond(result.map_err(::tauri_bindgen_host::tauri::InvokeError::from_anyhow));
    //             ");
    //         }

    //         uwriteln!(self.src, "}},");
    //     }

    //     uwriteln!(self.src, "func_name => {{");
    //     if self.gen.opts.tracing {
    //         uwriteln!(
    //             self.src,
    //             r#"::tauri_bindgen_host::tracing::error!(module = "{name}", function = func_name, "Not Found");"#
    //         );
    //     }
    //     uwriteln!(self.src, "invoke.resolver.reject(\"Not Found\");");
    //     uwriteln!(self.src, "}}");
    //     uwriteln!(self.src, "}}");
    //     uwriteln!(self.src, "}}");

    //     uwriteln!(self.src, "}}");
    // }

    fn print_result_ty(&mut self, results: &Results, mode: BorrowMode) {
        match results {
            Results::Named(rs) => match rs.len() {
                0 => self.push_str("()"),
                1 => self.print_ty(&rs[0].1, mode),
                _ => {
                    self.push_str("(");
                    for (i, (_, ty)) in rs.iter().enumerate() {
                        if i > 0 {
                            self.push_str(", ");
                        }
                        self.print_ty(ty, mode);
                    }
                    self.push_str(")");
                }
            },
            Results::Anon(ty) => self.print_ty(ty, mode),
        }
    }
}

impl<'a> RustGenerator<'a> for InterfaceGenerator<'a> {
    fn iface(&self) -> &'a Interface {
        self.iface
    }

    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    fn print_borrowed_str(&mut self, lifetime: &'static str) {
        self.push_str("&");
        if lifetime != "'_" {
            self.push_str(lifetime);
            self.push_str(" ");
        }
        self.push_str(" str");
    }

    fn default_param_mode(&self) -> BorrowMode {
        self.default_param_mode
    }

    fn info(&self, ty: TypeId) -> TypeInfo {
        self.types.get(ty)
    }
}

impl<'a> tauri_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn iface(&self) -> &'a Interface {
        self.iface
    }

    fn type_record(&mut self, id: TypeId, _name: &str, record: &wit_parser::Record, docs: &Docs) {
        self.print_typedef_record(id, record, docs, get_serde_attrs);
    }

    fn type_flags(&mut self, id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.push_str("::tauri_bindgen_host::bitflags::bitflags! {\n");
        self.print_rustdoc(docs);

        let repr = RustFlagsRepr::new(flags);
        let info = self.info(id);

        if let Some(attrs) = get_serde_attrs(name, self.uses_two_names(&info), info) {
            self.push_str(&attrs);
        }

        self.push_str(&format!(
            "pub struct {}: {} {{\n",
            name.to_upper_camel_case(),
            repr
        ));

        for (i, flag) in flags.flags.iter().enumerate() {
            self.print_rustdoc(&flag.docs);
            self.src.push_str(&format!(
                "const {} = 1 << {};\n",
                flag.name.to_shouty_snake_case(),
                i,
            ));
        }

        self.push_str("}\n}\n");
    }

    fn type_variant(
        &mut self,
        id: TypeId,
        _name: &str,
        variant: &wit_parser::Variant,
        docs: &Docs,
    ) {
        self.print_typedef_variant(id, variant, docs, get_serde_attrs);
    }

    fn type_union(&mut self, id: TypeId, _name: &str, union: &wit_parser::Union, docs: &Docs) {
        self.print_typedef_union(id, union, docs, get_serde_attrs);
    }

    fn type_enum(&mut self, id: TypeId, _name: &str, enum_: &wit_parser::Enum, docs: &Docs) {
        self.print_typedef_enum(id, enum_, docs, get_serde_attrs);
    }

    fn type_alias(&mut self, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        self.print_typedef_alias(id, ty, docs);
    }
}

#[allow(clippy::unnecessary_wraps)]
fn get_serde_attrs(name: &str, uses_two_names: bool, info: TypeInfo) -> Option<String> {
    let mut attrs = vec![];

    if uses_two_names {
        if name.ends_with("Param") {
            attrs.push("tauri_bindgen_abi::Readable");
        } else if name.ends_with("Result") {
            attrs.push("tauri_bindgen_abi::Writable");
        }
    } else {
        if info.contains(TypeInfo::PARAM) {
            attrs.push("tauri_bindgen_abi::Readable");
        }
        if info.contains(TypeInfo::RESULT) {
            attrs.push("tauri_bindgen_abi::Writable");
        }
    }

    Some(format!(
        "#[derive({}, Serialize, Deserialize)]\n",
        attrs.join(", ")
    ))
}
