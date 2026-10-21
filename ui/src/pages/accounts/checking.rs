use lc_shared_dto::{Decimal, MovementDto, NaiveDate};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use super::savings::BalanceSheet;
use super::transfers::MovementFeed;
use crate::components::style::BTN_PRIMARY;
use crate::components::{Amount, Card};
use crate::format;

/// Fiche d'un compte courant : solde courant, dernier solde saisi et mouvements comptés depuis.
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
        <Card class="flex flex-col gap-3">
            <div class="flex items-center justify-between">
                <span class="text-sm text-muted">"Solde actuel"</span>
                <Amount
                    value=balance
                    class=format!(
                        "text-lg font-semibold {}",
                        if balance.is_sign_negative() && !balance.is_zero() { "text-loss" } else { "" },
                    )
                />
            </div>
            <div class="flex items-center justify-between">
                <span class="text-sm text-muted">"Dernier solde saisi"</span>
                <span class="flex flex-col items-end">
                    <Amount value=reference_balance />
                    <span class="text-xs text-muted">"le " {format::date(reference_on)}</span>
                </span>
            </div>
            <p class="text-xs text-muted">
                "Les dépenses, revenus ponctuels, charges prélevées et virements comptés depuis sont déduits ou ajoutés. Recalez le solde quand vous consultez votre banque."
            </p>
        </Card>

        <MovementFeed
            movements=movements
            empty_body="Dépenses, charges prélevées et virements rattachés à ce compte apparaîtront ici et feront évoluer le solde."
            empty_action="Ajouter une dépense"
            on_empty_action=add_expense
        />

        <button class=BTN_PRIMARY on:click=move |_| balance_open.set(true)>"Mettre à jour le solde"</button>
        <BalanceSheet open=balance_open account_id=account_id current=balance allow_negative=true on_saved=on_saved />
    }
}
