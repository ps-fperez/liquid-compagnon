//! Budget : synthèse du mois, dépenses, revenus, charges fixes et catégories.

mod account_choice;
mod categories;
mod charges;
mod expenses;
mod incomes;

use lc_shared_dto::{
    AccountOptionDto, BudgetSummaryDto, CategoryDto, ChargeDto, ErrorDto, ExpenseFilterInput,
    ExpenseListDto, IncludeArchivedInput, MonthInput, OneOffIncomeDto, SalaryDto,
};
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::api;
use crate::components::style::{BTN_ICON, BTN_SECONDARY};
use crate::components::{Amount, Card, ErrorBanner, Skeleton};
use crate::format;
use crate::icons::{Icon, IconName};
use crate::util::{current_month, load};

use categories::CategoriesSheet;
use charges::ChargesSection;
use expenses::ExpensesSection;
use incomes::IncomesSection;

#[component]
pub fn BudgetPage() -> impl IntoView {
    let query = use_query_map();
    // La saisie rapide (bouton « + ») ouvre le bon formulaire, y compris depuis cet écran.
    let expense_open = RwSignal::new(false);
    let income_open = RwSignal::new(false);
    Effect::new(move |_| match query.with(|q| q.get("saisie")).as_deref() {
        Some("depense") => expense_open.set(true),
        Some("revenu") => income_open.set(true),
        _ => {}
    });
    let categories_open = RwSignal::new(false);

    let month = RwSignal::new(current_month());
    let filter = RwSignal::new(None::<String>);
    let summary = RwSignal::new(None::<Result<BudgetSummaryDto, ErrorDto>>);
    let expenses = RwSignal::new(None::<Result<ExpenseListDto, ErrorDto>>);
    let categories = RwSignal::new(None::<Result<Vec<CategoryDto>, ErrorDto>>);
    let salaries = RwSignal::new(None::<Result<Vec<SalaryDto>, ErrorDto>>);
    let incomes = RwSignal::new(None::<Result<Vec<OneOffIncomeDto>, ErrorDto>>);
    let charges = RwSignal::new(None::<Result<Vec<ChargeDto>, ErrorDto>>);
    let checking = RwSignal::new(None::<Result<Vec<AccountOptionDto>, ErrorDto>>);

    let reload = Callback::new(move |_: ()| {
        let selected = month.get_untracked();
        let category_id = filter.get_untracked();
        let for_summary = selected.clone();
        load(summary, move || {
            api::budget_summary(MonthInput {
                month: Some(for_summary),
            })
        });
        let for_expenses = selected.clone();
        load(expenses, move || {
            api::list_expenses(ExpenseFilterInput {
                month: for_expenses,
                category_id,
            })
        });
        load(categories, || {
            api::list_categories(IncludeArchivedInput {
                include_archived: true,
            })
        });
        load(salaries, api::salary_history);
        load(incomes, move || {
            api::list_one_off_incomes(MonthInput {
                month: Some(selected),
            })
        });
        load(charges, api::list_charges);
        load(checking, api::checking_accounts);
    });
    Effect::new(move |_| {
        month.track();
        filter.track();
        reload.run(());
    });

    let category_list =
        Signal::derive(move || categories.get().and_then(Result::ok).unwrap_or_default());
    let checking_list =
        Signal::derive(move || checking.get().and_then(Result::ok).unwrap_or_default());

    view! {
        <div class="flex items-center justify-between">
            <button class=BTN_ICON aria-label="Mois précédent" on:click=move |_| month.update(|m| *m = format::shift_month(m, -1).unwrap_or_else(|| m.clone()))>
                <Icon name=IconName::ChevronLeft />
            </button>
            <h1 class="text-xl font-medium capitalize">{move || format::month_label(&month.get())}</h1>
            <button class=BTN_ICON aria-label="Mois suivant" on:click=move |_| month.update(|m| *m = format::shift_month(m, 1).unwrap_or_else(|| m.clone()))>
                <Icon name=IconName::ChevronRight />
            </button>
        </div>

        {move || match summary.get() {
            None => view! { <Skeleton class="h-40" /> }.into_any(),
            Some(Err(error)) => view! { <ErrorBanner message=error.message /> }.into_any(),
            Some(Ok(summary)) => view! { <SummaryCard summary=summary /> }.into_any(),
        }}

        <ExpensesSection expenses=expenses filter=filter categories=category_list checking=checking_list open_new=expense_open on_saved=reload />
        <IncomesSection summary=summary salaries=salaries incomes=incomes checking=checking_list open_income=income_open on_saved=reload />
        <ChargesSection charges=charges categories=category_list on_saved=reload />

        <button class=BTN_SECONDARY on:click=move |_| categories_open.set(true)>"Gérer les catégories"</button>
        <CategoriesSheet open=categories_open categories=category_list on_saved=reload />
    }
}

#[component]
fn SummaryCard(summary: BudgetSummaryDto) -> impl IntoView {
    let capacity = summary.savings_capacity;
    let capacity_color = if capacity.is_sign_negative() && !capacity.is_zero() {
        "text-loss"
    } else if capacity.is_zero() {
        ""
    } else {
        "text-gain"
    };
    let row = |label: &'static str, value| {
        view! {
            <div class="flex items-center justify-between">
                <span class="text-sm text-muted">{label}</span>
                <Amount value=value />
            </div>
        }
    };
    view! {
        <Card class="flex flex-col gap-2">
            {row("Salaire", summary.salary)}
            {row("Revenus ponctuels", summary.one_off_incomes)}
            {row("Charges fixes (par mois)", summary.fixed_charges)}
            {row("Dépenses du mois", summary.expenses)}
            {row("Dépenses moyennes (3 mois)", summary.average_expenses)}
            <div class="mt-1 flex items-center justify-between border-t border-line pt-3">
                <span class="font-medium">"Capacité d'épargne"</span>
                <Amount value=capacity class=format!("text-lg font-semibold {capacity_color}") />
            </div>
            {(capacity.is_sign_negative() && !capacity.is_zero()).then(|| view! {
                <p class="text-sm text-loss">"Vos charges et dépenses dépassent votre salaire."</p>
            })}
        </Card>
    }
}
