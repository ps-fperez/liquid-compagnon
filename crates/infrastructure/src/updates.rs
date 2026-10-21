//! Mises à jour : manifeste `latest.json` des releases GitHub et téléchargement vérifié de l'APK.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use lc_application::updates::{PackageDownloader, ReleaseFeed, ReleaseManifest, UpdateError};
use reqwest::{Client, StatusCode};
use sha2::{Digest, Sha256};

/// Client du téléchargement de l'APK : délai large (une dizaine de Mo sur réseau mobile).
pub fn download_client() -> Result<Client, String> {
    Client::builder()
        .user_agent(concat!("LiquidCompagnon/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|error| error.to_string())
}

/// Manifeste publié par la CI comme asset des releases :
/// `…/releases/download/<tag>/latest.json`, ou `…/releases/latest/download/latest.json` pour le
/// canal `stable` (dernière version publiée).
pub struct GithubReleaseFeed {
    client: Client,
    releases_url: String,
}

impl GithubReleaseFeed {
    /// `releases_url` : `https://github.com/<propriétaire>/<dépôt>/releases`.
    pub fn new(client: Client, releases_url: &str) -> Self {
        Self {
            client,
            releases_url: releases_url.trim_end_matches('/').to_string(),
        }
    }

    fn manifest_url(&self, channel: &str) -> String {
        match channel {
            "stable" => format!("{}/latest/download/latest.json", self.releases_url),
            tag => format!("{}/download/{tag}/latest.json", self.releases_url),
        }
    }
}

#[async_trait]
impl ReleaseFeed for GithubReleaseFeed {
    async fn latest(&self, channel: &str) -> Result<ReleaseManifest, UpdateError> {
        let response = self
            .client
            .get(self.manifest_url(channel))
            .send()
            .await
            .map_err(|error| UpdateError::Unavailable(transport(&error)))?;
        match response.status() {
            StatusCode::NOT_FOUND => {
                return Err(UpdateError::Unavailable("aucune version publiée".into()));
            }
            status if !status.is_success() => {
                return Err(UpdateError::Unavailable(format!("HTTP {status}")));
            }
            _ => {}
        }
        response
            .json()
            .await
            .map_err(|error| UpdateError::InvalidManifest(error.to_string()))
    }
}

/// Téléchargement en flux dans `<dest>.part`, avec calcul de l'empreinte au fil de l'eau ; le
/// fichier n'est renommé en `dest` que si taille et empreinte correspondent au manifeste.
pub struct HttpPackageDownloader {
    client: Client,
}

impl HttpPackageDownloader {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl PackageDownloader for HttpPackageDownloader {
    async fn download(
        &self,
        manifest: &ReleaseManifest,
        dest: &Path,
        progress: &(dyn Fn(u64, u64) + Send + Sync),
    ) -> Result<(), UpdateError> {
        if let Some(dir) = dest.parent() {
            fs::create_dir_all(dir).map_err(storage)?;
        }
        let partial = dest.with_extension("part");
        let result = fetch(&self.client, manifest, &partial, progress).await;
        if let Err(error) = result {
            let _ = fs::remove_file(&partial);
            return Err(error);
        }
        fs::rename(&partial, dest).map_err(storage)
    }
}

async fn fetch(
    client: &Client,
    manifest: &ReleaseManifest,
    partial: &Path,
    progress: &(dyn Fn(u64, u64) + Send + Sync),
) -> Result<(), UpdateError> {
    let mut response = client
        .get(&manifest.url)
        .send()
        .await
        .map_err(|error| UpdateError::Unavailable(transport(&error)))?;
    if !response.status().is_success() {
        return Err(UpdateError::Unavailable(format!(
            "HTTP {}",
            response.status()
        )));
    }
    let mut file = File::create(partial).map_err(storage)?;
    let mut hasher = Sha256::new();
    let mut received: u64 = 0;
    progress(0, manifest.size);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| UpdateError::Unavailable(transport(&error)))?
    {
        received += chunk.len() as u64;
        if received > manifest.size {
            return Err(UpdateError::Corrupted);
        }
        hasher.update(&chunk);
        file.write_all(&chunk).map_err(storage)?;
        progress(received, manifest.size);
    }
    file.sync_all().map_err(storage)?;
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if received != manifest.size || !digest.eq_ignore_ascii_case(&manifest.sha256) {
        return Err(UpdateError::Corrupted);
    }
    Ok(())
}

fn transport(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "délai dépassé".into()
    } else if error.is_connect() {
        "pas de connexion".into()
    } else {
        error.to_string()
    }
}

fn storage(error: std::io::Error) -> UpdateError {
    UpdateError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn manifest(server: &MockServer, bytes: &[u8]) -> ReleaseManifest {
        ReleaseManifest {
            build: 60,
            version: "0.2.0".into(),
            commit: "abc1234".into(),
            url: format!("{}/download/dernier-build/app.apk", server.uri()),
            size: bytes.len() as u64,
            sha256: sha256(bytes),
            notes: String::new(),
            published_at: None,
        }
    }

    fn temp_apk(name: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("lc-update-test-{}", std::process::id()))
            .join(name)
    }

    #[tokio::test]
    async fn reads_the_manifest_of_a_channel() {
        let server = MockServer::start().await;
        Mock::given(path("/download/dernier-build/latest.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "build": 60, "version": "0.2.0", "commit": "abc1234",
                "url": "https://github.com/o/r/releases/download/dernier-build/app.apk",
                "size": 12, "sha256": "a".repeat(64), "notes": "Compte courant"
            })))
            .mount(&server)
            .await;
        Mock::given(path("/latest/download/latest.json"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let feed = GithubReleaseFeed::new(Client::new(), &server.uri());

        let manifest = feed.latest("dernier-build").await.unwrap();
        assert_eq!(manifest.build, 60);
        assert_eq!(manifest.notes, "Compte courant");
        assert!(matches!(
            feed.latest("stable").await,
            Err(UpdateError::Unavailable(_))
        ));
    }

    // Spec app-updates : « Mise à jour réussie »
    #[tokio::test]
    async fn verified_download_reports_progress() {
        let server = MockServer::start().await;
        let apk = vec![7_u8; 4096];
        Mock::given(path("/download/dernier-build/app.apk"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(apk.clone()))
            .mount(&server)
            .await;
        let dest = temp_apk("ok.apk");
        let seen = Mutex::new(Vec::new());

        HttpPackageDownloader::new(Client::new())
            .download(&manifest(&server, &apk), &dest, &|received, total| {
                seen.lock().unwrap().push((received, total));
            })
            .await
            .unwrap();

        assert_eq!(fs::read(&dest).unwrap(), apk);
        assert_eq!(seen.lock().unwrap().last(), Some(&(4096, 4096)));
        let _ = fs::remove_file(dest);
    }

    // Spec app-updates : « Fichier altéré »
    #[tokio::test]
    async fn altered_download_is_deleted() {
        let server = MockServer::start().await;
        Mock::given(path("/download/dernier-build/app.apk"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![6_u8; 4096]))
            .mount(&server)
            .await;
        let dest = temp_apk("altered.apk");
        let expected = manifest(&server, &[7_u8; 4096]);

        let result = HttpPackageDownloader::new(Client::new())
            .download(&expected, &dest, &|_, _| {})
            .await;

        assert_eq!(result, Err(UpdateError::Corrupted));
        assert!(!dest.exists());
        assert!(!dest.with_extension("part").exists());
    }
}
