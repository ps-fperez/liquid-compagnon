//! Projection de l'épargne mois par mois, selon trois scénarios de rendement.

use crate::account::{Account, AccountKind, Scenario, ScenarioRates};
use crate::math::monthly_factor;
use crate::{AccountId, Money};

/// Horizons proposés à l'utilisateur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Horizon {
    ThreeMonths,
    SixMonths,
    TwelveMonths,
}

impl Horizon {
    pub const ALL: [Horizon; 3] = [
        Horizon::ThreeMonths,
        Horizon::SixMonths,
        Horizon::TwelveMonths,
    ];

    pub const fn months(&self) -> u32 {
        match self {
            Horizon::ThreeMonths => 3,
            Horizon::SixMonths => 6,
            Horizon::TwelveMonths => 12,
        }
    }
}

/// Paramètres de projection d'un compte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedAccount {
    pub account_id: AccountId,
    pub current_value: Money,
    pub monthly_contribution: Money,
    pub rates: ScenarioRates,
    /// Plafond de versement (Livret A, LDDS) : au-delà, les versements programmés sont déroutés
    /// vers l'épargne non affectée.
    pub deposit_ceiling: Option<Money>,
}

impl ProjectedAccount {
    pub fn from_account(account: &Account, current_value: Money) -> Self {
        let deposit_ceiling = match account.kind() {
            AccountKind::Savings(details) => Some(details.deposit_ceiling()),
            AccountKind::Securities(_)
            | AccountKind::EmployeeSavings(_)
            | AccountKind::Checking(_) => None,
        };
        Self {
            account_id: account.id(),
            current_value,
            monthly_contribution: account.monthly_contribution(),
            rates: *account.scenario_rates(),
            deposit_ceiling,
        }
    }
}

/// Données d'entrée d'une projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionInput {
    pub accounts: Vec<ProjectedAccount>,
    /// Capacité d'épargne mensuelle (voir `BudgetCalculator::savings_capacity`).
    pub savings_capacity: Money,
}

impl ProjectionInput {
    /// Construit l'entrée à partir des comptes et de leur valeur actuelle ; les comptes archivés
    /// sont ignorés.
    pub fn from_accounts<'a>(
        valued_accounts: impl IntoIterator<Item = (&'a Account, Money)>,
        savings_capacity: Money,
    ) -> Self {
        Self {
            accounts: valued_accounts
                .into_iter()
                .filter(|(account, _)| !account.is_archived())
                .map(|(account, value)| ProjectedAccount::from_account(account, value))
                .collect(),
            savings_capacity,
        }
    }

    pub fn total_contributions(&self) -> Money {
        self.accounts.iter().map(|a| a.monthly_contribution).sum()
    }

    /// Montant dont les versements programmés dépassent la capacité d'épargne, s'il y en a un.
    pub fn contributions_excess(&self) -> Option<Money> {
        let excess = self.total_contributions() - self.savings_capacity;
        excess.is_positive().then_some(excess)
    }

    /// Part de la capacité d'épargne non affectée à un compte (jamais négative).
    fn unallocated_monthly(&self) -> Money {
        (self.savings_capacity - self.total_contributions()).max(Money::ZERO)
    }
}

/// Valeurs projetées d'un compte : l'indice `m` est la valeur dans `m` mois (0 = aujourd'hui).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountTrajectory {
    pub account_id: AccountId,
    pub values: Vec<Money>,
}

/// Résultat d'une projection pour un scénario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    pub scenario: Scenario,
    pub accounts: Vec<AccountTrajectory>,
    /// Épargne non affectée cumulée (rendement nul), indice = nombre de mois.
    pub unallocated: Vec<Money>,
    /// Patrimoine total projeté (comptes + épargne non affectée), indice = nombre de mois.
    pub totals: Vec<Money>,
}

impl Projection {
    pub fn months(&self) -> u32 {
        (self.totals.len() - 1) as u32
    }

    /// Patrimoine total dans `months` mois (dernière valeur si l'horizon dépasse la projection).
    pub fn total_at(&self, months: u32) -> Money {
        at(&self.totals, months)
    }

    pub fn account_at(&self, account_id: AccountId, months: u32) -> Option<Money> {
        self.accounts
            .iter()
            .find(|trajectory| trajectory.account_id == account_id)
            .map(|trajectory| at(&trajectory.values, months))
    }

    pub fn unallocated_at(&self, months: u32) -> Money {
        at(&self.unallocated, months)
    }
}

fn at(values: &[Money], months: u32) -> Money {
    let index = (months as usize).min(values.len() - 1);
    values[index]
}

