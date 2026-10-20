use chrono::{DateTime, Utc};
use lc_application::ports::{SettingsStore, StorageResult, UnlockAttempts};
use rusqlite::{OptionalExtension, params};

use super::{Database, SqlResultExt, corrupt, datetime};

const PIN_HASH: &str = "pin_hash";
const UNLOCK_FAILURES: &str = "unlock_failures";
const UNLOCK_LOCKED_UNTIL: &str = "unlock_locked_until";
const BIOMETRIC_ENABLED: &str = "biometric_enabled";
const ONBOARDING_DONE: &str = "onboarding_done";
const TWELVE_DATA_API_KEY: &str = "twelve_data_api_key";
const LAST_EXPORT_AT: &str = "last_export_at";

/// Réglages clé/valeur, dans la base privée de l'application (jamais exportés).
pub struct SqliteSettingsStore {
    db: Database,
}

impl SqliteSettingsStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    fn get(&self, key: &str) -> StorageResult<Option<String>> {
        self.db.read(|conn| {
            conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .db()
        })
    }

    fn set(&self, key: &str, value: Option<String>) -> StorageResult<()> {
        self.db.write(|tx| {
            match value {
                Some(value) => tx.execute(
                    "INSERT INTO settings (key, value) VALUES (?1, ?2)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![key, value],
                ),
                None => tx.execute("DELETE FROM settings WHERE key = ?1", [key]),
            }
            .db()
            .map(drop)
        })
    }
}

impl SettingsStore for SqliteSettingsStore {
    fn pin_hash(&self) -> StorageResult<Option<String>> {
        self.get(PIN_HASH)
    }

    fn set_pin_hash(&self, hash: &str) -> StorageResult<()> {
        self.set(PIN_HASH, Some(hash.to_string()))
    }

    fn unlock_attempts(&self) -> StorageResult<UnlockAttempts> {
        let consecutive_failures = self
            .get(UNLOCK_FAILURES)?
            .map(|raw| raw.parse().map_err(corrupt))
            .transpose()?
            .unwrap_or(0);
        let locked_until = self
            .get(UNLOCK_LOCKED_UNTIL)?
            .as_deref()
            .map(datetime)
            .transpose()?;
        Ok(UnlockAttempts {
            consecutive_failures,
            locked_until,
        })
    }

    fn set_unlock_attempts(&self, attempts: UnlockAttempts) -> StorageResult<()> {
        self.set(
            UNLOCK_FAILURES,
            Some(attempts.consecutive_failures.to_string()),
        )?;
        self.set(
            UNLOCK_LOCKED_UNTIL,
            attempts.locked_until.map(|at| at.to_rfc3339()),
        )
    }

    fn biometric_enabled(&self) -> StorageResult<bool> {
        Ok(self.get(BIOMETRIC_ENABLED)?.as_deref() == Some("true"))
    }

    fn set_biometric_enabled(&self, enabled: bool) -> StorageResult<()> {
        self.set(BIOMETRIC_ENABLED, Some(enabled.to_string()))
    }

    fn onboarding_done(&self) -> StorageResult<bool> {
        Ok(self.get(ONBOARDING_DONE)?.as_deref() == Some("true"))
    }

    fn set_onboarding_done(&self, done: bool) -> StorageResult<()> {
        self.set(ONBOARDING_DONE, Some(done.to_string()))
    }

    fn twelve_data_api_key(&self) -> StorageResult<Option<String>> {
        self.get(TWELVE_DATA_API_KEY)
    }

    fn set_twelve_data_api_key(&self, key: Option<&str>) -> StorageResult<()> {
        self.set(TWELVE_DATA_API_KEY, key.map(str::to_string))
    }

    fn last_export_at(&self) -> StorageResult<Option<DateTime<Utc>>> {
        self.get(LAST_EXPORT_AT)?
            .as_deref()
            .map(datetime)
            .transpose()
    }

    fn set_last_export_at(&self, at: DateTime<Utc>) -> StorageResult<()> {
        self.set(LAST_EXPORT_AT, Some(at.to_rfc3339()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc_domain::NaiveDate;

    fn store() -> SqliteSettingsStore {
        SqliteSettingsStore::new(Database::open_in_memory().unwrap())
    }

    #[test]
    fn defaults_when_nothing_is_stored() {
        let store = store();
        assert_eq!(store.pin_hash().unwrap(), None);
        assert_eq!(store.unlock_attempts().unwrap(), UnlockAttempts::default());
        assert!(!store.biometric_enabled().unwrap());
        assert!(!store.onboarding_done().unwrap());
        assert_eq!(store.twelve_data_api_key().unwrap(), None);
        assert_eq!(store.last_export_at().unwrap(), None);
    }

    #[test]
    fn roundtrips_every_setting() {
        let store = store();
        let at = NaiveDate::from_ymd_opt(2026, 10, 20)
            .unwrap()
            .and_hms_opt(9, 42, 30)
            .unwrap()
            .and_utc();
        let attempts = UnlockAttempts {
            consecutive_failures: 6,
            locked_until: Some(at),
        };

        store.set_pin_hash("$argon2id$hash").unwrap();
        store.set_unlock_attempts(attempts).unwrap();
        store.set_biometric_enabled(true).unwrap();
        store.set_onboarding_done(true).unwrap();
        store.set_twelve_data_api_key(Some("key")).unwrap();
        store.set_last_export_at(at).unwrap();

        assert_eq!(store.pin_hash().unwrap().as_deref(), Some("$argon2id$hash"));
        assert_eq!(store.unlock_attempts().unwrap(), attempts);
        assert!(store.biometric_enabled().unwrap());
        assert!(store.onboarding_done().unwrap());
        assert_eq!(store.twelve_data_api_key().unwrap().as_deref(), Some("key"));
        assert_eq!(store.last_export_at().unwrap(), Some(at));

        store.set_twelve_data_api_key(None).unwrap();
        store
            .set_unlock_attempts(UnlockAttempts::default())
            .unwrap();
        assert_eq!(store.twelve_data_api_key().unwrap(), None);
        assert_eq!(store.unlock_attempts().unwrap(), UnlockAttempts::default());
    }
}
