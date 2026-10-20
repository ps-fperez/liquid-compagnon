//! Projection de l'épargne à 3, 6 et 12 mois selon trois scénarios.

use lc_shared_dto::{
    AccountDto, AccountListDto, ErrorDto, IncludeArchivedInput, ProjectionDto, ScenarioDto,
    ScenarioProjectionDto, ScenarioRatesDto, ScenarioRatesInput,
};
use leptos::prelude::*;

use crate::api;
use crate::charts::{LineChart, Series, to_f64};
use crate::components::style::{BTN_GHOST, BTN_PRIMARY, SECTION_TITLE};
use crate::components::{Amount, BottomSheet, Card, EmptyState, ErrorBanner, Field, Skeleton};
use crate::format;
use crate::util::{load, pricing_issue_message, run_action};

const HORIZONS: [usize; 3] = [3, 6, 12];

fn scenario_label(scenario: ScenarioDto) -> &'static str {
    match scenario {
        ScenarioDto::Pessimistic => "Pessimiste",
        ScenarioDto::Neutral => "Neutre",
        ScenarioDto::Optimistic => "Optimiste",
    }
}

fn scenario_color(scenario: ScenarioDto) -> &'static str {
    match scenario {
        ScenarioDto::Pessimistic => "#60A5FA",
        ScenarioDto::Neutral => "#A855F7",
        ScenarioDto::Optimistic => "#C084FC",
    }
}

fn at(values: &[lc_shared_dto::Decimal], months: usize) -> lc_shared_dto::Decimal {
    values
        .get(months.min(values.len().saturating_sub(1)))
        .copied()
        .unwrap_or_default()
}

#[component]
pub fn ProjectionPage() -> impl IntoView {
    let data = RwSignal::new(None::<Result<ProjectionDto, ErrorDto>>);
    let accounts = RwSignal::new(None::<Result<AccountListDto, ErrorDto>>);
    let horizon = RwSignal::new(12usize);
    let scenario = RwSignal::new(ScenarioDto::Neutral);
    let reload = Callback::new(move |_: ()| {
        load(data, api::get_projection);
        load(accounts, || {
            api::list_accounts(IncludeArchivedInput {
                include_archived: false,
            })
        });
    });
    reload.run(());

    view! {
        <h1 class="px-1 text-2xl font-medium">"Projection"</h1>
        <div class="flex rounded-xl bg-surface p-1" role="tablist" aria-label="Horizon">
            {HORIZONS.into_iter().map(|months| view! {
                <button
                    role="tab"
                    aria-selected=move || (horizon.get() == months).to_string()
                    class=move || format!(
                        "min-h-10 flex-1 rounded-lg text-sm {}",
                        if horizon.get() == months { "bg-surface-raised text-ink shadow-glow-primary" } else { "text-muted" },
                    )
                    on:click=move |_| horizon.set(months)
                >
                    {format!("{months} mois")}
                </button>
            }).collect_view()}
        </div>
        {move || match data.get() {
            None => view! { <Skeleton class="h-64" /> }.into_any(),
            Some(Err(error)) => view! { <ErrorBanner message=error.message /> }.into_any(),
            Some(Ok(projection)) => {
                let list = accounts.get().and_then(Result::ok).map(|l| l.accounts).unwrap_or_default();
                view! { <ProjectionView projection=projection accounts=list horizon=horizon scenario=scenario on_saved=reload /> }.into_any()
            }
        }}
    }
}

