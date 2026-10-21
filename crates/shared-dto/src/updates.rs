use serde::{Deserialize, Serialize};

/// Build installé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildInfoDto {
    pub version: String,
    /// Numéro de build de la CI ; absent pour un build local.
    pub build: Option<u32>,
    pub channel: Option<String>,
    pub commit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum UpdateStatusDto {
    /// Build local : pas de mise à jour.
    Unsupported,
    UpToDate,
    Available {
        build: u32,
        version: String,
        notes: String,
        /// Taille de l'APK, en octets.
        size: u64,
    },
    /// Vérification automatique ignorée (moins de 6 h depuis la précédente).
    NotChecked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCheckDto {
    pub current: BuildInfoDto,
    pub status: UpdateStatusDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCheckInput {
    /// Vérification au démarrage : limitée à une toutes les 6 heures.
    pub automatic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallOutcomeDto {
    /// L'installeur Android est ouvert.
    Started,
    /// Android demande d'autoriser l'application à installer des applications.
    PermissionRequired,
    /// Installation impossible sur cette plateforme (ordinateur).
    Unsupported,
}

/// Progression du téléchargement (événement `update-progress`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateProgressDto {
    pub received: u64,
    pub total: u64,
}
