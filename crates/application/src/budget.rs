//! Revenus, charges fixes, dépenses ponctuelles, catégories et synthèse budgétaire.

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use lc_domain::account::AccountType;
use lc_domain::budget::{
    BudgetCalculator, Category, Expense, OneOffIncome, Periodicity, RecurringCharge, SalaryEntry,
};
use lc_domain::{
    AccountId, CategoryId, ChargeId, ExpenseId, IncomeId, Label, Money, Month, SalaryId, defaults,
};

use crate::ports::{AccountRepository, BudgetRepository, Clock};
use crate::{AppError, AppResult};

/// Synthèse budgétaire d'un mois.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetSummary {
    pub month: Month,
    pub salary: Money,
    pub one_off_incomes: Money,
    pub total_income: Money,
    pub fixed_charges: Money,
    pub expenses: Money,
    pub average_expenses: Money,
    pub savings_capacity: Money,
}

/// Dépenses d'un mois (éventuellement filtrées par catégorie), de la plus récente à la plus
/// ancienne.
/// Libellé de dépense déjà utilisé, proposé à la saisie avec la catégorie de sa dernière
/// utilisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelSuggestion {
    pub label: String,
    pub category: CategoryId,
    pub uses: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpenseList {
    pub month: Month,
    pub category: Option<CategoryId>,
    pub items: Vec<Expense>,
    pub total: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChargeInput {
    pub label: String,
    pub amount: Money,
    pub category: CategoryId,
    pub periodicity: Periodicity,
    pub start: Month,
    pub end: Option<Month>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpenseInput {
    pub amount: Money,
    pub category: CategoryId,
    /// Aujourd'hui si absent.
    pub date: Option<NaiveDate>,
    pub label: Option<String>,
    /// Compte courant débité, le cas échéant.
    pub account: Option<AccountId>,
}

pub(crate) fn summarize(repo: &dyn BudgetRepository, month: Month) -> AppResult<BudgetSummary> {
    let (salaries, incomes, charges, expenses) = (
        repo.salaries()?,
        repo.one_off_incomes()?,
        repo.charges()?,
        repo.expenses()?,
    );
    let calculator = BudgetCalculator::new(&salaries, &incomes, &charges, &expenses);
    Ok(BudgetSummary {
        month,
        salary: calculator.salary_for(month),
        one_off_incomes: calculator.one_off_incomes_for(month),
        total_income: calculator.total_income_for(month),
        fixed_charges: calculator.fixed_charges_for(month),
        expenses: calculator.expenses_for(month),
        average_expenses: calculator.average_monthly_expenses(month),
        savings_capacity: calculator.savings_capacity(month),
    })
}

pub struct BudgetUseCases {
    repo: Arc<dyn BudgetRepository>,
    /// Comptes courants auxquels les mouvements peuvent être rattachés.
    accounts: Arc<dyn AccountRepository>,
    clock: Arc<dyn Clock>,
}

impl BudgetUseCases {
    const SUGGESTION_LIMIT: usize = 30;

    pub fn new(
        repo: Arc<dyn BudgetRepository>,
        accounts: Arc<dyn AccountRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            accounts,
            clock,
        }
    }

    pub fn summary(&self, month: Option<Month>) -> AppResult<BudgetSummary> {
        summarize(
            self.repo.as_ref(),
            month.unwrap_or_else(|| self.current_month()),
        )
    }

    // --- Catégories ------------------------------------------------------------------------

    /// Premier lancement : crée les catégories par défaut si aucune n'existe.
    pub fn ensure_default_categories(&self) -> AppResult<()> {
        if !self.repo.categories()?.is_empty() {
            return Ok(());
        }
        for name in defaults::CATEGORY_NAMES {
            self.repo
                .save_category(&Category::new(CategoryId::generate(), Label::new(name)?))?;
        }
        Ok(())
    }

    pub fn categories(&self, include_archived: bool) -> AppResult<Vec<Category>> {
        let mut categories = self.repo.categories()?;
        categories.retain(|category| include_archived || category.is_selectable());
        Ok(categories)
    }

    pub fn create_category(&self, name: &str) -> AppResult<CategoryId> {
        let name = Label::new(name)?;
        self.ensure_unique_category_name(&name, None)?;
        let category = Category::new(CategoryId::generate(), name);
        self.repo.save_category(&category)?;
        Ok(category.id())
    }

    pub fn rename_category(&self, id: CategoryId, name: &str) -> AppResult<()> {
        let name = Label::new(name)?;
        self.ensure_unique_category_name(&name, Some(id))?;
        self.update_category(id, |category| category.rename(name))
    }

    pub fn archive_category(&self, id: CategoryId) -> AppResult<()> {
        self.update_category(id, Category::archive)
    }

    pub fn unarchive_category(&self, id: CategoryId) -> AppResult<()> {
        self.update_category(id, Category::unarchive)
    }

    /// Supprime une catégorie jamais utilisée ; une catégorie utilisée doit être archivée.
    pub fn delete_category(&self, id: CategoryId) -> AppResult<()> {
        let used = self.repo.expenses()?.iter().any(|e| e.category() == id)
            || self.repo.charges()?.iter().any(|c| c.category() == id);
        if used {
            return Err(AppError::CategoryInUse);
        }
        found(self.repo.delete_category(id)?, "Catégorie")
    }

    // --- Revenus ---------------------------------------------------------------------------

    /// Crée (`id` absent) ou modifie une entrée de salaire.
    pub fn upsert_salary(
        &self,
        id: Option<SalaryId>,
        effective_from: Month,
        net_amount: Money,
    ) -> AppResult<SalaryId> {
        if let Some(id) = id {
            self.require(
                self.repo.salaries()?.iter().any(|s| s.id() == id),
                "Salaire",
            )?;
        }
        let salary = SalaryEntry::new(
            id.unwrap_or_else(SalaryId::generate),
            effective_from,
            net_amount,
        )?;
        self.repo.save_salary(&salary)?;
        Ok(salary.id())
    }

    pub fn delete_salary(&self, id: SalaryId) -> AppResult<()> {
        found(self.repo.delete_salary(id)?, "Salaire")
    }

    /// Historique des salaires, du plus récent au plus ancien.
    pub fn salary_history(&self) -> AppResult<Vec<SalaryEntry>> {
        let mut salaries = self.repo.salaries()?;
        salaries.sort_by_key(|salary| std::cmp::Reverse(salary.effective_from()));
        Ok(salaries)
    }

    /// Crée ou modifie un revenu ponctuel, éventuellement versé sur un compte courant.
    pub fn upsert_one_off_income(
        &self,
        id: Option<IncomeId>,
        label: &str,
        amount: Money,
        date: NaiveDate,
        account: Option<AccountId>,
    ) -> AppResult<IncomeId> {
        let existing = match id {
            Some(id) => Some(
                self.repo
                    .one_off_incomes()?
                    .into_iter()
                    .find(|i| i.id() == id)
                    .ok_or(AppError::NotFound("Revenu"))?,
            ),
            None => None,
        };
        self.require_checking_link(account, existing.as_ref().and_then(OneOffIncome::account))?;
        let recorded_at = match &existing {
            Some(income) => income.recorded_at(),
            None => Some(self.clock.now()),
        };
        let income = OneOffIncome::new(
            id.unwrap_or_else(IncomeId::generate),
            Label::new(label)?,
            amount,
            date,
        )?
        .linked_to(account)
        .recorded(recorded_at);
        self.repo.save_one_off_income(&income)?;
        Ok(income.id())
    }

    pub fn delete_one_off_income(&self, id: IncomeId) -> AppResult<()> {
        found(self.repo.delete_one_off_income(id)?, "Revenu")
    }

    pub fn one_off_incomes(&self, month: Month) -> AppResult<Vec<OneOffIncome>> {
        let mut incomes = self.repo.one_off_incomes()?;
        incomes.retain(|income| income.month() == month);
        incomes.sort_by_key(|income| std::cmp::Reverse(income.date()));
        Ok(incomes)
    }

    // --- Charges fixes ---------------------------------------------------------------------

    pub fn upsert_charge(&self, id: Option<ChargeId>, input: ChargeInput) -> AppResult<ChargeId> {
        let previous_category = match id {
            Some(id) => {
                let charges = self.repo.charges()?;
                let existing = charges
                    .iter()
                    .find(|c| c.id() == id)
                    .ok_or(AppError::NotFound("Charge"))?;
                Some(existing.category())
            }
            None => None,
        };
        self.require_usable_category(input.category, previous_category)?;
        let charge = RecurringCharge::new(
            id.unwrap_or_else(ChargeId::generate),
            Label::new(&input.label)?,
            input.amount,
            input.category,
            input.periodicity,
            input.start,
            input.end,
        )?;
        self.repo.save_charge(&charge)?;
        Ok(charge.id())
    }

    pub fn delete_charge(&self, id: ChargeId) -> AppResult<()> {
        found(self.repo.delete_charge(id)?, "Charge")
    }

    pub fn charges(&self) -> AppResult<Vec<RecurringCharge>> {
        Ok(self.repo.charges()?)
    }

    // --- Dépenses ponctuelles --------------------------------------------------------------

    /// Saisie rapide : montant + catégorie suffisent (date du jour par défaut).
    pub fn add_expense(&self, input: ExpenseInput) -> AppResult<ExpenseId> {
        self.require_usable_category(input.category, None)?;
        self.require_checking_link(input.account, None)?;
        self.save_expense(ExpenseId::generate(), input, Some(self.clock.now()))
    }

    pub fn update_expense(&self, id: ExpenseId, input: ExpenseInput) -> AppResult<()> {
        let expenses = self.repo.expenses()?;
        let existing = expenses
            .iter()
            .find(|e| e.id() == id)
            .ok_or(AppError::NotFound("Dépense"))?;
        self.require_usable_category(input.category, Some(existing.category()))?;
        self.require_checking_link(input.account, existing.account())?;
        // L'instant de saisie d'origine est conservé : il situe la dépense par rapport au solde
        // de référence du compte courant.
        self.save_expense(id, input, existing.recorded_at())
            .map(|_| ())
    }

    pub fn delete_expense(&self, id: ExpenseId) -> AppResult<()> {
        found(self.repo.delete_expense(id)?, "Dépense")
    }

    pub fn list_expenses(
        &self,
        month: Month,
        category: Option<CategoryId>,
    ) -> AppResult<ExpenseList> {
        let mut items = self.repo.expenses()?;
        items.retain(|e| e.month() == month && category.is_none_or(|c| e.category() == c));
        items.sort_by_key(|e| std::cmp::Reverse(e.date()));
        let total = items.iter().map(Expense::amount).sum();
        Ok(ExpenseList {
            month,
            category,
            items,
            total,
        })
    }

    /// Libellés déjà saisis, du plus fréquent au moins fréquent (à fréquence égale, le plus
    /// récent d'abord), sans distinction de casse.
    pub fn expense_suggestions(&self) -> AppResult<Vec<LabelSuggestion>> {
        let mut expenses = self.repo.expenses()?;
        expenses.sort_by_key(|e| std::cmp::Reverse(e.date()));
        let mut suggestions: Vec<LabelSuggestion> = Vec::new();
        for expense in &expenses {
            let Some(label) = expense.label() else {
                continue;
            };
            let text = label.as_str();
            match suggestions
                .iter_mut()
                .find(|s| s.label.to_lowercase() == text.to_lowercase())
            {
                Some(suggestion) => suggestion.uses += 1,
                None => suggestions.push(LabelSuggestion {
                    label: text.to_string(),
                    category: expense.category(),
                    uses: 1,
                }),
            }
        }
        suggestions.sort_by_key(|s| std::cmp::Reverse(s.uses));
        suggestions.truncate(Self::SUGGESTION_LIMIT);
        Ok(suggestions)
    }

    // --- Aides -----------------------------------------------------------------------------

    /// Mois en cours (fuseau de l'utilisateur).
    pub fn current_month(&self) -> Month {
        Month::of(self.clock.today())
    }

    fn save_expense(
        &self,
        id: ExpenseId,
        input: ExpenseInput,
        recorded_at: Option<DateTime<Utc>>,
    ) -> AppResult<ExpenseId> {
        let label = input
            .label
            .as_deref()
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .map(Label::new)
            .transpose()?;
        let date = input.date.unwrap_or_else(|| self.clock.today());
        let expense = Expense::new(id, input.amount, input.category, date, label)?
            .linked_to(input.account)
            .recorded(recorded_at);
        self.repo.save_expense(&expense)?;
        Ok(expense.id())
    }

    /// Un mouvement ne peut être rattaché qu'à un compte courant actif, sauf si le rattachement
    /// est inchangé lors d'une modification (le compte a pu être clôturé depuis).
    fn require_checking_link(
        &self,
        account: Option<AccountId>,
        previous: Option<AccountId>,
    ) -> AppResult<()> {
        let Some(id) = account else {
            return Ok(());
        };
        if previous == Some(id) {
            return Ok(());
        }
        match self.accounts.get(id)? {
            Some(account)
                if account.account_type() == AccountType::Checking && !account.is_archived() =>
            {
                Ok(())
            }
            _ => Err(AppError::InvalidAccountLink),
        }
    }

    /// La catégorie doit exister et être sélectionnable, sauf si elle est inchangée lors d'une
    /// modification (une dépense ancienne peut rester dans une catégorie archivée).
    fn require_usable_category(
        &self,
        id: CategoryId,
        previous: Option<CategoryId>,
    ) -> AppResult<()> {
        let categories = self.repo.categories()?;
        let category = categories
            .iter()
            .find(|c| c.id() == id)
            .ok_or(AppError::NotFound("Catégorie"))?;
        if !category.is_selectable() && previous != Some(id) {
            return Err(AppError::CategoryArchived);
        }
        Ok(())
    }

    fn ensure_unique_category_name(
        &self,
        name: &Label,
        except: Option<CategoryId>,
    ) -> AppResult<()> {
        let taken = self.repo.categories()?.iter().any(|c| {
            Some(c.id()) != except
                && c.name().as_str().to_lowercase() == name.as_str().to_lowercase()
        });
        if taken {
            Err(AppError::DuplicateCategory)
        } else {
            Ok(())
        }
    }

    fn update_category(&self, id: CategoryId, change: impl FnOnce(&mut Category)) -> AppResult<()> {
        let mut category = self
            .repo
            .categories()?
            .into_iter()
            .find(|c| c.id() == id)
            .ok_or(AppError::NotFound("Catégorie"))?;
        change(&mut category);
        Ok(self.repo.save_category(&category)?)
    }

    fn require(&self, exists: bool, what: &'static str) -> AppResult<()> {
        if exists {
            Ok(())
        } else {
            Err(AppError::NotFound(what))
        }
    }
}

fn found(deleted: bool, what: &'static str) -> AppResult<()> {
    if deleted {
        Ok(())
    } else {
        Err(AppError::NotFound(what))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestEnv;
    use lc_domain::DomainError;
    use rust_decimal_macros::dec;

    fn month(year: i32, month: u32) -> Month {
        Month::new(year, month).unwrap()
    }

    fn category_named(budget: &BudgetUseCases, name: &str) -> CategoryId {
        budget
            .categories(true)
            .unwrap()
            .into_iter()
            .find(|c| c.name().as_str() == name)
            .unwrap()
            .id()
    }

    fn ready(env: &TestEnv) -> BudgetUseCases {
        let budget = env.budget();
        budget.ensure_default_categories().unwrap();
        budget
    }

    // Spec expense-tracking : « Libellés suggérés »
    #[test]
    fn suggestions_rank_labels_by_use_with_their_latest_category() {
        let env = TestEnv::new();
        let budget = ready(&env);
        let food = category_named(&budget, "Alimentation");
        let leisure = category_named(&budget, "Loisirs");
        let add = |label: Option<&str>, category, day| {
            budget
                .add_expense(ExpenseInput {
                    amount: Money::new(dec!(10)),
                    category,
                    date: NaiveDate::from_ymd_opt(2026, 10, day),
                    label: label.map(str::to_string),
                    account: None,
                })
                .unwrap();
        };
        add(Some("Cinéma"), leisure, 1);
        add(Some("Courses"), leisure, 2);
        add(Some("courses"), food, 5);
        add(Some("Boulangerie"), food, 6);
        add(None, food, 7);

        let suggestions = budget.expense_suggestions().unwrap();

        let labels: Vec<_> = suggestions.iter().map(|s| s.label.as_str()).collect();
        assert_eq!(labels, ["courses", "Boulangerie", "Cinéma"]);
        assert_eq!(suggestions[0].uses, 2);
        assert_eq!(
            suggestions[0].category, food,
            "catégorie de la saisie la plus récente"
        );
    }

    #[test]
    fn default_categories_are_created_once() {
        let env = TestEnv::new();
        let budget = ready(&env);
        budget.ensure_default_categories().unwrap();

        let names: Vec<_> = budget
            .categories(false)
            .unwrap()
            .iter()
            .map(|c| c.name().to_string())
            .collect();
        assert_eq!(names, defaults::CATEGORY_NAMES);
    }

    // Spec expense-tracking : « Saisie rapide »
    #[test]
    fn quick_expense_defaults_to_today() {
        let env = TestEnv::new();
        let budget = ready(&env);
        let food = category_named(&budget, "Alimentation");

        budget
            .add_expense(ExpenseInput {
                amount: Money::new(dec!(42.50)),
                category: food,
                date: None,
                label: None,
                account: None,
            })
            .unwrap();

        let list = budget.list_expenses(month(2026, 10), None).unwrap();
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].date(), env.clock.today());
        assert_eq!(list.total, Money::new(dec!(42.50)));
    }

    // Spec expense-tracking : « Total mensuel filtré »
    #[test]
    fn expenses_filtered_by_month_and_category() {
        let env = TestEnv::new();
        let budget = ready(&env);
        let transport = category_named(&budget, "Transport");
        let food = category_named(&budget, "Alimentation");
        let on = |m, d| Some(NaiveDate::from_ymd_opt(2026, m, d).unwrap());
        for (amount, category, date) in [
            (dec!(30), transport, on(10, 2)),
            (dec!(12), transport, on(10, 15)),
            (dec!(55), food, on(10, 3)),
            (dec!(40), transport, on(9, 28)),
        ] {
            budget
                .add_expense(ExpenseInput {
                    amount: Money::new(amount),
                    category,
                    date,
                    label: None,
                    account: None,
                })
                .unwrap();
        }

        let list = budget
            .list_expenses(month(2026, 10), Some(transport))
            .unwrap();

        assert_eq!(list.total, Money::new(dec!(42)));
        assert_eq!(list.items.len(), 2);
        assert!(list.items[0].date() > list.items[1].date());
    }

    // Spec expense-tracking : « Archivage d'une catégorie utilisée »
    #[test]
    fn used_category_can_only_be_archived() {
        let env = TestEnv::new();
        let budget = ready(&env);
        let leisure = category_named(&budget, "Loisirs");
        let expense = budget
            .add_expense(ExpenseInput {
                amount: Money::new(dec!(20)),
                category: leisure,
                date: None,
                label: Some("Cinéma".into()),
                account: None,
            })
            .unwrap();

        assert_eq!(
            budget.delete_category(leisure),
            Err(AppError::CategoryInUse)
        );
        budget.archive_category(leisure).unwrap();

        assert!(
            budget
                .categories(false)
                .unwrap()
                .iter()
                .all(|c| c.id() != leisure)
        );
        assert_eq!(
            budget.add_expense(ExpenseInput {
                amount: Money::new(dec!(5)),
                category: leisure,
                date: None,
                label: None,
                account: None,
            }),
            Err(AppError::CategoryArchived)
        );
        // Une dépense existante peut être modifiée sans changer de catégorie archivée.
        budget
            .update_expense(
                expense,
                ExpenseInput {
                    amount: Money::new(dec!(22)),
                    category: leisure,
                    date: None,
                    label: None,
                    account: None,
                },
            )
            .unwrap();
        assert_eq!(
            budget
                .list_expenses(month(2026, 10), Some(leisure))
                .unwrap()
                .total,
            Money::new(dec!(22))
        );
    }

    #[test]
    fn unused_category_can_be_deleted_and_names_are_unique() {
        let env = TestEnv::new();
        let budget = ready(&env);

        assert_eq!(
            budget.create_category("loisirs"),
            Err(AppError::DuplicateCategory)
        );
        let pets = budget.create_category("Animaux").unwrap();
        budget.delete_category(pets).unwrap();
        assert_eq!(
            budget.delete_category(pets),
            Err(AppError::NotFound("Catégorie"))
        );
    }

    // Spec income-management : « Augmentation » et « Historique des salaires »
    #[test]
    fn salary_history_and_summary() {
        let env = TestEnv::new();
        let budget = ready(&env);
        budget
            .upsert_salary(None, month(2026, 1), Money::new(dec!(3200)))
            .unwrap();
        let raise = budget
            .upsert_salary(None, month(2026, 9), Money::new(dec!(3400)))
            .unwrap();

        let history = budget.salary_history().unwrap();
        assert_eq!(history[0].id(), raise);
        assert_eq!(
            budget.summary(Some(month(2026, 8))).unwrap().salary,
            Money::new(dec!(3200))
        );
        assert_eq!(budget.summary(None).unwrap().salary, Money::new(dec!(3400)));

        budget.delete_salary(raise).unwrap();
        assert_eq!(budget.summary(None).unwrap().salary, Money::new(dec!(3200)));
    }

    // Spec income-management : « Montant négatif »
    #[test]
    fn negative_salary_is_rejected() {
        let env = TestEnv::new();
        let budget = ready(&env);
        assert_eq!(
            budget.upsert_salary(None, month(2026, 1), Money::new(dec!(-100))),
            Err(AppError::Domain(DomainError::NonPositiveAmount))
        );
    }

    // Spec expense-tracking : « Calcul nominal »
    #[test]
    fn summary_computes_savings_capacity() {
        let env = TestEnv::new();
        let budget = ready(&env);
        let housing = category_named(&budget, "Logement");
        let food = category_named(&budget, "Alimentation");
        budget
            .upsert_salary(None, month(2026, 1), Money::new(dec!(3200)))
            .unwrap();
        budget
            .upsert_charge(
                None,
                ChargeInput {
                    label: "Loyer".into(),
                    amount: Money::new(dec!(1300)),
                    category: housing,
                    periodicity: Periodicity::Monthly,
                    start: month(2026, 1),
                    end: None,
                },
            )
            .unwrap();
        for (m, amount) in [(7, dec!(600)), (8, dec!(750)), (9, dec!(450))] {
            let date = Some(NaiveDate::from_ymd_opt(2026, m, 12).unwrap());
            budget
                .add_expense(ExpenseInput {
                    amount: Money::new(amount),
                    category: food,
                    date,
                    label: None,
                    account: None,
                })
                .unwrap();
        }

        let summary = budget.summary(None).unwrap();

        assert_eq!(summary.fixed_charges, Money::new(dec!(1300)));
        assert_eq!(summary.average_expenses, Money::new(dec!(600)));
        assert_eq!(summary.savings_capacity, Money::new(dec!(1300)));
    }

    #[test]
    fn one_off_incomes_of_month() {
        let env = TestEnv::new();
        let budget = ready(&env);
        let december = NaiveDate::from_ymd_opt(2026, 12, 15).unwrap();
        budget
            .upsert_one_off_income(None, "Prime", Money::new(dec!(1500)), december, None)
            .unwrap();

        assert_eq!(budget.one_off_incomes(month(2026, 12)).unwrap().len(), 1);
        assert!(budget.one_off_incomes(month(2027, 1)).unwrap().is_empty());
        assert_eq!(
            budget
                .summary(Some(month(2026, 12)))
                .unwrap()
                .one_off_incomes,
            Money::new(dec!(1500))
        );
    }

    #[test]
    fn updating_unknown_items_fails() {
        let env = TestEnv::new();
        let budget = ready(&env);
        assert_eq!(
            budget.upsert_salary(
                Some(SalaryId::generate()),
                month(2026, 1),
                Money::new(dec!(1))
            ),
            Err(AppError::NotFound("Salaire"))
        );
        assert_eq!(
            budget.delete_expense(ExpenseId::generate()),
            Err(AppError::NotFound("Dépense"))
        );
    }
}
