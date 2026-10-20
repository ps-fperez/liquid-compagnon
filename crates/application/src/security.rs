//! Code PIN, déverrouillage, biométrie et clé API (réglages sensibles).

use std::sync::Arc;

use chrono::Duration;

use crate::ports::{Clock, PinHasher, SettingsStore, UnlockAttempts};
use crate::{AppError, AppResult};

pub struct SecurityUseCases {
    settings: Arc<dyn SettingsStore>,
    hasher: Arc<dyn PinHasher>,
    clock: Arc<dyn Clock>,
}

impl SecurityUseCases {
    /// Nombre d'échecs consécutifs tolérés avant blocage.
    pub const FREE_ATTEMPTS: u32 = 5;
    /// Durée du premier blocage, doublée à chaque nouvel échec.
    pub const BASE_LOCKOUT_SECONDS: u64 = 30;
    const MAX_LOCKOUT_SECONDS: u64 = 24 * 60 * 60;

    pub fn new(
        settings: Arc<dyn SettingsStore>,
        hasher: Arc<dyn PinHasher>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            settings,
            hasher,
            clock,
        }
    }

    pub fn is_pin_set(&self) -> AppResult<bool> {
        Ok(self.settings.pin_hash()?.is_some())
    }

    /// Premier lancement : définit le PIN (saisi deux fois).
    pub fn setup_pin(&self, pin: &str, confirmation: &str) -> AppResult<()> {
        if self.is_pin_set()? {
            return Err(AppError::PinAlreadySet);
        }
        self.store_new_pin(pin, confirmation)
    }

    /// Vérifie le PIN. Après 5 échecs consécutifs, chaque échec bloque la saisie pendant
    /// 30 s, 60 s, 120 s… Un succès remet le compteur à zéro.
    pub fn unlock(&self, pin: &str) -> AppResult<()> {
        let hash = self.settings.pin_hash()?.ok_or(AppError::PinNotSet)?;
        if let Some(seconds) = self.lockout_remaining_seconds()? {
            return Err(AppError::LockedOut { seconds });
        }
        let mut attempts = self.settings.unlock_attempts()?;
        if self.hasher.verify(pin, &hash) {
            self.settings
                .set_unlock_attempts(UnlockAttempts::default())?;
            return Ok(());
        }
        attempts.consecutive_failures += 1;
        let lockout_seconds = lockout_for(attempts.consecutive_failures);
        attempts.locked_until =
            lockout_seconds.map(|seconds| self.clock.now() + Duration::seconds(seconds as i64));
        self.settings.set_unlock_attempts(attempts)?;
        Err(AppError::WrongPin { lockout_seconds })
    }

    /// Secondes restantes avant de pouvoir ressaisir le PIN (compte à rebours), si bloqué.
    pub fn lockout_remaining_seconds(&self) -> AppResult<Option<u64>> {
        let attempts = self.settings.unlock_attempts()?;
        let remaining = attempts
            .locked_until
            .map(|until| (until - self.clock.now()).num_milliseconds())
            .filter(|millis| *millis > 0)
            .map(|millis| (millis as u64).div_ceil(1000));
        Ok(remaining)
    }

    pub fn change_pin(&self, current: &str, new_pin: &str, confirmation: &str) -> AppResult<()> {
        let hash = self.settings.pin_hash()?.ok_or(AppError::PinNotSet)?;
        if !self.hasher.verify(current, &hash) {
            return Err(AppError::WrongPin {
                lockout_seconds: None,
            });
        }
        self.store_new_pin(new_pin, confirmation)
    }

    pub fn biometric_enabled(&self) -> AppResult<bool> {
        Ok(self.settings.biometric_enabled()?)
    }

    pub fn set_biometric_enabled(&self, enabled: bool) -> AppResult<()> {
        Ok(self.settings.set_biometric_enabled(enabled)?)
    }

    pub fn onboarding_done(&self) -> AppResult<bool> {
        Ok(self.settings.onboarding_done()?)
    }

    pub fn set_onboarding_done(&self, done: bool) -> AppResult<()> {
        Ok(self.settings.set_onboarding_done(done)?)
    }

    pub fn has_twelve_data_api_key(&self) -> AppResult<bool> {
        Ok(self.settings.twelve_data_api_key()?.is_some())
    }

    /// Enregistre (ou efface, si vide) la clé API Twelve Data.
    pub fn set_twelve_data_api_key(&self, key: Option<&str>) -> AppResult<()> {
        let key = key.map(str::trim).filter(|key| !key.is_empty());
        Ok(self.settings.set_twelve_data_api_key(key)?)
    }

    fn store_new_pin(&self, pin: &str, confirmation: &str) -> AppResult<()> {
        validate_pin_format(pin)?;
        if pin != confirmation {
            return Err(AppError::PinMismatch);
        }
        let hash = self.hasher.hash(pin)?;
        self.settings.set_pin_hash(&hash)?;
        self.settings
            .set_unlock_attempts(UnlockAttempts::default())?;
        Ok(())
    }
}

fn validate_pin_format(pin: &str) -> AppResult<()> {
    let valid = (4..=6).contains(&pin.len()) && pin.chars().all(|c| c.is_ascii_digit());
    if valid {
        Ok(())
    } else {
        Err(AppError::InvalidPinFormat)
    }
}

