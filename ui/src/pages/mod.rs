//! Écrans de l'application.

mod accounts;
mod allowance;
mod budget;
mod dashboard;
mod gallery;
mod goals;
mod lock;
mod placeholders;
mod projection;
mod settings;

pub use accounts::{AccountDetailPage, AccountsPage};
pub use budget::BudgetPage;
pub use dashboard::DashboardPage;
pub use gallery::Gallery;
pub use goals::GoalRing;
pub use lock::{LockScreen, SetupPin};
pub use placeholders::NotFound;
pub use projection::ProjectionPage;
pub use settings::SettingsPage;
