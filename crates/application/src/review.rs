//! Bilan du mois (spec monthly-review) : bilan en attente au début d'un mois, chiffres du mois
//! comparés au précédent, recalage du compte courant, historique.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDate;
use lc_domain::account::{Account, AccountType};
use lc_domain::budget::BudgetCalculator;
use lc_domain::review::{MonthlyReview, suggested_start_balance};
use lc_domain::transfer::Endpoint;
use lc_domain::valuation::{PriceBook, value_account};
use lc_domain::{
    AccountId, CategoryId, Decimal, GoalId, Label, Money, Month, Snapshot, SnapshotKind,
};

use crate::flows::build_ledger;
use crate::ports::{
    AccountRepository, BudgetRepository, Clock, PlanningRepository, SnapshotRepository,
    TransferRepository,
};
use crate::{AppError, AppResult};

/// Progression d'un objectif en fin de mois.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalMonth {
    pub goal_id: GoalId,
    pub name: Label,
    pub percent: Decimal,
    /// En fin de mois précédent, si un snapshot le permet.
    pub previous_percent: Option<Decimal>,
}

/// Chiffres d'un mois écoulé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthReport {
    pub month: Month,
    /// Revenus et entrées extérieures moins charges, dépenses et sorties hors patrimoine.
    pub saved: Money,
    /// Mis de côté du mois précédent, s'il contient des données.
    pub previous_saved: Option<Money>,
    /// Revenus et entrées extérieures sur un compte courant.
    pub received: Money,
    pub one_off_incomes: Money,
    pub fixed_charges: Money,
    pub expenses: Money,
    pub external_out: Money,
    pub top_categories: Vec<(CategoryId, Money)>,
    /// Plus forte hausse sur un mois (catégorie dépensée les deux mois), en pourcentage.
    pub biggest_increase: Option<(CategoryId, Decimal)>,
    /// Patrimoine en fin de mois et variation sur le mois, selon les snapshots disponibles.
    pub wealth_end: Option<Money>,
    pub wealth_change: Option<Money>,
    pub wealth_change_percent: Option<Decimal>,
    pub goals: Vec<GoalMonth>,
}

/// Virement du compte courant déjà échu dans le mois, rappelé au recalage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludedTransfer {
    pub counterpart: String,
    pub amount: Money,
    pub date: NaiveDate,
}

/// Question de début de mois : « Votre compte courant repart à combien ? ».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartPrompt {
    pub account_id: AccountId,
    pub account_name: Label,
    pub suggested: Money,
    pub included: Vec<IncludedTransfer>,
}

pub struct ReviewUseCases {
    planning: Arc<dyn PlanningRepository>,
    accounts: Arc<dyn AccountRepository>,
    budget: Arc<dyn BudgetRepository>,
    transfers: Arc<dyn TransferRepository>,
    snapshots: Arc<dyn SnapshotRepository>,
    clock: Arc<dyn Clock>,
}

impl ReviewUseCases {
    /// Nombre maximal de bilans passés proposés.
    pub const HISTORY_MONTHS: usize = 24;

