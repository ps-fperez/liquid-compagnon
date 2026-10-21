//! Cas d'usage réels branchés sur les vrais adaptateurs (SQLite sur fichier, Argon2, JSON).

use std::path::PathBuf;
use std::sync::Arc;

use lc_application::AppError;
use lc_application::accounts::{AccountUseCases, NewAccount};
use lc_application::backup::BackupUseCases;
use lc_application::budget::{BudgetUseCases, ExpenseInput};
use lc_application::dashboard::DashboardUseCases;
use lc_application::ports::{AccountRepository, BudgetRepository, Clock, SnapshotRepository};
use lc_application::pricing::PricingService;
use lc_application::security::SecurityUseCases;
use lc_domain::account::AccountType;
use lc_domain::{Money, Month};
use lc_infrastructure::{
    Argon2PinHasher, Database, JsonBackupCodec, SqliteAccountRepository, SqliteBudgetRepository,
    SqliteDatasetStore, SqlitePlanningRepository, SqlitePriceCache, SqliteSettingsStore,
    SqliteSnapshotRepository, SqliteTransferRepository, SystemClock,
};
use rust_decimal_macros::dec;

/// Application complète sur une base fichier temporaire (supprimée à la fin du test).
struct App {
    path: PathBuf,
    accounts: Arc<SqliteAccountRepository>,
    budget: Arc<SqliteBudgetRepository>,
    transfers: Arc<SqliteTransferRepository>,
    planning: Arc<SqlitePlanningRepository>,
    snapshots: Arc<SqliteSnapshotRepository>,
    settings: Arc<SqliteSettingsStore>,
    dataset: Arc<SqliteDatasetStore>,
    pricing: Arc<PricingService>,
    clock: Arc<dyn Clock>,
}

impl App {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("lc-e2e-{}.db", uuid::Uuid::new_v4()));
        let db = Database::open(&path).unwrap();
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        Self {
            path,
            accounts: Arc::new(SqliteAccountRepository::new(db.clone())),
            budget: Arc::new(SqliteBudgetRepository::new(db.clone())),
            transfers: Arc::new(SqliteTransferRepository::new(db.clone())),
            planning: Arc::new(SqlitePlanningRepository::new(db.clone())),
            snapshots: Arc::new(SqliteSnapshotRepository::new(db.clone())),
            settings: Arc::new(SqliteSettingsStore::new(db.clone())),
            dataset: Arc::new(SqliteDatasetStore::new(db.clone())),
            pricing: Arc::new(PricingService::new(
                Vec::new(),
                Arc::new(SqlitePriceCache::new(db)),
                clock.clone(),
            )),
            clock,
        }
    }

    fn security(&self) -> SecurityUseCases {
        SecurityUseCases::new(
            self.settings.clone(),
            Arc::new(Argon2PinHasher),
            self.clock.clone(),
        )
    }

    fn budget(&self) -> BudgetUseCases {
        BudgetUseCases::new(
            self.budget.clone(),
            self.accounts.clone(),
            self.transfers.clone(),
            self.clock.clone(),
        )
    }

    fn accounts(&self) -> AccountUseCases {
        AccountUseCases::new(
            self.accounts.clone(),
            self.budget.clone(),
            self.transfers.clone(),
            self.pricing.clone(),
            self.clock.clone(),
        )
    }

    fn dashboard(&self) -> DashboardUseCases {
        DashboardUseCases::new(
            self.accounts.clone(),
            self.budget.clone(),
            self.transfers.clone(),
            self.snapshots.clone(),
            self.pricing.clone(),
            self.clock.clone(),
        )
    }

    fn backup(&self) -> BackupUseCases {
        BackupUseCases::new(
            self.accounts.clone(),
            self.budget.clone(),
            self.transfers.clone(),
            self.planning.clone(),
            self.snapshots.clone(),
            self.settings.clone(),
            self.dataset.clone(),
            Arc::new(JsonBackupCodec),
            self.clock.clone(),
        )
    }
}

impl Drop for App {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.path.display()));
        }
    }
}

#[tokio::test]
async fn full_journey_from_setup_to_restored_backup() {
    let phone = App::new();

    // Premier lancement : PIN (Argon2) et catégories par défaut.
    phone.security().setup_pin("482916", "482916").unwrap();
    assert!(phone.security().unlock("482916").is_ok());
    assert!(matches!(
        phone.security().unlock("000000"),
        Err(AppError::WrongPin { .. })
    ));
    phone.budget().ensure_default_categories().unwrap();

    // Budget et comptes.
    let food = phone.budget().categories(false).unwrap()[2].id();
    phone
        .budget()
        .upsert_salary(None, Month::of(phone.clock.today()), Money::new(dec!(3200)))
        .unwrap();
    // Compte courant : solde de référence, puis une dépense rattachée saisie après.
    let checking = phone
        .accounts()
        .create_account(NewAccount {
            name: "Compte courant".into(),
            account_type: AccountType::Checking,
            color: None,
        })
        .unwrap();
    phone
        .accounts()
        .update_balance(checking, Money::new(dec!(1000)))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    phone
        .budget()
        .add_expense(ExpenseInput {
            amount: Money::new(dec!(42.50)),
            category: food,
            date: None,
            label: None,
            account: Some(checking),
        })
        .unwrap();
    let livret = phone
        .accounts()
        .create_account(NewAccount {
            name: "Livret A".into(),
            account_type: AccountType::LivretA,
            color: None,
        })
        .unwrap();
    phone
        .accounts()
        .update_balance(livret, Money::new(dec!(10640)))
        .unwrap();

    // Tableau de bord : total et snapshot automatique persistés.
    let dashboard = phone.dashboard().dashboard(false).await.unwrap();
    // 10 640 € (livret) + 1 000 € − 42,50 € (compte courant).
    assert_eq!(dashboard.total, Money::new(dec!(11597.50)));
    assert_eq!(dashboard.budget.expenses, Money::new(dec!(42.50)));
    assert_eq!(phone.snapshots.list().unwrap().len(), 1);

    // Sauvegarde puis restauration sur un « nouveau téléphone ».
    let bytes = phone.backup().export().unwrap();
    let json = String::from_utf8(bytes.clone()).unwrap();
    assert!(
        !json.contains("argon2"),
        "le hash du PIN ne doit pas être exporté"
    );

    let new_phone = App::new();
    new_phone.backup().import(&bytes).unwrap();

    assert_eq!(
        new_phone.accounts.list().unwrap(),
        phone.accounts.list().unwrap()
    );
    assert_eq!(
        new_phone.budget.expenses().unwrap(),
        phone.budget.expenses().unwrap()
    );
    assert_eq!(
        new_phone.budget.categories().unwrap(),
        phone.budget.categories().unwrap()
    );
    assert_eq!(
        new_phone.snapshots.list().unwrap(),
        phone.snapshots.list().unwrap()
    );
    assert!(
        !new_phone.security().is_pin_set().unwrap(),
        "le PIN reste propre à chaque appareil"
    );

    // Un fichier corrompu ne touche à rien.
    assert!(
        new_phone
            .backup()
            .import(b"{\"schema_version\": 1, \"oups\": true}")
            .is_err()
    );
    assert_eq!(new_phone.accounts.list().unwrap().len(), 2);
}
