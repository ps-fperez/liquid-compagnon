use lc_shared_dto::{AccountDetailsDto, AccountDto, ErrorDto, IdInput, NavKindDto, ProvenanceDto};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use super::savings::SavingsDetails;
use super::settings::{AccountSettingsSheet, ArchiveControl};
use crate::api;
use crate::components::style::{BTN_SECONDARY, SECTION_TITLE};
use crate::components::{Amount, Card, ErrorBanner, ProvenanceBadge, Skeleton};
use crate::format;
use crate::icons::{Icon, IconName};
use crate::util::load;

#[component]
pub fn AccountDetailPage() -> impl IntoView {
    let params = use_params_map();
    let account_id = move || params.with(|p| p.get("id").unwrap_or_default());
    let data = RwSignal::new(None::<Result<AccountDto, ErrorDto>>);
    let reload = Callback::new(move |_: ()| {
        let id = account_id();
        load(data, move || api::get_account(IdInput { id }));
    });
    Effect::new(move |_| {
        account_id();
        reload.run(());
    });

    view! {
        <A href="/comptes" attr:class="-ml-1 flex min-h-11 items-center gap-1 self-start text-sm text-muted">
            <Icon name=IconName::ChevronLeft class="size-5" />
            "Comptes"
        </A>
        {move || match data.get() {
            None => view! { <Skeleton class="h-40" /> }.into_any(),
            Some(Err(error)) => view! { <ErrorBanner message=error.message /> }.into_any(),
            Some(Ok(account)) => view! { <AccountView account=account on_saved=reload /> }.into_any(),
        }}
    }
}

#[component]
fn AccountView(account: AccountDto, on_saved: Callback<()>) -> impl IntoView {
    let settings_open = RwSignal::new(false);
    let details = match account.details.clone() {
        AccountDetailsDto::Savings { balance, rate_percent, deposit_ceiling, balance_updated_on, ceiling_reached } => view! {
            <SavingsDetails
                account_id=account.id.clone()
                balance=balance
                rate_percent=rate_percent
                deposit_ceiling=deposit_ceiling
                balance_updated_on=balance_updated_on
                ceiling_reached=ceiling_reached
                on_saved=on_saved
            />
        }
        .into_any(),
        AccountDetailsDto::Securities { cash, positions } => view! {
            <Card class="flex flex-col gap-2">
                <div class="flex items-center justify-between">
                    <span class="text-sm text-muted">"Espèces"</span>
                    <Amount value=cash />
                </div>
                {positions.into_iter().map(|position| view! {
                    <div class="flex items-center justify-between gap-3 border-t border-line pt-2">
                        <span class="flex min-w-0 flex-col">
                            <span class="truncate">{position.asset.display_name}</span>
                            <span class="text-xs text-muted">{format::quantity(position.quantity)} " · " {position.asset.symbol}</span>
                        </span>
                        <span class="flex flex-col items-end gap-1">
                            <Amount value=position.value />
                            <ProvenanceBadge provenance=position.provenance />
                        </span>
                    </div>
                }).collect_view()}
            </Card>
        }
        .into_any(),
        AccountDetailsDto::EmployeeSavings { funds } => view! {
            <Card class="flex flex-col gap-2">
                {funds.into_iter().map(|fund| {
                    let provenance = match fund.nav_kind {
                        NavKindDto::Real => ProvenanceDto::Manual,
                        NavKindDto::Estimated => ProvenanceDto::Estimated,
                    };
                    view! {
                        <div class="flex items-center justify-between gap-3">
                            <span class="flex min-w-0 flex-col">
                                <span class="truncate">{fund.name}</span>
                                <span class="text-xs text-muted">{format::quantity(fund.units)} " parts"</span>
                            </span>
                            <span class="flex flex-col items-end gap-1">
                                <Amount value=fund.value />
                                <ProvenanceBadge provenance=provenance />
                            </span>
                        </div>
                    }
                }).collect_view()}
            </Card>
        }
        .into_any(),
    };

    view! {
        <header class="flex flex-col gap-1 px-1">
            <span class="flex items-center gap-2 text-sm text-muted">
                <span class="size-2.5 rounded-full" style=format!("background-color: {}", account.color)></span>
                {account.account_type.label()}
                {account.archived.then(|| view! { <span class="rounded-full bg-surface-raised px-2 py-0.5 text-xs">"clôturé"</span> })}
            </span>
            <h1 class="text-2xl font-medium">{account.name.clone()}</h1>
            <span class="flex items-center gap-2">
                <Amount value=account.value class="text-3xl font-semibold tracking-tight" />
                <ProvenanceBadge provenance=account.provenance />
            </span>
        </header>

        {details}

        <section class="flex flex-col gap-2">
            <h2 class=SECTION_TITLE>"Projection"</h2>
            <Card class="flex items-center justify-between">
                <span class="text-sm text-muted">"Versement mensuel programmé"</span>
                <Amount value=account.monthly_contribution />
            </Card>
        </section>

        <div class="flex flex-col gap-2">
            <button class=BTN_SECONDARY on:click=move |_| settings_open.set(true)>"Paramètres du compte"</button>
            <ArchiveControl account_id=account.id.clone() archived=account.archived on_saved=on_saved />
        </div>
        <AccountSettingsSheet open=settings_open account=account on_saved=on_saved />
    }
}
