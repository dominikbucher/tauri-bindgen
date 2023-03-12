#![allow(clippy::must_use_candidate)]

use heck::{ToLowerCamelCase, ToUpperCamelCase};
use std::fmt::Write as _;
use std::mem;
use tauri_bindgen_core::{
    postprocess, uwriteln, Files, InterfaceGenerator as _, Source, WorldGenerator,
};
use wit_parser::{Docs, Flags, Function, Int, Interface, Type, TypeDefKind, TypeId};

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", clap(group(
    clap::ArgGroup::new("fmt")
        .args(&["prettier", "romefmt"]),
)))]
pub struct Opts {
    /// Run `prettier` to format the generated code. This requires a global installation of `prettier`.
    #[cfg_attr(feature = "clap", clap(long))]
    pub prettier: bool,
    /// Run `rome format` to format the generated code. This formatter is much faster that `prettier`. Requires a global installation of `prettier`.
    #[cfg_attr(feature = "clap", clap(long))]
    pub romefmt: bool,
}

impl Opts {
    pub fn build(self) -> Box<dyn WorldGenerator> {
        Box::new(TypeScript {
            opts: self,
            ..Default::default()
        })
    }
}

#[derive(Debug, Default)]
struct TypeScript {
    src: Source,
    opts: Opts,
}

impl WorldGenerator for TypeScript {
    fn import(
        &mut self,
        _name: &str,
        iface: &wit_parser::Interface,
        _files: &mut Files,
        world_hash: &str,
    ) {
        let mut gen = InterfaceGenerator::new(iface, world_hash);

        gen.print_intro();
        gen.types();

        for func in &iface.functions {
            gen.generate_guest_import(func);
        }

        gen.print_outro();

        let module = &gen.src[..];
        uwriteln!(self.src, "{module}");

        // files.push(&format!("{name}.ts"), gen.src.as_bytes());

        // uwriteln!(
        //     self.src.ts,
        //     "{} {{ {camel} }} from './{name}';",
        //     // In instance mode, we have no way to assert the imported types
        //     // in the ambient declaration file. Instead we just export the
        //     // import namespace types for users to use.
        //     "export"
        // );

        // uwriteln!(self.import_object, "export const {name}: typeof {camel};");
    }

    fn finish(&mut self, name: &str, files: &mut Files, _world_hash: &str) {
        let mut src = mem::take(&mut self.src);
        if self.opts.prettier {
            postprocess(src.as_mut_string(), "prettier", ["--parser=typescript"])
                .expect("failed to run `rome format`");
        } else if self.opts.romefmt {
            postprocess(
                src.as_mut_string(),
                "rome",
                ["format", "--stdin-file-path", "index.ts"],
            )
            .expect("failed to run `rome format`");
        }

        files.push(&format!("{name}.ts"), src.as_bytes());
    }
}

struct InterfaceGenerator<'a> {
    src: Source,
    iface: &'a Interface,
    needs_ty_option: bool,
    needs_ty_result: bool,
    world_hash: &'a str,
}

impl<'a> InterfaceGenerator<'a> {
    pub fn new(iface: &'a Interface, world_hash: &'a str) -> Self {
        Self {
            src: Source::default(),
            iface,
            needs_ty_option: false,
            needs_ty_result: false,
            world_hash,
        }
    }

    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    fn print_typedoc(&mut self, docs: &Docs) {
        if !docs.contents.is_empty() {
            self.push_str("/**\n");
            for line in docs.contents.trim().lines() {
                self.push_str(&format!(" * {line}\n"));
            }
            self.push_str(" */\n");
        }
    }

