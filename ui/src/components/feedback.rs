use leptos::prelude::*;

use crate::icons::{Icon, IconName};

/// Invitation quand une liste est vide : titre, une phrase, et éventuellement une action.
#[component]
pub fn EmptyState(
    #[prop(into)] title: String,
    #[prop(into)] body: String,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="flex flex-col items-center gap-3 px-6 py-10 text-center">
            <Icon name=IconName::Sparkles class="size-8 text-primary-soft" />
            <p class="font-medium">{title}</p>
            <p class="text-sm text-muted">{body}</p>
            {children.map(|children| children())}
        </div>
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