fn lockout_for(consecutive_failures: u32) -> Option<u64> {
    let beyond = consecutive_failures.checked_sub(SecurityUseCases::FREE_ATTEMPTS)?;
    let factor = 1u64 << beyond.min(20);
    Some(
        SecurityUseCases::BASE_LOCKOUT_SECONDS
            .saturating_mul(factor)
            .min(SecurityUseCases::MAX_LOCKOUT_SECONDS),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestEnv;

    fn security_with_pin(env: &TestEnv) -> SecurityUseCases {
        let security = env.security();
        security.setup_pin("123456", "123456").unwrap();
        security
    }

    // Spec app-security : « Création réussie »
    #[test]
    fn setup_stores_only_a_hash() {
        let env = TestEnv::new();
        let security = security_with_pin(&env);

        assert!(security.is_pin_set().unwrap());
        assert_ne!(env.settings.pin_hash().unwrap().as_deref(), Some("123456"));
    }

    // Spec app-security : « Confirmation différente »
    #[test]
    fn setup_rejects_mismatch_and_bad_format() {
        let env = TestEnv::new();
        let security = env.security();

        assert_eq!(
            security.setup_pin("123456", "654321"),
            Err(AppError::PinMismatch)
        );
        assert_eq!(
            security.setup_pin("12a4", "12a4"),
            Err(AppError::InvalidPinFormat)
        );
        assert_eq!(
            security.setup_pin("123", "123"),
            Err(AppError::InvalidPinFormat)
        );
        assert_eq!(
            security.setup_pin("1234567", "1234567"),
            Err(AppError::InvalidPinFormat)
        );
        assert!(!security.is_pin_set().unwrap());
    }

    #[test]
    fn setup_cannot_overwrite_existing_pin() {
        let env = TestEnv::new();
        let security = security_with_pin(&env);
        assert_eq!(
            security.setup_pin("9999", "9999"),
            Err(AppError::PinAlreadySet)
        );
    }

    #[test]
    fn unlock_with_correct_pin() {
        let env = TestEnv::new();
        assert_eq!(security_with_pin(&env).unlock("123456"), Ok(()));
    }

    // Spec app-security : « Cinquième échec »
    #[test]
    fn fifth_failure_locks_for_thirty_seconds() {
        let env = TestEnv::new();
        let security = security_with_pin(&env);

        for _ in 0..4 {
            assert_eq!(
                security.unlock("000000"),
                Err(AppError::WrongPin {
                    lockout_seconds: None
                })
            );
        }
        assert_eq!(
            security.unlock("000000"),
            Err(AppError::WrongPin {
                lockout_seconds: Some(30)
            })
        );
        assert_eq!(
            security.unlock("123456"),
            Err(AppError::LockedOut { seconds: 30 })
        );

        env.clock.advance(Duration::seconds(12));
        assert_eq!(security.lockout_remaining_seconds().unwrap(), Some(18));
    }

    #[test]
    fn lockout_doubles_with_each_new_failure() {
        let env = TestEnv::new();
        let security = security_with_pin(&env);
        for _ in 0..5 {
            let _ = security.unlock("000000");
        }

        env.clock.advance(Duration::seconds(30));
        assert_eq!(
            security.unlock("000000"),
            Err(AppError::WrongPin {
                lockout_seconds: Some(60)
            })
        );
        env.clock.advance(Duration::seconds(60));
        assert_eq!(
            security.unlock("000000"),
            Err(AppError::WrongPin {
                lockout_seconds: Some(120)
            })
        );
    }

    // Spec app-security : « Réinitialisation du compteur »
    #[test]
    fn success_resets_failure_counter() {
        let env = TestEnv::new();
        let security = security_with_pin(&env);
        for _ in 0..4 {
            let _ = security.unlock("000000");
        }

        security.unlock("123456").unwrap();

        assert_eq!(
            env.settings.unlock_attempts().unwrap(),
            UnlockAttempts::default()
        );
        assert_eq!(
            security.unlock("000000"),
            Err(AppError::WrongPin {
                lockout_seconds: None
            })
        );
    }

    #[test]
    fn lockout_survives_restart_because_it_is_persisted() {
        let env = TestEnv::new();
        for _ in 0..5 {
            let _ = security_with_pin_once(&env).unlock("000000");
        }
        let after_restart = env.security();
        assert_eq!(
            after_restart.unlock("123456"),
            Err(AppError::LockedOut { seconds: 30 })
        );
    }

    fn security_with_pin_once(env: &TestEnv) -> SecurityUseCases {
        let security = env.security();
        if !security.is_pin_set().unwrap() {
            security.setup_pin("123456", "123456").unwrap();
        }
        security
    }

    // Spec app-security : « Changement accepté »
    #[test]
    fn change_pin_requires_current_pin() {
        let env = TestEnv::new();
        let security = security_with_pin(&env);

        assert_eq!(
            security.change_pin("000000", "4321", "4321"),
            Err(AppError::WrongPin {
                lockout_seconds: None
            })
        );
        security.change_pin("123456", "4321", "4321").unwrap();

        assert!(security.unlock("4321").is_ok());
        assert!(security.unlock("123456").is_err());
    }

    #[test]
    fn api_key_is_trimmed_and_blank_clears_it() {
        let env = TestEnv::new();
        let security = env.security();

        security.set_twelve_data_api_key(Some("  abc123 ")).unwrap();
        assert_eq!(
            env.settings.twelve_data_api_key().unwrap().as_deref(),
            Some("abc123")
        );

        security.set_twelve_data_api_key(Some("   ")).unwrap();
        assert!(!security.has_twelve_data_api_key().unwrap());
    }
}
