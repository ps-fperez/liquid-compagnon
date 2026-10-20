//! Ports : interfaces que l'infrastructure implémente (stockage, cours, horloge, sauvegarde).
//! Petites et ciblées (ISP) ; les cas d'usage n'en connaissent que l'abstraction (DIP).

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use lc_domain::account::{Account, AssetRef, QuoteSource};
use lc_domain::budget::{Category, Expense, OneOffIncome, RecurringCharge, SalaryEntry};
use lc_domain::valuation::Quote;
use lc_domain::{AccountId, CategoryId, ChargeId, ExpenseId, IncomeId, SalaryId, Snapshot};

use crate::backup::FullDataset;

/// Échec d'un adaptateur de stockage (message technique, journalisé).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct StorageError(pub String);

pub type StorageResult<T> = Result<T, StorageError>;

/// Heure et date courantes (jamais lues directement dans le domaine ni les cas d'usage).
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
    /// Date du jour dans le fuseau de l'utilisateur.
    fn today(&self) -> NaiveDate;
}

pub trait AccountRepository: Send + Sync {
    /// Tous les comptes, archivés compris.
    fn list(&self) -> StorageResult<Vec<Account>>;
    fn get(&self, id: AccountId) -> StorageResult<Option<Account>>;
    /// Crée ou remplace le compte (avec ses positions et fonds).
    fn save(&self, account: &Account) -> StorageResult<()>;
}

/// Revenus, charges, dépenses et catégories. Les suppressions renvoient `false` si l'élément
/// n'existait pas.
pub trait BudgetRepository: Send + Sync {
    fn categories(&self) -> StorageResult<Vec<Category>>;
    fn save_category(&self, category: &Category) -> StorageResult<()>;
    fn delete_category(&self, id: CategoryId) -> StorageResult<bool>;

    fn salaries(&self) -> StorageResult<Vec<SalaryEntry>>;
    fn save_salary(&self, salary: &SalaryEntry) -> StorageResult<()>;
    fn delete_salary(&self, id: SalaryId) -> StorageResult<bool>;

    fn one_off_incomes(&self) -> StorageResult<Vec<OneOffIncome>>;
    fn save_one_off_income(&self, income: &OneOffIncome) -> StorageResult<()>;
    fn delete_one_off_income(&self, id: IncomeId) -> StorageResult<bool>;

    fn charges(&self) -> StorageResult<Vec<RecurringCharge>>;
    fn save_charge(&self, charge: &RecurringCharge) -> StorageResult<()>;
    fn delete_charge(&self, id: ChargeId) -> StorageResult<bool>;

    fn expenses(&self) -> StorageResult<Vec<Expense>>;
    fn save_expense(&self, expense: &Expense) -> StorageResult<()>;
    fn delete_expense(&self, id: ExpenseId) -> StorageResult<bool>;
}

pub trait SnapshotRepository: Send + Sync {
    fn list(&self) -> StorageResult<Vec<Snapshot>>;
    fn save(&self, snapshot: &Snapshot) -> StorageResult<()>;
}

/// Derniers cours connus, conservés localement.
pub trait PriceCache: Send + Sync {
    fn get(&self, asset: &AssetRef) -> StorageResult<Option<Quote>>;
    fn put(&self, quote: &Quote) -> StorageResult<()>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PriceError {
    #[error("limite de requêtes atteinte, réessayez plus tard")]
    RateLimited,
    #[error("clé API manquante")]
    MissingApiKey,
    #[error("source injoignable ({0})")]
    Unavailable(String),
    #[error("réponse inattendue ({0})")]
    InvalidResponse(String),
}

/// Source de cotation (CoinGecko, Twelve Data…). Une requête ne transmet que des identifiants
/// d'actifs, jamais de montant ni de quantité.
#[async_trait]
pub trait PriceProvider: Send + Sync {
    fn source(&self) -> QuoteSource;
    /// Cours en euros des actifs demandés, en une requête groupée si possible. Les actifs
    /// inconnus de la source sont simplement absents du résultat.
    async fn quotes(&self, assets: &[AssetRef]) -> Result<Vec<Quote>, PriceError>;
    async fn search(&self, query: &str) -> Result<Vec<AssetRef>, PriceError>;
}

/// Hachage du code PIN (Argon2id dans l'infrastructure).
pub trait PinHasher: Send + Sync {
    fn hash(&self, pin: &str) -> StorageResult<String>;
    fn verify(&self, pin: &str, hash: &str) -> bool;
}

/// Compteur d'échecs de déverrouillage, persisté pour résister à un redémarrage.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnlockAttempts {
    pub consecutive_failures: u32,
    pub locked_until: Option<DateTime<Utc>>,
}

/// Réglages et secrets locaux. Jamais exportés.
pub trait SettingsStore: Send + Sync {
    fn pin_hash(&self) -> StorageResult<Option<String>>;
    fn set_pin_hash(&self, hash: &str) -> StorageResult<()>;
    fn unlock_attempts(&self) -> StorageResult<UnlockAttempts>;
    fn set_unlock_attempts(&self, attempts: UnlockAttempts) -> StorageResult<()>;
    fn biometric_enabled(&self) -> StorageResult<bool>;
    fn set_biometric_enabled(&self, enabled: bool) -> StorageResult<()>;
    fn twelve_data_api_key(&self) -> StorageResult<Option<String>>;
    fn set_twelve_data_api_key(&self, key: Option<&str>) -> StorageResult<()>;
    fn last_export_at(&self) -> StorageResult<Option<DateTime<Utc>>>;
    fn set_last_export_at(&self, at: DateTime<Utc>) -> StorageResult<()>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BackupError {
    #[error("{0}")]
    Invalid(String),
    #[error("version {found} non supportée (maximum {supported})")]
    NewerVersion { found: u32, supported: u32 },
}

/// Format du fichier de sauvegarde (JSON versionné, migrations des versions antérieures).
pub trait BackupCodec: Send + Sync {
    fn encode(&self, data: &FullDataset) -> Result<Vec<u8>, BackupError>;
    fn decode(&self, bytes: &[u8]) -> Result<FullDataset, BackupError>;
}

/// Ensemble des données métier, validées.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dataset {
    pub categories: Vec<Category>,
    pub salaries: Vec<SalaryEntry>,
    pub one_off_incomes: Vec<OneOffIncome>,
    pub charges: Vec<RecurringCharge>,
    pub expenses: Vec<Expense>,
    pub accounts: Vec<Account>,
    pub snapshots: Vec<Snapshot>,
}

/// Remplacement de toutes les données en une transaction (import).
pub trait DatasetStore: Send + Sync {
    fn replace_all(&self, data: &Dataset) -> StorageResult<()>;
}
