//! Fausses implémentations en mémoire des ports, pour tester les cas d'usage sans I/O.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use lc_domain::account::{Account, AssetRef, CurrencyCode, QuoteSource};
use lc_domain::budget::{Category, Expense, OneOffIncome, RecurringCharge, SalaryEntry};
use lc_domain::valuation::{Quote, QuoteFreshness};
use lc_domain::{
    AccountId, CategoryId, ChargeId, Decimal, ExpenseId, IncomeId, Money, SalaryId, Snapshot,
};

use crate::accounts::AccountUseCases;
use crate::backup::{BackupUseCases, FullDataset, SCHEMA_VERSION};
use crate::budget::BudgetUseCases;
use crate::dashboard::DashboardUseCases;
use crate::ports::*;
use crate::pricing::PricingService;
use crate::projection::ProjectionUseCases;
use crate::security::SecurityUseCases;

pub fn asset(source: QuoteSource, symbol: &str) -> AssetRef {
    AssetRef::new(source, symbol, "", CurrencyCode::eur()).unwrap()
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().expect("verrou de test empoisonné")
}

fn upsert<T: Clone>(items: &Mutex<Vec<T>>, item: &T, same: impl Fn(&T) -> bool) {
    let mut items = lock(items);
    match items.iter_mut().find(|existing| same(existing)) {
        Some(existing) => *existing = item.clone(),
        None => items.push(item.clone()),
    }
}

fn remove<T>(items: &Mutex<Vec<T>>, matches: impl Fn(&T) -> bool) -> bool {
    let mut items = lock(items);
    let before = items.len();
    items.retain(|item| !matches(item));
    items.len() != before
}

// --- Horloge -------------------------------------------------------------------------------

pub struct FixedClock(Mutex<DateTime<Utc>>);

impl FixedClock {
    /// 20 octobre 2026, 09:42 UTC.
    fn new() -> Self {
        Self(Mutex::new(Self::at(
            NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
        )))
    }

    fn at(date: NaiveDate) -> DateTime<Utc> {
        date.and_hms_opt(9, 42, 0).unwrap().and_utc()
    }

    pub fn advance(&self, duration: Duration) {
        *lock(&self.0) += duration;
    }

    pub fn set_date(&self, date: NaiveDate) {
        *lock(&self.0) = Self::at(date);
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        *lock(&self.0)
    }

    fn today(&self) -> NaiveDate {
        self.now().date_naive()
    }
}

// --- Dépôts ---------------------------------------------------------------------------------

#[derive(Default)]
pub struct InMemoryAccounts(Mutex<Vec<Account>>);

impl AccountRepository for InMemoryAccounts {
    fn list(&self) -> StorageResult<Vec<Account>> {
        Ok(lock(&self.0).clone())
    }

    fn get(&self, id: AccountId) -> StorageResult<Option<Account>> {
        Ok(lock(&self.0).iter().find(|a| a.id() == id).cloned())
    }

