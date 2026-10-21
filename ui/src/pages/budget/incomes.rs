use lc_shared_dto::{
    AccountOptionDto, BudgetSummaryDto, IdInput, NaiveDate, OneOffIncomeDto, OneOffIncomeInput,
    SalaryDto, SalaryInput,
};
use leptos::prelude::*;

use crate::api;
use crate::components::style::{BTN_DANGER, BTN_GHOST, BTN_PRIMARY, SECTION_TITLE};
use crate::components::{Amount, BottomSheet, Card, EmptyState, Field};
use crate::format;
use crate::util::{Loaded, current_month, run_action, today};

use super::account_choice::{AccountChoice, AccountSelection, resolve};

/// Salaire (et son historique) et revenus ponctuels du mois.
#[component]
pub fn IncomesSection(
    summary: Loaded<BudgetSummaryDto>,
    salaries: Loaded<Vec<SalaryDto>>,
    incomes: Loaded<Vec<OneOffIncomeDto>>,
    checking: Signal<Vec<AccountOptionDto>>,
    open_income: RwSignal<bool>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let new_salary = RwSignal::new(false);
    let editing_salary = RwSignal::new(None::<SalaryDto>);
    let editing_income = RwSignal::new(None::<OneOffIncomeDto>);
    let salary = move || {
        summary
            .get()
            .and_then(Result::ok)
            .map(|s| s.salary)
            .unwrap_or_default()
    };

    view! {
        <section class="flex flex-col gap-2">
            <div class="flex items-center justify-between">
                <h2 class=SECTION_TITLE>"Revenus"</h2>
                <button class=BTN_GHOST on:click=move |_| new_salary.set(true)>"Nouveau salaire"</button>
            </div>
            {move || {
                let entries = salaries.get().and_then(Result::ok).unwrap_or_default();
                if entries.is_empty() {
                    return view! {
                        <EmptyState
                            title="Renseignez votre salaire net"
                            body="Il sert à calculer votre capacité d'épargne et la projection."
                            action="Ajouter mon salaire"
                            on_action=Callback::new(move |_| new_salary.set(true))
                        />
                    }
                    .into_any();
                }
                view! {
                    <Card class="flex flex-col gap-1 p-2">
                        <div class="flex items-center justify-between px-2 py-1">
                            <span class="text-sm text-muted">"Salaire net mensuel"</span>
                            {move || view! { <Amount value=salary() class="font-semibold" /> }}
                        </div>
                        {entries.into_iter().map(|entry| {
                            let open = entry.clone();
                            view! {
                                <button
                                    class="flex min-h-11 items-center justify-between rounded-xl px-2 text-left text-sm active:bg-surface-raised"
                                    on:click=move |_| editing_salary.set(Some(open.clone()))
                                >
                                    <span class="text-muted">"depuis " {format::month_label(&entry.effective_from)}</span>
                                    <Amount value=entry.net_amount />
                                </button>
                            }
                        }).collect_view()}
                    </Card>
                }
                .into_any()
            }}

            <div class="flex items-center justify-between">
                <span class="px-1 text-sm text-muted">"Revenus ponctuels du mois"</span>
                <button class=BTN_GHOST aria-label="Ajouter un revenu ponctuel" on:click=move |_| open_income.set(true)>"Ajouter"</button>
            </div>
            {move || {
                let list = incomes.get().and_then(Result::ok).unwrap_or_default();
                (!list.is_empty()).then(|| view! {
                    <Card class="flex flex-col p-2">
                        {list.into_iter().map(|income| {
                            let open = income.clone();
                            view! {
                                <button
                                    class="flex min-h-12 items-center justify-between rounded-xl px-2 text-left active:bg-surface-raised"
                                    on:click=move |_| editing_income.set(Some(open.clone()))
                                >
                                    <span class="flex flex-col">
                                        <span class="text-sm">{income.label}</span>
                                        <span class="text-xs text-muted">{format::date(income.date)}</span>
                                    </span>
                                    <Amount value=income.amount class="text-gain" />
                                </button>
                            }
                        }).collect_view()}
                    </Card>
                })
            }}
        </section>

        <SalarySheet open=new_salary existing=None on_saved=on_saved />
        <IncomeSheet open=open_income checking=checking existing=None on_saved=on_saved />
        {move || editing_salary.get().map(|entry| {
            let open = RwSignal::new(true);
            Effect::new(move |_| if !open.get() { editing_salary.set(None) });
            view! { <SalarySheet open=open existing=Some(entry) on_saved=on_saved /> }
        })}
        {move || editing_income.get().map(|income| {
            let open = RwSignal::new(true);
            Effect::new(move |_| if !open.get() { editing_income.set(None) });
            view! { <IncomeSheet open=open checking=checking existing=Some(income) on_saved=on_saved /> }
        })}
    }
}

