use lc_application::ports::{Dataset, DatasetStore, StorageResult};

use super::accounts::save_account;
use super::budget::{save_category, save_charge, save_expense, save_one_off_income, save_salary};
use super::snapshots::save_snapshot;
use super::transfers::{save_external_account, save_transfer};
use super::{Database, SqlResultExt};

/// Remplacement de toutes les données métier (import), en une seule transaction. Les réglages
/// (PIN, clé API) et le cache des cours sont conservés.
pub struct SqliteDatasetStore {
    db: Database,
}

impl SqliteDatasetStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl DatasetStore for SqliteDatasetStore {
    fn replace_all(&self, data: &Dataset) -> StorageResult<()> {
        self.db.write(|tx| {
            tx.execute_batch(
                "DELETE FROM transfers;
                 DELETE FROM external_accounts;
                 DELETE FROM snapshot_lines;
                 DELETE FROM snapshots;
                 DELETE FROM positions;
                 DELETE FROM pei_funds;
                 DELETE FROM accounts;
                 DELETE FROM expenses;
                 DELETE FROM recurring_charges;
                 DELETE FROM one_off_incomes;
                 DELETE FROM salaries;
                 DELETE FROM categories;",
            )
            .db()?;
            for category in &data.categories {
                save_category(tx, category)?;
            }
            for salary in &data.salaries {
                save_salary(tx, salary)?;
            }
            for income in &data.one_off_incomes {
                save_one_off_income(tx, income)?;
            }
            for charge in &data.charges {
                save_charge(tx, charge)?;
            }
            for expense in &data.expenses {
                save_expense(tx, expense)?;
            }
            for account in &data.accounts {
                save_account(tx, account)?;
            }
            for snapshot in &data.snapshots {
                save_snapshot(tx, snapshot)?;
            }
            for account in &data.external_accounts {
                save_external_account(tx, account)?;
            }
            for transfer in &data.transfers {
                save_transfer(tx, transfer)?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::{SqliteAccountRepository, SqliteBudgetRepository, SqliteSettingsStore};
    use lc_application::ports::{AccountRepository, BudgetRepository, SettingsStore};
    use lc_domain::account::{Account, AccountType};
    use lc_domain::budget::{Category, Expense};
    use lc_domain::{AccountId, CategoryId, ExpenseId, Label, Money, NaiveDate, defaults};
    use rust_decimal_macros::dec;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()
    }

    fn dataset_with(categories: Vec<Category>, expenses: Vec<Expense>) -> Dataset {
        let account = Account::open(
            AccountId::generate(),
            Label::new("Livret A").unwrap(),
            defaults::color(AccountType::LivretA),
            AccountType::LivretA,
            today(),
        );
        Dataset {
            categories,
            expenses,
            accounts: vec![account],
            ..Dataset::default()
        }
    }

    fn expense(category: CategoryId) -> Expense {
        Expense::new(
            ExpenseId::generate(),
            Money::new(dec!(42)),
            category,
            today(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn replaces_everything_but_keeps_settings() {
        let db = Database::open_in_memory().unwrap();
        let store = SqliteDatasetStore::new(db.clone());
        let settings = SqliteSettingsStore::new(db.clone());
        settings.set_pin_hash("hash").unwrap();
        let old = Category::new(CategoryId::generate(), Label::new("Ancienne").unwrap());
        store
            .replace_all(&dataset_with(vec![old.clone()], vec![expense(old.id())]))
            .unwrap();

        let new = Category::new(CategoryId::generate(), Label::new("Nouvelle").unwrap());
        let replacement = dataset_with(vec![new.clone()], vec![]);
        store.replace_all(&replacement).unwrap();

        assert_eq!(
            SqliteBudgetRepository::new(db.clone())
                .categories()
                .unwrap(),
            vec![new]
        );
        assert!(
            SqliteBudgetRepository::new(db.clone())
                .expenses()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            SqliteAccountRepository::new(db.clone()).list().unwrap(),
            replacement.accounts
        );
        assert_eq!(settings.pin_hash().unwrap().as_deref(), Some("hash"));
    }

    #[test]
    fn failure_midway_leaves_previous_data_untouched() {
        let db = Database::open_in_memory().unwrap();
        let store = SqliteDatasetStore::new(db.clone());
        let kept = Category::new(CategoryId::generate(), Label::new("Conservée").unwrap());
        let before = dataset_with(vec![kept.clone()], vec![expense(kept.id())]);
        store.replace_all(&before).unwrap();

        // Dépense rattachée à une catégorie absente : la contrainte de clé étrangère échoue.
        let broken = dataset_with(vec![], vec![expense(CategoryId::generate())]);
        assert!(store.replace_all(&broken).is_err());

        let budget = SqliteBudgetRepository::new(db.clone());
        assert_eq!(budget.categories().unwrap(), vec![kept]);
        assert_eq!(budget.expenses().unwrap(), before.expenses);
        assert_eq!(
            SqliteAccountRepository::new(db).list().unwrap(),
            before.accounts
        );
    }
}
