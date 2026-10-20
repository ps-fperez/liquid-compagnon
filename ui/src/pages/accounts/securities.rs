use lc_shared_dto::{
    AccountTypeDto, AssetDto, CashInput, Decimal, PositionDto, PositionInput, QuoteSourceDto,
    RemovePositionInput,
};
use leptos::prelude::*;

use super::asset_search::AssetSearch;
use crate::api;
use crate::components::style::{BTN_DANGER, BTN_PRIMARY, BTN_SECONDARY, SECTION_TITLE};
use crate::components::{Amount, BottomSheet, Card, EmptyState, Field, ProvenanceBadge};
use crate::format;
use crate::util::run_action;

/// Fiche d'un PEA ou d'un portefeuille crypto : espèces et positions.
#[component]
pub fn SecuritiesDetails(
    account_id: String,
    account_type: AccountTypeDto,
    cash: Decimal,
    positions: Vec<PositionDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let source = if account_type == AccountTypeDto::Crypto {
        QuoteSourceDto::CoinGecko
    } else {
        QuoteSourceDto::TwelveData
    };
    let account_id = StoredValue::new(account_id);
    let cash_open = RwSignal::new(false);
    let new_open = RwSignal::new(false);
    let editing = RwSignal::new(None::<PositionDto>);
    let has_positions = !positions.is_empty();

    view! {
        <section class="flex flex-col gap-2">
            <h2 class=SECTION_TITLE>"Positions"</h2>
            <Card class="flex flex-col p-1">
                {if has_positions {
                    positions.into_iter().map(|position| {
                        let edit = position.clone();
                        view! {
                            <button
                                class="flex min-h-16 items-center gap-3 rounded-xl px-3 py-2 text-left active:bg-surface-raised"
                                on:click=move |_| editing.set(Some(edit.clone()))
                            >
                                <span class="flex min-w-0 flex-1 flex-col gap-1">
                                    <span class="truncate font-medium">{position.asset.display_name.clone()}</span>
                                    <span class="text-xs text-muted">{format::quantity(position.quantity)} " · " {position.asset.symbol.clone()}</span>
                                </span>
                                <span class="flex flex-col items-end gap-1">
                                    <Amount value=position.value class="font-medium" />
                                    {match (position.unrealized_gain, position.unrealized_gain_percent) {
                                        (Some(gain), percent) => view! {
                                            <span class="flex gap-1 text-xs">
                                                <Amount value=gain signed=true colored=true />
                                                {percent.map(|p| format!("({})", format::percent_signed(p, 1)))}
                                            </span>
                                        }.into_any(),
                                        (None, _) => view! { <ProvenanceBadge provenance=position.provenance /> }.into_any(),
                                    }}
                                </span>
                            </button>
                        }
                    }).collect_view().into_any()
                } else {
                    view! { <EmptyState title="Aucune position" body="Ajoutez les titres ou crypto-actifs que vous détenez." /> }.into_any()
                }}
                <button
                    class="flex min-h-14 items-center justify-between rounded-xl border-t border-line px-3 text-left active:bg-surface-raised"
                    aria-label="Modifier les espèces" on:click=move |_| cash_open.set(true)
                >
                    <span class="text-sm text-muted">"Espèces"</span>
                    <Amount value=cash />
                </button>
            </Card>
            <button class=BTN_PRIMARY on:click=move |_| new_open.set(true)>"Ajouter une position"</button>
        </section>

        <CashSheet open=cash_open account_id=account_id.get_value() cash=cash on_saved=on_saved />
        <BottomSheet open=new_open title="Nouvelle position">
            <PositionForm account_id=account_id.get_value() source=source existing=None open=new_open on_saved=on_saved />
        </BottomSheet>
        {move || editing.get().map(|position| {
            let open = RwSignal::new(true);
            Effect::new(move |_| if !open.get() { editing.set(None) });
            view! {
                <BottomSheet open=open title="Position">
                    <PositionForm account_id=account_id.get_value() source=source existing=Some(position.clone()) open=open on_saved=on_saved />
                </BottomSheet>
            }
        })}
    }
}

