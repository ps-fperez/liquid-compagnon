//! Reste à dépenser (spec spending-allowance) : carte de l'accueil, ligne de la synthèse du
//! budget et feuille de détail qui déroule le calcul.

use lc_shared_dto::{AllowanceDto, BudgetSummaryDto, Decimal, PaceDto};
use leptos::prelude::*;

use crate::components::style::{BTN_GHOST, KICKER, SECTION_TITLE};
use crate::components::{Amount, BottomSheet, EmptyState, Gauge};
use crate::format;
use crate::util::to_percent;

fn pace_look(pace: PaceDto) -> (&'static str, &'static str, &'static str) {
    match pace {
        PaceDto::OnTrack => ("Dans le rythme", "bg-gain/15 text-gain", "bg-gain"),
        PaceDto::Fast => ("Rythme élevé", "bg-warn/14 text-warn", "bg-warn"),
        PaceDto::Exceeded => ("Budget dépassé", "bg-loss/15 text-loss", "bg-loss"),
    }
}

/// Part de l'enveloppe déjà dépensée, de 0 à 100 (100 si l'enveloppe est nulle ou négative).
fn spent_percent(allowance: &AllowanceDto) -> f64 {
    if allowance.envelope <= Decimal::ZERO {
        return if allowance.spent.is_zero() {
            0.0
        } else {
            100.0
        };
    }
    to_percent(allowance.spent / allowance.envelope).clamp(0.0, 100.0)
}

/// Aucun revenu ni entrée : l'enveloppe n'aurait pas de sens.
fn has_income(allowance: &AllowanceDto) -> bool {
    !(allowance.income.is_zero()
        && allowance.external_in.is_zero()
        && allowance.repatriated.is_zero())
}

/// Carte « Reste à dépenser » de l'accueil (mois en cours) ; la toucher ouvre le détail.
#[component]
pub fn AllowanceCard(summary: BudgetSummaryDto) -> impl IntoView {
    let allowance = summary.allowance.clone();
    if !has_income(&allowance) {
        return view! {
            <section class="flex flex-col gap-2">
                <h2 class=SECTION_TITLE>"Reste à dépenser"</h2>
                <EmptyState
                    title="Combien pouvez-vous encore dépenser ?"
                    body="Renseignez votre salaire : l'application en déduit vos charges, votre épargne programmée et vos dépenses du mois."
                    action="Renseigner mon salaire"
                    href="/budget"
                />
            </section>
        }
        .into_any();
    }
    let open = RwSignal::new(false);
    let (pace_label, pace_tone, _) = pace_look(allowance.pace);
    let percent = spent_percent(&allowance);
    let today_marker = allowance
        .today
        .map(|day| f64::from(day) / f64::from(allowance.days_in_month) * 100.0);
    let month = summary.month.clone();
    let per_day = allowance.per_day;
    let days_left = allowance.days_left;
    let negative = allowance.remaining.is_sign_negative() && !allowance.remaining.is_zero();
    view! {
        <button
            class="flex flex-col gap-2.5 rounded-[18px] border border-line bg-surface p-4 text-left transition active:scale-[0.99] active:bg-surface-raised"
            aria-label="Détail du reste à dépenser"
            on:click=move |_| open.set(true)
        >
            <span class="flex items-start justify-between gap-2">
                <span class=KICKER>{format!("Reste à dépenser · {}", format::month_name(&month))}</span>
                <span class=format!("shrink-0 rounded-full px-2.5 py-0.5 text-[11px] {pace_tone}")>{pace_label}</span>
            </span>
            <span class="flex items-end justify-between gap-3">
                <Amount
                    value=allowance.remaining
                    animate=true
                    whole=true
                    class=format!("text-[32px] leading-none font-bold tracking-tight {}", if negative { "text-loss" } else { "" })
                />
                <span class="flex flex-col items-end text-xs text-muted">
                    {per_day.map(|value| view! {
                        <span>"soit " <Amount value=value whole=true class="font-semibold text-ink" /> "/jour"</span>
                    })}
                    {(days_left > 0).then(|| view! {
                        <span>{format!("{days_left} jour{s} restant{s}", s = if days_left > 1 { "s" } else { "" })}</span>
                    })}
                </span>
            </span>
            <Gauge percent=percent marker=today_marker.unwrap_or(100.0) alert=allowance.pace == PaceDto::Exceeded />
            <span class="flex items-center justify-between gap-2 text-[11px] text-muted">
                <span><Amount value=allowance.spent whole=true /> " dépensés sur " <Amount value=allowance.envelope whole=true /></span>
                <span>{format::day_month(crate::util::today())}</span>
            </span>
        </button>
        <AllowanceSheet open=open summary=summary />
    }
    .into_any()
}
/// Ligne « Reste à dépenser » de la synthèse du budget, avec accès au détail.
#[component]
pub fn AllowanceRow(summary: BudgetSummaryDto) -> impl IntoView {
    let open = RwSignal::new(false);
    let allowance = &summary.allowance;
    let (pace_label, pace_tone, _) = pace_look(allowance.pace);
    let remaining = allowance.remaining;
    let show_pace = allowance.today.is_some() && has_income(allowance);
    view! {
        <div class="flex items-center justify-between gap-3">
            <span class="flex min-w-0 flex-col items-start gap-1">
                <span class="whitespace-nowrap font-medium">"Reste à dépenser"</span>
                {show_pace.then(|| view! {
                    <span class=format!("whitespace-nowrap rounded-full px-2 py-0.5 text-xs {pace_tone}")>{pace_label}</span>
                })}
            </span>
            <span class="flex shrink-0 items-center gap-1">
                <Amount value=remaining class=format!("text-lg font-semibold {}", if remaining.is_sign_negative() && !remaining.is_zero() { "text-loss" } else { "" }) />
                <button class=format!("{BTN_GHOST} -mr-3 px-3") on:click=move |_| open.set(true)>"Détail"</button>
            </span>
        </div>
        <AllowanceSheet open=open summary=summary />
    }
}

