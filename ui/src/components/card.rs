use leptos::prelude::*;

/// Surface sobre (listes, sections).
#[component]
pub fn Card(children: Children, #[prop(optional, into)] class: String) -> impl IntoView {
    view! {
        <section class=format!("rounded-[18px] border border-line bg-surface p-4 {class}")>{children()}</section>
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

/// Carte principale d'un écran (maquette v0.3 « hero ») : bordure violette, dégradé et halo.
#[component]
pub fn HeroCard(children: Children, #[prop(optional, into)] class: String) -> impl IntoView {
    view! {
        <section class=format!(
            "flex flex-col gap-3 rounded-[18px] border border-primary/45 bg-[linear-gradient(160deg,#1d1033,var(--color-surface)_60%)] p-4 shadow-[0_0_26px_-10px_rgb(168_85_247/0.55)] {class}",
        )>
            {children()}
        </section>
    }
}
