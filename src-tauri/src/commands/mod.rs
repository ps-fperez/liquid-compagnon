//! Commandes Tauri exposées à l'UI. Chaque commande prend au plus un argument `input` (DTO) et
//! renvoie un DTO ou une `ErrorDto` au message français.
//!
//! Toutes les commandes métier exigent une session déverrouillée. Les cas d'usage synchrones
//! (SQLite) s'exécutent dans `spawn_blocking` pour ne jamais bloquer l'UI.

pub mod accounts;
pub mod budget;
pub mod overview;
pub mod session;
pub mod transfers;
pub mod updates;

use std::sync::Arc;

use lc_shared_dto::PingResponse;
use tauri::State;

use crate::error::{CommandResult, internal};
use crate::state::Services;

pub type AppState<'a> = State<'a, Arc<Services>>;

/// Services, après vérification que la session est déverrouillée.
pub(crate) fn unlocked(state: &AppState<'_>) -> CommandResult<Arc<Services>> {
    let services = state.inner().clone();
    services.session.require_unlocked()?;
    Ok(services)
}

/// Exécute un cas d'usage synchrone hors du thread de l'UI.
pub(crate) async fn blocking<T, F>(services: Arc<Services>, task: F) -> CommandResult<T>
where
    T: Send + 'static,
    F: FnOnce(&Services) -> CommandResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || task(&services))
        .await
        .map_err(internal)?
}

/// Vérifie la liaison UI ↔ backend (sans session).
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
