use std::collections::HashMap;

use lc_shared_dto::{CategoryDto, ChargeDto, ChargeInput, IdInput, PeriodicityDto};
use leptos::prelude::*;

use crate::api;
use crate::components::style::{BTN_DANGER, BTN_GHOST, BTN_PRIMARY, LABEL, SECTION_TITLE};
use crate::components::{Amount, BottomSheet, Card, EmptyState, Field};
use crate::format;
use crate::util::{Loaded, current_month, run_action};

fn periodicity_label(periodicity: PeriodicityDto) -> &'static str {
    match periodicity {
        PeriodicityDto::Monthly => "Mensuelle",
        PeriodicityDto::Quarterly => "Trimestrielle",
        PeriodicityDto::Yearly => "Annuelle",
    }
}

/// Charges fixes récurrentes (loyer, assurances, abonnements…).
#[component]
pub fn ChargesSection(
    charges: Loaded<Vec<ChargeDto>>,
    categories: Signal<Vec<CategoryDto>>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let new_open = RwSignal::new(false);
    let editing = RwSignal::new(None::<ChargeDto>);
    let names = Memo::new(move |_| {
        categories
            .get()
            .into_iter()
            .map(|c| (c.id, c.name))
            .collect::<HashMap<_, _>>()
    });

    view! {
        <section class="flex flex-col gap-2">
            <div class="flex items-center justify-between">
                <h2 class=SECTION_TITLE>"Charges fixes"</h2>
                <button class=BTN_GHOST aria-label="Ajouter une charge fixe" on:click=move |_| new_open.set(true)>"Ajouter"</button>
            </div>
            {move || {
                let list = charges.get().and_then(Result::ok).unwrap_or_default();
                if list.is_empty() {
                    return view! { <Card><EmptyState title="Aucune charge fixe" body="Loyer, assurances, abonnements : saisissez-les une fois." /></Card> }.into_any();
                }
                view! {
                    <Card class="flex flex-col p-2">
                        {list.into_iter().map(|charge| {
                            let open = charge.clone();
                            let category = names.with(|n| n.get(&charge.category_id).cloned().unwrap_or_default());
                            let monthly = (charge.periodicity != PeriodicityDto::Monthly).then_some(charge.monthly_equivalent);
                            view! {
                                <button
                                    class="flex min-h-14 items-center justify-between gap-3 rounded-xl px-2 text-left active:bg-surface-raised"
                                    on:click=move |_| editing.set(Some(open.clone()))
                                >
                                    <span class="flex min-w-0 flex-col">
                                        <span class="truncate text-sm">{charge.label}</span>
                                        <span class="text-xs text-muted">{periodicity_label(charge.periodicity)} " · " {category}</span>
                                    </span>
                                    <span class="flex flex-col items-end">
                                        <Amount value=charge.amount />
                                        {monthly.map(|m| view! { <span class="text-xs text-muted">"soit " {format::amount(m)} " / mois"</span> })}
                                    </span>
                                </button>
                            }
                        }).collect_view()}
                    </Card>
                }
                .into_any()
            }}
        </section>
        <ChargeSheet open=new_open categories=categories existing=None on_saved=on_saved />
        {move || editing.get().map(|charge| {
            let open = RwSignal::new(true);
            Effect::new(move |_| if !open.get() { editing.set(None) });
            view! { <ChargeSheet open=open categories=categories existing=Some(charge) on_saved=on_saved /> }
        })}
    }
}

