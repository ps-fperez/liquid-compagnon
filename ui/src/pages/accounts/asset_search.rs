use lc_shared_dto::{AssetDto, AssetSearchInput, QuoteSourceDto};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::style::{BTN_SECONDARY, INPUT, LABEL};
use crate::icons::{Icon, IconName};
use crate::util::source_label;

/// Recherche d'un actif coté (« CW8 », « bitcoin »…). Avec plusieurs sources, la première est
/// proposée par défaut et un sélecteur permet d'en changer.
#[component]
pub fn AssetSearch(
    sources: Vec<QuoteSourceDto>,
    selected: RwSignal<Option<AssetDto>>,
    #[prop(into)] label: String,
    #[prop(into)] placeholder: String,
) -> impl IntoView {
    let source = RwSignal::new(sources[0]);
    let query = RwSignal::new(String::new());
    let results = RwSignal::new(Vec::<AssetDto>::new());
    let searching = RwSignal::new(false);
    let message = RwSignal::new(None::<String>);

    let search = move || {
        let text = query.get_untracked();
        if text.trim().is_empty() || searching.get_untracked() {
            return;
        }
        searching.set(true);
        message.set(None);
        let source = source.get_untracked();
        spawn_local(async move {
            match api::search_assets(AssetSearchInput {
                source,
                query: text,
            })
            .await
            {
                Ok(found) if found.is_empty() => {
                    results.set(Vec::new());
                    message.set(Some("Aucun résultat".into()));
                }
                Ok(found) => results.set(found),
                Err(error) => message.set(Some(error.message)),
            }
            searching.set(false);
        });
    };

    view! {
        <div class="flex flex-col gap-1.5">
            <span class=LABEL>{label}</span>
            {move || selected.get().map(|asset| view! {
                <div class="flex items-center justify-between gap-2 rounded-xl border border-primary/50 bg-primary/10 px-4 py-2">
                    <span class="flex min-w-0 flex-col">
                        <span class="truncate text-sm">{asset.display_name}</span>
                        <span class="text-xs text-muted">{asset.symbol} " · " {asset.currency}</span>
                    </span>
                    <button class="text-sm text-secondary-light" on:click=move |_| selected.set(None)>"Changer"</button>
                </div>
            })}
            <Show when=move || selected.with(Option::is_none)>
                {(sources.len() > 1).then(|| view! {
                    <div class="flex gap-2" role="radiogroup" aria-label="Source des cours">
                        {sources.iter().map(|&option| view! {
                            <button
                                class=move || format!("rounded-full border px-3 py-1 text-xs {}", if source.get() == option { "border-primary bg-primary/15 text-primary-soft" } else { "border-line text-muted" })
                                role="radio"
                                aria-checked=move || (source.get() == option).to_string()
                                on:click=move |_| {
                                    source.set(option);
                                    results.set(Vec::new());
                                    message.set(None);
                                }
                            >
                                {source_label(option)}
                            </button>
                        }).collect_view()}
                    </div>
                })}
                <div class="flex gap-2">
                    <input
                        class=INPUT
                        placeholder=placeholder.clone()
                        autocomplete="off"
                        prop:value=move || query.get()
                        on:input=move |event| query.set(event_target_value(&event))
                        on:keydown=move |event| if event.key() == "Enter" { search() }
                    />
                    <button class=format!("{BTN_SECONDARY} px-3") aria-label="Rechercher" disabled=move || searching.get() on:click=move |_| search()>
                        <Icon name=IconName::Search class="size-5" />
                    </button>
                </div>
                {move || message.get().map(|text| view! { <span class="text-sm text-muted">{text}</span> })}
                <div class="flex max-h-56 flex-col overflow-y-auto">
                    {move || results.get().into_iter().map(|asset| {
                        let chosen = asset.clone();
                        view! {
                            <button
                                class="flex min-h-12 flex-col items-start justify-center rounded-xl px-3 py-1 text-left active:bg-surface-raised"
                                on:click=move |_| {
                                    selected.set(Some(chosen.clone()));
                                    results.set(Vec::new());
                                }
                            >
                                <span class="text-sm">{asset.display_name}</span>
                                <span class="text-xs text-muted">{asset.symbol} " · " {asset.currency}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }
}
