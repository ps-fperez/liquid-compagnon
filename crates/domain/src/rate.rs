use std::fmt;

use rust_decimal::Decimal;

use crate::DomainError;

/// Taux annuel exprimé en fraction (0,03 = 3 %). Peut être négatif (rendement pessimiste),
/// mais toujours strictement supérieur à -100 %.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rate(Decimal);

impl Rate {
    pub const ZERO: Rate = Rate(Decimal::ZERO);

    pub fn from_fraction(fraction: Decimal) -> Result<Self, DomainError> {
        if fraction <= Decimal::NEGATIVE_ONE {
            return Err(DomainError::RateOutOfRange);
        }
        Ok(Self(fraction))
    }

    /// `Rate::from_percent(dec!(3))` → 3 %.
    pub fn from_percent(percent: Decimal) -> Result<Self, DomainError> {
        Self::from_fraction(percent / Decimal::ONE_HUNDRED)
    }

    pub const fn as_fraction(&self) -> Decimal {
        self.0
    }

    pub fn as_percent(&self) -> Decimal {
        (self.0 * Decimal::ONE_HUNDRED).normalize()
    }
}

impl fmt::Display for Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} %", self.as_percent())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn converts_between_percent_and_fraction() {
        let rate = Rate::from_percent(dec!(1.7)).unwrap();
        assert_eq!(rate.as_fraction(), dec!(0.017));
        assert_eq!(rate.as_percent(), dec!(1.7));
    }

    #[test]
    fn accepts_negative_returns_above_minus_hundred_percent() {
        assert!(Rate::from_percent(dec!(-40)).is_ok());
        assert_eq!(
            Rate::from_percent(dec!(-100)),
            Err(DomainError::RateOutOfRange)
        );
        assert_eq!(
            Rate::from_fraction(dec!(-1.5)),
            Err(DomainError::RateOutOfRange)
        );
    }

    #[test]
    fn displays_as_percent() {
        assert_eq!(Rate::from_percent(dec!(5)).unwrap().to_string(), "5 %");
    }
}
