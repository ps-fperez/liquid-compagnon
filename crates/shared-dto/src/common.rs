use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountTypeDto {
    LivretA,
    Ldds,
    Pea,
    Pei,
    Crypto,
}

impl AccountTypeDto {
    pub const ALL: [AccountTypeDto; 5] = [
        AccountTypeDto::LivretA,
        AccountTypeDto::Ldds,
        AccountTypeDto::Pea,
        AccountTypeDto::Pei,
        AccountTypeDto::Crypto,
    ];

    pub const fn label(&self) -> &'static str {
        match self {
            AccountTypeDto::LivretA => "Livret A",
            AccountTypeDto::Ldds => "LDDS",
            AccountTypeDto::Pea => "PEA",
            AccountTypeDto::Pei => "PEI",
            AccountTypeDto::Crypto => "Portefeuille crypto",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuoteSourceDto {
    CoinGecko,
    TwelveData,
    Yahoo,
}

/// Actif coté (crypto, ETF, action).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetDto {
    pub source: QuoteSourceDto,
    pub symbol: String,
    pub display_name: String,
    pub currency: String,
}

/// Provenance d'une valeur affichée (badge « en direct », « en cache »…).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProvenanceDto {
    Live,
    Manual,
    Cached { as_of: DateTime<Utc> },
    Estimated,
    CostBasis,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PricingIssueKindDto {
    RateLimited,
    MissingApiKey,
    Unavailable,
    /// Actifs que la source ne cote pas (symbole inconnu, offre insuffisante).
    NotCovered,
}

/// Problème de cotation à signaler discrètement (les valeurs restent affichées).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricingIssueDto {
    pub source: QuoteSourceDto,
    pub kind: PricingIssueKindDto,
    /// Symboles concernés (`NotCovered`).
    #[serde(default)]
    pub symbols: Vec<String>,
}

/// Variation depuis le dernier snapshot automatique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariationDto {
    pub amount: Decimal,
    pub percent: Option<Decimal>,
}

/// Rendements annuels de projection, en pourcentage (5 = 5 %).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioRatesDto {
    pub pessimistic: Decimal,
    pub neutral: Decimal,
    pub optimistic: Decimal,
}
