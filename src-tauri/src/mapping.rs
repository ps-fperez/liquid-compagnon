//! Conversions domaine ↔ DTO (couche présentation). Les saisies de l'UI sont validées ici
//! (identifiants, mois) puis par le domaine.

use std::collections::HashMap;

use lc_application::budget::{BudgetSummary, ExpenseList};
use lc_application::dashboard::{Dashboard, Variation};
use lc_application::pricing::PricingIssue;
use lc_application::projection::ProjectionReport;
use lc_domain::account::{
    Account, AccountKind, AccountType, AssetRef, CurrencyCode, QuoteSource, Scenario, ScenarioRates,
};
use lc_domain::budget::{
    Category, Expense, OneOffIncome, Periodicity, RecurringCharge, SalaryEntry,
};
use lc_domain::valuation::{
    AccountValuation, FundValuation, NavKind, PositionValuation, Provenance,
};
use lc_domain::{AccountId, Decimal, Month, Rate, Uuid};
use lc_shared_dto::*;

use crate::error::{CommandResult, app_error, invalid};

// --- Saisies -> domaine -------------------------------------------------------------------

pub fn id<T>(raw: &str, make: fn(Uuid) -> T) -> CommandResult<T> {
    Uuid::parse_str(raw.trim())
        .map(make)
        .map_err(|_| invalid("Identifiant invalide"))
}

pub fn optional_id<T>(raw: Option<&str>, make: fn(Uuid) -> T) -> CommandResult<Option<T>> {
    raw.map(|raw| id(raw, make)).transpose()
}

pub fn month(raw: &str) -> CommandResult<Month> {
    raw.parse()
        .map_err(|_| invalid("Mois invalide (format attendu : AAAA-MM)"))
}

pub fn rate(percent: Decimal) -> CommandResult<Rate> {
    Rate::from_percent(percent).map_err(|error| app_error(error.into()))
}

pub fn scenario_rates(dto: ScenarioRatesDto) -> CommandResult<ScenarioRates> {
    Ok(ScenarioRates {
        pessimistic: rate(dto.pessimistic)?,
        neutral: rate(dto.neutral)?,
        optimistic: rate(dto.optimistic)?,
    })
}

pub fn account_type(dto: AccountTypeDto) -> AccountType {
    match dto {
        AccountTypeDto::LivretA => AccountType::LivretA,
        AccountTypeDto::Ldds => AccountType::Ldds,
        AccountTypeDto::Pea => AccountType::Pea,
        AccountTypeDto::Pei => AccountType::Pei,
        AccountTypeDto::Crypto => AccountType::Crypto,
    }
}

pub fn quote_source(dto: QuoteSourceDto) -> QuoteSource {
    match dto {
        QuoteSourceDto::CoinGecko => QuoteSource::CoinGecko,
        QuoteSourceDto::TwelveData => QuoteSource::TwelveData,
    }
}

pub fn asset(dto: &AssetDto) -> CommandResult<AssetRef> {
    let currency = CurrencyCode::new(&dto.currency).map_err(|error| app_error(error.into()))?;
    AssetRef::new(
        quote_source(dto.source),
        &dto.symbol,
        &dto.display_name,
        currency,
    )
    .map_err(|error| app_error(error.into()))
}

pub fn periodicity(dto: PeriodicityDto) -> Periodicity {
    match dto {
        PeriodicityDto::Monthly => Periodicity::Monthly,
        PeriodicityDto::Quarterly => Periodicity::Quarterly,
        PeriodicityDto::Yearly => Periodicity::Yearly,
    }
}

// --- Domaine -> DTO -----------------------------------------------------------------------

pub fn account_type_dto(account_type: AccountType) -> AccountTypeDto {
    match account_type {
        AccountType::LivretA => AccountTypeDto::LivretA,
        AccountType::Ldds => AccountTypeDto::Ldds,
        AccountType::Pea => AccountTypeDto::Pea,
        AccountType::Pei => AccountTypeDto::Pei,
        AccountType::Crypto => AccountTypeDto::Crypto,
    }
}

