//! Icônes SVG au trait (24 × 24, `currentColor`), embarquées : aucune police ni requête réseau.

use leptos::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconName {
    Home,
    Wallet,
    Plus,
    Budget,
    Projection,
    Settings,
    Eye,
    EyeOff,
    ChevronLeft,
    ChevronRight,
    Lock,
    Fingerprint,
    Refresh,
    Close,
    Receipt,
    Coins,
    Sparkles,
}

impl IconName {
    pub const ALL: [IconName; 17] = [
        IconName::Home,
        IconName::Wallet,
        IconName::Plus,
        IconName::Budget,
        IconName::Projection,
        IconName::Settings,
        IconName::Eye,
        IconName::EyeOff,
        IconName::ChevronLeft,
        IconName::ChevronRight,
        IconName::Lock,
        IconName::Fingerprint,
        IconName::Refresh,
        IconName::Close,
        IconName::Receipt,
        IconName::Coins,
        IconName::Sparkles,
    ];

    fn paths(self) -> &'static str {
        match self {
            IconName::Home => {
                r#"<path d="M3 10.5 12 3l9 7.5"/><path d="M5.5 9v11.5h13V9"/><path d="M10 20.5v-6h4v6"/>"#
            }
            IconName::Wallet => {
                r#"<rect x="3" y="6" width="18" height="14" rx="2.5"/><path d="M3 10h18"/><path d="M16 14.5h2"/><path d="M6 6l9-3 1.5 3"/>"#
            }
            IconName::Plus => r#"<path d="M12 5v14M5 12h14"/>"#,
            IconName::Budget => {
                r#"<path d="M21 12A9 9 0 1 1 12 3v9z"/><path d="M15 3.5A9 9 0 0 1 20.5 9H15z"/>"#
            }
            IconName::Projection => r#"<path d="M3 17l6-6 4 4 8-8"/><path d="M14 7h7v7"/>"#,
            IconName::Settings => {
                r#"<circle cx="12" cy="12" r="3"/><path d="M12 2.5v3M12 18.5v3M4.6 4.6l2.1 2.1M17.3 17.3l2.1 2.1M2.5 12h3M18.5 12h3M4.6 19.4l2.1-2.1M17.3 6.7l2.1-2.1"/>"#
            }
            IconName::Eye => {
                r#"<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>"#
            }
            IconName::EyeOff => {
                r#"<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/><path d="M3 3l18 18"/>"#
            }
            IconName::ChevronLeft => r#"<path d="M15 18l-6-6 6-6"/>"#,
            IconName::ChevronRight => r#"<path d="M9 18l6-6-6-6"/>"#,
            IconName::Lock => {
                r#"<rect x="4.5" y="11" width="15" height="10" rx="2.5"/><path d="M8 11V7.5a4 4 0 0 1 8 0V11"/>"#
            }
            IconName::Fingerprint => {
                r#"<path d="M12 11v4.5"/><path d="M8.5 12a3.5 3.5 0 0 1 7 0v2"/><path d="M5.5 12a6.5 6.5 0 0 1 13 0v1"/><path d="M9 17.5c.6 1.8 1.6 3 3 3.5"/><path d="M15.5 17c-.3 1.5-.8 2.6-1.5 3.5"/>"#
            }
            IconName::Refresh => {
                r#"<path d="M20.5 12a8.5 8.5 0 1 1-2.6-6.1L20.5 8.5"/><path d="M20.5 3.5v5h-5"/>"#
            }
            IconName::Close => r#"<path d="M18 6 6 18M6 6l12 12"/>"#,
            IconName::Receipt => {
                r#"<path d="M6 3h12v18l-3-2-3 2-3-2-3 2z"/><path d="M9 8h6M9 12h6"/>"#
            }
            IconName::Coins => {
                r#"<circle cx="9" cy="9" r="5.5"/><path d="M14.6 10.6a5.5 5.5 0 1 1-4 4"/>"#
            }
            IconName::Sparkles => {
                r#"<path d="M12 3l1.8 4.7L18.5 9.5l-4.7 1.8L12 16l-1.8-4.7L5.5 9.5l4.7-1.8z"/><path d="M19 15l.8 2.2 2.2.8-2.2.8L19 21l-.8-2.2-2.2-.8 2.2-.8z"/>"#
            }
        }
    }
}

#[component]
pub fn Icon(name: IconName, #[prop(optional, into)] class: String) -> impl IntoView {
    let class = if class.is_empty() {
        "size-6".to_string()
    } else {
        class
    };
    view! {
        <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            class=class
            aria-hidden="true"
            inner_html=name.paths()
        ></svg>
    }
}
