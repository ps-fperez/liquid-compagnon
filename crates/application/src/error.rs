use lc_domain::DomainError;

use crate::ports::{BackupError, PriceError, StorageError};

pub type AppResult<T> = Result<T, AppError>;

/// Erreur d'un cas d'usage. Les messages sont destinés à l'utilisateur final.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("Erreur de stockage : {0}")]
    Storage(#[from] StorageError),
    #[error("{0} introuvable")]
    NotFound(&'static str),
    #[error("Le code PIN doit contenir 4 à 6 chiffres")]
    InvalidPinFormat,
    #[error("Les codes ne correspondent pas")]
    PinMismatch,
    #[error("Un code PIN est déjà défini")]
    PinAlreadySet,
    #[error("Aucun code PIN n'est défini")]
    PinNotSet,
    #[error("Code PIN incorrect")]
    WrongPin {
        /// Durée de blocage déclenchée par cet échec, le cas échéant.
        lockout_seconds: Option<u64>,
    },
    #[error("Trop de tentatives : réessayez dans {seconds} s")]
    LockedOut { seconds: u64 },
    #[error("Cette catégorie est archivée")]
    CategoryArchived,
    #[error("Une catégorie porte déjà ce nom")]
    DuplicateCategory,
    #[error("Cette catégorie est utilisée : archivez-la plutôt")]
    CategoryInUse,
    #[error("Aucune source de cotation configurée pour cet actif")]
    NoPriceProvider,
    #[error("Cours indisponibles : {0}")]
    Pricing(#[from] PriceError),
    #[error("Fichier invalide : {0}")]
    InvalidBackup(String),
    #[error("Mettez à jour l'application pour importer ce fichier")]
    BackupFromNewerVersion,
}

impl From<BackupError> for AppError {
    fn from(error: BackupError) -> Self {
        match error {
            BackupError::Invalid(reason) => AppError::InvalidBackup(reason),
            BackupError::NewerVersion { .. } => AppError::BackupFromNewerVersion,
        }
    }
}
