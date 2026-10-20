//! Tableau de bord et projection.

use lc_shared_dto::*;

use super::{AppState, blocking, unlocked};
use crate::error::{CommandResult, app_error};
use crate::mapping;

#[tauri::command]
pub async fn get_dashboard(
    state: AppState<'_>,
    input: RefreshInput,
) -> CommandResult<DashboardDto> {
    let services = unlocked(&state)?;
    let dashboard = services
        .dashboard
        .dashboard(input.force_refresh)
        .await
        .map_err(app_error)?;
    let reminder = blocking(services, |services| {
        services.backup.export_reminder_due().map_err(app_error)
    })
    .await?;
    Ok(mapping::dashboard_dto(&dashboard, reminder))
}

#[tauri::command]
pub async fn take_manual_snapshot(state: AppState<'_>) -> CommandResult<()> {
    let services = unlocked(&state)?;
    services
        .dashboard
        .take_manual_snapshot()
        .await
        .map_err(app_error)?;
    Ok(())
}

#[tauri::command]
pub async fn get_projection(state: AppState<'_>) -> CommandResult<ProjectionDto> {
    let services = unlocked(&state)?;
    let report = services.projection.projection().await.map_err(app_error)?;
    let accounts = services.accounts.list_accounts(true).map_err(app_error)?;
    Ok(mapping::projection_dto(&report, &accounts))
}
