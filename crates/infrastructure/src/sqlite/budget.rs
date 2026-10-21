use lc_application::ports::{BudgetRepository, StorageResult};
use lc_domain::budget::{
    Category, Debit, Expense, OneOffIncome, Periodicity, RecurringCharge, SalaryEntry,
};
use lc_domain::{AccountId, CategoryId, ChargeId, ExpenseId, IncomeId, Label, SalaryId};
use rusqlite::{Connection, Params, Row, params};

use super::{Database, SqlResultExt, corrupt, date, datetime, domain, money, month, uuid};

pub struct SqliteBudgetRepository {
    db: Database,
}

impl SqliteBudgetRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    fn delete(&self, sql: &str, id: String) -> StorageResult<bool> {
        self.db.write(|tx| Ok(tx.execute(sql, [id]).db()? > 0))
    }
}

/// Lit toutes les lignes d'une requête puis les convertit (la conversion peut échouer si la
/// base contient une donnée invalide).
fn load<R, T>(
    conn: &Connection,
    sql: &str,
    params: impl Params,
    read: impl FnMut(&Row<'_>) -> rusqlite::Result<R>,
    convert: impl Fn(R) -> StorageResult<T>,
) -> StorageResult<Vec<T>> {
    let mut statement = conn.prepare(sql).db()?;
    let rows = statement
        .query_map(params, read)
        .db()?
        .collect::<rusqlite::Result<Vec<_>>>()
        .db()?;
    rows.into_iter().map(convert).collect()
}

impl BudgetRepository for SqliteBudgetRepository {
    fn categories(&self) -> StorageResult<Vec<Category>> {
        self.db.read(|conn| {
            load(
                conn,
                "SELECT id, name, archived FROM categories ORDER BY rowid",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, bool>(2)?,
                    ))
                },
                |(id, name, archived)| {
                    Ok(Category::restore(
                        CategoryId::from_uuid(uuid(&id)?),
                        domain(Label::new(&name))?,
                        archived,
                    ))
                },
            )
        })
    }

    fn save_category(&self, category: &Category) -> StorageResult<()> {
        self.db.write(|tx| save_category(tx, category))
    }

    fn delete_category(&self, id: CategoryId) -> StorageResult<bool> {
        self.delete("DELETE FROM categories WHERE id = ?1", id.to_string())
    }

    fn salaries(&self) -> StorageResult<Vec<SalaryEntry>> {
        self.db.read(|conn| {
            load(
                conn,
                "SELECT id, effective_from, net_amount FROM salaries ORDER BY rowid",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
                |(id, effective_from, amount)| {
                    domain(SalaryEntry::new(
                        SalaryId::from_uuid(uuid(&id)?),
                        month(&effective_from)?,
                        money(&amount)?,
                    ))
                },
            )
        })
    }

    fn save_salary(&self, salary: &SalaryEntry) -> StorageResult<()> {
        self.db.write(|tx| save_salary(tx, salary))
    }

    fn delete_salary(&self, id: SalaryId) -> StorageResult<bool> {
        self.delete("DELETE FROM salaries WHERE id = ?1", id.to_string())
    }

    fn one_off_incomes(&self) -> StorageResult<Vec<OneOffIncome>> {
        self.db.read(|conn| {
            load(
                conn,
                "SELECT id, label, amount, date, account_id, recorded_at FROM one_off_incomes
                 ORDER BY rowid",
                [],
                |row| {
                    Ok((
                        (
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        ),
                        (
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                        ),
                    ))
                },
                |((id, label, amount, on), (account, recorded_at))| {
                    Ok(domain(OneOffIncome::new(
                        IncomeId::from_uuid(uuid(&id)?),
                        domain(Label::new(&label))?,
                        money(&amount)?,
                        date(&on)?,
                    ))?
                    .linked_to(optional_account(account)?)
                    .recorded(recorded_at.as_deref().map(datetime).transpose()?))
                },
            )
        })
    }

    fn save_one_off_income(&self, income: &OneOffIncome) -> StorageResult<()> {
        self.db.write(|tx| save_one_off_income(tx, income))
    }

    fn delete_one_off_income(&self, id: IncomeId) -> StorageResult<bool> {
        self.delete("DELETE FROM one_off_incomes WHERE id = ?1", id.to_string())
    }

    fn charges(&self) -> StorageResult<Vec<RecurringCharge>> {
        self.db.read(|conn| {
            load(
                conn,
                "SELECT id, label, amount, category_id, periodicity, start_month, end_month,
                     debit_account_id, debit_day
                 FROM recurring_charges ORDER BY rowid",
                [],
                |row| {
                    Ok((
                        (
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        ),
                        (
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                            row.get::<_, Option<String>>(6)?,
                        ),
                        (
                            row.get::<_, Option<String>>(7)?,
                            row.get::<_, Option<u8>>(8)?,
                        ),
                    ))
                },
                |(
                    (id, label, amount, category),
                    (periodicity, start, end),
                    (debit_account, debit_day),
                )| {
                    let debit = match (debit_account, debit_day) {
                        (Some(account), Some(day)) => Some(Debit {
                            account: AccountId::from_uuid(uuid(&account)?),
                            day,
                        }),
                        _ => None,
                    };
                    domain(
                        RecurringCharge::new(
                            ChargeId::from_uuid(uuid(&id)?),
                            domain(Label::new(&label))?,
                            money(&amount)?,
                            CategoryId::from_uuid(uuid(&category)?),
                            periodicity_from_sql(&periodicity)?,
                            month(&start)?,
                            end.as_deref().map(month).transpose()?,
                        )
                        .and_then(|charge| charge.with_debit(debit)),
                    )
                },
            )
        })
    }

    fn save_charge(&self, charge: &RecurringCharge) -> StorageResult<()> {
        self.db.write(|tx| save_charge(tx, charge))
    }

    fn delete_charge(&self, id: ChargeId) -> StorageResult<bool> {
        self.delete(
            "DELETE FROM recurring_charges WHERE id = ?1",
            id.to_string(),
        )
    }

    fn expenses(&self) -> StorageResult<Vec<Expense>> {
        self.db.read(|conn| {
            load(
                conn,
                "SELECT id, amount, category_id, date, label, account_id, recorded_at FROM expenses
                 ORDER BY date, rowid",
                [],
                |row| {
                    Ok((
                        (
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, Option<String>>(4)?,
                        ),
                        (
                            row.get::<_, Option<String>>(5)?,
                            row.get::<_, Option<String>>(6)?,
                        ),
                    ))
                },
                |((id, amount, category, on, label), (account, recorded_at))| {
                    Ok(domain(Expense::new(
                        ExpenseId::from_uuid(uuid(&id)?),
                        money(&amount)?,
                        CategoryId::from_uuid(uuid(&category)?),
                        date(&on)?,
                        label
                            .as_deref()
                            .map(Label::new)
                            .transpose()
                            .map_err(corrupt)?,
                    ))?
                    .linked_to(optional_account(account)?)
                    .recorded(recorded_at.as_deref().map(datetime).transpose()?))
                },
            )
        })
    }

    fn save_expense(&self, expense: &Expense) -> StorageResult<()> {
        self.db.write(|tx| save_expense(tx, expense))
    }

    fn delete_expense(&self, id: ExpenseId) -> StorageResult<bool> {
        self.delete("DELETE FROM expenses WHERE id = ?1", id.to_string())
    }
}