    fn print_intro(&mut self) {
        self.push_str(
            "
        // declare global {
        //     interface Window {
        //         __TAURI_INVOKE__<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
        //     }
        // }
        // const invoke = window.__TAURI_INVOKE__;

        import * as _ from \"lodash\";

        function camelCaseDeep(anything) {
            const thing = _.cloneDeep(anything);
          
            if (
              _.isEmpty(thing) ||
              (!_.isObject(thing) && !_.isArray(thing))
            ) {
              return thing;
            }
          
            if (_.isArray(thing)) {
              const arr = thing;
              return arr.map(el => camelCaseDeep(el))
            }
          
            // thing can be only not empty object here
            const objWithMappedKeys = _.mapKeys(thing, (value, key) => _.camelCase(key));
            const objWithMappedValues = _.mapValues(objWithMappedKeys, value => camelCaseDeep(value));
          
            return objWithMappedValues;
        }

        import { invoke } from \"@tauri-apps/api/tauri\";
        import { encode, decode } from \"js-base64\";
        ",
        );
    }

    fn print_outro(&mut self) {
        if mem::take(&mut self.needs_ty_option) {
            self.push_str("export type Option<T> = { tag: 'none' } | { tag: 'some', val; T };\n");
        }
        if mem::take(&mut self.needs_ty_result) {
            self.push_str(
                "export type Result<T, E> = { tag: 'ok', val: T } | { tag: 'err', val: E };\n",
            );
        }
    }

    fn generate_guest_import(&mut self, func: &Function) {
        self.print_typedoc(&func.docs);

        self.push_str("export async function ");
        self.push_str(&func.name.to_lower_camel_case());
        self.push_str("(");

        for (i, (name, ty)) in func.params.iter().enumerate() {
            if i > 0 {
                self.push_str(", ");
            }
            self.push_str(to_js_ident(&name.to_lower_camel_case()));
            self.push_str(": ");
            self.print_ty(ty);
        }
        self.push_str("): Promise<");

        if let Some((ok_ty, _)) = func.results.throws() {
            self.print_optional_ty(ok_ty);
        } else {
            match func.results.len() {
                0 => self.push_str("void"),
                1 => self.print_ty(func.results.types().next().unwrap()),
                _ => {
                    self.push_str("[");
                    for (i, ty) in func.results.types().enumerate() {
                        if i != 0 {
                            self.push_str(", ");
                        }
                        self.print_ty(ty);
                    }
                    self.push_str("]");
                }
            }
        }
        self.push_str("> {\n");

        if !func.results.is_empty() {
            self.push_str("const result = ");
        }

        self.push_str("await invoke<string>(");

        // if let Some((ok_ty, _)) = func.results.throws() {
        //     self.print_optional_ty(ok_ty);
        // } else {
        //     match func.results.len() {
        //         0 => self.push_str("void"),
        //         1 => self.print_ty(func.results.types().next().unwrap()),
        //         _ => {
        //             self.push_str("[");
        //             for (i, ty) in func.results.types().enumerate() {
        //                 if i != 0 {
        //                     self.push_str(", ");
        //                 }
        //                 self.print_ty(ty);
        //             }
        //             self.push_str("]");
        //         }
        //     }
        // }

        // self.push_str(">(");

        self.push_str(&format!("\"plugin:{}|{}\",", self.world_hash, func.name));

        self.push_str("{encoded: encode(JSON.stringify({");
        if !func.params.is_empty() {
            for (i, (name, _ty)) in func.params.iter().enumerate() {
                if i > 0 {
                    self.push_str(", ");
                }
                self.push_str(&name.to_lower_camel_case());
                self.push_str(": ");
                self.push_str(to_js_ident(&name.to_lower_camel_case()));
            }
        }
        self.push_str("}), true)}");

        self.push_str(");\n");

        if !func.results.is_empty() {
            self.push_str("return camelCaseDeep(JSON.parse(decode(result)))\n");
        }

        self.push_str("}\n");
    }

    fn print_ty(&mut self, ty: &Type) {
        match ty {
            Type::Bool => self.push_str("boolean"),
            Type::U8
            | Type::U16
            | Type::U32
            | Type::S8
            | Type::S16
            | Type::S32
            | Type::Float32
            | Type::Float64 => self.push_str("number"),
            Type::U64 | Type::S64 => self.push_str("bigint"),
            Type::Char | Type::String => self.push_str("string"),
            Type::Tuple(tys) => self.print_tuple(tys),
            Type::List(ty) => self.print_list(ty),
            Type::Option(ty) => {
                if self.is_nullable(ty) {
                    self.needs_ty_option = true;
                    self.push_str("Option<");
                    self.print_ty(ty);
                    self.push_str(">");
                } else {
                    self.print_ty(ty);
                    self.push_str(" | null");
                }
            }
            Type::Result(r) => {
                self.needs_ty_result = true;
                self.push_str("Result<");
                self.print_optional_ty(r.ok.as_ref());
                self.push_str(", ");
                self.print_optional_ty(r.err.as_ref());
                self.push_str(">");
            }
            Type::Id(id) => {
                let ty = &self.iface.types[*id];

                self.push_str(&ty.name.to_upper_camel_case());
            }
        }
    }

