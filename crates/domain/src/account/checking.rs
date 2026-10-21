//! Compte courant : un solde de référence saisi par l'utilisateur, que font évoluer les dépenses
//! et revenus ponctuels rattachés, saisis après cette référence.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};

use crate::budget::{Expense, OneOffIncome};
use crate::{AccountId, DomainError, Money};

/// Solde de référence d'un compte courant (signé : un découvert est possible).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckingDetails {
    reference_balance: Money,
    reference_on: NaiveDate,
    reference_at: DateTime<Utc>,
}

impl CheckingDetails {
    pub fn new(
        reference_balance: Money,
        reference_on: NaiveDate,
        reference_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            reference_balance: reference_balance.validate_signed()?,
            reference_on,
            reference_at,
        })
    }

    pub fn reference_balance(&self) -> Money {
        self.reference_balance
    }

    /// Date de la référence, dans le fuseau de l'utilisateur.
    pub fn reference_on(&self) -> NaiveDate {
        self.reference_on
    }

    pub fn reference_at(&self) -> DateTime<Utc> {
        self.reference_at
    }

    /// Un mouvement compte s'il a été saisi après la référence et qu'il est daté du jour de la
    /// référence ou après. Sinon, il est réputé déjà inclus dans le solde saisi.
    pub fn counts(&self, movement: &Movement) -> bool {
        movement.recorded_at > self.reference_at && movement.date >= self.reference_on
    }

    /// Solde courant et mouvements comptés (du plus récent au plus ancien).
    pub fn statement(&self, movements: &[Movement]) -> CheckingStatement {
        let mut counted: Vec<Movement> = movements
            .iter()
            .filter(|m| self.counts(m))
            .cloned()
            .collect();
        counted.sort_by_key(|m| std::cmp::Reverse((m.date, m.recorded_at)));
        let balance = self.reference_balance + counted.iter().map(Movement::signed_amount).sum();
        CheckingStatement {
            balance,
            movements: counted,
        }
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

/// Solde courant d'un compte courant et mouvements pris en compte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckingStatement {
    pub balance: Money,
    pub movements: Vec<Movement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementKind {
    Expense,
    Income,
}

/// Dépense ou revenu ponctuel rattaché à un compte courant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movement {
    pub kind: MovementKind,
    /// Montant positif ; le sens est donné par `kind`.
    pub amount: Money,
    pub date: NaiveDate,
    pub recorded_at: DateTime<Utc>,
    pub label: Option<String>,
}

impl Movement {
    pub fn signed_amount(&self) -> Money {
        match self.kind {
            MovementKind::Expense => -self.amount,
            MovementKind::Income => self.amount,
        }
    }
}

/// Mouvements rattachés, regroupés par compte courant.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckingLedger {
    by_account: HashMap<AccountId, Vec<Movement>>,
}

impl CheckingLedger {
    /// Seuls les mouvements rattachés à un compte et horodatés sont retenus.
    pub fn from_budget(expenses: &[Expense], incomes: &[OneOffIncome]) -> Self {
        let mut ledger = Self::default();
        for expense in expenses {
            if let (Some(account), Some(recorded_at)) = (expense.account(), expense.recorded_at()) {
                ledger.push(
                    account,
                    Movement {
                        kind: MovementKind::Expense,
                        amount: expense.amount(),
                        date: expense.date(),
                        recorded_at,
                        label: expense.label().map(ToString::to_string),
                    },
                );
            }
        }
        for income in incomes {
            if let (Some(account), Some(recorded_at)) = (income.account(), income.recorded_at()) {
                ledger.push(
                    account,
                    Movement {
                        kind: MovementKind::Income,
                        amount: income.amount(),
                        date: income.date(),
                        recorded_at,
                        label: Some(income.label().to_string()),
                    },
                );
            }
        }
        ledger
    }

    pub fn movements(&self, account: AccountId) -> &[Movement] {
        self.by_account.get(&account).map_or(&[], Vec::as_slice)
    }

    fn push(&mut self, account: AccountId, movement: Movement) {
        self.by_account.entry(account).or_default().push(movement);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            recorded_at: recorded,
            label: None,
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
        use crate::valuation::{PriceBook, value_account};
        use crate::{CategoryId, ExpenseId, Label, Rate, defaults};

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
        let ledger = CheckingLedger::from_budget(&[groceries], &[]);

        let valuation = value_account(&account, &PriceBook::default(), &ledger);

        assert_eq!(valuation.value, Money::new(dec!(1207.50)));
        assert_eq!(valuation.movements.len(), 1);
        assert_eq!(
            *account.scenario_rates(),
            ScenarioRates::uniform(Rate::ZERO),
            "pas de rendement en projection"
        );
        assert_eq!(account.account_type(), AccountType::Checking);
    }

    #[test]
    fn ledger_keeps_only_linked_and_timestamped_movements() {
        use crate::budget::Expense;
        use crate::{CategoryId, ExpenseId};

        let account = AccountId::generate();
        let expense = |linked: Option<AccountId>, recorded: Option<DateTime<Utc>>| {
            Expense::new(
                ExpenseId::generate(),
                Money::new(dec!(10)),
                CategoryId::generate(),
                day(21),
                None,
            )
            .unwrap()
            .linked_to(linked)
            .recorded(recorded)
        };
        let expenses = [
            expense(Some(account), Some(at(21, 9))),
            expense(None, Some(at(21, 9))),
            expense(Some(account), None),
        ];

        let ledger = CheckingLedger::from_budget(&expenses, &[]);

        assert_eq!(ledger.movements(account).len(), 1);
        assert!(ledger.movements(AccountId::generate()).is_empty());
    }
}
