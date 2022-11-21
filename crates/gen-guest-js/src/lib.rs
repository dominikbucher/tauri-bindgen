use bindgen_core::{uwriteln, Source, WorldGenerator};
use heck::*;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use wit_parser::*;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not `prettier` is executed to format generated code.
    #[cfg_attr(feature = "clap", arg(long))]
    pub prettier: bool,
    /// Names of functions to skip generating bindings for.
    #[cfg_attr(feature = "clap", arg(long))]
    pub skip: Vec<String>,
}

impl Opts {
    pub fn build(self) -> Box<dyn WorldGenerator> {
        let mut r = JavaScript::default();
        r.skip = self.skip.iter().cloned().collect();
        r.opts = self;
        Box::new(r)
    }
}

#[derive(Debug, Default)]
struct JavaScript {
    src: Source,
    opts: Opts,
    skip: HashSet<String>,
}

impl WorldGenerator for JavaScript {
    fn import(
        &mut self,
        name: &str,
        iface: &wit_parser::Interface,
        _files: &mut bindgen_core::Files,
    ) {
        let mut gen = InterfaceGenerator::new(self, iface);

        for func in iface.functions.iter() {
            gen.generate_guest_import(name, func);
        }

        let module = &gen.src[..];
        uwriteln!(self.src, "{module}");
    }

    fn finish(&mut self, name: &str, files: &mut bindgen_core::Files) {
        let mut src = mem::take(&mut self.src);
        if self.opts.prettier {
            let mut child = Command::new("prettier")
                .arg(format!("--parser=babel"))
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("failed to spawn `prettier`");
            child
                .stdin
                .take()
                .unwrap()
                .write_all(src.as_bytes())
                .unwrap();
            src.as_mut_string().truncate(0);
            child
                .stdout
                .take()
                .unwrap()
                .read_to_string(src.as_mut_string())
                .unwrap();
            let status = child.wait().unwrap();
            assert!(status.success());
        }

        files.push(&format!("{name}.js"), src.as_bytes());
    }
}

struct InterfaceGenerator<'a> {
    src: Source,
    gen: &'a mut JavaScript,
    iface: &'a Interface,
}

impl<'a> InterfaceGenerator<'a> {
    pub fn new(gen: &'a mut JavaScript, iface: &'a Interface,) -> Self {
        Self {
            src: Source::default(),
            gen,
            iface
        }
    }

    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    fn print_jsdoc(&mut self, func: &Function) {
        let docs = match &func.docs.contents {
            Some(docs) => docs,
            None => return,
        };
        self.push_str("/**\n");
        
        for line in docs.trim().lines() {
            self.push_str(&format!(" * {}\n", line));
        }

        for (param, ty) in func.params.iter() {
            self.push_str(" * @param {");
            self.print_ty(ty);
            self.push_str("} ");
            self.push_str(param);
            self.push_str("\n");
        }

        match func.results.len() {
            0 => {},
            1 => {
                self.push_str(" * @returns {Promise<");
                self.print_ty(func.results.iter_types().next().unwrap());
                self.push_str(">}\n");
            },
            _ => {
                self.push_str(" * @returns {Promise<[");
                for (i, ty) in func.results.iter_types().enumerate() {
                    if i != 0 {
                        self.push_str(", ");
                    }
                    self.print_ty(ty);
                }
                self.push_str("]>}\n");
            }
        }

        self.push_str(" */\n");
    }

    fn generate_guest_import(&mut self, mod_name: &str, func: &Function) {
        if self.gen.skip.contains(&func.name) {
            return;
        }

        self.print_jsdoc(func);

        self.push_str("export async function ");
        self.push_str(&func.item_name().to_lower_camel_case());
        self.push_str("(");

        let param_start = match &func.kind {
            FunctionKind::Freestanding => 0,
        };

        for (i, (name, _)) in func.params[param_start..].iter().enumerate() {
            if i > 0 {
                self.push_str(", ");
            }
            self.push_str(to_js_ident(&name.to_lower_camel_case()));
        }
        self.push_str(") {\n");

        if func.results.len() > 0 {
            self.push_str("const result = ");
        }

        self.push_str(&format!(
            "await window.__TAURI__.tauri.invoke(\"plugin:{}|{}\",",
            mod_name.to_snake_case(),
            func.name.to_snake_case()
        ));

        if !func.params.is_empty() {
            self.push_str("{");
            for (i, (name, _ty)) in func.params.iter().enumerate() {
                if i > 0 {
                    self.push_str(", ");
                }
                self.push_str(&name.to_lower_camel_case());
                self.push_str(": ");
                self.push_str(to_js_ident(&name.to_lower_camel_case()));
            }
            self.push_str("}");
        }

        self.push_str(");\n");

        if func.results.len() > 0 {
            self.push_str("return result\n");
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
            Type::Id(id) => {
                let ty = &self.iface.types[*id];
                if let Some(name) = &ty.name {
                    return self.push_str(&name.to_upper_camel_case());
                }
                match &ty.kind {
                    TypeDefKind::Record(_) => todo!(),
                    TypeDefKind::Flags(_) => todo!(),
                    TypeDefKind::Tuple(ty) => self.print_tuple(ty),
                    TypeDefKind::Variant(_) => todo!(),
                    TypeDefKind::Enum(_) => todo!(),
                    TypeDefKind::Option(_) => todo!(),
                    TypeDefKind::Result(_) => todo!(),
                    TypeDefKind::Union(_) => todo!(),
                    TypeDefKind::List(ty) => self.print_list(ty),
                    TypeDefKind::Future(_) => todo!(),
                    TypeDefKind::Stream(_) => todo!(),
                    TypeDefKind::Type(ty) => self.print_ty(ty),
                }
            }
        }
    }

    fn print_tuple(&mut self, tuple: &Tuple) {
        self.push_str("[");
        for (i, ty) in tuple.types.iter().enumerate() {
            if i > 0 {
                self.push_str(", ");
            }
            self.print_ty(ty);
        }
        self.push_str("]");
    }

    fn print_list(&mut self, ty: &Type) {
        match self.array_ty(self.iface, ty) {
            Some(ty) => self.push_str(ty),
            None => {
                self.print_ty(ty);
                self.push_str("[]");
            }
        }
    }

    fn array_ty(&self, iface: &Interface, ty: &Type) -> Option<&'static str> {
        match ty {
            Type::Bool => None,
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
            Type::Char => None,
            Type::String => None,
            Type::Id(id) => match &iface.types[*id].kind {
                TypeDefKind::Type(t) => self.array_ty(iface, t),
                _ => None,
            },
        }
    }
}

fn to_js_ident(name: &str) -> &str {
    match name {
        "in" => "in_",
        "import" => "import_",
        s => s,
    }
}