    fn print_optional_ty(&mut self, ty: Option<&Type>) {
        match ty {
            Some(ty) => self.print_ty(ty),
            None => self.push_str("void"),
        }
    }

    fn print_tuple(&mut self, types: &[Type]) {
        self.push_str("[");
        for (i, ty) in types.iter().enumerate() {
            if i > 0 {
                self.push_str(", ");
            }
            self.print_ty(ty);
        }
        self.push_str("]");
    }

    fn print_list(&mut self, ty: &Type) {
        if let Some(ty) = array_ty(self.iface, ty) {
            self.push_str(ty);
        } else {
            self.print_ty(ty);
            self.push_str("[]");
        }
    }

    fn is_nullable(&self, ty: &Type) -> bool {
        let id = match ty {
            Type::Id(id) => *id,
            _ => return false,
        };
        match &self.iface.types[id].kind {
            // If `ty` points to an `option<T>`, then `ty` can be represented
            // with `null` if `t` itself can't be represented with null. For
            // example `option<option<u32>>` can't be represented with `null`
            // since that's ambiguous if it's `none` or `some(none)`.
            //
            // Note, oddly enough, that `option<option<option<u32>>>` can be
            // represented as `null` since:
            //
            // * `null` => `none`
            // * `{ tag: "none" }` => `some(none)`
            // * `{ tag: "some", val: null }` => `some(some(none))`
            // * `{ tag: "some", val: 1 }` => `some(some(some(1)))`
            //
            // It's doubtful anyone would actually rely on that though due to
            // how confusing it is.
            // TypeDefKind::Option(ty) => !self.is_nullable(ty),
            TypeDefKind::Alias(t) => self.is_nullable(t),
            _ => false,
        }
    }
}

