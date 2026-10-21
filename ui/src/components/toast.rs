//! Notifications (spec interaction-feedback : « Notifications d'issue ») : succès, erreur,
//! avertissement, information, en haut de l'écran sous l'en-tête.

use std::cell::Cell;
use std::time::Duration;

use leptos::prelude::*;

use crate::icons::{Icon, IconName};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Toast {
    id: u32,
    kind: ToastKind,
    message: String,
}

thread_local! {
    /// Accès aux notifications hors des composants (gestionnaires asynchrones, `run_action`).
    static TOASTS: Cell<Option<RwSignal<Vec<Toast>>>> = const { Cell::new(None) };
    static NEXT_ID: Cell<u32> = const { Cell::new(0) };
}

/// À appeler une fois, à la racine de l'application.
pub fn provide_toasts() {
    let toasts = RwSignal::new(Vec::<Toast>::new());
    TOASTS.with(|cell| cell.set(Some(toasts)));
}

/// Affiche une notification ; celles de succès et d'information se ferment seules.
pub fn show(kind: ToastKind, message: impl Into<String>) {
    let Some(toasts) = TOASTS.with(Cell::get) else {
        return;
    };
    let id = NEXT_ID.with(|next| {
        let id = next.get();
        next.set(id.wrapping_add(1));
        id
    });
    let message = message.into();
    toasts.update(|list| {
        // Pas de doublon empilé (double appui, erreur répétée) ; au plus 3 à l'écran.
        list.retain(|t| t.message != message);
        list.push(Toast { id, kind, message });
        let excess = list.len().saturating_sub(3);
        list.drain(..excess);
    });
    let delay = match kind {
        ToastKind::Success | ToastKind::Info => 3_500,
        ToastKind::Warning | ToastKind::Error => 7_000,
    };
    set_timeout(move || dismiss(id), Duration::from_millis(delay));
}

pub fn success(message: impl Into<String>) {
    show(ToastKind::Success, message);
}

pub fn error(message: impl Into<String>) {
    show(ToastKind::Error, message);
}

pub fn warning(message: impl Into<String>) {
    show(ToastKind::Warning, message);
}

pub fn info(message: impl Into<String>) {
    show(ToastKind::Info, message);
}

fn dismiss(id: u32) {
    if let Some(toasts) = TOASTS.with(Cell::get) {
        toasts.update(|list| list.retain(|t| t.id != id));
    }
}

/// Zone d'affichage des notifications (une seule, à la racine).
#[component]
pub fn ToastHost() -> impl IntoView {
    let Some(toasts) = TOASTS.with(Cell::get) else {
        return ().into_any();
    };
    view! {
        <div class="pointer-events-none fixed inset-x-0 top-[calc(4rem+env(safe-area-inset-top))] z-[60] mx-auto flex max-w-md flex-col gap-2 px-4">
            <For each=move || toasts.get() key=|toast| toast.id let:toast>
                <ToastCard toast=toast />
            </For>
        </div>
    }
    .into_any()
}

#[component]
fn ToastCard(toast: Toast) -> impl IntoView {
    let (icon, tone) = match toast.kind {
        ToastKind::Success => (IconName::Check, "border-gain/40 text-gain shadow-glow-gain"),
        ToastKind::Error => (IconName::Alert, "border-loss/40 text-loss shadow-glow-loss"),
        ToastKind::Warning => (IconName::Alert, "border-amber-400/40 text-amber-300"),
        ToastKind::Info => (IconName::Info, "border-secondary/40 text-secondary-light"),
    };
    let urgent = matches!(toast.kind, ToastKind::Error | ToastKind::Warning);
    let id = toast.id;
    view! {
        <div
            class=format!("pointer-events-auto flex animate-fade-in items-start gap-3 rounded-2xl border bg-surface-raised/95 px-4 py-3 backdrop-blur {tone}")
            role=if urgent { "alert" } else { "status" }
            aria-live=if urgent { "assertive" } else { "polite" }
        >
            <Icon name=icon class="mt-0.5 size-5 shrink-0" />
            <p class="flex-1 text-sm text-ink">{toast.message}</p>
            <button class="-m-1 shrink-0 p-1 text-muted" aria-label="Fermer la notification" on:click=move |_| dismiss(id)>
                <Icon name=IconName::Close class="size-4" />
            </button>
        </div>
    }
}
