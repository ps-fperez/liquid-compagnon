//! Ports : interfaces que l'infrastructure implémente (stockage, cours, horloge, sauvegarde).
//! Petites et ciblées (ISP) ; les cas d'usage n'en connaissent que l'abstraction (DIP).

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use lc_domain::account::{Account, AssetRef, QuoteSource};
use lc_domain::budget::{Category, Expense, OneOffIncome, RecurringCharge, SalaryEntry};
use lc_domain::transfer::{ExternalAccount, Transfer};
use lc_domain::valuation::Quote;
use lc_domain::{
    AccountId, CategoryId, ChargeId, ExpenseId, ExternalId, IncomeId, SalaryId, Snapshot,
    TransferId,
};

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

/// Virements et comptes hors patrimoine. Les suppressions renvoient `false` si l'élément
/// n'existait pas.
pub trait TransferRepository: Send + Sync {
    fn transfers(&self) -> StorageResult<Vec<Transfer>>;
    fn save_transfer(&self, transfer: &Transfer) -> StorageResult<()>;
    fn delete_transfer(&self, id: TransferId) -> StorageResult<bool>;

    fn external_accounts(&self) -> StorageResult<Vec<ExternalAccount>>;
    fn save_external_account(&self, account: &ExternalAccount) -> StorageResult<()>;
    fn delete_external_account(&self, id: ExternalId) -> StorageResult<bool>;
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

/// Résultat d'une demande de cours : les cours obtenus et les actifs que la source ne couvre pas
/// (offre insuffisante, symbole inconnu), à signaler plutôt qu'à ignorer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuoteBatch {
    pub quotes: Vec<Quote>,
    pub uncovered: Vec<AssetRef>,
}

/// Source de cotation (CoinGecko, Twelve Data, Yahoo Finance). Une requête ne transmet que des
/// identifiants d'actifs, jamais de montant ni de quantité.
#[async_trait]
pub trait PriceProvider: Send + Sync {
    fn source(&self) -> QuoteSource;
    /// Cours en euros des actifs demandés, en une requête groupée si possible. Les actifs que la
    /// source ne peut pas coter sont listés dans `uncovered`.
    async fn quotes(&self, assets: &[AssetRef]) -> Result<QuoteBatch, PriceError>;
    async fn search(&self, query: &str) -> Result<Vec<AssetRef>, PriceError>;
    /// État du compte auprès de la source (ex. offre associée à la clé), si elle en a un.
    async fn account_status(&self) -> Result<Option<String>, PriceError> {
        Ok(None)
    }
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
    /// Didacticiel de prise en main déjà parcouru (ou passé).
    fn onboarding_done(&self) -> StorageResult<bool>;
    fn set_onboarding_done(&self, done: bool) -> StorageResult<()>;
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
    pub transfers: Vec<Transfer>,
    pub external_accounts: Vec<ExternalAccount>,
}

/// Remplacement de toutes les données en une transaction (import).
pub trait DatasetStore: Send + Sync {
    fn replace_all(&self, data: &Dataset) -> StorageResult<()>;
}
