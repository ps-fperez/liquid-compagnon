//! Tableau de bord : patrimoine total, comptes, budget du mois, évolution.

use std::collections::HashMap;
use std::time::Duration;

use lc_shared_dto::{AccountLineDto, DashboardDto, Decimal, HistoryPointDto, RefreshInput};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use crate::api;
use crate::charts::{AllocationBar, AllocationSegment, LineChart, Series, to_f64};
use crate::components::style::{BTN_ICON, SECTION_TITLE};
use crate::components::{
    Amount, Card, EmptyState, ErrorBanner, GlowCard, ProvenanceBadge, Skeleton,
};
use crate::format;
use crate::icons::{Icon, IconName};
use crate::util::{current_month, pricing_issue_message};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Period {
    SixMonths,
    OneYear,
    All,
}

impl Period {
    fn label(self) -> &'static str {
        match self {
            Period::SixMonths => "6 mois",
            Period::OneYear => "1 an",
            Period::All => "Tout",
        }
    }

    /// Premier mois affiché (`None` : tout l'historique).
    fn first_month(self) -> Option<String> {
        let back = match self {
            Period::SixMonths => 5,
            Period::OneYear => 11,
            Period::All => return None,
        };
        format::shift_month(&current_month(), -back)
    }
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    let data = RwSignal::new(None::<Result<DashboardDto, lc_shared_dto::ErrorDto>>);
    let refreshing = RwSignal::new(false);
    // Sens de la dernière variation de valeur par compte, pour le flash après rafraîchissement.
    let flashes = RwSignal::new(HashMap::<String, bool>::new());
    let previous = StoredValue::new(HashMap::<String, Decimal>::new());

    let fetch = move |force: bool| {
        refreshing.set(force);
        spawn_local(async move {
            let result = api::get_dashboard(RefreshInput {
                force_refresh: force,
            })
            .await;
            if let Ok(dashboard) = &result {
                let before = previous.get_value();
                let changed: HashMap<String, bool> = dashboard
                    .accounts
                    .iter()
                    .filter_map(|line| {
                        let old = before.get(&line.id)?;
                        (*old != line.value).then(|| (line.id.clone(), line.value > *old))
                    })
                    .collect();
                previous.set_value(
                    dashboard
                        .accounts
                        .iter()
                        .map(|l| (l.id.clone(), l.value))
                        .collect(),
                );
                if force && !changed.is_empty() {
                    flashes.set(changed);
                    set_timeout(
                        move || flashes.set(HashMap::new()),
                        Duration::from_millis(1200),
                    );
                }
            }
            data.set(Some(result));
            refreshing.set(false);
        });
    };
    fetch(false);

    view! {
        {move || match data.get() {
            None => view! {
                <Skeleton class="h-44" />
                <Skeleton class="h-56" />
            }
            .into_any(),
            Some(Err(error)) => view! { <ErrorBanner message=error.message /> }.into_any(),
            Some(Ok(dashboard)) => view! {
                <Overview dashboard=dashboard refreshing=refreshing flashes=flashes on_refresh=Callback::new(move |_| fetch(true)) on_snapshot=Callback::new(move |_| spawn_local(async move { let _ = api::take_manual_snapshot().await; fetch(false); })) />
            }
            .into_any(),
        }}
    }
}

