use lc_shared_dto::AccountOptionDto;
use leptos::prelude::*;

use crate::components::style::LABEL;

/// Compte courant choisi pour un mouvement : `None` tant que l'utilisateur n'a rien touché (le
/// premier compte courant actif est alors retenu), `Some(None)` pour « Aucun compte ».
pub type AccountSelection = RwSignal<Option<Option<String>>>;

/// Compte effectivement rattaché, choix par défaut compris.
pub fn resolve(selection: Option<Option<String>>, options: &[AccountOptionDto]) -> Option<String> {
    match selection {
        Some(choice) => choice,
        None => options.first().map(|option| option.id.clone()),
    }
}

/// Puces des comptes courants actifs et « Aucun compte ». Rien n'est affiché sans compte courant :
/// la saisie rapide reste en trois gestes.
#[component]
pub fn AccountChoice(
    #[prop(into)] label: String,
    options: Signal<Vec<AccountOptionDto>>,
    selection: AccountSelection,
) -> impl IntoView {
    let label = StoredValue::new(label);
    move || {
        let list = options.get();
        if list.is_empty() {
            return None;
        }
        let current = resolve(selection.get(), &list);
        let chip = |choice: Option<String>, name: String| {
            let active = choice == current;
            view! {
                <button
                    class=format!(
                        "min-h-11 rounded-xl border px-4 text-sm {}",
                        if active { "border-primary bg-primary/15 text-ink" } else { "border-line text-muted" },
                    )
                    aria-pressed=active.to_string()
                    on:click=move |_| selection.set(Some(choice.clone()))
                >
                    {name}
                </button>
            }
        };
        Some(view! {
            <div class="flex flex-col gap-1.5">
                <span class=LABEL>{label.get_value()}</span>
                <div class="flex flex-wrap gap-2">
                    {list.iter().map(|option| chip(Some(option.id.clone()), option.name.clone())).collect_view()}
                    {chip(None, "Aucun compte".to_string())}
                </div>
            </div>
        })
    }
}
