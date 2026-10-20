//! Comptes : création, soldes, positions, fonds PEI et rendements de projection.

use std::sync::Arc;

use lc_domain::account::{
    Account, AccountKind, AccountType, AssetRef, NavAnchor, PeiFund, Position, QuoteSource,
    ScenarioRates,
};
use lc_domain::{AccountId, Color, Decimal, DomainError, FundId, Label, Money, Rate, defaults};

use lc_domain::valuation::{AccountValuation, value_account};

use crate::ports::{AccountRepository, Clock};
use crate::pricing::{PricingIssue, PricingService, tracked_assets};
use crate::{AppError, AppResult};

/// Comptes et leur valorisation, avec les éventuels problèmes de cours.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuedAccounts {
    pub accounts: Vec<(Account, AccountValuation)>,
    pub pricing_issues: Vec<PricingIssue>,
}

/// Actifs à coter pour valoriser ces comptes, archivés compris (la fiche d'un compte archivé
/// reste consultable).
fn tracked_assets_of_all(accounts: &[Account]) -> Vec<AssetRef> {
    let mut unarchived = accounts.to_vec();
    for account in &mut unarchived {
        account.unarchive();
    }
    tracked_assets(&unarchived)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAccount {
    pub name: String,
    pub account_type: AccountType,
    /// Couleur `#RRGGBB` ; la couleur par défaut du type si absente.
    pub color: Option<String>,
    pub monthly_contribution: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountUpdate {
    pub name: String,
    pub color: String,
    pub monthly_contribution: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundInput {
    pub name: String,
    pub units: Decimal,
    pub reference: Option<AssetRef>,
}

pub struct AccountUseCases {
    repo: Arc<dyn AccountRepository>,
    pricing: Arc<PricingService>,
    clock: Arc<dyn Clock>,
}

impl AccountUseCases {
    pub fn new(
        repo: Arc<dyn AccountRepository>,
        pricing: Arc<PricingService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            pricing,
            clock,
        }
    }

    pub fn list_accounts(&self, include_archived: bool) -> AppResult<Vec<Account>> {
        let mut accounts = self.repo.list()?;
        accounts.retain(|account| include_archived || !account.is_archived());
        Ok(accounts)
    }

    pub fn get_account(&self, id: AccountId) -> AppResult<Account> {
        self.repo.get(id)?.ok_or(AppError::NotFound("Compte"))
    }

    /// Comptes valorisés avec les cours disponibles (cache de 5 minutes respecté).
    pub async fn valued_accounts(&self, include_archived: bool) -> AppResult<ValuedAccounts> {
        let accounts = self.list_accounts(include_archived)?;
        let refresh = self
            .pricing
            .prices_for(&tracked_assets_of_all(&accounts), false)
            .await?;
        Ok(ValuedAccounts {
            accounts: accounts
                .into_iter()
                .map(|account| {
                    let valuation = value_account(&account, &refresh.prices);
                    (account, valuation)
                })
                .collect(),
            pricing_issues: refresh.issues,
        })
    }

    /// Un compte et sa valorisation détaillée (positions, fonds).
    pub async fn valued_account(
        &self,
        id: AccountId,
    ) -> AppResult<(Account, AccountValuation, Vec<PricingIssue>)> {
        let account = self.get_account(id)?;
        let refresh = self
            .pricing
            .prices_for(
                &tracked_assets_of_all(std::slice::from_ref(&account)),
                false,
            )
            .await?;
        let valuation = value_account(&account, &refresh.prices);
        Ok((account, valuation, refresh.issues))
    }

    pub fn create_account(&self, input: NewAccount) -> AppResult<AccountId> {
        let color = match input.color.as_deref() {
            Some(hex) => Color::new(hex)?,
            None => defaults::color(input.account_type),
        };
        let mut account = Account::open(
            AccountId::generate(),
            Label::new(&input.name)?,
            color,
            input.account_type,
            self.clock.today(),
        );
        account.set_monthly_contribution(input.monthly_contribution)?;
        self.repo.save(&account)?;
        Ok(account.id())
    }

    pub fn update_account(&self, id: AccountId, input: AccountUpdate) -> AppResult<()> {
        let name = Label::new(&input.name)?;
        let color = Color::new(&input.color)?;
        self.modify(id, |account| {
            account.rename(name);
            account.set_color(color);
            account.set_monthly_contribution(input.monthly_contribution)
        })
    }

    /// Clôture : le compte sort du patrimoine courant, l'historique est conservé.
    pub fn archive_account(&self, id: AccountId) -> AppResult<()> {
        self.modify(id, |account| {
            account.archive();
            Ok(())
        })
    }

    pub fn unarchive_account(&self, id: AccountId) -> AppResult<()> {
        self.modify(id, |account| {
            account.unarchive();
            Ok(())
        })
    }

    /// Mise à jour du solde d'un livret, datée du jour.
    pub fn update_savings_balance(&self, id: AccountId, balance: Money) -> AppResult<()> {
        let today = self.clock.today();
        self.modify(id, |account| account.update_savings_balance(balance, today))
    }

    pub fn set_savings_terms(
        &self,
        id: AccountId,
        rate: Rate,
        deposit_ceiling: Money,
    ) -> AppResult<()> {
        self.modify(id, |account| {
            account.set_savings_terms(rate, deposit_ceiling)
        })
    }

    /// Rendements de projection d'un compte (pessimiste, neutre, optimiste).
    pub fn set_scenario_rates(&self, id: AccountId, rates: ScenarioRates) -> AppResult<()> {
        self.modify(id, |account| {
            account.set_scenario_rates(rates);
            Ok(())
        })
    }

    pub fn set_cash(&self, id: AccountId, cash: Money) -> AppResult<()> {
        self.modify(id, |account| account.set_cash(cash))
    }

    pub fn upsert_position(
        &self,
        id: AccountId,
        asset: AssetRef,
        quantity: Decimal,
        unit_cost: Option<Money>,
    ) -> AppResult<()> {
        let position = Position::new(asset, quantity, unit_cost)?;
        self.modify(id, |account| account.upsert_position(position))
    }

    pub fn remove_position(&self, id: AccountId, asset: &AssetRef) -> AppResult<()> {
        self.modify(id, |account| account.remove_position(asset))
    }

    pub async fn search_assets(
        &self,
        source: QuoteSource,
        query: &str,
    ) -> AppResult<Vec<AssetRef>> {
        self.pricing.search(source, query).await
    }

    /// État du compte auprès d'une source de cours (offre de la clé, crédits consommés).
    pub async fn price_source_status(&self, source: QuoteSource) -> AppResult<Option<String>> {
        self.pricing.source_status(source).await
    }

    /// Ajoute un fonds au PEI avec sa valeur de part du jour ; le cours de son actif de
    /// référence est relevé maintenant et sert d'ancrage aux estimations.
    pub async fn create_pei_fund(
        &self,
        account_id: AccountId,
        input: FundInput,
        nav: Money,
    ) -> AppResult<FundId> {
        let reference_price = self.reference_price_today(input.reference.as_ref()).await?;
        let anchor = NavAnchor::new(nav, self.clock.today(), reference_price)?;
        let fund = PeiFund::new(
            FundId::generate(),
            Label::new(&input.name)?,
            input.units,
            input.reference,
            anchor,
        )?;
        let fund_id = fund.id();
        self.modify(account_id, |account| account.upsert_fund(fund))?;
        Ok(fund_id)
    }

    /// Modifie le nom, le nombre de parts ou l'actif de référence d'un fonds.
    pub fn update_pei_fund(
        &self,
        account_id: AccountId,
        fund_id: FundId,
        input: FundInput,
    ) -> AppResult<()> {
        let name = Label::new(&input.name)?;
        self.modify(account_id, |account| {
            let mut fund = find_fund(account, fund_id)?;
            fund.rename(name);
            fund.set_units(input.units)?;
            fund.set_reference(input.reference);
            account.upsert_fund(fund)
        })
    }

    /// Saisie de la valeur de part réelle : elle remplace l'estimation et devient le nouvel
    /// ancrage, avec le cours de référence du jour.
    pub async fn record_pei_nav(
        &self,
        account_id: AccountId,
        fund_id: FundId,
        nav: Money,
    ) -> AppResult<()> {
        let account = self.get_account(account_id)?;
        let fund = find_fund(&account, fund_id)?;
        let reference_price = self.reference_price_today(fund.reference()).await?;
        let anchor = NavAnchor::new(nav, self.clock.today(), reference_price)?;
        self.modify(account_id, |account| {
            account.record_pei_nav(fund_id, anchor)
        })
    }

    pub fn remove_pei_fund(&self, account_id: AccountId, fund_id: FundId) -> AppResult<()> {
        self.modify(account_id, |account| account.remove_fund(fund_id))
    }

    /// Cours du jour de l'actif de référence (un cours en cache d'un autre jour ne convient pas
    /// comme ancrage).
    async fn reference_price_today(
        &self,
        reference: Option<&AssetRef>,
    ) -> AppResult<Option<Money>> {
        let Some(reference) = reference else {
            return Ok(None);
        };
        let refresh = self
            .pricing
            .prices_for(std::slice::from_ref(reference), false)
            .await?;
        let today = self.clock.today();
        Ok(refresh
            .prices
            .get(reference)
            .filter(|quote| quote.as_of().date_naive() == today)
            .map(|quote| quote.price()))
    }

    fn modify(
        &self,
        id: AccountId,
        change: impl FnOnce(&mut Account) -> Result<(), DomainError>,
    ) -> AppResult<()> {
        let mut account = self.get_account(id)?;
        change(&mut account)?;
        Ok(self.repo.save(&account)?)
    }
}

fn find_fund(account: &Account, fund_id: FundId) -> Result<PeiFund, DomainError> {
    match account.kind() {
        AccountKind::EmployeeSavings(details) => details
            .funds()
            .iter()
            .find(|fund| fund.id() == fund_id)
            .cloned()
            .ok_or(DomainError::UnknownFund),
        _ => Err(DomainError::WrongAccountKind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestEnv, asset};
    use lc_domain::valuation::{NavKind, PriceBook, value_account};
    use pollster::block_on;
    use rust_decimal_macros::dec;

    fn new_account(name: &str, account_type: AccountType) -> NewAccount {
        NewAccount {
            name: name.into(),
            account_type,
            color: None,
            monthly_contribution: Money::ZERO,
        }
    }

    // Spec account-management : « Création d'un compte »
    #[test]
    fn creates_account_with_type_defaults() {
        let env = TestEnv::new();
        let accounts = env.accounts();

        let id = accounts
            .create_account(new_account("PEA Bourso", AccountType::Pea))
            .unwrap();

        let pea = accounts.get_account(id).unwrap();
        assert_eq!(pea.name().as_str(), "PEA Bourso");
        assert_eq!(pea.color(), &defaults::color(AccountType::Pea));
        assert_eq!(
            value_account(&pea, &PriceBook::default()).value,
            Money::ZERO
        );
    }

    #[test]
    fn valued_accounts_include_positions_at_market_price() {
        let env = TestEnv::new();
        env.coingecko.set_price("bitcoin", dec!(60000));
        let accounts = env.accounts();
        let wallet = accounts
            .create_account(new_account("Crypto", AccountType::Crypto))
            .unwrap();
        accounts
            .upsert_position(
                wallet,
                asset(QuoteSource::CoinGecko, "bitcoin"),
                dec!(0.15),
                None,
            )
            .unwrap();
        let closed = accounts
            .create_account(new_account("Ancien", AccountType::Crypto))
            .unwrap();
        accounts
            .upsert_position(
                closed,
                asset(QuoteSource::CoinGecko, "bitcoin"),
                dec!(1),
                None,
            )
            .unwrap();
        accounts.archive_account(closed).unwrap();

        let all = block_on(accounts.valued_accounts(true)).unwrap();
        let (_, detail, issues) = block_on(accounts.valued_account(closed)).unwrap();

        assert_eq!(all.accounts.len(), 2);
        assert_eq!(all.accounts[0].1.value, Money::new(dec!(9000)));
        assert_eq!(
            detail.value,
            Money::new(dec!(60000)),
            "un compte archivé reste valorisé dans sa fiche"
        );
        assert!(issues.is_empty());
        assert_eq!(
            block_on(accounts.valued_accounts(false))
                .unwrap()
                .accounts
                .len(),
            1
        );
    }

    #[test]
    fn several_accounts_of_same_type_are_allowed() {
        let env = TestEnv::new();
        let accounts = env.accounts();
        accounts
            .create_account(new_account("Ledger", AccountType::Crypto))
            .unwrap();
        accounts
            .create_account(new_account("Exchange", AccountType::Crypto))
            .unwrap();
        assert_eq!(accounts.list_accounts(false).unwrap().len(), 2);
    }

    // Spec account-management : « Mise à jour du solde »
    #[test]
    fn savings_balance_update_is_dated_today() {
        let env = TestEnv::new();
        let accounts = env.accounts();
        let id = accounts
            .create_account(new_account("Livret A", AccountType::LivretA))
            .unwrap();

        accounts
            .update_savings_balance(id, Money::new(dec!(10640)))
            .unwrap();

        let AccountKind::Savings(details) = accounts.get_account(id).unwrap().kind().clone() else {
            panic!()
        };
        assert_eq!(details.balance(), Money::new(dec!(10640)));
        assert_eq!(details.balance_updated_on(), env.clock.today());
    }

    #[test]
    fn positions_and_wrong_kind() {
        let env = TestEnv::new();
        let accounts = env.accounts();
        let wallet = accounts
            .create_account(new_account("Crypto", AccountType::Crypto))
            .unwrap();
        let livret = accounts
            .create_account(new_account("LDDS", AccountType::Ldds))
            .unwrap();
        let btc = asset(QuoteSource::CoinGecko, "bitcoin");

        accounts
            .upsert_position(wallet, btc.clone(), dec!(0.15), None)
            .unwrap();

        assert_eq!(
            accounts.upsert_position(livret, btc, dec!(1), None),
            Err(AppError::Domain(DomainError::WrongAccountKind))
        );
        assert_eq!(
            accounts.get_account(AccountId::generate()),
            Err(AppError::NotFound("Compte"))
        );
    }

    // Spec account-management : « Clôture d'un compte »
    #[test]
    fn archived_accounts_are_hidden_by_default() {
        let env = TestEnv::new();
        let accounts = env.accounts();
        let id = accounts
            .create_account(new_account("Crypto", AccountType::Crypto))
            .unwrap();

        accounts.archive_account(id).unwrap();

        assert!(accounts.list_accounts(false).unwrap().is_empty());
        assert_eq!(accounts.list_accounts(true).unwrap().len(), 1);
    }

    // Spec account-management : « Fonds rattaché à un indice » + spec market-pricing « Recalage »
    #[test]
    fn pei_fund_anchor_captures_reference_price_of_the_day() {
        let env = TestEnv::new();
        env.twelve_data.set_price("IWDA", dec!(100));
        let accounts = env.accounts();
        let pei = accounts
            .create_account(new_account("PEI", AccountType::Pei))
            .unwrap();
        let input = FundInput {
            name: "Amundi Actions Monde".into(),
            units: dec!(120),
            reference: Some(asset(QuoteSource::TwelveData, "IWDA")),
        };

        let fund_id = block_on(accounts.create_pei_fund(pei, input, Money::new(dec!(25)))).unwrap();

        let account = accounts.get_account(pei).unwrap();
        let fund = find_fund(&account, fund_id).unwrap();
        assert_eq!(fund.anchor().reference_price(), Some(Money::new(dec!(100))));
        assert_eq!(fund.anchor().date(), env.clock.today());

        // Le lendemain, l'indice monte de 4 % : la part est estimée à 26 €.
        env.clock.advance(chrono::Duration::days(1));
        env.twelve_data.set_price("IWDA", dec!(104));
        let refresh = block_on(
            env.pricing()
                .prices_for(&[asset(QuoteSource::TwelveData, "IWDA")], false),
        )
        .unwrap();
        let valuation = value_account(&account, &refresh.prices);
        assert_eq!(valuation.funds[0].nav, Money::new(dec!(26)));
        assert_eq!(valuation.funds[0].nav_kind, NavKind::Estimated);

        // Recalage avec la valeur réelle : elle devient l'ancrage, avec le cours du jour.
        block_on(accounts.record_pei_nav(pei, fund_id, Money::new(dec!(25.80)))).unwrap();
        let recalibrated = find_fund(&accounts.get_account(pei).unwrap(), fund_id).unwrap();
        assert_eq!(recalibrated.anchor().nav(), Money::new(dec!(25.80)));
        assert_eq!(
            recalibrated.anchor().reference_price(),
            Some(Money::new(dec!(104)))
        );
    }

    #[test]
    fn stale_cached_reference_price_is_not_used_as_anchor() {
        let env = TestEnv::new();
        env.twelve_data.set_price("IWDA", dec!(100));
        let accounts = env.accounts();
        let pei = accounts
            .create_account(new_account("PEI", AccountType::Pei))
            .unwrap();
        let iwda = asset(QuoteSource::TwelveData, "IWDA");
        block_on(env.pricing().prices_for(std::slice::from_ref(&iwda), false)).unwrap();

        env.clock.advance(chrono::Duration::days(2));
        env.twelve_data
            .fail_with(crate::ports::PriceError::Unavailable("hors ligne".into()));
        let input = FundInput {
            name: "Monde".into(),
            units: dec!(10),
            reference: Some(iwda),
        };
        let fund_id = block_on(accounts.create_pei_fund(pei, input, Money::new(dec!(25)))).unwrap();

        let fund = find_fund(&accounts.get_account(pei).unwrap(), fund_id).unwrap();
        assert_eq!(fund.anchor().reference_price(), None);
    }

    #[test]
    fn update_fund_and_remove() {
        let env = TestEnv::new();
        let accounts = env.accounts();
        let pei = accounts
            .create_account(new_account("PEI", AccountType::Pei))
            .unwrap();
        let input = FundInput {
            name: "Monétaire".into(),
            units: dec!(50),
            reference: None,
        };
        let fund_id = block_on(accounts.create_pei_fund(pei, input, Money::new(dec!(10)))).unwrap();

        accounts
            .update_pei_fund(
                pei,
                fund_id,
                FundInput {
                    name: "Monétaire ISR".into(),
                    units: dec!(55),
                    reference: None,
                },
            )
            .unwrap();
        let fund = find_fund(&accounts.get_account(pei).unwrap(), fund_id).unwrap();
        assert_eq!(
            (fund.name().as_str(), fund.units()),
            ("Monétaire ISR", dec!(55))
        );

        accounts.remove_pei_fund(pei, fund_id).unwrap();
        assert_eq!(
            accounts.remove_pei_fund(pei, fund_id),
            Err(AppError::Domain(DomainError::UnknownFund))
        );
    }

    #[test]
    fn scenario_rates_are_saved() {
        let env = TestEnv::new();
        let accounts = env.accounts();
        let id = accounts
            .create_account(new_account("Crypto", AccountType::Crypto))
            .unwrap();
        let mut rates = *accounts.get_account(id).unwrap().scenario_rates();
        rates.optimistic = Rate::from_percent(dec!(40)).unwrap();

        accounts.set_scenario_rates(id, rates).unwrap();

        assert_eq!(
            accounts
                .get_account(id)
                .unwrap()
                .scenario_rates()
                .optimistic
                .as_percent(),
            dec!(40)
        );
    }
}