pub fn asset_dto(asset: &AssetRef) -> AssetDto {
    AssetDto {
        source: match asset.source() {
            QuoteSource::CoinGecko => QuoteSourceDto::CoinGecko,
            QuoteSource::TwelveData => QuoteSourceDto::TwelveData,
        },
        symbol: asset.symbol().to_string(),
        display_name: asset.display_name().to_string(),
        currency: asset.currency().to_string(),
    }
}

pub fn provenance_dto(provenance: Provenance) -> ProvenanceDto {
    match provenance {
        Provenance::Live => ProvenanceDto::Live,
        Provenance::Manual => ProvenanceDto::Manual,
        Provenance::Cached { as_of } => ProvenanceDto::Cached { as_of },
        Provenance::Estimated => ProvenanceDto::Estimated,
        Provenance::CostBasis => ProvenanceDto::CostBasis,
        Provenance::Unavailable => ProvenanceDto::Unavailable,
    }
}

pub fn pricing_issues_dto(issues: &[PricingIssue]) -> Vec<PricingIssueDto> {
    issues
        .iter()
        .map(|issue| {
            let (source, kind) = match issue {
                PricingIssue::RateLimited(source) => (source, PricingIssueKindDto::RateLimited),
                PricingIssue::MissingApiKey(source) => (source, PricingIssueKindDto::MissingApiKey),
                PricingIssue::Unavailable(source) => (source, PricingIssueKindDto::Unavailable),
            };
            PricingIssueDto {
                source: match source {
                    QuoteSource::CoinGecko => QuoteSourceDto::CoinGecko,
                    QuoteSource::TwelveData => QuoteSourceDto::TwelveData,
                },
                kind,
            }
        })
        .collect()
}

fn variation_dto(variation: Variation) -> VariationDto {
    VariationDto {
        amount: variation.amount.amount(),
        percent: variation.percent,
    }
}

fn scenario_rates_dto(rates: &ScenarioRates) -> ScenarioRatesDto {
    ScenarioRatesDto {
        pessimistic: rates.pessimistic.as_percent(),
        neutral: rates.neutral.as_percent(),
        optimistic: rates.optimistic.as_percent(),
    }
}

pub fn budget_summary_dto(summary: &BudgetSummary) -> BudgetSummaryDto {
    BudgetSummaryDto {
        month: summary.month.to_string(),
        salary: summary.salary.amount(),
        one_off_incomes: summary.one_off_incomes.amount(),
        total_income: summary.total_income.amount(),
        fixed_charges: summary.fixed_charges.amount(),
        expenses: summary.expenses.amount(),
        average_expenses: summary.average_expenses.amount(),
        savings_capacity: summary.savings_capacity.amount(),
    }
}

pub fn dashboard_dto(dashboard: &Dashboard, export_reminder_due: bool) -> DashboardDto {
    DashboardDto {
        total: dashboard.total.amount(),
        variation: dashboard.variation.map(variation_dto),
        accounts: dashboard
            .accounts
            .iter()
            .map(|line| AccountLineDto {
                id: line.account_id.to_string(),
                name: line.name.to_string(),
                color: line.color.to_string(),
                account_type: account_type_dto(line.account_type),
                value: line.value.amount(),
                share_percent: line.share_percent,
                variation: line.variation.map(variation_dto),
                provenance: provenance_dto(line.valuation.provenance),
            })
            .collect(),
        budget: budget_summary_dto(&dashboard.budget),
        history: dashboard
            .history
            .iter()
            .map(|point| HistoryPointDto {
                month: point.month.to_string(),
                total: point.total.amount(),
            })
            .collect(),
        pricing_issues: pricing_issues_dto(&dashboard.pricing_issues),
        export_reminder_due,
    }
}

fn position_dto(position: &PositionValuation) -> PositionDto {
    PositionDto {
        asset: asset_dto(&position.asset),
        quantity: position.quantity,
        unit_price: position.unit_price.map(|p| p.amount()),
        value: position.value.amount(),
        provenance: provenance_dto(position.provenance),
        cost_basis: position.cost_basis.map(|c| c.amount()),
        unrealized_gain: position.unrealized_gain.map(|g| g.amount()),
        unrealized_gain_percent: position.unrealized_gain_percent,
    }
}

