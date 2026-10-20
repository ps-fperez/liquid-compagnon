use std::time::Duration;

use lc_shared_dto::{AssetDto, AssetSearchInput, QuoteSourceDto};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::style::{INPUT, LABEL};
use crate::icons::{Icon, IconName};
use crate::util::source_label;

/// Délai après la dernière frappe avant de lancer la recherche (limite les appels aux API).
const DEBOUNCE: Duration = Duration::from_millis(400);
const MIN_CHARS: usize = 2;

/// Actifs les plus courants, proposés avant toute saisie : un geste suffit pour les choisir.
/// (symbole, nom court affiché sur la puce, nom complet enregistré)
const POPULAR_CRYPTO: [(&str, &str, &str); 6] = [
    ("bitcoin", "Bitcoin", "Bitcoin (BTC)"),
    ("ethereum", "Ethereum", "Ethereum (ETH)"),
    ("solana", "Solana", "Solana (SOL)"),
    ("ripple", "XRP", "XRP (XRP)"),
    ("binancecoin", "BNB", "BNB (BNB)"),
    ("cardano", "Cardano", "Cardano (ADA)"),
];
const POPULAR_PEA_ETF: [(&str, &str, &str); 8] = [
    (
        "CW8.PA",
        "MSCI World (CW8)",
        "Amundi MSCI World Swap UCITS ETF EUR Acc · Paris",
    ),
    (
        "DCAM.PA",
        "PEA Monde (DCAM)",
        "Amundi PEA Monde (MSCI World) UCITS ETF · Paris",
    ),
    (
        "WPEA.PA",
        "iShares World PEA",
        "iShares MSCI World Swap PEA UCITS ETF · Paris",
    ),
    (
        "ESE.PA",
        "S&P 500 (ESE)",
        "BNP Paribas Easy S&P 500 UCITS ETF EUR C · Paris",
    ),
    (
        "PSP5.PA",
        "PEA S&P 500 (PSP5)",
        "Amundi PEA S&P 500 UCITS ETF Acc · Paris",
    ),
    (
        "PUST.PA",
        "Nasdaq-100 (PUST)",
        "Amundi PEA Nasdaq-100 UCITS ETF Acc · Paris",
    ),
    (
        "PAEEM.PA",
        "Émergents (PAEEM)",
        "Amundi PEA Émergent (MSCI Emerging) UCITS ETF Acc · Paris",
    ),
    (
        "PCEU.PA",
        "Europe (PCEU)",
        "Amundi PEA MSCI Europe UCITS ETF Acc · Paris",
    ),
];

fn popular(source: QuoteSourceDto) -> Vec<(&'static str, AssetDto)> {
    let list: &[(&str, &str, &str)] = match source {
        QuoteSourceDto::CoinGecko => &POPULAR_CRYPTO,
        QuoteSourceDto::Yahoo => &POPULAR_PEA_ETF,
        // Les places européennes ne sont pas couvertes par l'offre gratuite Twelve Data.
        QuoteSourceDto::TwelveData => &[],
    };
    list.iter()
        .map(|(symbol, short, name)| {
            (
                *short,
                AssetDto {
                    source,
                    symbol: (*symbol).to_string(),
                    display_name: (*name).to_string(),
                    currency: "EUR".to_string(),
                },
            )
        })
        .collect()
}

