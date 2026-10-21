//! Mises à jour de l'application : le build installé est comparé au manifeste `latest.json`
//! publié par la CI avec chaque APK, puis l'APK annoncé est téléchargé et vérifié.

use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Identité du build installé, fixée à la compilation par la CI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildInfo {
    pub version: String,
    /// Numéro de run de la CI ; absent pour un build local (développement).
    pub build: Option<u32>,
    /// `dernier-build` (branche main) ou `stable` (version taguée).
    pub channel: Option<String>,
    pub commit: Option<String>,
}

/// Contenu de `latest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReleaseManifest {
    pub build: u32,
    pub version: String,
    pub commit: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    /// Titre du commit ou de la version.
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UpdateError {
    #[error("Vérification impossible, réessayez plus tard ({0})")]
    Unavailable(String),
    #[error("Annonce de version invalide ({0})")]
    InvalidManifest(String),
    #[error("Téléchargement corrompu, réessayez")]
    Corrupted,
    #[error("Enregistrement du fichier impossible ({0})")]
    Storage(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    /// Build local, sans numéro : aucune mise à jour proposée.
    Unsupported,
    UpToDate,
    Available(ReleaseManifest),
}

/// Lecture du manifeste d'un canal.
#[async_trait]
pub trait ReleaseFeed: Send + Sync {
    async fn latest(&self, channel: &str) -> Result<ReleaseManifest, UpdateError>;
}

/// Téléchargement d'un APK vers `dest`, conservé seulement si sa taille et son empreinte
/// SHA-256 correspondent au manifeste. `progress` reçoit (octets reçus, taille attendue).
#[async_trait]
pub trait PackageDownloader: Send + Sync {
    async fn download(
        &self,
        manifest: &ReleaseManifest,
        dest: &Path,
        progress: &(dyn Fn(u64, u64) + Send + Sync),
    ) -> Result<(), UpdateError>;
}

pub struct UpdateService {
    feed: Box<dyn ReleaseFeed>,
    downloader: Box<dyn PackageDownloader>,
    build: BuildInfo,
    /// Seules les releases du dépôt de l'application sont acceptées.
    allowed_prefix: String,
}

impl UpdateService {
    /// `repository` : `propriétaire/dépôt` GitHub de l'application.
    pub fn new(
        feed: Box<dyn ReleaseFeed>,
        downloader: Box<dyn PackageDownloader>,
        build: BuildInfo,
        repository: &str,
    ) -> Self {
        Self {
            feed,
            downloader,
            build,
            allowed_prefix: format!("https://github.com/{repository}/releases/download/"),
        }
    }

    pub fn build(&self) -> &BuildInfo {
        &self.build
    }

    /// Une version n'est proposée que si son numéro de build dépasse celui installé.
    pub async fn check(&self) -> Result<UpdateStatus, UpdateError> {
        let (Some(installed), Some(channel)) = (self.build.build, self.build.channel.as_deref())
        else {
            return Ok(UpdateStatus::Unsupported);
        };
        let manifest = self.feed.latest(channel).await?;
        self.validate(&manifest)?;
        Ok(if manifest.build > installed {
            UpdateStatus::Available(manifest)
        } else {
            UpdateStatus::UpToDate
        })
    }

    pub async fn download(
        &self,
        manifest: &ReleaseManifest,
        dest: &Path,
        progress: &(dyn Fn(u64, u64) + Send + Sync),
    ) -> Result<(), UpdateError> {
        self.validate(manifest)?;
        self.downloader.download(manifest, dest, progress).await
    }

    fn validate(&self, manifest: &ReleaseManifest) -> Result<(), UpdateError> {
        if !manifest.url.starts_with(&self.allowed_prefix) || manifest.url.contains("..") {
            return Err(UpdateError::InvalidManifest(
                "APK hors des releases de l'application".into(),
            ));
        }
        let hex =
            manifest.sha256.len() == 64 && manifest.sha256.chars().all(|c| c.is_ascii_hexdigit());
        if !hex || manifest.size == 0 {
            return Err(UpdateError::InvalidManifest(
                "empreinte ou taille absente".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pollster::block_on;
    use std::sync::Mutex;

    const REPO: &str = "ps-fperez/liquid-compagnon";

    struct Feed(Mutex<Result<ReleaseManifest, UpdateError>>);

    #[async_trait]
    impl ReleaseFeed for Feed {
        async fn latest(&self, channel: &str) -> Result<ReleaseManifest, UpdateError> {
            assert_eq!(channel, "dernier-build");
            self.0.lock().unwrap().clone()
        }
    }

    struct NoDownload;

    #[async_trait]
    impl PackageDownloader for NoDownload {
        async fn download(
            &self,
            _: &ReleaseManifest,
            _: &Path,
            _: &(dyn Fn(u64, u64) + Send + Sync),
        ) -> Result<(), UpdateError> {
            Ok(())
        }
    }

    fn manifest(build: u32) -> ReleaseManifest {
        ReleaseManifest {
            build,
            version: "0.2.0".into(),
            commit: "abc1234".into(),
            url: format!("https://github.com/{REPO}/releases/download/dernier-build/app.apk"),
            size: 12_000_000,
            sha256: "a".repeat(64),
            notes: "Compte courant".into(),
            published_at: None,
        }
    }

    fn service(
        installed: Option<u32>,
        published: Result<ReleaseManifest, UpdateError>,
    ) -> UpdateService {
        UpdateService::new(
            Box::new(Feed(Mutex::new(published))),
            Box::new(NoDownload),
            BuildInfo {
                version: "0.2.0".into(),
                build: installed,
                channel: Some("dernier-build".into()),
                commit: None,
            },
            REPO,
        )
    }

    // Spec app-updates : « Version plus récente » et « Déjà à jour »
    #[test]
    fn newer_build_is_proposed() {
        assert_eq!(
            block_on(service(Some(57), Ok(manifest(60))).check()),
            Ok(UpdateStatus::Available(manifest(60)))
        );
        assert_eq!(
            block_on(service(Some(60), Ok(manifest(60))).check()),
            Ok(UpdateStatus::UpToDate)
        );
        assert_eq!(
            block_on(service(Some(61), Ok(manifest(60))).check()),
            Ok(UpdateStatus::UpToDate)
        );
    }

    #[test]
    fn local_builds_never_update() {
        assert_eq!(
            block_on(service(None, Ok(manifest(60))).check()),
            Ok(UpdateStatus::Unsupported)
        );
    }

    // Spec app-updates : « Hors ligne »
    #[test]
    fn feed_errors_are_reported() {
        let offline = Err(UpdateError::Unavailable("hors ligne".into()));
        assert!(matches!(
            block_on(service(Some(57), offline).check()),
            Err(UpdateError::Unavailable(_))
        ));
    }

    #[test]
    fn apk_must_come_from_the_app_releases() {
        let mut foreign = manifest(60);
        foreign.url = "https://example.com/app.apk".into();
        assert!(matches!(
            block_on(service(Some(57), Ok(foreign)).check()),
            Err(UpdateError::InvalidManifest(_))
        ));

        let mut traversal = manifest(60);
        traversal.url = format!("https://github.com/{REPO}/releases/download/../../evil.apk");
        assert!(matches!(
            block_on(service(Some(57), Ok(traversal)).check()),
            Err(UpdateError::InvalidManifest(_))
        ));

        let mut no_hash = manifest(60);
        no_hash.sha256 = "abc".into();
        assert!(matches!(
            block_on(service(Some(57), Ok(no_hash)).check()),
            Err(UpdateError::InvalidManifest(_))
        ));
    }
}