    fn save(&self, account: &Account) -> StorageResult<()> {
        upsert(&self.0, account, |a| a.id() == account.id());
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemoryBudget {
    categories: Mutex<Vec<Category>>,
    salaries: Mutex<Vec<SalaryEntry>>,
    incomes: Mutex<Vec<OneOffIncome>>,
    charges: Mutex<Vec<RecurringCharge>>,
    expenses: Mutex<Vec<Expense>>,
}

impl BudgetRepository for InMemoryBudget {
    fn categories(&self) -> StorageResult<Vec<Category>> {
        Ok(lock(&self.categories).clone())
    }
    fn save_category(&self, category: &Category) -> StorageResult<()> {
        upsert(&self.categories, category, |c| c.id() == category.id());
        Ok(())
    }
    fn delete_category(&self, id: CategoryId) -> StorageResult<bool> {
        Ok(remove(&self.categories, |c| c.id() == id))
    }

    fn salaries(&self) -> StorageResult<Vec<SalaryEntry>> {
        Ok(lock(&self.salaries).clone())
    }
    fn save_salary(&self, salary: &SalaryEntry) -> StorageResult<()> {
        upsert(&self.salaries, salary, |s| s.id() == salary.id());
        Ok(())
    }
    fn delete_salary(&self, id: SalaryId) -> StorageResult<bool> {
        Ok(remove(&self.salaries, |s| s.id() == id))
    }

    fn one_off_incomes(&self) -> StorageResult<Vec<OneOffIncome>> {
        Ok(lock(&self.incomes).clone())
    }
    fn save_one_off_income(&self, income: &OneOffIncome) -> StorageResult<()> {
        upsert(&self.incomes, income, |i| i.id() == income.id());
        Ok(())
    }
    fn delete_one_off_income(&self, id: IncomeId) -> StorageResult<bool> {
        Ok(remove(&self.incomes, |i| i.id() == id))
    }

    fn charges(&self) -> StorageResult<Vec<RecurringCharge>> {
        Ok(lock(&self.charges).clone())
    }
    fn save_charge(&self, charge: &RecurringCharge) -> StorageResult<()> {
        upsert(&self.charges, charge, |c| c.id() == charge.id());
        Ok(())
    }
    fn delete_charge(&self, id: ChargeId) -> StorageResult<bool> {
        Ok(remove(&self.charges, |c| c.id() == id))
    }

    fn expenses(&self) -> StorageResult<Vec<Expense>> {
        Ok(lock(&self.expenses).clone())
    }
    fn save_expense(&self, expense: &Expense) -> StorageResult<()> {
        upsert(&self.expenses, expense, |e| e.id() == expense.id());
        Ok(())
    }
    fn delete_expense(&self, id: ExpenseId) -> StorageResult<bool> {
        Ok(remove(&self.expenses, |e| e.id() == id))
    }
}

#[derive(Default)]
pub struct InMemorySnapshots(Mutex<Vec<Snapshot>>);

impl SnapshotRepository for InMemorySnapshots {
    fn list(&self) -> StorageResult<Vec<Snapshot>> {
        Ok(lock(&self.0).clone())
    }

    fn save(&self, snapshot: &Snapshot) -> StorageResult<()> {
        upsert(&self.0, snapshot, |s| s.id() == snapshot.id());
        Ok(())
    }
}

#[derive(Default)]
pub struct InMemoryPriceCache(Mutex<Vec<Quote>>);

impl PriceCache for InMemoryPriceCache {
    fn get(&self, asset: &AssetRef) -> StorageResult<Option<Quote>> {
        Ok(lock(&self.0)
            .iter()
            .find(|q| q.asset().is_same_asset(asset))
            .cloned())
    }

    fn put(&self, quote: &Quote) -> StorageResult<()> {
        upsert(&self.0, quote, |q| q.asset().is_same_asset(quote.asset()));
        Ok(())
    }
}

// --- Cours ----------------------------------------------------------------------------------

pub struct FakeProvider {
    source: QuoteSource,
    clock: Arc<FixedClock>,
    prices: Mutex<HashMap<String, Decimal>>,
    failure: Mutex<Option<PriceError>>,
    uncovered: Mutex<Vec<String>>,
    catalog: Vec<AssetRef>,
    pub calls: AtomicUsize,
}

impl FakeProvider {
    fn new(source: QuoteSource, clock: Arc<FixedClock>, catalog: Vec<AssetRef>) -> Self {
        Self {
            source,
            clock,
            prices: Mutex::default(),
            failure: Mutex::default(),
            uncovered: Mutex::default(),
            catalog,
            calls: AtomicUsize::new(0),
        }
    }

    pub fn set_price(&self, symbol: &str, price: Decimal) {
        let symbol = asset(self.source, symbol).symbol().to_string();
        lock(&self.prices).insert(symbol, price);
        *lock(&self.failure) = None;
    }

    pub fn fail_with(&self, error: PriceError) {
        *lock(&self.failure) = Some(error);
    }

    /// Simule un actif que la source refuse de coter (offre insuffisante).
    pub fn refuse(&self, symbol: &str) {
        let symbol = asset(self.source, symbol).symbol().to_string();
        lock(&self.uncovered).push(symbol);
    }
}

#[async_trait]
impl PriceProvider for FakeProvider {
    fn source(&self) -> QuoteSource {
        self.source
    }

    async fn quotes(&self, assets: &[AssetRef]) -> Result<QuoteBatch, PriceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = lock(&self.failure).clone() {
            return Err(error);
        }
        let prices = lock(&self.prices);
        let refused = lock(&self.uncovered);
        let uncovered = assets
            .iter()
            .filter(|a| refused.contains(&a.symbol().to_string()))
            .cloned()
            .collect();
        let quotes = assets
            .iter()
            .filter_map(|asset| {
                prices.get(asset.symbol()).map(|price| {
                    Quote::new(
                        asset.clone(),
                        Money::new(*price),
                        self.clock.now(),
                        QuoteFreshness::Live,
                    )
                })
            })
            .collect();
        Ok(QuoteBatch { quotes, uncovered })
    }

    async fn search(&self, query: &str) -> Result<Vec<AssetRef>, PriceError> {
        let query = query.to_lowercase();
        Ok(self
            .catalog
            .iter()
            .filter(|a| {
                a.symbol().to_lowercase().contains(&query)
                    || a.display_name().to_lowercase().contains(&query)
            })
            .cloned()
            .collect())
    }
}

// --- Sécurité et réglages -------------------------------------------------------------------

pub struct FakeHasher;

impl PinHasher for FakeHasher {
    fn hash(&self, pin: &str) -> StorageResult<String> {
        Ok(format!("hash::{}", pin.chars().rev().collect::<String>()))
    }

