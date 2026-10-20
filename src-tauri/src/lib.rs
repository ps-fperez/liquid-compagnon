//! Backend Tauri de Liquid Compagnon : couche présentation et composition root.
//!
//! Les commandes Tauri convertissent les DTO de `lc-shared-dto` vers les cas d'usage de
//! `lc-application`, dont les ports sont branchés ici sur les adaptateurs de `lc-infrastructure`.

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::ping])
        .run(tauri::generate_context!())
        .expect("échec du démarrage de l'application Tauri");
}
