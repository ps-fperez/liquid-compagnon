//! Calculs numériques en décimal exact.

use rust_decimal::Decimal;

use crate::Rate;

/// Facteur de capitalisation mensuel équivalent à un taux annuel : (1 + r)^(1/12).
pub(crate) fn monthly_factor(annual: Rate) -> Decimal {
    nth_root(Decimal::ONE + annual.as_fraction(), 12)
}

/// Racine n-ième d'un décimal strictement positif, par la méthode de Newton (déterministe, sans
/// passer par exp/ln).
fn nth_root(value: Decimal, n: u32) -> Decimal {
    debug_assert!(value > Decimal::ZERO && n > 0);
    if value == Decimal::ONE || n == 1 {
        return value;
    }
    let n_dec = Decimal::from(n);
    let tolerance = Decimal::new(1, 24);
    let mut x = Decimal::ONE + (value - Decimal::ONE) / n_dec;
    for _ in 0..200 {
        let next = ((n_dec - Decimal::ONE) * x + value / pow(x, n - 1)) / n_dec;
        if (next - x).abs() < tolerance {
            return next;
        }
        x = next;
    }
    x
}

fn pow(base: Decimal, exponent: u32) -> Decimal {
    (0..exponent).fold(Decimal::ONE, |acc, _| acc * base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rust_decimal_macros::dec;

    #[test]
    fn zero_rate_gives_unit_factor() {
        assert_eq!(monthly_factor(Rate::ZERO), Decimal::ONE);
    }

    #[test]
    fn twelve_months_of_three_percent_compound_to_three_percent() {
        let factor = monthly_factor(Rate::from_percent(dec!(3)).unwrap());
        let yearly = pow(factor, 12);
        assert!((yearly - dec!(1.03)).abs() < dec!(0.000000000000000001));
    }

    proptest! {
        #[test]
        fn factor_to_the_twelfth_recovers_annual_rate(permille in -900i64..3000) {
            let rate = Rate::from_fraction(Decimal::new(permille, 3)).unwrap();
            let yearly = pow(monthly_factor(rate), 12);
            let expected = Decimal::ONE + rate.as_fraction();
            prop_assert!((yearly - expected).abs() < dec!(0.000000000000000001), "{} vs {}", yearly, expected);
        }
    }
}
