//! Cours des actifs : routage vers la bonne source, cache de 5 minutes, repli hors ligne.

use std::sync::Arc;

use chrono::Duration;
use lc_domain::account::{Account, AccountKind, AssetRef, QuoteSource};
use lc_domain::ledger::Ledger;
use lc_domain::valuation::{AccountValuation, PriceBook, QuoteFreshness, value_account};

use crate::ports::{Clock, PriceCache, PriceError, PriceProvider};
use crate::{AppError, AppResult};

/// Problème rencontré lors d'un rafraîchissement : les valeurs restent affichées (cache ou prix
/// de revient), l'UI signale le problème discrètement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PricingIssue {
    RateLimited(QuoteSource),
    MissingApiKey(QuoteSource),
    Unavailable(QuoteSource),
    /// Actifs que la source ne cote pas (ex. place européenne hors de l'offre gratuite).
    NotCovered {
        source: QuoteSource,
        symbols: Vec<String>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PriceRefresh {
    pub prices: PriceBook,
    pub issues: Vec<PricingIssue>,
}

pub struct PricingService {
    providers: Vec<Arc<dyn PriceProvider>>,
    cache: Arc<dyn PriceCache>,
    clock: Arc<dyn Clock>,
}

impl PricingService {
    /// Un cours plus récent que cette durée est réutilisé sans appel réseau.
    pub const CACHE_TTL_SECONDS: i64 = 5 * 60;

    pub fn new(
        providers: Vec<Arc<dyn PriceProvider>>,
        cache: Arc<dyn PriceCache>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            providers,
            cache,
            clock,
        }
    }

    /// Cours des actifs demandés. Sans `force_refresh`, un cours en cache de moins de 5 minutes
    /// est réutilisé. Les actifs d'une même source sont demandés en une seule requête ; en cas
    /// d'échec, le dernier cours en cache est utilisé (marqué « en cache »).
    pub async fn prices_for(
        &self,
        assets: &[AssetRef],
        force_refresh: bool,
    ) -> AppResult<PriceRefresh> {
        let now = self.clock.now();
        let ttl = Duration::seconds(Self::CACHE_TTL_SECONDS);
        let mut refresh = PriceRefresh::default();
        let mut to_fetch: Vec<AssetRef> = Vec::new();

        for asset in assets {
            if to_fetch.iter().any(|a| a.is_same_asset(asset))
                || refresh.prices.get(asset).is_some()
            {
                continue;
            }
            match self.cache.get(asset)? {
                Some(quote) if !force_refresh && now - quote.as_of() < ttl => {
                    refresh
                        .prices
                        .insert(quote.with_freshness(QuoteFreshness::Live));
                }
                _ => to_fetch.push(asset.clone()),
            }
        }

        for source in QuoteSource::ALL {
            let batch: Vec<AssetRef> = to_fetch
                .iter()
                .filter(|a| a.source() == source)
                .cloned()
                .collect();
            if batch.is_empty() {
                continue;
            }
            match self.provider(source) {
                Some(provider) => match provider.quotes(&batch).await {
                    Ok(result) => {
                        if !result.uncovered.is_empty() {
                            refresh.issues.push(PricingIssue::NotCovered {
                                source,
                                symbols: result
                                    .uncovered
                                    .iter()
                                    .map(|a| a.symbol().to_string())
                                    .collect(),
                            });
                        }
                        for quote in result.quotes {
                            self.cache.put(&quote)?;
                            refresh
                                .prices
                                .insert(quote.with_freshness(QuoteFreshness::Live));
                        }
                    }
                    Err(error) => refresh.issues.push(issue(source, &error)),
                },
                None => refresh.issues.push(PricingIssue::Unavailable(source)),
            }
            for asset in &batch {
                if refresh.prices.get(asset).is_none()
                    && let Some(cached) = self.cache.get(asset)?
                {
                    refresh
                        .prices
                        .insert(cached.with_freshness(QuoteFreshness::Cached));
                }
            }
        }
        Ok(refresh)
    }

    /// Recherche d'actifs auprès d'une source (« CW8 », « bitcoin »…).
    pub async fn search(&self, source: QuoteSource, query: &str) -> AppResult<Vec<AssetRef>> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let provider = self.provider(source).ok_or(AppError::NoPriceProvider)?;
        Ok(provider.search(query).await?)
    }

    /// État du compte auprès d'une source (ex. offre de la clé Twelve Data).
    pub async fn source_status(&self, source: QuoteSource) -> AppResult<Option<String>> {
        let provider = self.provider(source).ok_or(AppError::NoPriceProvider)?;
        Ok(provider.account_status().await?)
    }

    fn provider(&self, source: QuoteSource) -> Option<&Arc<dyn PriceProvider>> {
        self.providers.iter().find(|p| p.source() == source)
    }
}

