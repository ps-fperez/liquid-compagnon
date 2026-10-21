//! Projection de l'épargne à 3, 6 et 12 mois.

use std::sync::Arc;

use lc_domain::account::Scenario;
use lc_domain::projection::{Projection, ProjectionInput, project_all};
use lc_domain::{Money, Month};

use crate::AppResult;
use crate::budget::summarize;
use crate::flows::{add_projection_flows, build_ledger};
use crate::ports::{AccountRepository, BudgetRepository, Clock, TransferRepository};
use crate::pricing::{PricingIssue, PricingService, value_active_accounts};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionReport {
    /// Pessimiste, neutre, optimiste — sur 12 mois.
    pub scenarios: [Projection; 3],
    pub savings_capacity: Money,
    pub total_contributions: Money,
    /// Montant dont les versements programmés dépassent la capacité d'épargne.
    pub contributions_excess: Option<Money>,
    pub pricing_issues: Vec<PricingIssue>,
}

impl ProjectionReport {
    pub fn scenario(&self, scenario: Scenario) -> &Projection {
        let index = Scenario::ALL
            .iter()
            .position(|s| *s == scenario)
            .expect("scénario connu");
        &self.scenarios[index]
    }
}

pub struct ProjectionUseCases {
    accounts: Arc<dyn AccountRepository>,
    budget: Arc<dyn BudgetRepository>,
    transfers: Arc<dyn TransferRepository>,
    pricing: Arc<PricingService>,
    clock: Arc<dyn Clock>,
}

impl ProjectionUseCases {
    pub const MONTHS: u32 = 12;

    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        budget: Arc<dyn BudgetRepository>,
        transfers: Arc<dyn TransferRepository>,
        pricing: Arc<PricingService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            accounts,
            budget,
            transfers,
            pricing,
            clock,
        }
    }

    pub async fn projection(&self) -> AppResult<ProjectionReport> {
        let today = self.clock.today();
        let accounts = self.accounts.list()?;
        let (valued, pricing_issues) = value_active_accounts(
            accounts.clone(),
            &self.pricing,
            &build_ledger(self.budget.as_ref(), self.transfers.as_ref(), today)?,
            false,
        )
        .await?;
        let budget = summarize(
            self.budget.as_ref(),
            self.transfers.as_ref(),
            Month::of(today),
        )?;
        let mut input = ProjectionInput::from_accounts(
            valued
                .iter()
                .map(|(account, valuation)| (account, valuation.value)),
            budget.savings_capacity,
            Month::of(today),
        );
        add_projection_flows(&mut input, &accounts, &self.transfers.transfers()?);
        Ok(ProjectionReport {
            scenarios: project_all(&input, Self::MONTHS),
            savings_capacity: budget.savings_capacity,
            total_contributions: input.total_contributions(),
            contributions_excess: input.contributions_excess(),
            pricing_issues,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::NewAccount;
    use crate::budget::ChargeInput;
    use crate::testing::TestEnv;
    use crate::transfers::TransferInput;
    use lc_domain::account::AccountType;
    use lc_domain::budget::Periodicity;
    use lc_domain::projection::Horizon;
    use lc_domain::transfer::{Endpoint, Schedule};
    use pollster::block_on;
    use rust_decimal_macros::dec;

    fn month(year: i32, m: u32) -> Month {
        Month::new(year, m).unwrap()
    }

    fn setup_budget(env: &TestEnv, salary: Money, charges: Money) {
        let budget = env.budget();
        budget.ensure_default_categories().unwrap();
        let housing = budget.categories(false).unwrap()[0].id();
        budget.upsert_salary(None, month(2026, 1), salary).unwrap();
        budget
            .upsert_charge(
                None,
                ChargeInput {
                    label: "Loyer".into(),
                    amount: charges,
                    category: housing,
                    periodicity: Periodicity::Monthly,
                    start: month(2026, 1),
                    end: None,
                    debit: None,
                },
            )
            .unwrap();
    }

    /// Ouvre un compte alimenté chaque mois par virement depuis le compte courant (créé au besoin).
    fn open_with_contribution(env: &TestEnv, account_type: AccountType, contribution: Money) {
        let accounts = env.accounts();
        let open = |account_type: AccountType| {
            accounts
                .create_account(NewAccount {
                    name: account_type.label().into(),
                    account_type,
                    color: None,
                })
                .unwrap()
        };
        let checking = accounts
            .list_accounts(false)
            .unwrap()
            .into_iter()
            .find(|a| a.account_type() == AccountType::Checking)
            .map_or_else(|| open(AccountType::Checking), |a| a.id());
        let target = open(account_type);
        env.transfers()
            .create_transfer(TransferInput {
                from: Endpoint::Account(checking),
                to: Endpoint::Account(target),
                sent: contribution,
                received: None,
                label: None,
                schedule: Schedule::Monthly {
                    day: 1,
                    from: Month::of(env.clock.today()),
                    until: None,
                },
            })
            .unwrap();
    }

    // Spec savings-projection : « Reliquat »
    #[test]
    fn unprogrammed_capacity_goes_to_unallocated_savings() {
        let env = TestEnv::new();
        setup_budget(&env, Money::new(dec!(3200)), Money::new(dec!(1900)));
        open_with_contribution(&env, AccountType::Pea, Money::new(dec!(600)));
        open_with_contribution(&env, AccountType::LivretA, Money::new(dec!(300)));

        let report = block_on(env.projection().projection()).unwrap();

        assert_eq!(report.savings_capacity, Money::new(dec!(1300)));
        assert_eq!(report.total_contributions, Money::new(dec!(900)));
        assert_eq!(report.contributions_excess, None);
        let neutral = report.scenario(Scenario::Neutral);
        assert_eq!(
            neutral.unallocated_at(Horizon::ThreeMonths.months()),
            Money::new(dec!(1200))
        );
    }

    // Spec savings-projection : « Versements excessifs »
    #[test]
    fn reports_contributions_above_capacity() {
        let env = TestEnv::new();
        setup_budget(&env, Money::new(dec!(3200)), Money::new(dec!(1900)));
        open_with_contribution(&env, AccountType::Pea, Money::new(dec!(1500)));

        let report = block_on(env.projection().projection()).unwrap();

        assert_eq!(report.contributions_excess, Some(Money::new(dec!(200))));
    }

    // Spec savings-projection : « Lecture d'une projection »
    #[test]
    fn three_scenarios_over_twelve_months() {
        let env = TestEnv::new();
        setup_budget(&env, Money::new(dec!(3200)), Money::new(dec!(1900)));
        open_with_contribution(&env, AccountType::Crypto, Money::new(dec!(100)));

        let report = block_on(env.projection().projection()).unwrap();

        for projection in &report.scenarios {
            assert_eq!(projection.months(), 12);
        }
        let at_year = |s| report.scenario(s).total_at(12);
        assert!(at_year(Scenario::Pessimistic) < at_year(Scenario::Neutral));
        assert!(at_year(Scenario::Neutral) < at_year(Scenario::Optimistic));
    }
}
