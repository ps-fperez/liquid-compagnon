use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use lc_application::ports::{Clock, PriceError, PriceProvider, SettingsStore};
use lc_domain::account::{AssetRef, CurrencyCode, QuoteSource};
use lc_domain::valuation::{Quote, QuoteFreshness};
use lc_domain::{Decimal, Money};
use reqwest::{Client, StatusCode};
use serde_json::Value;

use super::{json_decimal, transport_error};

/// Cours des ETF, actions et indices via Twelve Data, convertis en euros.
///
/// L'offre gratuite autorise 8 crédits par minute (1 crédit par symbole et par conversion de
/// devise). Un compteur local ne sert que ce que le quota permet ; les actifs restants sont
/// laissés au cache plutôt que de déclencher une erreur 429.
pub struct TwelveDataProvider {
    client: Client,
    base_url: String,
    settings: Arc<dyn SettingsStore>,
    clock: Arc<dyn Clock>,
    spent: Mutex<VecDeque<(DateTime<Utc>, u32)>>,
}

impl TwelveDataProvider {
    pub const BASE_URL: &'static str = "https://api.twelvedata.com";
    pub const CREDITS_PER_MINUTE: u32 = 8;
    const SEARCH_LIMIT: &'static str = "20";

    pub fn new(client: Client, settings: Arc<dyn SettingsStore>, clock: Arc<dyn Clock>) -> Self {
        Self::with_base_url(client, Self::BASE_URL, settings, clock)
    }

    pub fn with_base_url(
        client: Client,
        base_url: &str,
        settings: Arc<dyn SettingsStore>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            settings,
            clock,
            spent: Mutex::default(),
        }
    }

    fn api_key(&self) -> Result<Option<String>, PriceError> {
        self.settings
            .twelve_data_api_key()
            .map_err(|error| PriceError::Unavailable(error.to_string()))
    }

    /// Réserve les crédits nécessaires et renvoie le nombre d'actifs (en tête de liste) qui
    /// peuvent être demandés, avec les devises à convertir.
    fn reserve_credits(
        &self,
        assets: &[AssetRef],
    ) -> Result<(usize, Vec<CurrencyCode>), PriceError> {
        let now = self.clock.now();
        let mut spent = self
            .spent
            .lock()
            .map_err(|_| PriceError::Unavailable("compteur de crédits".into()))?;
        spent.retain(|(at, _)| now - *at < Duration::minutes(1));
        let mut available =
            Self::CREDITS_PER_MINUTE.saturating_sub(spent.iter().map(|(_, credits)| credits).sum());
        let mut served = 0;
        let mut currencies: Vec<CurrencyCode> = Vec::new();
        for asset in assets {
            let needs_conversion =
                !asset.currency().is_eur() && !currencies.contains(asset.currency());
            let cost = 1 + u32::from(needs_conversion);
            if cost > available {
                break;
            }
            available -= cost;
            served += 1;
            if needs_conversion {
                currencies.push(asset.currency().clone());
            }
        }
        if served == 0 {
            return Err(PriceError::RateLimited);
        }
        spent.push_back((now, served as u32 + currencies.len() as u32));
        Ok((served, currencies))
    }

    async fn get_json(
        &self,
        path: &str,
        query: &[(&str, &str)],
        key: Option<&str>,
    ) -> Result<Value, PriceError> {
        let mut request = self
            .client
            .get(format!("{}{path}", self.base_url))
            .query(query);
        if let Some(key) = key {
            request = request.header("Authorization", format!("apikey {key}"));
        }
        let response = request.send().await.map_err(transport_error)?;
        match response.status() {
            StatusCode::TOO_MANY_REQUESTS => return Err(PriceError::RateLimited),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(PriceError::MissingApiKey);
            }
            status if !status.is_success() => {
                return Err(PriceError::Unavailable(format!("HTTP {status}")));
            }
            _ => {}
        }
        let body: Value = response.json().await.map_err(transport_error)?;
        match api_error(&body) {
            Some(error) => Err(error),
            None => Ok(body),
        }
    }
}

/// Twelve Data signale souvent ses erreurs dans un corps HTTP 200 :
/// `{"code": 429, "message": "...", "status": "error"}`.
fn api_error(body: &Value) -> Option<PriceError> {
    if body["status"].as_str() != Some("error") {
        return None;
    }
    Some(match body["code"].as_u64() {
        Some(429) => PriceError::RateLimited,
        Some(401 | 403) => PriceError::MissingApiKey,
        _ => PriceError::InvalidResponse(
            body["message"]
                .as_str()
                .unwrap_or("erreur Twelve Data")
                .to_string(),
        ),
    })
}

/// Entrée d'un symbole dans une réponse groupée (objet indexé par symbole). Le symbole peut être
/// renvoyé avec ou sans sa place de cotation, dans une casse différente.
fn batch_entry<'a>(body: &'a Value, symbol: &str) -> Option<&'a Value> {
    let ticker = symbol.split_once(':').map_or(symbol, |(ticker, _)| ticker);
    body.as_object()?
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(symbol))
        .or_else(|| {
            body.as_object()?
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(ticker))
        })
        .map(|(_, entry)| entry)
}