#[component]
fn Overview(
    dashboard: DashboardDto,
    refreshing: RwSignal<bool>,
    flashes: RwSignal<HashMap<String, bool>>,
    on_refresh: Callback<()>,
    on_snapshot: Callback<()>,
) -> impl IntoView {
    let segments = dashboard
        .accounts
        .iter()
        .map(|line| AllocationSegment {
            color: line.color.clone(),
            share: to_f64(line.share_percent),
        })
        .collect::<Vec<_>>();
    let issues = dashboard
        .pricing_issues
        .iter()
        .map(pricing_issue_message)
        .collect::<Vec<_>>();
    let has_accounts = !dashboard.accounts.is_empty();

    view! {
        <GlowCard>
            <div class="flex items-start justify-between">
                <p class="text-sm text-muted">"Patrimoine total"</p>
                <button
                    class=move || format!("{BTN_ICON} -m-2 {}", if refreshing.get() { "animate-spin text-primary" } else { "" })
                    aria-label="Actualiser les cours"
                    on:click=move |_| on_refresh.run(())
                >
                    <Icon name=IconName::Refresh class="size-5" />
                </button>
            </div>
            <Amount value=dashboard.total class="mt-1 block text-4xl font-semibold tracking-tight" />
            // Une variation n'a de sens que par rapport à une référence non nulle (pas au tout premier mois).
            {dashboard.variation.filter(|v| v.percent.is_some()).map(|variation| view! {
                <p class="mt-2 flex items-center gap-2 text-sm">
                    <Amount value=variation.amount signed=true colored=true />
                    {variation.percent.map(|percent| {
                        let color = if percent.is_sign_negative() { "text-loss" } else { "text-gain" };
                        view! { <span class=color>{format::percent_signed(percent, 1)}</span> }
                    })}
                    <span class="text-muted">"ce mois-ci"</span>
                </p>
            })}
            {has_accounts.then(|| view! { <div class="mt-5"><AllocationBar segments=segments /></div> })}
        </GlowCard>

        {issues.into_iter().map(|message| view! { <ErrorBanner message=message /> }).collect_view()}
        {dashboard.export_reminder_due.then(|| view! {
            <A href="/reglages" attr:class="rounded-xl border border-primary/30 bg-primary/10 px-4 py-3 text-sm text-primary-soft">
                "Pensez à exporter une sauvegarde : vos données n'existent que sur ce téléphone."
            </A>
        })}

        <section class="flex flex-col gap-2">
            <h2 class=SECTION_TITLE>"Comptes"</h2>
            {if has_accounts {
                view! {
                    <Card class="flex flex-col p-1">
                        {dashboard
                            .accounts
                            .into_iter()
                            .map(|line| view! { <AccountRow line=line flashes=flashes /> })
                            .collect_view()}
                    </Card>
                }
                .into_any()
            } else {
                view! {
                    <EmptyState
                        title="Ajoutez votre premier compte"
                        body="Livret A, LDDS, PEA, PEI ou portefeuille crypto."
                        action="Ajouter un compte"
                        href="/comptes?nouveau=1"
                    />
                }
                .into_any()
            }}
        </section>

        <MonthSummary dashboard_budget=dashboard.budget />
        <History points=dashboard.history on_snapshot=on_snapshot />
    }
}

#[component]
fn AccountRow(line: AccountLineDto, flashes: RwSignal<HashMap<String, bool>>) -> impl IntoView {
    let id = line.id.clone();
    let flash = move || match flashes.with(|f| f.get(&id).copied()) {
        Some(true) => "animate-flash-gain",
        Some(false) => "animate-flash-loss",
        None => "",
    };
    view! {
        <A
            href=format!("/comptes/{}", line.id)
            attr:class=move || format!("flex min-h-16 items-center gap-3 rounded-xl px-3 py-2 transition active:bg-surface-raised {}", flash())
        >
            <span class="size-2.5 shrink-0 rounded-full" style=format!("background-color: {}", line.color)></span>
            <span class="flex min-w-0 flex-1 flex-col items-start gap-1">
                <span class="truncate font-medium">{line.name}</span>
                <ProvenanceBadge provenance=line.provenance />
            </span>
            <span class="flex flex-col items-end gap-0.5">
                <Amount value=line.value class="font-medium" />
                <span class="flex gap-2 text-xs text-muted">
                    {format::percent(line.share_percent, 1)}
                    {line.variation.filter(|v| !v.amount.is_zero()).map(|v| view! { <Amount value=v.amount signed=true colored=true /> })}
                </span>
            </span>
        </A>
    }
}

