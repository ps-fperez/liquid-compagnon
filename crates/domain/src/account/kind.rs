use chrono::{DateTime, NaiveDate, Utc};

use super::{CheckingDetails, PeiFund, Position};
use crate::ledger::BalanceReference;
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
    Livret,
    AssuranceVie,
}

impl AccountType {
    pub const ALL: [AccountType; 8] = [
        AccountType::Checking,
        AccountType::LivretA,
        AccountType::Ldds,
        AccountType::Livret,
        AccountType::Pea,
        AccountType::AssuranceVie,
        AccountType::Crypto,
        AccountType::Pei,
    ];

    pub const fn label(&self) -> &'static str {
        match self {
            AccountType::LivretA => "Livret A",
            AccountType::Ldds => "LDDS",
            AccountType::Pea => "PEA",
            AccountType::Pei => "Épargne salariale",
            AccountType::Crypto => "Portefeuille crypto",
            AccountType::Checking => "Compte courant",
            AccountType::Livret => "Autre livret",
            AccountType::AssuranceVie => "Assurance-vie ou PER",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SavingsProduct {
    LivretA,
    Ldds,
    /// Autre livret (LEP, Livret Jeune, livret bancaire…) : taux libre, plafond facultatif.
    Livret,
    /// Assurance-vie ou PER individuel : valeur saisie, sans plafond.
    AssuranceVie,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecuritiesProduct {
    Pea,
    Crypto,
}

/// Livret ou assurance-vie : un solde de référence (dernier solde saisi), un taux et un plafond
/// de versement facultatif. Le solde courant ajoute les virements postérieurs à la référence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavingsDetails {
    product: SavingsProduct,
    reference: BalanceReference,
    rate: Rate,
    deposit_ceiling: Option<Money>,
}

impl SavingsDetails {
    pub fn new(
        product: SavingsProduct,
        reference: BalanceReference,
        rate: Rate,
        deposit_ceiling: Option<Money>,
    ) -> Result<Self, DomainError> {
        reference.amount.validate_balance()?;
        Ok(Self {
            product,
            reference,
            rate,
            deposit_ceiling: deposit_ceiling.map(Money::validate_entry).transpose()?,
        })
    }

    pub fn product(&self) -> SavingsProduct {
        self.product
    }

    /// Dernier solde saisi.
    pub fn balance(&self) -> Money {
        self.reference.amount
    }

    pub fn reference(&self) -> &BalanceReference {
        &self.reference
    }

    pub fn rate(&self) -> Rate {
        self.rate
    }

    pub fn deposit_ceiling(&self) -> Option<Money> {
        self.deposit_ceiling
    }

    pub fn balance_updated_on(&self) -> NaiveDate {
        self.reference.on
    }

    /// Le solde peut dépasser le plafond (intérêts capitalisés), mais plus aucun versement n'est
    /// alors possible.
    pub fn is_ceiling_reached(&self, balance: Money) -> bool {
        self.deposit_ceiling
            .is_some_and(|ceiling| balance >= ceiling)
    }

    /// Montant encore versable avant d'atteindre le plafond (illimité sans plafond).
    pub fn remaining_deposit_room(&self, balance: Money) -> Option<Money> {
        self.deposit_ceiling
            .map(|ceiling| (ceiling - balance).max(Money::ZERO))
    }

    pub(super) fn update_balance(
        &mut self,
        balance: Money,
        on: NaiveDate,
        at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.reference = BalanceReference::new(balance.validate_balance()?, on, at);
        Ok(())
    }

    pub(super) fn update_terms(
        &mut self,
        rate: Rate,
        deposit_ceiling: Option<Money>,
    ) -> Result<(), DomainError> {
        self.deposit_ceiling = deposit_ceiling.map(Money::validate_entry).transpose()?;
        self.rate = rate;
        Ok(())
    }
}

/// PEA ou portefeuille crypto : des positions valorisées au cours, plus des espèces (solde de
/// référence, que les virements font évoluer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecuritiesDetails {
    product: SecuritiesProduct,
    cash: BalanceReference,
    positions: Vec<Position>,
}

impl SecuritiesDetails {
    pub fn new(
        product: SecuritiesProduct,
        cash: BalanceReference,
        positions: Vec<Position>,
    ) -> Result<Self, DomainError> {
        cash.amount.validate_balance()?;
        Ok(Self {
            product,
            cash,
            positions,
        })
    }

    pub fn product(&self) -> SecuritiesProduct {
        self.product
    }

    /// Dernier solde espèces saisi.
    pub fn cash(&self) -> Money {
        self.cash.amount
    }

    pub fn cash_reference(&self) -> &BalanceReference {
        &self.cash
    }

    pub fn positions(&self) -> &[Position] {
        &self.positions
    }

    pub(super) fn set_cash(
        &mut self,
        cash: Money,
        on: NaiveDate,
        at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.cash = BalanceReference::new(cash.validate_balance()?, on, at);
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
                SavingsProduct::Livret => AccountType::Livret,
                SavingsProduct::AssuranceVie => AccountType::AssuranceVie,
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
    use chrono::TimeZone;
    use rust_decimal_macros::dec;

    fn reference(amount: Money) -> BalanceReference {
        BalanceReference::new(
            amount,
            NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
            Utc.with_ymd_and_hms(2026, 10, 20, 9, 0, 0).unwrap(),
        )
    }

    fn livret(balance: Money) -> SavingsDetails {
        SavingsDetails::new(
            SavingsProduct::Ldds,
            reference(balance),
            Rate::from_percent(dec!(1.7)).unwrap(),
            Some(Money::new(dec!(12000))),
        )
        .unwrap()
    }

    #[test]
    fn deposit_room_until_ceiling() {
        let ldds = livret(Money::new(dec!(11900)));
        assert_eq!(
            ldds.remaining_deposit_room(Money::new(dec!(11900))),
            Some(Money::new(dec!(100)))
        );
        assert!(!ldds.is_ceiling_reached(Money::new(dec!(11900))));
        assert!(ldds.is_ceiling_reached(Money::new(dec!(12150))));
        assert_eq!(
            ldds.remaining_deposit_room(Money::new(dec!(12150))),
            Some(Money::ZERO)
        );
    }

    // Spec generic-accounts : « Livret sans plafond »
    #[test]
    fn livret_without_ceiling_never_reaches_it() {
        let open = SavingsDetails::new(
            SavingsProduct::Livret,
            reference(Money::new(dec!(50000))),
            Rate::ZERO,
            None,
        )
        .unwrap();
        assert!(!open.is_ceiling_reached(Money::new(dec!(1000000))));
        assert_eq!(open.remaining_deposit_room(Money::ZERO), None);
    }

    #[test]
    fn rejects_negative_balance() {
        let result = SavingsDetails::new(
            SavingsProduct::LivretA,
            reference(Money::new(dec!(-1))),
            Rate::ZERO,
            Some(Money::new(dec!(22950))),
        );
        assert_eq!(result, Err(DomainError::NegativeAmount));
    }

    #[test]
    fn kind_maps_to_account_type() {
        let pea = AccountKind::Securities(
            SecuritiesDetails::new(SecuritiesProduct::Pea, reference(Money::ZERO), vec![]).unwrap(),
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
        assert_eq!(AccountType::Pei.label(), "Épargne salariale");
    }
}
