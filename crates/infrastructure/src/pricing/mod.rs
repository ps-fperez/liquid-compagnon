//! Sources de cotation HTTP. Les requêtes ne contiennent que des identifiants d'actifs.

mod coingecko;
mod twelve_data;
mod yahoo;

use std::str::FromStr;
use std::time::Duration;

use lc_application::ports::PriceError;
use lc_domain::Decimal;
use serde_json::Value;

pub use coingecko::CoinGeckoProvider;
pub use twelve_data::TwelveDataProvider;
pub use yahoo::YahooFinanceProvider;

/// Client HTTP partagé : délai de 10 s, TLS rustls avec certificats racine embarqués.
pub fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("LiquidCompagnon/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| error.to_string())
}

/// Lit un nombre JSON (nombre ou chaîne, éventuellement en notation scientifique) en décimal.
fn json_decimal(value: &Value) -> Option<Decimal> {
    let raw = match value {
        Value::String(text) => text.trim().to_string(),
        Value::Number(number) => number.to_string(),
        _ => return None,
    };
    Decimal::from_str(&raw)
        .or_else(|_| Decimal::from_scientific(&raw))
        .ok()
}

fn transport_error(error: reqwest::Error) -> PriceError {
    if error.is_timeout() {
        PriceError::Unavailable("délai dépassé".into())
    } else if error.is_decode() {
        PriceError::InvalidResponse(error.to_string())
    } else {
        PriceError::Unavailable(error.to_string())
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Mutex;

    use chrono::{DateTime, Duration, NaiveDate, Utc};
    use lc_application::ports::Clock;

    pub struct TestClock(Mutex<DateTime<Utc>>);

    impl TestClock {
        pub fn new() -> Self {
            Self(Mutex::new(
                NaiveDate::from_ymd_opt(2026, 10, 20)
                    .unwrap()
                    .and_hms_opt(9, 42, 0)
                    .unwrap()
                    .and_utc(),
            ))
        }

        pub fn advance(&self, duration: Duration) {
            *self.0.lock().unwrap() += duration;
        }
    }

    impl Clock for TestClock {
        fn now(&self) -> DateTime<Utc> {
            *self.0.lock().unwrap()
        }

        fn today(&self) -> NaiveDate {
            self.now().date_naive()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn reads_numbers_strings_and_scientific_notation() {
        assert_eq!(json_decimal(&json!(60000.12)), Some(dec!(60000.12)));
        assert_eq!(json_decimal(&json!("520.10")), Some(dec!(520.10)));
        assert_eq!(json_decimal(&json!(0.0000001)), Some(dec!(0.0000001)));
        assert_eq!(json_decimal(&json!(null)), None);
        assert_eq!(json_decimal(&json!("abc")), None);
    }
}
