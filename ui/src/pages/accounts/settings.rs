use lc_shared_dto::{AccountDto, AccountUpdateInput, IdInput};
use leptos::prelude::*;

use super::create::{ColorPicker, optional_amount};
use crate::api;
use crate::components::style::{BTN_DANGER, BTN_PRIMARY, BTN_SECONDARY};
use crate::components::{BottomSheet, Field};
use crate::format;
use crate::util::run_action;

/// Nom, couleur et versement mensuel programmé d'un compte.
#[component]
pub fn AccountSettingsSheet(
    open: RwSignal<bool>,
    account: AccountDto,
    on_saved: Callback<()>,
) -> impl IntoView {
    let name = RwSignal::new(account.name.clone());
    let color = RwSignal::new(account.color.clone());
    let contribution = RwSignal::new(format::quantity(account.monthly_contribution));
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let account_id = StoredValue::new(account.id);
    let submit = move |_| {
        let monthly_contribution = match optional_amount(&contribution.get_untracked()) {
            Ok(amount) => amount,
            Err(message) => return error.set(Some(message)),
        };
        let input = AccountUpdateInput {
            account_id: account_id.get_value(),
            name: name.get_untracked(),
            color: color.get_untracked(),
            monthly_contribution,
        };
        run_action(busy, error, api::update_account(input), move |()| {
            open.set(false);
            on_saved.run(());
        });
    };
    view! {
        <BottomSheet open=open title="Paramètres du compte">
            <div class="flex flex-col gap-4">
                <Field label="Nom" value=name />
                <ColorPicker selected=color />
                <Field label="Versement mensuel programmé" value=contribution input_mode="decimal" suffix="€" />
                <p class="text-xs text-muted">"Utilisé par la projection pour alimenter ce compte chaque mois."</p>
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit>"Enregistrer"</button>
            </div>
        </BottomSheet>
    }
}

/// Clôture (archivage) ou réouverture d'un compte, avec confirmation.
#[component]
pub fn ArchiveControl(account_id: String, archived: bool, on_saved: Callback<()>) -> impl IntoView {
    let confirm = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let account_id = StoredValue::new(account_id);
    let apply = move |_| {
        let input = IdInput {
            id: account_id.get_value(),
        };
        let on_done = move |()| {
            confirm.set(false);
            on_saved.run(());
        };
        if archived {
            run_action(busy, error, api::unarchive_account(input), on_done);
        } else {
            run_action(busy, error, api::archive_account(input), on_done);
        }
    };
    view! {
        {if archived {
            view! { <button class=BTN_SECONDARY on:click=apply>"Rouvrir le compte"</button> }.into_any()
        } else {
            view! { <button class=BTN_DANGER on:click=move |_| confirm.set(true)>"Clôturer le compte"</button> }.into_any()
        }}
        <BottomSheet open=confirm title="Clôturer ce compte ?">
            <div class="flex flex-col gap-4">
                <p class="text-sm text-muted">
                    "Il sort de votre patrimoine actuel. Son historique est conservé et vous pourrez le rouvrir."
                </p>
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_DANGER disabled=move || busy.get() on:click=apply>"Clôturer"</button>
            </div>
        </BottomSheet>
    }
}
