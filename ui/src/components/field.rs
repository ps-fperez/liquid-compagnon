use leptos::html::Input;
use leptos::prelude::*;

use super::style::{INPUT, LABEL};

/// Champ de saisie avec libellé, suffixe (ex. « € »), texte d'aide et message d'erreur.
///
/// `input_mode` règle le clavier mobile : `"decimal"` pour les montants (clavier numérique avec
/// séparateur), `"numeric"` pour un PIN, `"text"` par défaut.
///
/// Le champ n'est pas réécrit pendant la frappe : sous Android, modifier `value` pendant une
/// composition du clavier (saisie prédictive) peut désynchroniser l'affichage et la valeur lue.
/// Le DOM n'est mis à jour que si le signal change réellement (réinitialisation, pré-remplissage),
/// et le signal est relu à la frappe, à la validation et à la sortie du champ.
#[component]
pub fn Field(
    #[prop(into)] label: String,
    value: RwSignal<String>,
    #[prop(default = "text")] input_mode: &'static str,
    /// `"date"` (AAAA-MM-JJ), `"month"` (AAAA-MM) ou `"password"`.
    #[prop(default = "text")]
    input_type: &'static str,
    #[prop(optional)] suffix: Option<&'static str>,
    #[prop(optional, into)] placeholder: String,
    /// Erreur de validation, affichée sous le champ (prioritaire sur l'aide).
    #[prop(optional, into)]
    error: MaybeProp<String>,
    /// Texte d'aide sous le champ (exemple, montant reconnu…).
    #[prop(optional, into)]
    hint: MaybeProp<String>,
    /// Grand champ de montant (saisie principale d'une feuille).
    #[prop(optional)]
    large: bool,
) -> impl IntoView {
    let padding = match (suffix.is_some(), large) {
        (true, true) => "pr-12 min-h-14 text-[28px] font-bold",
        (true, false) => "pr-10",
        (false, true) => "min-h-14 text-[28px] font-bold",
        (false, false) => "",
    };
    let input_ref = NodeRef::<Input>::new();
    sync_input(input_ref, value);
    let update = move |current: String| {
        if value.get_untracked() != current {
            value.set(current);
        }
    };
    let invalid = move || error.get().is_some();

    view! {
        <label class="flex flex-col gap-1.5">
            <span class=LABEL>{label}</span>
            <span class="relative">
                <input
                    node_ref=input_ref
                    class=move || format!(
                        "{INPUT} {padding} {}",
                        if invalid() { "border-loss/70" } else { "" },
                    )
                    type=input_type
                    inputmode=input_mode
                    autocomplete="off"
                    placeholder=placeholder
                    aria-invalid=move || invalid().to_string()
                    value=value.get_untracked()
                    on:input=move |event| update(event_target_value(&event))
                    on:change=move |event| update(event_target_value(&event))
                    on:blur=move |event| update(event_target_value(&event))
                />
                {suffix.map(|suffix| view! {
                    <span class="pointer-events-none absolute inset-y-0 right-4 flex items-center text-muted">
                        {suffix}
                    </span>
                })}
            </span>
            {move || match (error.get(), hint.get()) {
                (Some(message), _) => Some(view! { <span class="text-sm text-loss" role="alert">{message}</span> }.into_any()),
                (None, Some(text)) => Some(view! { <span class="text-xs text-muted">{text}</span> }.into_any()),
                (None, None) => None,
            }}
        </label>
    }
}

/// Recopie `value` dans le champ seulement si elle diffère de ce qu'il affiche : jamais pendant
/// la frappe (voir `Field`), mais à une réinitialisation ou un pré-remplissage.
pub fn sync_input(input: NodeRef<Input>, value: RwSignal<String>) {
    Effect::new(move |_| {
        let wanted = value.get();
        if let Some(element) = input.get()
            && element.value() != wanted
        {
            element.set_value(&wanted);
        }
    });
}

/// Aide d'un champ de montant, mise à jour pendant la frappe : montant reconnu (« = 20,00 € »),
/// ou format attendu si la saisie n'est pas (encore) un montant. Exemple tant que rien n'est saisi.
pub fn amount_hint(raw: &str) -> String {
    if raw.trim().is_empty() {
        return "Exemple : 12,50".into();
    }
    match crate::format::parse_amount(raw) {
        Ok(value) => format!("= {}", crate::format::amount(value)),
        Err(_) => "Format attendu : 12,50 (2 décimales au plus)".into(),
    }
}

/// Erreur de validation liée à une saisie : affichée tant que le champ n'a pas été modifié.
#[derive(Clone, Copy)]
pub struct FieldError(RwSignal<Option<(String, String)>>);

impl FieldError {
    pub fn new() -> Self {
        Self(RwSignal::new(None))
    }

    /// Signale `message` pour la valeur `raw` du champ.
    pub fn set(self, raw: String, message: impl Into<String>) {
        self.0.set(Some((raw, message.into())));
    }

    pub fn clear(self) {
        self.0.set(None);
    }

    /// Message à afficher pour la valeur courante du champ (aucun dès qu'elle change).
    pub fn for_value(self, value: RwSignal<String>) -> Signal<Option<String>> {
        Signal::derive(move || {
            self.0.with(|error| {
                error
                    .as_ref()
                    .filter(|(raw, _)| value.with(|current| current == raw))
                    .map(|(_, message)| message.clone())
            })
        })
    }
}

impl Default for FieldError {
    fn default() -> Self {
        Self::new()
    }
}
