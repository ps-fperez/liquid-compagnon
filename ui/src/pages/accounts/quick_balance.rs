use lc_shared_dto::{AccountDetailsDto, AccountDto, BalanceInput};
use leptos::prelude::*;

use crate::api;
use crate::components::Busy;
use crate::components::amount_hint;
use crate::components::style::{BTN_PRIMARY, LABEL};
use crate::components::toast;
use crate::components::{BottomSheet, Field};
use crate::format;
use crate::util::run_action;

/// Saisie rapide « Mise à jour d'un solde » : choisir un livret ou un compte courant, saisir son
/// nouveau solde (négatif possible pour un compte courant).
#[component]
pub fn QuickBalanceSheet(
    open: RwSignal<bool>,
    accounts: Vec<AccountDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let selected = RwSignal::new(accounts.first().map(|a| a.id.clone()).unwrap_or_default());
    let amount = RwSignal::new(String::new());
    let overdrawn = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let accounts = StoredValue::new(accounts);

    let current_balance = move || {
        accounts.with_value(|list| {
            list.iter()
                .find(|a| a.id == selected.get())
                .and_then(|a| match a.details {
                    AccountDetailsDto::Savings { balance, .. }
                    | AccountDetailsDto::Checking { balance, .. } => Some(balance),
                    _ => None,
                })
        })
    };
    let is_checking = move || {
        accounts.with_value(|list| {
            list.iter().any(|a| {
                a.id == selected.get() && matches!(a.details, AccountDetailsDto::Checking { .. })
            })
        })
    };

    let is_checking_untracked = move || {
        let id = selected.get_untracked();
        accounts.with_value(|list| {
            list.iter()
                .any(|a| a.id == id && matches!(a.details, AccountDetailsDto::Checking { .. }))
        })
    };
    let submit = move |_| {
        let entered = match format::parse_amount(&amount.get_untracked()) {
            Ok(balance) => balance,
            Err(message) => return error.set(Some(message)),
        };
        let balance = if overdrawn.get_untracked() && is_checking_untracked() {
            -entered.abs()
        } else {
            entered
        };
        let input = BalanceInput {
            account_id: selected.get_untracked(),
            balance,
        };
        run_action(busy, error, api::update_balance(input), move |()| {
            toast::success("Solde mis à jour");
            amount.set(String::new());
            overdrawn.set(false);
            open.set(false);
            on_saved.run(());
        });
    };

    view! {
        <BottomSheet open=open title="Mise à jour d'un solde">
            {move || if accounts.with_value(Vec::is_empty) {
                view! { <p class="text-sm text-muted">"Créez d'abord un livret ou un compte courant."</p> }.into_any()
            } else {
                view! {
                    <div class="flex flex-col gap-4">
                        <div class="flex flex-col gap-1.5">
                            <span class=LABEL>"Compte"</span>
                            <div class="flex flex-wrap gap-2">
                                {accounts.get_value().into_iter().map(|account| {
                                    let id = account.id.clone();
                                    let check = id.clone();
                                    view! {
                                        <button
                                            class=move || format!(
                                                "min-h-11 rounded-xl border px-4 text-sm {}",
                                                if selected.get() == check { "border-primary bg-primary/15 text-ink" } else { "border-line text-muted" },
                                            )
                                            on:click=move |_| selected.set(id.clone())
                                        >
                                            {account.name}
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                            {move || current_balance().map(|b| view! {
                                <span class="text-xs text-muted">"Solde actuel : " {format::amount(b)}</span>
                            })}
                        </div>
                        <Field label="Nouveau solde" value=amount input_mode="decimal" suffix="€" placeholder="0,00" hint=Signal::derive(move || Some(amount_hint(&amount.get()))) />
                        <Show when=is_checking>
                            <label class="flex min-h-11 items-center justify-between gap-3">
                                <span class="text-sm">"Solde négatif (découvert)"</span>
                                <input
                                    type="checkbox"
                                    class="size-5 accent-primary"
                                    prop:checked=move || overdrawn.get()
                                    on:change=move |event| overdrawn.set(event_target_checked(&event))
                                />
                            </label>
                        </Show>
                        <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                        <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit><Busy busy=busy />"Enregistrer"</button>
                    </div>
                }
                .into_any()
            }}
        </BottomSheet>
    }
}