fn issue(source: QuoteSource, error: &PriceError) -> PricingIssue {
    match error {
        PriceError::RateLimited => PricingIssue::RateLimited(source),
        PriceError::MissingApiKey => PricingIssue::MissingApiKey(source),
        PriceError::Unavailable(_) | PriceError::InvalidResponse(_) => {
            PricingIssue::Unavailable(source)
        }
    }
}

/// Actifs dont le cours est nécessaire pour valoriser les comptes actifs : positions des PEA et
/// portefeuilles crypto, actifs de référence des fonds PEI.
pub fn tracked_assets(accounts: &[Account]) -> Vec<AssetRef> {
    let mut assets: Vec<AssetRef> = Vec::new();
    for account in accounts.iter().filter(|a| !a.is_archived()) {
        let candidates: Vec<&AssetRef> = match account.kind() {
            AccountKind::Savings(_) | AccountKind::Checking(_) => Vec::new(),
            AccountKind::Securities(details) => {
                details.positions().iter().map(|p| p.asset()).collect()
            }
            AccountKind::EmployeeSavings(details) => details
                .funds()
                .iter()
                .filter_map(|f| f.reference())
                .collect(),
        };
        for asset in candidates {
            if !assets.iter().any(|a| a.is_same_asset(asset)) {
                assets.push(asset.clone());
            }
        }
    }
    assets
}

