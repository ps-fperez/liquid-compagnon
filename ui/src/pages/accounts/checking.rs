use lc_shared_dto::{Decimal, MovementDto, NaiveDate};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use super::savings::BalanceSheet;
use super::transfers::MovementFeed;
use crate::components::style::BTN_PRIMARY;
use crate::components::{Amount, Card, Gauge};
use crate::format;
use crate::util::{to_percent, today};

/// Jauge du compte courant (maquette v0.3) : ce qui est parti depuis le dernier solde saisi,
/// rapporté à ce solde plus ce qui est arrivé depuis, avec le rythme sur le reste du mois.
pub fn checking_gauge(
    balance: Decimal,
    reference: Decimal,
    reference_on: NaiveDate,
    movements: &[MovementDto],
) -> impl IntoView + use<> {
    let received: Decimal = movements
        .iter()
        .map(|m| m.amount)
        .filter(|a| a.is_sign_positive())
        .sum();
    let consumed: Decimal = movements
        .iter()
        .map(|m| m.amount)
        .filter(|a| a.is_sign_negative())
        .map(|a| -a)
        .sum();
    let available = reference + received;
    let percent = if available > Decimal::ZERO {
        to_percent(consumed / available)
    } else if consumed.is_zero() {
        0.0
    } else {
        100.0
    };
    // Rythme : part du mois écoulée depuis le dernier solde saisi (mois en cours seulement).
    let now = today();
    let same_month = format::iso_month(now) == format::iso_month(reference_on);
    let days_in_month = format::days_in_month(now);
    let (marker, pace) = if same_month {
        let span = f64::from(days_in_month - chrono::Datelike::day(&reference_on) + 1);
        let elapsed =
            f64::from(chrono::Datelike::day(&now) - chrono::Datelike::day(&reference_on) + 1);
        let fraction = (elapsed / span).clamp(0.0, 1.0);
        let pace = if balance.is_sign_negative() && !balance.is_zero() {
            ("budget dépassé", "text-loss")
        } else if percent / 100.0 - fraction > 0.02 {
            ("rythme élevé", "text-warn")
        } else {
            ("dans le rythme", "text-gain")
        };
        (Some(fraction * 100.0), Some(pace))
    } else {
        (None, None)
    };
    view! {
        <Gauge percent=percent marker=marker.unwrap_or(100.0) alert=balance.is_sign_negative() && !balance.is_zero() />
        <span class="flex items-start justify-between gap-3 text-[11px] text-muted">
            <span>
                <Amount value=consumed /> " consommés sur " <Amount value=reference />
                {(!received.is_zero()).then(|| view! { " + " <Amount value=received /> " reçus" })}
            </span>
            {pace.map(|(label, tone)| view! { <span class=format!("shrink-0 {tone}")>{label}</span> })}
        </span>
    }
}

/// Fiche d'un compte courant : dernier solde saisi et mouvements comptés depuis (le solde et la
/// jauge sont dans la carte principale).
#[component]
pub fn CheckingDetails(
    account_id: String,
    balance: Decimal,
    reference_balance: Decimal,
    reference_on: NaiveDate,
    movements: Vec<MovementDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let balance_open = RwSignal::new(false);
    let navigate = StoredValue::new_local(use_navigate());
    let add_expense = Callback::new(move |_: ()| {
        // Même parcours que la saisie rapide : le compte courant est proposé par défaut.
        let target = format!("/budget?saisie=depense&n={}", js_sys::Date::now() as u64);
        navigate.with_value(|navigate| navigate(&target, Default::default()));
    });

    view! {
        <MovementFeed
            movements=movements
            since=Some(reference_on)
            empty_body="Dépenses, charges prélevées et virements rattachés à ce compte apparaîtront ici et feront évoluer le solde."
            empty_action="Ajouter une dépense"
            on_empty_action=add_expense
        />

        <Card class="flex flex-col gap-2">
            <div class="flex items-center justify-between text-sm">
                <span class="text-muted">"Dernier solde saisi"</span>
                <span class="flex items-baseline gap-2">
                    <Amount value=reference_balance class="font-medium" />
                    <span class="text-xs text-muted">{format!("le {}", format::day_month(reference_on))}</span>
                </span>
            </div>
            <p class="text-xs text-muted">
                "Les dépenses, revenus ponctuels, charges prélevées et virements comptés depuis sont déduits ou ajoutés. Recalez le solde quand vous consultez votre banque."
            </p>
        </Card>

        <button class=BTN_PRIMARY on:click=move |_| balance_open.set(true)>"Mettre à jour le solde"</button>
        <BalanceSheet open=balance_open account_id=account_id current=balance allow_negative=true on_saved=on_saved />
    }
}
