//! Session, PIN, biométrie, réglages et sauvegarde.

use std::time::Instant;

use lc_shared_dto::*;
use tauri::AppHandle;

use super::{AppState, blocking, unlocked};
use crate::biometric;
use crate::error::{CommandResult, app_error, invalid};

#[tauri::command]
pub async fn session_status(
    app: AppHandle,
    state: AppState<'_>,
) -> CommandResult<SessionStatusDto> {
    blocking(state.inner().clone(), move |services| {
        let security = &services.security;
        Ok(SessionStatusDto {
            pin_set: security.is_pin_set().map_err(app_error)?,
            unlocked: services.session.is_unlocked(),
            biometric_enabled: security.biometric_enabled().map_err(app_error)?,
            biometric_available: biometric::available(&app),
            lockout_remaining_seconds: security.lockout_remaining_seconds().map_err(app_error)?,
        })
    })
    .await
}

/// Premier lancement : définit le PIN puis ouvre la session.
#[tauri::command]
pub async fn setup_pin(state: AppState<'_>, input: SetupPinInput) -> CommandResult<()> {
    blocking(state.inner().clone(), move |services| {
        services
            .security
            .setup_pin(&input.pin, &input.confirmation)
            .map_err(app_error)?;
        services.session.unlock();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn unlock(state: AppState<'_>, input: PinInput) -> CommandResult<()> {
    blocking(state.inner().clone(), move |services| {
        services.security.unlock(&input.pin).map_err(app_error)?;
        services.session.unlock();
        Ok(())
    })
    .await
}

/// L'invite biométrique est affichée par le backend : l'UI ne peut pas ouvrir la session seule.
#[tauri::command]
pub async fn unlock_with_biometric(app: AppHandle, state: AppState<'_>) -> CommandResult<()> {
    blocking(state.inner().clone(), move |services| {
        if !services.security.biometric_enabled().map_err(app_error)? {
            return Err(invalid("La biométrie n'est pas activée dans les réglages"));
        }
        biometric::authenticate(&app)?;
        services.session.unlock();
        Ok(())
    })
    .await
}

#[tauri::command]
pub fn lock(state: AppState<'_>) {
    state.session.lock();
}

/// L'UI signale qu'elle est masquée (application en arrière-plan).
#[tauri::command]
pub fn notify_hidden(state: AppState<'_>) {
    state.session.went_to_background(Instant::now());
}

/// L'UI signale qu'elle est de nouveau visible. Renvoie `true` si la session vient d'être
/// verrouillée (plus de 60 s en arrière-plan).
#[tauri::command]
pub fn notify_visible(state: AppState<'_>) -> bool {
    state.session.came_to_foreground(Instant::now())
}

#[tauri::command]
pub async fn change_pin(state: AppState<'_>, input: ChangePinInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .security
            .change_pin(&input.current, &input.new_pin, &input.confirmation)
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn get_settings(app: AppHandle, state: AppState<'_>) -> CommandResult<SettingsDto> {
    blocking(unlocked(&state)?, move |services| {
        let last_export_at = services.backup.last_export_at().map_err(app_error)?;
        Ok(SettingsDto {
            biometric_enabled: services.security.biometric_enabled().map_err(app_error)?,
            biometric_available: biometric::available(&app),
            has_twelve_data_key: services
                .security
                .has_twelve_data_api_key()
                .map_err(app_error)?,
            last_export_at,
            export_reminder_due: services.backup.export_reminder_due().map_err(app_error)?,
            onboarding_done: services.security.onboarding_done().map_err(app_error)?,
        })
    })
    .await
}

/// Marque le didacticiel comme parcouru (`false` pour le revoir au prochain affichage).
#[tauri::command]
pub async fn set_onboarding_done(state: AppState<'_>, input: ToggleInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .security
            .set_onboarding_done(input.enabled)
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn set_biometric_enabled(
    app: AppHandle,
    state: AppState<'_>,
    input: ToggleInput,
) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        if input.enabled && !biometric::available(&app) {
            return Err(invalid(
                "Aucune biométrie n'est enregistrée sur cet appareil",
            ));
        }
        services
            .security
            .set_biometric_enabled(input.enabled)
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn set_twelve_data_api_key(state: AppState<'_>, input: ApiKeyInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .security
            .set_twelve_data_api_key(input.key.as_deref())
            .map_err(app_error)
    })
    .await
}

/// Contenu de l'export (le PIN et la clé API en sont exclus) ; l'UI l'enregistre via le
/// sélecteur de fichiers.
#[tauri::command]
pub async fn export_backup(state: AppState<'_>) -> CommandResult<ExportDto> {
    blocking(unlocked(&state)?, move |services| {
        let bytes = services.backup.export().map_err(app_error)?;
        Ok(ExportDto {
            file_name: services.backup.suggested_file_name(),
            content: String::from_utf8(bytes).map_err(crate::error::internal)?,
        })
    })
    .await
}

/// Remplace toutes les données par celles du fichier, après validation complète.
#[tauri::command]
pub async fn import_backup(
    state: AppState<'_>,
    input: ImportInput,
) -> CommandResult<ImportSummaryDto> {
    blocking(unlocked(&state)?, move |services| {
        let summary = services
            .backup
            .import(input.content.as_bytes())
            .map_err(app_error)?;
        Ok(ImportSummaryDto {
            accounts: summary.accounts,
            expenses: summary.expenses,
            snapshots: summary.snapshots,
        })
    })
    .await
}
