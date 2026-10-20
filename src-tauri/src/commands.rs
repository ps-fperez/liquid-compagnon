//! Commandes Tauri exposées à l'UI.

use lc_shared_dto::PingResponse;

/// Vérifie la liaison UI ↔ backend.
#[tauri::command]
pub fn ping() -> PingResponse {
    PingResponse {
        message: "pong".into(),
        backend_version: env!("CARGO_PKG_VERSION").into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_returns_pong_with_backend_version() {
        let response = ping();

        assert_eq!(response.message, "pong");
        assert_eq!(response.backend_version, env!("CARGO_PKG_VERSION"));
    }
}
