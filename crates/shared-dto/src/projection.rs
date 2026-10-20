use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::PricingIssueDto;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioDto {
    Pessimistic,
    Neutral,
    Optimistic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountTrajectoryDto {
    pub account_id: String,
    pub name: String,
    pub color: String,
    /// Valeur dans `m` mois à l'indice `m` (0 = aujourd'hui).
    pub values: Vec<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioProjectionDto {
    pub scenario: ScenarioDto,
    /// Patrimoine total projeté, indice = nombre de mois.
    pub totals: Vec<Decimal>,
    /// Épargne non affectée cumulée, indice = nombre de mois.
    pub unallocated: Vec<Decimal>,
    pub accounts: Vec<AccountTrajectoryDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionDto {
    /// Pessimiste, neutre, optimiste, sur 12 mois.
    pub scenarios: Vec<ScenarioProjectionDto>,
    pub savings_capacity: Decimal,
    pub total_contributions: Decimal,
    /// Montant dont les versements programmés dépassent la capacité d'épargne.
    pub contributions_excess: Option<Decimal>,
    pub pricing_issues: Vec<PricingIssueDto>,
}
