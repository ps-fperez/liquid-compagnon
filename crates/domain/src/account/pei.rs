use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::AssetRef;
use crate::{DomainError, FundId, Label, Money};

/// Valeur de part saisie manuellement à une date, avec le cours de l'actif de référence (en
/// euros) relevé ce jour-là. C'est le point de départ de l'estimation par indice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavAnchor {
    nav: Money,
    date: NaiveDate,
    reference_price: Option<Money>,
}

impl NavAnchor {
    pub fn new(
        nav: Money,
        date: NaiveDate,
        reference_price: Option<Money>,
    ) -> Result<Self, DomainError> {
        if !nav.is_positive() || reference_price.is_some_and(|price| !price.is_positive()) {
            return Err(DomainError::NonPositiveAmount);
        }
        Ok(Self {
            nav,
            date,
            reference_price,
        })
    }

    pub fn nav(&self) -> Money {
        self.nav
    }

    pub fn date(&self) -> NaiveDate {
        self.date
    }

    pub fn reference_price(&self) -> Option<Money> {
        self.reference_price
    }
}

/// Fonds d'un Plan d'Épargne Interentreprises (FCPE). Les fonds n'ont pas d'API de cotation :
/// ils sont valorisés à partir de la dernière valeur de part saisie, éventuellement extrapolée
/// avec un actif de référence coté.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeiFund {
    id: FundId,
    name: Label,
    units: Decimal,
    reference: Option<AssetRef>,
    anchor: NavAnchor,
}

impl PeiFund {
    pub fn new(
        id: FundId,
        name: Label,
        units: Decimal,
        reference: Option<AssetRef>,
        anchor: NavAnchor,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id,
            name,
            units: validated_units(units)?,
            reference,
            anchor,
        })
    }

    pub fn id(&self) -> FundId {
        self.id
    }

    pub fn name(&self) -> &Label {
        &self.name
    }

    pub fn units(&self) -> Decimal {
        self.units
    }

    pub fn reference(&self) -> Option<&AssetRef> {
        self.reference.as_ref()
    }

    pub fn anchor(&self) -> &NavAnchor {
        &self.anchor
    }

    pub fn rename(&mut self, name: Label) {
        self.name = name;
    }

    pub fn set_units(&mut self, units: Decimal) -> Result<(), DomainError> {
        self.units = validated_units(units)?;
        Ok(())
    }

    /// Change l'actif de référence. L'ancien cours d'ancrage ne correspond plus à rien : il est
    /// effacé et le fonds est valorisé à sa valeur de part manuelle jusqu'au prochain ancrage.
    pub fn set_reference(&mut self, reference: Option<AssetRef>) {
        let unchanged = match (&self.reference, &reference) {
            (Some(current), Some(new)) => current.is_same_asset(new),
            (None, None) => true,
            _ => false,
        };
        if !unchanged {
            self.anchor.reference_price = None;
        }
        self.reference = reference;
    }

    /// Enregistre une nouvelle valeur de part réelle : elle remplace l'estimation et devient le
    /// nouveau point d'ancrage. Une valeur plus ancienne que l'ancrage actuel est refusée.
    pub fn record_nav(&mut self, anchor: NavAnchor) -> Result<(), DomainError> {
        if anchor.date < self.anchor.date {
            return Err(DomainError::OutdatedNav);
        }
        self.anchor = anchor;
        Ok(())
    }
}

fn validated_units(units: Decimal) -> Result<Decimal, DomainError> {
    if units < Decimal::ZERO {
        return Err(DomainError::NonPositiveQuantity);
    }
    Ok(units)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{CurrencyCode, QuoteSource};
    use rust_decimal_macros::dec;

    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    fn iwda() -> AssetRef {
        AssetRef::new(
            QuoteSource::TwelveData,
            "IWDA",
            "iShares Core MSCI World",
            CurrencyCode::eur(),
        )
        .unwrap()
    }

    fn world_fund() -> PeiFund {
        PeiFund::new(
            FundId::generate(),
            Label::new("Amundi Actions Monde").unwrap(),
            dec!(120),
            Some(iwda()),
            NavAnchor::new(
                Money::new(dec!(25)),
                date(10, 1),
                Some(Money::new(dec!(100))),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn newer_nav_becomes_the_anchor() {
        let mut fund = world_fund();
        let real = NavAnchor::new(
            Money::new(dec!(25.80)),
            date(10, 20),
            Some(Money::new(dec!(104))),
        )
        .unwrap();

        fund.record_nav(real.clone()).unwrap();

        assert_eq!(fund.anchor(), &real);
    }

    #[test]
    fn older_nav_is_rejected() {
        let mut fund = world_fund();
        let stale = NavAnchor::new(Money::new(dec!(24)), date(9, 1), None).unwrap();
        assert_eq!(fund.record_nav(stale), Err(DomainError::OutdatedNav));
    }

    #[test]
    fn changing_reference_clears_anchor_price() {
        let mut fund = world_fund();
        let sp500 =
            AssetRef::new(QuoteSource::TwelveData, "CSPX", "", CurrencyCode::eur()).unwrap();

        fund.set_reference(Some(iwda()));
        assert_eq!(fund.anchor().reference_price(), Some(Money::new(dec!(100))));

        fund.set_reference(Some(sp500));
        assert_eq!(fund.anchor().reference_price(), None);
        assert_eq!(fund.anchor().nav(), Money::new(dec!(25)));
    }

    #[test]
    fn rejects_negative_units_and_non_positive_nav() {
        let mut fund = world_fund();
        assert_eq!(
            fund.set_units(dec!(-1)),
            Err(DomainError::NonPositiveQuantity)
        );
        assert!(fund.set_units(dec!(0)).is_ok());
        assert_eq!(
            NavAnchor::new(Money::ZERO, date(10, 1), None),
            Err(DomainError::NonPositiveAmount)
        );
    }
}
