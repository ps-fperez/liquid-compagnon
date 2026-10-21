//! Bandeaux d'information et d'avertissement refermables (spec interaction-feedback :
//! « Avertissements refermables »). La fermeture est mémorisée sur l'appareil jusqu'à échéance.

use leptos::prelude::*;

use crate::icons::{Icon, IconName};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Warning,
}

/// Bandeau. Avec `dismiss_key`, il se ferme et reste fermé `dismiss_days` jours (1 par défaut).
#[component]
pub fn Notice(
    kind: NoticeKind,
    #[prop(into)] message: String,
    #[prop(optional, into)] dismiss_key: Option<String>,
    #[prop(default = 1)] dismiss_days: u32,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let hidden = RwSignal::new(dismiss_key.as_deref().is_some_and(is_dismissed));
    let (icon, tone) = match kind {
        NoticeKind::Info => (
            IconName::Info,
            "border-secondary/30 bg-secondary/10 text-secondary-light",
        ),
        NoticeKind::Warning => (
            IconName::Alert,
            "border-amber-400/30 bg-warn/10 text-amber-200",
        ),
    };
    let key = StoredValue::new(dismiss_key);
    let close = move |_| {
        if let Some(key) = key.get_value() {
            remember_dismissal(&key, dismiss_days);
        }
        hidden.set(true);
    };
    // Le contenu (lien d'action…) est rendu une fois ; fermer le bandeau le masque.
    let extra = children.map(|children| children());
    view! {
        <div
            class=move || format!(
                "items-start gap-3 rounded-xl border px-4 py-3 text-sm {tone} {}",
                if hidden.get() { "hidden" } else { "flex" },
            )
            role=if kind == NoticeKind::Info { "status" } else { "alert" }
        >
            <Icon name=icon class="mt-0.5 size-5 shrink-0" />
            <div class="flex flex-1 flex-col gap-2">
                <p>{message}</p>
                {extra}
            </div>
            {key.with_value(Option::is_some).then(|| view! {
                <button class="-m-1 shrink-0 p-1 opacity-80" aria-label="Fermer l'avertissement" on:click=close>
                    <Icon name=IconName::Close class="size-4" />
                </button>
            })}
        </div>
    }
}

const PREFIX: &str = "lc.ferme.";

fn now_days() -> f64 {
    (js_sys::Date::now() / 86_400_000.0).floor()
}

fn is_dismissed(key: &str) -> bool {
    storage()
        .and_then(|s| s.get_item(&format!("{PREFIX}{key}")).ok().flatten())
        .and_then(|until| until.parse::<f64>().ok())
        .is_some_and(|until| now_days() < until)
}

fn remember_dismissal(key: &str, days: u32) {
    if let Some(storage) = storage() {
        let until = now_days() + f64::from(days);
        let _ = storage.set_item(&format!("{PREFIX}{key}"), &until.to_string());
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}
