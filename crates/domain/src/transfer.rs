//! Virements entre comptes : ponctuels ou récurrents, entre comptes suivis ou avec un compte hors
//! patrimoine (compte joint, banque d'un crédit…). Le montant reçu peut différer du montant
//! envoyé (abondement net de prélèvements sociaux) : les deux sont saisis, jamais calculés.

use chrono::{DateTime, Datelike, NaiveDate, Utc};

use crate::{AccountId, DomainError, ExternalId, Label, Money, Month, TransferId};

/// Côté d'un virement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endpoint {
    Account(AccountId),
    External(ExternalId),
}

impl Endpoint {
    pub fn account(self) -> Option<AccountId> {
        match self {
            Endpoint::Account(id) => Some(id),
            Endpoint::External(_) => None,
        }
    }

    pub fn is_external(self) -> bool {
        matches!(self, Endpoint::External(_))
    }
}

/// Fréquence d'un virement. Un jour absent du mois est ramené au dernier jour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schedule {
    Once {
        date: NaiveDate,
    },
    Monthly {
        day: u8,
        from: Month,
        until: Option<Month>,
    },
    Yearly {
        month: u32,
        day: u8,
        from_year: i32,
        until_year: Option<i32>,
    },
}

impl Schedule {
    fn validate(self) -> Result<Self, DomainError> {
        let day_ok = |day: u8| (1..=31).contains(&day);
        match self {
            Schedule::Once { .. } => Ok(self),
            Schedule::Monthly { day, from, until } => {
                if !day_ok(day) {
                    return Err(DomainError::InvalidDayOfMonth);
                }
                if until.is_some_and(|until| until < from) {
                    return Err(DomainError::EndBeforeStart);
                }
                Ok(self)
            }
            Schedule::Yearly {
                month,
                day,
                from_year,
                until_year,
            } => {
                if !day_ok(day) {
                    return Err(DomainError::InvalidDayOfMonth);
                }
                Month::new(from_year, month)?;
                if until_year.is_some_and(|until| until < from_year) {
                    return Err(DomainError::EndBeforeStart);
                }
                Ok(self)
            }
        }
    }

    pub fn is_recurring(self) -> bool {
        !matches!(self, Schedule::Once { .. })
    }

    /// Dates des occurrences comprises entre `from` et `to` inclus.
    pub fn occurrences_between(self, from: NaiveDate, to: NaiveDate) -> Vec<NaiveDate> {
        if to < from {
            return Vec::new();
        }
        let dates: Vec<NaiveDate> = match self {
            Schedule::Once { date } => vec![date],
            Schedule::Monthly {
                day,
                from: start,
                until,
            } => {
                let first = start.max(Month::of(from));
                let last = until.map_or(Month::of(to), |until| until.min(Month::of(to)));
                let mut month = first;
                let mut dates = Vec::new();
                while month <= last {
                    dates.push(month.day(day));
                    month = month.next();
                }
                dates
            }
            Schedule::Yearly {
                month,
                day,
                from_year,
                until_year,
            } => {
                let first = from_year.max(from.year());
                let last = until_year.map_or(to.year(), |until| until.min(to.year()));
                (first..=last)
                    .filter_map(|year| Month::new(year, month).ok())
                    .map(|m| m.day(day))
                    .collect()
            }
        };
        dates
            .into_iter()
            .filter(|date| (from..=to).contains(date))
            .collect()
    }

    /// Occurrences datées du mois `month`.
    pub fn occurrences_in(self, month: Month) -> Vec<NaiveDate> {
        let last = month
            .next()
            .first_day()
            .pred_opt()
            .expect("veille du mois suivant");
        self.occurrences_between(month.first_day(), last)
    }
}

/// Virement entre deux comptes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    id: TransferId,
    from: Endpoint,
    to: Endpoint,
    sent: Money,
    received: Money,
    label: Option<Label>,
    schedule: Schedule,
    /// Instant de la saisie : situe un virement ponctuel par rapport au solde de référence.
    recorded_at: DateTime<Utc>,
}

