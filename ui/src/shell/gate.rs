use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::components::style::BTN_SECONDARY;
use crate::icons::{Icon, IconName};
use crate::state::use_session;

/// N'affiche les écrans métier que si la session est déverrouillée. La galerie de composants
/// (`/galerie`, builds de développement uniquement) reste accessible sans session.
#[component]
pub fn SessionGate(children: ChildrenFn) -> impl IntoView {
    let session = use_session();
    let location = use_location();
    session.refresh();

    move || {
        let gallery = cfg!(debug_assertions) && location.pathname.get().starts_with("/galerie");
        if gallery {
            return children().into_any();
        }
        if let Some(message) = session.error.get() {
            return view! { <BackendError message=message /> }.into_any();
        }
        match session.status.get() {
            None => view! { <Splash /> }.into_any(),
            Some(status) if !status.pin_set => view! {
                <LockedScreen
                    title="Bienvenue"
                    body="La création du code PIN arrive avec les écrans de l'étape 9."
                />
            }
            .into_any(),
            Some(status) if !status.unlocked => view! {
                <LockedScreen
                    title="Application verrouillée"
                    body="Le déverrouillage par code PIN arrive avec les écrans de l'étape 9."
                />
            }
            .into_any(),
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
fn LockedScreen(title: &'static str, body: &'static str) -> impl IntoView {
    view! {
        <div class="flex min-h-dvh flex-col items-center justify-center gap-4 px-8 text-center">
            <span class="flex size-16 items-center justify-center rounded-2xl bg-surface-raised text-primary shadow-glow-primary">
                <Icon name=IconName::Lock class="size-8" />
            </span>
            <h1 class="text-xl font-medium">"Liquid Compagnon"</h1>
            <p class="font-medium">{title}</p>
            <p class="text-sm text-muted">{body}</p>
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
