use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetSummaryDto {
    /// `AAAA-MM`
    pub month: String,
    pub salary: Decimal,
    pub one_off_incomes: Decimal,
    pub total_income: Decimal,
    pub fixed_charges: Decimal,
    pub expenses: Decimal,
    pub average_expenses: Decimal,
    pub savings_capacity: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryDto {
    pub id: String,
    pub name: String,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalaryDto {
    pub id: String,
    /// `AAAA-MM`
    pub effective_from: String,
    pub net_amount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneOffIncomeDto {
    pub id: String,
    pub label: String,
    pub amount: Decimal,
    pub date: NaiveDate,
    /// Compte courant crédité.
    #[serde(default)]
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeriodicityDto {
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeDto {
    pub id: String,
    pub label: String,
    pub amount: Decimal,
    pub monthly_equivalent: Decimal,
    pub category_id: String,
    pub periodicity: PeriodicityDto,
    pub start: String,
    pub end: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpenseDto {
    pub id: String,
    pub amount: Decimal,
    pub category_id: String,
    pub date: NaiveDate,
    pub label: Option<String>,
    /// Compte courant débité.
    #[serde(default)]
    pub account_id: Option<String>,
}

/// Libellé déjà utilisé, proposé à la saisie d'une dépense avec sa dernière catégorie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelSuggestionDto {
    pub label: String,
    pub category_id: String,
    pub uses: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpenseListDto {
    pub month: String,
    pub category_id: Option<String>,
    pub items: Vec<ExpenseDto>,
    pub total: Decimal,
}

// --- Saisies ------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonthInput {
    /// Mois courant si absent.
    pub month: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpenseFilterInput {
    pub month: String,
    pub category_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryNameInput {
    /// Absent pour une création.
    pub category_id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalaryInput {
    /// Absent pour une création.
    pub id: Option<String>,
    pub effective_from: String,
    pub net_amount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneOffIncomeInput {
    pub id: Option<String>,
    pub label: String,
    pub amount: Decimal,
    pub date: NaiveDate,
    /// Compte courant crédité, le cas échéant.
    #[serde(default)]
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeInput {
    pub id: Option<String>,
    pub label: String,
    pub amount: Decimal,
    pub category_id: String,
    pub periodicity: PeriodicityDto,
    pub start: String,
    pub end: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpenseInput {
    /// Absent pour une création.
    pub id: Option<String>,
    pub amount: Decimal,
    pub category_id: String,
    /// Aujourd'hui si absent.
    pub date: Option<NaiveDate>,
    pub label: Option<String>,
    /// Compte courant débité, le cas échéant.
    #[serde(default)]
    pub account_id: Option<String>,
}

/// Référence à un élément à supprimer, archiver… (identifiant UUID).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdInput {
    pub id: String,
}
