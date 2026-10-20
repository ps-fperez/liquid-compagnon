use std::sync::Arc;

use async_trait::async_trait;
use lc_application::ports::{Clock, PriceError, PriceProvider};
use lc_domain::Money;
use lc_domain::account::{AssetRef, CurrencyCode, QuoteSource};
use lc_domain::valuation::{Quote, QuoteFreshness};
use reqwest::{Client, StatusCode};
use serde_json::Value;

use super::{json_decimal, transport_error};

/// Cours des crypto-actifs en euros via l'API publique CoinGecko (sans clé).
pub struct CoinGeckoProvider {
    client: Client,
    base_url: String,
    clock: Arc<dyn Clock>,
}

impl CoinGeckoProvider {
    pub const BASE_URL: &'static str = "https://api.coingecko.com/api/v3";
    const SEARCH_LIMIT: usize = 20;

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

    async fn get_json(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, PriceError> {
        let response = self
            .client
            .get(format!("{}{path}", self.base_url))
            .query(query)
            .send()
            .await
            .map_err(transport_error)?;
        match response.status() {
            StatusCode::TOO_MANY_REQUESTS => Err(PriceError::RateLimited),
            status if !status.is_success() => {
                Err(PriceError::Unavailable(format!("HTTP {status}")))
            }
            _ => response.json().await.map_err(transport_error),
        }
    }
}

#[async_trait]
impl PriceProvider for CoinGeckoProvider {
    fn source(&self) -> QuoteSource {
        QuoteSource::CoinGecko
    }

    /// Tous les actifs en une requête : `/simple/price?ids=bitcoin,ethereum&vs_currencies=eur`.
    async fn quotes(&self, assets: &[AssetRef]) -> Result<Vec<Quote>, PriceError> {
        if assets.is_empty() {
            return Ok(Vec::new());
        }
        let ids = assets
            .iter()
            .map(AssetRef::symbol)
            .collect::<Vec<_>>()
            .join(",");
        let body = self
            .get_json("/simple/price", &[("ids", &ids), ("vs_currencies", "eur")])
            .await?;
        let now = self.clock.now();
        Ok(assets
            .iter()
            .filter_map(|asset| {
                json_decimal(&body[asset.symbol()]["eur"]).map(|price| {
                    Quote::new(asset.clone(), Money::new(price), now, QuoteFreshness::Live)
                })
            })
            .collect())
    }

    async fn search(&self, query: &str) -> Result<Vec<AssetRef>, PriceError> {
        let body = self.get_json("/search", &[("query", query)]).await?;
        let coins = body["coins"]
            .as_array()
            .ok_or_else(|| PriceError::InvalidResponse("liste « coins » absente".into()))?;
        Ok(coins
            .iter()
            .filter_map(|coin| {
                let id = coin["id"].as_str()?;
                let name = coin["name"].as_str().unwrap_or(id);
                let ticker = coin["symbol"].as_str().unwrap_or_default().to_uppercase();
                let display_name = if ticker.is_empty() {
                    name.to_string()
                } else {
                    format!("{name} ({ticker})")
                };
                AssetRef::new(
                    QuoteSource::CoinGecko,
                    id,
                    &display_name,
                    CurrencyCode::eur(),
                )
                .ok()
            })
            .take(Self::SEARCH_LIMIT)
            .collect())
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

    fn provider(server: &MockServer) -> CoinGeckoProvider {
        CoinGeckoProvider::with_base_url(Client::new(), &server.uri(), Arc::new(TestClock::new()))
    }

    fn crypto(id: &str) -> AssetRef {
        AssetRef::new(QuoteSource::CoinGecko, id, "", CurrencyCode::eur()).unwrap()
    }

    // Spec market-pricing : « Rafraîchissement groupé »
    #[tokio::test]
    async fn fetches_all_assets_in_one_request() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/simple/price"))
            .and(query_param("ids", "bitcoin,ethereum,solana"))
            .and(query_param("vs_currencies", "eur"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "bitcoin": { "eur": 60000.12 },
                "ethereum": { "eur": 2500 }
            })))
            .expect(1)
            .mount(&server)
            .await;

        let quotes = provider(&server)
            .quotes(&[crypto("bitcoin"), crypto("ethereum"), crypto("solana")])
            .await
            .unwrap();

        assert_eq!(quotes.len(), 2, "solana est inconnu de la réponse");
        assert_eq!(quotes[0].price(), Money::new(dec!(60000.12)));
        assert_eq!(quotes[1].asset().symbol(), "ethereum");
        assert_eq!(quotes[1].freshness(), QuoteFreshness::Live);
    }

    // Spec market-pricing : « Limite atteinte »
    #[tokio::test]
    async fn maps_http_errors() {
        let server = MockServer::start().await;
        Mock::given(path("/simple/price"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;
        assert_eq!(
            provider(&server).quotes(&[crypto("bitcoin")]).await,
            Err(PriceError::RateLimited)
        );

        let down = MockServer::start().await;
        Mock::given(path("/simple/price"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&down)
            .await;
        assert!(matches!(
            provider(&down).quotes(&[crypto("bitcoin")]).await,
            Err(PriceError::Unavailable(_))
        ));
    }

    #[tokio::test]
    async fn no_request_without_assets() {
        let server = MockServer::start().await;
        assert_eq!(provider(&server).quotes(&[]).await, Ok(vec![]));
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    // Spec account-management : « Recherche d'actif »
    #[tokio::test]
    async fn search_returns_coin_ids_with_readable_names() {
        let server = MockServer::start().await;
        Mock::given(path("/search"))
            .and(query_param("query", "bitcoin"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "coins": [
                    { "id": "bitcoin", "name": "Bitcoin", "symbol": "btc" },
                    { "id": "wrapped-bitcoin", "name": "Wrapped Bitcoin", "symbol": "wbtc" }
                ]
            })))
            .mount(&server)
            .await;

        let results = provider(&server).search("bitcoin").await.unwrap();

        assert_eq!(results[0].symbol(), "bitcoin");
        assert_eq!(results[0].display_name(), "Bitcoin (BTC)");
        assert_eq!(results.len(), 2);
    }
}