    fn verify(&self, pin: &str, hash: &str) -> bool {
        self.hash(pin).is_ok_and(|expected| expected == hash)
    }
}

#[derive(Default)]
pub struct InMemorySettings {
    pin_hash: Mutex<Option<String>>,
    attempts: Mutex<UnlockAttempts>,
    biometric: Mutex<bool>,
    api_key: Mutex<Option<String>>,
    last_export: Mutex<Option<DateTime<Utc>>>,
}

impl SettingsStore for InMemorySettings {
    fn pin_hash(&self) -> StorageResult<Option<String>> {
        Ok(lock(&self.pin_hash).clone())
    }
    fn set_pin_hash(&self, hash: &str) -> StorageResult<()> {
        *lock(&self.pin_hash) = Some(hash.to_string());
        Ok(())
    }
    fn unlock_attempts(&self) -> StorageResult<UnlockAttempts> {
        Ok(*lock(&self.attempts))
    }
    fn set_unlock_attempts(&self, attempts: UnlockAttempts) -> StorageResult<()> {
        *lock(&self.attempts) = attempts;
        Ok(())
    }
    fn biometric_enabled(&self) -> StorageResult<bool> {
        Ok(*lock(&self.biometric))
    }
    fn set_biometric_enabled(&self, enabled: bool) -> StorageResult<()> {
        *lock(&self.biometric) = enabled;
        Ok(())
    }
    fn twelve_data_api_key(&self) -> StorageResult<Option<String>> {
        Ok(lock(&self.api_key).clone())
    }
    fn set_twelve_data_api_key(&self, key: Option<&str>) -> StorageResult<()> {
        *lock(&self.api_key) = key.map(str::to_string);
        Ok(())
    }
    fn last_export_at(&self) -> StorageResult<Option<DateTime<Utc>>> {
        Ok(*lock(&self.last_export))
    }
    fn set_last_export_at(&self, at: DateTime<Utc>) -> StorageResult<()> {
        *lock(&self.last_export) = Some(at);
        Ok(())
    }
}

// --- Sauvegarde -----------------------------------------------------------------------------

pub struct JsonCodec;

impl BackupCodec for JsonCodec {
    fn encode(&self, data: &FullDataset) -> Result<Vec<u8>, BackupError> {
        serde_json::to_vec(data).map_err(|e| BackupError::Invalid(e.to_string()))
    }

    fn decode(&self, bytes: &[u8]) -> Result<FullDataset, BackupError> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|e| BackupError::Invalid(e.to_string()))?;
        let version = value["schema_version"].as_u64().unwrap_or(0) as u32;
        if version > SCHEMA_VERSION {
            return Err(BackupError::NewerVersion {
                found: version,
                supported: SCHEMA_VERSION,
            });
        }
        serde_json::from_value(value).map_err(|e| BackupError::Invalid(e.to_string()))
    }
}

