use chrono::NaiveDate;

use crate::{DomainError, IncomeId, Label, Money, Month, SalaryId};

/// Salaire net mensuel (net à payer) applicable à partir d'un mois donné.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SalaryEntry {
    id: SalaryId,
    effective_from: Month,
    net_amount: Money,
}

impl SalaryEntry {
    pub fn new(
        id: SalaryId,
        effective_from: Month,
        net_amount: Money,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id,
            effective_from,
            net_amount: net_amount.validate_entry()?,
        })
    }

    pub fn id(&self) -> SalaryId {
        self.id
    }

    pub fn effective_from(&self) -> Month {
        self.effective_from
    }

    pub fn net_amount(&self) -> Money {
        self.net_amount
    }
}

/// Revenu exceptionnel (prime, remboursement…) compté uniquement dans le mois de sa date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneOffIncome {
    id: IncomeId,
    label: Label,
    amount: Money,
    date: NaiveDate,
}

impl OneOffIncome {
    pub fn new(
        id: IncomeId,
        label: Label,
        amount: Money,
        date: NaiveDate,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id,
            label,
            amount: amount.validate_entry()?,
            date,
        })
    }

    pub fn id(&self) -> IncomeId {
        self.id
    }

    pub fn label(&self) -> &Label {
        &self.label
    }

    pub fn amount(&self) -> Money {
        self.amount
    }

    pub fn date(&self) -> NaiveDate {
        self.date
    }

    pub fn month(&self) -> Month {
        Month::of(self.date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn salary_must_be_positive() {
        let result = SalaryEntry::new(
            SalaryId::generate(),
            Month::new(2026, 1).unwrap(),
            Money::new(dec!(-100)),
        );
        assert_eq!(result, Err(DomainError::NonPositiveAmount));
    }

    #[test]
    fn salary_keeps_effective_month_and_amount() {
        let salary = SalaryEntry::new(
            SalaryId::generate(),
            Month::new(2026, 1).unwrap(),
            Money::new(dec!(3200.00)),
        )
        .unwrap();
        assert_eq!(salary.effective_from(), Month::new(2026, 1).unwrap());
        assert_eq!(salary.net_amount(), Money::new(dec!(3200)));
    }

    #[test]
    fn one_off_income_belongs_to_its_month() {
        let bonus = OneOffIncome::new(
            IncomeId::generate(),
            Label::new("Prime de fin d'année").unwrap(),
            Money::new(dec!(1500)),
            NaiveDate::from_ymd_opt(2026, 12, 15).unwrap(),
        )
        .unwrap();
        assert_eq!(bonus.month(), Month::new(2026, 12).unwrap());
    }

    #[test]
    fn one_off_income_rejects_sub_cent_amounts() {
        let result = OneOffIncome::new(
            IncomeId::generate(),
            Label::new("Remboursement").unwrap(),
            Money::new(dec!(12.345)),
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
        );
        assert_eq!(result, Err(DomainError::TooManyDecimals));
    }
}
