use leptos::prelude::*;

use crate::components::EmptyState;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <EmptyState title="Page introuvable" body="Ce lien ne mène nulle part." action="Revenir à l'accueil" href="/" />
    }
}
