//! Bilan du mois (spec monthly-review) : cartes plein écran qui défilent seules comme des
//! stories, ouverture au premier déverrouillage du mois, recalage du compte courant, historique.

use std::time::Duration;

use lc_shared_dto::{Decimal, MonthInput, ReviewDto, StartMonthInput};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::Busy;
use crate::components::amount_hint;
use crate::components::style::{BTN_ICON, BTN_PRIMARY};
use crate::components::toast;
use crate::components::{Amount, Field};
use crate::format;
use crate::icons::{Icon, IconName};
use crate::util::run_action;

/// Durée d'affichage d'une carte avant de passer à la suivante.
const CARD_MS: u64 = 5_000;

/// Bilan affiché (contexte global : ouverture automatique ou depuis l'historique) et compteur
/// de recalages, pour que l'accueil se recharge après « C'est parti ».
#[derive(Clone, Copy)]
pub struct Review(RwSignal<Option<ReviewDto>>, RwSignal<u32>);

impl Review {
    pub fn provide() {
        provide_context(Self(RwSignal::new(None), RwSignal::new(0)));
    }

    pub fn show(self, review: ReviewDto) {
        self.0.set(Some(review));
    }

    /// Nombre de recalages effectués (à suivre pour recharger un écran).
    pub fn restarts(self) -> u32 {
        self.1.get()
    }

    /// Ouvre le bilan d'un mois passé (sans recalage).
    pub fn open_month(self, month: String) {
        spawn_local(async move {
            match api::get_review(MonthInput { month: Some(month) }).await {
                Ok(review) => self.show(review),
                Err(error) => toast::error(error.message),
            }
        });
    }
}

pub fn use_review() -> Review {
    expect_context::<Review>()
}

/// Au déverrouillage : ouvre le bilan du mois précédent s'il n'a jamais été présenté.
#[component]
pub fn ReviewWatcher() -> impl IntoView {
    let review = use_review();
    spawn_local(async move {
        if let Ok(Some(pending)) = api::pending_review().await {
            review.show(pending);
        }
    });
}