/// Feuille de détail : calcul ligne à ligne, rythme et fin de mois projetée, catégories.
#[component]
fn AllowanceSheet(open: RwSignal<bool>, summary: BudgetSummaryDto) -> impl IntoView {
    let summary = StoredValue::new(summary);
    view! {
        <BottomSheet open=open title="Reste à dépenser">
            {move || {
                let summary = summary.get_value();
                view! { <AllowanceDetail summary=summary /> }
            }}
        </BottomSheet>
    }
}

#[component]
fn AllowanceDetail(summary: BudgetSummaryDto) -> impl IntoView {
    let a = summary.allowance.clone();
    let (pace_label, pace_tone, _) = pace_look(a.pace);
    let line = |label: String, value: Decimal, income: bool| {
        view! {
            <div class="flex items-center justify-between gap-3">
                <span>{label}</span>
                <Amount value=if income { value } else { -value } signed=true class=if income { "text-gain" } else { "" } />
            </div>
        }
    };
    let optional = move |label: &'static str, value: Decimal, income: bool| {
        (!value.is_zero()).then(|| line(label.to_string(), value, income))
    };
    let negative_envelope = a.envelope.is_sign_negative() && !a.envelope.is_zero();
    let message = if negative_envelope {
        Some(view! {
            <p>
                "Vos charges, votre épargne programmée et vos virements hors patrimoine dépassent vos revenus du mois de "
                <Amount value=-a.envelope whole=true class="font-semibold text-loss" /> "."
            </p>
        }.into_any())
    } else {
        match (a.daily_spending, a.projected_end) {
            (Some(daily), Some(end)) => {
                let short = end.is_sign_negative() && !end.is_zero();
                Some(view! {
                    <p>
                        "Au rythme actuel (" <Amount value=daily whole=true /> "/jour), "
                        {if short { "vous finiriez le mois à " } else { "il vous resterait " }}
                        <Amount value=end whole=true class=if short { "font-semibold text-loss" } else { "font-semibold text-gain" } />
                        {if short { "." } else { " en fin de mois." }}
                    </p>
                    {(short && a.per_day.is_some()).then(|| view! {
                        <p class="text-xs text-muted">
                            "Pour finir à l'équilibre : " <Amount value=a.per_day.unwrap_or_default() whole=true /> {format!("/jour jusqu'au {}.", a.days_in_month)}
                        </p>
                    })}
                }.into_any())
            }
            _ => a.per_day.map(|per_day| view! {
                <p>"Vous pouvez dépenser " <Amount value=per_day whole=true class="font-semibold" /> {format!("/jour jusqu'au {}.", a.days_in_month)}</p>
            }.into_any()),
        }
    };
    let largest = summary
        .top_categories
        .first()
        .map(|c| c.amount)
        .unwrap_or_default();
    const BAR_COLORS: [&str; 3] = ["bg-primary", "bg-secondary", "bg-secondary-light"];
    view! {
        <div class="flex flex-col gap-4">
            {a.today.is_some().then(|| view! {
                <span class=format!("-mt-2 self-start rounded-full px-2.5 py-0.5 text-[11px] {pace_tone}")>{pace_label}</span>
            })}
            <div class="flex flex-col gap-1.5 text-sm">
                {line(format!("Revenus {}", format::of_month_name(&summary.month)), a.income, true)}
                {optional("Reçus de l'extérieur", a.external_in, true)}
                {optional("Rapatriements d'épargne", a.repatriated, true)}
                {optional("Charges fixes", a.fixed_charges, false)}
                {optional("Épargne programmée", a.scheduled_savings, false)}
                {optional("Virements hors patrimoine", a.external_out, false)}
                {line("Dépenses du mois".to_string(), a.spent, false)}
                <div class="mt-1 flex items-center justify-between gap-3 border-t border-line pt-2 text-base font-bold">
                    <span>"Reste"</span>
                    <Amount value=a.remaining class=if a.remaining.is_sign_negative() && !a.remaining.is_zero() { "text-loss" } else { "" } />
                </div>
            </div>
            {message.map(|message| view! {
                <div class="flex flex-col gap-1 rounded-xl bg-surface-raised p-3 text-sm">{message}</div>
            })}
            {(!summary.top_categories.is_empty()).then(|| view! {
                <section class="flex flex-col gap-2.5">
                    <h3 class=KICKER>"Où part l'argent"</h3>
                    {summary.top_categories.iter().enumerate().map(|(index, category)| {
                        let width = if largest > Decimal::ZERO { to_percent(category.amount / largest) } else { 0.0 };
                        let color = BAR_COLORS[index % BAR_COLORS.len()];
                        view! {
                            <div class="flex animate-rise flex-col gap-1" style=crate::components::cascade(index)>
                                <span class="flex items-center justify-between text-sm">
                                    <span>{category.name.clone()}</span>
                                    <Amount value=category.amount whole=true />
                                </span>
                                <span class="h-1.5 overflow-hidden rounded-full bg-surface-raised">
                                    <span class=format!("block h-full rounded-full {color}") style=format!("width: {width:.0}%")></span>
                                </span>
                            </div>
                        }
                    }).collect_view()}
                </section>
            })}
            <p class="text-xs text-muted">
                "L'enveloppe compte toutes les échéances du mois, même à venir : salaire, charges fixes, virements vers l'épargne et vers les comptes hors patrimoine partis d'un compte courant."
            </p>
        </div>
    }
}
