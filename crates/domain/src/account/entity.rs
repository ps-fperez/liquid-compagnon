use chrono::NaiveDate;

use super::{
    AccountKind, AccountType, AssetRef, EmployeeSavingsDetails, NavAnchor, PeiFund, Position,
    SavingsDetails, SavingsProduct, ScenarioRates, SecuritiesDetails, SecuritiesProduct,
};
use crate::{AccountId, Color, DomainError, FundId, Label, Money, Rate, defaults};

/// Compte d'épargne (agrégat). Toutes les modifications passent par ses méthodes, qui
/// garantissent les invariants (montants valides, opérations cohérentes avec la nature du compte).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    name: Label,
    color: Color,
    kind: AccountKind,
    monthly_contribution: Money,
    scenario_rates: ScenarioRates,
    archived: bool,
}

impl Account {
    /// Ouvre un compte vide avec les valeurs par défaut de son type (taux et plafond des livrets,
    /// rendements de projection).
    pub fn open(
        id: AccountId,
        name: Label,
        color: Color,
        account_type: AccountType,
        today: NaiveDate,
    ) -> Self {
        let kind = match account_type {
            AccountType::LivretA => empty_savings(SavingsProduct::LivretA, today),
            AccountType::Ldds => empty_savings(SavingsProduct::Ldds, today),
            AccountType::Pea => empty_securities(SecuritiesProduct::Pea),
            AccountType::Crypto => empty_securities(SecuritiesProduct::Crypto),
            AccountType::Pei => AccountKind::EmployeeSavings(EmployeeSavingsDetails::default()),
        };
        let savings_rate = match &kind {
            AccountKind::Savings(details) => Some(details.rate()),
            _ => None,
        };
        Self {
            id,
            name,
            color,
            kind,
            monthly_contribution: Money::ZERO,
            scenario_rates: defaults::scenario_rates(account_type, savings_rate),
            archived: false,
        }
    }

    /// Reconstitue un compte persisté.
    pub fn restore(
        id: AccountId,
        name: Label,
        color: Color,
        kind: AccountKind,
        monthly_contribution: Money,
        scenario_rates: ScenarioRates,
        archived: bool,
    ) -> Self {
        Self {
            id,
            name,
            color,
            kind,
            monthly_contribution,
            scenario_rates,
            archived,
        }
    }

    pub fn id(&self) -> AccountId {
        self.id
    }

    pub fn name(&self) -> &Label {
        &self.name
    }

    pub fn color(&self) -> &Color {
        &self.color
    }

    pub fn kind(&self) -> &AccountKind {
        &self.kind
    }

    pub fn account_type(&self) -> AccountType {
        self.kind.account_type()
    }

    pub fn monthly_contribution(&self) -> Money {
        self.monthly_contribution
    }

    pub fn scenario_rates(&self) -> &ScenarioRates {
        &self.scenario_rates
    }

    pub fn is_archived(&self) -> bool {
        self.archived
    }