#[async_trait]
impl PriceProvider for TwelveDataProvider {
    fn source(&self) -> QuoteSource {
        QuoteSource::TwelveData
    }

    async fn quotes(&self, assets: &[AssetRef]) -> Result<Vec<Quote>, PriceError> {
        if assets.is_empty() {
            return Ok(Vec::new());
        }
        let key = self.api_key()?.ok_or(PriceError::MissingApiKey)?;
        let (served, currencies) = self.reserve_credits(assets)?;
        let assets = &assets[..served];

        let symbols = assets
            .iter()
            .map(AssetRef::symbol)
            .collect::<Vec<_>>()
            .join(",");
        let body = self
            .get_json("/price", &[("symbol", &symbols)], Some(&key))
            .await?;

        let mut to_eur: HashMap<String, Decimal> =
            HashMap::from([("EUR".to_string(), Decimal::ONE)]);
        for currency in currencies {
            let pair = format!("{currency}/EUR");
            let rate_body = self
                .get_json("/exchange_rate", &[("symbol", &pair)], Some(&key))
                .await?;
            let rate = json_decimal(&rate_body["rate"])
                .ok_or_else(|| PriceError::InvalidResponse(format!("taux {pair} absent")))?;
            to_eur.insert(currency.to_string(), rate);
        }

        let now = self.clock.now();
        Ok(assets
            .iter()
            .filter_map(|asset| {
                let entry = if assets.len() == 1 {
                    Some(&body)
                } else {
                    batch_entry(&body, asset.symbol())
                }?;
                if api_error(entry).is_some() {
                    return None;
                }
                let price = json_decimal(&entry["price"])?;
                let rate = to_eur.get(asset.currency().as_str())?;
                Some(Quote::new(
                    asset.clone(),
                    Money::new(price * rate),
                    now,
                    QuoteFreshness::Live,
                ))
            })
            .collect())
    }

