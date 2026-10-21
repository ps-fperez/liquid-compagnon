//! Comptes, positions, fonds PEI et recherche d'actifs.

use lc_application::accounts::{AccountUpdate, FundInput, NewAccount};
use lc_domain::account::AccountType;
use lc_domain::{AccountId, FundId, Money};
use lc_shared_dto::*;

use super::{AppState, blocking, unlocked};
use crate::error::{CommandResult, app_error};
use crate::mapping::{self, asset, id};

#[tauri::command]
pub async fn list_accounts(
    state: AppState<'_>,
    input: IncludeArchivedInput,
) -> CommandResult<AccountListDto> {
    let services = unlocked(&state)?;
    let valued = services
        .accounts
        .valued_accounts(input.include_archived)
        .await
        .map_err(app_error)?;
    Ok(AccountListDto {
        accounts: valued
            .accounts
            .iter()
            .map(|(account, valuation)| mapping::account_dto(account, valuation))
            .collect(),
        pricing_issues: mapping::pricing_issues_dto(&valued.pricing_issues),
    })
}

#[tauri::command]
pub async fn get_account(state: AppState<'_>, input: IdInput) -> CommandResult<AccountDto> {
    let services = unlocked(&state)?;
    let account_id = id(&input.id, AccountId::from_uuid)?;
    let (account, valuation, _) = services
        .accounts
        .valued_account(account_id)
        .await
        .map_err(app_error)?;
    Ok(mapping::account_dto(&account, &valuation))
}

#[tauri::command]
pub async fn create_account(state: AppState<'_>, input: NewAccountInput) -> CommandResult<String> {
    blocking(unlocked(&state)?, move |services| {
        let account_id = services
            .accounts
            .create_account(NewAccount {
                name: input.name,
                account_type: mapping::account_type(input.account_type),
                color: input.color,
                monthly_contribution: Money::new(input.monthly_contribution),
            })
            .map_err(app_error)?;
        Ok(account_id.to_string())
    })
    .await
}

#[tauri::command]
pub async fn update_account(state: AppState<'_>, input: AccountUpdateInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        let account_id = id(&input.account_id, AccountId::from_uuid)?;
        services
            .accounts
            .update_account(
                account_id,
                AccountUpdate {
                    name: input.name,
                    color: input.color,
                    monthly_contribution: Money::new(input.monthly_contribution),
                },
            )
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn archive_account(state: AppState<'_>, input: IdInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .accounts
            .archive_account(id(&input.id, AccountId::from_uuid)?)
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn unarchive_account(state: AppState<'_>, input: IdInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .accounts
            .unarchive_account(id(&input.id, AccountId::from_uuid)?)
            .map_err(app_error)
    })
    .await
}

/// Comptes courants actifs, dans l'ordre de création : le premier est proposé par défaut pour
/// les dépenses et revenus ponctuels. Sans valorisation (aucun appel réseau).
#[tauri::command]
pub async fn checking_accounts(state: AppState<'_>) -> CommandResult<Vec<AccountOptionDto>> {
    blocking(unlocked(&state)?, |services| {
        let accounts = services.accounts.list_accounts(false).map_err(app_error)?;
        Ok(accounts
            .iter()
            .filter(|a| a.account_type() == AccountType::Checking)
            .map(|a| AccountOptionDto {
                id: a.id().to_string(),
                name: a.name().to_string(),
            })
            .collect())
    })
    .await
}

/// Nouveau solde d'un livret ou d'un compte courant (qui devient sa référence).
#[tauri::command]
pub async fn update_balance(state: AppState<'_>, input: BalanceInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .accounts
            .update_balance(
                id(&input.account_id, AccountId::from_uuid)?,
                Money::new(input.balance),
            )
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn set_savings_terms(state: AppState<'_>, input: SavingsTermsInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .accounts
            .set_savings_terms(
                id(&input.account_id, AccountId::from_uuid)?,
                mapping::rate(input.rate_percent)?,
                Money::new(input.deposit_ceiling),
            )
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn set_scenario_rates(
    state: AppState<'_>,
    input: ScenarioRatesInput,
) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .accounts
            .set_scenario_rates(
                id(&input.account_id, AccountId::from_uuid)?,
                mapping::scenario_rates(input.rates)?,
            )
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn set_cash(state: AppState<'_>, input: CashInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .accounts
            .set_cash(
                id(&input.account_id, AccountId::from_uuid)?,
                Money::new(input.cash),
            )
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn upsert_position(state: AppState<'_>, input: PositionInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .accounts
            .upsert_position(
                id(&input.account_id, AccountId::from_uuid)?,
                asset(&input.asset)?,
                input.quantity,
                input.unit_cost.map(Money::new),
            )
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn remove_position(state: AppState<'_>, input: RemovePositionInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .accounts
            .remove_position(
                id(&input.account_id, AccountId::from_uuid)?,
                &asset(&input.asset)?,
            )
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn search_assets(
    state: AppState<'_>,
    input: AssetSearchInput,
) -> CommandResult<Vec<AssetDto>> {
    let services = unlocked(&state)?;
    let results = services
        .accounts
        .search_assets(mapping::quote_source(input.source), &input.query)
        .await
        .map_err(app_error)?;
    Ok(results.iter().map(mapping::asset_dto).collect())
}

/// Vérifie la clé d'une source et décrit l'offre associée (`None` pour une source sans compte).
#[tauri::command]
pub async fn price_source_status(
    state: AppState<'_>,
    input: QuoteSourceDto,
) -> CommandResult<Option<String>> {
    let services = unlocked(&state)?;
    services
        .accounts
        .price_source_status(mapping::quote_source(input))
        .await
        .map_err(app_error)
}

#[tauri::command]
pub async fn create_pei_fund(state: AppState<'_>, input: NewFundInput) -> CommandResult<String> {
    let services = unlocked(&state)?;
    let fund = FundInput {
        name: input.name,
        units: input.units,
        reference: input.reference.as_ref().map(asset).transpose()?,
    };
    let fund_id = services
        .accounts
        .create_pei_fund(
            id(&input.account_id, AccountId::from_uuid)?,
            fund,
            Money::new(input.nav),
        )
        .await
        .map_err(app_error)?;
    Ok(fund_id.to_string())
}

#[tauri::command]
pub async fn update_pei_fund(state: AppState<'_>, input: FundUpdateInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        let fund = FundInput {
            name: input.name,
            units: input.units,
            reference: input.reference.as_ref().map(asset).transpose()?,
        };
        services
            .accounts
            .update_pei_fund(
                id(&input.account_id, AccountId::from_uuid)?,
                id(&input.fund_id, FundId::from_uuid)?,
                fund,
            )
            .map_err(app_error)
    })
    .await
}

/// Valeur de part réelle : elle devient le nouvel ancrage de l'estimation.
#[tauri::command]
pub async fn record_pei_nav(state: AppState<'_>, input: FundNavInput) -> CommandResult<()> {
    let services = unlocked(&state)?;
    services
        .accounts
        .record_pei_nav(
            id(&input.account_id, AccountId::from_uuid)?,
            id(&input.fund_id, FundId::from_uuid)?,
            Money::new(input.nav),
        )
        .await
        .map_err(app_error)
}

#[tauri::command]
pub async fn remove_pei_fund(state: AppState<'_>, input: FundRefInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .accounts
            .remove_pei_fund(
                id(&input.account_id, AccountId::from_uuid)?,
                id(&input.fund_id, FundId::from_uuid)?,
            )
            .map_err(app_error)
    })
    .await
}
