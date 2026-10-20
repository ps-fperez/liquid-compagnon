//! DTO partagés entre l'UI (WebAssembly) et le backend Tauri.
//!
//! Seule frontière connue de l'UI : elle n'a accès ni au domaine ni à l'infrastructure.

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
}
