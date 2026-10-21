//! Indications de chargement (spec interaction-feedback : « Indication de chargement »).

use std::cell::Cell;
use std::time::Duration;

use leptos::prelude::*;

thread_local! {
    /// Appels au backend en cours, tenus à jour par `api::call`.
    static PENDING: Cell<Option<RwSignal<u32>>> = const { Cell::new(None) };
}

/// À appeler une fois, à la racine de l'application.
pub fn provide_loading() {
    PENDING.with(|cell| cell.set(Some(RwSignal::new(0))));
}

/// Début d'un appel ; l'appel se termine quand le jeton est libéré.
pub fn begin() -> PendingCall {
    if let Some(pending) = PENDING.with(Cell::get) {
        pending.update(|count| *count += 1);
    }
    PendingCall
}

pub struct PendingCall;

impl Drop for PendingCall {
    fn drop(&mut self) {
        if let Some(pending) = PENDING.with(Cell::get) {
            pending.update(|count| *count = count.saturating_sub(1));
        }
    }
}

/// Barre fine en haut de l'écran pendant les échanges avec le backend, affichée seulement après
/// 150 ms pour ne pas clignoter sur les réponses instantanées.
#[component]
pub fn LoadingBar() -> impl IntoView {
    let Some(pending) = PENDING.with(Cell::get) else {
        return ().into_any();
    };
    let visible = RwSignal::new(false);
    let generation = StoredValue::new(0_u32);
    Effect::new(move |_| {
        let busy = pending.get() > 0;
        generation.update_value(|g| *g += 1);
        if !busy {
            visible.set(false);
            return;
        }
        let current = generation.get_value();
        set_timeout(
            move || {
                if generation.get_value() == current && pending.get_untracked() > 0 {
                    visible.set(true);
                }
            },
            Duration::from_millis(150),
        );
    });
    view! {
        <Show when=move || visible.get()>
            <div class="fixed inset-x-0 top-0 z-[70] h-0.5 overflow-hidden bg-primary/20" role="progressbar" aria-label="Chargement">
                <div class="h-full w-1/3 animate-loading-bar bg-primary shadow-glow-primary"></div>
            </div>
        </Show>
    }
    .into_any()
}

/// Roue de chargement (hérite de la couleur du texte).
#[component]
pub fn Spinner(#[prop(optional, into)] class: String) -> impl IntoView {
    let class = if class.is_empty() {
        "size-5".to_string()
    } else {
        class
    };
    view! {
        <svg viewBox="0 0 24 24" fill="none" class=format!("animate-spin {class}") aria-hidden="true">
            <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-opacity="0.25" stroke-width="3" />
            <path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" stroke-width="3" stroke-linecap="round" />
        </svg>
    }
}

/// Roue affichée dans un bouton tant que son action est en cours.
#[component]
pub fn Busy(busy: RwSignal<bool>) -> impl IntoView {
    move || busy.get().then(|| view! { <Spinner class="size-4" /> })
}
