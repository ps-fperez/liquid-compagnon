use lc_shared_dto::{
    AssetDto, FundDto, FundNavInput, FundRefInput, FundUpdateInput, NavKindDto, NewFundInput,
    ProvenanceDto, QuoteSourceDto,
};
use leptos::prelude::*;

use super::asset_search::AssetSearch;
use crate::api;
use crate::components::style::{BTN_DANGER, BTN_PRIMARY, BTN_SECONDARY, SECTION_TITLE};
use crate::components::{Amount, BottomSheet, Card, EmptyState, Field, ProvenanceBadge};
use crate::format;
use crate::util::run_action;

const REFERENCE_HELP: &str = "Un ETF qui suit le même marché que le fonds (ex. MSCI World) : sa variation sert à estimer la valeur de part entre deux relevés.";

/// Fiche d'un PEI : fonds valorisés par leur dernière valeur de part, estimée via un indice.
#[component]
pub fn PeiDetails(
    account_id: String,
    funds: Vec<FundDto>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let account_id = StoredValue::new(account_id);
    let new_open = RwSignal::new(false);
    let selected = RwSignal::new(None::<FundDto>);
    let has_funds = !funds.is_empty();

    view! {
        <section class="flex flex-col gap-2">
            <h2 class=SECTION_TITLE>"Fonds"</h2>
            <Card class="flex flex-col p-1">
                {if has_funds {
                    funds.into_iter().map(|fund| {
                        let open_fund = fund.clone();
                        let provenance = match fund.nav_kind {
                            NavKindDto::Real => ProvenanceDto::Manual,
                            NavKindDto::Estimated => ProvenanceDto::Estimated,
                        };
                        view! {
                            <button
                                class="flex min-h-16 items-center gap-3 rounded-xl px-3 py-2 text-left active:bg-surface-raised"
                                on:click=move |_| selected.set(Some(open_fund.clone()))
                            >
                                <span class="flex min-w-0 flex-1 flex-col gap-1">
                                    <span class="truncate font-medium">{fund.name.clone()}</span>
                                    <span class="text-xs text-muted">
                                        {format::quantity(fund.units)} " parts × " {format::amount(fund.nav)}
                                    </span>
                                </span>
                                <span class="flex flex-col items-end gap-1">
                                    <Amount value=fund.value class="font-medium" />
                                    <ProvenanceBadge provenance=provenance />
                                </span>
                            </button>
                        }
                    }).collect_view().into_any()
                } else {
                    view! { <EmptyState title="Aucun fonds" body="Ajoutez les fonds de votre plan d'épargne salariale." /> }.into_any()
                }}
            </Card>
            <button class=BTN_PRIMARY on:click=move |_| new_open.set(true)>"Ajouter un fonds"</button>
        </section>

        <NewFundSheet open=new_open account_id=account_id.get_value() on_saved=on_saved />
        {move || selected.get().map(|fund| {
            let open = RwSignal::new(true);
            Effect::new(move |_| if !open.get() { selected.set(None) });
            view! { <FundSheet open=open account_id=account_id.get_value() fund=fund on_saved=on_saved /> }
        })}
    }
}

#[component]
fn NewFundSheet(open: RwSignal<bool>, account_id: String, on_saved: Callback<()>) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let units = RwSignal::new(String::new());
    let nav = RwSignal::new(String::new());
    let reference = RwSignal::new(None::<AssetDto>);
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let account_id = StoredValue::new(account_id);
    let submit = move |_| {
        let units = match format::parse_decimal(&units.get_untracked()) {
            Ok(units) => units,
            Err(message) => return error.set(Some(message)),
        };
        let nav = match format::parse_decimal(&nav.get_untracked()) {
            Ok(nav) => nav,
            Err(message) => return error.set(Some(message)),
        };
        let input = NewFundInput {
            account_id: account_id.get_value(),
            name: name.get_untracked(),
            units,
            reference: reference.get_untracked(),
            nav,
        };
        run_action(
            busy,
            error,
            api::create_pei_fund(input),
            move |_: String| {
                open.set(false);
                on_saved.run(());
            },
        );
    };
    view! {
        <BottomSheet open=open title="Nouveau fonds">
            <div class="flex flex-col gap-4">
                <Field label="Nom du fonds" value=name placeholder="Amundi Actions Monde" />
                <Field label="Nombre de parts" value=units input_mode="decimal" />
                <Field label="Valeur de part du jour" value=nav input_mode="decimal" suffix="€" />
                <AssetSearch source=QuoteSourceDto::TwelveData selected=reference label="Indice de référence (facultatif)" placeholder="IWDA, CW8…" />
                <p class="text-xs text-muted">{REFERENCE_HELP}</p>
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=submit>"Ajouter"</button>
            </div>
        </BottomSheet>
    }
}

