use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// État de la session, demandé par l'UI au démarrage et au retour au premier plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStatusDto {
    /// Faux au premier lancement : afficher la création du PIN.
    pub pin_set: bool,
    pub unlocked: bool,
    pub biometric_enabled: bool,
    /// Vrai si l'appareil propose une biométrie enregistrée (toujours faux sur ordinateur).
    pub biometric_available: bool,
    pub lockout_remaining_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinInput {
    pub pin: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToggleInput {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyInput {
    /// Vide ou absente pour effacer la clé.
    pub key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshInput {
    /// Ignore le cache de 5 minutes (« tirer pour rafraîchir »).
    pub force_refresh: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncludeArchivedInput {
    pub include_archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportInput {
    /// Contenu JSON du fichier choisi par l'utilisateur.
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupPinInput {
    pub pin: String,
    pub confirmation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangePinInput {
    pub current: String,
    pub new_pin: String,
    pub confirmation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsDto {
    pub biometric_enabled: bool,
    pub biometric_available: bool,
    pub has_twelve_data_key: bool,
    pub last_export_at: Option<DateTime<Utc>>,
    pub export_reminder_due: bool,
}

/// Contenu d'un export, que l'UI enregistre via le sélecteur de fichiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportDto {
    pub file_name: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSummaryDto {
    pub accounts: usize,
    pub expenses: usize,
    pub snapshots: usize,
}
