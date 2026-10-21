use super::allowance::{AllowanceSources, SpendingAllowance};
use super::{Expense, OneOffIncome, RecurringCharge, SalaryEntry};
use crate::transfer::{Endpoint, Schedule, Transfer};
use crate::{AccountId, CategoryId, Money, Month};
use chrono::NaiveDate;

/// Calculs budgétaires sur un mois : revenus, charges, dépenses et capacité d'épargne.
#[derive(Debug, Clone, Copy)]
pub struct BudgetCalculator<'a> {
    salaries: &'a [SalaryEntry],
    one_off_incomes: &'a [OneOffIncome],
    charges: &'a [RecurringCharge],
    expenses: &'a [Expense],
    transfers: &'a [Transfer],
    /// Comptes courants : le « porte-monnaie » du foyer pour le classement des virements.
    checking: &'a [AccountId],
}

impl<'a> BudgetCalculator<'a> {
    /// Nombre de mois complets utilisés pour la moyenne des dépenses ponctuelles.
    pub const EXPENSE_AVERAGE_MONTHS: i32 = 3;

    pub fn new(
        salaries: &'a [SalaryEntry],
        one_off_incomes: &'a [OneOffIncome],
        charges: &'a [RecurringCharge],
        expenses: &'a [Expense],
    ) -> Self {
        Self {
            salaries,
            one_off_incomes,
            charges,
            expenses,
            transfers: &[],
            checking: &[],
        }
    }

    /// Ajoute les virements : ceux échangés avec un compte hors patrimoine entrent dans le budget
    /// (sorties et entrées du foyer) ; ceux entre comptes suivis sont neutres.
    pub fn with_transfers(mut self, transfers: &'a [Transfer]) -> Self {
        self.transfers = transfers;
        self
    }

    /// Déclare les comptes courants (spec spending-allowance, D2).
    pub fn with_checking(mut self, checking: &'a [AccountId]) -> Self {
        self.checking = checking;
        self
    }

    fn is_checking(&self, endpoint: Endpoint) -> bool {
        endpoint
            .account()
            .is_some_and(|id| self.checking.contains(&id))
    }

    /// Compte suivi qui n'est pas un compte courant (livret, placement, épargne salariale).
    fn is_savings(&self, endpoint: Endpoint) -> bool {
        endpoint.account().is_some() && !self.is_checking(endpoint)
    }

    /// Somme du mois des virements retenus par `keep`, au montant envoyé ou reçu.
    fn transfers_for(
        &self,
        month: Month,
        keep: impl Fn(&Transfer) -> bool,
        amount: fn(&Transfer) -> Money,
    ) -> Money {
        self.transfers
            .iter()
            .filter(|t| keep(t))
            .map(|t| amount(t).times(occurrences(t.schedule(), month)))
            .sum()
    }

    /// Virements du mois vers un compte hors patrimoine (montants envoyés).
    pub fn external_out_for(&self, month: Month) -> Money {
        self.transfers_for(
            month,
            |t| t.to().is_external() && !t.from().is_external(),
            Transfer::sent,
        )
    }

    /// Virements du mois reçus de l'extérieur sur un compte courant (montants reçus). Un virement
    /// de l'extérieur vers une épargne est de l'épargne financée par les revenus, pas un revenu.
    pub fn external_in_for(&self, month: Month) -> Money {
        self.transfers_for(
            month,
            |t| t.from().is_external() && self.is_checking(t.to()),
            Transfer::received,
        )
    }

    /// Épargne programmée du mois : d'un compte courant ou de l'extérieur vers une épargne.
    pub fn scheduled_savings_for(&self, month: Month) -> Money {
        self.transfers_for(
            month,
            |t| (self.is_checking(t.from()) || t.from().is_external()) && self.is_savings(t.to()),
            Transfer::sent,
        )
    }

    /// Rapatriements du mois : d'une épargne vers un compte courant.
    pub fn repatriated_for(&self, month: Month) -> Money {
        self.transfers_for(
            month,
            |t| self.is_savings(t.from()) && self.is_checking(t.to()),
            Transfer::received,
        )
    }

    /// Virements du mois d'un compte courant vers l'extérieur.
    pub fn checking_external_out_for(&self, month: Month) -> Money {
        self.transfers_for(
            month,
            |t| self.is_checking(t.from()) && t.to().is_external(),
            Transfer::sent,
        )
    }

