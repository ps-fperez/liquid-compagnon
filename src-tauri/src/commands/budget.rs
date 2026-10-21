//! Revenus, charges, dépenses et catégories.

use lc_application::budget::{
    ChargeInput as UseCaseChargeInput, ExpenseInput as UseCaseExpenseInput,
};
use lc_domain::budget::Debit;
use lc_domain::{AccountId, CategoryId, ChargeId, ExpenseId, IncomeId, Money, SalaryId};
use lc_shared_dto::*;

use super::{AppState, blocking, unlocked};
use crate::error::{CommandResult, app_error};
use crate::mapping::{self, id, month, optional_id};

#[tauri::command]
pub async fn budget_summary(
    state: AppState<'_>,
    input: MonthInput,
) -> CommandResult<BudgetSummaryDto> {
    blocking(unlocked(&state)?, move |services| {
        let month = input.month.as_deref().map(month).transpose()?;
        let summary = services.budget.summary(month).map_err(app_error)?;
        Ok(mapping::budget_summary_dto(&summary))
    })
    .await
}

// --- Catégories ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_categories(
    state: AppState<'_>,
    input: IncludeArchivedInput,
) -> CommandResult<Vec<CategoryDto>> {
    blocking(unlocked(&state)?, move |services| {
        let categories = services
            .budget
            .categories(input.include_archived)
            .map_err(app_error)?;
        Ok(categories.iter().map(mapping::category_dto).collect())
    })
    .await
}

/// Crée une catégorie (`category_id` absent) ou la renomme.
#[tauri::command]
pub async fn save_category(state: AppState<'_>, input: CategoryNameInput) -> CommandResult<String> {
    blocking(unlocked(&state)?, move |services| {
        match optional_id(input.category_id.as_deref(), CategoryId::from_uuid)? {
            Some(category_id) => {
                services
                    .budget
                    .rename_category(category_id, &input.name)
                    .map_err(app_error)?;
                Ok(category_id.to_string())
            }
            None => Ok(services
                .budget
                .create_category(&input.name)
                .map_err(app_error)?
                .to_string()),
        }
    })
    .await
}

#[tauri::command]
pub async fn archive_category(state: AppState<'_>, input: IdInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .budget
            .archive_category(id(&input.id, CategoryId::from_uuid)?)
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn unarchive_category(state: AppState<'_>, input: IdInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .budget
            .unarchive_category(id(&input.id, CategoryId::from_uuid)?)
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn delete_category(state: AppState<'_>, input: IdInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .budget
            .delete_category(id(&input.id, CategoryId::from_uuid)?)
            .map_err(app_error)
    })
    .await
}

// --- Revenus ------------------------------------------------------------------------------

#[tauri::command]
pub async fn salary_history(state: AppState<'_>) -> CommandResult<Vec<SalaryDto>> {
    blocking(unlocked(&state)?, |services| {
        let history = services.budget.salary_history().map_err(app_error)?;
        Ok(history.iter().map(mapping::salary_dto).collect())
    })
    .await
}

#[tauri::command]
pub async fn upsert_salary(state: AppState<'_>, input: SalaryInput) -> CommandResult<String> {
    blocking(unlocked(&state)?, move |services| {
        let salary_id = services
            .budget
            .upsert_salary(
                optional_id(input.id.as_deref(), SalaryId::from_uuid)?,
                month(&input.effective_from)?,
                Money::new(input.net_amount),
            )
            .map_err(app_error)?;
        Ok(salary_id.to_string())
    })
    .await
}

#[tauri::command]
pub async fn delete_salary(state: AppState<'_>, input: IdInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .budget
            .delete_salary(id(&input.id, SalaryId::from_uuid)?)
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn list_one_off_incomes(
    state: AppState<'_>,
    input: MonthInput,
) -> CommandResult<Vec<OneOffIncomeDto>> {
    blocking(unlocked(&state)?, move |services| {
        let month = match input.month.as_deref() {
            Some(raw) => month(raw)?,
            None => services.budget.current_month(),
        };
        let incomes = services.budget.one_off_incomes(month).map_err(app_error)?;
        Ok(incomes.iter().map(mapping::one_off_income_dto).collect())
    })
    .await
}

#[tauri::command]
pub async fn upsert_one_off_income(
    state: AppState<'_>,
    input: OneOffIncomeInput,
) -> CommandResult<String> {
    blocking(unlocked(&state)?, move |services| {
        let income_id = services
            .budget
            .upsert_one_off_income(
                optional_id(input.id.as_deref(), IncomeId::from_uuid)?,
                &input.label,
                Money::new(input.amount),
                input.date,
                optional_id(input.account_id.as_deref(), AccountId::from_uuid)?,
            )
            .map_err(app_error)?;
        Ok(income_id.to_string())
    })
    .await
}

#[tauri::command]
pub async fn delete_one_off_income(state: AppState<'_>, input: IdInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .budget
            .delete_one_off_income(id(&input.id, IncomeId::from_uuid)?)
            .map_err(app_error)
    })
    .await
}