impl<'a> tauri_bindgen_core::InterfaceGenerator<'a> for InterfaceGenerator<'a> {
    fn iface(&self) -> &'a wit_parser::Interface {
        self.iface
    }

    fn type_record(
        &mut self,
        _id: wit_parser::TypeId,
        name: &str,
        record: &wit_parser::Record,
        docs: &wit_parser::Docs,
    ) {
        self.print_typedoc(docs);
        self.push_str(&format!(
            "export interface {} {{\n",
            name.to_upper_camel_case()
        ));

        for field in &record.fields {
            self.print_typedoc(&field.docs);
            self.push_str(&field.name.to_lower_camel_case());
            if self.is_nullable(&field.ty) {
                self.push_str("?");
            }
            self.push_str(": ");
            self.print_ty(&field.ty);

            self.push_str(",\n");
        }

        self.push_str("};\n");
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.print_typedoc(docs);

        match flags.repr() {
            Int::U8 | Int::U16 => {
                self.push_str(&format!("export enum {} {{\n", name.to_upper_camel_case()));
            }
            Int::U32 | Int::U64 => {
                self.push_str(&format!(
                    "export type {} = typeof {};",
                    name.to_upper_camel_case(),
                    name.to_upper_camel_case()
                ));
                self.push_str(&format!(
                    "export const {} = {{\n",
                    name.to_upper_camel_case()
                ));
            }
        }

        let base: usize = 1;
        for (i, flag) in flags.flags.iter().enumerate() {
            self.print_typedoc(&flag.docs);

            match flags.repr() {
                Int::U8 | Int::U16 => self.push_str(&format!(
                    "{} = {},\n",
                    flag.name.to_upper_camel_case(),
                    base << i
                )),
                Int::U32 | Int::U64 => self.push_str(&format!(
                    "{}: BigInt({}),\n",
                    flag.name.to_upper_camel_case(),
                    base << i
                )),
            }
        }

        self.push_str("}\n");
    }

    fn type_variant(
        &mut self,
        _id: wit_parser::TypeId,
        name: &str,
        variant: &wit_parser::Variant,
        docs: &wit_parser::Docs,
    ) {
        self.print_typedoc(docs);
        self.push_str(&format!("export type {} = ", name.to_upper_camel_case()));
        for (i, case) in variant.cases.iter().enumerate() {
            if i > 0 {
                self.push_str("| ");
            }
            self.push_str(&format!("{name}_{}", case.name).to_upper_camel_case());
        }
        self.push_str(";\n");

        for case in &variant.cases {
            self.print_typedoc(&case.docs);
            self.push_str(&format!(
                "export interface {} {{\n",
                format!("{name}_{}", case.name).to_upper_camel_case()
            ));

            self.push_str("tag: '");
            self.push_str(&case.name);
            self.push_str("',\n");

            if let Some(ty) = &case.ty {
                self.push_str("val: ");
                self.print_ty(ty);
                self.push_str(",\n");
            }
            self.push_str("}\n");
        }
    }

    fn type_union(
        &mut self,
        _id: wit_parser::TypeId,
        name: &str,
        union: &wit_parser::Union,
        docs: &wit_parser::Docs,
    ) {
        self.print_typedoc(docs);
        let name = name.to_upper_camel_case();
        self.push_str(&format!("export type {name} = "));
        for i in 0..union.cases.len() {
            if i > 0 {
                self.push_str(" | ");
            }
            self.push_str(&format!("{name}{i}"));
        }
        self.push_str(";\n");
        for (i, case) in union.cases.iter().enumerate() {
            self.print_typedoc(&case.docs);
            self.push_str(&format!("export interface {name}{i} {{\n"));
            self.push_str(&format!("tag: {i},\n"));
            self.push_str("val: ");
            self.print_ty(&case.ty);
            self.push_str(",\n");
            self.push_str("}\n");
        }
    }

    fn type_enum(
        &mut self,
        _id: wit_parser::TypeId,
        name: &str,
        enum_: &wit_parser::Enum,
        docs: &wit_parser::Docs,
    ) {
        self.print_typedoc(docs);

        self.push_str(&format!("export type {} = ", name.to_upper_camel_case()));
        for (i, case) in enum_.cases.iter().enumerate() {
            if i != 0 {
                self.push_str(" | ");
            }
            self.push_str(&format!("'{}'", case.name));
        }

        self.push_str(";\n");
    }

    fn type_alias(
        &mut self,
        _id: wit_parser::TypeId,
        name: &str,
        ty: &wit_parser::Type,
        docs: &wit_parser::Docs,
    ) {
        self.print_typedoc(docs);
        self.push_str(&format!("export type {} = ", name.to_upper_camel_case()));
        self.print_ty(ty);
        self.push_str(";\n");
    }
}

fn to_js_ident(name: &str) -> &str {
    match name {
        "in" => "in_",
        "import" => "import_",
        s => s,
    }
}

fn array_ty(iface: &Interface, ty: &Type) -> Option<&'static str> {
    match ty {
        Type::U8 => Some("Uint8Array"),
        Type::S8 => Some("Int8Array"),
        Type::U16 => Some("Uint16Array"),
        Type::S16 => Some("Int16Array"),
        Type::U32 => Some("Uint32Array"),
        Type::S32 => Some("Int32Array"),
        Type::U64 => Some("BigUint64Array"),
        Type::S64 => Some("BigInt64Array"),
        Type::Float32 => Some("Float32Array"),
        Type::Float64 => Some("Float64Array"),
        Type::Id(id) => match &iface.types[*id].kind {
            TypeDefKind::Alias(t) => array_ty(iface, t),
            _ => None,
        },
        Type::Bool
        | Type::Tuple(_)
        | Type::List(_)
        | Type::Option(_)
        | Type::Result(_)
        | Type::Char
        | Type::String => None,
    }
}
