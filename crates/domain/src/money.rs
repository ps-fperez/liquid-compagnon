use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

use rust_decimal::{Decimal, RoundingStrategy};

use crate::DomainError;

/// Montant en euros, en décimal exact (jamais de flottant).
///
/// Les calculs internes conservent toute la précision ; l'arrondi au centime ne se fait qu'à
/// l'affichage ([`Money::round_to_cents`]) ou à la validation d'une saisie.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Money(Decimal);

impl Money {
    pub const ZERO: Money = Money(Decimal::ZERO);

    pub const fn new(amount: Decimal) -> Self {
        Self(amount)
    }

    pub fn from_cents(cents: i64) -> Self {
        Self(Decimal::new(cents, 2))
    }

    pub const fn amount(&self) -> Decimal {
        self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn is_positive(&self) -> bool {
        self.0 > Decimal::ZERO
    }

    pub fn is_negative(&self) -> bool {
        self.0 < Decimal::ZERO
    }

    /// Arrondi au centime le plus proche (0,005 → 0,01).
    pub fn round_to_cents(&self) -> Self {
        Self(
            self.0
                .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero),
        )
    }

    /// Vrai si le montant ne comporte pas plus de 2 décimales significatives.
    pub fn has_at_most_cents(&self) -> bool {
        self.0.normalize().scale() <= 2
    }

    pub fn times(&self, factor: Decimal) -> Self {
        Self(self.0 * factor)
    }

    /// Division par un entier non nul (ex. charge annuelle / 12). `None` si `divisor` vaut 0.
    pub fn divided_by(&self, divisor: u32) -> Option<Self> {
        self.0.checked_div(Decimal::from(divisor)).map(Self)
    }

    pub fn max(self, other: Self) -> Self {
        if self >= other { self } else { other }
    }

    pub fn min(self, other: Self) -> Self {
        if self <= other { self } else { other }
    }

    /// Valide un montant saisi par l'utilisateur : strictement positif, au plus 2 décimales.
    pub fn validate_entry(self) -> Result<Self, DomainError> {
        if !self.is_positive() {
            return Err(DomainError::NonPositiveAmount);
        }
        if !self.has_at_most_cents() {
            return Err(DomainError::TooManyDecimals);
        }
        Ok(self)
    }

    /// Valide un solde signé (compte courant à découvert) : au plus 2 décimales.
    pub fn validate_signed(self) -> Result<Self, DomainError> {
        if !self.has_at_most_cents() {
            return Err(DomainError::TooManyDecimals);
        }
        Ok(self)
    }

    /// Valide un solde ou un montant pouvant être nul : positif ou nul, au plus 2 décimales.
    pub fn validate_balance(self) -> Result<Self, DomainError> {
        if self.is_negative() {
            return Err(DomainError::NegativeAmount);
        }
        if !self.has_at_most_cents() {
            return Err(DomainError::TooManyDecimals);
        }
        Ok(self)
    }
}

impl fmt::Display for Money {
    /// Représentation technique (débogage, logs) : « 1234.56 EUR ». Le formatage français est
    /// à la charge de l'UI.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} EUR", self.round_to_cents().0)
    }
}

impl Add for Money {
    type Output = Money;
    fn add(self, rhs: Money) -> Money {
        Money(self.0 + rhs.0)
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Money) {
        self.0 += rhs.0;
    }
}

impl Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Money {
        Money(self.0 - rhs.0)
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Money) {
        self.0 -= rhs.0;
    }
}

impl Neg for Money {
    type Output = Money;
    fn neg(self) -> Money {
        Money(-self.0)
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        iter.fold(Money::ZERO, Add::add)
    }
}

impl<'a> Sum<&'a Money> for Money {
    fn sum<I: Iterator<Item = &'a Money>>(iter: I) -> Money {
        iter.copied().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rust_decimal_macros::dec;

    #[test]
    fn rounds_half_cent_away_from_zero() {
        assert_eq!(
            Money::new(dec!(0.005)).round_to_cents(),
            Money::new(dec!(0.01))
        );
        assert_eq!(
            Money::new(dec!(-0.005)).round_to_cents(),
            Money::new(dec!(-0.01))
        );
        assert_eq!(
            Money::new(dec!(10.004)).round_to_cents(),
            Money::new(dec!(10.00))
        );
    }

    #[test]
    fn from_cents_builds_exact_amount() {
        assert_eq!(Money::from_cents(4250), Money::new(dec!(42.50)));
    }

    #[test]
    fn entry_validation_rejects_zero_negative_and_sub_cents() {
        assert_eq!(
            Money::new(dec!(-100)).validate_entry(),
            Err(DomainError::NonPositiveAmount)
        );
        assert_eq!(
            Money::ZERO.validate_entry(),
            Err(DomainError::NonPositiveAmount)
        );
        assert_eq!(
            Money::new(dec!(1.234)).validate_entry(),
            Err(DomainError::TooManyDecimals)
        );
        assert_eq!(
            Money::new(dec!(42.5)).validate_entry(),
            Ok(Money::new(dec!(42.5)))
        );
    }

    #[test]
    fn trailing_zeros_do_not_count_as_decimals() {
        assert!(Money::new(dec!(3200.000)).has_at_most_cents());
    }

    #[test]
    fn balance_validation_accepts_zero_but_not_negative() {
        assert_eq!(Money::ZERO.validate_balance(), Ok(Money::ZERO));
        assert_eq!(
            Money::new(dec!(-1)).validate_balance(),
            Err(DomainError::NegativeAmount)
        );
    }

    #[test]
    fn division_by_zero_is_none() {
        assert_eq!(Money::new(dec!(240)).divided_by(0), None);
        assert_eq!(
            Money::new(dec!(240)).divided_by(12),
            Some(Money::new(dec!(20)))
        );
    }

    #[test]
    fn sums_amounts() {
        let total: Money = [dec!(10640), dec!(5200), dec!(14510), dec!(9310), dec!(8702)]
            .into_iter()
            .map(Money::new)
            .sum();
        assert_eq!(total, Money::new(dec!(48362)));
    }

    #[test]
    fn display_is_rounded_technical_format() {
        assert_eq!(Money::new(dec!(1234.567)).to_string(), "1234.57 EUR");
    }

    proptest! {
        #[test]
        fn addition_is_commutative_and_subtraction_inverts_it(a in -1_000_000_000i64..1_000_000_000, b in -1_000_000_000i64..1_000_000_000) {
            let (a, b) = (Money::from_cents(a), Money::from_cents(b));
            prop_assert_eq!(a + b, b + a);
            prop_assert_eq!((a + b) - b, a);
        }

        #[test]
        fn rounding_is_idempotent(units in -1_000_000_000_000i64..1_000_000_000_000, scale in 0u32..8) {
            let rounded = Money::new(Decimal::new(units, scale)).round_to_cents();
            prop_assert_eq!(rounded.round_to_cents(), rounded);
            prop_assert!(rounded.has_at_most_cents());
        }

        #[test]
        fn monthly_split_of_yearly_amount_recombines(cents in 1i64..10_000_000) {
            let yearly = Money::from_cents(cents);
            let monthly = yearly.divided_by(12).unwrap();
            prop_assert_eq!(monthly.times(Decimal::from(12)).round_to_cents(), yearly);
        }
    }
}