    pub fn new(
        planning: Arc<dyn PlanningRepository>,
        accounts: Arc<dyn AccountRepository>,
        budget: Arc<dyn BudgetRepository>,
        transfers: Arc<dyn TransferRepository>,
        snapshots: Arc<dyn SnapshotRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            planning,
            accounts,
            budget,
            transfers,
            snapshots,
            clock,
        }
    }

    fn current_month(&self) -> Month {
        Month::of(self.clock.today())
    }

    /// Mois où des données existent (dépense saisie ou snapshot pris).
    fn data_months(&self) -> AppResult<Vec<Month>> {
        let mut months: Vec<Month> = self
            .budget
            .expenses()?
            .iter()
            .map(|e| e.month())
            .chain(self.snapshots.list()?.iter().map(Snapshot::month))
            .collect();
        months.sort();
        months.dedup();
        Ok(months)
    }

    /// Mois précédent si son bilan n'a jamais été ouvert et que l'application a servi avant le
    /// mois en cours.
    pub fn pending_review(&self) -> AppResult<Option<Month>> {
        let current = self.current_month();
        let previous = current.previous();
        let used_before = self.data_months()?.iter().any(|m| *m < current);
        let seen = self
            .planning
            .reviews()?
            .iter()
            .any(|r| r.month() == previous);
        Ok((used_before && !seen).then_some(previous))
    }

    /// Bilans passés consultables, du plus récent au plus ancien.
    pub fn review_months(&self) -> AppResult<Vec<Month>> {
        let current = self.current_month();
        let Some(first) = self.data_months()?.into_iter().find(|m| *m < current) else {
            return Ok(Vec::new());
        };
        let mut months = Vec::new();
        let mut month = current.previous();
        while month >= first && months.len() < Self::HISTORY_MONTHS {
            months.push(month);
            month = month.previous();
        }
        Ok(months)
    }

    /// Enregistre le bilan de `month` comme vu (il ne s'ouvrira plus tout seul).
    pub fn open_review(&self, month: Month) -> AppResult<()> {
        if self.planning.reviews()?.iter().any(|r| r.month() == month) {
            return Ok(());
        }
        Ok(self
            .planning
            .save_review(&MonthlyReview::new(month, self.clock.now()))?)
    }

    pub fn month_report(&self, month: Month) -> AppResult<MonthReport> {
        let (salaries, incomes, charges, expenses, transfers) = (
            self.budget.salaries()?,
            self.budget.one_off_incomes()?,
            self.budget.charges()?,
            self.budget.expenses()?,
            self.transfers.transfers()?,
        );
        let checking: Vec<AccountId> = self
            .accounts
            .list()?
            .iter()
            .filter(|a| a.account_type() == AccountType::Checking)
            .map(Account::id)
            .collect();
        let calculator = BudgetCalculator::new(&salaries, &incomes, &charges, &expenses)
            .with_transfers(&transfers)
            .with_checking(&checking);
        let received = |m: Month| calculator.total_income_for(m) + calculator.external_in_for(m);
        let saved = |m: Month| {
            received(m)
                - calculator.fixed_charges_for(m)
                - calculator.expenses_for(m)
                - calculator.checking_external_out_for(m)
        };
        let previous = month.previous();
        let months_with_data = self.data_months()?;
        let previous_has_data = months_with_data.contains(&previous)
            || salaries.iter().any(|s| s.effective_from() <= previous);

        let by_category = |m: Month| {
            let mut totals: HashMap<CategoryId, Money> = HashMap::new();
            for expense in expenses.iter().filter(|e| e.month() == m) {
                *totals.entry(expense.category()).or_default() += expense.amount();
            }
            totals
        };
        let current_categories = by_category(month);
        let previous_categories = by_category(previous);
        let mut top_categories: Vec<(CategoryId, Money)> =
            current_categories.iter().map(|(c, m)| (*c, *m)).collect();
        top_categories.sort_by_key(|(_, amount)| std::cmp::Reverse(*amount));
        top_categories.truncate(3);
        let biggest_increase = current_categories
            .iter()
            .filter_map(|(category, amount)| {
                let before = previous_categories.get(category)?;
                (before.is_positive() && amount > before).then(|| {
                    let percent = ((amount.amount() - before.amount()) / before.amount()
                        * Decimal::ONE_HUNDRED)
                        .round_dp(0);
                    (*category, percent)
                })
            })
            .max_by_key(|(_, percent)| *percent);

        let snapshots = self.snapshots.list()?;
        let end = end_of(&snapshots, month);
        let start = auto_of(&snapshots, month);
        let wealth_change = match (start, end) {
            (Some(start), Some(end)) if !std::ptr::eq(start, end) => {
                Some(end.total() - start.total())
            }
            _ => None,
        };
        let wealth_change_percent = match (start, wealth_change) {
            (Some(start), Some(change)) if start.total().is_positive() => {
                Some((change.amount() / start.total().amount() * Decimal::ONE_HUNDRED).round_dp(1))
            }
            _ => None,
        };
        let previous_end = end_of(&snapshots, previous);
        let goals = self
            .planning
            .goals()?
            .into_iter()
            .filter_map(|goal| {
                let at = |snapshot: &Snapshot| -> Money {
                    goal.accounts()
                        .iter()
                        .filter_map(|id| snapshot.value_of(*id))
                        .sum()
                };
                let percent = goal.progress_percent(at(end?));
                Some(GoalMonth {
                    goal_id: goal.id(),
                    name: goal.name().clone(),
                    percent,
                    previous_percent: previous_end.map(|s| goal.progress_percent(at(s))),
                })
            })
            .collect();

        Ok(MonthReport {
            month,
            saved: saved(month),
            previous_saved: previous_has_data.then(|| saved(previous)),
            received: received(month),
            one_off_incomes: calculator.one_off_incomes_for(month),
            fixed_charges: calculator.fixed_charges_for(month),
            expenses: calculator.expenses_for(month),
            external_out: calculator.checking_external_out_for(month),
            top_categories,
            biggest_increase,
            wealth_end: end.map(Snapshot::total),
            wealth_change,
            wealth_change_percent,
            goals,
        })
    }

    fn first_checking(&self) -> AppResult<Option<Account>> {
        Ok(self
            .accounts
            .list()?
            .into_iter()
            .find(|a| a.account_type() == AccountType::Checking && !a.is_archived()))
    }

    /// Question de début de mois pour le premier compte courant actif.
    pub fn start_prompt(&self) -> AppResult<Option<StartPrompt>> {
        let Some(checking) = self.first_checking()? else {
            return Ok(None);
        };
        let today = self.clock.today();
        let month = Month::of(today);
        let ledger = build_ledger(self.budget.as_ref(), self.transfers.as_ref(), today)?;
        let current = value_account(&checking, &PriceBook::default(), &ledger).value;
        let reviews = self.planning.reviews()?;
        let mut names: HashMap<Endpoint, String> = self
            .accounts
            .list()?
            .iter()
            .map(|a| (Endpoint::Account(a.id()), a.name().to_string()))
            .collect();
        for external in self.transfers.external_accounts()? {
            names.insert(
                Endpoint::External(external.id()),
                external.name().to_string(),
            );
        }
        let mut included: Vec<IncludedTransfer> = self
            .transfers
            .transfers()?
            .iter()
            .filter(|t| t.from() == Endpoint::Account(checking.id()))
            .flat_map(|t| {
                let counterpart = names.get(&t.to()).cloned().unwrap_or_default();
                t.schedule()
                    .occurrences_between(month.first_day(), today)
                    .into_iter()
                    .map(move |date| IncludedTransfer {
                        counterpart: counterpart.clone(),
                        amount: t.sent(),
                        date,
                    })
            })
            .collect();
        included.sort_by_key(|t| t.date);
        Ok(Some(StartPrompt {
            account_id: checking.id(),
            account_name: checking.name().clone(),
            suggested: suggested_start_balance(&reviews, month, current),
            included,
        }))
    }

    /// Recale le premier compte courant au solde de départ du mois et le retient pour le mois
    /// suivant.
    pub fn start_month(&self, balance: Money) -> AppResult<()> {
        let Some(mut checking) = self.first_checking()? else {
            return Err(AppError::NotFound("Compte courant"));
        };
        checking.update_checking_balance(balance, self.clock.today(), self.clock.now())?;
        self.accounts.save(&checking)?;
        let previous = self.current_month().previous();
        let mut review = self
            .planning
            .reviews()?
            .into_iter()
            .find(|r| r.month() == previous)
            .unwrap_or_else(|| MonthlyReview::new(previous, self.clock.now()));
        review.set_next_start_balance(balance);
        Ok(self.planning.save_review(&review)?)
    }
}

