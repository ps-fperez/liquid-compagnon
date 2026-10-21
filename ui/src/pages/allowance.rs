//! Reste à dépenser (spec spending-allowance) : carte de l'accueil, ligne de la synthèse du
//! budget et feuille de détail qui déroule le calcul.

use lc_shared_dto::{AllowanceDto, BudgetSummaryDto, Decimal, PaceDto};
use leptos::prelude::*;

use crate::components::style::{BTN_GHOST, SECTION_TITLE};
use crate::components::{Amount, BottomSheet, Card, EmptyState};
use crate::format;
use crate::util::to_percent;

fn pace_look(pace: PaceDto) -> (&'static str, &'static str, &'static str) {
    match pace {
        PaceDto::OnTrack => ("Dans le rythme", "bg-gain/15 text-gain", "bg-gain"),
        PaceDto::Fast => (
            "Rythme élevé",
            "bg-amber-400/15 text-amber-300",
            "bg-amber-400",
        ),
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
    let (pace_label, pace_tone, bar_tone) = pace_look(allowance.pace);
    let percent = spent_percent(&allowance);
    let today_marker = allowance
        .today
        .map(|day| f64::from(day) / f64::from(allowance.days_in_month) * 100.0);
    // La jauge se remplit depuis zéro à l'affichage.
    let filled = RwSignal::new(false);
    request_animation_frame(move || {
        let _ = filled.try_set(true);
    });
    let month = summary.month.clone();
    let per_day = allowance.per_day;
    let days_left = allowance.days_left;
    view! {
        <button
            class="group flex flex-col gap-3 rounded-2xl border border-line bg-surface p-4 text-left transition active:scale-[0.99] active:bg-surface-raised"
            aria-label="Détail du reste à dépenser"
            on:click=move |_| open.set(true)
        >
            <span class="flex items-center justify-between gap-2">
                <span class="text-sm text-muted">{format!("Reste à dépenser · {}", format::month_name(&month))}</span>
                <span class=format!("rounded-full px-2.5 py-0.5 text-xs font-medium {pace_tone}")>{pace_label}</span>
            </span>
            <span class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
                <Amount
                    value=allowance.remaining
                    animate=true
                    class=format!(
                        "text-3xl font-semibold tracking-tight {}",
                        if allowance.remaining.is_sign_negative() && !allowance.remaining.is_zero() { "text-loss" } else { "" },
                    )
                />
                {per_day.map(|value| view! {
                    <span class="text-sm text-muted">"soit " <Amount value=value /> "/jour"</span>
                })}
            </span>
            <span class="relative h-2.5 overflow-hidden rounded-full bg-surface-raised">
                <span
                    class=format!("absolute inset-y-0 left-0 rounded-full transition-[width] duration-700 ease-out {bar_tone}")
                    style=move || format!("width: {}%", if filled.get() { percent } else { 0.0 })
                ></span>
                {today_marker.map(|left| view! {
                    <span class="absolute inset-y-0 w-0.5 bg-ink/80" style=format!("left: {left:.1}%") aria-hidden="true"></span>
                })}
            </span>
            <span class="flex items-center justify-between gap-2 text-xs text-muted">
                <span><Amount value=allowance.spent /> " dépensés sur " <Amount value=allowance.envelope /></span>
                {(days_left > 0).then(|| view! {
                    <span>{format!("{days_left} jour{} restant{}", if days_left > 1 { "s" } else { "" }, if days_left > 1 { "s" } else { "" })}</span>
                })}
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
    let line = |label: String, value: Decimal, sign: i8| {
        let shown = match sign {
            -1 => -value,
            _ => value,
        };
        view! {
            <div class="flex items-center justify-between gap-3 text-sm">
                <span class="text-muted">{label}</span>
                <Amount value=shown signed=true />
            </div>
        }
    };
    let optional = move |label: &'static str, value: Decimal, sign: i8| {
        (!value.is_zero()).then(|| line(label.to_string(), value, sign))
    };
    let rhythm = match (a.daily_spending, a.projected_end) {
        _ if a.envelope.is_sign_negative() && !a.envelope.is_zero() => Some(format!(
            "Vos charges, votre épargne programmée et vos virements hors patrimoine dépassent vos revenus du mois de {}.",
            format::amount(-a.envelope)
        )),
        (Some(daily), Some(end)) if end.is_sign_negative() && !end.is_zero() => Some(format!(
            "Au rythme actuel ({}/jour), vous finiriez le mois à {}.{}",
            format::amount(daily),
            format::amount(end),
            a.per_day
                .map(|per_day| format!(
                    " Pour finir à l'équilibre : {}/jour jusqu'au {}.",
                    format::amount(per_day),
                    a.days_in_month
                ))
                .unwrap_or_default(),
        )),
        (Some(daily), Some(end)) => Some(format!(
            "Au rythme actuel ({}/jour), il vous resterait {} en fin de mois.",
            format::amount(daily),
            format::amount(end),
        )),
        _ => a.per_day.map(|per_day| {
            format!(
                "Vous pouvez dépenser {}/jour jusqu'au {}.",
                format::amount(per_day),
                a.days_in_month
            )
        }),
    };
    let largest = summary
        .top_categories
        .first()
        .map(|c| c.amount)
        .unwrap_or_default();
    view! {
        <div class="flex flex-col gap-4">
            {a.today.is_some().then(|| view! {
                <span class=format!("self-start rounded-full px-2.5 py-0.5 text-xs font-medium {pace_tone}")>{pace_label}</span>
            })}
            <Card class="flex flex-col gap-2">
                {line(format!("Revenus de {}", format::month_name(&summary.month)), a.income, 1)}
                {optional("Reçus de l'extérieur", a.external_in, 1)}
                {optional("Rapatriements d'épargne", a.repatriated, 1)}
                {optional("Charges fixes (par mois)", a.fixed_charges, -1)}
                {optional("Épargne programmée", a.scheduled_savings, -1)}
                {optional("Virements hors patrimoine", a.external_out, -1)}
                <div class="flex items-center justify-between gap-3 border-t border-line pt-2 text-sm">
                    <span>"Enveloppe du mois"</span>
                    <Amount value=a.envelope class="font-medium" />
                </div>
                {line("Dépenses du mois".to_string(), a.spent, -1)}
                <div class="mt-1 flex items-center justify-between gap-3 border-t border-line pt-3">
                    <span class="font-medium">"Reste"</span>
                    <Amount value=a.remaining class=format!("text-lg font-semibold {}", if a.remaining.is_sign_negative() && !a.remaining.is_zero() { "text-loss" } else { "" }) />
                </div>
            </Card>
            {rhythm.map(|text| view! { <p class="text-sm">{text}</p> })}
            {(!summary.top_categories.is_empty()).then(|| view! {
                <section class="flex flex-col gap-2">
                    <h3 class=SECTION_TITLE>"Où part l'argent"</h3>
                    <Card class="flex flex-col gap-3">
                        {summary.top_categories.iter().enumerate().map(|(index, category)| {
                            let width = if largest > Decimal::ZERO { to_percent(category.amount / largest) } else { 0.0 };
                            view! {
                                <div class="flex animate-rise flex-col gap-1" style=crate::components::cascade(index)>
                                    <span class="flex items-center justify-between text-sm">
                                        <span>{category.name.clone()}</span>
                                        <Amount value=category.amount />
                                    </span>
                                    <span class="h-1.5 overflow-hidden rounded-full bg-surface-raised">
                                        <span class="block h-full rounded-full bg-primary" style=format!("width: {width:.0}%")></span>
                                    </span>
                                </div>
                            }
                        }).collect_view()}
                    </Card>
                </section>
            })}
            <p class="text-xs text-muted">
                "L'enveloppe compte toutes les échéances du mois, même à venir : salaire, charges fixes, virements vers l'épargne et vers les comptes hors patrimoine partis d'un compte courant."
            </p>
        </div>
    }
}
