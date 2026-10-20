use leptos::prelude::*;

use super::style::BTN_ICON;
use crate::icons::{Icon, IconName};

/// Feuille qui monte du bas de l'écran (saisie rapide, formulaires courts). Se ferme en touchant
/// le fond ou la croix.
#[component]
pub fn BottomSheet(
    open: RwSignal<bool>,
    #[prop(into)] title: String,
    children: ChildrenFn,
) -> impl IntoView {
    let title = StoredValue::new(title);
    view! {
        <Show when=move || open.get()>
            <div class="fixed inset-0 z-40 flex flex-col justify-end">
                <button
                    class="absolute inset-0 animate-fade-in bg-black/60"
                    aria-label="Fermer"
                    on:click=move |_| open.set(false)
                ></button>
                <div
                    class="relative max-h-[85dvh] animate-sheet-up overflow-y-auto rounded-t-3xl border-t border-line bg-surface px-4 pt-3 pb-[calc(1.5rem+env(safe-area-inset-bottom))]"
                    role="dialog"
                    aria-modal="true"
                >
                    <div class="mx-auto mb-2 h-1 w-10 rounded-full bg-line"></div>
                    <div class="mb-4 flex items-center justify-between">
                        <h2 class="text-lg font-medium">{title.get_value()}</h2>
                        <button class=BTN_ICON aria-label="Fermer" on:click=move |_| open.set(false)>
                            <Icon name=IconName::Close />
                        </button>
                    </div>
                    {children()}
                </div>
            </div>
        </Show>
    }
}
