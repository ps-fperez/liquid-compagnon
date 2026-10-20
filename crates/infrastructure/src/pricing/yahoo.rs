use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use lc_application::ports::{Clock, PriceError, PriceProvider, QuoteBatch};
use lc_domain::account::{AssetRef, CurrencyCode, QuoteSource};
use lc_domain::valuation::{Quote, QuoteFreshness};
use lc_domain::{Decimal, Money};
use reqwest::{Client, StatusCode};
use serde_json::Value;

use super::{json_decimal, transport_error};

/// Cours des ETF et actions (dont places européennes : Paris, Amsterdam, Francfort, Londres…)
/// via Yahoo Finance, sans clé. API non officielle : un changement de format se traduit par une
/// erreur « réponse inattendue » et les valeurs restent servies par le cache.
pub struct YahooFinanceProvider {
    client: Client,
    base_url: String,
    clock: Arc<dyn Clock>,
}

/// Types d'instruments proposés à la recherche (pas de devises, de contrats à terme…).
const SEARCHABLE_TYPES: [&str; 4] = ["ETF", "EQUITY", "MUTUALFUND", "INDEX"];

enum Fetch {
    Found(Value),
    NotFound,
}

impl YahooFinanceProvider {
    pub const BASE_URL: &'static str = "https://query1.finance.yahoo.com";

    pub fn new(client: Client, clock: Arc<dyn Clock>) -> Self {
        Self::with_base_url(client, Self::BASE_URL, clock)
    }

    pub fn with_base_url(client: Client, base_url: &str, clock: Arc<dyn Clock>) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            clock,
        }
    }

    async fn get_json(&self, path: &str, query: &[(&str, &str)]) -> Result<Fetch, PriceError> {
        let response = self
            .client
            .get(format!("{}{path}", self.base_url))
            .query(query)
            .send()
            .await
            .map_err(transport_error)?;
        match response.status() {
            StatusCode::TOO_MANY_REQUESTS => Err(PriceError::RateLimited),
            StatusCode::NOT_FOUND => Ok(Fetch::NotFound),
            status if !status.is_success() => {
                Err(PriceError::Unavailable(format!("HTTP {status}")))
            }
            _ => response
                .json()
                .await
                .map(Fetch::Found)
                .map_err(transport_error),
        }
    }

    /// Dernier cours et devise de cotation d'un symbole (`None` si inconnu).
    async fn market_price(&self, symbol: &str) -> Result<Option<(Decimal, String)>, PriceError> {
        let body = match self
            .get_json(
                &format!("/v8/finance/chart/{symbol}"),
                &[("range", "1d"), ("interval", "1d")],
            )
            .await?
        {
            Fetch::Found(body) => body,
            Fetch::NotFound => return Ok(None),
        };
        let meta = &body["chart"]["result"][0]["meta"];
        let (Some(price), Some(currency)) = (
            json_decimal(&meta["regularMarketPrice"]),
            meta["currency"].as_str(),
        ) else {
            return Ok(None);
        };
        Ok(Some((price, currency.to_string())))
    }

    /// Prix en euros : les cotations en pence (`GBp`, `GBX`) sont ramenées en livres, puis
    /// converties au taux du jour (`GBPEUR=X`…), mémorisé pour la requête.
    async fn to_eur(
        &self,
        price: Decimal,
        currency: &str,
        rates: &mut HashMap<String, Decimal>,
    ) -> Result<Option<Decimal>, PriceError> {
        let (price, currency) = match currency {
            "GBp" | "GBX" => (price / Decimal::ONE_HUNDRED, "GBP".to_string()),
            other => (price, other.to_ascii_uppercase()),
        };
        if currency == "EUR" {
            return Ok(Some(price));
        }
        if let Some(rate) = rates.get(&currency) {
            return Ok(Some(price * rate));
        }
        let Some((rate, _)) = self.market_price(&format!("{currency}EUR=X")).await? else {
            return Ok(None);
        };
        rates.insert(currency, rate);
        Ok(Some(price * rate))
    }
}

/// Ordre d'affichage des places : Paris (0), autres places de la zone euro (1), le reste (2).
fn place_rank(symbol: &str) -> u8 {
    match symbol.rsplit_once('.').map(|(_, suffix)| suffix) {
        Some("PA") => 0,
        Some(suffix) if EURO_PLACES.contains(&suffix) => 1,
        _ => 2,
    }
}

const EURO_PLACES: [&str; 11] = [
    "PA", "AS", "DE", "F", "MI", "BR", "MC", "LS", "VI", "HE", "IR",
];

