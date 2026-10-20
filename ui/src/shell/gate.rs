use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::components::style::BTN_SECONDARY;
use crate::icons::{Icon, IconName};
use crate::pages::{LockScreen, SetupPin};
use crate::state::use_session;

/// N'affiche les écrans métier que si la session est déverrouillée : sinon création du PIN
/// (premier lancement) ou écran de verrouillage. La galerie de composants (`/galerie`, builds de
/// développement uniquement) reste accessible sans session.
#[component]
pub fn SessionGate(children: ChildrenFn) -> impl IntoView {
    let session = use_session();
    let location = use_location();
    session.refresh();

    move || {
        // Lecture non réactive : naviguer ne doit pas reconstruire toute l'application.
        let gallery =
            cfg!(debug_assertions) && location.pathname.get_untracked().starts_with("/galerie");
        if gallery {
            return children().into_any();
        }
        if let Some(message) = session.error.get() {
            return view! { <BackendError message=message /> }.into_any();
        }
        match session.status.get() {
            None => view! { <Splash /> }.into_any(),
            Some(status) if !status.pin_set => view! { <SetupPin /> }.into_any(),
            Some(status) if !status.unlocked => view! { <LockScreen status=status /> }.into_any(),
            Some(_) => children().into_any(),
        }
    }
}

#[component]
fn Splash() -> impl IntoView {
    view! {
        <div class="flex min-h-dvh items-center justify-center">
            <span class="flex size-16 animate-pulse items-center justify-center rounded-2xl bg-primary text-xl font-semibold text-bg shadow-glow-primary">
                "LC"
            </span>
        </div>
    }
}

#[component]
fn BackendError(message: String) -> impl IntoView {
    let session = use_session();
    view! {
        <div class="flex min-h-dvh flex-col items-center justify-center gap-4 px-8 text-center">
            <h1 class="text-xl font-medium">"Liquid Compagnon"</h1>
            <p class="text-sm text-loss">"Le cœur de l'application ne répond pas : " {message}</p>
            <button class=BTN_SECONDARY on:click=move |_| session.refresh()>
                <Icon name=IconName::Refresh class="size-5" />
                "Réessayer"
            </button>
        </div>
    }
}
