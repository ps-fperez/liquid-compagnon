//! Mises à jour : identité du build (fixée par la CI à la compilation), mémoire des
//! vérifications, et pont vers l'installeur Android (`ApkInstallerPlugin.kt`).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use lc_application::updates::{BuildInfo, ReleaseManifest, UpdateService, UpdateStatus};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Manager, Runtime};

/// Dépôt GitHub dont les releases publient l'APK et `latest.json`.
pub const REPOSITORY: &str = "ps-fperez/liquid-compagnon";
/// Événement de progression du téléchargement (`UpdateProgressDto`).
pub const PROGRESS_EVENT: &str = "update-progress";
const AUTOMATIC_CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// Build installé. `LC_BUILD_NUMBER`, `LC_UPDATE_CHANNEL` et `LC_COMMIT` ne sont définis que par
/// la CI : un build local n'a pas de numéro et ne propose jamais de mise à jour.
pub fn build_info() -> BuildInfo {
    let defined = |value: Option<&str>| value.filter(|v| !v.is_empty()).map(String::from);
    BuildInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        build: option_env!("LC_BUILD_NUMBER").and_then(|n| n.parse().ok()),
        channel: defined(option_env!("LC_UPDATE_CHANNEL")),
        commit: defined(option_env!("LC_COMMIT")),
    }
}

/// Service de mise à jour et mémoire de la session : dernière vérification automatique et
/// dernier résultat connu.
pub struct Updates {
    pub service: UpdateService,
    last_automatic_check: Mutex<Option<Instant>>,
    latest: Mutex<Option<UpdateStatus>>,
}

impl Updates {
    pub fn new(service: UpdateService) -> Self {
        Self {
            service,
            last_automatic_check: Mutex::default(),
            latest: Mutex::default(),
        }
    }

    /// Vrai si une vérification automatique est due ; elle est alors réservée.
    pub fn automatic_check_due(&self, now: Instant) -> bool {
        let mut last = self
            .last_automatic_check
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if last.is_some_and(|at| now.duration_since(at) < AUTOMATIC_CHECK_INTERVAL) {
            return false;
        }
        *last = Some(now);
        true
    }

    pub fn remember(&self, status: UpdateStatus) {
        *self.latest.lock().unwrap_or_else(|p| p.into_inner()) = Some(status);
    }

    pub fn latest(&self) -> Option<UpdateStatus> {
        self.latest
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    pub fn available(&self) -> Option<ReleaseManifest> {
        match self.latest() {
            Some(UpdateStatus::Available(manifest)) => Some(manifest),
            _ => None,
        }
    }
}

/// Emplacement de l'APK téléchargé : cache privé, partagé avec l'installeur par le
/// FileProvider (`res/xml/file_paths.xml`, chemin `updates/`).
pub fn apk_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?;
    Ok(cache.join("updates").join("liquid-compagnon.apk"))
}

// Chaque plateforme n'en produit qu'une partie (Android : `Started` ou `PermissionRequired`).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOutcome {
    Started,
    PermissionRequired,
    Unsupported,
}

#[cfg(target_os = "android")]
struct ApkInstaller<R: Runtime>(tauri::plugin::PluginHandle<R>);

/// Enregistre le plugin Kotlin `ApkInstallerPlugin` (module `app` du projet Android).
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("apk-installer")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle =
                    api.register_android_plugin("com.liquidcompagnon.app", "ApkInstallerPlugin")?;
                app.manage(ApkInstaller(handle));
            }
            #[cfg(not(target_os = "android"))]
            let _ = (app, api);
            Ok(())
        })
        .build()
}

/// Ouvre l'installeur Android sur l'APK vérifié (bloquant : à appeler hors du thread de l'UI).
#[cfg(target_os = "android")]
pub fn install<R: Runtime>(app: &AppHandle<R>, apk: &Path) -> Result<InstallOutcome, String> {
    #[derive(serde::Serialize)]
    struct InstallArgs {
        path: String,
    }
    #[derive(serde::Deserialize)]
    struct InstallResponse {
        status: String,
    }
    let installer = app
        .try_state::<ApkInstaller<R>>()
        .ok_or("installeur Android indisponible")?;
    let response: InstallResponse = installer
        .0
        .run_mobile_plugin(
            "install",
            InstallArgs {
                path: apk.display().to_string(),
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(match response.status.as_str() {
        "permission_required" => InstallOutcome::PermissionRequired,
        _ => InstallOutcome::Started,
    })
}

#[cfg(not(target_os = "android"))]
pub fn install<R: Runtime>(_app: &AppHandle<R>, _apk: &Path) -> Result<InstallOutcome, String> {
    Ok(InstallOutcome::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_checks_are_spaced_by_six_hours() {
        let updates = Updates {
            service: UpdateService::new(
                Box::new(NeverFeed),
                Box::new(NeverDownload),
                build_info(),
                REPOSITORY,
            ),
            last_automatic_check: Mutex::default(),
            latest: Mutex::default(),
        };
        let start = Instant::now();

        assert!(updates.automatic_check_due(start));
        assert!(!updates.automatic_check_due(start + Duration::from_secs(60)));
        assert!(updates.automatic_check_due(start + AUTOMATIC_CHECK_INTERVAL));
    }

    #[test]
    fn local_builds_have_no_number() {
        // Les tests ne sont jamais compilés par la CI de l'APK : pas de numéro de build.
        assert_eq!(build_info().version, env!("CARGO_PKG_VERSION"));
    }

    struct NeverFeed;

    #[async_trait::async_trait]
    impl lc_application::updates::ReleaseFeed for NeverFeed {
        async fn latest(
            &self,
            _: &str,
        ) -> Result<ReleaseManifest, lc_application::updates::UpdateError> {
            unreachable!()
        }
    }

    struct NeverDownload;

    #[async_trait::async_trait]
    impl lc_application::updates::PackageDownloader for NeverDownload {
        async fn download(
            &self,
            _: &ReleaseManifest,
            _: &Path,
            _: &(dyn Fn(u64, u64) + Send + Sync),
        ) -> Result<(), lc_application::updates::UpdateError> {
            unreachable!()
        }
    }
}
