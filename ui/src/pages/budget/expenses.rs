use std::collections::HashMap;

use lc_shared_dto::{
    AccountOptionDto, CategoryDto, ExpenseDto, ExpenseInput, ExpenseListDto, IdInput,
    LabelSuggestionDto, NaiveDate,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::style::{BTN_DANGER, BTN_GHOST, BTN_PRIMARY, LABEL, SECTION_TITLE};
use crate::components::{Amount, BottomSheet, Card, EmptyState, ErrorBanner, Field, Skeleton};
use crate::format;
use crate::util::{Loaded, run_action, today};

use super::account_choice::{AccountChoice, AccountSelection, resolve};

/// Dépenses du mois : filtre par catégorie, total, liste groupée par jour.
#[component]
pub fn ExpensesSection(
    expenses: Loaded<ExpenseListDto>,
    filter: RwSignal<Option<String>>,
    categories: Signal<Vec<CategoryDto>>,
    checking: Signal<Vec<AccountOptionDto>>,
    open_new: RwSignal<bool>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let editing = RwSignal::new(None::<ExpenseDto>);
    let names = Memo::new(move |_| {
        categories
            .get()
            .into_iter()
            .map(|c| (c.id, c.name))
            .collect::<HashMap<_, _>>()
    });

    let chip = move |id: Option<String>, label: String| {
        let selected_id = id.clone();
        view! {
            <button
                class=move || format!(
                    "min-h-9 shrink-0 rounded-full border px-3 text-sm {}",
                    if filter.get() == selected_id { "border-primary bg-primary/15 text-ink" } else { "border-line text-muted" },
                )
                on:click=move |_| filter.set(id.clone())
            >
                {label}
            </button>
        }
    };

    view! {
        <section class="flex flex-col gap-2">
            <div class="flex items-center justify-between">
                <h2 class=SECTION_TITLE>"Dépenses"</h2>
                <button class=BTN_GHOST aria-label="Ajouter une dépense" on:click=move |_| open_new.set(true)>"Ajouter"</button>
            </div>
            <div class="-mx-4 flex gap-2 overflow-x-auto px-4 pb-1 [scrollbar-width:none]">
                {chip(None, "Toutes".into())}
                {move || categories.get().into_iter().filter(|c| !c.archived).map(|c| chip(Some(c.id), c.name)).collect_view()}
            </div>
            {move || match expenses.get() {
                None => view! { <Skeleton class="h-32" /> }.into_any(),
                Some(Err(error)) => view! { <ErrorBanner message=error.message /> }.into_any(),
                Some(Ok(list)) if list.items.is_empty() => view! {
                    <EmptyState
                        title="Aucune dépense ce mois-ci"
                        body="Un montant, une catégorie : c'est noté."
                        action="Ajouter une dépense"
                        on_action=Callback::new(move |_| open_new.set(true))
                    />
                }
                .into_any(),
                Some(Ok(list)) => {
                    let mut groups: Vec<(NaiveDate, Vec<ExpenseDto>)> = Vec::new();
                    for item in list.items {
                        match groups.last_mut() {
                            Some((date, items)) if *date == item.date => items.push(item),
                            _ => groups.push((item.date, vec![item])),
                        }
                    }
                    view! {
                        <Card class="flex flex-col gap-1 p-2">
                            <div class="flex items-center justify-between px-2 py-1">
                                <span class="text-sm text-muted">"Total"</span>
                                <Amount value=list.total class="font-semibold" />
                            </div>
                            {groups.into_iter().map(|(date, items)| view! {
                                <p class="px-2 pt-2 text-xs text-muted">{format::date(date)}</p>
                                {items.into_iter().map(|expense| {
                                    let open = expense.clone();
                                    let category = names.with(|n| n.get(&expense.category_id).cloned().unwrap_or_default());
                                    view! {
                                        <button
                                            class="flex min-h-12 items-center gap-3 rounded-xl px-2 text-left active:bg-surface-raised"
                                            on:click=move |_| editing.set(Some(open.clone()))
                                        >
                                            <span class="flex min-w-0 flex-1 flex-col">
                                                <span class="truncate text-sm">{expense.label.clone().unwrap_or_else(|| category.clone())}</span>
                                                {expense.label.is_some().then(|| view! { <span class="text-xs text-muted">{category}</span> })}
                                            </span>
                                            <Amount value=expense.amount />
                                        </button>
                                    }
                                }).collect_view()}
                            }).collect_view()}
                        </Card>
                    }
                    .into_any()
                }
            }}
        </section>
        <ExpenseSheet open=open_new categories=categories checking=checking existing=None on_saved=on_saved />
        {move || editing.get().map(|expense| {
            let open = RwSignal::new(true);
            Effect::new(move |_| if !open.get() { editing.set(None) });
            view! { <ExpenseSheet open=open categories=categories checking=checking existing=Some(expense) on_saved=on_saved /> }
        })}
    }
}

/// Saisie d'une dépense. Pour une nouvelle dépense, toucher une catégorie enregistre aussitôt
/// (saisie rapide : « + », « Dépense », montant, catégorie).
#[component]
pub fn ExpenseSheet(
    open: RwSignal<bool>,
    categories: Signal<Vec<CategoryDto>>,
    checking: Signal<Vec<AccountOptionDto>>,
    existing: Option<ExpenseDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let editing = existing.is_some();
    let account: AccountSelection = RwSignal::new(existing.as_ref().map(|e| e.account_id.clone()));
    let amount = RwSignal::new(
        existing
            .as_ref()
            .map(|e| format::quantity(e.amount))
            .unwrap_or_default(),
    );
    let category = RwSignal::new(existing.as_ref().map(|e| e.category_id.clone()));
    let date = RwSignal::new(
        existing
            .as_ref()
            .map_or_else(today, |e| e.date)
            .format("%Y-%m-%d")
            .to_string(),
    );
    let label = RwSignal::new(
        existing
            .as_ref()
            .and_then(|e| e.label.clone())
            .unwrap_or_default(),
    );
    let more = RwSignal::new(editing);
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let ids = StoredValue::new((
        existing.as_ref().map(|e| e.id.clone()),
        existing.as_ref().map(|e| e.category_id.clone()),
    ));

    let save = move |category_id: String| {
        let parsed_amount = match format::parse_amount(&amount.get_untracked()) {
            Ok(amount) => amount,
            Err(message) => return error.set(Some(message)),
        };
        let Ok(date) = NaiveDate::parse_from_str(&date.get_untracked(), "%Y-%m-%d") else {
            return error.set(Some("Date invalide".into()));
        };
        let label_text = Some(label.get_untracked()).filter(|l| !l.trim().is_empty());
        let input = ExpenseInput {
            id: ids.get_value().0,
            amount: parsed_amount,
            category_id,
            date: Some(date),
            label: label_text,
            account_id: resolve(account.get_untracked(), &checking.get_untracked()),
        };
        run_action(busy, error, api::save_expense(input), move |_: String| {
            // Une nouvelle saisie repart d'une feuille vide.
            if !editing {
                amount.set(String::new());
                label.set(String::new());
                category.set(None);
                more.set(false);
                account.set(None);
            }
            open.set(false);
            on_saved.run(());
        });
    };

    let remove = move |_| {
        let Some(id) = ids.get_value().0 else { return };
        run_action(
            busy,
            error,
            api::delete_expense(IdInput { id }),
            move |()| {
                open.set(false);
                on_saved.run(());
            },
        );
    };

    // Libellés déjà saisis : rechargés à chaque ouverture pour inclure la dernière saisie.
    let suggestions = RwSignal::new(Vec::<LabelSuggestionDto>::new());
    Effect::new(move |_| {
        if open.get() {
            spawn_local(async move {
                if let Ok(found) = api::expense_suggestions().await {
                    suggestions.set(found);
                }
            });
        }
    });
    // Toucher une suggestion reprend son libellé et sa catégorie ; pour une nouvelle dépense,
    // elle est enregistrée aussitôt (comme les catégories).
    let apply = move |suggestion: LabelSuggestionDto| {
        label.set(suggestion.label);
        category.set(Some(suggestion.category_id.clone()));
        if !editing {
            save(suggestion.category_id);
        }
    };
    let category_name = move |id: &str| {
        categories.with(|list| {
            list.iter()
                .find(|c| c.id == id && !c.archived)
                .map(|c| c.name.clone())
        })
    };
    // Suggestions dont la catégorie existe toujours, filtrées par le libellé en cours de saisie.
    let matching = move |limit: usize| {
        let typed = fold(&label.get());
        suggestions
            .get()
            .into_iter()
            .filter_map(|s| category_name(&s.category_id).map(|name| (s, name)))
            .filter(|(s, _)| {
                let candidate = fold(&s.label);
                typed.is_empty() || (candidate.contains(&typed) && candidate != typed)
            })
            .take(limit)
            .collect::<Vec<_>>()
    };
    let chips = move |items: Vec<(LabelSuggestionDto, String)>| {
        items
            .into_iter()
            .map(|(suggestion, category)| {
                let text = suggestion.label.clone();
                let description = format!("{text} · {category}");
                view! {
                    <button
                        aria-label=description
                        class="flex min-h-9 items-center gap-1.5 rounded-full border border-line bg-surface-raised px-3 text-sm active:border-primary active:bg-primary/15"
                        disabled=move || busy.get()
                        on:click=move |_| apply(suggestion.clone())
                    >
                        {text}
                        <span class="text-xs text-muted">{category}</span>
                    </button>
                }
            })
            .collect_view()
    };

    let usable = move || {
        let original = ids.get_value().1;
        categories
            .get()
            .into_iter()
            .filter(|c| !c.archived || Some(&c.id) == original.as_ref())
            .collect::<Vec<_>>()
    };

    view! {
        <BottomSheet open=open title=if editing { "Dépense" } else { "Nouvelle dépense" }>
            <div class="flex flex-col gap-4">
                <Field label="Montant" value=amount input_mode="decimal" suffix="€" placeholder="0,00" />
                {move || {
                    let frequent = if editing || more.get() { Vec::new() } else { matching(4) };
                    (!frequent.is_empty()).then(|| view! {
                        <div class="flex flex-col gap-1.5">
                            <span class=LABEL>"Habituelles (touchez pour enregistrer)"</span>
                            <div class="flex flex-wrap gap-2">{chips(frequent)}</div>
                        </div>
                    })
                }}
                <Show when=move || more.get()>
                    <Field label="Date" value=date input_type="date" />
                    <Field label="Libellé (facultatif)" value=label placeholder="Courses du marché" />
                    {move || {
                        let found = if label.with(|l| l.trim().is_empty()) { Vec::new() } else { matching(4) };
                        (!found.is_empty()).then(|| view! {
                            <div class="-mt-2 flex flex-wrap gap-2" aria-label="Libellés déjà utilisés">{chips(found)}</div>
                        })
                    }}
                </Show>
                <Show when=move || !more.get()>
                    <button class=format!("{BTN_GHOST} self-start px-0") on:click=move |_| more.set(true)>"Date et libellé"</button>
                </Show>
                <AccountChoice label="Payé depuis" options=checking selection=account />
                <div class="flex flex-col gap-1.5">
                    <span class=LABEL>
                        {if editing { "Catégorie" } else { "Catégorie (touchez pour enregistrer)" }}
                    </span>
                    <div class="flex flex-wrap gap-2">
                        {move || usable().into_iter().map(|c| {
                            let id = c.id.clone();
                            let check = c.id.clone();
                            view! {
                                <button
                                    class=move || format!(
                                        "min-h-11 rounded-xl border px-4 text-sm {}",
                                        if category.get().as_deref() == Some(check.as_str()) { "border-primary bg-primary/15 text-ink" } else { "border-line text-ink" },
                                    )
                                    disabled=move || busy.get()
                                    on:click=move |_| {
                                        category.set(Some(id.clone()));
                                        if !editing {
                                            save(id.clone());
                                        }
                                    }
                                >
                                    {c.name}
                                </button>
                            }
                        }).collect_view()}
                    </div>
                </div>
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                {editing.then(|| view! {
                    <button
                        class=BTN_PRIMARY
                        disabled=move || busy.get()
                        on:click=move |_| match category.get_untracked() {
                            Some(id) => save(id),
                            None => error.set(Some("Choisissez une catégorie".into())),
                        }
                    >
                        "Enregistrer"
                    </button>
                    <button class=BTN_DANGER disabled=move || busy.get() on:click=remove>"Supprimer"</button>
                })}
            </div>
        </BottomSheet>
    }
}

/// Forme comparable d'un libellé : minuscules, sans accents.
fn fold(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}
