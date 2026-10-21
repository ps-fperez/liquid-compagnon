//! Objectifs d'épargne (spec savings-goals) : un objectif suit des comptes existants, sa
//! progression est leur valeur ; effort mensuel, dates d'arrivée par scénario, atteinte.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::projection::Projection;
use crate::{AccountId, DomainError, GoalId, Label, Money, Month};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavingsGoal {
    id: GoalId,
    name: Label,
    target: Money,
    deadline: Option<Month>,
    accounts: Vec<AccountId>,
    /// Date de la première atteinte de la cible (jamais effacée ensuite).
    reached_on: Option<NaiveDate>,
}

/// État d'un objectif par rapport à son échéance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalStatus {
    Reached,
    OnTrack,
    Late,
    /// Pas d'échéance : seule la progression compte.
    Open,
}

impl SavingsGoal {
    pub fn new(
        id: GoalId,
        name: Label,
        target: Money,
        deadline: Option<Month>,
        accounts: Vec<AccountId>,
    ) -> Result<Self, DomainError> {
        let mut goal = Self {
            id,
            name,
            target: target.validate_entry()?,
            deadline,
            accounts: Vec::new(),
            reached_on: None,
        };
        goal.set_accounts(accounts)?;
        Ok(goal)
    }

    /// Reconstruction depuis le stockage (date d'atteinte comprise).
    pub fn with_reached_on(mut self, reached_on: Option<NaiveDate>) -> Self {
        self.reached_on = reached_on;
        self
    }

    pub fn id(&self) -> GoalId {
        self.id
    }

    pub fn name(&self) -> &Label {
        &self.name
    }

    pub fn target(&self) -> Money {
        self.target
    }

    pub fn deadline(&self) -> Option<Month> {
        self.deadline
    }

    pub fn accounts(&self) -> &[AccountId] {
        &self.accounts
    }

    pub fn reached_on(&self) -> Option<NaiveDate> {
        self.reached_on
    }

    pub fn update(
        &mut self,
        name: Label,
        target: Money,
        deadline: Option<Month>,
        accounts: Vec<AccountId>,
    ) -> Result<(), DomainError> {
        self.target = target.validate_entry()?;
        self.set_accounts(accounts)?;
        self.name = name;
        self.deadline = deadline;
        Ok(())
    }

    fn set_accounts(&mut self, mut accounts: Vec<AccountId>) -> Result<(), DomainError> {
        let mut seen = Vec::new();
        accounts.retain(|id| {
            let fresh = !seen.contains(id);
            seen.push(*id);
            fresh
        });
        if accounts.is_empty() {
            return Err(DomainError::GoalWithoutAccount);
        }
        self.accounts = accounts;
        Ok(())
    }

    pub fn involves(&self, account: AccountId) -> bool {
        self.accounts.contains(&account)
    }

    /// Retire un compte (clôturé ou supprimé). L'objectif peut alors n'en suivre aucun.
    pub fn forget_account(&mut self, account: AccountId) {
        self.accounts.retain(|id| *id != account);
    }

    /// Progression en pourcentage de la cible (peut dépasser 100).
    pub fn progress_percent(&self, current: Money) -> Decimal {
        (current.amount() / self.target.amount() * Decimal::ONE_HUNDRED).round_dp(1)
    }

    pub fn is_reached(&self, current: Money) -> bool {
        current >= self.target
    }

    /// Enregistre la première atteinte ; renvoie `true` si elle vient d'avoir lieu.
    pub fn record_progress(&mut self, current: Money, today: NaiveDate) -> bool {
        if self.reached_on.is_none() && self.is_reached(current) {
            self.reached_on = Some(today);
            return true;
        }
        false
    }

    /// Effort mensuel pour tenir l'échéance, sans rendement ; le mois de l'échéance compte.
    pub fn required_monthly(&self, current: Money, today: NaiveDate) -> Option<Money> {
        let deadline = self.deadline?;
        if self.is_reached(current) {
            return Some(Money::ZERO);
        }
        let months = Month::of(today).months_until(deadline).max(1);
        (self.target - current)
            .divided_by(months.unsigned_abs())
            .map(|m| m.round_to_cents())
    }

    /// Premier mois où la somme des comptes liés atteint la cible dans `projection` (mois 0 =
    /// `start`), dans l'horizon de la projection.
    pub fn arrival(&self, projection: &Projection, start: Month) -> Option<Month> {
        (0..=projection.months()).find_map(|index| {
            let total: Money = self
                .accounts
                .iter()
                .filter_map(|id| projection.account_at(*id, index))
                .sum();
            (total >= self.target).then(|| start.plus_months(index as i32))
        })
    }

