//! Comptes d'épargne : livrets (Livret A, LDDS), comptes-titres (PEA, crypto) et PEI.

mod asset;
mod checking;
mod entity;
mod kind;
mod pei;
mod position;
mod scenario;

pub use asset::{AssetRef, CurrencyCode, QuoteSource};
pub use checking::CheckingDetails;
pub use entity::Account;
pub use kind::{
    AccountKind, AccountType, EmployeeSavingsDetails, SavingsDetails, SavingsProduct,
    SecuritiesDetails, SecuritiesProduct,
};
pub use pei::{NavAnchor, PeiFund};
pub use position::Position;
pub use scenario::{Scenario, ScenarioRates};
