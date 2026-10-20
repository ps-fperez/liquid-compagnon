use lc_shared_dto::{AccountDetailsDto, AccountDto, SavingsBalanceInput};
use leptos::prelude::*;

use crate::api;
use crate::components::style::{BTN_PRIMARY, LABEL};
use crate::components::{BottomSheet, Field};
use crate::format;
use crate::util::run_action;

/// Saisie rapide « Mise à jour d'un solde » : choisir un livret, saisir son nouveau solde.
#[component]
pub fn QuickBalanceSheet(
    open: RwSignal<bool>,
    accounts: Vec<AccountDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let selected = RwSignal::new(accounts.first().map(|a| a.id.clone()).unwrap_or_default());
    let amount = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let accounts = StoredValue::new(accounts);

    let current_balance = move || {
        accounts.with_value(|list| {
            list.iter()
                .find(|a| a.id == selected.get())
                .and_then(|a| match a.details {
                    AccountDetailsDto::Savings { balance, .. } => Some(balance),
                    _ => None,
                })
        })
    };

    let submit = move |_| {
        let balance = match format::parse_amount(&amount.get_untracked()) {
            Ok(balance) => balance,
            Err(message) => return error.set(Some(message)),
        };
        let input = SavingsBalanceInput {
            account_id: selected.get_untracked(),
            balance,
        };
        run_action(busy, error, api::update_savings_balance(input), move |()| {
            amount.set(String::new());
            open.set(false);
            on_saved.run(());
        });
    };

    view! {
        <BottomSheet open=open title="Mise à jour d'un solde">
            {move || if accounts.with_value(Vec::is_empty) {
                view! { <p class="text-sm text-muted">"Créez d'abord un Livret A ou un LDDS."</p> }.into_any()
            } else {
                view! {
                    <div class="flex flex-col gap-4">
                        <div class="flex flex-col gap-1.5">
                            <span class=LABEL>"Livret"</span>
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
                        <Field label="Nouveau solde" value=amount input_mode="decimal" suffix="€" placeholder="0,00" />
                        <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                        <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit>"Enregistrer"</button>
                    </div>
                }
                .into_any()
            }}
        </BottomSheet>
    }
}
