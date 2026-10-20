use crate::{CategoryId, ChargeId, DomainError, Label, Money, Month};

/// Fréquence de paiement d'une charge fixe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Periodicity {
    Monthly,
    Quarterly,
    Yearly,
}

impl Periodicity {
    /// Nombre de mois couverts par un paiement.
    pub const fn months(&self) -> u32 {
        match self {
            Periodicity::Monthly => 1,
            Periodicity::Quarterly => 3,
            Periodicity::Yearly => 12,
        }
    }
}

/// Charge fixe récurrente (loyer, assurance, abonnement…), active de `start` à `end` inclus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurringCharge {
    id: ChargeId,
    label: Label,
    amount: Money,
    category: CategoryId,
    periodicity: Periodicity,
    start: Month,
    end: Option<Month>,
}

impl RecurringCharge {
    pub fn new(
        id: ChargeId,
        label: Label,
        amount: Money,
        category: CategoryId,
        periodicity: Periodicity,
        start: Month,
        end: Option<Month>,
    ) -> Result<Self, DomainError> {
        if end.is_some_and(|end| end < start) {
            return Err(DomainError::EndBeforeStart);
        }
        Ok(Self {
            id,
            label,
            amount: amount.validate_entry()?,
            category,
            periodicity,
            start,
            end,
        })
    }

    pub fn id(&self) -> ChargeId {
        self.id
    }

    pub fn label(&self) -> &Label {
        &self.label
    }

    pub fn amount(&self) -> Money {
        self.amount
    }

    pub fn category(&self) -> CategoryId {
        self.category
    }

    pub fn periodicity(&self) -> Periodicity {
        self.periodicity
    }

    pub fn start(&self) -> Month {
        self.start
    }

    pub fn end(&self) -> Option<Month> {
        self.end
    }

    /// Montant ramené au mois (non arrondi) : montant ÷ 1, 3 ou 12.
    pub fn monthly_equivalent(&self) -> Money {
        self.amount
            .divided_by(self.periodicity.months())
            .expect("une périodicité couvre toujours au moins un mois")
    }

    pub fn is_active_in(&self, month: Month) -> bool {
        month >= self.start && self.end.is_none_or(|end| month <= end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn month(year: i32, month: u32) -> Month {
        Month::new(year, month).unwrap()
    }

    fn charge(
        amount: Money,
        periodicity: Periodicity,
        start: Month,
        end: Option<Month>,
    ) -> Result<RecurringCharge, DomainError> {
        RecurringCharge::new(
            ChargeId::generate(),
            Label::new("Assurance habitation").unwrap(),
            amount,
            CategoryId::generate(),
            periodicity,
            start,
            end,
        )
    }

    #[test]
    fn yearly_charge_monthly_equivalent() {
        let insurance = charge(
            Money::new(dec!(240)),
            Periodicity::Yearly,
            month(2026, 1),
            None,
        )
        .unwrap();
        assert_eq!(insurance.monthly_equivalent(), Money::new(dec!(20)));
    }

    #[test]
    fn quarterly_charge_monthly_equivalent() {
        let water = charge(
            Money::new(dec!(90)),
            Periodicity::Quarterly,
            month(2026, 1),
            None,
        )
        .unwrap();
        assert_eq!(water.monthly_equivalent(), Money::new(dec!(30)));
    }

    #[test]
    fn ended_charge_is_inactive_after_end_month() {
        let ended = charge(
            Money::new(dec!(50)),
            Periodicity::Monthly,
            month(2025, 6),
            Some(month(2026, 3)),
        )
        .unwrap();
        assert!(!ended.is_active_in(month(2025, 5)));
        assert!(ended.is_active_in(month(2025, 6)));
        assert!(ended.is_active_in(month(2026, 3)));
        assert!(!ended.is_active_in(month(2026, 4)));
    }

    #[test]
    fn open_ended_charge_stays_active() {
        let rent = charge(
            Money::new(dec!(900)),
            Periodicity::Monthly,
            month(2024, 1),
            None,
        )
        .unwrap();
        assert!(rent.is_active_in(month(2030, 1)));
    }

    #[test]
    fn rejects_end_before_start_and_invalid_amount() {
        assert_eq!(
            charge(
                Money::new(dec!(10)),
                Periodicity::Monthly,
                month(2026, 5),
                Some(month(2026, 4))
            ),
            Err(DomainError::EndBeforeStart)
        );
        assert_eq!(
            charge(Money::ZERO, Periodicity::Monthly, month(2026, 5), None),
            Err(DomainError::NonPositiveAmount)
        );
    }
}
