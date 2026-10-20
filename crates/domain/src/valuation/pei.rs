use crate::Money;
use crate::account::NavAnchor;

use super::Quote;

/// Nature d'une valeur de part de PEI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavKind {
    /// Valeur saisie par l'utilisateur (relevé de son espace d'épargne salariale).
    Real,
    /// Valeur extrapolée par la variation de l'actif de référence depuis la dernière saisie.
    Estimated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavEstimate {
    pub nav: Money,
    pub kind: NavKind,
}

/// Valeur de part d'un fonds PEI.
///
/// Estimée = dernière valeur réelle × (cours de référence actuel ÷ cours de référence à la date
/// de cette valeur réelle). L'estimation n'a lieu que si le cours d'ancrage est connu et que le
/// cours actuel est postérieur au jour de l'ancrage ; sinon la valeur réelle est conservée.
pub fn estimate_nav(anchor: &NavAnchor, reference_quote: Option<&Quote>) -> NavEstimate {
    let real = NavEstimate {
        nav: anchor.nav(),
        kind: NavKind::Real,
    };
    let (Some(anchor_price), Some(quote)) = (anchor.reference_price(), reference_quote) else {
        return real;
    };
    if quote.as_of().date_naive() <= anchor.date() || !anchor_price.is_positive() {
        return real;
    }
    NavEstimate {
        nav: anchor
            .nav()
            .times(quote.price().amount() / anchor_price.amount()),
        kind: NavKind::Estimated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{AssetRef, CurrencyCode, QuoteSource};
    use crate::valuation::QuoteFreshness;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    fn iwda_quote(price: Money, on: NaiveDate) -> Quote {
        let iwda = AssetRef::new(QuoteSource::TwelveData, "IWDA", "", CurrencyCode::eur()).unwrap();
        Quote::new(
            iwda,
            price,
            on.and_hms_opt(9, 42, 0).unwrap().and_utc(),
            QuoteFreshness::Live,
        )
    }

    // Spec market-pricing : « Estimation par l'indice »
    #[test]
    fn extrapolates_nav_with_reference_variation() {
        let anchor = NavAnchor::new(
            Money::new(dec!(25)),
            date(10, 1),
            Some(Money::new(dec!(100))),
        )
        .unwrap();
        let quote = iwda_quote(Money::new(dec!(104)), date(10, 20));

        let estimate = estimate_nav(&anchor, Some(&quote));

        assert_eq!(estimate.nav, Money::new(dec!(26)));
        assert_eq!(estimate.kind, NavKind::Estimated);
    }

    // Spec market-pricing : « Recalage »
    #[test]
    fn freshly_recorded_nav_is_real() {
        let anchor = NavAnchor::new(
            Money::new(dec!(25.80)),
            date(10, 20),
            Some(Money::new(dec!(104))),
        )
        .unwrap();
        let same_day = iwda_quote(Money::new(dec!(104)), date(10, 20));

        let estimate = estimate_nav(&anchor, Some(&same_day));

        assert_eq!(
            estimate,
            NavEstimate {
                nav: Money::new(dec!(25.80)),
                kind: NavKind::Real
            }
        );
    }

    #[test]
    fn later_estimates_start_from_new_anchor() {
        let anchor = NavAnchor::new(
            Money::new(dec!(25.80)),
            date(10, 20),
            Some(Money::new(dec!(104))),
        )
        .unwrap();
        let next_week = iwda_quote(Money::new(dec!(106.08)), date(10, 27));

        assert_eq!(
            estimate_nav(&anchor, Some(&next_week)).nav,
            Money::new(dec!(26.316))
        );
    }

    #[test]
    fn keeps_real_nav_without_reference_price_or_quote() {
        let without_price = NavAnchor::new(Money::new(dec!(25)), date(10, 1), None).unwrap();
        let quote = iwda_quote(Money::new(dec!(104)), date(10, 20));
        assert_eq!(
            estimate_nav(&without_price, Some(&quote)).kind,
            NavKind::Real
        );

        let with_price = NavAnchor::new(
            Money::new(dec!(25)),
            date(10, 1),
            Some(Money::new(dec!(100))),
        )
        .unwrap();
        assert_eq!(
            estimate_nav(&with_price, None),
            NavEstimate {
                nav: Money::new(dec!(25)),
                kind: NavKind::Real
            }
        );
    }
}