// --- Écritures partagées avec l'import ----------------------------------------------------

pub(crate) fn save_category(conn: &Connection, category: &Category) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO categories (id, name, archived) VALUES (?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, archived = excluded.archived",
        params![
            category.id().to_string(),
            category.name().as_str(),
            category.is_archived()
        ],
    )
    .db()
    .map(drop)
}

pub(crate) fn save_salary(conn: &Connection, salary: &SalaryEntry) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO salaries (id, effective_from, net_amount) VALUES (?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET effective_from = excluded.effective_from,
             net_amount = excluded.net_amount",
        params![
            salary.id().to_string(),
            salary.effective_from().to_string(),
            salary.net_amount().amount().to_string()
        ],
    )
    .db()
    .map(drop)
}

pub(crate) fn save_one_off_income(conn: &Connection, income: &OneOffIncome) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO one_off_incomes (id, label, amount, date, account_id, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET label = excluded.label, amount = excluded.amount,
             date = excluded.date, account_id = excluded.account_id,
             recorded_at = excluded.recorded_at",
        params![
            income.id().to_string(),
            income.label().as_str(),
            income.amount().amount().to_string(),
            income.date().to_string(),
            income.account().map(|a| a.to_string()),
            income.recorded_at().map(|at| at.to_rfc3339()),
        ],
    )
    .db()
    .map(drop)
}

pub(crate) fn save_charge(conn: &Connection, charge: &RecurringCharge) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO recurring_charges (id, label, amount, category_id, periodicity, start_month,
             end_month, debit_account_id, debit_day)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(id) DO UPDATE SET label = excluded.label, amount = excluded.amount,
             category_id = excluded.category_id, periodicity = excluded.periodicity,
             start_month = excluded.start_month, end_month = excluded.end_month,
             debit_account_id = excluded.debit_account_id, debit_day = excluded.debit_day",
        params![
            charge.id().to_string(),
            charge.label().as_str(),
            charge.amount().amount().to_string(),
            charge.category().to_string(),
            periodicity_to_sql(charge.periodicity()),
            charge.start().to_string(),
            charge.end().map(|end| end.to_string()),
            charge.debit().map(|d| d.account.to_string()),
            charge.debit().map(|d| d.day),
        ],
    )
    .db()
    .map(drop)
}

