//! Composition root : seul endroit où les adaptateurs concrets sont instanciés et injectés.

use std::path::Path;
use std::sync::Arc;

use lc_application::accounts::AccountUseCases;
use lc_application::backup::BackupUseCases;
use lc_application::budget::BudgetUseCases;
use lc_application::dashboard::DashboardUseCases;
use lc_application::goals::GoalUseCases;
use lc_application::ports::{Clock, PriceProvider};
use lc_application::pricing::PricingService;
use lc_application::projection::ProjectionUseCases;
use lc_application::review::ReviewUseCases;
use lc_application::security::SecurityUseCases;
use lc_application::transfers::TransferUseCases;
use lc_application::updates::UpdateService;
use lc_infrastructure::{
    Argon2PinHasher, CoinGeckoProvider, Database, GithubReleaseFeed, HttpPackageDownloader,
    JsonBackupCodec, SqliteAccountRepository, SqliteBudgetRepository, SqliteDatasetStore,
    SqlitePlanningRepository, SqlitePriceCache, SqliteSettingsStore, SqliteSnapshotRepository,
    SqliteTransferRepository, SystemClock, TwelveDataProvider, YahooFinanceProvider,
    download_client, http_client,
};

use crate::session::Session;
use crate::updater::{self, Updates};

/// Nom du fichier de base, dans le répertoire privé de l'application.
pub const DATABASE_FILE: &str = "liquid-compagnon.db";

pub struct Services {
    pub session: Session,
    pub security: SecurityUseCases,
    pub budget: BudgetUseCases,
    pub transfers: TransferUseCases,
    pub accounts: AccountUseCases,
    pub dashboard: DashboardUseCases,
    pub projection: ProjectionUseCases,
    pub goals: GoalUseCases,
    pub reviews: ReviewUseCases,
    pub backup: BackupUseCases,
    pub updates: Updates,
}

impl Services {
    pub fn compose(data_dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir)
            .map_err(|error| format!("répertoire de données : {error}"))?;
        let db =
            Database::open(&data_dir.join(DATABASE_FILE)).map_err(|error| error.to_string())?;

        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let accounts = Arc::new(SqliteAccountRepository::new(db.clone()));
        let budget = Arc::new(SqliteBudgetRepository::new(db.clone()));
        let transfers = Arc::new(SqliteTransferRepository::new(db.clone()));
        let planning = Arc::new(SqlitePlanningRepository::new(db.clone()));
        let snapshots = Arc::new(SqliteSnapshotRepository::new(db.clone()));
        let settings = Arc::new(SqliteSettingsStore::new(db.clone()));
        let http = http_client()?;
        let providers: Vec<Arc<dyn PriceProvider>> = vec![
            Arc::new(CoinGeckoProvider::new(http.clone(), clock.clone())),
            Arc::new(TwelveDataProvider::new(
                http.clone(),
                settings.clone(),
                clock.clone(),
            )),
            Arc::new(YahooFinanceProvider::new(http.clone(), clock.clone())),
        ];
        let pricing = Arc::new(PricingService::new(
            providers,
            Arc::new(SqlitePriceCache::new(db.clone())),
            clock.clone(),
        ));

        let updates = Updates::new(UpdateService::new(
            Box::new(GithubReleaseFeed::new(
                http,
                &format!("https://github.com/{}/releases", updater::REPOSITORY),
            )),
            Box::new(HttpPackageDownloader::new(download_client()?)),
            updater::build_info(),
            updater::REPOSITORY,
        ));

        let services = Self {
            session: Session::default(),
            updates,
            security: SecurityUseCases::new(
                settings.clone(),
                Arc::new(Argon2PinHasher),
                clock.clone(),
            ),
            budget: BudgetUseCases::new(
                budget.clone(),
                accounts.clone(),
                transfers.clone(),
                clock.clone(),
            ),
            transfers: TransferUseCases::new(transfers.clone(), accounts.clone(), clock.clone()),
            accounts: AccountUseCases::new(
                accounts.clone(),
                budget.clone(),
                transfers.clone(),
                pricing.clone(),
                clock.clone(),
            ),
            dashboard: DashboardUseCases::new(
                accounts.clone(),
                budget.clone(),
                transfers.clone(),
                snapshots.clone(),
                pricing.clone(),
                clock.clone(),
            ),
            projection: ProjectionUseCases::new(
                accounts.clone(),
                budget.clone(),
                transfers.clone(),
                pricing.clone(),
                clock.clone(),
            ),
            goals: GoalUseCases::new(
                planning.clone(),
                accounts.clone(),
                budget.clone(),
                transfers.clone(),
                pricing,
                clock.clone(),
            ),
            reviews: ReviewUseCases::new(
                planning.clone(),
                accounts.clone(),
                budget.clone(),
                transfers.clone(),
                snapshots.clone(),
                clock.clone(),
            ),
            backup: BackupUseCases::new(
                accounts,
                budget,
                transfers,
                planning,
                snapshots,
                settings,
                Arc::new(SqliteDatasetStore::new(db)),
                Arc::new(JsonBackupCodec),
                clock,
            ),
        };
        services
            .budget
            .ensure_default_categories()
            .map_err(|error| error.to_string())?;
        services
            .transfers
            .convert_scheduled_contributions()
            .map_err(|error| error.to_string())?;
        Ok(services)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composes_on_a_fresh_directory_with_default_categories() {
        let dir = std::env::temp_dir().join(format!("lc-compose-{}", lc_domain::Uuid::new_v4()));

        let services = Services::compose(&dir).unwrap();

        assert!(dir.join(DATABASE_FILE).exists());
        assert_eq!(services.budget.categories(false).unwrap().len(), 8);
        assert!(!services.security.is_pin_set().unwrap());
        assert!(!services.session.is_unlocked());
        drop(services);
        let _ = std::fs::remove_dir_all(dir);
    }
}
