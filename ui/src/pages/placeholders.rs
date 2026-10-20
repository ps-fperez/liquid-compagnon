use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::EmptyState;
use crate::components::style::BTN_SECONDARY;

#[component]
fn Upcoming(title: &'static str) -> impl IntoView {
    view! {
        <h1 class="px-1 text-2xl font-medium">{title}</h1>
        <EmptyState title="Écran en préparation" body="Il arrive avec l'étape 9 de la spécification." />
    }
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    view! { <Upcoming title="Patrimoine" /> }
}

#[component]
pub fn AccountsPage() -> impl IntoView {
    view! { <Upcoming title="Comptes" /> }
}

#[component]
pub fn AccountDetailPage() -> impl IntoView {
    view! { <Upcoming title="Compte" /> }
}

#[component]
pub fn BudgetPage() -> impl IntoView {
    view! { <Upcoming title="Budget" /> }
}

#[component]
pub fn ProjectionPage() -> impl IntoView {
    view! { <Upcoming title="Projection" /> }
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    view! { <Upcoming title="Réglages" /> }
}

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <EmptyState title="Page introuvable" body="Ce lien ne mène nulle part.">
            <A href="/" attr:class=BTN_SECONDARY>"Revenir à l'accueil"</A>
        </EmptyState>
    }
}
