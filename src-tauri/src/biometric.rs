//! Biométrie Android (BiometricPrompt) via `tauri-plugin-biometric`. Sur ordinateur, elle est
//! toujours indisponible et seul le PIN est proposé.

use lc_shared_dto::{ErrorCode, ErrorDto};
use tauri::{AppHandle, Runtime};

/// Vrai si l'appareil propose une biométrie enregistrée.
#[cfg(mobile)]
pub fn available<R: Runtime>(app: &AppHandle<R>) -> bool {
    use tauri_plugin_biometric::BiometricExt;
    app.biometric()
        .status()
        .is_ok_and(|status| status.is_available)
}

#[cfg(not(mobile))]
pub fn available<R: Runtime>(_app: &AppHandle<R>) -> bool {
    false
}

/// Affiche l'invite biométrique système ; bloque jusqu'à la réponse de l'utilisateur.
#[cfg(mobile)]
pub fn authenticate<R: Runtime>(app: &AppHandle<R>) -> Result<(), ErrorDto> {
    use tauri_plugin_biometric::{AuthOptions, BiometricExt};
    let options = AuthOptions {
        allow_device_credential: false,
        title: Some("Liquid Compagnon".into()),
        cancel_title: Some("Utiliser le code PIN".into()),
        ..AuthOptions::default()
    };
    app.biometric()
        .authenticate("Déverrouiller l'application".into(), options)
        .map_err(|error| {
            log::info!("biométrie refusée : {error}");
            ErrorDto::new(
                ErrorCode::Biometric,
                "Authentification biométrique annulée ou refusée",
            )
        })
}

#[cfg(not(mobile))]
pub fn authenticate<R: Runtime>(_app: &AppHandle<R>) -> Result<(), ErrorDto> {
    Err(ErrorDto::new(
        ErrorCode::Biometric,
        "Biométrie indisponible sur cet appareil",
    ))
}