#[component]
fn SalarySheet(
    open: RwSignal<bool>,
    existing: Option<SalaryDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let editing = existing.is_some();
    let amount = RwSignal::new(
        existing
            .as_ref()
            .map(|s| format::quantity(s.net_amount))
            .unwrap_or_default(),
    );
    let month = RwSignal::new(
        existing
            .as_ref()
            .map_or_else(current_month, |s| s.effective_from.clone()),
    );
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let id = StoredValue::new(existing.map(|s| s.id));
    let submit = move |_| {
        let net_amount = match format::parse_amount(&amount.get_untracked()) {
            Ok(amount) => amount,
            Err(message) => return error.set(Some(message)),
        };
        let input = SalaryInput {
            id: id.get_value(),
            effective_from: month.get_untracked(),
            net_amount,
        };
        run_action(busy, error, api::upsert_salary(input), move |_: String| {
            open.set(false);
            on_saved.run(());
        });
    };
    let remove = move |_| {
        let Some(id) = id.get_value() else { return };
        run_action(busy, error, api::delete_salary(IdInput { id }), move |()| {
            open.set(false);
            on_saved.run(());
        });
    };
    view! {
        <BottomSheet open=open title=if editing { "Salaire" } else { "Nouveau salaire" }>
            <div class="flex flex-col gap-4">
                <Field label="Salaire net mensuel (net à payer)" value=amount input_mode="decimal" suffix="€" />
                <Field label="À partir de" value=month input_type="month" />
                <p class="text-xs text-muted">"Le salaire s'applique à ce mois et aux suivants, jusqu'au prochain changement."</p>
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit>"Enregistrer"</button>
                {editing.then(|| view! { <button class=BTN_DANGER disabled=move || busy.get() on:click=remove>"Supprimer"</button> })}
            </div>
        </BottomSheet>
    }
}

#[component]
fn IncomeSheet(
    open: RwSignal<bool>,
    checking: Signal<Vec<AccountOptionDto>>,
    existing: Option<OneOffIncomeDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let editing = existing.is_some();
    let account: AccountSelection = RwSignal::new(existing.as_ref().map(|i| i.account_id.clone()));
    let label = RwSignal::new(
        existing
            .as_ref()
            .map(|i| i.label.clone())
            .unwrap_or_default(),
    );
    let amount = RwSignal::new(
        existing
            .as_ref()
            .map(|i| format::quantity(i.amount))
            .unwrap_or_default(),
    );
    let date = RwSignal::new(
        existing
            .as_ref()
            .map_or_else(today, |i| i.date)
            .format("%Y-%m-%d")
            .to_string(),
    );
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let id = StoredValue::new(existing.map(|i| i.id));
    let submit = move |_| {
        let parsed_amount = match format::parse_amount(&amount.get_untracked()) {
            Ok(amount) => amount,
            Err(message) => return error.set(Some(message)),
        };
        let Ok(date) = NaiveDate::parse_from_str(&date.get_untracked(), "%Y-%m-%d") else {
            return error.set(Some("Date invalide".into()));
        };
        let input = OneOffIncomeInput {
            id: id.get_value(),
            label: label.get_untracked(),
            amount: parsed_amount,
            date,
            account_id: resolve(account.get_untracked(), &checking.get_untracked()),
        };
        run_action(
            busy,
            error,
            api::upsert_one_off_income(input),
            move |_: String| {
                if !editing {
                    label.set(String::new());
                    amount.set(String::new());
                    account.set(None);
                }
                open.set(false);
                on_saved.run(());
            },
        );
    };
    let remove = move |_| {
        let Some(id) = id.get_value() else { return };
        run_action(
            busy,
            error,
            api::delete_one_off_income(IdInput { id }),
            move |()| {
                open.set(false);
                on_saved.run(());
            },
        );
    };
    view! {
        <BottomSheet open=open title=if editing { "Revenu ponctuel" } else { "Nouveau revenu ponctuel" }>
            <div class="flex flex-col gap-4">
                <Field label="Libellé" value=label placeholder="Prime de fin d'année" />
                <Field label="Montant" value=amount input_mode="decimal" suffix="€" />
                <Field label="Date" value=date input_type="date" />
                <AccountChoice label="Versé sur" options=checking selection=account />
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit>"Enregistrer"</button>
                {editing.then(|| view! { <button class=BTN_DANGER disabled=move || busy.get() on:click=remove>"Supprimer"</button> })}
            </div>
        </BottomSheet>
    }
}