    /// Reste à dépenser du mois `month`, vu le jour `today`.
    pub fn allowance(&self, month: Month, today: NaiveDate) -> SpendingAllowance {
        SpendingAllowance::compute(
            AllowanceSources {
                month,
                income: self.total_income_for(month),
                external_in: self.external_in_for(month),
                repatriated: self.repatriated_for(month),
                fixed_charges: self.fixed_charges_for(month),
                scheduled_savings: self.scheduled_savings_for(month),
                external_out: self.checking_external_out_for(month),
                spent: self.expenses_for(month),
            },
            today,
        )
    }
    /// Équivalent mensuel net des virements récurrents avec l'extérieur actifs en `month`
    /// (sorties − entrées ; un virement annuel compte pour 1/12).
    pub fn recurring_external_net_for(&self, month: Month) -> Money {
        self.transfers
            .iter()
            .filter_map(|t| {
                let monthly = match t.schedule() {
                    Schedule::Once { .. } => return None,
                    Schedule::Monthly { from, until, .. } => {
                        let active = month >= from && until.is_none_or(|until| month <= until);
                        if !active {
                            return None;
                        }
                        Money::new(rust_decimal::Decimal::ONE)
                    }
                    Schedule::Yearly {
                        from_year,
                        until_year,
                        ..
                    } => {
                        let active = month.year() >= from_year
                            && until_year.is_none_or(|until| month.year() <= until);
                        if !active {
                            return None;
                        }
                        Money::new(rust_decimal::Decimal::ONE).divided_by(12)?
                    }
                };
                let factor = monthly.amount();
                match (t.from().is_external(), t.to().is_external()) {
                    (false, true) => Some(t.sent().times(factor)),
                    (true, false) if self.is_checking(t.to()) => Some(-t.received().times(factor)),
                    _ => None,
                }
            })
            .sum()
    }

    /// Salaire applicable : celui dont la date d'effet est la plus récente sans dépasser `month`.
    pub fn salary_for(&self, month: Month) -> Money {
        self.salaries
            .iter()
            .filter(|salary| salary.effective_from() <= month)
            .max_by_key(|salary| salary.effective_from())
            .map_or(Money::ZERO, SalaryEntry::net_amount)
    }

    pub fn one_off_incomes_for(&self, month: Month) -> Money {
        self.one_off_incomes
            .iter()
            .filter(|income| income.month() == month)
            .map(OneOffIncome::amount)
            .sum()
    }

    /// Revenu total du mois : salaire applicable + revenus ponctuels du mois.
    pub fn total_income_for(&self, month: Month) -> Money {
        self.salary_for(month) + self.one_off_incomes_for(month)
    }

    /// Somme des équivalents mensuels des charges fixes actives dans `month`.
    pub fn fixed_charges_for(&self, month: Month) -> Money {
        self.charges
            .iter()
            .filter(|charge| charge.is_active_in(month))
            .map(RecurringCharge::monthly_equivalent)
            .sum()
    }

    pub fn expenses_for(&self, month: Month) -> Money {
        self.expenses
            .iter()
            .filter(|expense| expense.month() == month)
            .map(Expense::amount)
            .sum()
    }

    pub fn expenses_for_category(&self, month: Month, category: CategoryId) -> Money {
        self.expenses
            .iter()
            .filter(|expense| expense.month() == month && expense.category() == category)
            .map(Expense::amount)
            .sum()
    }

    /// Moyenne mensuelle des dépenses ponctuelles sur les 3 derniers mois complets avant
    /// `current`. Les mois antérieurs à la première dépense saisie ne comptent pas ; s'il n'existe
    /// aucun mois complet, c'est le total du mois en cours qui est retenu.
    pub fn average_monthly_expenses(&self, current: Month) -> Money {
        let Some(first_tracked) = self.expenses.iter().map(Expense::month).min() else {
            return Money::ZERO;
        };
        let window_start = current
            .plus_months(-Self::EXPENSE_AVERAGE_MONTHS)
            .max(first_tracked);
        let complete_months = window_start.months_until(current);
        if complete_months <= 0 {
            return self.expenses_for(current);
        }
        let total: Money = self
            .expenses
            .iter()
            .filter(|expense| expense.month() >= window_start && expense.month() < current)
            .map(Expense::amount)
            .sum();
        total
            .divided_by(complete_months.unsigned_abs())
            .expect("au moins un mois complet")
    }

