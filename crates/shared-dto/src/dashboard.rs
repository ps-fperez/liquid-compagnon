use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{AccountTypeDto, BudgetSummaryDto, PricingIssueDto, ProvenanceDto, VariationDto};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountLineDto {
    pub id: String,
    pub name: String,
    pub color: String,
    pub account_type: AccountTypeDto,
    pub value: Decimal,
    /// Part du patrimoine total, en pourcentage.
    pub share_percent: Decimal,
    pub variation: Option<VariationDto>,
    pub provenance: ProvenanceDto,
}

/// Point de la courbe d'évolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryPointDto {
    /// `AAAA-MM`
    pub month: String,
    pub total: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardDto {
    pub total: Decimal,
    pub variation: Option<VariationDto>,
    pub accounts: Vec<AccountLineDto>,
    pub budget: BudgetSummaryDto,
    pub history: Vec<HistoryPointDto>,
    pub pricing_issues: Vec<PricingIssueDto>,
    pub export_reminder_due: bool,
}
