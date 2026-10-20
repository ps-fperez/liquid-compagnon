//! Domaine de Liquid Compagnon : entités, value objects et services métier purs.
//!
//! Cette crate ne dépend d'aucune bibliothèque d'I/O (base de données, HTTP, système de fichiers,
//! horloge système) : la date du jour est toujours fournie par l'appelant.

pub mod account;
pub mod budget;
pub mod defaults;
mod error;
mod ids;
mod money;
mod month;
mod rate;
pub mod snapshot;
mod text;

pub use error::DomainError;
pub use ids::{AccountId, CategoryId, ChargeId, ExpenseId, FundId, IncomeId, SalaryId, SnapshotId};
pub use money::Money;
pub use month::Month;
pub use rate::Rate;
pub use snapshot::{Snapshot, SnapshotKind, SnapshotLine};
pub use text::{Color, Label};

/// Ré-exports pour que les couches externes utilisent les mêmes types que le domaine.
pub use chrono::NaiveDate;
pub use rust_decimal::Decimal;
pub use uuid::Uuid;