/// Devise probable d'après le suffixe de place (informative : la conversion utilise la devise
/// renvoyée avec le cours).
fn currency_for(symbol: &str) -> &'static str {
    match symbol.rsplit_once('.').map(|(_, suffix)| suffix) {
        Some(suffix) if EURO_PLACES.contains(&suffix) => "EUR",
        Some("L") => "GBP",
        Some("SW") => "CHF",
        Some(_) => "EUR",
        None => "USD",
    }
}

#[async_trait]
impl PriceProvider for YahooFinanceProvider {
    fn source(&self) -> QuoteSource {
        QuoteSource::Yahoo
    }

    /// Une requête par actif (l'API groupée de Yahoo exige une authentification) ; le cache de
    /// 5 minutes du service de cours limite leur nombre.
    async fn quotes(&self, assets: &[AssetRef]) -> Result<QuoteBatch, PriceError> {
        let mut batch = QuoteBatch::default();
        let mut rates = HashMap::new();
        let now = self.clock.now();
        for asset in assets {
            let price = match self.market_price(asset.symbol()).await? {
                Some((price, currency)) => self.to_eur(price, &currency, &mut rates).await?,
                None => None,
            };
            match price {
                Some(price) => batch.quotes.push(Quote::new(
                    asset.clone(),
                    Money::new(price),
                    now,
                    QuoteFreshness::Live,
                )),
                None => batch.uncovered.push(asset.clone()),
            }
        }
        Ok(batch)
    }

    async fn search(&self, query: &str) -> Result<Vec<AssetRef>, PriceError> {
        let body = match self
            .get_json(
                "/v1/finance/search",
                &[("q", query), ("quotesCount", "15"), ("newsCount", "0")],
            )
            .await?
        {
            Fetch::Found(body) => body,
            Fetch::NotFound => return Ok(Vec::new()),
        };
        let items = body["quotes"]
            .as_array()
            .ok_or_else(|| PriceError::InvalidResponse("liste « quotes » absente".into()))?;
        let mut found: Vec<AssetRef> = items
            .iter()
            .filter(|item| {
                item["quoteType"]
                    .as_str()
                    .is_some_and(|t| SEARCHABLE_TYPES.contains(&t))
            })
            .filter_map(|item| {
                let symbol = item["symbol"].as_str()?;
                let name = item["longname"]
                    .as_str()
                    .or(item["shortname"].as_str())
                    .unwrap_or(symbol);
                let display_name = match item["exchDisp"].as_str() {
                    Some(place) => format!("{name} · {place}"),
                    None => name.to_string(),
                };
                let currency = CurrencyCode::new(currency_for(symbol)).ok()?;
                AssetRef::new(QuoteSource::Yahoo, symbol, &display_name, currency).ok()
            })
            .collect();
        // Cotations éligibles au PEA en tête : Paris, puis le reste de la zone euro.
        found.sort_by_key(|asset| place_rank(asset.symbol()));
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pricing::test_support::TestClock;
    use rust_decimal_macros::dec;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider(server: &MockServer) -> YahooFinanceProvider {
        YahooFinanceProvider::with_base_url(
            Client::new(),
            &server.uri(),
            Arc::new(TestClock::new()),
        )
    }

    fn listing(symbol: &str) -> AssetRef {
        AssetRef::new(
            QuoteSource::Yahoo,
            symbol,
            "",
            CurrencyCode::new(currency_for(symbol)).unwrap(),
        )
        .unwrap()
    }

    fn chart(price: f64, currency: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(json!({
            "chart": { "result": [{ "meta": { "currency": currency, "regularMarketPrice": price } }], "error": null }
        }))
    }

    async fn mount_chart(server: &MockServer, symbol: &str, price: f64, currency: &str) {
        Mock::given(method("GET"))
            .and(path(format!("/v8/finance/chart/{symbol}")))
            .respond_with(chart(price, currency))
            .mount(server)
            .await;
    }

    // Cas réel qui a motivé cette source : CW8 à Paris, absent de l'offre gratuite Twelve Data.
    #[tokio::test]
    async fn euronext_etf_in_euros() {
        let server = MockServer::start().await;
        mount_chart(&server, "CW8.PA", 709.33, "EUR").await;

        let batch = provider(&server)
            .quotes(&[listing("CW8.PA")])
            .await
            .unwrap();

        assert_eq!(batch.quotes[0].price(), Money::new(dec!(709.33)));
        assert!(batch.uncovered.is_empty());
    }

    #[tokio::test]
    async fn london_pence_are_converted_to_euros() {
        let server = MockServer::start().await;
        mount_chart(&server, "SWDA.L", 10960.0, "GBp").await;
        mount_chart(&server, "GBPEUR=X", 1.15, "EUR").await;

        let batch = provider(&server)
            .quotes(&[listing("SWDA.L")])
            .await
            .unwrap();

        assert_eq!(batch.quotes[0].price(), Money::new(dec!(126.04)));
    }

    #[tokio::test]
    async fn exchange_rate_is_fetched_once_per_currency() {
        let server = MockServer::start().await;
        mount_chart(&server, "AAPL", 340.0, "USD").await;
        mount_chart(&server, "MSFT", 400.0, "USD").await;
        Mock::given(path("/v8/finance/chart/USDEUR=X"))
            .respond_with(chart(0.9, "EUR"))
            .expect(1)
            .mount(&server)
            .await;

        let batch = provider(&server)
            .quotes(&[listing("AAPL"), listing("MSFT")])
            .await
            .unwrap();

        assert_eq!(batch.quotes[0].price(), Money::new(dec!(306)));
        assert_eq!(batch.quotes[1].price(), Money::new(dec!(360)));
    }

    #[tokio::test]
    async fn unknown_symbol_is_reported_as_uncovered() {
        let server = MockServer::start().await;
        Mock::given(path("/v8/finance/chart/ZZZZ.PA"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "chart": { "result": null, "error": { "code": "Not Found", "description": "No data found" } }
            })))
            .mount(&server)
            .await;
        mount_chart(&server, "CW8.PA", 709.33, "EUR").await;

