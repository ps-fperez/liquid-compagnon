//! Objectifs d'épargne (spec savings-goals) : anneau de progression, liste dans Projection,
//! aperçu sur l'accueil, feuille de création avec l'effort mensuel en direct.

use lc_shared_dto::{
    AccountDto, AccountListDto, Decimal, GoalDto, GoalInput, GoalStatusDto, IdInput,
    IncludeArchivedInput,
};
use leptos::prelude::*;
use leptos_router::components::A;

use crate::api;
use crate::components::Busy;
use crate::components::amount_hint;
use crate::components::style::{BTN_DANGER, BTN_GHOST, BTN_PRIMARY, LABEL, SECTION_TITLE};
use crate::components::toast;
use crate::components::{Amount, BottomSheet, Card, EmptyState, Field, cascade};
use crate::format;
use crate::util::{Loaded, current_month, load, load_then, run_action};

fn status_look(status: GoalStatusDto) -> Option<(&'static str, &'static str)> {
    match status {
        GoalStatusDto::Reached => Some(("Atteint", "bg-gain/15 text-gain")),
        GoalStatusDto::OnTrack => Some(("Dans les temps", "bg-gain/15 text-gain")),
        GoalStatusDto::Late => Some(("En retard", "bg-amber-400/15 text-amber-300")),
        GoalStatusDto::Open => None,
    }
}

/// Félicite une fois par objectif qui vient d'être atteint (le backend ne le signale qu'une fois).
fn celebrate(goals: &[GoalDto]) {
    for goal in goals.iter().filter(|g| g.just_reached) {
        toast::success(format!("Objectif atteint : {}", goal.name));
    }
}

/// Anneau de progression qui se remplit depuis zéro à l'affichage ; il s'illumine à 100 %.
#[component]
pub fn GoalRing(
    percent: Decimal,
    #[prop(default = 64)] size: u32,
    #[prop(optional)] reached: bool,
) -> impl IntoView {
    let stroke = 6.0;
    let radius = f64::from(size) / 2.0 - stroke;
    let circumference = 2.0 * std::f64::consts::PI * radius;
    let fraction = (crate::charts::to_f64(percent) / 100.0).clamp(0.0, 1.0);
    let filled = RwSignal::new(false);
    request_animation_frame(move || {
        let _ = filled.try_set(true);
    });
    let center = f64::from(size) / 2.0;
    let color = if reached {
        "var(--color-gain)"
    } else {
        "var(--color-primary)"
    };
    view! {
        <span class=format!("relative inline-flex shrink-0 items-center justify-center {}", if reached { "animate-ring-glow" } else { "" }) style=format!("width: {size}px; height: {size}px")>
            <svg width=size height=size viewBox=format!("0 0 {size} {size}") class="-rotate-90" aria-hidden="true">
                <circle cx=center cy=center r=radius fill="none" stroke="var(--color-surface-raised)" stroke-width=stroke />
                <circle
                    cx=center cy=center r=radius fill="none" stroke=color stroke-width=stroke stroke-linecap="round"
                    stroke-dasharray=circumference
                    style=move || format!(
                        "stroke-dashoffset: {:.2}; transition: stroke-dashoffset 900ms cubic-bezier(0.33, 1, 0.68, 1)",
                        circumference * (1.0 - if filled.get() { fraction } else { 0.0 }),
                    )
                />
            </svg>
            <span class="absolute text-xs font-semibold tabular-nums">{format::percent(percent.round(), 0)}</span>
        </span>
    }
}

