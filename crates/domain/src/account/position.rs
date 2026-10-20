use rust_decimal::Decimal;

use super::AssetRef;
use crate::{DomainError, Money};

/// Ligne d'un PEA ou d'un portefeuille crypto : une quantité d'un actif coté.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    asset: AssetRef,
    quantity: Decimal,
    unit_cost: Option<Money>,
}

impl Position {
    /// `unit_cost` est le prix de revient unitaire en euros (facultatif, non borné aux centimes :
    /// le prix d'un crypto-actif peut avoir beaucoup de décimales).
    pub fn new(
        asset: AssetRef,
        quantity: Decimal,
        unit_cost: Option<Money>,
    ) -> Result<Self, DomainError> {
        if quantity <= Decimal::ZERO {
            return Err(DomainError::NonPositiveQuantity);
        }
        if unit_cost.is_some_and(|cost| cost.is_negative()) {
            return Err(DomainError::NegativeAmount);
        }
        Ok(Self {
            asset,
            quantity,
            unit_cost,
        })
    }

    pub fn asset(&self) -> &AssetRef {
        &self.asset
    }

    pub fn quantity(&self) -> Decimal {
        self.quantity
    }

    pub fn unit_cost(&self) -> Option<Money> {
        self.unit_cost
    }

    /// Prix de revient total (quantité × prix de revient unitaire), si connu.
    pub fn cost_basis(&self) -> Option<Money> {
        self.unit_cost.map(|cost| cost.times(self.quantity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{CurrencyCode, QuoteSource};
    use rust_decimal_macros::dec;

    fn bitcoin() -> AssetRef {
        AssetRef::new(
            QuoteSource::CoinGecko,
            "bitcoin",
            "Bitcoin",
            CurrencyCode::eur(),
        )
        .unwrap()
    }

    #[test]
    fn cost_basis_is_quantity_times_unit_cost() {
        let position = Position::new(bitcoin(), dec!(0.15), Some(Money::new(dec!(40000)))).unwrap();
        assert_eq!(position.cost_basis(), Some(Money::new(dec!(6000))));
    }

    #[test]
    fn cost_basis_is_unknown_without_unit_cost() {
        let position = Position::new(bitcoin(), dec!(0.15), None).unwrap();
        assert_eq!(position.cost_basis(), None);
    }

    #[test]
    fn rejects_non_positive_quantity_and_negative_cost() {
        assert_eq!(
            Position::new(bitcoin(), dec!(0), None),
            Err(DomainError::NonPositiveQuantity)
        );
        assert_eq!(
            Position::new(bitcoin(), dec!(1), Some(Money::new(dec!(-1)))),
            Err(DomainError::NegativeAmount)
        );
    }
}