    /// État d'après la valeur actuelle et l'arrivée dans le scénario neutre.
    pub fn status(&self, current: Money, neutral_arrival: Option<Month>) -> GoalStatus {
        if self.is_reached(current) {
            return GoalStatus::Reached;
        }
        match self.deadline {
            None => GoalStatus::Open,
            Some(deadline) => match neutral_arrival {
                Some(arrival) if arrival <= deadline => GoalStatus::OnTrack,
                _ => GoalStatus::Late,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{Account, AccountType};
    use crate::projection::{FlowTiming, ProjectedFlow, ProjectionInput, project_all};
    use crate::{Label, defaults};
    use rust_decimal_macros::dec;

    fn eur(amount: Decimal) -> Money {
        Money::new(amount)
    }

    fn month(y: i32, m: u32) -> Month {
        Month::new(y, m).unwrap()
    }

    fn goal(target: Decimal, deadline: Option<Month>, accounts: Vec<AccountId>) -> SavingsGoal {
        SavingsGoal::new(
            GoalId::generate(),
            Label::new("Apport immobilier").unwrap(),
            eur(target),
            deadline,
            accounts,
        )
        .unwrap()
    }

    // Spec savings-goals : « Apport immobilier » et « Sans compte »
    #[test]
    fn progress_is_the_value_of_linked_accounts() {
        let apport = goal(dec!(30000), None, vec![AccountId::generate()]);
        assert_eq!(apport.progress_percent(eur(dec!(18600))), dec!(62.0));
        assert_eq!(
            SavingsGoal::new(
                GoalId::generate(),
                Label::new("Voyage").unwrap(),
                eur(dec!(4000)),
                None,
                vec![],
            ),
            Err(DomainError::GoalWithoutAccount)
        );
        let id = AccountId::generate();
        assert_eq!(goal(dec!(10), None, vec![id, id]).accounts(), [id]);
    }

    // Spec savings-goals : « Effort mensuel »
    #[test]
    fn required_monthly_counts_months_until_the_deadline() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap();
        let apport = goal(
            dec!(30000),
            Some(month(2028, 12)),
            vec![AccountId::generate()],
        );
        assert_eq!(
            apport.required_monthly(eur(dec!(18600)), today),
            Some(eur(dec!(438.46)))
        );
        assert_eq!(
            goal(dec!(10), None, vec![AccountId::generate()]).required_monthly(Money::ZERO, today),
            None
        );
    }

    // Spec savings-goals : « Dans les temps grâce aux rendements », « Hors d'atteinte »
    #[test]
    fn arrival_and_status_follow_the_projection() {
        let start = month(2026, 10);
        let today = start.first_day();
        let livret = Account::open(
            AccountId::generate(),
            Label::new("Assurance-vie").unwrap(),
            defaults::color(AccountType::AssuranceVie),
            AccountType::AssuranceVie,
            today,
        );
        let mut input =
            ProjectionInput::from_accounts([(&livret, eur(dec!(18600)))], Money::ZERO, start);
        input.add_flow(
            livret.id(),
            ProjectedFlow {
                amount: eur(dec!(400)),
                timing: FlowTiming::EveryMonth,
                from: start,
                until: None,
            },
        );
        let [_, neutral, _] = project_all(&input, 120);
        let apport = goal(dec!(30000), Some(month(2028, 12)), vec![livret.id()]);
        let arrival = apport.arrival(&neutral, start).expect("atteint en 10 ans");
        assert!(arrival <= month(2028, 12), "arrivée {arrival}");
        assert_eq!(
            apport.status(eur(dec!(18600)), Some(arrival)),
            GoalStatus::OnTrack
        );
        assert_eq!(apport.status(eur(dec!(18600)), None), GoalStatus::Late);

        let lune = goal(dec!(10000000), None, vec![livret.id()]);
        assert_eq!(lune.arrival(&neutral, start), None);
        assert_eq!(lune.status(eur(dec!(18600)), None), GoalStatus::Open);
    }

    // Spec savings-goals : « Première atteinte »
    #[test]
    fn reaching_the_target_is_recorded_once() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap();
        let mut japon = goal(dec!(4000), None, vec![AccountId::generate()]);
        assert!(!japon.record_progress(eur(dec!(3990)), today));
        assert!(japon.record_progress(eur(dec!(4050)), today));
        assert!(!japon.record_progress(eur(dec!(4100)), today));
        assert_eq!(japon.reached_on(), Some(today));
        assert_eq!(japon.status(eur(dec!(4050)), None), GoalStatus::Reached);
    }
}
