//! Compte courant : un solde de référence saisi par l'utilisateur (signé : un découvert est
//! possible), que font évoluer les mouvements du grand livre postérieurs à cette référence.

use chrono::{DateTime, NaiveDate, Utc};

use crate::ledger::{BalanceReference, Movement, Statement};
use crate::{DomainError, Money};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckingDetails {
    reference: BalanceReference,
}

impl CheckingDetails {
    pub fn new(
        reference_balance: Money,
        reference_on: NaiveDate,
        reference_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            reference: BalanceReference::new(
                reference_balance.validate_signed()?,
                reference_on,
                reference_at,
            ),
        })
    }

    pub fn reference(&self) -> &BalanceReference {
        &self.reference
    }

    pub fn reference_balance(&self) -> Money {
        self.reference.amount
    }

    /// Date de la référence, dans le fuseau de l'utilisateur.
    pub fn reference_on(&self) -> NaiveDate {
        self.reference.on
    }

    pub fn reference_at(&self) -> DateTime<Utc> {
        self.reference.at
    }

    /// Solde courant et mouvements comptés (du plus récent au plus ancien).
    pub fn statement(&self, movements: &[Movement]) -> Statement {
        self.reference.statement(movements)
    }

    pub(super) fn reset(
        &mut self,
        balance: Money,
        on: NaiveDate,
        at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        *self = Self::new(balance, on, at)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{MovementKind, Timing};
    use chrono::TimeZone;
    use rust_decimal_macros::dec;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, d).unwrap()
    }

    fn at(d: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, d, hour, 0, 0).unwrap()
    }

    fn movement(kind: MovementKind, amount: Money, date: u32, recorded: DateTime<Utc>) -> Movement {
        Movement {
            kind,
            amount,
            date: day(date),
            timing: Timing::Recorded(recorded),
            label: None,
            counterpart: None,
        }
    }

    fn reference(balance: Money) -> CheckingDetails {
        CheckingDetails::new(balance, day(20), at(20, 8)).unwrap()
    }

    // Spec checking-account : « Dépenses après la référence »
    #[test]
    fn movements_after_the_reference_change_the_balance() {
        let movements = [
            movement(
                MovementKind::Expense,
                Money::new(dec!(42.50)),
                20,
                at(20, 12),
            ),
            movement(MovementKind::Income, Money::new(dec!(100)), 21, at(21, 9)),
        ];

        let statement = reference(Money::new(dec!(1000))).statement(&movements);

        assert_eq!(statement.balance, Money::new(dec!(1057.50)));
        assert_eq!(statement.movements.len(), 2);
        assert_eq!(
            statement.movements[0].date,
            day(21),
            "le plus récent d'abord"
        );
    }

    // Spec checking-account : « Recalage » et « Dépense oubliée d'avant la référence »
    #[test]
    fn movements_already_included_in_the_reference_are_ignored() {
        let movements = [
            // Saisie avant la référence : déjà dans le solde saisi.
            movement(MovementKind::Expense, Money::new(dec!(30)), 20, at(20, 7)),
            // Saisie après, mais datée d'avant la référence : déjà débitée par la banque.
            movement(MovementKind::Expense, Money::new(dec!(15)), 15, at(22, 9)),
        ];

        let statement = reference(Money::new(dec!(980))).statement(&movements);

        assert_eq!(statement.balance, Money::new(dec!(980)));
        assert!(statement.movements.is_empty());
    }

    // Spec checking-account : « Découvert »
    #[test]
    fn overdraft_is_allowed_but_not_sub_cent_amounts() {
        let overdrawn = reference(Money::new(dec!(-85.40)));
        assert_eq!(overdrawn.reference_balance(), Money::new(dec!(-85.40)));
        assert_eq!(
            CheckingDetails::new(Money::new(dec!(1.001)), day(20), at(20, 8)),
            Err(DomainError::TooManyDecimals)
        );
    }

    // Spec checking-account : « Création et premier solde » et « Projection »
    #[test]
    fn checking_account_is_valued_from_its_reference_and_ledger() {
        use crate::account::{Account, AccountType, ScenarioRates};
        use crate::budget::Expense;
        use crate::ledger::{Ledger, LedgerSources};
        use crate::valuation::{PriceBook, value_account};
        use crate::{AccountId, CategoryId, ExpenseId, Label, Rate, defaults};

        let mut account = Account::open(
            AccountId::generate(),
            Label::new("Compte courant").unwrap(),
            defaults::color(AccountType::Checking),
            AccountType::Checking,
            day(20),
        );
        account
            .update_checking_balance(Money::new(dec!(1250)), day(20), at(20, 8))
            .unwrap();
        let groceries = Expense::new(
            ExpenseId::generate(),
            Money::new(dec!(42.50)),
            CategoryId::generate(),
            day(20),
            None,
        )
        .unwrap()
        .linked_to(Some(account.id()))
        .recorded(Some(at(20, 18)));
        let expenses = [groceries];
        let ledger = Ledger::build(&LedgerSources {
            expenses: &expenses,
            incomes: &[],
            charges: &[],
            transfers: &[],
            today: day(21),
        });

        let valuation = value_account(&account, &PriceBook::default(), &ledger);

        assert_eq!(valuation.value, Money::new(dec!(1207.50)));
        assert_eq!(valuation.movements.len(), 1);
        assert_eq!(
            *account.scenario_rates(),
            ScenarioRates::uniform(Rate::ZERO),
            "pas de rendement en projection"
        );
    }
}
