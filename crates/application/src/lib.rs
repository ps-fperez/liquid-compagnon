//! Couche application : cas d'usage et ports (traits) implémentés par l'infrastructure.
//!
//! Dépend uniquement de `lc-domain`. Les cas d'usage sont regroupés par domaine fonctionnel,
//! une méthode par cas d'usage ; leurs dépendances sont des `Arc<dyn Port>` injectés par la
//! composition root (`src-tauri`).

pub mod accounts;
pub mod backup;
pub mod budget;
pub mod dashboard;
mod error;
mod flows;
pub mod ports;
pub mod pricing;
pub mod projection;
pub mod security;
pub mod transfers;
pub mod updates;

#[cfg(test)]
mod testing;

pub use error::{AppError, AppResult};