/// Valeur de part réelle (recalage), modification et suppression d'un fonds.
#[component]
fn FundSheet(
    open: RwSignal<bool>,
    account_id: String,
    fund: FundDto,
    on_saved: Callback<()>,
) -> impl IntoView {
    let nav = RwSignal::new(String::new());
    let name = RwSignal::new(fund.name.clone());
    let units = RwSignal::new(format::quantity(fund.units));
    let reference = RwSignal::new(fund.reference.clone());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let ids = StoredValue::new((account_id, fund.id.clone()));

    let record = move |_| {
        let nav = match format::parse_decimal(&nav.get_untracked()) {
            Ok(nav) => nav,
            Err(message) => return error.set(Some(message)),
        };
        let (account_id, fund_id) = ids.get_value();
        run_action(
            busy,
            error,
            api::record_pei_nav(FundNavInput {
                account_id,
                fund_id,
                nav,
            }),
            move |()| {
                open.set(false);
                on_saved.run(());
            },
        );
    };
    let update = move |_| {
        let units = match format::parse_decimal(&units.get_untracked()) {
            Ok(units) => units,
            Err(message) => return error.set(Some(message)),
        };
        let (account_id, fund_id) = ids.get_value();
        let input = FundUpdateInput {
            account_id,
            fund_id,
            name: name.get_untracked(),
            units,
            reference: reference.get_untracked(),
        };
        run_action(busy, error, api::update_pei_fund(input), move |()| {
            open.set(false);
            on_saved.run(());
        });
    };
    let remove = move |_| {
        let (account_id, fund_id) = ids.get_value();
        run_action(
            busy,
            error,
            api::remove_pei_fund(FundRefInput {
                account_id,
                fund_id,
            }),
            move |()| {
                open.set(false);
                on_saved.run(());
            },
        );
    };

    let kind = match fund.nav_kind {
        NavKindDto::Real => "réelle",
        NavKindDto::Estimated => "estimée",
    };
    view! {
        <BottomSheet open=open title=fund.name.clone()>
            <div class="flex flex-col gap-4">
                <p class="text-sm text-muted">
                    "Valeur de part " {kind} " : " {format::amount(fund.nav)}
                    " (dernière saisie le " {format::date(fund.nav_date)} ")"
                </p>
                <Field label="Valeur de part réelle" value=nav input_mode="decimal" suffix="€" placeholder="Lue sur votre espace d'épargne salariale" />
                <button class=BTN_PRIMARY disabled=move || busy.get() on:click=record>"Enregistrer la valeur de part"</button>

                <h3 class=format!("{SECTION_TITLE} mt-2")>"Modifier le fonds"</h3>
                <Field label="Nom du fonds" value=name />
                <Field label="Nombre de parts" value=units input_mode="decimal" />
                <AssetSearch source=QuoteSourceDto::TwelveData selected=reference label="Indice de référence" placeholder="IWDA, CW8…" />
                <p class="min-h-5 text-sm text-loss" role="alert">{move || error.get()}</p>
                <button class=BTN_SECONDARY disabled=move || busy.get() on:click=update>"Enregistrer les modifications"</button>
                <button class=BTN_DANGER disabled=move || busy.get() on:click=remove>"Supprimer ce fonds"</button>
            </div>
        </BottomSheet>
    }
}