/// Projette l'épargne sur `months` mois pour un scénario.
///
/// Chaque mois : valeur(m+1) = valeur(m) × (1 + r)^(1/12) + versement, où le versement d'un
/// livret est limité à la marge restant sous son plafond (calculée avant intérêts) ; l'excédent et
/// la part de la capacité d'épargne non programmée alimentent l'épargne non affectée.
pub fn project(input: &ProjectionInput, scenario: Scenario, months: u32) -> Projection {
    let factors: Vec<_> = input
        .accounts
        .iter()
        .map(|account| monthly_factor(account.rates.rate(scenario)))
        .collect();
    let mut trajectories: Vec<AccountTrajectory> = input
        .accounts
        .iter()
        .map(|account| AccountTrajectory {
            account_id: account.account_id,
            values: vec![account.current_value],
        })
        .collect();
    let mut unallocated = vec![Money::ZERO];
    let base_unallocated = input.unallocated_monthly();

    for _ in 0..months {
        let mut unallocated_deposit = base_unallocated;
        for ((account, factor), trajectory) in
            input.accounts.iter().zip(&factors).zip(&mut trajectories)
        {
            let value = *trajectory.values.last().expect("valeur initiale présente");
            let deposit = match account.deposit_ceiling {
                Some(ceiling) => account
                    .monthly_contribution
                    .min((ceiling - value).max(Money::ZERO)),
                None => account.monthly_contribution,
            };
            unallocated_deposit += account.monthly_contribution - deposit;
            trajectory.values.push(value.times(*factor) + deposit);
        }
        let previous = *unallocated.last().expect("valeur initiale présente");
        unallocated.push(previous + unallocated_deposit);
    }

    let totals = (0..=months as usize)
        .map(|m| unallocated[m] + trajectories.iter().map(|t| t.values[m]).sum::<Money>())
        .collect();

    Projection {
        scenario,
        accounts: trajectories,
        unallocated,
        totals,
    }
}