impl Transfer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: TransferId,
        from: Endpoint,
        to: Endpoint,
        sent: Money,
        received: Money,
        label: Option<Label>,
        schedule: Schedule,
        recorded_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if from == to {
            return Err(DomainError::SameTransferEndpoints);
        }
        if from.is_external() && to.is_external() {
            return Err(DomainError::TransferWithoutTrackedAccount);
        }
        Ok(Self {
            id,
            from,
            to,
            sent: sent.validate_entry()?,
            received: received.validate_entry()?,
            label,
            schedule: schedule.validate()?,
            recorded_at,
        })
    }

    pub fn id(&self) -> TransferId {
        self.id
    }

    pub fn from(&self) -> Endpoint {
        self.from
    }

    pub fn to(&self) -> Endpoint {
        self.to
    }

    pub fn sent(&self) -> Money {
        self.sent
    }

    pub fn received(&self) -> Money {
        self.received
    }

    /// Écart entre reçu et envoyé (abondement net, s'il est positif).
    pub fn bonus(&self) -> Money {
        self.received - self.sent
    }

    pub fn label(&self) -> Option<&Label> {
        self.label.as_ref()
    }

    pub fn schedule(&self) -> Schedule {
        self.schedule
    }

    pub fn recorded_at(&self) -> DateTime<Utc> {
        self.recorded_at
    }

    pub fn involves(&self, endpoint: Endpoint) -> bool {
        self.from == endpoint || self.to == endpoint
    }
}

/// Compte hors patrimoine : il n'a qu'un nom et n'entre ni dans le patrimoine ni dans la
/// projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalAccount {
    id: ExternalId,
    name: Label,
}

impl ExternalAccount {
    pub fn new(id: ExternalId, name: Label) -> Self {
        Self { id, name }
    }

    pub fn id(&self) -> ExternalId {
        self.id
    }

    pub fn name(&self) -> &Label {
        &self.name
    }

    pub fn rename(&mut self, name: Label) {
        self.name = name;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rust_decimal_macros::dec;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn month(y: i32, m: u32) -> Month {
        Month::new(y, m).unwrap()
    }

    fn transfer(from: Endpoint, to: Endpoint, schedule: Schedule) -> Result<Transfer, DomainError> {
        Transfer::new(
            TransferId::generate(),
            from,
            to,
            Money::new(dec!(500)),
            Money::new(dec!(1493.30)),
            None,
            schedule,
            Utc.with_ymd_and_hms(2026, 10, 21, 9, 0, 0).unwrap(),
        )
    }

    // Spec account-transfers : « Côtés identiques »
    #[test]
    fn endpoints_must_differ_and_include_a_tracked_account() {
        let account = Endpoint::Account(AccountId::generate());
        let once = Schedule::Once {
            date: day(2026, 10, 21),
        };
        assert_eq!(
            transfer(account, account, once),
            Err(DomainError::SameTransferEndpoints)
        );
        let joint = Endpoint::External(ExternalId::generate());
        let bank = Endpoint::External(ExternalId::generate());
        assert_eq!(
            transfer(joint, bank, once),
            Err(DomainError::TransferWithoutTrackedAccount)
        );
    }

    // Spec account-transfers : « Montant reçu différent »
    #[test]
    fn received_amount_may_exceed_sent_amount() {
        let pei = transfer(
            Endpoint::Account(AccountId::generate()),
            Endpoint::Account(AccountId::generate()),
            Schedule::Yearly {
                month: 1,
                day: 15,
                from_year: 2027,
                until_year: None,
            },
        )
        .unwrap();
        assert_eq!(pei.bonus(), Money::new(dec!(993.30)));
    }

    // Spec scheduled-debits : « Jour absent du mois »
    #[test]
    fn monthly_occurrences_clamp_to_the_last_day() {
        let schedule = Schedule::Monthly {
            day: 31,
            from: month(2026, 9),
            until: None,
        };
        assert_eq!(
            schedule.occurrences_between(day(2026, 9, 1), day(2026, 11, 30)),
            [day(2026, 9, 30), day(2026, 10, 31), day(2026, 11, 30)]
        );
        assert_eq!(schedule.occurrences_in(month(2027, 2)), [day(2027, 2, 28)]);
    }

    #[test]
    fn occurrences_respect_bounds() {
        let monthly = Schedule::Monthly {
            day: 1,
            from: month(2026, 10),
            until: Some(month(2026, 12)),
        };
        assert_eq!(
            monthly
                .occurrences_between(day(2026, 1, 1), day(2027, 6, 1))
                .len(),
            3
        );
        let yearly = Schedule::Yearly {
            month: 1,
            day: 15,
            from_year: 2027,
            until_year: None,
        };
        assert_eq!(
            yearly.occurrences_between(day(2026, 1, 1), day(2028, 1, 14)),
            [day(2027, 1, 15)]
        );
        let once = Schedule::Once {
            date: day(2026, 10, 21),
        };
        assert!(
            once.occurrences_between(day(2026, 10, 22), day(2026, 12, 1))
                .is_empty()
        );
        assert_eq!(
            Schedule::Monthly {
                day: 0,
                from: month(2026, 10),
                until: None
            }
            .validate(),
            Err(DomainError::InvalidDayOfMonth)
        );
    }
}
