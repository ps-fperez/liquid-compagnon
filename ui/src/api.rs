//! Appels typés vers les commandes Tauri. Seul module de l'UI qui connaît les noms des commandes.
//!
//! Chaque commande prend au plus un argument `input` (DTO). Une erreur `session_locked`
//! renvoyée par n'importe quelle commande repasse l'UI sur l'écran de verrouillage.

use lc_shared_dto::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::prelude::*;

use crate::state;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

pub type ApiResult<T> = Result<T, ErrorDto>;

/// Vrai si l'UI tourne dans Tauri (faux dans un simple navigateur, ex. `trunk serve`).
pub fn tauri_available() -> bool {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("__TAURI__"))
        .map(|value| !value.is_undefined())
        .unwrap_or(false)
}

#[derive(Serialize)]
struct WithInput<T> {
    input: T,
}

#[derive(Serialize)]
struct NoArgs {}

fn internal(message: impl Into<String>) -> ErrorDto {
    ErrorDto::new(ErrorCode::Internal, message)
}

async fn call<T: DeserializeOwned>(command: &str, args: impl Serialize) -> ApiResult<T> {
    if !tauri_available() {
        return Err(internal(
            "backend Tauri absent (interface ouverte hors de l'application)",
        ));
    }
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    let args = args
        .serialize(&serializer)
        .map_err(|error| internal(error.to_string()))?;
    match tauri_invoke(command, args).await {
        Ok(value) => {
            serde_wasm_bindgen::from_value(value).map_err(|error| internal(error.to_string()))
        }
        Err(raw) => {
            let error =
                serde_wasm_bindgen::from_value::<ErrorDto>(raw.clone()).unwrap_or_else(|_| {
                    internal(raw.as_string().unwrap_or_else(|| format!("{raw:?}")))
                });
            if error.code == ErrorCode::SessionLocked {
                state::session_locked();
            }
            Err(error)
        }
    }
}

macro_rules! commands_with_input {
    ($($(#[$doc:meta])* $name:ident($input:ty) -> $output:ty;)*) => {$(
        $(#[$doc])*
        pub async fn $name(input: $input) -> ApiResult<$output> {
            call(stringify!($name), WithInput { input }).await
        }
    )*};
}

macro_rules! commands_without_input {
    ($($(#[$doc:meta])* $name:ident -> $output:ty;)*) => {$(
        $(#[$doc])*
        pub async fn $name() -> ApiResult<$output> {
            call(stringify!($name), NoArgs {}).await
        }
    )*};
}

commands_without_input! {
    ping -> PingResponse;
    // Session, sécurité, réglages, sauvegarde
    session_status -> SessionStatusDto;
    unlock_with_biometric -> ();
    lock -> ();
    notify_hidden -> ();
    /// `true` si la session vient d'être verrouillée (plus de 60 s en arrière-plan).
    notify_visible -> bool;
    get_settings -> SettingsDto;
    export_backup -> ExportDto;
    // Tableau de bord et projection
    take_manual_snapshot -> ();
    get_projection -> ProjectionDto;
    // Budget
    salary_history -> Vec<SalaryDto>;
    list_charges -> Vec<ChargeDto>;
    expense_suggestions -> Vec<LabelSuggestionDto>;
}

commands_with_input! {
    setup_pin(SetupPinInput) -> ();
    unlock(PinInput) -> ();
    change_pin(ChangePinInput) -> ();
    set_biometric_enabled(ToggleInput) -> ();
    set_onboarding_done(ToggleInput) -> ();
    set_twelve_data_api_key(ApiKeyInput) -> ();
    import_backup(ImportInput) -> ImportSummaryDto;
    get_dashboard(RefreshInput) -> DashboardDto;
    // Comptes
    list_accounts(IncludeArchivedInput) -> AccountListDto;
    get_account(IdInput) -> AccountDto;
    /// Renvoie l'identifiant du compte créé.
    create_account(NewAccountInput) -> String;
    update_account(AccountUpdateInput) -> ();
    archive_account(IdInput) -> ();
    unarchive_account(IdInput) -> ();
    update_savings_balance(SavingsBalanceInput) -> ();
    set_savings_terms(SavingsTermsInput) -> ();
    set_scenario_rates(ScenarioRatesInput) -> ();
    set_cash(CashInput) -> ();
    upsert_position(PositionInput) -> ();
    remove_position(RemovePositionInput) -> ();
    search_assets(AssetSearchInput) -> Vec<AssetDto>;
    /// Offre associée à la clé (`None` pour une source sans compte).
    price_source_status(QuoteSourceDto) -> Option<String>;
    create_pei_fund(NewFundInput) -> String;
    update_pei_fund(FundUpdateInput) -> ();
    record_pei_nav(FundNavInput) -> ();
    remove_pei_fund(FundRefInput) -> ();
    // Budget
    budget_summary(MonthInput) -> BudgetSummaryDto;
    list_categories(IncludeArchivedInput) -> Vec<CategoryDto>;
    save_category(CategoryNameInput) -> String;
    archive_category(IdInput) -> ();
    unarchive_category(IdInput) -> ();
    delete_category(IdInput) -> ();
    upsert_salary(SalaryInput) -> String;
    delete_salary(IdInput) -> ();
    list_one_off_incomes(MonthInput) -> Vec<OneOffIncomeDto>;
    upsert_one_off_income(OneOffIncomeInput) -> String;
    delete_one_off_income(IdInput) -> ();
    upsert_charge(ChargeInput) -> String;
    delete_charge(IdInput) -> ();
    save_expense(ExpenseInput) -> String;
    delete_expense(IdInput) -> ();
    list_expenses(ExpenseFilterInput) -> ExpenseListDto;
}
