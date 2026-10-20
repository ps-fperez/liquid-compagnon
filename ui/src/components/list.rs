use leptos::prelude::*;
use leptos_router::components::A;

use crate::icons::{Icon, IconName};

/// Ligne de liste cliquable (au moins 56 px de haut), menant à `href`.
#[component]
pub fn ListRow(#[prop(into)] href: String, children: Children) -> impl IntoView {
    view! {
        <A href=href attr:class="flex min-h-14 items-center gap-3 rounded-xl px-3 py-2 transition active:bg-surface-raised">
            <div class="flex min-w-0 flex-1 items-center gap-3">{children()}</div>
            <Icon name=IconName::ChevronRight class="size-5 shrink-0 text-muted" />
        </A>
    }
}
