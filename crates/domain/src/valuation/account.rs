use rust_decimal::Decimal;

use super::{NavKind, PriceBook, Provenance, estimate_nav};
use crate::account::{Account, AccountKind, AssetRef, PeiFund, Position};
use crate::ledger::{Ledger, Movement};
use crate::{AccountId, FundId, Label, Money};

/// Valeur d'une position de PEA ou de portefeuille crypto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionValuation {
    pub asset: AssetRef,
    pub quantity: Decimal,
    /// Prix unitaire utilisé (cours, ou prix de revient à défaut).
    pub unit_price: Option<Money>,
    pub value: Money,
    pub provenance: Provenance,
    pub cost_basis: Option<Money>,
    /// Plus ou moins-value latente, connue seulement si un cours et un prix de revient existent.
    pub unrealized_gain: Option<Money>,
    /// Plus ou moins-value latente en pourcentage du prix de revient.
    pub unrealized_gain_percent: Option<Decimal>,
}

/// Valeur d'un fonds de PEI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundValuation {
    pub fund_id: FundId,
    pub name: Label,
    pub units: Decimal,
    pub nav: Money,
    pub nav_kind: NavKind,
    pub value: Money,
}

/// Valeur d'un compte et de ses composantes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountValuation {
    pub account_id: AccountId,
    pub value: Money,
    /// Provenance la moins fiable parmi les composantes du compte.
    pub provenance: Provenance,
    pub positions: Vec<PositionValuation>,
    pub funds: Vec<FundValuation>,
    /// Compte courant : mouvements comptés depuis le solde de référence.
    pub movements: Vec<Movement>,
}

/// Valorise un compte avec les cours disponibles et le grand livre : les comptes à solde (compte
/// courant, livrets, assurance-vie, espèces d'un compte-titres) ajoutent à leur dernier solde
/// saisi les mouvements postérieurs. L'épargne salariale reste valorisée par ses fonds.
pub fn value_account(account: &Account, prices: &PriceBook, ledger: &Ledger) -> AccountValuation {
    let movements = ledger.movements(account.id());
    match account.kind() {
        AccountKind::Savings(details) => {
            let statement = details.reference().statement(movements);
            AccountValuation {
                account_id: account.id(),
                value: statement.balance,
                provenance: Provenance::Manual,
                positions: Vec::new(),
                funds: Vec::new(),
                movements: statement.movements,
            }
        }
        AccountKind::Checking(details) => {
            let statement = details.statement(movements);
            AccountValuation {
                account_id: account.id(),
                value: statement.balance,
                provenance: Provenance::Manual,
                positions: Vec::new(),
                funds: Vec::new(),
                movements: statement.movements,
            }
        }
        AccountKind::Securities(details) => {
            let positions: Vec<_> = details
                .positions()
                .iter()
                .map(|position| value_position(position, prices))
                .collect();
            let cash = details.cash_reference().statement(movements);
            AccountValuation {
                account_id: account.id(),
                value: cash.balance + positions.iter().map(|p| p.value).sum::<Money>(),
                provenance: combined(positions.iter().map(|p| p.provenance)),
                positions,
                funds: Vec::new(),
                movements: cash.movements,
            }
        }
        AccountKind::EmployeeSavings(details) => {
            let funds: Vec<_> = details
                .funds()
                .iter()
                .map(|fund| value_fund(fund, prices))
                .collect();
            AccountValuation {
                account_id: account.id(),
                value: funds.iter().map(|f| f.value).sum(),
                provenance: combined(funds.iter().map(|f| match f.nav_kind {
                    NavKind::Real => Provenance::Manual,
                    NavKind::Estimated => Provenance::Estimated,
                })),
                positions: Vec::new(),
                funds,
                movements: Vec::new(),
            }
        }
    }
}
fn value_position(position: &Position, prices: &PriceBook) -> PositionValuation {
    let cost_basis = position.cost_basis();
    let (unit_price, provenance) = match (
        prices.get(position.asset()),
        prices.provenance_of(position.asset()),
    ) {
        (Some(quote), Some(provenance)) => (Some(quote.price()), provenance),
        _ => match position.unit_cost() {
            Some(cost) => (Some(cost), Provenance::CostBasis),
            None => (None, Provenance::Unavailable),
        },
    };
    let value = unit_price.map_or(Money::ZERO, |price| price.times(position.quantity()));
    let has_market_price = !matches!(provenance, Provenance::CostBasis | Provenance::Unavailable);
    let unrealized_gain = cost_basis
        .filter(|_| has_market_price)
        .map(|cost| value - cost);
    let unrealized_gain_percent = unrealized_gain
        .zip(cost_basis)
        .filter(|(_, cost)| cost.is_positive())
        .map(|(gain, cost)| gain.amount() / cost.amount() * Decimal::ONE_HUNDRED);

    PositionValuation {
        asset: position.asset().clone(),
        quantity: position.quantity(),
        unit_price,
        value,
        provenance,
        cost_basis,
        unrealized_gain,
        unrealized_gain_percent,
    }
}

