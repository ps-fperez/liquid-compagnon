//! Composants de base. Cibles tactiles d'au moins 44 px, glow réservé aux éléments clés.

mod amount;
mod badge;
mod card;
mod feedback;
mod field;
mod gauge;
mod list;
pub mod loading;
mod logo;
pub mod motion;
mod notice;
mod segmented;
mod sheet;
pub mod style;
pub mod toast;

pub use amount::Amount;
pub use badge::ProvenanceBadge;
pub use card::{Card, GlowCard, HeroCard};
pub use feedback::{EmptyState, ErrorBanner, Skeleton};
pub use field::{Field, FieldError, amount_hint, sync_input};
pub use gauge::Gauge;
pub use list::ListRow;
pub use loading::{Busy, LoadingBar, Spinner};
pub use logo::AppLogo;
pub use motion::{PageTransition, cascade};
pub use notice::{Notice, NoticeKind};
pub use segmented::Segmented;
pub use sheet::BottomSheet;
pub use toast::ToastHost;
