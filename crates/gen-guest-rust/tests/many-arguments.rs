#[allow(clippy::all, unused)]
pub mod imports {

    #[::wasm_bindgen::prelude::wasm_bindgen]
    extern "C" {
        #[::wasm_bindgen::prelude::wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"])]
        pub async fn invoke(
            cmd: ::wasm_bindgen::prelude::JsValue,
            args: ::wasm_bindgen::prelude::JsValue,
        ) -> ::wasm_bindgen::prelude::JsValue;
    }

    #[derive(Debug, Clone, ::serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct BigStruct<'a> {
        pub a1: &'a str,
        pub a2: &'a str,
        pub a3: &'a str,
        pub a4: &'a str,
        pub a5: &'a str,
        pub a6: &'a str,
        pub a7: &'a str,
        pub a8: &'a str,
        pub a9: &'a str,
        pub a10: &'a str,
        pub a11: &'a str,
        pub a12: &'a str,
        pub a13: &'a str,
        pub a14: &'a str,
        pub a15: &'a str,
        pub a16: &'a str,
        pub a17: &'a str,
        pub a18: &'a str,
        pub a19: &'a str,
        pub a20: &'a str,
    }
    pub async fn many_args(
        a1: u64,
        a2: u64,
        a3: u64,
        a4: u64,
        a5: u64,
        a6: u64,
        a7: u64,
        a8: u64,
        a9: u64,
        a10: u64,
        a11: u64,
        a12: u64,
        a13: u64,
        a14: u64,
        a15: u64,
        a16: u64,
    ) -> () {
        #[derive(::serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            a1: u64,
            a2: u64,
            a3: u64,
            a4: u64,
            a5: u64,
            a6: u64,
            a7: u64,
            a8: u64,
            a9: u64,
            a10: u64,
            a11: u64,
            a12: u64,
            a13: u64,
            a14: u64,
            a15: u64,
            a16: u64,
        }
        let params = Params {
            a1,
            a2,
            a3,
            a4,
            a5,
            a6,
            a7,
            a8,
            a9,
            a10,
            a11,
            a12,
            a13,
            a14,
            a15,
            a16,
        };
        let raw = invoke(
            ::wasm_bindgen::JsValue::from_str("plugin:imports|many_args"),
            ::serde_wasm_bindgen::to_value(&params).unwrap(),
        )
        .await;
        ::serde_wasm_bindgen::from_value(raw).unwrap()
    }
    pub async fn big_argument(x: BigStruct<'_>) -> () {
        #[derive(::serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Params<'a> {
            x: BigStruct<'a>,
        }
        let params = Params { x };
        let raw = invoke(
            ::wasm_bindgen::JsValue::from_str("plugin:imports|big_argument"),
            ::serde_wasm_bindgen::to_value(&params).unwrap(),
        )
        .await;
        ::serde_wasm_bindgen::from_value(raw).unwrap()
    }
}
