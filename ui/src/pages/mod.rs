//! Écrans. Ceux marqués « à venir » sont remplis au fil de l'étape 9.

mod accounts;
mod dashboard;
mod gallery;
mod lock;
mod placeholders;

pub use accounts::{AccountDetailPage, AccountsPage};
pub use dashboard::DashboardPage;
pub use gallery::Gallery;
pub use lock::{LockScreen, SetupPin};
pub use placeholders::{BudgetPage, NotFound, ProjectionPage, SettingsPage};
