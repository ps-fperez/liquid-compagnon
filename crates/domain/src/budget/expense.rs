use chrono::NaiveDate;

use crate::{CategoryId, DomainError, ExpenseId, Label, Money, Month};

/// Dépense ponctuelle, rattachée à une catégorie et à une date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expense {
    id: ExpenseId,
    amount: Money,
    category: CategoryId,
    date: NaiveDate,
    label: Option<Label>,
}

impl Expense {
    pub fn new(
        id: ExpenseId,
        amount: Money,
        category: CategoryId,
        date: NaiveDate,
        label: Option<Label>,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id,
            amount: amount.validate_entry()?,
            category,
            date,
            label,
        })
    }

    pub fn id(&self) -> ExpenseId {
        self.id
    }

    pub fn amount(&self) -> Money {
        self.amount
    }

    pub fn category(&self) -> CategoryId {
        self.category
    }

    pub fn date(&self) -> NaiveDate {
        self.date
    }

    pub fn label(&self) -> Option<&Label> {
        self.label.as_ref()
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
    fn quick_expense_without_label() {
        let groceries = CategoryId::generate();
        let date = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();

        let expense = Expense::new(
            ExpenseId::generate(),
            Money::new(dec!(42.50)),
            groceries,
            date,
            None,
        )
        .unwrap();

        assert_eq!(expense.amount(), Money::new(dec!(42.5)));
        assert_eq!(expense.category(), groceries);
        assert_eq!(expense.month(), Month::new(2026, 10).unwrap());
        assert!(expense.label().is_none());
    }

    #[test]
    fn rejects_non_positive_amount() {
        let result = Expense::new(
            ExpenseId::generate(),
            Money::ZERO,
            CategoryId::generate(),
            NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
            None,
        );
        assert_eq!(result, Err(DomainError::NonPositiveAmount));
    }
}
