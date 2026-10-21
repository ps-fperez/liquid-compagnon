//! Conversion des erreurs applicatives en erreurs affichables par l'UI.

use lc_application::AppError;
use lc_shared_dto::{ErrorCode, ErrorDto};

pub type CommandResult<T> = Result<T, ErrorDto>;

pub fn app_error(error: AppError) -> ErrorDto {
    let message = error.to_string();
    match error {
        AppError::WrongPin { lockout_seconds } => ErrorDto {
            code: ErrorCode::WrongPin,
            message,
            lockout_seconds,
        },
        AppError::LockedOut { seconds } => ErrorDto {
            code: ErrorCode::LockedOut,
            message,
            lockout_seconds: Some(seconds),
        },
        AppError::NotFound(_) => ErrorDto::new(ErrorCode::NotFound, message),
        AppError::Domain(_)
        | AppError::InvalidAccountLink
        | AppError::InvalidPinFormat
        | AppError::PinMismatch
        | AppError::PinAlreadySet
        | AppError::PinNotSet
        | AppError::CategoryArchived
        | AppError::DuplicateCategory
        | AppError::CategoryInUse
        | AppError::UnavailableAccount
        | AppError::DuplicateExternalAccount
        | AppError::ExternalAccountInUse => ErrorDto::new(ErrorCode::Invalid, message),
        AppError::Pricing(_) | AppError::NoPriceProvider => {
            ErrorDto::new(ErrorCode::Pricing, message)
        }
        AppError::InvalidBackup(_) | AppError::BackupFromNewerVersion => {
            ErrorDto::new(ErrorCode::Backup, message)
        }
        AppError::Storage(detail) => {
            log::error!("stockage : {detail}");
            ErrorDto::new(ErrorCode::Storage, "L'enregistrement a échoué. Réessayez.")
        }
    }
}

pub fn invalid(message: impl Into<String>) -> ErrorDto {
    ErrorDto::new(ErrorCode::Invalid, message)
}

pub fn internal(detail: impl std::fmt::Display) -> ErrorDto {
    log::error!("erreur interne : {detail}");
    ErrorDto::new(ErrorCode::Internal, "Une erreur inattendue est survenue.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc_application::ports::StorageError;
    use lc_domain::DomainError;

    #[test]
    fn maps_codes_and_keeps_french_messages() {
        let wrong = app_error(AppError::WrongPin {
            lockout_seconds: Some(30),
        });
        assert_eq!(
            (wrong.code, wrong.lockout_seconds),
            (ErrorCode::WrongPin, Some(30))
        );
        assert_eq!(wrong.message, "Code PIN incorrect");

        let negative = app_error(AppError::Domain(DomainError::NonPositiveAmount));
        assert_eq!(negative.code, ErrorCode::Invalid);
        assert_eq!(negative.message, "Saisissez un montant positif");

        let locked_out = app_error(AppError::LockedOut { seconds: 18 });
        assert_eq!(locked_out.lockout_seconds, Some(18));
    }

    #[test]
    fn storage_details_are_not_shown_to_the_user() {
        let error = app_error(AppError::Storage(StorageError(
            "SQLite : disk I/O error".into(),
        )));
        assert_eq!(error.code, ErrorCode::Storage);
        assert!(!error.message.contains("SQLite"));
    }
}
