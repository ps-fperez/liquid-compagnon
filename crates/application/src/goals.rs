//! Objectifs d'épargne (spec savings-goals) : création, modification, suppression, et rapport
//! (progression, effort, versé, dates d'arrivée par scénario sur 10 ans).

use std::collections::HashMap;
use std::sync::Arc;

use lc_domain::goal::{GoalStatus, SavingsGoal};
use lc_domain::projection::project_all;
use lc_domain::{AccountId, Decimal, GoalId, Label, Money, Month};

use crate::ports::{
    AccountRepository, BudgetRepository, Clock, PlanningRepository, TransferRepository,
};
use crate::pricing::{PricingIssue, PricingService};
use crate::projection::prepare;
use crate::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalInput {
    pub name: String,
    pub target: Money,
    pub deadline: Option<Month>,
    pub accounts: Vec<AccountId>,
}

/// Situation d'un objectif aujourd'hui.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalProgress {
    pub goal: SavingsGoal,
    /// Valeur actuelle des comptes liés actifs.
    pub current: Money,
    pub percent: Decimal,
    /// Reste à épargner (zéro si atteint).
    pub remaining: Money,
    /// Effort mensuel pour tenir l'échéance (si elle est fixée), sans rendement.
    pub required_monthly: Option<Money>,
    /// Équivalent mensuel net des virements récurrents vers les comptes liés.
    pub contributed_monthly: Money,
    /// Mois d'arrivée : pessimiste, neutre, optimiste (`None` au-delà de 10 ans).
    pub arrivals: [Option<Month>; 3],
    pub status: GoalStatus,
    /// La cible vient d'être atteinte pour la première fois : à féliciter une fois.
    pub just_reached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalsReport {
    pub goals: Vec<GoalProgress>,
    pub pricing_issues: Vec<PricingIssue>,
}

pub struct GoalUseCases {
    planning: Arc<dyn PlanningRepository>,
    accounts: Arc<dyn AccountRepository>,
    budget: Arc<dyn BudgetRepository>,
    transfers: Arc<dyn TransferRepository>,
    pricing: Arc<PricingService>,
    clock: Arc<dyn Clock>,
}

impl GoalUseCases {
    /// Horizon des dates d'arrivée (la projection affichée reste à 12 mois).
    pub const HORIZON_MONTHS: u32 = 120;

