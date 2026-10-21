//! Bilan du mois (spec monthly-review) : mois déjà présenté et solde de départ saisi pour le
//! compte courant au début du mois suivant.

use chrono::{DateTime, Utc};

use crate::{Money, Month};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthlyReview {
    /// Mois résumé par le bilan.
    month: Month,
    seen_at: DateTime<Utc>,
    /// Solde de départ du compte courant saisi pour le mois suivant (`month + 1`).
    next_start_balance: Option<Money>,
}

impl MonthlyReview {
    pub fn new(month: Month, seen_at: DateTime<Utc>) -> Self {
        Self {
            month,
            seen_at,
            next_start_balance: None,
        }
    }

    pub fn with_next_start_balance(mut self, balance: Option<Money>) -> Self {
        self.next_start_balance = balance;
        self
    }

    pub fn month(&self) -> Month {
        self.month
    }

    pub fn seen_at(&self) -> DateTime<Utc> {
        self.seen_at
    }

    pub fn next_start_balance(&self) -> Option<Money> {
        self.next_start_balance
    }

    pub fn set_next_start_balance(&mut self, balance: Money) {
        self.next_start_balance = Some(balance);
    }
}

/// Solde proposé pour démarrer le mois : celui saisi au début du mois précédent, sinon `current`.
pub fn suggested_start_balance(reviews: &[MonthlyReview], month: Month, current: Money) -> Money {
    reviews
        .iter()
        .filter(|r| r.month() < month)
        .filter_map(|r| r.next_start_balance().map(|balance| (r.month(), balance)))
        .max_by_key(|(m, _)| *m)
        .map_or(current, |(_, balance)| balance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rust_decimal_macros::dec;

    // Spec monthly-review : « Mois suivant »
    #[test]
    fn previous_start_balance_is_suggested() {
        let at = Utc.with_ymd_and_hms(2026, 11, 1, 8, 0, 0).unwrap();
        let october = Month::new(2026, 10).unwrap();
        let november = Month::new(2026, 11).unwrap();
        let mut reviews = vec![MonthlyReview::new(october, at)];
        assert_eq!(
            suggested_start_balance(&reviews, november, Money::new(dec!(12))),
            Money::new(dec!(12))
        );
        reviews[0].set_next_start_balance(Money::new(dec!(250)));
        assert_eq!(
            suggested_start_balance(&reviews, november, Money::new(dec!(12))),
            Money::new(dec!(250))
        );
    }
}
