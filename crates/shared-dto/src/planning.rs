use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalStatusDto {
    Reached,
    OnTrack,
    Late,
    /// Sans échéance.
    Open,
}

/// Objectif d'épargne et sa situation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalDto {
    pub id: String,
    pub name: String,
    pub target: Decimal,
    /// `AAAA-MM`
    pub deadline: Option<String>,
    pub account_ids: Vec<String>,
    /// Noms des comptes liés actifs.
    pub account_names: Vec<String>,
    pub current: Decimal,
    pub percent: Decimal,
    pub remaining: Decimal,
    pub required_monthly: Option<Decimal>,
    pub contributed_monthly: Decimal,
    /// `AAAA-MM` : pessimiste, neutre, optimiste ; absent au-delà de 10 ans.
    pub arrivals: [Option<String>; 3],
    pub status: GoalStatusDto,
    pub just_reached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalInput {
    pub id: Option<String>,
    pub name: String,
    pub target: Decimal,
    pub deadline: Option<String>,
    pub account_ids: Vec<String>,
}

/// Progression d'un objectif en fin de mois.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalMonthDto {
    pub name: String,
    pub percent: Decimal,
    pub previous_percent: Option<Decimal>,
}

/// Bilan d'un mois écoulé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonthReportDto {
    /// `AAAA-MM`
    pub month: String,
    pub saved: Decimal,
    pub previous_saved: Option<Decimal>,
    pub received: Decimal,
    pub one_off_incomes: Decimal,
    pub fixed_charges: Decimal,
    pub expenses: Decimal,
    pub external_out: Decimal,
    pub top_categories: Vec<crate::CategoryAmountDto>,
    /// Catégorie en plus forte hausse sur un mois et hausse en pourcentage.
    pub biggest_increase: Option<(String, Decimal)>,
    pub wealth_end: Option<Decimal>,
    pub wealth_change: Option<Decimal>,
    pub wealth_change_percent: Option<Decimal>,
    pub goals: Vec<GoalMonthDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncludedTransferDto {
    pub counterpart: String,
    pub amount: Decimal,
    pub date: chrono::NaiveDate,
}

/// « Votre compte courant repart à combien ? »
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartPromptDto {
    pub account_name: String,
    pub suggested: Decimal,
    pub included: Vec<IncludedTransferDto>,
}

/// Bilan à présenter : le rapport et, en début de mois, la question de recalage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewDto {
    pub report: MonthReportDto,
    pub start: Option<StartPromptDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartMonthInput {
    pub balance: Decimal,
}
