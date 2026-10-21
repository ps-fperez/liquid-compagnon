//! Notification unique (spec motion-feedback : « Notification unique avec action ») : en bas,
//! au-dessus de la barre de navigation, ou en haut tant qu'une feuille est ouverte (pour ne pas
//! masquer son bouton). Une nouvelle notification remplace la précédente.

use std::cell::Cell;
use std::sync::Arc;
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

/// Action proposée dans la notification (« Annuler »). Elle survit à la feuille qui l'a créée.
#[derive(Clone)]
struct ToastAction {
    label: &'static str,
    run: Arc<dyn Fn() + Send + Sync>,
}

#[derive(Clone)]
struct Toast {
    id: u32,
    kind: ToastKind,
    message: String,
    action: Option<ToastAction>,
}

#[derive(Clone, Copy)]
struct Toasts {
    current: RwSignal<Option<Toast>>,
    /// Feuilles ouvertes : la notification passe en haut tant qu'il y en a une.
    open_sheets: RwSignal<u32>,
}

thread_local! {
    /// Accès aux notifications hors des composants (gestionnaires asynchrones, `run_action`).
    static TOASTS: Cell<Option<Toasts>> = const { Cell::new(None) };
    static NEXT_ID: Cell<u32> = const { Cell::new(0) };
}

/// À appeler une fois, à la racine de l'application.
pub fn provide_toasts() {
    let toasts = Toasts {
        current: RwSignal::new(None),
        open_sheets: RwSignal::new(0),
    };
    TOASTS.with(|cell| cell.set(Some(toasts)));
}

fn push(kind: ToastKind, message: String, action: Option<ToastAction>) {
    let Some(toasts) = TOASTS.with(Cell::get) else {
        return;
    };
    let id = NEXT_ID.with(|next| {
        let id = next.get();
        next.set(id.wrapping_add(1));
        id
    });
    toasts.current.set(Some(Toast {
        id,
        kind,
        message,
        action,
    }));
    let delay = match kind {
        ToastKind::Success | ToastKind::Info => 5_000,
        ToastKind::Warning | ToastKind::Error => 7_000,
    };
    set_timeout(move || dismiss(id), Duration::from_millis(delay));
}

/// Affiche une notification ; elle se ferme seule (5 s, 7 s pour une erreur ou un avertissement).
pub fn show(kind: ToastKind, message: impl Into<String>) {
    push(kind, message.into(), None);
}

pub fn success(message: impl Into<String>) {
    show(ToastKind::Success, message);
}

/// Succès avec « Annuler » : `undo` défait l'enregistrement (suppression de l'élément ajouté).
pub fn success_with_undo(message: impl Into<String>, undo: impl Fn() + Send + Sync + 'static) {
    push(
        ToastKind::Success,
        message.into(),
        Some(ToastAction {
            label: "Annuler",
            run: Arc::new(undo),
        }),
    );
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
    if let Some(toasts) = TOASTS.with(Cell::get)
        && toasts
            .current
            .with_untracked(|t| t.as_ref().is_some_and(|t| t.id == id))
    {
        toasts.current.set(None);
    }
}

/// Signale l'ouverture (`true`) ou la fermeture d'une feuille.
pub fn sheet_visible(visible: bool) {
    if let Some(toasts) = TOASTS.with(Cell::get) {
        toasts.open_sheets.update(|count| {
            *count = if visible {
                *count + 1
            } else {
                count.saturating_sub(1)
            };
        });
    }
}

/// Zone d'affichage de la notification (une seule, à la racine).
#[component]
pub fn ToastHost() -> impl IntoView {
    let Some(toasts) = TOASTS.with(Cell::get) else {
        return ().into_any();
    };
    let position = move || {
        if toasts.open_sheets.get() > 0 {
            "top-[calc(1rem+env(safe-area-inset-top))]"
        } else {
            "bottom-[calc(5.75rem+env(safe-area-inset-bottom))]"
        }
    };
    view! {
        <div class=move || format!("pointer-events-none fixed inset-x-0 z-[60] mx-auto flex max-w-md flex-col px-4 {}", position())>
            {move || toasts.current.get().map(|toast| view! { <ToastCard toast=toast /> })}
        </div>
    }
    .into_any()
}

#[component]
fn ToastCard(toast: Toast) -> impl IntoView {
    let (icon, tone) = match toast.kind {
        ToastKind::Success => (IconName::Check, "border-gain/40 text-gain"),
        ToastKind::Error => (IconName::Alert, "border-loss/40 text-loss"),
        ToastKind::Warning => (IconName::Alert, "border-warn/40 text-warn"),
        ToastKind::Info => (IconName::Info, "border-secondary/40 text-secondary-light"),
    };
    let urgent = matches!(toast.kind, ToastKind::Error | ToastKind::Warning);
    let id = toast.id;
    view! {
        <div
            class=format!("pointer-events-auto flex animate-toast-in items-center gap-3 rounded-[14px] border bg-surface-raised px-4 py-2 shadow-[0_10px_30px_-10px_rgb(0_0_0/0.7)] {tone}")
            role=if urgent { "alert" } else { "status" }
            aria-live=if urgent { "assertive" } else { "polite" }
        >
            <Icon name=icon class="size-5 shrink-0" />
            <p class="flex-1 py-1 text-sm text-ink">{toast.message}</p>
            {toast.action.map(|action| view! {
                <button
                    class="-my-1 min-h-11 shrink-0 rounded-xl px-3 text-sm font-semibold text-secondary-light active:bg-secondary/15"
                    on:click=move |_| {
                        dismiss(id);
                        (action.run)();
                    }
                >
                    {action.label}
                </button>
            })}
            <button class="-m-1 shrink-0 p-1 text-muted" aria-label="Fermer la notification" on:click=move |_| dismiss(id)>
                <Icon name=IconName::Close class="size-4" />
            </button>
        </div>
    }
}