pub(crate) fn save_expense(conn: &Connection, expense: &Expense) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO expenses (id, amount, category_id, date, label, account_id, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET amount = excluded.amount,
             category_id = excluded.category_id, date = excluded.date, label = excluded.label,
             account_id = excluded.account_id, recorded_at = excluded.recorded_at",
        params![
            expense.id().to_string(),
            expense.amount().amount().to_string(),
            expense.category().to_string(),
            expense.date().to_string(),
            expense.label().map(Label::as_str),
            expense.account().map(|a| a.to_string()),
            expense.recorded_at().map(|at| at.to_rfc3339()),
        ],
    )
    .db()
    .map(drop)
}

fn optional_account(raw: Option<String>) -> StorageResult<Option<AccountId>> {
    raw.as_deref()
        .map(|id| uuid(id).map(AccountId::from_uuid))
        .transpose()
}

fn periodicity_to_sql(periodicity: Periodicity) -> &'static str {
    match periodicity {
        Periodicity::Monthly => "monthly",
        Periodicity::Quarterly => "quarterly",
        Periodicity::Yearly => "yearly",
    }
}

fn periodicity_from_sql(raw: &str) -> StorageResult<Periodicity> {
    match raw {
        "monthly" => Ok(Periodicity::Monthly),
        "quarterly" => Ok(Periodicity::Quarterly),
        "yearly" => Ok(Periodicity::Yearly),
        other => Err(corrupt(format!("périodicité inconnue « {other} »"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc_domain::{Money, Month, NaiveDate};
    use rust_decimal_macros::dec;

    fn repo() -> SqliteBudgetRepository {
        SqliteBudgetRepository::new(Database::open_in_memory().unwrap())
    }

    fn category(repo: &SqliteBudgetRepository, name: &str) -> Category {
        let category = Category::new(CategoryId::generate(), Label::new(name).unwrap());
        repo.save_category(&category).unwrap();
        category
    }

    #[test]
    fn categories_keep_creation_order_and_updates() {
        let repo = repo();
        let mut housing = category(&repo, "Logement");
        let food = category(&repo, "Alimentation");

        housing.archive();
        repo.save_category(&housing).unwrap();

        assert_eq!(repo.categories().unwrap(), vec![housing, food.clone()]);
        assert!(repo.delete_category(food.id()).unwrap());
        assert!(!repo.delete_category(food.id()).unwrap());
    }

    #[test]
    fn roundtrips_incomes_charges_and_expenses() {
        let repo = repo();
        let food = category(&repo, "Alimentation");
        let salary = SalaryEntry::new(
            SalaryId::generate(),
            Month::new(2026, 9).unwrap(),
            Money::new(dec!(3400)),
        )
        .unwrap();
        let bonus = OneOffIncome::new(
            IncomeId::generate(),
            Label::new("Prime").unwrap(),
            Money::new(dec!(1500.5)),
            NaiveDate::from_ymd_opt(2026, 12, 15).unwrap(),
        )
        .unwrap();
        let charge = RecurringCharge::new(
            ChargeId::generate(),
            Label::new("Assurance").unwrap(),
            Money::new(dec!(240)),
            food.id(),
            Periodicity::Yearly,
            Month::new(2026, 1).unwrap(),
            Some(Month::new(2027, 12).unwrap()),
        )
        .unwrap();
        let expense = Expense::new(
            ExpenseId::generate(),
            Money::new(dec!(42.50)),
            food.id(),
            NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
            Some(Label::new("Marché").unwrap()),
        )
        .unwrap();

        repo.save_salary(&salary).unwrap();
        repo.save_one_off_income(&bonus).unwrap();
        repo.save_charge(&charge).unwrap();
        repo.save_expense(&expense).unwrap();

        assert_eq!(repo.salaries().unwrap(), vec![salary.clone()]);
        assert_eq!(repo.one_off_incomes().unwrap(), vec![bonus]);
        assert_eq!(repo.charges().unwrap(), vec![charge]);
        assert_eq!(repo.expenses().unwrap(), vec![expense.clone()]);
        assert!(repo.delete_expense(expense.id()).unwrap());
        assert!(repo.expenses().unwrap().is_empty());
        assert!(repo.delete_salary(salary.id()).unwrap());
    }

    #[test]
    fn category_in_use_cannot_be_deleted_at_database_level() {
        let repo = repo();
        let food = category(&repo, "Alimentation");
        let expense = Expense::new(
            ExpenseId::generate(),
            Money::new(dec!(5)),
            food.id(),
            NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
            None,
        )
        .unwrap();
        repo.save_expense(&expense).unwrap();

        assert!(repo.delete_category(food.id()).is_err());
    }

    #[test]
    fn expense_with_unknown_category_is_rejected() {
        let repo = repo();
        let orphan = Expense::new(
            ExpenseId::generate(),
            Money::new(dec!(5)),
            CategoryId::generate(),
            NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
            None,
        )
        .unwrap();
        assert!(repo.save_expense(&orphan).is_err());
    }
}
