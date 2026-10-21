//! DTO partagés entre l'UI (WebAssembly) et le backend Tauri.
//!
//! Seule frontière connue de l'UI : elle n'a accès ni au domaine ni à l'infrastructure.
//! Conventions : montants et pourcentages en `Decimal` (sérialisés en chaînes exactes),
//! identifiants en chaînes UUID, mois au format `AAAA-MM`, dates ISO `AAAA-MM-JJ`.

mod accounts;
mod budget;
mod common;
mod dashboard;
mod error;
mod planning;
mod projection;
mod session;
mod transfers;
mod updates;

pub use accounts::*;
pub use budget::*;
pub use common::*;
pub use dashboard::*;
pub use error::*;
pub use planning::*;
pub use projection::*;
pub use session::*;
pub use transfers::*;
pub use updates::*;

pub use chrono::{DateTime, NaiveDate, Utc};
pub use rust_decimal::Decimal;

use serde::{Deserialize, Serialize};

/// Réponse de la commande `ping`, utilisée pour vérifier la liaison UI ↔ backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PingResponse {
    pub message: String,
    pub backend_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn ping_response_roundtrips_through_json() {
        let original = PingResponse {
            message: "pong".into(),
            backend_version: "0.1.0".into(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let decoded: PingResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn amounts_are_exact_strings_on_the_wire() {
        let variation = VariationDto {
            amount: dec!(1132.10),
            percent: Some(dec!(2.4)),
        };

        let json = serde_json::to_string(&variation).unwrap();

        assert_eq!(json, r#"{"amount":"1132.10","percent":"2.4"}"#);
        assert_eq!(
            serde_json::from_str::<VariationDto>(&json).unwrap(),
            variation
        );
    }

    #[test]
    fn tagged_enums_are_readable() {
        let provenance = ProvenanceDto::Cached {
            as_of: "2026-10-20T09:42:00Z".parse().unwrap(),
        };
        assert_eq!(
            serde_json::to_string(&provenance).unwrap(),
            r#"{"kind":"cached","as_of":"2026-10-20T09:42:00Z"}"#
        );
        assert_eq!(
            serde_json::to_string(&AccountTypeDto::LivretA).unwrap(),
            r#""livret_a""#
        );
    }
}