// --- Charges fixes ------------------------------------------------------------------------

#[tauri::command]
pub async fn expense_suggestions(state: AppState<'_>) -> CommandResult<Vec<LabelSuggestionDto>> {
    blocking(unlocked(&state)?, |services| {
        let suggestions = services.budget.expense_suggestions().map_err(app_error)?;
        Ok(suggestions
            .into_iter()
            .map(|s| LabelSuggestionDto {
                label: s.label,
                category_id: s.category.to_string(),
                uses: u32::try_from(s.uses).unwrap_or(u32::MAX),
            })
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn list_charges(state: AppState<'_>) -> CommandResult<Vec<ChargeDto>> {
    blocking(unlocked(&state)?, |services| {
        let charges = services.budget.charges().map_err(app_error)?;
        Ok(charges.iter().map(mapping::charge_dto).collect())
    })
    .await
}

#[tauri::command]
pub async fn upsert_charge(state: AppState<'_>, input: ChargeInput) -> CommandResult<String> {
    blocking(unlocked(&state)?, move |services| {
        let charge = UseCaseChargeInput {
            label: input.label,
            amount: Money::new(input.amount),
            category: id(&input.category_id, CategoryId::from_uuid)?,
            periodicity: mapping::periodicity(input.periodicity),
            start: month(&input.start)?,
            end: input.end.as_deref().map(month).transpose()?,
            debit: match (input.debit_account_id.as_deref(), input.debit_day) {
                (Some(account), Some(day)) => Some(Debit {
                    account: id(account, AccountId::from_uuid)?,
                    day,
                }),
                _ => None,
            },
        };
        let charge_id = services
            .budget
            .upsert_charge(
                optional_id(input.id.as_deref(), ChargeId::from_uuid)?,
                charge,
            )
            .map_err(app_error)?;
        Ok(charge_id.to_string())
    })
    .await
}

#[tauri::command]
pub async fn delete_charge(state: AppState<'_>, input: IdInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .budget
            .delete_charge(id(&input.id, ChargeId::from_uuid)?)
            .map_err(app_error)
    })
    .await
}

// --- Dépenses -----------------------------------------------------------------------------

/// Ajoute une dépense (`id` absent) ou la modifie.
#[tauri::command]
pub async fn save_expense(state: AppState<'_>, input: ExpenseInput) -> CommandResult<String> {
    blocking(unlocked(&state)?, move |services| {
        let expense = UseCaseExpenseInput {
            amount: Money::new(input.amount),
            category: id(&input.category_id, CategoryId::from_uuid)?,
            date: input.date,
            label: input.label,
            account: optional_id(input.account_id.as_deref(), AccountId::from_uuid)?,
        };
        match optional_id(input.id.as_deref(), ExpenseId::from_uuid)? {
            Some(expense_id) => {
                services
                    .budget
                    .update_expense(expense_id, expense)
                    .map_err(app_error)?;
                Ok(expense_id.to_string())
            }
            None => Ok(services
                .budget
                .add_expense(expense)
                .map_err(app_error)?
                .to_string()),
        }
    })
    .await
}

#[tauri::command]
pub async fn delete_expense(state: AppState<'_>, input: IdInput) -> CommandResult<()> {
    blocking(unlocked(&state)?, move |services| {
        services
            .budget
            .delete_expense(id(&input.id, ExpenseId::from_uuid)?)
            .map_err(app_error)
    })
    .await
}

#[tauri::command]
pub async fn list_expenses(
    state: AppState<'_>,
    input: ExpenseFilterInput,
) -> CommandResult<ExpenseListDto> {
    blocking(unlocked(&state)?, move |services| {
        let list = services
            .budget
            .list_expenses(
                month(&input.month)?,
                optional_id(input.category_id.as_deref(), CategoryId::from_uuid)?,
            )
            .map_err(app_error)?;
        Ok(mapping::expense_list_dto(&list))
    })
    .await
}
