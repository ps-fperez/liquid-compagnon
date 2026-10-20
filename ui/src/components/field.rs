use leptos::prelude::*;

use super::style::{INPUT, LABEL};

/// Champ de saisie avec libellé, suffixe (ex. « € ») et message d'erreur.
///
/// `input_mode` règle le clavier mobile : `"decimal"` pour les montants (clavier numérique avec
/// séparateur), `"numeric"` pour un PIN, `"text"` par défaut.
#[component]
pub fn Field(
    #[prop(into)] label: String,
    value: RwSignal<String>,
    #[prop(default = "text")] input_mode: &'static str,
    /// `"date"` (AAAA-MM-JJ) ou `"month"` (AAAA-MM) pour un sélecteur natif.
    #[prop(default = "text")]
    input_type: &'static str,
    #[prop(optional)] suffix: Option<&'static str>,
    #[prop(optional, into)] placeholder: String,
    #[prop(optional, into)] error: MaybeProp<String>,
) -> impl IntoView {
    let padding = if suffix.is_some() { "pr-10" } else { "" };
    view! {
        <label class="flex flex-col gap-1.5">
            <span class=LABEL>{label}</span>
            <span class="relative">
                <input
                    class=format!("{INPUT} {padding}")
                    type=input_type
                    inputmode=input_mode
                    autocomplete="off"
                    placeholder=placeholder
                    prop:value=move || value.get()
                    on:input=move |event| value.set(event_target_value(&event))
                />
                {suffix.map(|suffix| view! {
                    <span class="pointer-events-none absolute inset-y-0 right-4 flex items-center text-muted">
                        {suffix}
                    </span>
                })}
            </span>
            {move || error.get().map(|message| view! { <span class="text-sm text-loss">{message}</span> })}
        </label>
    }
}