    pub fn new(
        planning: Arc<dyn PlanningRepository>,
        accounts: Arc<dyn AccountRepository>,
        budget: Arc<dyn BudgetRepository>,
        transfers: Arc<dyn TransferRepository>,
        pricing: Arc<PricingService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            planning,
            accounts,
            budget,
            transfers,
            pricing,
            clock,
        }
    }

    pub fn goals(&self) -> AppResult<Vec<SavingsGoal>> {
        Ok(self.planning.goals()?)
    }

    pub fn create_goal(&self, input: GoalInput) -> AppResult<GoalId> {
        self.check_accounts(&input.accounts)?;
        let goal = SavingsGoal::new(
            GoalId::generate(),
            Label::new(&input.name)?,
            input.target,
            input.deadline,
            input.accounts,
        )?;
        self.planning.save_goal(&goal)?;
        Ok(goal.id())
    }

    pub fn update_goal(&self, id: GoalId, input: GoalInput) -> AppResult<()> {
        self.check_accounts(&input.accounts)?;
        let mut goal = self
            .planning
            .goals()?
            .into_iter()
            .find(|g| g.id() == id)
            .ok_or(AppError::NotFound("Objectif"))?;
        goal.update(
            Label::new(&input.name)?,
            input.target,
            input.deadline,
            input.accounts,
        )?;
        Ok(self.planning.save_goal(&goal)?)
    }

    pub fn delete_goal(&self, id: GoalId) -> AppResult<()> {
        if self.planning.delete_goal(id)? {
            Ok(())
        } else {
            Err(AppError::NotFound("Objectif"))
        }
    }

    /// Les comptes d'un objectif doivent exister et être actifs.
    fn check_accounts(&self, ids: &[AccountId]) -> AppResult<()> {
        let accounts = self.accounts.list()?;
        let usable = |id: &AccountId| accounts.iter().any(|a| a.id() == *id && !a.is_archived());
        if ids.iter().all(usable) {
            Ok(())
        } else {
            Err(AppError::UnavailableAccount)
        }
    }

    pub async fn report(&self) -> AppResult<GoalsReport> {
        let today = self.clock.today();
        let start = Month::of(today);
        let prepared = prepare(
            self.accounts.as_ref(),
            self.budget.as_ref(),
            self.transfers.as_ref(),
            &self.pricing,
            today,
        )
        .await?;
        let scenarios = project_all(&prepared.input, Self::HORIZON_MONTHS);
        let values: HashMap<AccountId, Money> = prepared
            .valued
            .iter()
            .map(|(account, valuation)| (account.id(), valuation.value))
            .collect();
        let monthly_flows: HashMap<AccountId, Money> = prepared
            .input
            .accounts
            .iter()
            .map(|account| {
                let flows = account
                    .flows
                    .iter()
                    .filter(|f| f.until.is_none_or(|until| until >= start))
                    .map(|f| f.monthly_equivalent())
                    .sum();
                (account.account_id, flows)
            })
            .collect();

        let mut goals = Vec::new();
        for mut goal in self.planning.goals()? {
            let linked = |map: &HashMap<AccountId, Money>| -> Money {
                goal.accounts().iter().filter_map(|id| map.get(id)).sum()
            };
            let current = linked(&values);
            let contributed_monthly = linked(&monthly_flows).round_to_cents();
            let arrivals = scenarios.each_ref().map(|p| goal.arrival(p, start));
            let just_reached = goal.record_progress(current, today);
            if just_reached {
                self.planning.save_goal(&goal)?;
            }
            goals.push(GoalProgress {
                current,
                percent: goal.progress_percent(current),
                remaining: (goal.target() - current).max(Money::ZERO),
                required_monthly: goal.required_monthly(current, today),
                contributed_monthly,
                arrivals,
                status: goal.status(current, arrivals[1]),
                just_reached,
                goal,
            });
        }
        Ok(GoalsReport {
            goals,
            pricing_issues: prepared.pricing_issues,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::NewAccount;
    use crate::testing::TestEnv;
    use crate::transfers::TransferInput;
    use lc_domain::DomainError;
    use lc_domain::account::AccountType;
    use lc_domain::transfer::{Endpoint, Schedule};
    use pollster::block_on;
    use rust_decimal_macros::dec;

    fn create(env: &TestEnv, name: &str, account_type: AccountType, balance: Decimal) -> AccountId {
        let id = env
            .accounts()
            .create_account(NewAccount {
                name: name.into(),
                account_type,
                color: None,
            })
            .unwrap();
        if !balance.is_zero() {
            env.accounts()
                .update_balance(id, Money::new(balance))
                .unwrap();
        }
        id
    }

    fn input(target: Decimal, deadline: Option<Month>, accounts: Vec<AccountId>) -> GoalInput {
        GoalInput {
            name: "Apport immobilier".into(),
            target: Money::new(target),
            deadline,
            accounts,
        }
    }

    // Spec savings-goals : « Apport immobilier », « Effort mensuel », « Sans compte »
    #[test]
    fn report_shows_progress_effort_and_contributions() {
        let env = TestEnv::new();
        let checking = create(&env, "Compte courant", AccountType::Checking, Decimal::ZERO);
        let livret = create(&env, "Livret A", AccountType::LivretA, dec!(12840));
        let assurance = create(&env, "Assurance-vie", AccountType::AssuranceVie, dec!(5760));
        let today = env.clock.today();
        env.transfers()
            .create_transfer(TransferInput {
                from: Endpoint::Account(checking),
                to: Endpoint::Account(assurance),
                sent: Money::new(dec!(400)),
                received: None,
                label: None,
                schedule: Schedule::Monthly {
                    day: 1,
                    from: Month::of(today),
                    until: None,
                },
            })
            .unwrap();
        let goals = env.goals();
        let deadline = Month::of(today).plus_months(26);
        goals
            .create_goal(input(dec!(30000), Some(deadline), vec![livret, assurance]))
            .unwrap();

        let report = block_on(goals.report()).unwrap();
        let progress = &report.goals[0];
        assert_eq!(progress.current, Money::new(dec!(18600)));
        assert_eq!(progress.percent, dec!(62.0));
        assert_eq!(progress.required_monthly, Some(Money::new(dec!(438.46))));
        assert_eq!(progress.contributed_monthly, Money::new(dec!(400)));
        assert!(progress.arrivals[2].is_some());
        assert!(!progress.just_reached);

        assert_eq!(
            goals.create_goal(input(dec!(100), None, vec![])),
            Err(AppError::Domain(DomainError::GoalWithoutAccount))
        );
        env.accounts().archive_account(livret).unwrap();
        assert_eq!(
            goals.create_goal(input(dec!(100), None, vec![livret])),
            Err(AppError::UnavailableAccount)
        );
    }

    // Spec savings-goals : « Première atteinte »
    #[test]
    fn reaching_a_goal_is_reported_once() {
        let env = TestEnv::new();
        let livret = create(&env, "Livret A", AccountType::LivretA, dec!(4050));
        let goals = env.goals();
        goals
            .create_goal(GoalInput {
                name: "Voyage au Japon".into(),
                target: Money::new(dec!(4000)),
                deadline: None,
                accounts: vec![livret],
            })
            .unwrap();
        let first = block_on(goals.report()).unwrap();
        assert!(first.goals[0].just_reached);
        assert_eq!(first.goals[0].status, GoalStatus::Reached);
        assert_eq!(first.goals[0].remaining, Money::ZERO);
        let second = block_on(goals.report()).unwrap();
        assert!(!second.goals[0].just_reached);
    }
}
