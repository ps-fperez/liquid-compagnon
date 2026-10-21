//! Grand livre : mouvements de chaque compte à solde (dépenses, revenus, prélèvements, virements),
//! comptés par rapport au dernier solde saisi (solde de référence).

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};

use crate::budget::{Expense, OneOffIncome, RecurringCharge};
use crate::transfer::{Endpoint, Transfer};
use crate::{AccountId, Money};

/// Dernier solde saisi d'un compte : son montant, sa date (fuseau de l'utilisateur) et l'instant
/// de la saisie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalanceReference {
    pub amount: Money,
    pub on: NaiveDate,
    pub at: DateTime<Utc>,
}

impl BalanceReference {
    pub fn new(amount: Money, on: NaiveDate, at: DateTime<Utc>) -> Self {
        Self { amount, on, at }
    }

    /// Un mouvement saisi compte s'il l'a été après la référence et qu'il est daté du jour de la
    /// référence ou après ; une échéance compte si elle tombe après le jour de la référence. Sinon,
    /// le mouvement est réputé déjà inclus dans le solde saisi.
    pub fn counts(&self, movement: &Movement) -> bool {
        match movement.timing {
            Timing::Recorded(at) => at > self.at && movement.date >= self.on,
            Timing::Scheduled => movement.date > self.on,
        }
    }

    /// Solde courant et mouvements comptés, du plus récent au plus ancien.
    pub fn statement(&self, movements: &[Movement]) -> Statement {
        let mut counted: Vec<Movement> = movements
            .iter()
            .filter(|m| self.counts(m))
            .cloned()
            .collect();
        counted.sort_by_key(|m| std::cmp::Reverse((m.date, m.sort_instant())));
        Statement {
            balance: self.amount + counted.iter().map(Movement::signed_amount).sum(),
            movements: counted,
        }
    }
}

/// Solde courant d'un compte et mouvements pris en compte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub balance: Money,
    pub movements: Vec<Movement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementKind {
    Expense,
    Income,
    /// Charge fixe prélevée.
    Charge,
    TransferIn,
    TransferOut,
}

/// Temporalité d'un mouvement : saisi à un instant, ou échéance d'une récurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timing {
    Recorded(DateTime<Utc>),
    Scheduled,
}

/// Mouvement d'un compte. Le montant est positif ; le sens est donné par `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movement {
    pub kind: MovementKind,
    pub amount: Money,
    pub date: NaiveDate,
    pub timing: Timing,
    pub label: Option<String>,
    /// Compte d'en face d'un virement.
    pub counterpart: Option<Endpoint>,
}

impl Movement {
    pub fn signed_amount(&self) -> Money {
        match self.kind {
            MovementKind::Expense | MovementKind::Charge | MovementKind::TransferOut => {
                -self.amount
            }
            MovementKind::Income | MovementKind::TransferIn => self.amount,
        }
    }

    fn sort_instant(&self) -> Option<DateTime<Utc>> {
        match self.timing {
            Timing::Recorded(at) => Some(at),
            Timing::Scheduled => None,
        }
    }
}

/// Sources du grand livre, avec la date du jour : seules les échéances passées sont comptées.
pub struct LedgerSources<'a> {
    pub expenses: &'a [Expense],
    pub incomes: &'a [OneOffIncome],
    pub charges: &'a [RecurringCharge],
    pub transfers: &'a [Transfer],
    pub today: NaiveDate,
}

/// Mouvements regroupés par compte.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ledger {
    by_account: HashMap<AccountId, Vec<Movement>>,
}

impl Ledger {
    pub fn build(sources: &LedgerSources<'_>) -> Self {
        let mut ledger = Self::default();
        for expense in sources.expenses {
            if let (Some(account), Some(at)) = (expense.account(), expense.recorded_at()) {
                ledger.push(
                    account,
                    MovementKind::Expense,
                    expense.amount(),
                    expense.date(),
                    Timing::Recorded(at),
                    expense.label().map(ToString::to_string),
                    None,
                );
            }
        }
        for income in sources.incomes {
            if let (Some(account), Some(at)) = (income.account(), income.recorded_at()) {
                ledger.push(
                    account,
                    MovementKind::Income,
                    income.amount(),
                    income.date(),
                    Timing::Recorded(at),
                    Some(income.label().to_string()),
                    None,
                );
            }
        }
        for charge in sources.charges {
            let Some(debit) = charge.debit() else {
                continue;
            };
            for date in charge.due_dates_until(sources.today) {
                ledger.push(
                    debit.account,
                    MovementKind::Charge,
                    charge.amount(),
                    date,
                    Timing::Scheduled,
                    Some(charge.label().to_string()),
                    None,
                );
            }
        }
        for transfer in sources.transfers {
            let schedule = transfer.schedule();
            let timing = if schedule.is_recurring() {
                Timing::Scheduled
            } else {
                Timing::Recorded(transfer.recorded_at())
            };
            let label = transfer.label().map(ToString::to_string);
            let earliest = NaiveDate::MIN;
            for date in schedule.occurrences_between(earliest, sources.today) {
                if let Some(account) = transfer.from().account() {
                    ledger.push(
                        account,
                        MovementKind::TransferOut,
                        transfer.sent(),
                        date,
                        timing,
                        label.clone(),
                        Some(transfer.to()),
                    );
                }
                if let Some(account) = transfer.to().account() {
                    ledger.push(
                        account,
                        MovementKind::TransferIn,
                        transfer.received(),
                        date,
                        timing,
                        label.clone(),
                        Some(transfer.from()),
                    );
                }
            }
        }
        ledger
    }