/// Couche plein écran du bilan ouvert.
#[component]
pub fn ReviewOverlay() -> impl IntoView {
    let review = use_review();
    let close = Callback::new(move |_: ()| review.0.set(None));
    move || {
        review
            .0
            .get()
            .map(|r| view! { <ReviewStory review=r on_close=close /> })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CardKind {
    Saved,
    Flows,
    Categories,
    Wealth,
    Goals,
    Start,
}

fn cards_of(review: &ReviewDto) -> Vec<CardKind> {
    let report = &review.report;
    let mut cards = vec![CardKind::Saved, CardKind::Flows];
    if !report.top_categories.is_empty() {
        cards.push(CardKind::Categories);
    }
    if report.wealth_end.is_some() {
        cards.push(CardKind::Wealth);
    }
    if !report.goals.is_empty() {
        cards.push(CardKind::Goals);
    }
    if review.start.is_some() {
        cards.push(CardKind::Start);
    }
    cards
}

#[component]
fn ReviewStory(review: ReviewDto, on_close: Callback<()>) -> impl IntoView {
    let cards = cards_of(&review);
    let count = cards.len();
    let index = RwSignal::new(0_usize);
    let review = StoredValue::new(review);
    let cards = StoredValue::new(cards);
    // Défilement automatique : chaque changement de carte relance le minuteur, l'ancien est ignoré.
    let generation = StoredValue::new(0_u32);
    Effect::new(move |_| {
        let current = index.get();
        let ticket = generation.get_value().wrapping_add(1);
        generation.set_value(ticket);
        if current + 1 < count {
            set_timeout(
                move || {
                    if generation.try_get_value() == Some(ticket) {
                        let _ = index.try_set(current + 1);
                    }
                },
                Duration::from_millis(CARD_MS),
            );
        }
    });
    let step = move |forward: bool| {
        let current = index.get_untracked();
        if forward && current + 1 < count {
            index.set(current + 1);
        } else if !forward && current > 0 {
            index.set(current - 1);
        }
    };
    // Toucher la gauche revient, la droite avance ; les champs et boutons gardent leur rôle.
    let on_tap = move |event: leptos::ev::MouseEvent| {
        let target: web_sys::Element = event_target(&event);
        if target
            .closest("input, button, label")
            .ok()
            .flatten()
            .is_some()
        {
            return;
        }
        let width = window()
            .inner_width()
            .ok()
            .and_then(|w| w.as_f64())
            .unwrap_or(400.0);
        step(f64::from(event.client_x()) > width / 3.0);
    };
    let month_label = review.with_value(|r| format::month_label(&r.report.month));

    view! {
        <div
            class="fixed inset-0 z-50 flex flex-col bg-bg bg-[radial-gradient(120%_80%_at_50%_0%,rgb(124_58_237/0.35),var(--color-bg)_70%)] px-5 pt-[calc(0.75rem+env(safe-area-inset-top))] pb-[calc(1.5rem+env(safe-area-inset-bottom))] animate-fade-in"
            role="dialog"
            aria-modal="true"
            aria-label=format!("Bilan de {month_label}")
            on:click=on_tap
        >
            <div class="mx-auto flex w-full max-w-md gap-1.5" aria-hidden="true">
                {(0..count).map(|i| view! {
                    <span class="h-1 flex-1 overflow-hidden rounded-full bg-ink/20">
                        {move || {
                            let current = index.get();
                            if i < current {
                                Some(view! { <span class="block h-full w-full rounded-full bg-ink"></span> }.into_any())
                            } else if i == current {
                                // Recréée à chaque carte : l'animation repart de zéro.
                                let last = i + 1 == count;
                                Some(view! { <span class=format!("block h-full rounded-full bg-ink {}", if last { "w-full" } else { "w-full animate-story" })></span> }.into_any())
                            } else {
                                None
                            }
                        }}
                    </span>
                }).collect_view()}
            </div>
            <div class="mx-auto mt-3 flex w-full max-w-md items-center justify-between">
                <span class="text-sm font-medium">{format!("Bilan de {month_label}")}</span>
                <button class=BTN_ICON aria-label="Fermer le bilan" on:click=move |_| on_close.run(())>
                    <Icon name=IconName::Close />
                </button>
            </div>
            <div class="mx-auto flex w-full max-w-md flex-1 flex-col justify-center">
                {move || {
                    let kind = cards.with_value(|c| c[index.get()]);
                    review.with_value(|r| view! {
                        <div class="flex animate-rise flex-col gap-4">
                            {card(kind, r, on_close)}
                        </div>
                    })
                }}
            </div>
        </div>
    }
}

fn comparison(current: Decimal, previous: Option<Decimal>, previous_month: &str) -> Option<String> {
    let previous = previous?;
    let delta = current - previous;
    Some(if delta.is_zero() {
        format!("Autant qu'en {previous_month}.")
    } else {
        format!(
            "{} par rapport à {previous_month}.",
            format::amount_signed(delta)
        )
    })
}

fn card(kind: CardKind, review: &ReviewDto, on_close: Callback<()>) -> AnyView {
    let report = &review.report;
    let previous_month = format::shift_month(&report.month, -1)
        .map(|m| format::month_name(&m))
        .unwrap_or_default();
    let kicker = |text: &'static str| view! { <p class="text-sm font-medium uppercase tracking-wider text-primary-soft">{text}</p> };
    match kind {
        CardKind::Saved => {
            let negative = report.saved.is_sign_negative() && !report.saved.is_zero();
            view! {
                {kicker("Votre mois en un chiffre")}
                <p class="text-xl">{if negative { "Vous avez puisé dans votre épargne" } else { "Vous avez mis de côté" }}</p>
                <Amount value=report.saved.abs() animate=true class=format!("text-5xl font-semibold tracking-tight {}", if negative { "text-loss" } else { "text-gain" }) />
                {comparison(report.saved, report.previous_saved, &previous_month).map(|text| view! { <p class="text-muted">{text}</p> })}
            }
            .into_any()
        }
        CardKind::Flows => {
            let spent = report.fixed_charges + report.expenses + report.external_out;
            view! {
                {kicker("Entrées et sorties")}
                <p class="text-2xl font-medium leading-snug">
                    <Amount value=report.received /> " reçus, " <Amount value=spent /> " dépensés"
                </p>
                {(!report.one_off_incomes.is_zero()).then(|| view! {
                    <p class="text-muted">"Dont " <Amount value=report.one_off_incomes /> " de revenus ponctuels en plus du salaire."</p>
                })}
                {weights(report).map(|text| view! { <p class="text-muted">{text}</p> })}            }
            .into_any()
        }
        CardKind::Categories => {
            let largest = report
                .top_categories
                .first()
                .map(|c| c.amount)
                .unwrap_or_default();
            view! {
                {kicker("Là où l'argent est parti")}
                <div class="flex flex-col gap-3">
                    {report.top_categories.iter().map(|category| {
                        let width = if largest > Decimal::ZERO { crate::util::to_percent(category.amount / largest) } else { 0.0 };
                        view! {
                            <div class="flex flex-col gap-1">
                                <span class="flex justify-between text-lg"><span>{category.name.clone()}</span><Amount value=category.amount /></span>
                                <span class="h-2 overflow-hidden rounded-full bg-ink/10">
                                    <span class="block h-full rounded-full bg-primary transition-[width] duration-700" style=format!("width: {width:.0}%")></span>
                                </span>
                            </div>
                        }
                    }).collect_view()}
                </div>
                {report.biggest_increase.clone().map(|(name, percent)| view! {
                    <p class="text-muted">{format!("{name} en hausse de {} % sur un mois.", percent.round())}</p>
                })}
            }
            .into_any()
        }
        CardKind::Wealth => view! {
            {kicker("Patrimoine")}
            <Amount value=report.wealth_end.unwrap_or_default() animate=true class="text-5xl font-semibold tracking-tight" />
            {report.wealth_change.map(|change| view! {
                <p class="flex flex-wrap items-center gap-2 text-lg">
                    {report.wealth_change_percent.map(|p| view! {
                        <span class=if p.is_sign_negative() { "text-loss" } else { "text-gain" }>{format::percent_signed(p, 1)}</span>
                    })}
                    <Amount value=change signed=true colored=true />
                    <span class="text-muted">"sur le mois"</span>
                </p>
            })}
        }
        .into_any(),
        CardKind::Goals => view! {
            {kicker("Objectifs")}
            <div class="flex flex-col gap-4">
                {report.goals.iter().map(|goal| {
                    let points = goal.previous_percent.map(|before| (goal.percent - before).round());
                    let reached = goal.percent >= Decimal::ONE_HUNDRED;
                    view! {
                        <div class="flex items-center gap-4">
                            <crate::pages::GoalRing percent=goal.percent size=72 reached=reached />
                            <span class="flex flex-col">
                                <span class="text-lg font-medium">{goal.name.clone()}</span>
                                {points.filter(|p| !p.is_zero()).map(|p| view! {
                                    <span class="text-muted">{format!("{}{} point{} ce mois-ci", if p.is_sign_negative() { "" } else { "+" }, p, if p.abs() > Decimal::ONE { "s" } else { "" })}</span>
                                })}
                            </span>
                        </div>
                    }
                }).collect_view()}
            </div>
        }
        .into_any(),
        CardKind::Start => match review.start.clone() {
            Some(start) => view! { <StartCard start=start month=report.month.clone() on_close=on_close /> }.into_any(),
            None => ().into_any(),
        },
    }
}

/// Dernière carte : « Votre compte courant repart à combien ? ».
#[component]
fn StartCard(
    start: lc_shared_dto::StartPromptDto,
    month: String,
    on_close: Callback<()>,
) -> impl IntoView {
    let balance = RwSignal::new(format::quantity(start.suggested));
    let restarts = use_review().1;
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let next = format::shift_month(&month, 1)
        .map(|m| format::month_name(&m))
        .unwrap_or_default();
    let submit = move |_| {
        let balance = match format::parse_decimal(&balance.get_untracked()) {
            Ok(value) => value,
            Err(message) => return error.set(Some(message)),
        };
        run_action(
            busy,
            error,
            api::start_month(StartMonthInput { balance }),
            move |()| {
                toast::success("C'est parti pour ce mois-ci");
                restarts.update(|n| *n += 1);
                on_close.run(());
            },
        );
    };
    view! {
        <p class="text-sm font-medium uppercase tracking-wider text-primary-soft">{format!("{} commence", capitalize(&next))}</p>
        <p class="text-2xl font-medium">{format!("{} repart à combien ?", start.account_name)}</p>
        <Field label="Solde de départ" value=balance input_mode="decimal" suffix="€" hint=Signal::derive(move || Some(amount_hint(&balance.get()))) />
        <p class="text-sm text-muted">"Après les virements de début de mois."</p>
        {(!start.included.is_empty()).then(|| view! {
            <p class="text-sm text-muted">
                "Déjà pris en compte : "
                {start.included.iter().map(|t| format!("{} {}", t.counterpart, format::amount(-t.amount))).collect::<Vec<_>>().join(", ")}
                "."
            </p>
        })}
        <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
        <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit><Busy busy=busy />"C'est parti"</button>
    }
}

/// « Les charges fixes pèsent 1 300 €, les dépenses du quotidien 1 540 €. » (parts non nulles).
fn weights(report: &lc_shared_dto::MonthReportDto) -> Option<String> {
    let parts: Vec<(&str, Decimal)> = [
        ("charges fixes", report.fixed_charges),
        ("dépenses du quotidien", report.expenses),
        ("virements hors patrimoine", report.external_out),
    ]
    .into_iter()
    .filter(|(_, amount)| !amount.is_zero())
    .collect();
    let ((first, amount), rest) = parts.split_first()?;
    let mut sentence = format!("Les {first} pèsent {}", format::amount(*amount));
    for (label, amount) in rest {
        sentence.push_str(&format!(", les {label} {}", format::amount(*amount)));
    }
    sentence.push('.');
    Some(sentence)
}
fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}