    /// Capacité d'épargne mensuelle : salaire applicable − charges fixes − moyenne des dépenses
    /// ponctuelles − virements récurrents nets vers l'extérieur. Peut être négative.
    pub fn savings_capacity(&self, current: Month) -> Money {
        self.salary_for(current)
            - self.fixed_charges_for(current)
            - self.average_monthly_expenses(current)
            - self.recurring_external_net_for(current)
    }
}

/// Nombre d'occurrences d'un virement dans le mois, en décimal pour multiplier un montant.
fn occurrences(schedule: Schedule, month: Month) -> rust_decimal::Decimal {
    rust_decimal::Decimal::from(schedule.occurrences_in(month).len())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::Periodicity;
    use crate::{ChargeId, ExpenseId, IncomeId, Label, SalaryId};
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    fn month(year: i32, month: u32) -> Month {
        Month::new(year, month).unwrap()
    }

    fn eur(amount: i64) -> Money {
        Money::new(Decimal::from(amount))
    }

    fn salary(year: i32, m: u32, amount: i64) -> SalaryEntry {
        SalaryEntry::new(SalaryId::generate(), month(year, m), eur(amount)).unwrap()
    }

    fn expense(year: i32, m: u32, amount: i64) -> Expense {
        let date = NaiveDate::from_ymd_opt(year, m, 10).unwrap();
        Expense::new(
            ExpenseId::generate(),
            eur(amount),
            CategoryId::generate(),
            date,
            None,
        )
        .unwrap()
    }

    fn charge(
        amount: i64,
        periodicity: Periodicity,
        start: Month,
        end: Option<Month>,
    ) -> RecurringCharge {
        RecurringCharge::new(
            ChargeId::generate(),
            Label::new("Charge").unwrap(),
            eur(amount),
            CategoryId::generate(),
            periodicity,
            start,
            end,
        )
        .unwrap()
    }

    fn salaries_only(salaries: &[SalaryEntry]) -> BudgetCalculator<'_> {
        BudgetCalculator::new(salaries, &[], &[], &[])
    }

    // Spec income-management : « Augmentation »
    #[test]
    fn raise_applies_from_its_effective_month() {
        let salaries = [salary(2026, 1, 3200), salary(2026, 9, 3400)];
        let budget = salaries_only(&salaries);

        assert_eq!(budget.salary_for(month(2025, 12)), Money::ZERO);
        assert_eq!(budget.salary_for(month(2026, 1)), eur(3200));
        assert_eq!(budget.salary_for(month(2026, 8)), eur(3200));
        assert_eq!(budget.salary_for(month(2026, 9)), eur(3400));
        assert_eq!(budget.salary_for(month(2027, 3)), eur(3400));
    }

    // Spec income-management : « Suppression d'une entrée »
    #[test]
    fn previous_salary_applies_again_after_deleting_raise() {
        let salaries = [salary(2026, 1, 3200)];
        assert_eq!(
            salaries_only(&salaries).salary_for(month(2026, 9)),
            eur(3200)
        );
    }

    // Spec income-management : « Prime de fin d'année »
    #[test]
    fn one_off_income_counts_only_in_its_month() {
        let salaries = [salary(2026, 1, 3200)];
        let bonus = [OneOffIncome::new(
            IncomeId::generate(),
            Label::new("Prime").unwrap(),
            eur(1500),
            NaiveDate::from_ymd_opt(2026, 12, 15).unwrap(),
        )
        .unwrap()];
        let budget = BudgetCalculator::new(&salaries, &bonus, &[], &[]);

        assert_eq!(budget.total_income_for(month(2026, 12)), eur(4700));
        assert_eq!(budget.total_income_for(month(2027, 1)), eur(3200));
    }

    // Spec expense-tracking : « Charge annuelle » et « Charge terminée »
    #[test]
    fn fixed_charges_use_monthly_equivalents_of_active_charges() {
        let charges = [
            charge(900, Periodicity::Monthly, month(2025, 1), None),
            charge(240, Periodicity::Yearly, month(2025, 1), None),
            charge(
                50,
                Periodicity::Monthly,
                month(2025, 1),
                Some(month(2026, 3)),
            ),
        ];
        let budget = BudgetCalculator::new(&[], &[], &charges, &[]);

        assert_eq!(budget.fixed_charges_for(month(2026, 3)), eur(970));
        assert_eq!(budget.fixed_charges_for(month(2026, 4)), eur(920));
    }

    // Spec expense-tracking : « Calcul nominal »
    #[test]
    fn savings_capacity_nominal_case() {
        let salaries = [salary(2026, 1, 3200)];
        let charges = [charge(1300, Periodicity::Monthly, month(2026, 1), None)];
        let expenses = [
            expense(2026, 7, 600),
            expense(2026, 8, 750),
            expense(2026, 9, 450),
            expense(2026, 10, 999), // mois en cours : exclu de la moyenne
        ];
        let budget = BudgetCalculator::new(&salaries, &[], &charges, &expenses);

        assert_eq!(budget.average_monthly_expenses(month(2026, 10)), eur(600));
        assert_eq!(budget.savings_capacity(month(2026, 10)), eur(1300));
    }

    #[test]
    fn months_before_first_expense_are_ignored() {
        let expenses = [expense(2026, 9, 450), expense(2026, 10, 80)];
        let budget = BudgetCalculator::new(&[], &[], &[], &expenses);

        assert_eq!(budget.average_monthly_expenses(month(2026, 10)), eur(450));
    }

    #[test]
    fn current_month_is_used_when_no_complete_month_exists() {
        let expenses = [expense(2026, 10, 80), expense(2026, 10, 20)];
        let budget = BudgetCalculator::new(&[], &[], &[], &expenses);

        assert_eq!(budget.average_monthly_expenses(month(2026, 10)), eur(100));
    }

    #[test]
    fn months_without_expenses_count_as_zero_inside_window() {
        let expenses = [expense(2026, 7, 600), expense(2026, 9, 300)];
        let budget = BudgetCalculator::new(&[], &[], &[], &expenses);

        assert_eq!(budget.average_monthly_expenses(month(2026, 10)), eur(300));
    }

    #[test]
    fn no_expenses_means_zero_average() {
        let budget = BudgetCalculator::new(&[], &[], &[], &[]);
        assert_eq!(
            budget.average_monthly_expenses(month(2026, 10)),
            Money::ZERO
        );
    }

    // Spec expense-tracking : « Capacité négative »
    #[test]
    fn capacity_can_be_negative() {
        let salaries = [salary(2026, 1, 2000)];
        let charges = [charge(1800, Periodicity::Monthly, month(2026, 1), None)];
        let expenses = [expense(2026, 9, 500)];
        let budget = BudgetCalculator::new(&salaries, &[], &charges, &expenses);

        assert_eq!(budget.savings_capacity(month(2026, 10)), eur(-300));
    }

    #[test]
    fn category_total_filters_by_category_and_month() {
        let transport = CategoryId::generate();
        let date = NaiveDate::from_ymd_opt(2026, 10, 3).unwrap();
        let expenses = [
            Expense::new(ExpenseId::generate(), eur(30), transport, date, None).unwrap(),
            Expense::new(ExpenseId::generate(), eur(12), transport, date, None).unwrap(),
            expense(2026, 10, 99),
        ];
        let budget = BudgetCalculator::new(&[], &[], &[], &expenses);

        assert_eq!(
            budget.expenses_for_category(month(2026, 10), transport),
            eur(42)
        );
        assert_eq!(budget.expenses_for(month(2026, 10)), eur(141));
    }

    // Spec account-transfers : « Sortie vers le compte joint »
    #[test]
    fn external_transfers_count_in_the_budget() {
        use crate::transfer::{Endpoint, Schedule, Transfer};
        use crate::{AccountId, ExternalId, TransferId};
        use chrono::{TimeZone, Utc};
        use rust_decimal_macros::dec;

        let checking = Endpoint::Account(AccountId::generate());
        let ldds = Endpoint::Account(AccountId::generate());
        let joint = Endpoint::External(ExternalId::generate());
        let transfer = |from, to, amount: Decimal, schedule| {
            Transfer::new(
                TransferId::generate(),
                from,
                to,
                Money::new(amount),
                Money::new(amount),
                None,
                schedule,
                Utc.with_ymd_and_hms(2026, 10, 1, 9, 0, 0).unwrap(),
            )
            .unwrap()
        };
        let transfers = [
            transfer(
                ldds,
                joint,
                dec!(600),
                Schedule::Once {
                    date: NaiveDate::from_ymd_opt(2026, 10, 12).unwrap(),
                },
            ),
            transfer(
                checking,
                joint,
                dec!(1560),
                Schedule::Monthly {
                    day: 1,
                    from: month(2026, 1),
                    until: None,
                },
            ),
            // Entre comptes suivis : neutre.
            transfer(
                checking,
                ldds,
                dec!(850),
                Schedule::Monthly {
                    day: 1,
                    from: month(2026, 1),
                    until: None,
                },
            ),
        ];
        let salaries =
            [
                SalaryEntry::new(SalaryId::generate(), month(2026, 1), Money::new(dec!(3340)))
                    .unwrap(),
            ];
        let calculator = BudgetCalculator::new(&salaries, &[], &[], &[]).with_transfers(&transfers);

        assert_eq!(
            calculator.external_out_for(month(2026, 10)),
            Money::new(dec!(2160))
        );
        assert_eq!(calculator.external_in_for(month(2026, 10)), Money::ZERO);
        assert_eq!(
            calculator.savings_capacity(month(2026, 10)),
            Money::new(dec!(1780)),
            "3 340 − 1 560"
        );
    }

    // Spec spending-allowance : « Mois type », « Rapatriement », « Virement payé par l'épargne »,
    // « Épargne depuis un compte non suivi »
    #[test]
    fn allowance_classifies_transfers_by_account_nature() {
        use crate::transfer::{Endpoint, Schedule, Transfer};
        use crate::{ExternalId, TransferId};
        use chrono::{TimeZone, Utc};
        use rust_decimal_macros::dec;

        let checking_id = AccountId::generate();
        let checking = Endpoint::Account(checking_id);
        let ldds = Endpoint::Account(AccountId::generate());
        let livret_a = Endpoint::Account(AccountId::generate());
        let joint = Endpoint::External(ExternalId::generate());
        let credit = Endpoint::External(ExternalId::generate());
        let untracked = Endpoint::External(ExternalId::generate());
        let monthly = Schedule::Monthly {
            day: 1,
            from: month(2026, 1),
            until: None,
        };
        let once = Schedule::Once {
            date: NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
        };
        let transfer = |from, to, amount: Decimal, schedule| {
            Transfer::new(
                TransferId::generate(),
                from,
                to,
                Money::new(amount),
                Money::new(amount),
                None,
                schedule,
                Utc.with_ymd_and_hms(2026, 10, 1, 9, 0, 0).unwrap(),
            )
            .unwrap()
        };
        let salaries =
            [
                SalaryEntry::new(SalaryId::generate(), month(2026, 1), Money::new(dec!(3340)))
                    .unwrap(),
            ];
        let checking_ids = [checking_id];
        let october = month(2026, 10);
        let today = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();

        let typical = [
            transfer(checking, joint, dec!(1560), monthly),
            transfer(checking, credit, dec!(686), monthly),
            transfer(checking, ldds, dec!(850), monthly),
        ];
        let calculator = BudgetCalculator::new(&salaries, &[], &[], &[])
            .with_transfers(&typical)
            .with_checking(&checking_ids);
        assert_eq!(
            calculator.allowance(october, today).envelope,
            Money::new(dec!(244))
        );

        let mut more = typical.to_vec();
        more.push(transfer(ldds, checking, dec!(100), once));
        more.push(transfer(ldds, joint, dec!(600), once));
        more.push(transfer(untracked, livret_a, dec!(200), monthly));
        let calculator = BudgetCalculator::new(&salaries, &[], &[], &[])
            .with_transfers(&more)
            .with_checking(&checking_ids);
        let allowance = calculator.allowance(october, today);
        assert_eq!(allowance.sources.repatriated, Money::new(dec!(100)));
        assert_eq!(allowance.sources.scheduled_savings, Money::new(dec!(1050)));
        assert_eq!(allowance.sources.external_out, Money::new(dec!(2246)));
        assert_eq!(allowance.envelope, Money::new(dec!(144)), "244 + 100 − 200");
        assert_eq!(calculator.external_in_for(october), Money::ZERO);
        assert_eq!(
            calculator.savings_capacity(october),
            Money::new(dec!(1094)),
            "3 340 − 1 560 − 686 : l'épargne depuis le compte non suivi n'est pas un revenu"
        );
    }
}
