use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::EmptyState;
use crate::components::style::BTN_SECONDARY;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <EmptyState title="Page introuvable" body="Ce lien ne mène nulle part.">
            <A href="/" attr:class=BTN_SECONDARY>"Revenir à l'accueil"</A>
        </EmptyState>
    }
}
