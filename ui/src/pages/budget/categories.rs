use lc_shared_dto::{CategoryDto, CategoryNameInput, IdInput};
use leptos::prelude::*;

use crate::api;
use crate::components::style::{BTN_GHOST, BTN_SECONDARY, INPUT};
use crate::components::{BottomSheet, Field};
use crate::util::run_action;

/// Création, renommage, archivage et suppression des catégories. Une catégorie utilisée ne peut
/// pas être supprimée : le backend le refuse et le message invite à l'archiver.
#[component]
pub fn CategoriesSheet(
    open: RwSignal<bool>,
    categories: Signal<Vec<CategoryDto>>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let new_name = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);

    let create = move |_| {
        let input = CategoryNameInput {
            category_id: None,
            name: new_name.get_untracked(),
        };
        run_action(busy, error, api::save_category(input), move |_: String| {
            new_name.set(String::new());
            on_saved.run(());
        });
    };

    view! {
        <BottomSheet open=open title="Catégories">
            <div class="flex flex-col gap-4">
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <div class="flex flex-col gap-1">
                    {move || categories.get().into_iter().map(|category| view! {
                        <CategoryRow category=category busy=busy error=error on_saved=on_saved />
                    }).collect_view()}
                </div>
                <Field label="Nouvelle catégorie" value=new_name placeholder="Animaux" />
                <button class=BTN_SECONDARY disabled=move || busy.get() on:click=create>"Ajouter la catégorie"</button>
            </div>
        </BottomSheet>
    }
}

#[component]
fn CategoryRow(
    category: CategoryDto,
    busy: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let renaming = RwSignal::new(false);
    let name = RwSignal::new(category.name.clone());
    let id = StoredValue::new(category.id.clone());
    let archived = category.archived;

    let rename = move |_| {
        let input = CategoryNameInput {
            category_id: Some(id.get_value()),
            name: name.get_untracked(),
        };
        run_action(busy, error, api::save_category(input), move |_: String| {
            renaming.set(false);
            on_saved.run(());
        });
    };
    let toggle_archive = move |_| {
        let input = IdInput { id: id.get_value() };
        if archived {
            run_action(busy, error, api::unarchive_category(input), move |()| {
                on_saved.run(())
            });
        } else {
            run_action(busy, error, api::archive_category(input), move |()| {
                on_saved.run(())
            });
        }
    };
    let delete = move |_| {
        run_action(
            busy,
            error,
            api::delete_category(IdInput { id: id.get_value() }),
            move |()| on_saved.run(()),
        );
    };

    view! {
        <div class="flex flex-col gap-1 rounded-xl border border-line px-3 py-2">
            <Show
                when=move || renaming.get()
                fallback=move || view! {
                    <div class="flex items-center justify-between gap-2">
                        <span class=if archived { "text-muted line-through" } else { "" }>{move || name.get()}</span>
                        <button class=BTN_GHOST on:click=move |_| renaming.set(true)>"Renommer"</button>
                    </div>
                }
            >
                <div class="flex gap-2">
                    <input
                        class=INPUT
                        aria-label="Nom de la catégorie"
                        prop:value=move || name.get()
                        on:input=move |event| name.set(event_target_value(&event))
                    />
                    <button class=BTN_SECONDARY disabled=move || busy.get() on:click=rename>"OK"</button>
                </div>
            </Show>
            <div class="flex gap-2">
                <button class=format!("{BTN_GHOST} px-2 text-sm") disabled=move || busy.get() on:click=toggle_archive>
                    {if archived { "Réactiver" } else { "Archiver" }}
                </button>
                <button class=format!("{BTN_GHOST} px-2 text-sm text-loss") disabled=move || busy.get() on:click=delete>"Supprimer"</button>
            </div>
        </div>
    }
}
