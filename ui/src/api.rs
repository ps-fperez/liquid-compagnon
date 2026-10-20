//! Appels typés vers les commandes Tauri. Seul module de l'UI qui connaît les noms des commandes.

use lc_shared_dto::PingResponse;
use serde::de::DeserializeOwned;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

/// Vrai si l'UI tourne dans Tauri (faux dans un simple navigateur, ex. `trunk serve`).
fn tauri_available() -> bool {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("__TAURI__"))
        .map(|value| !value.is_undefined())
        .unwrap_or(false)
}

async fn invoke<T: DeserializeOwned>(cmd: &str) -> Result<T, String> {
    if !tauri_available() {
        return Err("backend Tauri absent (interface ouverte hors de l'application)".into());
    }
    let value = tauri_invoke(cmd, js_sys::Object::new().into())
        .await
        .map_err(|err| err.as_string().unwrap_or_else(|| format!("{err:?}")))?;
    serde_wasm_bindgen::from_value(value).map_err(|err| err.to_string())
}

pub async fn ping() -> Result<PingResponse, String> {
    invoke("ping").await
}