/// Recherche d'un actif coté avec autocomplétion (« CW8 », « bitcoin », « Amundi World »…).
/// Avant toute saisie, les actifs les plus courants sont proposés. Avec plusieurs sources, la
/// première est proposée par défaut et un sélecteur permet d'en changer.
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
    // Numéro de la dernière recherche : une réponse plus ancienne arrivée en retard est ignorée.
    let generation = StoredValue::new(0_u32);
    let pending = StoredValue::new(None::<TimeoutHandle>);

    let cancel_pending = move || {
        if let Some(handle) = pending.get_value() {
            handle.clear();
        }
        pending.set_value(None);
    };

    let search = move || {
        cancel_pending();
        let text = query.get_untracked().trim().to_string();
        generation.update_value(|g| *g += 1);
        let current = generation.get_value();
        if text.chars().count() < MIN_CHARS {
            results.set(Vec::new());
            message.set(None);
            searching.set(false);
            return;
        }
        searching.set(true);
        message.set(None);
        let source = source.get_untracked();
        spawn_local(async move {
            let outcome = api::search_assets(AssetSearchInput {
                source,
                query: text,
            })
            .await;
            if generation.get_value() != current {
                return;
            }
            match outcome {
                Ok(found) if found.is_empty() => {
                    results.set(Vec::new());
                    message.set(Some(match source {
                        QuoteSourceDto::CoinGecko => {
                            "Aucun résultat. Essayez le nom complet (« ethereum »).".into()
                        }
                        _ => "Aucun résultat. Essayez le ticker (« CW8 ») ou le code ISIN (« LU1681043599 »).".into(),
                    }));
                }
                Ok(found) => results.set(found),
                Err(error) => message.set(Some(error.message)),
            }
            searching.set(false);
        });
    };

    let on_input = move |text: String| {
        query.set(text);
        cancel_pending();
        if query.with_untracked(|q| q.trim().chars().count() < MIN_CHARS) {
            search();
            return;
        }
        searching.set(true);
        pending.set_value(set_timeout_with_handle(search, DEBOUNCE).ok());
    };

    let choose = move |asset: AssetDto| {
        cancel_pending();
        generation.update_value(|g| *g += 1);
        selected.set(Some(asset));
        results.set(Vec::new());
        query.set(String::new());
        message.set(None);
        searching.set(false);
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
                                    search();
                                }
                            >
                                {source_label(option)}
                            </button>
                        }).collect_view()}
                    </div>
                })}
                <div class="relative">
                    <Icon name=IconName::Search class="pointer-events-none absolute top-1/2 left-3.5 size-5 -translate-y-1/2 text-muted" />
                    <input
                        class=format!("{INPUT} pl-11 pr-11")
                        role="combobox"
                        aria-label="Rechercher un actif"
                        aria-autocomplete="list"
                        aria-expanded=move || results.with(|r| !r.is_empty()).to_string()
                        placeholder=placeholder.clone()
                        autocomplete="off"
                        enterkeyhint="search"
                        prop:value=move || query.get()
                        on:input=move |event| on_input(event_target_value(&event))
                        on:keydown=move |event| if event.key() == "Enter" { search() }
                    />
                    <span class="absolute top-1/2 right-3.5 flex -translate-y-1/2">
                        {move || if searching.get() {
                            view! { <Icon name=IconName::Refresh class="size-5 animate-spin text-primary-soft" /> }.into_any()
                        } else if query.with(|q| !q.is_empty()) {
                            view! {
                                <button class="flex text-muted" aria-label="Effacer la recherche" on:click=move |_| on_input(String::new())>
                                    <Icon name=IconName::Close class="size-5" />
                                </button>
                            }.into_any()
                        } else {
                            ().into_any()
                        }}
                    </span>
                </div>
                {move || message.get().map(|text| view! { <span class="text-sm text-muted">{text}</span> })}
                {move || {
                    let suggestions = popular(source.get());
                    let idle = query.with(|q| q.trim().is_empty()) && results.with(Vec::is_empty);
                    (idle && !suggestions.is_empty()).then(|| view! {
                        <div class="flex flex-col gap-1.5 pt-1">
                            <span class="text-xs text-muted">"Populaires · touchez pour choisir"</span>
                            <div class="flex flex-wrap gap-2">
                                {suggestions.into_iter().map(|(short, asset)| view! {
                                    <button
                                        class="min-h-9 rounded-full border border-line bg-surface-raised px-3 text-sm active:border-primary active:bg-primary/15"
                                        on:click=move |_| choose(asset.clone())
                                    >
                                        {short}
                                    </button>
                                }).collect_view()}
                            </div>
                        </div>
                    })
                }}
                <div class="flex max-h-64 flex-col overflow-y-auto" role="listbox">
                    {move || results.get().into_iter().map(|asset| {
                        let chosen = asset.clone();
                        view! {
                            <button
                                class="flex min-h-12 flex-col items-start justify-center rounded-xl px-3 py-1 text-left active:bg-surface-raised"
                                role="option"
                                on:click=move |_| choose(chosen.clone())
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
