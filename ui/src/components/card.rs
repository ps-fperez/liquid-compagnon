use leptos::prelude::*;

/// Surface sobre (listes, sections).
#[component]
pub fn Card(children: Children, #[prop(optional, into)] class: String) -> impl IntoView {
    view! {
        <section class=format!("rounded-2xl border border-line bg-surface p-4 {class}")>{children()}</section>
    }
}

/// Carte mise en avant (patrimoine total) : dégradé violet → bleu en bordure et halo.
#[component]
pub fn GlowCard(children: Children, #[prop(optional, into)] class: String) -> impl IntoView {
    view! {
        <section class=format!(
            "rounded-2xl bg-linear-to-br from-primary via-primary-strong to-secondary p-px shadow-glow-primary {class}",
        )>
            <div class="rounded-[calc(1rem-1px)] bg-linear-to-br from-surface-raised to-surface p-5">
                {children()}
            </div>
        </section>
    }
}