fn value_fund(fund: &PeiFund, prices: &PriceBook) -> FundValuation {
    let quote = fund.reference().and_then(|reference| prices.get(reference));
    let estimate = estimate_nav(fund.anchor(), quote);
    FundValuation {
        fund_id: fund.id(),
        name: fund.name().clone(),
        units: fund.units(),
        nav: estimate.nav,
        nav_kind: estimate.kind,
        value: estimate.nav.times(fund.units()),
    }
}

/// Provenance d'un compte : la moins fiable de ses composantes ; un compte sans composante
/// (seulement des espèces) est une valeur saisie.
fn combined(provenances: impl Iterator<Item = Provenance>) -> Provenance {
    provenances
        .reduce(Provenance::least_reliable)
        .unwrap_or(Provenance::Manual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{AccountType, CurrencyCode, NavAnchor, QuoteSource};
    use crate::valuation::{Quote, QuoteFreshness};
    use crate::{Label, defaults};
    use chrono::{DateTime, NaiveDate, Utc};
    use rust_decimal_macros::dec;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()
    }

    fn now() -> DateTime<Utc> {
        today().and_hms_opt(9, 42, 0).unwrap().and_utc()
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

    fn live(asset: AssetRef, price: Money) -> Quote {
        Quote::new(asset, price, now(), QuoteFreshness::Live)
    }

    #[test]
    fn savings_account_is_valued_at_its_manual_balance() {
        let mut livret = open(AccountType::LivretA);
        livret
            .update_savings_balance(Money::new(dec!(10640)), today(), now())
            .unwrap();

        let valuation = value_account(&livret, &PriceBook::default(), &Ledger::default());

        assert_eq!(valuation.value, Money::new(dec!(10640)));
        assert_eq!(valuation.provenance, Provenance::Manual);
    }

    // Spec account-management : « Ajout d'une position crypto » et « Plus-value latente »
    #[test]
    fn position_value_and_unrealized_gain() {
        let btc = asset(QuoteSource::CoinGecko, "bitcoin");
        let mut wallet = open(AccountType::Crypto);
        wallet
            .upsert_position(
                Position::new(btc.clone(), dec!(0.15), Some(Money::new(dec!(40000)))).unwrap(),
            )
            .unwrap();
        let prices = PriceBook::new([live(btc, Money::new(dec!(60000)))]);

        let valuation = value_account(&wallet, &prices, &Ledger::default());
        let position = &valuation.positions[0];

        assert_eq!(valuation.value, Money::new(dec!(9000)));
        assert_eq!(valuation.provenance, Provenance::Live);
        assert_eq!(position.unrealized_gain, Some(Money::new(dec!(3000))));
        assert_eq!(position.unrealized_gain_percent, Some(dec!(50)));
    }

    // Spec market-pricing : « Clé absente » (repli sur le prix de revient)
    #[test]
    fn falls_back_to_cost_basis_without_quote() {
        let cw8 = asset(QuoteSource::TwelveData, "CW8");
        let mut pea = open(AccountType::Pea);
        pea.set_cash(Money::new(dec!(110)), today(), now()).unwrap();
        pea.upsert_position(Position::new(cw8, dec!(42), Some(Money::new(dec!(340)))).unwrap())
            .unwrap();

        let valuation = value_account(&pea, &PriceBook::default(), &Ledger::default());

        assert_eq!(valuation.value, Money::new(dec!(14390)));
        assert_eq!(valuation.provenance, Provenance::CostBasis);
        assert_eq!(valuation.positions[0].unrealized_gain, None);
    }

    #[test]
    fn position_without_quote_nor_cost_is_unavailable() {
        let mut wallet = open(AccountType::Crypto);
        wallet
            .upsert_position(
                Position::new(asset(QuoteSource::CoinGecko, "solana"), dec!(3), None).unwrap(),
            )
            .unwrap();

        let valuation = value_account(&wallet, &PriceBook::default(), &Ledger::default());

        assert_eq!(valuation.value, Money::ZERO);
        assert_eq!(valuation.provenance, Provenance::Unavailable);
    }

    #[test]
    fn cached_quote_marks_account_as_cached() {
        let eth = asset(QuoteSource::CoinGecko, "ethereum");
        let btc = asset(QuoteSource::CoinGecko, "bitcoin");
        let mut wallet = open(AccountType::Crypto);
        wallet
            .upsert_position(Position::new(eth.clone(), dec!(2), None).unwrap())
            .unwrap();
        wallet
            .upsert_position(Position::new(btc.clone(), dec!(0.1), None).unwrap())
            .unwrap();
        let yesterday = now() - chrono::Duration::days(1);
        let prices = PriceBook::new([
            live(btc, Money::new(dec!(60000))),
            Quote::new(
                eth,
                Money::new(dec!(2500)),
                yesterday,
                QuoteFreshness::Cached,
            ),
        ]);

        let valuation = value_account(&wallet, &prices, &Ledger::default());

        assert_eq!(valuation.value, Money::new(dec!(11000)));
        assert_eq!(
            valuation.provenance,
            Provenance::Cached { as_of: yesterday }
        );
    }

    #[test]
    fn pei_funds_are_estimated_through_reference() {
        let iwda = asset(QuoteSource::TwelveData, "IWDA");
        let mut pei = open(AccountType::Pei);
        let anchor_day = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let world = PeiFund::new(
            FundId::generate(),
            Label::new("Amundi Actions Monde").unwrap(),
            dec!(120),
            Some(iwda.clone()),
            NavAnchor::new(
                Money::new(dec!(25)),
                anchor_day,
                Some(Money::new(dec!(100))),
            )
            .unwrap(),
        )
        .unwrap();
        let money_market = PeiFund::new(
            FundId::generate(),
            Label::new("Monétaire").unwrap(),
            dec!(50),
            None,
            NavAnchor::new(Money::new(dec!(10)), anchor_day, None).unwrap(),
        )
        .unwrap();
        pei.upsert_fund(world).unwrap();
        pei.upsert_fund(money_market).unwrap();
        let prices = PriceBook::new([live(iwda, Money::new(dec!(104)))]);

        let valuation = value_account(&pei, &prices, &Ledger::default());

        assert_eq!(valuation.funds[0].nav, Money::new(dec!(26)));
        assert_eq!(valuation.funds[0].value, Money::new(dec!(3120)));
        assert_eq!(valuation.funds[1].nav_kind, NavKind::Real);
        assert_eq!(valuation.value, Money::new(dec!(3620)));
        assert_eq!(valuation.provenance, Provenance::Estimated);
    }
}