    pub fn rename(&mut self, name: Label) {
        self.name = name;
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_monthly_contribution(&mut self, amount: Money) -> Result<(), DomainError> {
        self.monthly_contribution = amount.validate_balance()?;
        Ok(())
    }

    pub fn set_scenario_rates(&mut self, rates: ScenarioRates) {
        self.scenario_rates = rates;
    }

    pub fn archive(&mut self) {
        self.archived = true;
    }

    pub fn unarchive(&mut self) {
        self.archived = false;
    }

    // --- Livrets ---------------------------------------------------------------------------

    pub fn update_savings_balance(
        &mut self,
        balance: Money,
        on: NaiveDate,
    ) -> Result<(), DomainError> {
        self.savings_mut()?.update_balance(balance, on)
    }

    /// Met à jour le taux et le plafond d'un livret. Si les rendements de projection suivaient le
    /// taux du livret (valeur par défaut), ils suivent le nouveau taux.
    pub fn set_savings_terms(
        &mut self,
        rate: Rate,
        deposit_ceiling: Money,
    ) -> Result<(), DomainError> {
        let previous_rate = self.savings_mut()?.rate();
        self.savings_mut()?.update_terms(rate, deposit_ceiling)?;
        if self.scenario_rates.is_uniform(previous_rate) {
            self.scenario_rates = ScenarioRates::uniform(rate);
        }
        Ok(())
    }

    // --- PEA / crypto ----------------------------------------------------------------------

    pub fn set_cash(&mut self, cash: Money) -> Result<(), DomainError> {
        self.securities_mut()?.set_cash(cash)
    }

    /// Ajoute une position, ou remplace celle portant sur le même actif.
    pub fn upsert_position(&mut self, position: Position) -> Result<(), DomainError> {
        let positions = self.securities_mut()?.positions_mut();
        match positions
            .iter_mut()
            .find(|p| p.asset().is_same_asset(position.asset()))
        {
            Some(existing) => *existing = position,
            None => positions.push(position),
        }
        Ok(())
    }

    pub fn remove_position(&mut self, asset: &AssetRef) -> Result<(), DomainError> {
        let positions = self.securities_mut()?.positions_mut();
        let before = positions.len();
        positions.retain(|p| !p.asset().is_same_asset(asset));
        if positions.len() == before {
            return Err(DomainError::UnknownPosition);
        }
        Ok(())
    }

    // --- PEI -------------------------------------------------------------------------------

    /// Ajoute un fonds, ou remplace celui de même identifiant.
    pub fn upsert_fund(&mut self, fund: PeiFund) -> Result<(), DomainError> {
        let funds = self.employee_savings_mut()?.funds_mut();
        match funds.iter_mut().find(|f| f.id() == fund.id()) {
            Some(existing) => *existing = fund,
            None => funds.push(fund),
        }
        Ok(())
    }

    pub fn remove_fund(&mut self, fund_id: FundId) -> Result<(), DomainError> {
        let funds = self.employee_savings_mut()?.funds_mut();
        let before = funds.len();
        funds.retain(|f| f.id() != fund_id);
        if funds.len() == before {
            return Err(DomainError::UnknownFund);
        }
        Ok(())
    }

    pub fn record_pei_nav(
        &mut self,
        fund_id: FundId,
        anchor: NavAnchor,
    ) -> Result<(), DomainError> {
        self.employee_savings_mut()?
            .funds_mut()
            .iter_mut()
            .find(|f| f.id() == fund_id)
            .ok_or(DomainError::UnknownFund)?
            .record_nav(anchor)
    }

    fn savings_mut(&mut self) -> Result<&mut SavingsDetails, DomainError> {
        match &mut self.kind {
            AccountKind::Savings(details) => Ok(details),
            _ => Err(DomainError::WrongAccountKind),
        }
    }

    fn securities_mut(&mut self) -> Result<&mut SecuritiesDetails, DomainError> {
        match &mut self.kind {
            AccountKind::Securities(details) => Ok(details),
            _ => Err(DomainError::WrongAccountKind),
        }
    }

    fn employee_savings_mut(&mut self) -> Result<&mut EmployeeSavingsDetails, DomainError> {
        match &mut self.kind {
            AccountKind::EmployeeSavings(details) => Ok(details),
            _ => Err(DomainError::WrongAccountKind),
        }
    }
}

fn empty_savings(product: SavingsProduct, today: NaiveDate) -> AccountKind {
    let (rate, ceiling) = defaults::savings_terms(product);
    AccountKind::Savings(
        SavingsDetails::new(product, Money::ZERO, rate, ceiling, today)
            .expect("les valeurs par défaut d'un livret sont valides"),
    )
}

fn empty_securities(product: SecuritiesProduct) -> AccountKind {
    AccountKind::Securities(
        SecuritiesDetails::new(product, Money::ZERO, Vec::new())
            .expect("un solde espèces nul est valide"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{CurrencyCode, QuoteSource};
    use rust_decimal_macros::dec;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()
    }

    fn open(account_type: AccountType) -> Account {
        Account::open(
            AccountId::generate(),
            Label::new(account_type.label()).unwrap(),
            defaults::color(account_type),
            account_type,
            today(),
        )
    }

    fn asset(source: QuoteSource, symbol: &str) -> AssetRef {
        AssetRef::new(source, symbol, "", CurrencyCode::eur()).unwrap()
    }

    #[test]
    fn opened_pea_is_empty_with_default_rates() {
        let pea = open(AccountType::Pea);

        assert_eq!(pea.account_type(), AccountType::Pea);
        assert_eq!(pea.monthly_contribution(), Money::ZERO);
        assert_eq!(
            pea.scenario_rates(),
            &defaults::scenario_rates(AccountType::Pea, None)
        );
        assert!(matches!(pea.kind(), AccountKind::Securities(d) if d.positions().is_empty()));
    }

    #[test]
    fn opened_livret_a_uses_regulated_defaults() {
        let livret = open(AccountType::LivretA);
        let AccountKind::Savings(details) = livret.kind() else {
            panic!("livret attendu")
        };

        assert_eq!(details.rate(), defaults::livret_a_rate());
        assert_eq!(details.deposit_ceiling(), defaults::livret_a_ceiling());
        assert!(
            livret
                .scenario_rates()
                .is_uniform(defaults::livret_a_rate())
        );
    }

    #[test]
    fn updates_savings_balance_with_date() {
        let mut livret = open(AccountType::LivretA);
        let on = NaiveDate::from_ymd_opt(2026, 10, 21).unwrap();

        livret
            .update_savings_balance(Money::new(dec!(10640)), on)
            .unwrap();

        let AccountKind::Savings(details) = livret.kind() else {
            panic!("livret attendu")
        };
        assert_eq!(details.balance(), Money::new(dec!(10640)));
        assert_eq!(details.balance_updated_on(), on);
    }

    #[test]
    fn scenario_rates_follow_new_savings_rate_unless_customized() {
        let new_rate = Rate::from_percent(dec!(2)).unwrap();

        let mut following = open(AccountType::Ldds);
        following
            .set_savings_terms(new_rate, defaults::ldds_ceiling())
            .unwrap();
        assert!(following.scenario_rates().is_uniform(new_rate));

        let mut customized = open(AccountType::Ldds);
        let custom = ScenarioRates::uniform(Rate::from_percent(dec!(1)).unwrap());
        customized.set_scenario_rates(custom);
        customized
            .set_savings_terms(new_rate, defaults::ldds_ceiling())
            .unwrap();
        assert_eq!(customized.scenario_rates(), &custom);
    }

    #[test]
    fn upsert_position_replaces_same_asset() {
        let mut crypto = open(AccountType::Crypto);
        let btc = asset(QuoteSource::CoinGecko, "bitcoin");

        crypto
            .upsert_position(Position::new(btc.clone(), dec!(0.1), None).unwrap())
            .unwrap();
        crypto
            .upsert_position(Position::new(btc.clone(), dec!(0.15), None).unwrap())
            .unwrap();
        crypto
            .upsert_position(
                Position::new(asset(QuoteSource::CoinGecko, "ethereum"), dec!(2), None).unwrap(),
            )
            .unwrap();

        let AccountKind::Securities(details) = crypto.kind() else {
            panic!("crypto attendu")
        };
        assert_eq!(details.positions().len(), 2);
        assert_eq!(details.positions()[0].quantity(), dec!(0.15));
    }

    #[test]
    fn remove_unknown_position_fails() {
        let mut pea = open(AccountType::Pea);
        assert_eq!(
            pea.remove_position(&asset(QuoteSource::TwelveData, "CW8")),
            Err(DomainError::UnknownPosition)
        );
    }

    #[test]
    fn operations_on_wrong_kind_are_rejected() {
        let mut livret = open(AccountType::LivretA);
        let position = Position::new(asset(QuoteSource::TwelveData, "CW8"), dec!(1), None).unwrap();

        assert_eq!(
            livret.upsert_position(position),
            Err(DomainError::WrongAccountKind)
        );
        assert_eq!(
            livret.set_cash(Money::ZERO),
            Err(DomainError::WrongAccountKind)
        );
        assert_eq!(
            open(AccountType::Pea).update_savings_balance(Money::ZERO, today()),
            Err(DomainError::WrongAccountKind)
        );
    }

    #[test]
    fn records_nav_on_existing_fund_only() {
        let mut pei = open(AccountType::Pei);
        let anchor = NavAnchor::new(Money::new(dec!(25)), today(), None).unwrap();
        let fund = PeiFund::new(
            FundId::generate(),
            Label::new("Monétaire").unwrap(),
            dec!(10),
            None,
            anchor.clone(),
        )
        .unwrap();
        let fund_id = fund.id();
        pei.upsert_fund(fund).unwrap();

        let later = NavAnchor::new(
            Money::new(dec!(25.10)),
            NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(),
            None,
        )
        .unwrap();
        pei.record_pei_nav(fund_id, later.clone()).unwrap();

        let AccountKind::EmployeeSavings(details) = pei.kind() else {
            panic!("PEI attendu")
        };
        assert_eq!(details.funds()[0].anchor(), &later);
        assert_eq!(
            pei.record_pei_nav(FundId::generate(), anchor),
            Err(DomainError::UnknownFund)
        );
    }

    #[test]
    fn monthly_contribution_cannot_be_negative() {
        let mut pea = open(AccountType::Pea);
        assert_eq!(
            pea.set_monthly_contribution(Money::new(dec!(-1))),
            Err(DomainError::NegativeAmount)
        );
        pea.set_monthly_contribution(Money::new(dec!(300))).unwrap();
        assert_eq!(pea.monthly_contribution(), Money::new(dec!(300)));
    }

    #[test]
    fn archive_and_unarchive() {
        let mut crypto = open(AccountType::Crypto);
        crypto.archive();
        assert!(crypto.is_archived());
        crypto.unarchive();
        assert!(!crypto.is_archived());
    }
}