pub struct InMemoryDatasetStore {
    accounts: Arc<InMemoryAccounts>,
    budget: Arc<InMemoryBudget>,
    snapshots: Arc<InMemorySnapshots>,
}

impl DatasetStore for InMemoryDatasetStore {
    fn replace_all(&self, data: &Dataset) -> StorageResult<()> {
        *lock(&self.accounts.0) = data.accounts.clone();
        *lock(&self.budget.categories) = data.categories.clone();
        *lock(&self.budget.salaries) = data.salaries.clone();
        *lock(&self.budget.incomes) = data.one_off_incomes.clone();
        *lock(&self.budget.charges) = data.charges.clone();
        *lock(&self.budget.expenses) = data.expenses.clone();
        *lock(&self.snapshots.0) = data.snapshots.clone();
        Ok(())
    }
}

// --- Environnement de test ------------------------------------------------------------------

pub struct TestEnv {
    pub clock: Arc<FixedClock>,
    pub accounts: Arc<InMemoryAccounts>,
    pub budget_repo: Arc<InMemoryBudget>,
    pub snapshots: Arc<InMemorySnapshots>,
    pub cache: Arc<InMemoryPriceCache>,
    pub settings: Arc<InMemorySettings>,
    pub coingecko: Arc<FakeProvider>,
    pub twelve_data: Arc<FakeProvider>,
}

impl TestEnv {
    pub fn new() -> Self {
        let clock = Arc::new(FixedClock::new());
        let eur = CurrencyCode::eur;
        let catalog = |source, entries: &[(&str, &str)]| {
            entries
                .iter()
                .map(|(symbol, name)| AssetRef::new(source, symbol, name, eur()).unwrap())
                .collect()
        };
        Self {
            coingecko: Arc::new(FakeProvider::new(
                QuoteSource::CoinGecko,
                clock.clone(),
                catalog(
                    QuoteSource::CoinGecko,
                    &[("bitcoin", "Bitcoin"), ("ethereum", "Ethereum")],
                ),
            )),
            twelve_data: Arc::new(FakeProvider::new(
                QuoteSource::TwelveData,
                clock.clone(),
                catalog(
                    QuoteSource::TwelveData,
                    &[
                        ("CW8", "Amundi MSCI World"),
                        ("IWDA", "iShares Core MSCI World"),
                    ],
                ),
            )),
            clock,
            accounts: Arc::default(),
            budget_repo: Arc::default(),
            snapshots: Arc::default(),
            cache: Arc::default(),
            settings: Arc::default(),
        }
    }

    pub fn pricing(&self) -> Arc<PricingService> {
        Arc::new(PricingService::new(
            vec![self.coingecko.clone(), self.twelve_data.clone()],
            self.cache.clone(),
            self.clock.clone(),
        ))
    }

    pub fn security(&self) -> SecurityUseCases {
        SecurityUseCases::new(
            self.settings.clone(),
            Arc::new(FakeHasher),
            self.clock.clone(),
        )
    }

    pub fn budget(&self) -> BudgetUseCases {
        BudgetUseCases::new(self.budget_repo.clone(), self.clock.clone())
    }

    pub fn accounts(&self) -> AccountUseCases {
        AccountUseCases::new(self.accounts.clone(), self.pricing(), self.clock.clone())
    }

    pub fn dashboard(&self) -> DashboardUseCases {
        DashboardUseCases::new(
            self.accounts.clone(),
            self.budget_repo.clone(),
            self.snapshots.clone(),
            self.pricing(),
            self.clock.clone(),
        )
    }

    pub fn projection(&self) -> ProjectionUseCases {
        ProjectionUseCases::new(
            self.accounts.clone(),
            self.budget_repo.clone(),
            self.pricing(),
            self.clock.clone(),
        )
    }

    pub fn backup(&self) -> BackupUseCases {
        BackupUseCases::new(
            self.accounts.clone(),
            self.budget_repo.clone(),
            self.snapshots.clone(),
            self.settings.clone(),
            Arc::new(InMemoryDatasetStore {
                accounts: self.accounts.clone(),
                budget: self.budget_repo.clone(),
                snapshots: self.snapshots.clone(),
            }),
            Arc::new(JsonCodec),
            self.clock.clone(),
        )
    }
}
