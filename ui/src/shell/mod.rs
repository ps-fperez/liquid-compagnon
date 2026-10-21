//! Coque de l'application : porte de session, en-tête, navigation inférieure, saisie rapide.

mod gate;
mod header;
mod nav;
mod onboarding;
mod quick_add;
mod updates;

pub use gate::SessionGate;
pub use header::Header;
pub use nav::BottomNav;
pub use onboarding::{Onboarding, OnboardingOverlay, use_onboarding};
pub use updates::{AvailableUpdate, InstallControls, UpdateWatcher, Updates, use_updates};
