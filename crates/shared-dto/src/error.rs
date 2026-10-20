use serde::{Deserialize, Serialize};

/// Catégorie d'erreur, pour que l'UI réagisse sans analyser le message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// L'application est verrouillée : afficher l'écran de déverrouillage.
    SessionLocked,
    WrongPin,
    /// Trop d'essais : compte à rebours (`lockout_seconds`).
    LockedOut,
    NotFound,
    /// Saisie refusée par une règle métier (message à afficher près du champ).
    Invalid,
    Pricing,
    Backup,
    Biometric,
    Storage,
    Internal,
}

/// Erreur renvoyée par une commande. `message` est en français, prêt à afficher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorDto {
    pub code: ErrorCode,
    pub message: String,
    pub lockout_seconds: Option<u64>,
}

impl ErrorDto {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            lockout_seconds: None,
        }
    }
}

impl std::fmt::Display for ErrorDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
