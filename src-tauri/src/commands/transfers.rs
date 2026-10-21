//! Virements entre comptes et comptes hors patrimoine.

use lc_application::transfers::TransferInput as UseCaseTransferInput;
use lc_domain::transfer::Endpoint;
use lc_domain::{ExternalId, Money, TransferId};
use lc_shared_dto::*;

use super::{AppState, blocking, unlocked};
use crate::error::{CommandResult, app_error};
use crate::mapping::{self, Names, id};
use crate::state::Services;

/// Noms de tous les comptes, archivés compris, pour nommer le compte d'en face d'un virement.
pub(crate) fn names(services: &Services) -> CommandResult<Names> {
    let accounts = services.accounts.list_accounts(true).map_err(app_error)?;
    let externals = services.transfers.external_accounts().map_err(app_error)?;
    Ok(Names::new(&accounts, &externals))
}

/// Virements d'un compte (suivi ou hors patrimoine), ou tous les virements sans `id`.
#[tauri::command]
pub async fn list_transfers(
    state: AppState<'_>,
    input: Option<EndpointDto>,
) -> CommandResult<Vec<TransferDto>> {
    blocking(unlocked(&state)?, move |services| {
        let transfers = match input.as_ref() {
            Some(endpoint) => services
                .transfers
                .transfers_of(mapping::endpoint(endpoint)?),
            None => services.transfers.transfers(),
        }
        .map_err(app_error)?;
        let names = names(services)?;
        Ok(transfers
            .iter()
            .map(|transfer| mapping::transfer_dto(transfer, &names))
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn save_transfer(state: AppState<'_>, input: TransferInput) -> CommandResult<String> {
    blocking(unlocked(&state)?, move |services| {
        let transfer = UseCaseTransferInput {
            from: mapping::endpoint(&input.from)?,
            to: mapping::endpoint(&input.to)?,
            sent: Money::new(input.sent),
            received: input.received.map(Money::new),
            label: input.label,
            schedule: mapping::schedule(&input.schedule)?,
        };
        match input.id.as_deref() {
            Some(raw) => {
                let transfer_id = id(raw, TransferId::from_uuid)?;
                services
                    .transfers
                    .update_transfer(transfer_id, transfer)
                    .map_err(app_error)?;
                Ok(transfer_id.to_string())
            }
            None => services
                .transfers
                .create_transfer(transfer)
                .map(|transfer_id| transfer_id.to_string())
                .map_err(app_error),
        }
    })
    .await
}

#[tauri::command]
pub async fn delete_transfer(state: AppState<'_>, input: IdInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .transfers
            .delete_transfer(id(&input.id, TransferId::from_uuid)?)
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn list_external_accounts(state: AppState<'_>) -> CommandResult<Vec<ExternalAccountDto>> {
    blocking(unlocked(&state)?, |services| {
        let transfers = services.transfers.transfers().map_err(app_error)?;
        let externals = services.transfers.external_accounts().map_err(app_error)?;
        Ok(externals
            .iter()
            .map(|external| ExternalAccountDto {
                id: external.id().to_string(),
                name: external.name().to_string(),
                used: transfers
                    .iter()
                    .any(|t| t.involves(Endpoint::External(external.id()))),
            })
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn create_external_account(
    state: AppState<'_>,
    input: NameInput,
) -> CommandResult<String> {
    blocking(unlocked(&state)?, move |services| {
        services
            .transfers
            .create_external_account(&input.name)
            .map(|external_id| external_id.to_string())
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn rename_external_account(state: AppState<'_>, input: RenameInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .transfers
            .rename_external_account(id(&input.id, ExternalId::from_uuid)?, &input.name)
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn delete_external_account(state: AppState<'_>, input: IdInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .transfers
            .delete_external_account(id(&input.id, ExternalId::from_uuid)?)
            .map_err(app_error)
    })
    .await
}
