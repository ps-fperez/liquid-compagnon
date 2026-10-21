use lc_shared_dto::{
    AccountDetailsDto, AccountDto, AccountTypeDto, ErrorDto, IdInput, MovementDto, NaiveDate,
};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use super::checking::{CheckingDetails, checking_gauge};
use super::pei::PeiDetails;
use super::savings::SavingsDetails;
use super::securities::SecuritiesDetails;
use super::settings::{AccountSettingsSheet, ArchiveControl};
use super::transfers::{MovementFeed, ScheduledTransfers};
use crate::api;
use crate::components::style::BTN_SECONDARY;
use crate::components::{Amount, ErrorBanner, Gauge, HeroCard, ProvenanceBadge, Skeleton};
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
    let new_transfer = RwSignal::new(false);
    let open_transfer = Callback::new(move |_: ()| new_transfer.set(true));
    let transfer_feed = move |movements: Vec<MovementDto>, since: Option<NaiveDate>| {
        view! {
            <MovementFeed
                movements=movements
                since=since
                empty_body="Les virements reçus ou émis depuis le dernier solde saisi apparaîtront ici et feront évoluer le solde."
                empty_action="Nouveau virement"
                on_empty_action=open_transfer
            />
        }
    };
    let (details, feed) = match account.details.clone() {
        AccountDetailsDto::Savings {
            balance,
            reference_balance,
            rate_percent,
            deposit_ceiling,
            balance_updated_on,
            ceiling_reached,
            movements,
        } => (
            view! {
                <SavingsDetails
                    account_id=account.id.clone()
                    balance=balance
                    reference_balance=reference_balance
                    rate_percent=rate_percent
                    deposit_ceiling=deposit_ceiling
                    balance_updated_on=balance_updated_on
                    ceiling_reached=ceiling_reached
                    on_saved=on_saved
                />
            }
            .into_any(),
            Some(transfer_feed(movements, Some(balance_updated_on)).into_any()),
        ),
        AccountDetailsDto::Securities {
            cash,
            positions,
            movements,
        } => (
            view! {
                <SecuritiesDetails
                    account_id=account.id.clone()
                    account_type=account.account_type
                    cash=cash
                    positions=positions
                    on_saved=on_saved
                />
            }
            .into_any(),
            (!movements.is_empty()).then(|| transfer_feed(movements, None).into_any()),
        ),
        AccountDetailsDto::Checking {
            balance,
            reference_balance,
            reference_on,
            movements,
        } => (
            view! {
                <CheckingDetails
                    account_id=account.id.clone()
                    balance=balance
                    reference_balance=reference_balance
                    reference_on=reference_on
                    movements=movements
                    on_saved=on_saved
                />
            }
            .into_any(),
            None,
        ),
        AccountDetailsDto::EmployeeSavings { funds } => (
            view! {
                <PeiDetails account_id=account.id.clone() funds=funds on_saved=on_saved />
            }
            .into_any(),
            None,
        ),
    };

    let hero_extra = match &account.details {
        AccountDetailsDto::Checking {
            reference_balance,
            reference_on,
            movements,
            balance,
        } => Some(checking_gauge(*balance, *reference_balance, *reference_on, movements).into_any()),
        AccountDetailsDto::Savings {
            balance,
            deposit_ceiling: Some(ceiling),
            ..
        } if ceiling.is_sign_positive() && !ceiling.is_zero() => Some(view! {
            <Gauge percent=crate::util::to_percent(*balance / *ceiling) />
            <span class="text-[11px] text-muted">
                <Amount value=*balance whole=true /> " sur un plafond de " <Amount value=*ceiling whole=true />
            </span>
        }.into_any()),
        _ => None,
    };
    let negative = account.value.is_sign_negative() && !account.value.is_zero();
    // « Fortuneo · compte courant » ; le type seul s'il sert déjà de nom.
    let type_label = account.account_type.label();
    let hero_title = if account.name.eq_ignore_ascii_case(type_label) {
        account.name.clone()
    } else {
        format!("{} · {}", account.name, type_label.to_lowercase())
    };
    let dot = format!("background-color: {}", account.color);
    let (value, provenance, archived) = (account.value, account.provenance, account.archived);

    view! {
        <HeroCard class="gap-2">
            <span class="flex items-center gap-2 text-sm text-muted">
                <span class="size-2.5 rounded-full" style=dot></span>
                <span class="truncate">{hero_title}</span>
                {archived.then(|| view! { <span class="rounded-full bg-surface-raised px-2 py-0.5 text-xs">"clôturé"</span> })}
            </span>
            <span class="flex flex-wrap items-center gap-2">
                <Amount value=value animate=true class=format!("text-[32px] leading-tight font-bold tracking-tight {}", if negative { "text-loss" } else { "" }) />
                <ProvenanceBadge provenance=provenance />
            </span>
            {hero_extra}
        </HeroCard>
        {details}
        {feed}
        <ScheduledTransfers
            account_id=account.id.clone()
            incoming=account.account_type != AccountTypeDto::Checking
            new_open=new_transfer
            on_saved=on_saved
        />

        <div class="flex flex-col gap-2">
            <button class=BTN_SECONDARY on:click=move |_| settings_open.set(true)>"Paramètres du compte"</button>
            <ArchiveControl account_id=account.id.clone() archived=account.archived on_saved=on_saved />
        </div>
        <AccountSettingsSheet open=settings_open account=account on_saved=on_saved />
    }
}
