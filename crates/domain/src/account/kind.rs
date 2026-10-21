use chrono::NaiveDate;

use super::{CheckingDetails, PeiFund, Position};
use crate::{DomainError, Money, Rate};

/// Type de compte, tel que choisi par l'utilisateur à la création.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountType {
    LivretA,
    Ldds,
    Pea,
    Pei,
    Crypto,
    Checking,
}

impl AccountType {
    pub const ALL: [AccountType; 6] = [
        AccountType::Checking,
        AccountType::LivretA,
        AccountType::Ldds,
        AccountType::Pea,
        AccountType::Pei,
        AccountType::Crypto,
    ];

    pub const fn label(&self) -> &'static str {
        match self {
            AccountType::LivretA => "Livret A",
            AccountType::Ldds => "LDDS",
            AccountType::Pea => "PEA",
            AccountType::Pei => "PEI",
            AccountType::Crypto => "Portefeuille crypto",
            AccountType::Checking => "Compte courant",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SavingsProduct {
    LivretA,
    Ldds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecuritiesProduct {
    Pea,
    Crypto,
}

/// Livret réglementé : un solde, un taux et un plafond de versement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavingsDetails {
    product: SavingsProduct,
    balance: Money,
    rate: Rate,
    deposit_ceiling: Money,
    balance_updated_on: NaiveDate,
}

impl SavingsDetails {
    pub fn new(
        product: SavingsProduct,
        balance: Money,
        rate: Rate,
        deposit_ceiling: Money,
        balance_updated_on: NaiveDate,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            product,
            balance: balance.validate_balance()?,
            rate,
            deposit_ceiling: deposit_ceiling.validate_entry()?,
            balance_updated_on,
        })
    }

    pub fn product(&self) -> SavingsProduct {
        self.product
    }

    pub fn balance(&self) -> Money {
        self.balance
    }

    pub fn rate(&self) -> Rate {
        self.rate
    }

    pub fn deposit_ceiling(&self) -> Money {
        self.deposit_ceiling
    }

    pub fn balance_updated_on(&self) -> NaiveDate {
        self.balance_updated_on
    }

    /// Le solde peut dépasser le plafond (intérêts capitalisés), mais plus aucun versement n'est
    /// alors possible.
    pub fn is_ceiling_reached(&self) -> bool {
        self.balance >= self.deposit_ceiling
    }

    /// Montant encore versable avant d'atteindre le plafond.
    pub fn remaining_deposit_room(&self) -> Money {
        (self.deposit_ceiling - self.balance).max(Money::ZERO)
    }

    pub(super) fn update_balance(
        &mut self,
        balance: Money,
        on: NaiveDate,
    ) -> Result<(), DomainError> {
        self.balance = balance.validate_balance()?;
        self.balance_updated_on = on;
        Ok(())
    }

    pub(super) fn update_terms(
        &mut self,
        rate: Rate,
        deposit_ceiling: Money,
    ) -> Result<(), DomainError> {
        self.deposit_ceiling = deposit_ceiling.validate_entry()?;
        self.rate = rate;
        Ok(())
    }
}

/// PEA ou portefeuille crypto : des positions valorisées au cours, plus un solde espèces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecuritiesDetails {
    product: SecuritiesProduct,
    cash: Money,
    positions: Vec<Position>,
}

impl SecuritiesDetails {
    pub fn new(
        product: SecuritiesProduct,
        cash: Money,
        positions: Vec<Position>,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            product,
            cash: cash.validate_balance()?,
            positions,
        })
    }

    pub fn product(&self) -> SecuritiesProduct {
        self.product
    }

    pub fn cash(&self) -> Money {
        self.cash
    }

    pub fn positions(&self) -> &[Position] {
        &self.positions
    }

    pub(super) fn set_cash(&mut self, cash: Money) -> Result<(), DomainError> {
        self.cash = cash.validate_balance()?;
        Ok(())
    }

    pub(super) fn positions_mut(&mut self) -> &mut Vec<Position> {
        &mut self.positions
    }
}

/// PEI : un ensemble de fonds d'épargne salariale.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmployeeSavingsDetails {
    funds: Vec<PeiFund>,
}

impl EmployeeSavingsDetails {
    pub fn new(funds: Vec<PeiFund>) -> Self {
        Self { funds }
    }

    pub fn funds(&self) -> &[PeiFund] {
        &self.funds
    }

    pub(super) fn funds_mut(&mut self) -> &mut Vec<PeiFund> {
        &mut self.funds
    }
}

/// Contenu d'un compte selon sa nature. L'ensemble est fermé : un `match` exhaustif garantit que
/// chaque nature est traitée partout (valorisation, projection, persistance).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountKind {
    Savings(SavingsDetails),
    Securities(SecuritiesDetails),
    EmployeeSavings(EmployeeSavingsDetails),
    Checking(CheckingDetails),
}

impl AccountKind {
    pub fn account_type(&self) -> AccountType {
        match self {
            AccountKind::Savings(details) => match details.product {
                SavingsProduct::LivretA => AccountType::LivretA,
                SavingsProduct::Ldds => AccountType::Ldds,
            },
            AccountKind::Securities(details) => match details.product {
                SecuritiesProduct::Pea => AccountType::Pea,
                SecuritiesProduct::Crypto => AccountType::Crypto,
            },
            AccountKind::EmployeeSavings(_) => AccountType::Pei,
            AccountKind::Checking(_) => AccountType::Checking,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn livret(balance: Money) -> SavingsDetails {
        SavingsDetails::new(
            SavingsProduct::Ldds,
            balance,
            Rate::from_percent(dec!(1.7)).unwrap(),
            Money::new(dec!(12000)),
            NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn deposit_room_until_ceiling() {
        assert_eq!(
            livret(Money::new(dec!(11900))).remaining_deposit_room(),
            Money::new(dec!(100))
        );
        assert!(!livret(Money::new(dec!(11900))).is_ceiling_reached());
    }

    #[test]
    fn balance_above_ceiling_is_allowed_but_ceiling_reached() {
        let above = livret(Money::new(dec!(12150)));
        assert!(above.is_ceiling_reached());
        assert_eq!(above.remaining_deposit_room(), Money::ZERO);
    }

    #[test]
    fn rejects_negative_balance() {
        let result = SavingsDetails::new(
            SavingsProduct::LivretA,
            Money::new(dec!(-1)),
            Rate::ZERO,
            Money::new(dec!(22950)),
            NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
        );
        assert_eq!(result, Err(DomainError::NegativeAmount));
    }

    #[test]
    fn kind_maps_to_account_type() {
        let pea = AccountKind::Securities(
            SecuritiesDetails::new(SecuritiesProduct::Pea, Money::ZERO, vec![]).unwrap(),
        );
        assert_eq!(pea.account_type(), AccountType::Pea);
        assert_eq!(
            AccountKind::Savings(livret(Money::ZERO)).account_type(),
            AccountType::Ldds
        );
        assert_eq!(
            AccountKind::EmployeeSavings(EmployeeSavingsDetails::default()).account_type(),
            AccountType::Pei
        );
    }
}