fn fund_dto(account: &Account, fund: &FundValuation) -> FundDto {
    let stored = match account.kind() {
        AccountKind::EmployeeSavings(details) => {
            details.funds().iter().find(|f| f.id() == fund.fund_id)
        }
        _ => None,
    };
    FundDto {
        id: fund.fund_id.to_string(),
        name: fund.name.to_string(),
        units: fund.units,
        reference: stored.and_then(|f| f.reference()).map(asset_dto),
        nav: fund.nav.amount(),
        nav_kind: match fund.nav_kind {
            NavKind::Real => NavKindDto::Real,
            NavKind::Estimated => NavKindDto::Estimated,
        },
        nav_date: stored.map_or_else(Default::default, |f| f.anchor().date()),
        value: fund.value.amount(),
    }
}

pub fn account_dto(account: &Account, valuation: &AccountValuation) -> AccountDto {
    let details = match account.kind() {
        AccountKind::Savings(d) => AccountDetailsDto::Savings {
            balance: d.balance().amount(),
            rate_percent: d.rate().as_percent(),
            deposit_ceiling: d.deposit_ceiling().amount(),
            balance_updated_on: d.balance_updated_on(),
            ceiling_reached: d.is_ceiling_reached(),
        },
        AccountKind::Securities(d) => AccountDetailsDto::Securities {
            cash: d.cash().amount(),
            positions: valuation.positions.iter().map(position_dto).collect(),
        },
        AccountKind::EmployeeSavings(_) => AccountDetailsDto::EmployeeSavings {
            funds: valuation
                .funds
                .iter()
                .map(|fund| fund_dto(account, fund))
                .collect(),
        },
    };
    AccountDto {
        id: account.id().to_string(),
        name: account.name().to_string(),
        color: account.color().to_string(),
        account_type: account_type_dto(account.account_type()),
        archived: account.is_archived(),
        monthly_contribution: account.monthly_contribution().amount(),
        scenario_rates: scenario_rates_dto(account.scenario_rates()),
        value: valuation.value.amount(),
        provenance: provenance_dto(valuation.provenance),
        details,
    }
}

pub fn category_dto(category: &Category) -> CategoryDto {
    CategoryDto {
        id: category.id().to_string(),
        name: category.name().to_string(),
        archived: category.is_archived(),
    }
}

pub fn salary_dto(salary: &SalaryEntry) -> SalaryDto {
    SalaryDto {
        id: salary.id().to_string(),
        effective_from: salary.effective_from().to_string(),
        net_amount: salary.net_amount().amount(),
    }
}

pub fn one_off_income_dto(income: &OneOffIncome) -> OneOffIncomeDto {
    OneOffIncomeDto {
        id: income.id().to_string(),
        label: income.label().to_string(),
        amount: income.amount().amount(),
        date: income.date(),
    }
}

pub fn charge_dto(charge: &RecurringCharge) -> ChargeDto {
    ChargeDto {
        id: charge.id().to_string(),
        label: charge.label().to_string(),
        amount: charge.amount().amount(),
        monthly_equivalent: charge.monthly_equivalent().amount(),
        category_id: charge.category().to_string(),
        periodicity: match charge.periodicity() {
            Periodicity::Monthly => PeriodicityDto::Monthly,
            Periodicity::Quarterly => PeriodicityDto::Quarterly,
            Periodicity::Yearly => PeriodicityDto::Yearly,
        },
        start: charge.start().to_string(),
        end: charge.end().map(|end| end.to_string()),
    }
}

pub fn expense_dto(expense: &Expense) -> ExpenseDto {
    ExpenseDto {
        id: expense.id().to_string(),
        amount: expense.amount().amount(),
        category_id: expense.category().to_string(),
        date: expense.date(),
        label: expense.label().map(|label| label.to_string()),
    }
}

pub fn expense_list_dto(list: &ExpenseList) -> ExpenseListDto {
    ExpenseListDto {
        month: list.month.to_string(),
        category_id: list.category.map(|id| id.to_string()),
        items: list.items.iter().map(expense_dto).collect(),
        total: list.total.amount(),
    }
}