    pub fn movements(&self, account: AccountId) -> &[Movement] {
        self.by_account.get(&account).map_or(&[], Vec::as_slice)
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        account: AccountId,
        kind: MovementKind,
        amount: Money,
        date: NaiveDate,
        timing: Timing,
        label: Option<String>,
        counterpart: Option<Endpoint>,
    ) {
        self.by_account.entry(account).or_default().push(Movement {
            kind,
            amount,
            date,
            timing,
            label,
            counterpart,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::{Debit, Periodicity};
    use crate::transfer::Schedule;
    use crate::{CategoryId, ChargeId, ExternalId, Label, Month, TransferId};
    use chrono::TimeZone;
    use rust_decimal_macros::dec;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, d).unwrap()
    }

    fn at(d: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, d, hour, 0, 0).unwrap()
    }

    fn eur(amount: rust_decimal::Decimal) -> Money {
        Money::new(amount)
    }

    fn charge(
        label: &str,
        amount: rust_decimal::Decimal,
        account: AccountId,
        debit_day: u8,
    ) -> RecurringCharge {
        RecurringCharge::new(
            ChargeId::generate(),
            Label::new(label).unwrap(),
            eur(amount),
            CategoryId::generate(),
            Periodicity::Monthly,
            Month::new(2026, 1).unwrap(),
            None,
        )
        .unwrap()
        .with_debit(Some(Debit {
            account,
            day: debit_day,
        }))
        .unwrap()
    }

    fn transfer(
        from: Endpoint,
        to: Endpoint,
        amount: rust_decimal::Decimal,
        schedule: Schedule,
        recorded: DateTime<Utc>,
    ) -> Transfer {
        Transfer::new(
            TransferId::generate(),
            from,
            to,
            eur(amount),
            eur(amount),
            None,
            schedule,
            recorded,
        )
        .unwrap()
    }

    // Spec scheduled-debits : « Abonnement prélevé » ; account-transfers : « Fil du compte courant »
    #[test]
    fn checking_follows_debits_and_transfers_after_the_reference() {
        let checking = AccountId::generate();
        let ldds = AccountId::generate();
        let charges = [charge("Sosh", dec!(15.99), checking, 5)];
        let transfers = [
            // Virement mensuel du 1er : déjà passé quand le solde est saisi le 1er.
            transfer(
                Endpoint::Account(checking),
                Endpoint::Account(ldds),
                dec!(850),
                Schedule::Monthly {
                    day: 1,
                    from: Month::new(2026, 10).unwrap(),
                    until: None,
                },
                at(1, 7),
            ),
            // Rapatriement saisi le 18.
            transfer(
                Endpoint::Account(ldds),
                Endpoint::Account(checking),
                dec!(100),
                Schedule::Once { date: day(18) },
                at(18, 12),
            ),
        ];
        let ledger = Ledger::build(&LedgerSources {
            expenses: &[],
            incomes: &[],
            charges: &charges,
            transfers: &transfers,
            today: day(21),
        });
        let reference = BalanceReference::new(eur(dec!(250)), day(1), at(1, 9));

        let statement = reference.statement(ledger.movements(checking));

        assert_eq!(statement.balance, eur(dec!(334.01)), "250 − 15,99 + 100");
        assert_eq!(statement.movements[0].kind, MovementKind::TransferIn);
        assert_eq!(statement.movements[1].label.as_deref(), Some("Sosh"));
    }

    // Spec account-transfers : « Virement déjà inclus dans le solde saisi »
    #[test]
    fn scheduled_occurrence_of_the_reference_day_is_already_included() {
        let ldds = AccountId::generate();
        let joint = Endpoint::External(ExternalId::generate());
        let monthly = transfer(
            joint,
            Endpoint::Account(ldds),
            dec!(850),
            Schedule::Monthly {
                day: 1,
                from: Month::new(2026, 9).unwrap(),
                until: None,
            },
            at(1, 7),
        );
        let ledger = Ledger::build(&LedgerSources {
            expenses: &[],
            incomes: &[],
            charges: &[],
            transfers: &[monthly],
            today: day(21),
        });

        let saved_on_the_3rd = BalanceReference::new(eur(dec!(10000)), day(3), at(3, 10));
        assert_eq!(
            saved_on_the_3rd.statement(ledger.movements(ldds)).balance,
            eur(dec!(10000))
        );

        let saved_in_september = BalanceReference::new(
            eur(dec!(9150)),
            NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
            Utc.with_ymd_and_hms(2026, 9, 20, 10, 0, 0).unwrap(),
        );
        assert_eq!(
            saved_in_september.statement(ledger.movements(ldds)).balance,
            eur(dec!(10000))
        );
    }

    #[test]
    fn future_occurrences_are_not_counted_yet() {
        let checking = AccountId::generate();
        let charges = [charge("Ionos", dec!(7.20), checking, 25)];
        let ledger = Ledger::build(&LedgerSources {
            expenses: &[],
            incomes: &[],
            charges: &charges,
            transfers: &[],
            today: day(21),
        });
        assert!(ledger.movements(checking).iter().all(|m| m.date <= day(21)));
    }
}