#[component]
fn ProjectionView(
    projection: ProjectionDto,
    accounts: Vec<AccountDto>,
    horizon: RwSignal<usize>,
    scenario: RwSignal<ScenarioDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let scenarios = StoredValue::new(projection.scenarios.clone());
    let editing = RwSignal::new(None::<AccountDto>);
    let accounts = StoredValue::new(accounts);
    let find = move |s: ScenarioDto| {
        scenarios.with_value(|all| all.iter().find(|p| p.scenario == s).cloned())
    };

    let chart = move || {
        let months = horizon.get();
        let series = [
            ScenarioDto::Pessimistic,
            ScenarioDto::Optimistic,
            ScenarioDto::Neutral,
        ]
        .into_iter()
        .filter_map(find)
        .map(|p: ScenarioProjectionDto| Series {
            label: scenario_label(p.scenario).into(),
            color: scenario_color(p.scenario).into(),
            values: p
                .totals
                .iter()
                .take(months + 1)
                .map(|v| to_f64(*v))
                .collect(),
            emphasized: p.scenario == ScenarioDto::Neutral,
            dashed: p.scenario != ScenarioDto::Neutral,
        })
        .collect::<Vec<_>>();
        let labels = vec![
            "Aujourd'hui".to_string(),
            format!("{} mois", months / 2),
            format!("{months} mois"),
        ];
        view! { <LineChart series=series x_labels=labels description="Projection du patrimoine selon trois scénarios" /> }
    };

    let excess = projection.contributions_excess;
    let issues = projection
        .pricing_issues
        .iter()
        .map(pricing_issue_message)
        .collect::<Vec<_>>();

    view! {
        {issues.into_iter().map(|message| view! { <ErrorBanner message=message /> }).collect_view()}
        <Card>{chart}</Card>

        <div class="grid grid-cols-3 gap-2">
            {[ScenarioDto::Pessimistic, ScenarioDto::Neutral, ScenarioDto::Optimistic].into_iter().map(|s| {
                let neutral = s == ScenarioDto::Neutral;
                view! {
                    <div class=format!(
                        "flex flex-col gap-1 rounded-2xl border p-3 {}",
                        if neutral { "border-primary bg-primary/10 shadow-glow-primary" } else { "border-line bg-surface" },
                    )>
                        <span class="text-xs text-muted">{scenario_label(s)}</span>
                        {move || find(s).map(|p| view! { <Amount value=at(&p.totals, horizon.get()) class="text-sm font-semibold" /> })}
                    </div>
                }
            }).collect_view()}
        </div>

        {excess.map(|amount| view! {
            <ErrorBanner message=format!("Vos versements programmés dépassent votre capacité d'épargne de {}", format::amount(amount)) />
        })}

        <Card class="flex flex-col gap-2">
            <div class="flex items-center justify-between">
                <span class="text-sm text-muted">"Capacité d'épargne mensuelle"</span>
                <Amount value=projection.savings_capacity />
            </div>
            <div class="flex items-center justify-between">
                <span class="text-sm text-muted">"Versements programmés"</span>
                <Amount value=projection.total_contributions />
            </div>
            <div class="flex items-center justify-between">
                <span class="text-sm text-muted">"Épargne non affectée à l'horizon"</span>
                {move || find(scenario.get()).map(|p| view! { <Amount value=at(&p.unallocated, horizon.get()) /> })}
            </div>
        </Card>

        <section class="flex flex-col gap-2">
            <div class="flex items-center justify-between">
                <h2 class=SECTION_TITLE>"Par compte"</h2>
                <div class="flex gap-1 text-xs">
                    {[ScenarioDto::Pessimistic, ScenarioDto::Neutral, ScenarioDto::Optimistic].into_iter().map(|s| view! {
                        <button
                            class=move || format!(
                                "min-h-9 rounded-lg px-2 {}",
                                if scenario.get() == s { "bg-surface-raised text-ink" } else { "text-muted" },
                            )
                            on:click=move |_| scenario.set(s)
                        >
                            {scenario_label(s)}
                        </button>
                    }).collect_view()}
                </div>
            </div>
            {move || {
                let Some(selected) = find(scenario.get()) else {
                    return view! { <EmptyState title="Aucun compte" body="Ajoutez des comptes pour voir leur projection." /> }.into_any();
                };
                if selected.accounts.is_empty() {
                    return view! { <Card><EmptyState title="Aucun compte" body="Ajoutez des comptes pour voir leur projection." /></Card> }.into_any();
                }
                let months = horizon.get();
                view! {
                    <Card class="flex flex-col p-1">
                        {selected.accounts.into_iter().map(|trajectory| {
                            let account = accounts.with_value(|list| list.iter().find(|a| a.id == trajectory.account_id).cloned());
                            view! {
                                <div class="flex min-h-14 items-center gap-3 rounded-xl px-3 py-2">
                                    <span class="size-2.5 shrink-0 rounded-full" style=format!("background-color: {}", trajectory.color)></span>
                                    <span class="flex min-w-0 flex-1 flex-col">
                                        <span class="truncate text-sm">{trajectory.name}</span>
                                        <span class="text-xs text-muted">"aujourd'hui " {format::amount(at(&trajectory.values, 0))}</span>
                                    </span>
                                    <Amount value=at(&trajectory.values, months) class="font-medium" />
                                    {account.map(|account| view! {
                                        <button class=format!("{BTN_GHOST} px-2 text-xs") on:click=move |_| editing.set(Some(account.clone()))>
                                            "Rendements"
                                        </button>
                                    })}
                                </div>
                            }
                        }).collect_view()}
                    </Card>
                }
                .into_any()
            }}
        </section>

        {move || editing.get().map(|account| {
            let open = RwSignal::new(true);
            Effect::new(move |_| if !open.get() { editing.set(None) });
            view! { <RatesSheet open=open account=account on_saved=on_saved /> }
        })}
    }
}

fn parse_rates(
    pessimistic: &str,
    neutral: &str,
    optimistic: &str,
) -> Result<ScenarioRatesDto, String> {
    Ok(ScenarioRatesDto {
        pessimistic: format::parse_decimal(pessimistic)?,
        neutral: format::parse_decimal(neutral)?,
        optimistic: format::parse_decimal(optimistic)?,
    })
}

/// Rendements annuels d'un compte dans chaque scénario (en %).
#[component]
fn RatesSheet(open: RwSignal<bool>, account: AccountDto, on_saved: Callback<()>) -> impl IntoView {
    let rates = account.scenario_rates;
    let pessimistic = RwSignal::new(format::quantity(rates.pessimistic));
    let neutral = RwSignal::new(format::quantity(rates.neutral));
    let optimistic = RwSignal::new(format::quantity(rates.optimistic));
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let account_id = StoredValue::new(account.id);
    let submit = move |_| {
        let parsed = parse_rates(
            &pessimistic.get_untracked(),
            &neutral.get_untracked(),
            &optimistic.get_untracked(),
        );
        let rates = match parsed {
            Ok(rates) => rates,
            Err(message) => return error.set(Some(message)),
        };
        run_action(
            busy,
            error,
            api::set_scenario_rates(ScenarioRatesInput {
                account_id: account_id.get_value(),
                rates,
            }),
            move |()| {
                open.set(false);
                on_saved.run(());
            },
        );
    };
    view! {
        <BottomSheet open=open title=format!("Rendements · {}", account.name)>
            <div class="flex flex-col gap-4">
                <p class="text-sm text-muted">"Rendement annuel estimé dans chaque scénario. Il peut être négatif (ex. −40 % pour la crypto)."</p>
                <Field label="Pessimiste" value=pessimistic input_mode="decimal" suffix="%" />
                <Field label="Neutre" value=neutral input_mode="decimal" suffix="%" />
                <Field label="Optimiste" value=optimistic input_mode="decimal" suffix="%" />
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit>"Enregistrer"</button>
            </div>
        </BottomSheet>
    }
}