/// `accounts` fournit le nom et la couleur des trajectoires.
pub fn projection_dto(report: &ProjectionReport, accounts: &[Account]) -> ProjectionDto {
    let names: HashMap<AccountId, (&str, &str)> = accounts
        .iter()
        .map(|a| (a.id(), (a.name().as_str(), a.color().as_str())))
        .collect();
    ProjectionDto {
        scenarios: report
            .scenarios
            .iter()
            .map(|projection| ScenarioProjectionDto {
                scenario: match projection.scenario {
                    Scenario::Pessimistic => ScenarioDto::Pessimistic,
                    Scenario::Neutral => ScenarioDto::Neutral,
                    Scenario::Optimistic => ScenarioDto::Optimistic,
                },
                totals: projection.totals.iter().map(|m| m.amount()).collect(),
                unallocated: projection.unallocated.iter().map(|m| m.amount()).collect(),
                accounts: projection
                    .accounts
                    .iter()
                    .map(|trajectory| {
                        let (name, color) = names
                            .get(&trajectory.account_id)
                            .copied()
                            .unwrap_or(("Compte", "#A855F7"));
                        AccountTrajectoryDto {
                            account_id: trajectory.account_id.to_string(),
                            name: name.to_string(),
                            color: color.to_string(),
                            values: trajectory.values.iter().map(|m| m.amount()).collect(),
                        }
                    })
                    .collect(),
            })
            .collect(),
        savings_capacity: report.savings_capacity.amount(),
        total_contributions: report.total_contributions.amount(),
        contributions_excess: report.contributions_excess.map(|m| m.amount()),
        pricing_issues: pricing_issues_dto(&report.pricing_issues),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc_domain::account::Position;
    use lc_domain::valuation::{PriceBook, Quote, QuoteFreshness, value_account};
    use lc_domain::{CategoryId, Label, Money, NaiveDate, defaults};
    use lc_shared_dto::ErrorCode;
    use rust_decimal_macros::dec;

    #[test]
    fn rejects_malformed_ids_and_months() {
        assert_eq!(
            id("pas-un-uuid", CategoryId::from_uuid).unwrap_err().code,
            ErrorCode::Invalid
        );
        assert!(id(&Uuid::new_v4().to_string(), CategoryId::from_uuid).is_ok());
        assert_eq!(month("2026-13").unwrap_err().code, ErrorCode::Invalid);
        assert_eq!(month("2026-10").unwrap(), Month::new(2026, 10).unwrap());
        assert_eq!(rate(dec!(-100)).unwrap_err().code, ErrorCode::Invalid);
    }

    #[test]
    fn asset_roundtrip() {
        let dto = AssetDto {
            source: QuoteSourceDto::TwelveData,
            symbol: "cw8:Euronext".into(),
            display_name: "Amundi MSCI World".into(),
            currency: "eur".into(),
        };
        let asset = asset(&dto).unwrap();
        assert_eq!(asset.symbol(), "CW8:Euronext");
        assert_eq!(asset_dto(&asset).currency, "EUR");
    }

    #[test]
    fn crypto_account_dto_carries_positions_and_provenance() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        let mut wallet = Account::open(
            AccountId::generate(),
            Label::new("Crypto").unwrap(),
            defaults::color(AccountType::Crypto),
            AccountType::Crypto,
            today,
        );
        let btc = AssetRef::new(
            QuoteSource::CoinGecko,
            "bitcoin",
            "Bitcoin",
            CurrencyCode::eur(),
        )
        .unwrap();
        wallet
            .upsert_position(
                Position::new(btc.clone(), dec!(0.15), Some(Money::new(dec!(40000)))).unwrap(),
            )
            .unwrap();
        let now = today.and_hms_opt(9, 42, 0).unwrap().and_utc();
        let prices = PriceBook::new([Quote::new(
            btc,
            Money::new(dec!(60000)),
            now,
            QuoteFreshness::Live,
        )]);

        let dto = account_dto(&wallet, &value_account(&wallet, &prices));

        assert_eq!(dto.value, dec!(9000));
        assert_eq!(dto.provenance, ProvenanceDto::Live);
        assert_eq!(dto.scenario_rates.pessimistic, dec!(-40));
        let AccountDetailsDto::Securities { positions, .. } = dto.details else {
            panic!("compte-titres attendu")
        };
        assert_eq!(positions[0].unrealized_gain, Some(dec!(3000)));
    }
}