#[component]
fn ChargeSheet(
    open: RwSignal<bool>,
    categories: Signal<Vec<CategoryDto>>,
    existing: Option<ChargeDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let editing = existing.is_some();
    let label = RwSignal::new(
        existing
            .as_ref()
            .map(|c| c.label.clone())
            .unwrap_or_default(),
    );
    let amount = RwSignal::new(
        existing
            .as_ref()
            .map(|c| format::quantity(c.amount))
            .unwrap_or_default(),
    );
    let periodicity = RwSignal::new(
        existing
            .as_ref()
            .map_or(PeriodicityDto::Monthly, |c| c.periodicity),
    );
    let category = RwSignal::new(existing.as_ref().map(|c| c.category_id.clone()));
    let start = RwSignal::new(
        existing
            .as_ref()
            .map_or_else(current_month, |c| c.start.clone()),
    );
    let end = RwSignal::new(
        existing
            .as_ref()
            .and_then(|c| c.end.clone())
            .unwrap_or_default(),
    );
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let ids = StoredValue::new((
        existing.as_ref().map(|c| c.id.clone()),
        existing.map(|c| c.category_id),
    ));

    let submit = move |_| {
        let parsed_amount = match format::parse_amount(&amount.get_untracked()) {
            Ok(amount) => amount,
            Err(message) => return error.set(Some(message)),
        };
        let Some(category_id) = category.get_untracked() else {
            return error.set(Some("Choisissez une catégorie".into()));
        };
        let input = ChargeInput {
            id: ids.get_value().0,
            label: label.get_untracked(),
            amount: parsed_amount,
            category_id,
            periodicity: periodicity.get_untracked(),
            start: start.get_untracked(),
            end: Some(end.get_untracked()).filter(|e| !e.trim().is_empty()),
        };
        run_action(busy, error, api::upsert_charge(input), move |_: String| {
            if !editing {
                label.set(String::new());
                amount.set(String::new());
            }
            open.set(false);
            on_saved.run(());
        });
    };
    let remove = move |_| {
        let Some(id) = ids.get_value().0 else { return };
        run_action(busy, error, api::delete_charge(IdInput { id }), move |()| {
            open.set(false);
            on_saved.run(());
        });
    };
    let chip_class = |active: bool| {
        format!(
            "min-h-11 rounded-xl border px-4 text-sm {}",
            if active {
                "border-primary bg-primary/15 text-ink"
            } else {
                "border-line text-muted"
            }
        )
    };

    view! {
        <BottomSheet open=open title=if editing { "Charge fixe" } else { "Nouvelle charge fixe" }>
            <div class="flex flex-col gap-4">
                <Field label="Libellé" value=label placeholder="Loyer" />
                <Field label="Montant" value=amount input_mode="decimal" suffix="€" />
                <div class="flex flex-col gap-1.5">
                    <span class=LABEL>"Fréquence"</span>
                    <div class="flex flex-wrap gap-2">
                        {[PeriodicityDto::Monthly, PeriodicityDto::Quarterly, PeriodicityDto::Yearly].into_iter().map(|p| view! {
                            <button class=move || chip_class(periodicity.get() == p) on:click=move |_| periodicity.set(p)>
                                {periodicity_label(p)}
                            </button>
                        }).collect_view()}
                    </div>
                </div>
                <div class="flex flex-col gap-1.5">
                    <span class=LABEL>"Catégorie"</span>
                    <div class="flex flex-wrap gap-2">
                        {move || {
                            let original = ids.get_value().1;
                            categories.get().into_iter().filter(|c| !c.archived || Some(&c.id) == original.as_ref()).map(|c| {
                                let id = c.id.clone();
                                let check = c.id.clone();
                                view! {
                                    <button
                                        class=move || chip_class(category.get().as_deref() == Some(check.as_str()))
                                        on:click=move |_| category.set(Some(id.clone()))
                                    >
                                        {c.name}
                                    </button>
                                }
                            }).collect_view()
                        }}
                    </div>
                </div>
                <div class="grid grid-cols-2 gap-2">
                    <Field label="Depuis" value=start input_type="month" />
                    <Field label="Jusqu'à (facultatif)" value=end input_type="month" />
                </div>
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit>"Enregistrer"</button>
                {editing.then(|| view! { <button class=BTN_DANGER disabled=move || busy.get() on:click=remove>"Supprimer"</button> })}
            </div>
        </BottomSheet>
    }
}
