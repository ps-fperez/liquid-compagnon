//! Écrans de l'application.

mod accounts;
mod budget;
mod dashboard;
mod gallery;
mod lock;
mod placeholders;
mod projection;
mod settings;

pub use accounts::{AccountDetailPage, AccountsPage};
pub use budget::BudgetPage;
pub use dashboard::DashboardPage;
pub use gallery::Gallery;
pub use lock::{LockScreen, SetupPin};
pub use placeholders::NotFound;
pub use projection::ProjectionPage;
pub use settings::SettingsPage;