fn auto_of(snapshots: &[Snapshot], month: Month) -> Option<&Snapshot> {
    snapshots
        .iter()
        .filter(|s| s.kind() == SnapshotKind::Auto && s.month() == month)
        .min_by_key(|s| s.taken_on())
}

/// Patrimoine en fin de mois : snapshot automatique du mois suivant, sinon dernier du mois.
fn end_of(snapshots: &[Snapshot], month: Month) -> Option<&Snapshot> {
    auto_of(snapshots, month.next()).or_else(|| {
        snapshots
            .iter()
            .filter(|s| s.month() == month)
            .max_by_key(|s| s.taken_on())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::NewAccount;
    use crate::budget::ExpenseInput;
    use crate::goals::GoalInput;
    use crate::testing::TestEnv;
    use crate::transfers::TransferInput;
    use lc_domain::transfer::Schedule;
    use pollster::block_on;
    use rust_decimal_macros::dec;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn create(env: &TestEnv, name: &str, account_type: AccountType) -> AccountId {
        env.accounts()
            .create_account(NewAccount {
                name: name.into(),
                account_type,
                color: None,
            })
            .unwrap()
    }

    fn spend(env: &TestEnv, category: CategoryId, amount: Decimal, date: NaiveDate) {
        env.budget()
            .add_expense(ExpenseInput {
                amount: Money::new(amount),
                category,
                date: Some(date),
                label: None,
                account: None,
            })
            .unwrap();
    }

    // Spec monthly-review : « Première utilisation », « Début de mois »
    #[test]
    fn review_is_pending_once_after_a_month_of_use() {
        let env = TestEnv::new();
        env.clock.set_date(day(2026, 10, 21));
        let reviews = env.reviews();
        create(&env, "Livret A", AccountType::LivretA);
        block_on(env.dashboard().dashboard(false)).unwrap(); // snapshot d'octobre
        assert_eq!(reviews.pending_review().unwrap(), None);

        env.clock.set_date(day(2026, 11, 1));
        let october = Month::new(2026, 10).unwrap();
        assert_eq!(reviews.pending_review().unwrap(), Some(october));
        reviews.open_review(october).unwrap();
        assert_eq!(reviews.pending_review().unwrap(), None);
        assert_eq!(reviews.review_months().unwrap(), vec![october]);
    }

    // Spec monthly-review : « Mis de côté », « Hausse d'une catégorie »
    #[test]
    fn month_report_compares_with_the_previous_month() {
        let env = TestEnv::new();
        let budget = env.budget();
        budget.ensure_default_categories().unwrap();
        let categories = budget.categories(false).unwrap();
        let (leisure, food) = (categories[0].id(), categories[1].id());
        let september = Month::new(2026, 9).unwrap();
        let october = Month::new(2026, 10).unwrap();
        budget
            .upsert_salary(None, september, Money::new(dec!(3200)))
            .unwrap();
        budget
            .upsert_one_off_income(
                None,
                "Prime",
                Money::new(dec!(250)),
                day(2026, 10, 15),
                None,
            )
            .unwrap();
        env.clock.set_date(day(2026, 9, 1));
        budget
            .upsert_charge(
                None,
                crate::budget::ChargeInput {
                    label: "Loyer".into(),
                    amount: Money::new(dec!(1300)),
                    category: food,
                    periodicity: lc_domain::budget::Periodicity::Monthly,
                    start: september,
                    end: None,
                    debit: None,
                },
            )
            .unwrap();
        spend(&env, leisure, dec!(230), day(2026, 9, 10));
        spend(&env, food, dec!(1250), day(2026, 9, 12));
        spend(&env, leisure, dec!(310), day(2026, 10, 10));
        spend(&env, food, dec!(1230), day(2026, 10, 12));
        env.clock.set_date(day(2026, 11, 1));

        let report = env.reviews().month_report(october).unwrap();
        assert_eq!(report.received, Money::new(dec!(3450)));
        assert_eq!(report.saved, Money::new(dec!(610)));
        assert_eq!(report.previous_saved, Some(Money::new(dec!(420))));
        assert_eq!(report.biggest_increase, Some((leisure, dec!(35))));
        assert_eq!(report.top_categories[0], (food, Money::new(dec!(1230))));
    }

    #[test]
    fn wealth_and_goals_come_from_snapshots() {
        let env = TestEnv::new();
        env.clock.set_date(day(2026, 10, 1));
        let livret = create(&env, "Livret A", AccountType::LivretA);
        env.accounts()
            .update_balance(livret, Money::new(dec!(10000)))
            .unwrap();
        env.goals()
            .create_goal(GoalInput {
                name: "Voyage".into(),
                target: Money::new(dec!(20000)),
                deadline: None,
                accounts: vec![livret],
            })
            .unwrap();
        block_on(env.dashboard().dashboard(false)).unwrap();
        env.clock.set_date(day(2026, 11, 1));
        env.accounts()
            .update_balance(livret, Money::new(dec!(11000)))
            .unwrap();
        block_on(env.dashboard().dashboard(false)).unwrap();

        let report = env
            .reviews()
            .month_report(Month::new(2026, 10).unwrap())
            .unwrap();
        assert_eq!(report.wealth_end, Some(Money::new(dec!(11000))));
        assert_eq!(report.wealth_change, Some(Money::new(dec!(1000))));
        assert_eq!(report.wealth_change_percent, Some(dec!(10.0)));
        assert_eq!(report.goals[0].percent, dec!(55.0));
        // Fin septembre = snapshot automatique d'octobre.
        assert_eq!(report.goals[0].previous_percent, Some(dec!(50.0)));
    }

    // Spec monthly-review : « Compte perso à 250 € », « Mois suivant »
    #[test]
    fn start_of_month_resets_the_checking_account() {
        let env = TestEnv::new();
        env.clock.set_date(day(2026, 11, 1));
        let checking = create(&env, "Compte courant", AccountType::Checking);
        let ldds = create(&env, "LDDS", AccountType::Ldds);
        env.accounts()
            .update_balance(checking, Money::new(dec!(2660)))
            .unwrap();
        env.clock.advance(chrono::Duration::minutes(5));
        let joint = env
            .transfers()
            .create_external_account("Compte joint")
            .unwrap();
        for (to, amount) in [
            (Endpoint::Account(ldds), dec!(850)),
            (Endpoint::External(joint), dec!(1560)),
        ] {
            env.transfers()
                .create_transfer(TransferInput {
                    from: Endpoint::Account(checking),
                    to,
                    sent: Money::new(amount),
                    received: None,
                    label: None,
                    schedule: Schedule::Monthly {
                        day: 1,
                        from: Month::new(2026, 11).unwrap(),
                        until: None,
                    },
                })
                .unwrap();
        }
        let reviews = env.reviews();
        let prompt = reviews.start_prompt().unwrap().unwrap();
        assert_eq!(prompt.included.len(), 2);
        assert_eq!(prompt.included[1].counterpart, "Compte joint");

        env.clock.advance(chrono::Duration::minutes(5));
        reviews.start_month(Money::new(dec!(250))).unwrap();
        let value = block_on(env.accounts().valued_account(checking))
            .unwrap()
            .1
            .value;
        assert_eq!(value, Money::new(dec!(250)));

        env.clock.set_date(day(2026, 12, 1));
        assert_eq!(
            reviews.start_prompt().unwrap().unwrap().suggested,
            Money::new(dec!(250))
        );
    }
}
