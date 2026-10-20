use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::style::BTN_ICON;
use crate::icons::{Icon, IconName};
use crate::state::use_privacy;

/// En-tête : nom de l'application, mode discret, accès aux réglages.
#[component]
pub fn Header() -> impl IntoView {
    let privacy = use_privacy();
    view! {
        <header class="sticky top-0 z-20 flex items-center justify-between bg-bg/90 px-4 pt-[calc(0.75rem+env(safe-area-inset-top))] pb-2 backdrop-blur">
            <A href="/" attr:class="flex items-center gap-2.5">
                <span class="flex size-9 items-center justify-center rounded-xl bg-primary text-sm font-semibold text-bg shadow-glow-primary">
                    "LC"
                </span>
                <span class="text-base font-medium">"Liquid Compagnon"</span>
            </A>
            <div class="flex items-center gap-1">
                <button
                    class=BTN_ICON
                    aria-label=move || if privacy.hidden() { "Afficher les montants" } else { "Masquer les montants" }
                    aria-pressed=move || privacy.hidden().to_string()
                    on:click=move |_| privacy.toggle()
                >
                    {move || {
                        let name = if privacy.hidden() { IconName::EyeOff } else { IconName::Eye };
                        view! { <Icon name=name /> }
                    }}
                </button>
                <A href="/reglages" attr:class=BTN_ICON attr:aria-label="Réglages">
                    <Icon name=IconName::Settings />
                </A>
            </div>
        </header>
    }
}
