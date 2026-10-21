//! Infrastructure : implémentations concrètes des ports définis dans `lc-application`.

mod backup;
mod clock;
mod pricing;
mod security;
mod sqlite;
mod updates;

pub use backup::JsonBackupCodec;
pub use clock::SystemClock;
pub use pricing::{CoinGeckoProvider, TwelveDataProvider, YahooFinanceProvider, http_client};
pub use security::Argon2PinHasher;
pub use sqlite::{
    Database, SqliteAccountRepository, SqliteBudgetRepository, SqliteDatasetStore,
    SqlitePriceCache, SqliteSettingsStore, SqliteSnapshotRepository, SqliteTransferRepository,
};
pub use updates::{GithubReleaseFeed, HttpPackageDownloader, download_client};
