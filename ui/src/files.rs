//! Fichiers choisis par l'utilisateur (export / import JSON) via les plugins Tauri dialog et fs.
//! Seuls les fichiers sélectionnés dans le sélecteur système deviennent accessibles.

use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"], js_name = save, catch)]
    async fn dialog_save(options: JsValue) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"], js_name = open, catch)]
    async fn dialog_open(options: JsValue) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "fs"], js_name = writeTextFile, catch)]
    async fn write_text_file(path: &str, contents: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "fs"], js_name = readTextFile, catch)]
    async fn read_text_file(path: &str) -> Result<JsValue, JsValue>;
}

#[derive(Serialize)]
struct Filter {
    name: &'static str,
    extensions: [&'static str; 1],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveOptions {
    default_path: String,
    filters: [Filter; 1],
}

#[derive(Serialize)]
struct OpenOptions {
    multiple: bool,
    directory: bool,
    filters: [Filter; 1],
}

const JSON_FILTER: Filter = Filter {
    name: "Sauvegarde Liquid Compagnon",
    extensions: ["json"],
};

fn js_error(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}

fn to_js(value: &impl Serialize) -> Result<JsValue, String> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|error| error.to_string())
}

/// Demande où enregistrer `contents`. `Ok(false)` si l'utilisateur annule.
pub async fn save_text(file_name: &str, contents: &str) -> Result<bool, String> {
    let options = to_js(&SaveOptions {
        default_path: file_name.to_string(),
        filters: [JSON_FILTER],
    })?;
    let chosen = dialog_save(options).await.map_err(js_error)?;
    let Some(path) = chosen.as_string() else {
        return Ok(false);
    };
    write_text_file(&path, contents).await.map_err(js_error)?;
    Ok(true)
}

/// Fait choisir un fichier JSON et renvoie son contenu. `Ok(None)` si l'utilisateur annule.
pub async fn open_text() -> Result<Option<String>, String> {
    let options = to_js(&OpenOptions {
        multiple: false,
        directory: false,
        filters: [JSON_FILTER],
    })?;
    let chosen = dialog_open(options).await.map_err(js_error)?;
    let Some(path) = chosen.as_string() else {
        return Ok(None);
    };
    let contents = read_text_file(&path).await.map_err(js_error)?;
    Ok(contents.as_string())
}