/// Objectifs de l'onglet Projection : liste détaillée et création.
#[component]
pub fn GoalsSection(#[prop(optional)] open_new: bool) -> impl IntoView {
    let goals: Loaded<Vec<GoalDto>> = RwSignal::new(None);
    let reload = Callback::new(move |_: ()| {
        load_then(goals, api::list_goals, |list: &Vec<GoalDto>| {
            celebrate(list)
        })
    });
    reload.run(());
    let new_open = RwSignal::new(open_new);
    let editing = RwSignal::new(None::<GoalDto>);
    view! {
        <section class="flex flex-col gap-2">
            <div class="flex items-center justify-between">
                <h2 class=SECTION_TITLE>"Objectifs"</h2>
                <button class=BTN_GHOST on:click=move |_| new_open.set(true)>"Nouvel objectif"</button>
            </div>
            {move || {
                let list = goals.get().and_then(Result::ok).unwrap_or_default();
                if list.is_empty() {
                    return view! {
                        <EmptyState
                            title="Fixez-vous un objectif"
                            body="Apport immobilier, voyage, matelas de sécurité : choisissez les comptes qui y contribuent, l'application suit la progression et la date d'arrivée."
                            action="Créer un objectif"
                            on_action=Callback::new(move |_| new_open.set(true))
                        />
                    }.into_any();
                }
                list.into_iter().enumerate().map(|(index, goal)| {
                    let open_it = goal.clone();
                    view! {
                        <button class="animate-rise text-left" style=cascade(index) on:click=move |_| editing.set(Some(open_it.clone()))>
                            <GoalCard goal=goal />
                        </button>
                    }
                }).collect_view().into_any()
            }}
        </section>
        <GoalSheet open=new_open existing=None on_saved=reload />
        {move || editing.get().map(|goal| {
            let open = RwSignal::new(true);
            Effect::new(move |_| if !open.get() { editing.set(None) });
            view! { <GoalSheet open=open existing=Some(goal) on_saved=reload /> }
        })}
    }
}

#[component]
fn GoalCard(goal: GoalDto) -> impl IntoView {
    let reached = goal.status == GoalStatusDto::Reached;
    let status = status_look(goal.status);
    let deadline = goal
        .deadline
        .as_deref()
        .map(|m| format!(" · objectif {}", format::month_label(m)));
    let returns_help = goal.status == GoalStatusDto::OnTrack
        && goal
            .required_monthly
            .is_some_and(|required| required > goal.contributed_monthly);
    let arrivals = ["Pessimiste", "Neutre", "Optimiste"]
        .into_iter()
        .zip(goal.arrivals.clone())
        .collect::<Vec<_>>();
    view! {
        <Card class="flex flex-col gap-3">
            <div class="flex items-center gap-3">
                <GoalRing percent=goal.percent size=64 reached=reached />
                <span class="flex min-w-0 flex-1 flex-col gap-1">
                    <span class="flex items-center justify-between gap-2">
                        <span class="truncate font-medium">{goal.name.clone()}</span>
                        {status.map(|(label, tone)| view! {
                            <span class=format!("shrink-0 rounded-full px-2 py-0.5 text-xs {tone}")>{label}</span>
                        })}
                    </span>
                    <span class="text-xs text-muted">
                        <Amount value=goal.current /> " sur " <Amount value=goal.target /> {deadline}
                    </span>
                </span>
            </div>
            {(!reached).then(|| view! {
                <div class="flex flex-col gap-1.5 text-sm">
                    {goal.required_monthly.map(|required| view! {
                        <span class="flex justify-between"><span class="text-muted">"Effort nécessaire"</span><span><Amount value=required /> "/mois"</span></span>
                    })}
                    <span class="flex justify-between"><span class="text-muted">"Versé aujourd'hui"</span><span><Amount value=goal.contributed_monthly /> "/mois"</span></span>
                    {returns_help.then(|| view! {
                        <span class="text-xs text-muted">"Les rendements comblent l'écart dans le scénario neutre."</span>
                    })}
                </div>
                <div class="grid grid-cols-3 gap-2 border-t border-line pt-3 text-center">
                    {arrivals.into_iter().map(|(label, arrival)| view! {
                        <span class="flex flex-col gap-0.5">
                            <span class="text-[11px] text-muted">{label}</span>
                            <span class="text-xs font-medium">{arrival.as_deref().map_or_else(|| "plus de 10 ans".to_string(), format::month_short)}</span>
                        </span>
                    }).collect_view()}
                </div>
            })}
            {(!goal.account_names.is_empty()).then(|| view! {
                <span class="text-xs text-muted">{format!("Comptes suivis : {}", goal.account_names.join(", "))}</span>
            })}
        </Card>
    }
}