    /// Recherche par ticker ou ISIN. Le symbole retenu inclut la place de cotation
    /// (`CW8:Euronext`) pour lever les ambiguïtés entre marchés.
    async fn search(&self, query: &str) -> Result<Vec<AssetRef>, PriceError> {
        let key = self.api_key()?;
        let body = self
            .get_json(
                "/symbol_search",
                &[("symbol", query), ("outputsize", Self::SEARCH_LIMIT)],
                key.as_deref(),
            )
            .await?;
        let items = body["data"]
            .as_array()
            .ok_or_else(|| PriceError::InvalidResponse("liste « data » absente".into()))?;
        Ok(items
            .iter()
            .filter_map(|item| {
                let ticker = item["symbol"].as_str()?;
                let exchange = item["exchange"].as_str().unwrap_or_default();
                let name = item["instrument_name"].as_str().unwrap_or(ticker);
                let currency = CurrencyCode::new(item["currency"].as_str()?).ok()?;
                let (symbol, display_name) = if exchange.is_empty() {
                    (ticker.to_string(), name.to_string())
                } else {
                    (
                        format!("{ticker}:{exchange}"),
                        format!("{name} · {exchange}"),
                    )
                };
                AssetRef::new(QuoteSource::TwelveData, &symbol, &display_name, currency).ok()
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pricing::test_support::TestClock;
    use crate::{Database, SqliteSettingsStore};
    use rust_decimal_macros::dec;
    use serde_json::json;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct Setup {
        provider: TwelveDataProvider,
        clock: Arc<TestClock>,
    }

    fn setup(server: &MockServer, key: Option<&str>) -> Setup {
        let settings = SqliteSettingsStore::new(Database::open_in_memory().unwrap());
        settings.set_twelve_data_api_key(key).unwrap();
        let clock = Arc::new(TestClock::new());
        let provider = TwelveDataProvider::with_base_url(
            Client::new(),
            &server.uri(),
            Arc::new(settings),
            clock.clone(),
        );
        Setup { provider, clock }
    }

    fn security(symbol: &str, currency: &str) -> AssetRef {
        AssetRef::new(
            QuoteSource::TwelveData,
            symbol,
            "",
            CurrencyCode::new(currency).unwrap(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn batch_request_with_api_key_header() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/price"))
            .and(query_param("symbol", "CW8:Euronext,IWDA"))
            .and(header("Authorization", "apikey secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "CW8:Euronext": { "price": "520.10" },
                "IWDA": { "price": "100.5" }
            })))
            .expect(1)
            .mount(&server)
            .await;
        let setup = setup(&server, Some("secret"));

        let quotes = setup
            .provider
            .quotes(&[security("CW8:Euronext", "EUR"), security("IWDA", "EUR")])
            .await
            .unwrap();

        assert_eq!(quotes.len(), 2);
        assert_eq!(quotes[0].price(), Money::new(dec!(520.10)));
        assert_eq!(quotes[1].price(), Money::new(dec!(100.5)));
    }

    #[tokio::test]
    async fn single_symbol_response_and_ticker_only_keys() {
        let server = MockServer::start().await;
        Mock::given(path("/price"))
            .and(query_param("symbol", "CW8:Euronext"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "price": "520.10" })))
            .mount(&server)
            .await;
        let setup = setup(&server, Some("k"));
        let single = setup
            .provider
            .quotes(&[security("CW8:Euronext", "EUR")])
            .await
            .unwrap();
        assert_eq!(single[0].price(), Money::new(dec!(520.10)));

        let body = json!({ "CW8": { "price": "1" }, "ESE": { "code": 400, "status": "error", "message": "symbole inconnu" } });
        assert_eq!(
            batch_entry(&body, "CW8:Euronext"),
            Some(&json!({ "price": "1" }))
        );
        assert!(api_error(batch_entry(&body, "ESE").unwrap()).is_some());
    }

    // Spec market-pricing : « ETF coté en dollars »
    #[tokio::test]
    async fn converts_foreign_currency_to_euros() {
        let server = MockServer::start().await;
        Mock::given(path("/price"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "price": "100" })))
            .mount(&server)
            .await;
        Mock::given(path("/exchange_rate"))
            .and(query_param("symbol", "USD/EUR"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "symbol": "USD/EUR", "rate": 0.92 })),
            )
            .expect(1)
            .mount(&server)
            .await;
        let setup = setup(&server, Some("k"));

        let quotes = setup
            .provider
            .quotes(&[security("CSPX", "USD")])
            .await
            .unwrap();

        assert_eq!(quotes[0].price(), Money::new(dec!(92)));
    }

    // Spec market-pricing : « Clé absente »
    #[tokio::test]
    async fn missing_key_means_no_request() {
        let server = MockServer::start().await;
        let setup = setup(&server, None);

        assert_eq!(
            setup.provider.quotes(&[security("CW8", "EUR")]).await,
            Err(PriceError::MissingApiKey)
        );
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    // Spec market-pricing : « Limite atteinte »
    #[tokio::test]
    async fn api_errors_in_body_are_mapped() {
        let server = MockServer::start().await;
        Mock::given(path("/price"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 429, "message": "API credits exhausted", "status": "error"
            })))
            .mount(&server)
            .await;
        let setup = setup(&server, Some("k"));
        assert_eq!(
            setup.provider.quotes(&[security("CW8", "EUR")]).await,
            Err(PriceError::RateLimited)
        );

        let invalid_key = MockServer::start().await;
        Mock::given(path("/price"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&invalid_key)
            .await;
        let setup = self::setup(&invalid_key, Some("wrong"));
        assert_eq!(
            setup.provider.quotes(&[security("CW8", "EUR")]).await,
            Err(PriceError::MissingApiKey)
        );
    }

    #[tokio::test]
    async fn free_plan_credits_are_respected_per_minute() {
        let server = MockServer::start().await;
        Mock::given(path("/price"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "A": { "price": "1" }, "B": { "price": "1" }, "C": { "price": "1" }
            })))
            .mount(&server)
            .await;
        let setup = setup(&server, Some("k"));
        let three = [
            security("A", "EUR"),
            security("B", "EUR"),
            security("C", "EUR"),
        ];

        assert_eq!(setup.provider.quotes(&three).await.unwrap().len(), 3);
        assert_eq!(setup.provider.quotes(&three).await.unwrap().len(), 3);
        // 6 crédits utilisés sur 8 : seuls les 2 premiers actifs sont demandés.
        assert_eq!(setup.provider.quotes(&three).await.unwrap().len(), 2);
        assert_eq!(
            setup.provider.quotes(&three).await,
            Err(PriceError::RateLimited)
        );

        setup.clock.advance(Duration::seconds(61));
        assert_eq!(setup.provider.quotes(&three).await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn search_builds_symbols_with_exchange() {
        let server = MockServer::start().await;
        Mock::given(path("/symbol_search"))
            .and(query_param("symbol", "CW8"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    { "symbol": "CW8", "instrument_name": "Amundi MSCI World UCITS ETF", "exchange": "Euronext", "currency": "EUR" },
                    { "symbol": "CW8", "instrument_name": "Amundi MSCI World", "exchange": "XETR", "currency": "EUR" },
                    { "symbol": "BAD", "instrument_name": "Sans devise", "exchange": "X" }
                ],
                "status": "ok"
            })))
            .mount(&server)
            .await;
        let setup = setup(&server, None);

        let results = setup.provider.search("CW8").await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].symbol(), "CW8:Euronext");
        assert_eq!(
            results[0].display_name(),
            "Amundi MSCI World UCITS ETF · Euronext"
        );
    }
}
