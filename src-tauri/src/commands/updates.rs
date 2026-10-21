use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use lc_application::updates::{BuildInfo, UpdateError, UpdateStatus};
use lc_shared_dto::*;
use tauri::{AppHandle, Emitter};

use super::{AppState, unlocked};
use crate::error::{CommandResult, internal};
use crate::updater::{self, InstallOutcome, PROGRESS_EVENT};

/// Vérifie si une version plus récente est publiée. En mode automatique (au déverrouillage),
/// au plus une requête toutes les 6 heures : sinon, le dernier résultat connu est renvoyé.
#[tauri::command]
pub async fn check_for_update(
    state: AppState<'_>,
    input: UpdateCheckInput,
) -> CommandResult<UpdateCheckDto> {
    let services = unlocked(&state)?;
    let updates = &services.updates;
    let current = build_dto(updates.service.build());
    if input.automatic && !updates.automatic_check_due(Instant::now()) {
        let status = updates
            .latest()
            .map_or(UpdateStatusDto::NotChecked, |status| status_dto(&status));
        return Ok(UpdateCheckDto { current, status });
    }
    let status = updates.service.check().await.map_err(update_error)?;
    updates.remember(status.clone());
    Ok(UpdateCheckDto {
        current,
        status: status_dto(&status),
    })
}

/// Télécharge l'APK annoncé (progression : événement `update-progress`), vérifie sa taille et
/// son empreinte, puis ouvre l'installeur Android.
#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    state: AppState<'_>,
) -> CommandResult<InstallOutcomeDto> {
    let services = unlocked(&state)?;
    if !cfg!(target_os = "android") {
        return Ok(InstallOutcomeDto::Unsupported);
    }
    let manifest = match services.updates.available() {
        Some(manifest) => manifest,
        None => match services
            .updates
            .service
            .check()
            .await
            .map_err(update_error)?
        {
            UpdateStatus::Available(manifest) => manifest,
            _ => {
                return Err(ErrorDto::new(
                    ErrorCode::Update,
                    "Aucune mise à jour disponible",
                ));
            }
        },
    };
    let apk = updater::apk_path(&app).map_err(internal)?;
    let emitter = app.clone();
    let last_percent = AtomicU64::new(u64::MAX);
    let progress = move |received: u64, total: u64| {
        // Un événement par pour cent : quelques dizaines de messages vers l'UI, pas des milliers.
        let percent = received.saturating_mul(100) / total.max(1);
        if last_percent.swap(percent, Ordering::Relaxed) != percent {
            let _ = emitter.emit(PROGRESS_EVENT, UpdateProgressDto { received, total });
        }
    };
    services
        .updates
        .service
        .download(&manifest, &apk, &progress)
        .await
        .map_err(update_error)?;
    let outcome = tauri::async_runtime::spawn_blocking(move || updater::install(&app, &apk))
        .await
        .map_err(internal)?
        .map_err(|message| ErrorDto::new(ErrorCode::Update, message))?;
    Ok(match outcome {
        InstallOutcome::Started => InstallOutcomeDto::Started,
        InstallOutcome::PermissionRequired => InstallOutcomeDto::PermissionRequired,
        InstallOutcome::Unsupported => InstallOutcomeDto::Unsupported,
    })
}

fn build_dto(build: &BuildInfo) -> BuildInfoDto {
    BuildInfoDto {
        version: build.version.clone(),
        build: build.build,
        channel: build.channel.clone(),
        commit: build.commit.clone(),
    }
}

fn status_dto(status: &UpdateStatus) -> UpdateStatusDto {
    match status {
        UpdateStatus::Unsupported => UpdateStatusDto::Unsupported,
        UpdateStatus::UpToDate => UpdateStatusDto::UpToDate,
        UpdateStatus::Available(manifest) => UpdateStatusDto::Available {
            build: manifest.build,
            version: manifest.version.clone(),
            notes: manifest.notes.clone(),
            size: manifest.size,
        },
    }
}

fn update_error(error: UpdateError) -> ErrorDto {
    ErrorDto::new(ErrorCode::Update, error.to_string())
}
