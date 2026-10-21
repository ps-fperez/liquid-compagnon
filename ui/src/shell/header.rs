use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::AppLogo;
use crate::components::style::BTN_ICON;
use crate::icons::{Icon, IconName};
use crate::shell::use_updates;
use crate::state::use_privacy;

/// En-tête : nom de l'application, mode discret, accès aux réglages.
#[component]
pub fn Header() -> impl IntoView {
    let privacy = use_privacy();
    let updates = use_updates();
    view! {
        <header class="sticky top-0 z-20 flex items-center justify-between bg-bg/90 px-4 pt-[calc(0.75rem+env(safe-area-inset-top))] pb-2 backdrop-blur">
            <A href="/" attr:class="flex items-center gap-2.5">
                <AppLogo class="size-9 drop-shadow-[0_0_10px_rgb(168_85_247/0.5)]" />
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
                <A href="/reglages" attr:class=format!("{BTN_ICON} relative") attr:aria-label="Réglages">
                    <Icon name=IconName::Settings />
                    // Pastille : une mise à jour attend dans les réglages.
                    {move || updates.available_build().map(|_| view! {
                        <span class="absolute top-2 right-2 size-2.5 rounded-full bg-secondary shadow-glow-secondary" aria-label="Mise à jour disponible"></span>
                    })}
                </A>
            </div>
        </header>
    }
}
