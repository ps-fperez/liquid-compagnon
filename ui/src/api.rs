//! Appels typés vers les commandes Tauri. Seul module de l'UI qui connaît les noms des commandes.

use lc_shared_dto::PingResponse;
use serde::de::DeserializeOwned;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

async fn invoke<T: DeserializeOwned>(cmd: &str) -> Result<T, String> {
    let value = tauri_invoke(cmd, js_sys::Object::new().into())
        .await
        .map_err(|err| err.as_string().unwrap_or_else(|| format!("{err:?}")))?;
    serde_wasm_bindgen::from_value(value).map_err(|err| err.to_string())
}

pub async fn ping() -> Result<PingResponse, String> {
    invoke("ping").await
}