/// Projette les trois scénarios (pessimiste, neutre, optimiste).
pub fn project_all(input: &ProjectionInput, months: u32) -> [Projection; 3] {
    Scenario::ALL.map(|scenario| project(input, scenario, months))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::AccountType;
    use crate::{Label, Rate, defaults};
    use chrono::NaiveDate;
    use proptest::prelude::*;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    fn eur(amount: Decimal) -> Money {
        Money::new(amount)
    }

    fn projected(
        value: Decimal,
        contribution: Decimal,
        rate_percent: Decimal,
        ceiling: Option<Decimal>,
    ) -> ProjectedAccount {
        ProjectedAccount {
            account_id: AccountId::generate(),
            current_value: eur(value),
            monthly_contribution: eur(contribution),
            rates: ScenarioRates::uniform(Rate::from_percent(rate_percent).unwrap()),
            deposit_ceiling: ceiling.map(eur),
        }
    }

    fn input(accounts: Vec<ProjectedAccount>, capacity: Decimal) -> ProjectionInput {
        ProjectionInput {
            accounts,
            savings_capacity: eur(capacity),
        }
    }

    // Spec savings-projection : « Livret sans versement »
    #[test]
    fn savings_without_contribution_compound_to_annual_rate() {
        let livret = projected(dec!(10000), dec!(0), dec!(3), None);
        let id = livret.account_id;

        let projection = project(&input(vec![livret], dec!(0)), Scenario::Neutral, 12);

        assert_eq!(
            projection.account_at(id, 12).unwrap().round_to_cents(),
            eur(dec!(10300))
        );
    }

    // Spec savings-projection : « PEA avec versements »
    #[test]
    fn contributions_are_added_monthly() {
        let pea = projected(dec!(10000), dec!(300), dec!(0), None);
        let id = pea.account_id;

        let projection = project(&input(vec![pea], dec!(300)), Scenario::Neutral, 6);

        assert_eq!(projection.account_at(id, 6), Some(eur(dec!(11800))));
        assert_eq!(projection.account_at(id, 3), Some(eur(dec!(10900))));
    }

    // Spec savings-projection : « Plafond atteint »
    #[test]
    fn ceiling_diverts_excess_contribution_to_unallocated() {
        let ldds = projected(dec!(11900), dec!(200), dec!(1.7), Some(dec!(12000)));
        let id = ldds.account_id;
        let factor = monthly_factor(Rate::from_percent(dec!(1.7)).unwrap());

        let projection = project(&input(vec![ldds], dec!(200)), Scenario::Neutral, 2);

        assert_eq!(
            projection.account_at(id, 1),
            Some(eur(dec!(11900) * factor + dec!(100)))
        );
        assert_eq!(projection.unallocated_at(1), eur(dec!(100)));
        // Au-dessus du plafond ensuite : plus aucun versement, tout part en non affecté.
        assert_eq!(projection.unallocated_at(2), eur(dec!(300)));
    }

    // Spec savings-projection : « Reliquat »
    #[test]
    fn remaining_capacity_accumulates_as_unallocated() {
        let pea = projected(dec!(0), dec!(600), dec!(0), None);
        let livret = projected(dec!(0), dec!(300), dec!(0), Some(dec!(22950)));

        let projection = project(&input(vec![pea, livret], dec!(1300)), Scenario::Neutral, 3);

        assert_eq!(projection.unallocated_at(1), eur(dec!(400)));
        assert_eq!(projection.unallocated_at(3), eur(dec!(1200)));
        assert_eq!(projection.total_at(3), eur(dec!(3900)));
    }

    // Spec savings-projection : « Versements excessifs »
    #[test]
    fn detects_contributions_above_capacity() {
        let accounts = vec![
            projected(dec!(0), dec!(1000), dec!(0), None),
            projected(dec!(0), dec!(500), dec!(0), None),
        ];
        let excessive = input(accounts, dec!(1300));

        assert_eq!(excessive.contributions_excess(), Some(eur(dec!(200))));
        assert_eq!(
            project(&excessive, Scenario::Neutral, 1).unallocated_at(1),
            Money::ZERO
        );
        assert_eq!(input(vec![], dec!(1300)).contributions_excess(), None);
    }

    #[test]
    fn three_scenarios_are_ordered_for_market_accounts() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        let pea = Account::open(
            AccountId::generate(),
            Label::new("PEA").unwrap(),
            defaults::color(AccountType::Pea),
            AccountType::Pea,
            today,
        );
        let input = ProjectionInput::from_accounts([(&pea, eur(dec!(14510)))], eur(dec!(0)));

        let [pessimistic, neutral, optimistic] = project_all(&input, 12);

        for horizon in Horizon::ALL {
            let months = horizon.months();
            assert!(pessimistic.total_at(months) <= neutral.total_at(months));
            assert!(neutral.total_at(months) < optimistic.total_at(months));
        }
        assert_eq!(pessimistic.total_at(12), eur(dec!(14510)));
    }

    #[test]
    fn archived_accounts_are_not_projected() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        let mut crypto = Account::open(
            AccountId::generate(),
            Label::new("Crypto").unwrap(),
            defaults::color(AccountType::Crypto),
            AccountType::Crypto,
            today,
        );
        crypto.archive();

        let input = ProjectionInput::from_accounts([(&crypto, eur(dec!(8702)))], eur(dec!(0)));

        assert!(input.accounts.is_empty());
        assert_eq!(
            project(&input, Scenario::Neutral, 3).total_at(3),
            Money::ZERO
        );
    }

    #[test]
    fn horizon_beyond_projection_returns_last_value() {
        let projection = project(
            &input(
                vec![projected(dec!(100), dec!(10), dec!(0), None)],
                dec!(10),
            ),
            Scenario::Neutral,
            3,
        );
        assert_eq!(projection.months(), 3);
        assert_eq!(projection.total_at(12), projection.total_at(3));
    }

    proptest! {
        #[test]
        fn zero_rate_projection_is_linear(value in 0i64..10_000_000, contribution in 0i64..500_000, months in 0u32..=12) {
            let account = projected(Decimal::new(value, 2), Decimal::new(contribution, 2), dec!(0), None);
            let id = account.account_id;
            let projection = project(&input(vec![account], Decimal::new(contribution, 2)), Scenario::Neutral, months);
            let expected = Money::from_cents(value + contribution * i64::from(months));
            prop_assert_eq!(projection.account_at(id, months), Some(expected));
        }

        #[test]
        fn savings_never_receive_deposits_above_ceiling(value in 0i64..2_000_000, contribution in 0i64..100_000) {
            let ceiling = dec!(12000);
            let account = projected(Decimal::new(value, 2), Decimal::new(contribution, 2), dec!(0), Some(ceiling));
            let id = account.account_id;
            let projection = project(&input(vec![account], Decimal::new(contribution, 2)), Scenario::Neutral, 12);
            let final_value = projection.account_at(id, 12).unwrap();
            prop_assert!(final_value <= eur(ceiling).max(eur(Decimal::new(value, 2))));
            // Rien ne se perd : ce qui n'est pas versé sur le livret est en épargne non affectée.
            prop_assert_eq!(projection.total_at(12), Money::from_cents(value + contribution * 12));
        }
    }
}