/// Ajout (`existing` absent) ou modification d'une position.
#[component]
fn PositionForm(
    account_id: String,
    source: QuoteSourceDto,
    existing: Option<PositionDto>,
    open: RwSignal<bool>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let editing = existing.is_some();
    let asset = RwSignal::new(existing.as_ref().map(|p| p.asset.clone()));
    let quantity = RwSignal::new(
        existing
            .as_ref()
            .map(|p| format::quantity(p.quantity))
            .unwrap_or_default(),
    );
    let unit_cost = RwSignal::new(
        existing
            .as_ref()
            .and_then(|p| p.cost_basis.zip(Some(p.quantity)))
            .filter(|(_, q)| !q.is_zero())
            .map(|(cost, q)| format::quantity((cost / q).round_dp(6)))
            .unwrap_or_default(),
    );
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let account_id = StoredValue::new(account_id);

    let submit = move |_| {
        let Some(chosen) = asset.get_untracked() else {
            return error.set(Some("Choisissez un actif".into()));
        };
        let quantity = match format::parse_decimal(&quantity.get_untracked()) {
            Ok(value) => value,
            Err(message) => return error.set(Some(message)),
        };
        let raw_cost = unit_cost.get_untracked();
        let unit_cost = if raw_cost.trim().is_empty() {
            None
        } else {
            match format::parse_decimal(&raw_cost) {
                Ok(cost) => Some(cost),
                Err(message) => return error.set(Some(message)),
            }
        };
        let input = PositionInput {
            account_id: account_id.get_value(),
            asset: chosen,
            quantity,
            unit_cost,
        };
        run_action(busy, error, api::upsert_position(input), move |()| {
            open.set(false);
            on_saved.run(());
        });
    };

    let remove = move |_| {
        let Some(chosen): Option<AssetDto> = asset.get_untracked() else {
            return;
        };
        let input = RemovePositionInput {
            account_id: account_id.get_value(),
            asset: chosen,
        };
        run_action(busy, error, api::remove_position(input), move |()| {
            open.set(false);
            on_saved.run(());
        });
    };

    let placeholder = if source == QuoteSourceDto::CoinGecko {
        "bitcoin, ethereum…"
    } else {
        "CW8, IWDA, FR0010315770…"
    };
    view! {
        <div class="flex flex-col gap-4">
            {if editing {
                view! {
                    {move || asset.get().map(|a| view! {
                        <p class="text-sm"><span class="font-medium">{a.display_name}</span> <span class="text-muted">" · " {a.symbol}</span></p>
                    })}
                }.into_any()
            } else {
                view! { <AssetSearch source=source selected=asset label="Actif" placeholder=placeholder /> }.into_any()
            }}
            <Field label="Quantité détenue" value=quantity input_mode="decimal" placeholder="0,15" />
            <Field label="Prix de revient unitaire (facultatif)" value=unit_cost input_mode="decimal" suffix="€" />
            <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
            <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit>"Enregistrer"</button>
            {editing.then(|| view! { <button class=BTN_DANGER disabled=move || busy.get() on:click=remove>"Retirer cette position"</button> })}
        </div>
    }
}

#[component]
fn CashSheet(
    open: RwSignal<bool>,
    account_id: String,
    cash: Decimal,
    on_saved: Callback<()>,
) -> impl IntoView {
    let amount = RwSignal::new(format::quantity(cash));
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let account_id = StoredValue::new(account_id);
    let submit = move |_| {
        let cash = match format::parse_amount(&amount.get_untracked()) {
            Ok(cash) => cash,
            Err(message) => return error.set(Some(message)),
        };
        run_action(
            busy,
            error,
            api::set_cash(CashInput {
                account_id: account_id.get_value(),
                cash,
            }),
            move |()| {
                open.set(false);
                on_saved.run(());
            },
        );
    };
    view! {
        <BottomSheet open=open title="Espèces">
            <div class="flex flex-col gap-4">
                <Field label="Solde espèces" value=amount input_mode="decimal" suffix="€" />
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_SECONDARY disabled=move || busy.get() on:click=submit>"Enregistrer"</button>
            </div>
        </BottomSheet>
    }
}
