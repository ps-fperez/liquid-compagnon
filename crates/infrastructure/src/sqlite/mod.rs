//! Persistance SQLite : connexion unique, migrations embarquées (`PRAGMA user_version`),
//! dépôts implémentant les ports de `lc-application`.

mod accounts;
mod budget;
mod dataset;
mod price_cache;
mod settings;
mod snapshots;

use std::fmt::Display;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, NaiveDate, Utc};
use lc_application::ports::{StorageError, StorageResult};
use lc_domain::account::QuoteSource;
use lc_domain::{Decimal, DomainError, Money, Month, Uuid};
use rusqlite::{Connection, Transaction};

pub use accounts::SqliteAccountRepository;
pub use budget::SqliteBudgetRepository;
pub use dataset::SqliteDatasetStore;
pub use price_cache::SqlitePriceCache;
pub use settings::SqliteSettingsStore;
pub use snapshots::SqliteSnapshotRepository;

/// Migrations appliquées dans l'ordre ; l'index + 1 est la version de schéma atteinte.
const MIGRATIONS: &[&str] = &[include_str!("../../migrations/001_initial.sql")];

/// Base de données de l'application, partagée entre les dépôts.
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Ouvre (ou crée) la base et applique les migrations manquantes.
    pub fn open(path: &Path) -> StorageResult<Self> {
        let conn = Connection::open(path).db()?;
        conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))
            .db()?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> StorageResult<Self> {
        Self::init(Connection::open_in_memory().db()?)
    }

    fn init(mut conn: Connection) -> StorageResult<Self> {
        conn.pragma_update(None, "foreign_keys", true).db()?;
        migrate(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn schema_version(&self) -> StorageResult<u32> {
        self.read(|conn| {
            conn.pragma_query_value(None, "user_version", |row| row.get(0))
                .db()
        })
    }

    pub(crate) fn read<T>(
        &self,
        query: impl FnOnce(&Connection) -> StorageResult<T>,
    ) -> StorageResult<T> {
        let conn = self.lock()?;
        query(&conn)
    }

    /// Exécute `change` dans une transaction : tout est validé ou rien ne l'est.
    pub(crate) fn write<T>(
        &self,
        change: impl FnOnce(&Transaction<'_>) -> StorageResult<T>,
    ) -> StorageResult<T> {
        let mut conn = self.lock()?;
        let tx = conn.transaction().db()?;
        let value = change(&tx)?;
        tx.commit().db()?;
        Ok(value)
    }

    fn lock(&self) -> StorageResult<MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| {
            StorageError("base de données inutilisable après une erreur interne".into())
        })
    }
}

fn migrate(conn: &mut Connection) -> StorageResult<()> {
    let current: u32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .db()?;
    if current as usize > MIGRATIONS.len() {
        return Err(StorageError(format!(
            "base créée par une version plus récente de l'application (schéma {current})"
        )));
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction().db()?;
        tx.execute_batch(sql).db()?;
        tx.pragma_update(None, "user_version", (index + 1) as u32)
            .db()?;
        tx.commit().db()?;
    }
    Ok(())
}

// --- Conversions --------------------------------------------------------------------------

/// Conversion des erreurs SQLite en erreurs de stockage.
pub(crate) trait SqlResultExt<T> {
    fn db(self) -> StorageResult<T>;
}

impl<T> SqlResultExt<T> for rusqlite::Result<T> {
    fn db(self) -> StorageResult<T> {
        self.map_err(|error| StorageError(format!("SQLite : {error}")))
    }
}

pub(crate) fn corrupt(error: impl Display) -> StorageError {
    StorageError(format!("donnée invalide en base : {error}"))
}

pub(crate) fn domain<T>(result: Result<T, DomainError>) -> StorageResult<T> {
    result.map_err(corrupt)
}

pub(crate) fn decimal(raw: &str) -> StorageResult<Decimal> {
    Decimal::from_str(raw).map_err(corrupt)
}

pub(crate) fn money(raw: &str) -> StorageResult<Money> {
    decimal(raw).map(Money::new)
}

pub(crate) fn optional_money(raw: Option<String>) -> StorageResult<Option<Money>> {
    raw.as_deref().map(money).transpose()
}

pub(crate) fn date(raw: &str) -> StorageResult<NaiveDate> {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(corrupt)
}

pub(crate) fn datetime(raw: &str) -> StorageResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|value| value.with_timezone(&Utc))
        .map_err(corrupt)
}

pub(crate) fn month(raw: &str) -> StorageResult<Month> {
    domain(raw.parse())
}

pub(crate) fn uuid(raw: &str) -> StorageResult<Uuid> {
    Uuid::parse_str(raw).map_err(corrupt)
}

pub(crate) fn source_to_sql(source: QuoteSource) -> &'static str {
    match source {
        QuoteSource::CoinGecko => "coingecko",
        QuoteSource::TwelveData => "twelve_data",
        QuoteSource::Yahoo => "yahoo",
    }
}

pub(crate) fn source_from_sql(raw: &str) -> StorageResult<QuoteSource> {
    match raw {
        "coingecko" => Ok(QuoteSource::CoinGecko),
        "twelve_data" => Ok(QuoteSource::TwelveData),
        "yahoo" => Ok(QuoteSource::Yahoo),
        other => Err(corrupt(format!("source de cotation inconnue « {other} »"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_database_is_migrated_to_latest_schema() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap() as usize, MIGRATIONS.len());
    }

    #[test]
    fn reopening_a_file_keeps_data_and_does_not_reapply_migrations() {
        let path = std::env::temp_dir().join(format!("lc-test-{}.db", Uuid::new_v4()));
        {
            let db = Database::open(&path).unwrap();
            db.write(|tx| {
                tx.execute("INSERT INTO settings (key, value) VALUES ('k', 'v')", [])
                    .db()
            })
            .unwrap();
        }
        let reopened = Database::open(&path).unwrap();
        let value: String = reopened
            .read(|conn| {
                conn.query_row("SELECT value FROM settings WHERE key = 'k'", [], |r| {
                    r.get(0)
                })
                .db()
            })
            .unwrap();
        assert_eq!(value, "v");
        assert_eq!(
            reopened.schema_version().unwrap() as usize,
            MIGRATIONS.len()
        );
        drop(reopened);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
    }

    #[test]
    fn refuses_database_from_newer_application() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        assert!(migrate(&mut conn).is_err());
    }

    #[test]
    fn failed_write_is_rolled_back() {
        let db = Database::open_in_memory().unwrap();
        let result: StorageResult<()> = db.write(|tx| {
            tx.execute("INSERT INTO settings (key, value) VALUES ('k', 'v')", [])
                .db()?;
            Err(StorageError("échec simulé".into()))
        });
        assert!(result.is_err());
        let count: i64 = db
            .read(|conn| {
                conn.query_row("SELECT COUNT(*) FROM settings", [], |r| r.get(0))
                    .db()
            })
            .unwrap();
        assert_eq!(count, 0);
    }
}
