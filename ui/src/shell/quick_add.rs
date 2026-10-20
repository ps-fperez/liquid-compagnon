use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::components::BottomSheet;
use crate::icons::{Icon, IconName};

/// Choix proposés par le bouton « + » (spec app-shell-theme : « Saisie rapide »).
#[component]
pub fn QuickAddSheet(open: RwSignal<bool>) -> impl IntoView {
    let navigate = use_navigate();
    let go = move |target: &'static str| {
        let navigate = navigate.clone();
        move |_| {
            open.set(false);
            // Identifiant unique : une nouvelle saisie rapide depuis le même écran rouvre le formulaire.
            navigate(
                &format!("{target}&n={}", js_sys::Date::now() as u64),
                Default::default(),
            );
        }
    };
    view! {
        <BottomSheet open=open title="Saisie rapide">
            <div class="flex flex-col gap-2">
                <button class="flex min-h-14 items-center gap-3 rounded-xl bg-surface-raised px-4 text-left" on:click=go("/budget?saisie=depense")>
                    <Icon name=IconName::Receipt class="size-6 text-primary-soft" />
                    <span class="font-medium">"Dépense"</span>
                </button>
                <button class="flex min-h-14 items-center gap-3 rounded-xl bg-surface-raised px-4 text-left" on:click=go("/budget?saisie=revenu")>
                    <Icon name=IconName::Coins class="size-6 text-gain" />
                    <span class="font-medium">"Revenu ponctuel"</span>
                </button>
                <button class="flex min-h-14 items-center gap-3 rounded-xl bg-surface-raised px-4 text-left" on:click=go("/comptes?saisie=solde")>
                    <Icon name=IconName::Wallet class="size-6 text-secondary-light" />
                    <span class="font-medium">"Mise à jour d'un solde"</span>
                </button>
            </div>
        </BottomSheet>
    }
}
