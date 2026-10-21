use lc_shared_dto::{AccountTypeDto, NewAccountInput};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::api;
use crate::components::Busy;
use crate::components::style::{BTN_PRIMARY, LABEL};
use crate::components::toast;
use crate::components::{BottomSheet, Field};
use crate::util::{ACCOUNT_COLORS, run_action};

/// Couleur proposée par défaut pour chaque type (palette « Purple Electro »).
pub fn default_color(account_type: AccountTypeDto) -> &'static str {
    match account_type {
        AccountTypeDto::LivretA => "#A855F7",
        AccountTypeDto::Ldds => "#C084FC",
        AccountTypeDto::Pea => "#3B82F6",
        AccountTypeDto::Pei => "#60A5FA",
        AccountTypeDto::Crypto => "#7C3AED",
        AccountTypeDto::Checking => "#22D3EE",
        AccountTypeDto::Livret => "#E879F9",
        AccountTypeDto::AssuranceVie => "#38BDF8",
    }
}

/// Choix de la couleur d'identification d'un compte.
#[component]
pub fn ColorPicker(selected: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-1.5">
            <span class=LABEL>"Couleur"</span>
            <div class="flex flex-wrap gap-2" role="radiogroup" aria-label="Couleur du compte">
                {ACCOUNT_COLORS
                    .into_iter()
                    .map(|color| view! {
                        <button
                            role="radio"
                            aria-label=color
                            aria-checked=move || (selected.get() == color).to_string()
                            class=move || format!(
                                "size-11 rounded-full border-2 transition {}",
                                if selected.get() == color { "border-ink scale-110" } else { "border-transparent" },
                            )
                            style=format!("background-color: {color}")
                            on:click=move |_| selected.set(color.to_string())
                        ></button>
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

/// Familles de comptes, dans l'ordre d'affichage à la création.
const FAMILIES: [&str; 4] = ["Courant", "Livrets", "Placements", "Épargne salariale"];

fn type_hint(account_type: AccountTypeDto) -> &'static str {
    match account_type {
        AccountTypeDto::Checking => "Le solde suit vos dépenses, charges prélevées et virements.",
        AccountTypeDto::LivretA | AccountTypeDto::Ldds => {
            "Taux et plafond réglementaires préremplis, modifiables ensuite."
        }
        AccountTypeDto::Livret => {
            "Livret bancaire, Livret jeune, LEP… Taux et plafond à renseigner."
        }
        AccountTypeDto::Pea => "Positions cotées et espèces.",
        AccountTypeDto::AssuranceVie => "Solde à mettre à jour, rendement de projection réglable.",
        AccountTypeDto::Crypto => "Crypto-actifs cotés en direct.",
        AccountTypeDto::Pei => "PEE, PEI, PERCO : fonds et valeurs de part.",
    }
}

#[component]
pub fn CreateAccountSheet(open: RwSignal<bool>) -> impl IntoView {
    // Copiable (contrairement à la fonction de navigation) : le contenu de la feuille peut être
    // reconstruit à chaque ouverture.
    let navigate = StoredValue::new_local(use_navigate());
    let account_type = RwSignal::new(AccountTypeDto::LivretA);
    let name = RwSignal::new(AccountTypeDto::LivretA.label().to_string());
    let color = RwSignal::new(default_color(AccountTypeDto::LivretA).to_string());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);

    let choose = move |chosen: AccountTypeDto| {
        // Le nom suit le type tant que l'utilisateur ne l'a pas personnalisé.
        let current = name.get_untracked();
        if current.trim().is_empty() || AccountTypeDto::ALL.iter().any(|t| t.label() == current) {
            name.set(chosen.label().to_string());
        }
        if color.get_untracked() == default_color(account_type.get_untracked()) {
            color.set(default_color(chosen).to_string());
        }
        account_type.set(chosen);
    };

    let submit = move |_| {
        let input = NewAccountInput {
            name: name.get_untracked(),
            account_type: account_type.get_untracked(),
            color: Some(color.get_untracked()),
        };
        run_action(
            busy,
            error,
            api::create_account(input),
            move |id: String| {
                toast::success("Compte créé");
                open.set(false);
                navigate
                    .with_value(|navigate| navigate(&format!("/comptes/{id}"), Default::default()));
            },
        );
    };

    view! {
        <BottomSheet open=open title="Nouveau compte">
            <div class="flex flex-col gap-4">
                <div class="flex flex-col gap-3">
                    {FAMILIES.into_iter().map(|family| view! {
                        <div class="flex flex-col gap-1.5">
                            <span class=LABEL>{family}</span>
                            <div class="flex flex-wrap gap-2">
                                {AccountTypeDto::ALL
                                    .into_iter()
                                    .filter(|t| t.family() == family)
                                    .map(|t| view! {
                                        <button
                                            class=move || format!(
                                                "min-h-11 rounded-xl border px-4 text-sm transition {}",
                                                if account_type.get() == t { "border-primary bg-primary/15 text-ink" } else { "border-line text-muted" },
                                            )
                                            aria-pressed=move || (account_type.get() == t).to_string()
                                            on:click=move |_| choose(t)
                                        >
                                            {t.label()}
                                        </button>
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                    }).collect_view()}
                    <p class="text-xs text-muted">{move || type_hint(account_type.get())}</p>
                </div>
                <Field label="Nom" value=name placeholder="PEA Bourso" />
                <ColorPicker selected=color />
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit><Busy busy=busy />"Créer le compte"</button>
            </div>
        </BottomSheet>
    }
}
