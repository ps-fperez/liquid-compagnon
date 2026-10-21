use std::fmt;

use chrono::{Datelike, NaiveDate};

use crate::DomainError;

/// Mois calendaire (année + mois), utilisé pour les dates d'effet, le budget et les snapshots.
///
/// L'ordre naturel est chronologique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Month {
    year: i32,
    month: u32,
}

impl Month {
    pub fn new(year: i32, month: u32) -> Result<Self, DomainError> {
        if !(1..=12).contains(&month) || !(1900..=9999).contains(&year) {
            return Err(DomainError::InvalidMonth { year, month });
        }
        Ok(Self { year, month })
    }

    pub fn of(date: NaiveDate) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
        }
    }

    pub const fn year(&self) -> i32 {
        self.year
    }

    pub const fn month(&self) -> u32 {
        self.month
    }

    pub fn first_day(&self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.year, self.month, 1)
            .expect("un Month valide a toujours un 1er jour")
    }

    /// Décale de `n` mois (négatif pour revenir en arrière).
    pub fn plus_months(&self, n: i32) -> Self {
        let index = self.index() + n;
        Self {
            year: index.div_euclid(12),
            month: index.rem_euclid(12) as u32 + 1,
        }
    }

    pub fn next(&self) -> Self {
        self.plus_months(1)
    }

    pub fn previous(&self) -> Self {
        self.plus_months(-1)
    }

    /// Nombre de mois de `self` jusqu'à `other` (négatif si `other` est antérieur).
    pub fn months_until(&self, other: Month) -> i32 {
        other.index() - self.index()
    }

    /// Date du `day` du mois, ramenée au dernier jour si le mois est plus court (31 → 30, 28…).
    pub fn day(&self, day: u8) -> NaiveDate {
        let last = self
            .next()
            .first_day()
            .pred_opt()
            .expect("un mois a toujours une veille");
        let wanted = u32::from(day.max(1)).min(last.day());
        NaiveDate::from_ymd_opt(self.year, self.month, wanted).expect("jour borné au mois")
    }

    pub fn contains(&self, date: NaiveDate) -> bool {
        Self::of(date) == *self
    }

    fn index(&self) -> i32 {
        self.year * 12 + self.month as i32 - 1
    }
}

impl fmt::Display for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

impl std::str::FromStr for Month {
    type Err = DomainError;

    /// Lit un mois au format ISO `AAAA-MM`.
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let invalid = || DomainError::InvalidMonth { year: 0, month: 0 };
        let (year, month) = raw.trim().split_once('-').ok_or_else(invalid)?;
        let year = year.parse().map_err(|_| invalid())?;
        let month = month.parse().map_err(|_| invalid())?;
        Month::new(year, month)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn month(year: i32, month: u32) -> Month {
        Month::new(year, month).unwrap()
    }

    #[test]
    fn rejects_invalid_month_numbers() {
        assert_eq!(
            Month::new(2026, 13),
            Err(DomainError::InvalidMonth {
                year: 2026,
                month: 13
            })
        );
        assert!(Month::new(2026, 0).is_err());
    }

    #[test]
    fn wraps_across_years() {
        assert_eq!(month(2026, 12).next(), month(2027, 1));
        assert_eq!(month(2026, 1).previous(), month(2025, 12));
        assert_eq!(month(2026, 10).plus_months(15), month(2028, 1));
    }

    #[test]
    fn orders_chronologically() {
        assert!(month(2025, 12) < month(2026, 1));
        assert!(month(2026, 2) > month(2026, 1));
    }

    #[test]
    fn derives_from_date_and_contains_it() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        assert_eq!(Month::of(date), month(2026, 10));
        assert!(month(2026, 10).contains(date));
        assert_eq!(
            month(2026, 10).first_day(),
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()
        );
    }

    #[test]
    fn displays_as_iso_year_month() {
        assert_eq!(month(2026, 3).to_string(), "2026-03");
    }

    #[test]
    fn parses_iso_year_month() {
        assert_eq!("2026-03".parse::<Month>(), Ok(month(2026, 3)));
        assert!("2026-13".parse::<Month>().is_err());
        assert!("mars 2026".parse::<Month>().is_err());
    }

    proptest! {
        #[test]
        fn plus_then_minus_is_identity(year in 1950i32..2100, m in 1u32..=12, n in -600i32..600) {
            let start = month(year, m);
            prop_assert_eq!(start.plus_months(n).plus_months(-n), start);
            prop_assert_eq!(start.months_until(start.plus_months(n)), n);
        }
    }
}
