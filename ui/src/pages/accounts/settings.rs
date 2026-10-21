use lc_shared_dto::{AccountDto, AccountUpdateInput, IdInput};
use leptos::prelude::*;

use super::create::ColorPicker;
use crate::api;
use crate::components::Busy;
use crate::components::style::{BTN_DANGER, BTN_PRIMARY, BTN_SECONDARY};
use crate::components::toast;
use crate::components::{BottomSheet, Field};
use crate::util::run_action;

/// Nom et couleur d'un compte.
#[component]
pub fn AccountSettingsSheet(
    open: RwSignal<bool>,
    account: AccountDto,
    on_saved: Callback<()>,
) -> impl IntoView {
    let name = RwSignal::new(account.name.clone());
    let color = RwSignal::new(account.color.clone());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let account_id = StoredValue::new(account.id);
    let submit = move |_| {
        let input = AccountUpdateInput {
            account_id: account_id.get_value(),
            name: name.get_untracked(),
            color: color.get_untracked(),
        };
        run_action(busy, error, api::update_account(input), move |()| {
            toast::success("Compte modifié");
            open.set(false);
            on_saved.run(());
        });
    };
    view! {
        <BottomSheet open=open title="Paramètres du compte">
            <div class="flex flex-col gap-4">
                <Field label="Nom" value=name />
                <ColorPicker selected=color />
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit><Busy busy=busy />"Enregistrer"</button>
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
            toast::success(if archived {
                "Compte rouvert"
            } else {
                "Compte clôturé"
            });
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
                <button class=BTN_DANGER disabled=move || busy.get() on:click=apply><Busy busy=busy />"Clôturer"</button>
            </div>
        </BottomSheet>
    }
}
