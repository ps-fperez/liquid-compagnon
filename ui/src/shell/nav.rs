use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_location;

use super::quick_add::QuickAddSheet;
use crate::icons::{Icon, IconName};

struct Tab {
    href: &'static str,
    label: &'static str,
    icon: IconName,
}

const LEFT: [Tab; 2] = [
    Tab {
        href: "/",
        label: "Accueil",
        icon: IconName::Home,
    },
    Tab {
        href: "/comptes",
        label: "Comptes",
        icon: IconName::Wallet,
    },
];
const RIGHT: [Tab; 2] = [
    Tab {
        href: "/budget",
        label: "Budget",
        icon: IconName::Budget,
    },
    Tab {
        href: "/projection",
        label: "Projection",
        icon: IconName::Projection,
    },
];

/// Vrai si `path` appartient à l'onglet `href` (l'accueil n'est actif que sur « / »).
fn is_active(href: &str, path: &str) -> bool {
    if href == "/" {
        path == "/"
    } else {
        path == href || path.starts_with(&format!("{href}/"))
    }
}

/// Barre de navigation inférieure : quatre onglets et le bouton « + » central (saisie rapide).
#[component]
pub fn BottomNav() -> impl IntoView {
    let location = use_location();
    let quick_add = RwSignal::new(false);

    let tab = move |tab: &'static Tab| {
        let active = move || is_active(tab.href, &location.pathname.get());
        view! {
            <A
                href=tab.href
                attr:class=move || {
                    let color = if active() { "text-primary" } else { "text-muted" };
                    format!("flex min-h-14 flex-1 flex-col items-center justify-center gap-1 text-[11px] {color}")
                }
            >
                <span class=move || {
                    if active() { "rounded-xl px-3 py-1 bg-primary/15 shadow-glow-primary" } else { "px-3 py-1" }
                }>
                    <Icon name=tab.icon class="size-6" />
                </span>
                {tab.label}
            </A>
        }
    };

    view! {
        <nav
            class="fixed inset-x-0 bottom-0 z-30 border-t border-line bg-surface/95 pb-[env(safe-area-inset-bottom)] backdrop-blur"
            aria-label="Navigation principale"
        >
            <div class="mx-auto flex max-w-md items-end px-2">
                {LEFT.iter().map(tab).collect_view()}
                <div class="flex flex-1 justify-center">
                    <button
                        class="-mt-6 flex size-16 items-center justify-center rounded-full bg-secondary text-ink animate-glow-pulse active:scale-95"
                        aria-label="Saisie rapide"
                        on:click=move |_| quick_add.set(true)
                    >
                        <Icon name=IconName::Plus class="size-8" />
                    </button>
                </div>
                {RIGHT.iter().map(tab).collect_view()}
            </div>
        </nav>
        <QuickAddSheet open=quick_add />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_is_only_active_on_root() {
        assert!(is_active("/", "/"));
        assert!(!is_active("/", "/comptes"));
        assert!(is_active("/comptes", "/comptes/123"));
        assert!(!is_active("/comptes", "/comptesx"));
    }
}
