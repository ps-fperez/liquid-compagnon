use std::fmt;

use crate::DomainError;

/// Source de cotation d'un actif.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuoteSource {
    /// Crypto-actifs (identifiants CoinGecko en minuscules : `bitcoin`, `ethereum`…).
    CoinGecko,
    /// ETF, actions, indices (symboles Twelve Data en majuscules : `CW8`, `IWDA`…).
    TwelveData,
}

/// Code devise ISO 4217 (3 lettres majuscules).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CurrencyCode(String);

impl CurrencyCode {
    pub fn new(raw: &str) -> Result<Self, DomainError> {
        let code = raw.trim().to_ascii_uppercase();
        if code.len() != 3 || !code.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(DomainError::InvalidCurrency(raw.to_string()));
        }
        Ok(Self(code))
    }

    pub fn eur() -> Self {
        Self("EUR".into())
    }

    pub fn is_eur(&self) -> bool {
        self.0 == "EUR"
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Référence vers un actif coté. Deux références désignent le même actif si elles ont la même
/// source et le même symbole (le nom affiché et la devise sont descriptifs).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetRef {
    source: QuoteSource,
    symbol: String,
    display_name: String,
    currency: CurrencyCode,
}

impl AssetRef {
    pub fn new(
        source: QuoteSource,
        symbol: &str,
        display_name: &str,
        currency: CurrencyCode,
    ) -> Result<Self, DomainError> {
        let symbol = symbol.trim();
        if symbol.is_empty() {
            return Err(DomainError::InvalidSymbol);
        }
        let symbol = match source {
            QuoteSource::CoinGecko => symbol.to_ascii_lowercase(),
            QuoteSource::TwelveData => symbol.to_ascii_uppercase(),
        };
        let display_name = match display_name.trim() {
            "" => symbol.clone(),
            name => name.to_string(),
        };
        Ok(Self {
            source,
            symbol,
            display_name,
            currency,
        })
    }

    pub fn source(&self) -> QuoteSource {
        self.source
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn currency(&self) -> &CurrencyCode {
        &self.currency
    }

    pub fn is_same_asset(&self, other: &AssetRef) -> bool {
        self.source == other.source && self.symbol == other.symbol
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_symbol_per_source() {
        let btc =
            AssetRef::new(QuoteSource::CoinGecko, " Bitcoin ", "", CurrencyCode::eur()).unwrap();
        let cw8 = AssetRef::new(
            QuoteSource::TwelveData,
            "cw8",
            "Amundi MSCI World",
            CurrencyCode::eur(),
        )
        .unwrap();

        assert_eq!(btc.symbol(), "bitcoin");
        assert_eq!(btc.display_name(), "bitcoin");
        assert_eq!(cw8.symbol(), "CW8");
        assert_eq!(cw8.display_name(), "Amundi MSCI World");
    }

    #[test]
    fn same_asset_ignores_display_name() {
        let a = AssetRef::new(
            QuoteSource::TwelveData,
            "IWDA",
            "iShares",
            CurrencyCode::eur(),
        )
        .unwrap();
        let b = AssetRef::new(
            QuoteSource::TwelveData,
            "iwda",
            "iShares Core MSCI World",
            CurrencyCode::eur(),
        )
        .unwrap();
        let c = AssetRef::new(QuoteSource::CoinGecko, "iwda", "", CurrencyCode::eur()).unwrap();

        assert!(a.is_same_asset(&b));
        assert!(!a.is_same_asset(&c));
    }

    #[test]
    fn rejects_blank_symbol_and_bad_currency() {
        assert_eq!(
            AssetRef::new(QuoteSource::CoinGecko, "  ", "", CurrencyCode::eur()),
            Err(DomainError::InvalidSymbol)
        );
        assert!(CurrencyCode::new("usd").unwrap().as_str() == "USD");
        assert!(CurrencyCode::new("EURO").is_err());
        assert!(CurrencyCode::new("E1R").is_err());
    }
}
