use leptos::prelude::*;
use leptos_router::components::A;

use crate::icons::{Icon, IconName};

/// Liste vide. Avec une action (`on_action` ou `href`), toute la zone devient le bouton d'ajout :
/// l'utilisateur ajoute directement depuis la carte vide, sans chercher un autre bouton.
#[component]
pub fn EmptyState(
    #[prop(into)] title: String,
    #[prop(into)] body: String,
    /// Libellé de l'action (« Ajouter une position »), lu par les lecteurs d'écran.
    #[prop(optional, into)]
    action: Option<String>,
    #[prop(optional)] on_action: Option<Callback<()>>,
    #[prop(optional, into)] href: Option<String>,
) -> impl IntoView {
    let Some(action) = action else {
        return view! {
            <div class="flex flex-col items-center gap-3 px-6 py-10 text-center">
                <Icon name=IconName::Sparkles class="size-8 text-primary-soft" />
                <p class="font-medium">{title}</p>
                <p class="text-sm text-muted">{body}</p>
            </div>
        }
        .into_any();
    };
    let content = view! {
        <span class="flex size-12 items-center justify-center rounded-full bg-primary/15 text-primary-soft shadow-glow-primary transition group-active:scale-90">
            <Icon name=IconName::Plus class="size-6" />
        </span>
        <span class="font-medium text-ink">{title}</span>
        <span class="text-sm text-muted">{body}</span>
        <span class="text-sm font-medium text-primary-soft">{action.clone()}</span>
    };
    let class = "group flex w-full flex-col items-center gap-2 rounded-2xl border border-dashed border-primary/40 px-6 py-8 text-center transition active:bg-primary/10";
    match href {
        Some(href) => view! {
            <A href=href attr:class=class attr:aria-label=action>{content}</A>
        }
        .into_any(),
        None => view! {
            <button
                class=class
                aria-label=action
                on:click=move |_| {
                    if let Some(on_action) = on_action {
                        on_action.run(());
                    }
                }
            >
                {content}
            </button>
        }
        .into_any(),
    }
}

/// Bandeau d'erreur discret (problème de cours, échec d'enregistrement…).
#[component]
pub fn ErrorBanner(#[prop(into)] message: String) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-loss/30 bg-loss/10 px-4 py-3 text-sm text-loss" role="alert">
            {message}
        </div>
    }
}

/// Bloc de chargement (silhouette animée).
#[component]
pub fn Skeleton(#[prop(optional, into)] class: String) -> impl IntoView {
    let class = if class.is_empty() {
        "h-16".to_string()
    } else {
        class
    };
    view! { <div class=format!("animate-pulse rounded-2xl bg-surface-raised {class}")></div> }
}
