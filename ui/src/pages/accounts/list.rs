use lc_shared_dto::{
    AccountDetailsDto, AccountDto, AccountListDto, ErrorDto, IncludeArchivedInput,
};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_query_map;

use super::create::CreateAccountSheet;
use super::quick_balance::QuickBalanceSheet;
use crate::api;
use crate::components::style::BTN_PRIMARY;
use crate::components::{Amount, Card, EmptyState, ErrorBanner, ProvenanceBadge, Skeleton};
use crate::icons::{Icon, IconName};
use crate::util::{load, pricing_issue_message};

#[component]
pub fn AccountsPage() -> impl IntoView {
    let query = use_query_map();
    let create_open = RwSignal::new(false);
    let balance_open = RwSignal::new(false);
    Effect::new(move |_| {
        if query.with(|q| q.get("nouveau").is_some()) {
            create_open.set(true);
        }
        if query.with(|q| q.get("saisie").as_deref() == Some("solde")) {
            balance_open.set(true);
        }
    });
    let include_archived = RwSignal::new(false);
    let data = RwSignal::new(None::<Result<AccountListDto, ErrorDto>>);
    let reload = move || {
        let include_archived = include_archived.get_untracked();
        load(data, move || {
            api::list_accounts(IncludeArchivedInput { include_archived })
        });
    };
    Effect::new(move |_| {
        include_archived.track();
        reload();
    });

    view! {
        <div class="flex items-center justify-between">
            <h1 class="px-1 text-2xl font-medium">"Comptes"</h1>
            <button class=BTN_PRIMARY on:click=move |_| create_open.set(true)>
                <Icon name=IconName::Plus class="size-5" />
                "Nouveau"
            </button>
        </div>

        {move || match data.get() {
            None => view! { <Skeleton class="h-48" /> }.into_any(),
            Some(Err(error)) => view! { <ErrorBanner message=error.message /> }.into_any(),
            Some(Ok(list)) if list.accounts.is_empty() => view! {
                <Card>
                    <EmptyState title="Ajoutez votre premier compte" body="Livret A, LDDS, PEA, PEI ou portefeuille crypto.">
                        <button class=BTN_PRIMARY on:click=move |_| create_open.set(true)>"Ajouter un compte"</button>
                    </EmptyState>
                </Card>
            }
            .into_any(),
            Some(Ok(list)) => {
                let savings: Vec<AccountDto> = list
                    .accounts
                    .iter()
                    .filter(|a| !a.archived && matches!(a.details, AccountDetailsDto::Savings { .. }))
                    .cloned()
                    .collect();
                view! {
                    {list.pricing_issues.iter().map(|issue| view! { <ErrorBanner message=pricing_issue_message(issue) /> }).collect_view()}
                    <Card class="flex flex-col p-1">
                        {list.accounts.into_iter().map(|account| view! { <AccountRow account=account /> }).collect_view()}
                    </Card>
                    <QuickBalanceSheet open=balance_open accounts=savings on_saved=Callback::new(move |_| reload()) />
                }
                .into_any()
            }
        }}

        <label class="flex min-h-11 items-center gap-3 px-1 text-sm text-muted">
            <input
                type="checkbox"
                class="size-5 accent-primary"
                prop:checked=move || include_archived.get()
                on:change=move |event| include_archived.set(event_target_checked(&event))
            />
            "Afficher les comptes clôturés"
        </label>

        <CreateAccountSheet open=create_open />
    }
}

#[component]
fn AccountRow(account: AccountDto) -> impl IntoView {
    view! {
        <A href=format!("/comptes/{}", account.id) attr:class="flex min-h-16 items-center gap-3 rounded-xl px-3 py-2 transition active:bg-surface-raised">
            <span class="size-2.5 shrink-0 rounded-full" style=format!("background-color: {}", account.color)></span>
            <span class="flex min-w-0 flex-1 flex-col items-start gap-1">
                <span class="truncate font-medium">{account.name}</span>
                <span class="flex flex-wrap items-center gap-1.5 text-xs text-muted">
                    {account.account_type.label()}
                    {account.archived.then(|| view! { <span class="rounded-full bg-surface-raised px-2 py-0.5">"clôturé"</span> })}
                </span>
            </span>
            <span class="flex flex-col items-end gap-1">
                <Amount value=account.value class="font-medium" />
                <ProvenanceBadge provenance=account.provenance />
            </span>
        </A>
    }
}
