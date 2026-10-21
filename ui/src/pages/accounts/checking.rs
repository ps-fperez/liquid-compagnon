use lc_shared_dto::{CheckingMovementDto, Decimal, MovementKindDto, NaiveDate};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use super::savings::BalanceSheet;
use crate::components::style::{BTN_PRIMARY, SECTION_TITLE};
use crate::components::{Amount, Card, EmptyState};
use crate::format;

/// Fiche d'un compte courant : solde courant, dernier solde saisi et mouvements comptés depuis.
#[component]
pub fn CheckingDetails(
    account_id: String,
    balance: Decimal,
    reference_balance: Decimal,
    reference_on: NaiveDate,
    movements: Vec<CheckingMovementDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let balance_open = RwSignal::new(false);
    let navigate = StoredValue::new_local(use_navigate());
    let add_expense = Callback::new(move |_: ()| {
        // Même parcours que la saisie rapide : le compte courant est proposé par défaut.
        let target = format!("/budget?saisie=depense&n={}", js_sys::Date::now() as u64);
        navigate.with_value(|navigate| navigate(&target, Default::default()));
    });
    let has_movements = !movements.is_empty();

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
                "Les dépenses et revenus ponctuels rattachés à ce compte, saisis depuis, sont déduits ou ajoutés. Recalez le solde quand vous consultez votre banque."
            </p>
        </Card>

        <section class="flex flex-col gap-2">
            <h2 class=SECTION_TITLE>"Depuis le dernier solde saisi"</h2>
            {if has_movements {
                view! {
                    <Card class="flex flex-col p-2">
                        {movements.into_iter().map(|movement| {
                            let fallback = match movement.kind {
                                MovementKindDto::Expense => "Dépense",
                                MovementKindDto::Income => "Revenu ponctuel",
                            };
                            view! {
                                <div class="flex min-h-12 items-center justify-between gap-3 px-2">
                                    <span class="flex min-w-0 flex-col">
                                        <span class="truncate text-sm">{movement.label.unwrap_or_else(|| fallback.to_string())}</span>
                                        <span class="text-xs text-muted">{format::date(movement.date)}</span>
                                    </span>
                                    <Amount value=movement.amount signed=true colored=true />
                                </div>
                            }
                        }).collect_view()}
                    </Card>
                }
                .into_any()
            } else {
                view! {
                    <EmptyState
                        title="Aucun mouvement depuis"
                        body="Vos dépenses rattachées à ce compte apparaîtront ici et feront évoluer le solde."
                        action="Ajouter une dépense"
                        on_action=add_expense
                    />
                }
                .into_any()
            }}
        </section>

        <button class=BTN_PRIMARY on:click=move |_| balance_open.set(true)>"Mettre à jour le solde"</button>
        <BalanceSheet open=balance_open account_id=account_id current=balance allow_negative=true on_saved=on_saved />
    }
}