#[component]
fn MonthSummary(dashboard_budget: lc_shared_dto::BudgetSummaryDto) -> impl IntoView {
    let budget = dashboard_budget;
    let negative = budget.savings_capacity.is_sign_negative() && !budget.savings_capacity.is_zero();
    let capacity_color = if negative {
        "font-medium text-loss"
    } else if budget.savings_capacity.is_zero() {
        "font-medium"
    } else {
        "font-medium text-gain"
    };
    view! {
        <section class="flex flex-col gap-2">
            <div class="flex items-center justify-between">
                <h2 class=SECTION_TITLE>{format!("Budget · {}", format::month_label(&budget.month))}</h2>
                <A href="/budget" attr:class="text-sm text-secondary-light">"Détail"</A>
            </div>
            <div class="grid grid-cols-2 gap-2">
                <Stat label="Revenus du mois" value=budget.total_income />
                <Stat label="Charges fixes" value=budget.fixed_charges />
                <Stat label="Dépenses du mois" value=budget.expenses />
                <div class=format!(
                    "flex flex-col gap-1 rounded-2xl border p-3 {}",
                    if negative { "border-loss/40 bg-loss/5" } else { "border-line bg-surface" },
                )>
                    <span class="text-xs text-muted">"Capacité d'épargne"</span>
                    <Amount value=budget.savings_capacity class=capacity_color />
                </div>
            </div>
            {negative.then(|| view! {
                <p class="px-1 text-sm text-loss">"Vos charges et dépenses dépassent votre salaire."</p>
            })}
        </section>
    }
}

#[component]
fn Stat(label: &'static str, value: Decimal) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-1 rounded-2xl border border-line bg-surface p-3">
            <span class="text-xs text-muted">{label}</span>
            <Amount value=value class="font-medium" />
        </div>
    }
}

#[component]
fn History(points: Vec<HistoryPointDto>, on_snapshot: Callback<()>) -> impl IntoView {
    let period = RwSignal::new(Period::SixMonths);
    let points = StoredValue::new(points);
    view! {
        <section class="flex flex-col gap-2">
            <div class="flex items-center justify-between">
                <span class="flex items-center gap-1"><h2 class=SECTION_TITLE>"Évolution"</h2><button class="min-h-9 px-2 text-xs text-secondary-light" on:click=move |_| on_snapshot.run(())>"Figer ce mois"</button></span>
                <div class="flex rounded-xl bg-surface p-1 text-xs" role="tablist">
                    {[Period::SixMonths, Period::OneYear, Period::All]
                        .into_iter()
                        .map(|p| view! {
                            <button
                                role="tab"
                                aria-selected=move || (period.get() == p).to_string()
                                class=move || format!(
                                    "min-h-9 rounded-lg px-3 {}",
                                    if period.get() == p { "bg-surface-raised text-ink" } else { "text-muted" },
                                )
                                on:click=move |_| period.set(p)
                            >
                                {p.label()}
                            </button>
                        })
                        .collect_view()}
                </div>
            </div>
            <Card>
                {move || {
                    let first = period.get().first_month();
                    let visible: Vec<HistoryPointDto> = points
                        .get_value()
                        .into_iter()
                        .filter(|p| first.as_ref().is_none_or(|first| p.month >= *first))
                        .collect();
                    if visible.len() < 2 {
                        return view! {
                            <p class="py-8 text-center text-sm text-muted">"Votre courbe apparaîtra après votre deuxième mois"</p>
                        }
                        .into_any();
                    }
                    let labels = [0, (visible.len() - 1) / 2, visible.len() - 1]
                        .map(|i| format::month_short(&visible[i].month))
                        .to_vec();
                    let series = Series {
                        label: "Patrimoine".into(),
                        color: "#A855F7".into(),
                        values: visible.iter().map(|p| to_f64(p.total)).collect(),
                        emphasized: true,
                        dashed: false,
                    };
                    view! { <LineChart series=vec![series] x_labels=labels description="Évolution du patrimoine" /> }.into_any()
                }}
            </Card>
        </section>
    }
}
