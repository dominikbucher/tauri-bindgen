#[allow(clippy::all)]
pub mod imports {
    pub trait Imports: Sized {
        fn f1(&self) -> ::tauri_bindgen_host::anyhow::Result<()>;
        fn f2(&self, a: u32) -> ::tauri_bindgen_host::anyhow::Result<()>;
        fn f3(&self, a: u32, b: u32) -> ::tauri_bindgen_host::anyhow::Result<()>;
        fn f4(&self) -> ::tauri_bindgen_host::anyhow::Result<u32>;
        fn f5(&self) -> ::tauri_bindgen_host::anyhow::Result<(u32, u32)>;
        fn f6(
            &self,
            a: u32,
            b: u32,
            c: u32,
        ) -> ::tauri_bindgen_host::anyhow::Result<(u32, u32, u32)>;
    }

    pub fn invoke_handler<U, R>(ctx: U) -> impl Fn(::tauri_bindgen_host::tauri::Invoke<R>)
    where
        U: Imports + Send + Sync + 'static,
        R: ::tauri_bindgen_host::tauri::Runtime,
    {
        move |invoke| match invoke.message.command() {
            "f1" => {
                #[allow(unused_variables)]
                let ::tauri_bindgen_host::tauri::Invoke {
                    message: __tauri_message__,
                    resolver: __tauri_resolver__,
                } = invoke;

                let result = ctx.f1();

                __tauri_resolver__
                    .respond(result.map_err(::tauri_bindgen_host::tauri::InvokeError::from_anyhow));
            }
            "f2" => {
                #[allow(unused_variables)]
                let ::tauri_bindgen_host::tauri::Invoke {
                    message: __tauri_message__,
                    resolver: __tauri_resolver__,
                } = invoke;

                let result = ctx.f2(
                    match ::tauri_bindgen_host::tauri::command::CommandArg::from_command(
                        ::tauri_bindgen_host::tauri::command::CommandItem {
                            name: "f2",
                            key: "a",
                            message: &__tauri_message__,
                        },
                    ) {
                        Ok(arg) => arg,
                        Err(err) => return __tauri_resolver__.invoke_error(err),
                    },
                );

                __tauri_resolver__
                    .respond(result.map_err(::tauri_bindgen_host::tauri::InvokeError::from_anyhow));
            }
            "f3" => {
                #[allow(unused_variables)]
                let ::tauri_bindgen_host::tauri::Invoke {
                    message: __tauri_message__,
                    resolver: __tauri_resolver__,
                } = invoke;

                let result = ctx.f3(
                    match ::tauri_bindgen_host::tauri::command::CommandArg::from_command(
                        ::tauri_bindgen_host::tauri::command::CommandItem {
                            name: "f3",
                            key: "a",
                            message: &__tauri_message__,
                        },
                    ) {
                        Ok(arg) => arg,
                        Err(err) => return __tauri_resolver__.invoke_error(err),
                    },
                    match ::tauri_bindgen_host::tauri::command::CommandArg::from_command(
                        ::tauri_bindgen_host::tauri::command::CommandItem {
                            name: "f3",
                            key: "b",
                            message: &__tauri_message__,
                        },
                    ) {
                        Ok(arg) => arg,
                        Err(err) => return __tauri_resolver__.invoke_error(err),
                    },
                );

                __tauri_resolver__
                    .respond(result.map_err(::tauri_bindgen_host::tauri::InvokeError::from_anyhow));
            }
            "f4" => {
                #[allow(unused_variables)]
                let ::tauri_bindgen_host::tauri::Invoke {
                    message: __tauri_message__,
                    resolver: __tauri_resolver__,
                } = invoke;

                let result = ctx.f4();

                __tauri_resolver__
                    .respond(result.map_err(::tauri_bindgen_host::tauri::InvokeError::from_anyhow));
            }
            "f5" => {
                #[allow(unused_variables)]
                let ::tauri_bindgen_host::tauri::Invoke {
                    message: __tauri_message__,
                    resolver: __tauri_resolver__,
                } = invoke;

                let result = ctx.f5();

                __tauri_resolver__
                    .respond(result.map_err(::tauri_bindgen_host::tauri::InvokeError::from_anyhow));
            }
            "f6" => {
                #[allow(unused_variables)]
                let ::tauri_bindgen_host::tauri::Invoke {
                    message: __tauri_message__,
                    resolver: __tauri_resolver__,
                } = invoke;

                let result = ctx.f6(
                    match ::tauri_bindgen_host::tauri::command::CommandArg::from_command(
                        ::tauri_bindgen_host::tauri::command::CommandItem {
                            name: "f6",
                            key: "a",
                            message: &__tauri_message__,
                        },
                    ) {
                        Ok(arg) => arg,
                        Err(err) => return __tauri_resolver__.invoke_error(err),
                    },
                    match ::tauri_bindgen_host::tauri::command::CommandArg::from_command(
                        ::tauri_bindgen_host::tauri::command::CommandItem {
                            name: "f6",
                            key: "b",
                            message: &__tauri_message__,
                        },
                    ) {
                        Ok(arg) => arg,
                        Err(err) => return __tauri_resolver__.invoke_error(err),
                    },
                    match ::tauri_bindgen_host::tauri::command::CommandArg::from_command(
                        ::tauri_bindgen_host::tauri::command::CommandItem {
                            name: "f6",
                            key: "c",
                            message: &__tauri_message__,
                        },
                    ) {
                        Ok(arg) => arg,
                        Err(err) => return __tauri_resolver__.invoke_error(err),
                    },
                );

                __tauri_resolver__
                    .respond(result.map_err(::tauri_bindgen_host::tauri::InvokeError::from_anyhow));
            }
            _ => invoke.resolver.reject("Not Found"),
        }
    }
}
