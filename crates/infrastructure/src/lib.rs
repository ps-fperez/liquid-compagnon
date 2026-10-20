//! Infrastructure : implémentations concrètes des ports définis dans `lc-application`.

mod backup;
mod clock;
mod pricing;
mod security;
mod sqlite;

pub use backup::JsonBackupCodec;
pub use clock::SystemClock;
pub use pricing::{CoinGeckoProvider, TwelveDataProvider, http_client};
pub use security::Argon2PinHasher;
pub use sqlite::{
    Database, SqliteAccountRepository, SqliteBudgetRepository, SqliteDatasetStore,
    SqlitePriceCache, SqliteSettingsStore, SqliteSnapshotRepository,
};
