//! Objectifs d'épargne et bilan du mois.

use lc_application::goals::GoalInput as UseCaseGoalInput;
use lc_domain::{AccountId, GoalId, Money};
use lc_shared_dto::*;

use super::{AppState, blocking, unlocked};
use crate::error::{CommandResult, app_error};
use crate::mapping::{self, id, month};

#[tauri::command]
pub async fn list_goals(state: AppState<'_>) -> CommandResult<Vec<GoalDto>> {
    let services = unlocked(&state)?;
    let report = services.goals.report().await.map_err(app_error)?;
    let accounts = services.accounts.list_accounts(false).map_err(app_error)?;
    Ok(report
        .goals
        .iter()
        .map(|progress| mapping::goal_dto(progress, &accounts))
        .collect())
}

#[tauri::command]
pub async fn save_goal(state: AppState<'_>, input: GoalInput) -> CommandResult<String> {
    blocking(unlocked(&state)?, move |services| {
        let goal = UseCaseGoalInput {
            name: input.name,
            target: Money::new(input.target),
            deadline: input.deadline.as_deref().map(month).transpose()?,
            accounts: input
                .account_ids
                .iter()
                .map(|raw| id(raw, AccountId::from_uuid))
                .collect::<CommandResult<_>>()?,
        };
        match input.id.as_deref() {
            Some(raw) => {
                let goal_id = id(raw, GoalId::from_uuid)?;
                services
                    .goals
                    .update_goal(goal_id, goal)
                    .map_err(app_error)?;
                Ok(goal_id.to_string())
            }
            None => services
                .goals
                .create_goal(goal)
                .map(|goal_id| goal_id.to_string())
                .map_err(app_error),
        }
    })
    .await
}

#[tauri::command]
pub async fn delete_goal(state: AppState<'_>, input: IdInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .goals
            .delete_goal(id(&input.id, GoalId::from_uuid)?)
            .map_err(app_error)
    })
    .await
}

/// Bilan du mois précédent s'il n'a jamais été présenté (il est alors marqué comme vu), avec la
/// question de recalage du compte courant.
#[tauri::command]
pub async fn pending_review(state: AppState<'_>) -> CommandResult<Option<ReviewDto>> {
    blocking(unlocked(&state)?, |services| {
        let reviews = &services.reviews;
        let Some(month) = reviews.pending_review().map_err(app_error)? else {
            return Ok(None);
        };
        reviews.open_review(month).map_err(app_error)?;
        let report = reviews.month_report(month).map_err(app_error)?;
        let start = reviews.start_prompt().map_err(app_error)?;
        let categories = services.budget.categories(true).map_err(app_error)?;
        Ok(Some(ReviewDto {
            report: mapping::month_report_dto(&report, &categories),
            start: start.as_ref().map(mapping::start_prompt_dto),
        }))
    })
    .await
}

/// Bilan d'un mois passé (historique), sans recalage.
#[tauri::command]
pub async fn get_review(state: AppState<'_>, input: MonthInput) -> CommandResult<ReviewDto> {
    blocking(unlocked(&state)?, move |services| {
        let raw = input.month.unwrap_or_default();
        let report = services
            .reviews
            .month_report(month(&raw)?)
            .map_err(app_error)?;
        let categories = services.budget.categories(true).map_err(app_error)?;
        Ok(ReviewDto {
            report: mapping::month_report_dto(&report, &categories),
            start: None,
        })
    })
    .await
}

/// Mois dont le bilan peut être revu, du plus récent au plus ancien (`AAAA-MM`).
#[tauri::command]
pub async fn review_months(state: AppState<'_>) -> CommandResult<Vec<String>> {
    blocking(unlocked(&state)?, |services| {
        Ok(services
            .reviews
            .review_months()
            .map_err(app_error)?
            .iter()
            .map(ToString::to_string)
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn start_month(state: AppState<'_>, input: StartMonthInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .reviews
            .start_month(Money::new(input.balance))
            .map_err(app_error)
    })
    .await
}
