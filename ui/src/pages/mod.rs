//! Écrans. Ceux marqués « à venir » sont remplis à l'étape 9.

mod gallery;
mod placeholders;

pub use gallery::Gallery;
pub use placeholders::{
    AccountDetailPage, AccountsPage, BudgetPage, DashboardPage, NotFound, ProjectionPage,
    SettingsPage,
};