/// Aperçu de l'accueil : anneaux défilant horizontalement et accès « Tout voir ».
#[component]
pub fn GoalsPreview() -> impl IntoView {
    let goals: Loaded<Vec<GoalDto>> = RwSignal::new(None);
    load_then(goals, api::list_goals, |list: &Vec<GoalDto>| {
        celebrate(list)
    });
    view! {
        <section class="flex flex-col gap-2">
            <div class="flex items-center justify-between">
                <h2 class=SECTION_TITLE>"Objectifs"</h2>
                <A href="/projection" attr:class="text-sm text-secondary-light">"Tout voir"</A>
            </div>
            {move || match goals.get() {
                None => None,
                Some(Err(_)) => None,
                Some(Ok(list)) if list.is_empty() => Some(view! {
                    <EmptyState
                        title="Fixez-vous un objectif"
                        body="Suivez un projet d'épargne à partir de vos comptes."
                        action="Créer un objectif"
                        href="/projection?objectif=nouveau"
                    />
                }.into_any()),
                Some(Ok(list)) => Some(view! {
                    <div class="-mx-4 flex snap-x gap-2 overflow-x-auto px-4 pb-1">
                        {list.into_iter().enumerate().map(|(index, goal)| {
                            let reached = goal.status == GoalStatusDto::Reached;
                            let caption = match (goal.status, goal.deadline.as_deref()) {
                                (GoalStatusDto::Reached, _) => "atteint".to_string(),
                                (GoalStatusDto::OnTrack, _) => "dans les temps".to_string(),
                                (GoalStatusDto::Late, _) => "en retard".to_string(),
                                (GoalStatusDto::Open, _) => goal.arrivals[1].as_deref().map_or_else(|| "plus de 10 ans".into(), format::month_label),
                            };
                            view! {
                                <A href="/projection" attr:class="flex w-36 shrink-0 snap-start animate-rise flex-col items-center gap-2 rounded-2xl border border-line bg-surface p-3 text-center" attr:style=cascade(index)>
                                    <GoalRing percent=goal.percent size=72 reached=reached />
                                    <span class="w-full truncate text-sm font-medium">{goal.name}</span>
                                    <span class="text-xs text-muted">{caption}</span>
                                </A>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()),
            }}
        </section>
    }
}

/// Création (`existing` absent) ou modification d'un objectif.
#[component]
fn GoalSheet(
    open: RwSignal<bool>,
    existing: Option<GoalDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let title = if existing.is_some() {
        "Objectif"
    } else {
        "Nouvel objectif"
    };
    let existing = StoredValue::new(existing);
    view! {
        <BottomSheet open=open title=title>
            {move || view! { <GoalForm open=open existing=existing.get_value() on_saved=on_saved /> }}
        </BottomSheet>
    }
}

#[component]
fn GoalForm(
    open: RwSignal<bool>,
    existing: Option<GoalDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let accounts: Loaded<AccountListDto> = RwSignal::new(None);
    load(accounts, || {
        api::list_accounts(IncludeArchivedInput {
            include_archived: false,
        })
    });
    let editing = existing.is_some();
    let name = RwSignal::new(
        existing
            .as_ref()
            .map(|g| g.name.clone())
            .unwrap_or_default(),
    );
    let target = RwSignal::new(
        existing
            .as_ref()
            .map(|g| format::quantity(g.target))
            .unwrap_or_default(),
    );
    let deadline = RwSignal::new(
        existing
            .as_ref()
            .and_then(|g| g.deadline.clone())
            .unwrap_or_default(),
    );
    let selected = RwSignal::new(
        existing
            .as_ref()
            .map(|g| g.account_ids.clone())
            .unwrap_or_default(),
    );
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let goal_id = StoredValue::new(existing.map(|g| g.id));

    let tracked = Signal::derive(move || {
        accounts
            .get()
            .and_then(Result::ok)
            .map(|l| {
                l.accounts
                    .into_iter()
                    .filter(|a| a.account_type != lc_shared_dto::AccountTypeDto::Checking)
                    .collect::<Vec<AccountDto>>()
            })
            .unwrap_or_default()
    });
    // Effort mensuel pendant la saisie : (cible − valeur des comptes cochés) / mois restants.
    let effort = Signal::derive(move || {
        let target = format::parse_amount(&target.get()).ok()?;
        let months = format::months_between(&current_month(), &deadline.get())?.max(1);
        let current: Decimal = tracked
            .get()
            .iter()
            .filter(|a| selected.with(|s| s.contains(&a.id)))
            .map(|a| a.value)
            .sum();
        let remaining = (target - current).max(Decimal::ZERO);
        Some(format!(
            "Effort nécessaire : {}/mois pendant {months} mois",
            format::amount((remaining / Decimal::from(months)).round_dp(2))
        ))
    });

    let submit = move |_| {
        let target = match format::parse_amount(&target.get_untracked()) {
            Ok(amount) => amount,
            Err(message) => return error.set(Some(message)),
        };
        let input = GoalInput {
            id: goal_id.get_value(),
            name: name.get_untracked(),
            target,
            deadline: Some(deadline.get_untracked()).filter(|d| !d.trim().is_empty()),
            account_ids: selected.get_untracked(),
        };
        run_action(busy, error, api::save_goal(input), move |_: String| {
            toast::success(if editing {
                "Objectif modifié"
            } else {
                "Objectif créé"
            });
            open.set(false);
            on_saved.run(());
        });
    };
    let remove = move |_| {
        let Some(id) = goal_id.get_value() else {
            return;
        };
        run_action(busy, error, api::delete_goal(IdInput { id }), move |()| {
            toast::success("Objectif supprimé");
            open.set(false);
            on_saved.run(());
        });
    };

    view! {
        <div class="flex flex-col gap-4">
            <Field label="Nom" value=name placeholder="Apport immobilier" />
            <Field label="Montant visé" value=target input_mode="decimal" suffix="€" hint=Signal::derive(move || Some(amount_hint(&target.get()))) />
            <Field label="Échéance (facultatif)" value=deadline input_type="month" hint=effort />
            <div class="flex flex-col gap-1.5">
                <span class=LABEL>"Comptes suivis"</span>
                <div class="flex flex-wrap gap-2">
                    {move || tracked.get().into_iter().map(|account| {
                        let id = account.id.clone();
                        let check = account.id.clone();
                        let active = Memo::new(move |_| selected.with(|s| s.contains(&check)));
                        view! {
                            <button
                                class=move || format!(
                                    "min-h-11 rounded-xl border px-4 text-sm transition {}",
                                    if active.get() { "border-primary bg-primary/15 text-ink" } else { "border-line text-muted" },
                                )
                                aria-pressed=move || active.get().to_string()
                                on:click=move |_| selected.update(|s| {
                                    if let Some(i) = s.iter().position(|x| *x == id) { s.remove(i); } else { s.push(id.clone()); }
                                })
                            >
                                <span class="flex items-center gap-2">
                                    <span class="size-2 rounded-full" style=format!("background-color: {}", account.color)></span>
                                    {account.name.clone()}
                                </span>
                            </button>
                        }
                    }).collect_view()}
                </div>
                <span class="text-xs text-muted">"La progression est la valeur de ces comptes : rien à saisir en double."</span>
            </div>
            <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
            <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit><Busy busy=busy />"Enregistrer l'objectif"</button>
            {editing.then(|| view! { <button class=BTN_DANGER disabled=move || busy.get() on:click=remove><Busy busy=busy />"Supprimer cet objectif"</button> })}
        </div>
    }
}
