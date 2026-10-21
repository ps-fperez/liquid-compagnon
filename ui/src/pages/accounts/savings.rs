use lc_shared_dto::{BalanceInput, Decimal, NaiveDate, SavingsTermsInput};
use leptos::prelude::*;

use crate::api;
use crate::components::Busy;
use crate::components::amount_hint;
use crate::components::style::{BTN_PRIMARY, BTN_SECONDARY};
use crate::components::toast;
use crate::components::{Amount, BottomSheet, Card, Field};
use crate::format;
use crate::util::run_action;

/// Fiche d'un livret ou d'une assurance-vie : solde courant (dernier solde saisi et virements
/// comptés depuis), taux, plafond facultatif.
#[component]
pub fn SavingsDetails(
    account_id: String,
    balance: Decimal,
    reference_balance: Decimal,
    rate_percent: Decimal,
    deposit_ceiling: Option<Decimal>,
    balance_updated_on: NaiveDate,
    ceiling_reached: bool,
    on_saved: Callback<()>,
) -> impl IntoView {
    let balance_open = RwSignal::new(false);
    let terms_open = RwSignal::new(false);
    view! {
        <Card class="flex flex-col gap-3">
            <div class="flex items-center justify-between">
                <span class="text-sm text-muted">"Solde actuel"</span>
                <Amount value=balance class="text-lg font-semibold" />
            </div>
            <div class="flex items-center justify-between">
                <span class="text-sm text-muted">"Dernier solde saisi"</span>
                <span class="flex flex-col items-end">
                    <Amount value=reference_balance />
                    <span class="text-xs text-muted">"le " {format::date(balance_updated_on)}</span>
                </span>
            </div>
            <div class="flex items-center justify-between">
                <span class="text-sm text-muted">"Taux annuel"</span>
                <span class="tabular-nums">{format::percent(rate_percent, 2)}</span>
            </div>
            <div class="flex items-center justify-between">
                <span class="text-sm text-muted">"Plafond de versement"</span>
                {match deposit_ceiling {
                    Some(ceiling) => view! { <Amount value=ceiling /> }.into_any(),
                    None => view! { <span class="text-sm text-muted">"Aucun"</span> }.into_any(),
                }}
            </div>
            {ceiling_reached.then(|| view! {
                <p class="rounded-xl bg-secondary/10 px-3 py-2 text-sm text-secondary-light">
                    "Plafond atteint : plus aucun versement possible (les intérêts restent acquis)."
                </p>
            })}
        </Card>
        <div class="flex flex-col gap-2">
            <button class=BTN_PRIMARY on:click=move |_| balance_open.set(true)>"Mettre à jour le solde"</button>
            <button class=BTN_SECONDARY on:click=move |_| terms_open.set(true)>"Taux et plafond"</button>
        </div>
        <BalanceSheet open=balance_open account_id=account_id.clone() current=balance allow_negative=false on_saved=on_saved />
        <TermsSheet open=terms_open account_id=account_id rate_percent=rate_percent deposit_ceiling=deposit_ceiling on_saved=on_saved />
    }
}
/// Nouveau solde d'un livret ou, avec `allow_negative`, d'un compte courant (le clavier
/// numérique d'Android n'a pas toujours de signe moins : un interrupteur « Découvert » le remplace).
#[component]
pub fn BalanceSheet(
    open: RwSignal<bool>,
    account_id: String,
    current: Decimal,
    allow_negative: bool,
    on_saved: Callback<()>,
) -> impl IntoView {
    let amount = RwSignal::new(format::quantity(current.abs()));
    let overdrawn = RwSignal::new(current.is_sign_negative() && !current.is_zero());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let account_id = StoredValue::new(account_id);
    let submit = move |_| {
        let entered = match format::parse_amount(&amount.get_untracked()) {
            Ok(balance) => balance,
            Err(message) => return error.set(Some(message)),
        };
        let balance = if allow_negative && overdrawn.get_untracked() {
            -entered.abs()
        } else {
            entered
        };
        let input = BalanceInput {
            account_id: account_id.get_value(),
            balance,
        };
        run_action(busy, error, api::update_balance(input), move |()| {
            toast::success("Solde mis à jour");
            open.set(false);
            on_saved.run(());
        });
    };
    view! {
        <BottomSheet open=open title="Mettre à jour le solde">
            <div class="flex flex-col gap-4">
                <Field label="Nouveau solde" value=amount input_mode="decimal" suffix="€" hint=Signal::derive(move || Some(amount_hint(&amount.get()))) />
                {allow_negative.then(|| view! {
                    <label class="flex min-h-11 items-center justify-between gap-3">
                        <span class="text-sm">"Solde négatif (découvert)"</span>
                        <input
                            type="checkbox"
                            class="size-5 accent-primary"
                            prop:checked=move || overdrawn.get()
                            on:change=move |event| overdrawn.set(event_target_checked(&event))
                        />
                    </label>
                })}
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit><Busy busy=busy />"Enregistrer"</button>
            </div>
        </BottomSheet>
    }
}

#[component]
fn TermsSheet(
    open: RwSignal<bool>,
    account_id: String,
    rate_percent: Decimal,
    deposit_ceiling: Option<Decimal>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let rate = RwSignal::new(format::quantity(rate_percent));
    let ceiling = RwSignal::new(deposit_ceiling.map(format::quantity).unwrap_or_default());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let account_id = StoredValue::new(account_id);
    let submit = move |_| {
        let rate_percent = match format::parse_decimal(&rate.get_untracked()) {
            Ok(rate) => rate,
            Err(message) => return error.set(Some(message)),
        };
        let raw_ceiling = ceiling.get_untracked();
        let deposit_ceiling = match format::parse_amount(&raw_ceiling) {
            _ if raw_ceiling.trim().is_empty() => None,
            Ok(ceiling) => Some(ceiling),
            Err(message) => return error.set(Some(message)),
        };
        let input = SavingsTermsInput {
            account_id: account_id.get_value(),
            rate_percent,
            deposit_ceiling,
        };
        run_action(busy, error, api::set_savings_terms(input), move |()| {
            toast::success("Taux et plafond enregistrés");
            open.set(false);
            on_saved.run(());
        });
    };
    view! {
        <BottomSheet open=open title="Taux et plafond">
            <div class="flex flex-col gap-4">
                <Field label="Taux annuel" value=rate input_mode="decimal" suffix="%" />
                <Field label="Plafond de versement (facultatif)" value=ceiling input_mode="decimal" suffix="€" hint="Laissez vide s'il n'y en a pas" />
                <p class="text-xs text-muted">
                    "Si vos rendements de projection suivent le taux du livret, ils sont mis à jour aussi."
                </p>
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit><Busy busy=busy />"Enregistrer"</button>
            </div>
        </BottomSheet>
    }
}