/// Valorise les comptes actifs (non archivés) avec les cours disponibles.
pub(crate) async fn value_active_accounts(
    accounts: Vec<Account>,
    pricing: &PricingService,
    ledger: &Ledger,
    force_refresh: bool,
) -> AppResult<(Vec<(Account, AccountValuation)>, Vec<PricingIssue>)> {
    let active: Vec<Account> = accounts.into_iter().filter(|a| !a.is_archived()).collect();
    let refresh = pricing
        .prices_for(&tracked_assets(&active), force_refresh)
        .await?;
    let valued = active
        .into_iter()
        .map(|account| {
            let valuation = value_account(&account, &refresh.prices, ledger);
            (account, valuation)
        })
        .collect();
    Ok((valued, refresh.issues))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestEnv, asset};
    use lc_domain::Money;
    use pollster::block_on;
    use rust_decimal_macros::dec;
    use std::sync::atomic::Ordering;

    // Spec market-pricing : « Rafraîchissement groupé »
    #[test]
    fn assets_of_same_source_are_fetched_in_one_request() {
        let env = TestEnv::new();
        env.coingecko.set_price("bitcoin", dec!(60000));
        env.coingecko.set_price("ethereum", dec!(2500));
        env.coingecko.set_price("solana", dec!(150));
        let assets = ["bitcoin", "ethereum", "solana"].map(|s| asset(QuoteSource::CoinGecko, s));

        let refresh = block_on(env.pricing().prices_for(&assets, false)).unwrap();

        assert_eq!(env.coingecko.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            refresh.prices.get(&assets[2]).unwrap().price(),
            Money::new(dec!(150))
        );
        assert!(refresh.issues.is_empty());
    }

    // Spec market-pricing : « Cache frais »
    #[test]
    fn fresh_cache_avoids_network_calls() {
        let env = TestEnv::new();
        env.coingecko.set_price("bitcoin", dec!(60000));
        let btc = [asset(QuoteSource::CoinGecko, "bitcoin")];
        let pricing = env.pricing();
        block_on(pricing.prices_for(&btc, false)).unwrap();

        env.clock.advance(Duration::minutes(2));
        let refresh = block_on(pricing.prices_for(&btc, false)).unwrap();

        assert_eq!(env.coingecko.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            refresh.prices.get(&btc[0]).unwrap().freshness(),
            QuoteFreshness::Live
        );

        env.clock.advance(Duration::minutes(4));
        block_on(pricing.prices_for(&btc, false)).unwrap();
        assert_eq!(env.coingecko.calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn forced_refresh_ignores_fresh_cache() {
        let env = TestEnv::new();
        env.coingecko.set_price("bitcoin", dec!(60000));
        let btc = [asset(QuoteSource::CoinGecko, "bitcoin")];
        let pricing = env.pricing();
        block_on(pricing.prices_for(&btc, false)).unwrap();
        block_on(pricing.prices_for(&btc, true)).unwrap();
        assert_eq!(env.coingecko.calls.load(Ordering::SeqCst), 2);
    }

    // Spec market-pricing : « Hors ligne » et « Limite atteinte »
    #[test]
    fn falls_back_to_cache_when_source_fails() {
        let env = TestEnv::new();
        env.twelve_data.set_price("CW8", dec!(520));
        let cw8 = [asset(QuoteSource::TwelveData, "CW8")];
        let pricing = env.pricing();
        block_on(pricing.prices_for(&cw8, false)).unwrap();
        let first_quote_time = env.clock.now();

        env.clock.advance(Duration::hours(3));
        env.twelve_data.fail_with(PriceError::RateLimited);
        let refresh = block_on(pricing.prices_for(&cw8, false)).unwrap();

        let quote = refresh.prices.get(&cw8[0]).unwrap();
        assert_eq!(quote.freshness(), QuoteFreshness::Cached);
        assert_eq!(quote.as_of(), first_quote_time);
        assert_eq!(
            refresh.issues,
            vec![PricingIssue::RateLimited(QuoteSource::TwelveData)]
        );
    }

    // Spec market-pricing : « Clé absente »
    #[test]
    fn missing_api_key_is_reported() {
        let env = TestEnv::new();
        env.twelve_data.fail_with(PriceError::MissingApiKey);
        let refresh = block_on(
            env.pricing()
                .prices_for(&[asset(QuoteSource::TwelveData, "CW8")], false),
        )
        .unwrap();

        assert!(
            refresh
                .prices
                .get(&asset(QuoteSource::TwelveData, "CW8"))
                .is_none()
        );
        assert_eq!(
            refresh.issues,
            vec![PricingIssue::MissingApiKey(QuoteSource::TwelveData)]
        );
    }

    #[test]
    fn one_failing_source_does_not_affect_the_other() {
        let env = TestEnv::new();
        env.coingecko.set_price("bitcoin", dec!(60000));
        env.twelve_data
            .fail_with(PriceError::Unavailable("timeout".into()));
        let assets = [
            asset(QuoteSource::CoinGecko, "bitcoin"),
            asset(QuoteSource::TwelveData, "CW8"),
        ];

        let refresh = block_on(env.pricing().prices_for(&assets, false)).unwrap();

        assert!(refresh.prices.get(&assets[0]).is_some());
        assert_eq!(
            refresh.issues,
            vec![PricingIssue::Unavailable(QuoteSource::TwelveData)]
        );
    }

    #[test]
    fn uncovered_assets_are_reported() {
        let env = TestEnv::new();
        env.twelve_data.set_price("AAPL", dec!(340));
        env.twelve_data.refuse("CW8:Euronext");
        let assets = [
            asset(QuoteSource::TwelveData, "AAPL"),
            asset(QuoteSource::TwelveData, "CW8:Euronext"),
        ];

        let refresh = block_on(env.pricing().prices_for(&assets, false)).unwrap();

        assert!(refresh.prices.get(&assets[0]).is_some());
        assert!(refresh.prices.get(&assets[1]).is_none());
        assert_eq!(
            refresh.issues,
            vec![PricingIssue::NotCovered {
                source: QuoteSource::TwelveData,
                symbols: vec!["CW8:Euronext".into()]
            }]
        );
    }

    #[test]
    fn search_delegates_to_source_and_ignores_blank_queries() {
        let env = TestEnv::new();
        let pricing = env.pricing();

        let results = block_on(pricing.search(QuoteSource::TwelveData, "cw8")).unwrap();
        assert_eq!(results[0].symbol(), "CW8");
        assert!(
            block_on(pricing.search(QuoteSource::CoinGecko, "  "))
                .unwrap()
                .is_empty()
        );
    }
}