        let batch = provider(&server)
            .quotes(&[listing("ZZZZ.PA"), listing("CW8.PA")])
            .await
            .unwrap();

        assert_eq!(batch.quotes.len(), 1);
        assert_eq!(batch.uncovered[0].symbol(), "ZZZZ.PA");
    }

    #[tokio::test]
    async fn rate_limit_is_an_error() {
        let server = MockServer::start().await;
        Mock::given(path("/v8/finance/chart/CW8.PA"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;
        assert_eq!(
            provider(&server).quotes(&[listing("CW8.PA")]).await,
            Err(PriceError::RateLimited)
        );
    }

    /// Contrôle manuel contre la vraie API (non officielle) : `cargo test -p lc-infrastructure
    /// yahoo -- --ignored`.
    #[tokio::test]
    #[ignore = "réseau"]
    async fn live_api_still_matches_the_expected_format() {
        let provider = YahooFinanceProvider::new(
            crate::pricing::http_client().unwrap(),
            Arc::new(TestClock::new()),
        );
        let found = provider.search("CW8").await.unwrap();
        assert!(
            found.iter().any(|asset| asset.symbol() == "CW8.PA"),
            "{found:?}"
        );

        let batch = provider
            .quotes(&[
                listing("CW8.PA"),
                listing("SWDA.L"),
                listing("AAPL"),
                listing("ZZZZQ.PA"),
            ])
            .await
            .unwrap();
        for quote in &batch.quotes {
            println!("{} = {:?}", quote.asset().symbol(), quote.price());
        }
        assert_eq!(batch.quotes.len(), 3);
        assert_eq!(batch.uncovered, vec![listing("ZZZZQ.PA")]);
    }

    // Spec account-management : « Recherche d'actif »
    #[tokio::test]
    async fn search_keeps_securities_with_their_place() {
        let server = MockServer::start().await;
        Mock::given(path("/v1/finance/search"))
            .and(query_param("q", "CW8"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "quotes": [
                    { "symbol": "CW8.PA", "quoteType": "ETF", "longname": "Amundi MSCI World Swap UCITS ETF EUR Acc", "exchDisp": "Paris" },
                    { "symbol": "CW8U.L", "quoteType": "ETF", "shortname": "AMUNDI MSCI WORLD", "exchDisp": "London" },
                    { "symbol": "EURUSD=X", "quoteType": "CURRENCY", "shortname": "EUR/USD" },
                    { "symbol": "CW8.MI", "quoteType": "ETF", "shortname": "AMUNDI MSCI WORLD", "exchDisp": "Milan" }
                ]
            })))
            .mount(&server)
            .await;

        let results = provider(&server).search("CW8").await.unwrap();

        let symbols: Vec<_> = results.iter().map(AssetRef::symbol).collect();
        assert_eq!(
            symbols,
            ["CW8.PA", "CW8.MI", "CW8U.L"],
            "Paris, zone euro, puis le reste"
        );
        assert_eq!(
            results[0].display_name(),
            "Amundi MSCI World Swap UCITS ETF EUR Acc · Paris"
        );
        assert_eq!(results[0].currency().as_str(), "EUR");
        assert_eq!(results[2].currency().as_str(), "GBP");
    }
}
