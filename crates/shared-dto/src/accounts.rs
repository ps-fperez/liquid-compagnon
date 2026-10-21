use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{AccountTypeDto, AssetDto, PricingIssueDto, ProvenanceDto, ScenarioRatesDto};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionDto {
    pub asset: AssetDto,
    pub quantity: Decimal,
    pub unit_price: Option<Decimal>,
    pub value: Decimal,
    pub provenance: ProvenanceDto,
    pub cost_basis: Option<Decimal>,
    pub unrealized_gain: Option<Decimal>,
    pub unrealized_gain_percent: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavKindDto {
    /// Valeur de part saisie.
    Real,
    /// Valeur extrapolée par l'actif de référence.
    Estimated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FundDto {
    pub id: String,
    pub name: String,
    pub units: Decimal,
    pub reference: Option<AssetDto>,
    pub nav: Decimal,
    pub nav_kind: NavKindDto,
    /// Date de la dernière valeur de part saisie.
    pub nav_date: NaiveDate,
    pub value: Decimal,
}

/// Contenu propre à la nature du compte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "nature", rename_all = "snake_case")]
pub enum AccountDetailsDto {
    Savings {
        balance: Decimal,
        rate_percent: Decimal,
        deposit_ceiling: Decimal,
        balance_updated_on: NaiveDate,
        ceiling_reached: bool,
    },
    Securities {
        cash: Decimal,
        positions: Vec<PositionDto>,
    },
    EmployeeSavings {
        funds: Vec<FundDto>,
    },
    Checking {
        /// Solde courant : référence et mouvements comptés.
        balance: Decimal,
        reference_balance: Decimal,
        reference_on: NaiveDate,
        /// Mouvements comptés depuis la référence, du plus récent au plus ancien.
        movements: Vec<CheckingMovementDto>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementKindDto {
    Expense,
    Income,
}

/// Dépense ou revenu ponctuel rattaché à un compte courant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckingMovementDto {
    pub kind: MovementKindDto,
    /// Montant signé : négatif pour une dépense.
    pub amount: Decimal,
    pub date: NaiveDate,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountDto {
    pub id: String,
    pub name: String,
    pub color: String,
    pub account_type: AccountTypeDto,
    pub archived: bool,
    pub monthly_contribution: Decimal,
    pub scenario_rates: ScenarioRatesDto,
    pub value: Decimal,
    pub provenance: ProvenanceDto,
    pub details: AccountDetailsDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountListDto {
    pub accounts: Vec<AccountDto>,
    pub pricing_issues: Vec<PricingIssueDto>,
}

/// Compte courant proposé au rattachement d'une dépense ou d'un revenu ponctuel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountOptionDto {
    pub id: String,
    pub name: String,
}

// --- Saisies ------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewAccountInput {
    pub name: String,
    pub account_type: AccountTypeDto,
    /// `#RRGGBB` ; couleur par défaut du type si absente.
    pub color: Option<String>,
    pub monthly_contribution: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountUpdateInput {
    pub account_id: String,
    pub name: String,
    pub color: String,
    pub monthly_contribution: Decimal,
}

/// Nouveau solde d'un livret (positif ou nul) ou d'un compte courant (signé).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceInput {
    pub account_id: String,
    pub balance: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavingsTermsInput {
    pub account_id: String,
    pub rate_percent: Decimal,
    pub deposit_ceiling: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioRatesInput {
    pub account_id: String,
    pub rates: ScenarioRatesDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashInput {
    pub account_id: String,
    pub cash: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionInput {
    pub account_id: String,
    pub asset: AssetDto,
    pub quantity: Decimal,
    pub unit_cost: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemovePositionInput {
    pub account_id: String,
    pub asset: AssetDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetSearchInput {
    pub source: crate::QuoteSourceDto,
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewFundInput {
    pub account_id: String,
    pub name: String,
    pub units: Decimal,
    pub reference: Option<AssetDto>,
    /// Valeur de part du jour.
    pub nav: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FundUpdateInput {
    pub account_id: String,
    pub fund_id: String,
    pub name: String,
    pub units: Decimal,
    pub reference: Option<AssetDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FundNavInput {
    pub account_id: String,
    pub fund_id: String,
    pub nav: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FundRefInput {
    pub account_id: String,
    pub fund_id: String,
}
