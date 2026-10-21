//! Conversions domaine ↔ DTO (couche présentation). Les saisies de l'UI sont validées ici
//! (identifiants, mois) puis par le domaine.

use std::collections::HashMap;

use lc_application::budget::{BudgetSummary, ExpenseList};
use lc_application::dashboard::{Dashboard, Variation};
use lc_application::goals::GoalProgress;
use lc_application::pricing::PricingIssue;
use lc_application::projection::ProjectionReport;
use lc_application::review::{MonthReport, StartPrompt};
use lc_domain::account::{
    Account, AccountKind, AccountType, AssetRef, CurrencyCode, QuoteSource, Scenario, ScenarioRates,
};
use lc_domain::budget::{
    Category, Expense, OneOffIncome, Pace, Period, Periodicity, RecurringCharge, SalaryEntry,
    SpendingAllowance,
};
use lc_domain::goal::GoalStatus;
use lc_domain::ledger::{Movement, MovementKind};
use lc_domain::transfer::{Endpoint, ExternalAccount, Schedule, Transfer};
use lc_domain::valuation::{
    AccountValuation, FundValuation, NavKind, PositionValuation, Provenance,
};
use lc_domain::{AccountId, CategoryId, Decimal, ExternalId, Money, Month, Rate, Uuid};
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
        AccountTypeDto::Checking => AccountType::Checking,
        AccountTypeDto::Livret => AccountType::Livret,
        AccountTypeDto::AssuranceVie => AccountType::AssuranceVie,
    }
}

pub fn quote_source(dto: QuoteSourceDto) -> QuoteSource {
    match dto {
        QuoteSourceDto::CoinGecko => QuoteSource::CoinGecko,
        QuoteSourceDto::TwelveData => QuoteSource::TwelveData,
        QuoteSourceDto::Yahoo => QuoteSource::Yahoo,
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
        AccountType::Checking => AccountTypeDto::Checking,
        AccountType::Livret => AccountTypeDto::Livret,
        AccountType::AssuranceVie => AccountTypeDto::AssuranceVie,
    }
}

pub fn quote_source_dto(source: QuoteSource) -> QuoteSourceDto {
    match source {
        QuoteSource::CoinGecko => QuoteSourceDto::CoinGecko,
        QuoteSource::TwelveData => QuoteSourceDto::TwelveData,
        QuoteSource::Yahoo => QuoteSourceDto::Yahoo,
    }
}

pub fn asset_dto(asset: &AssetRef) -> AssetDto {
    AssetDto {
        source: quote_source_dto(asset.source()),
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
            let (source, kind, symbols) = match issue {
                PricingIssue::RateLimited(source) => {
                    (source, PricingIssueKindDto::RateLimited, Vec::new())
                }
                PricingIssue::MissingApiKey(source) => {
                    (source, PricingIssueKindDto::MissingApiKey, Vec::new())
                }
                PricingIssue::Unavailable(source) => {
                    (source, PricingIssueKindDto::Unavailable, Vec::new())
                }
                PricingIssue::NotCovered { source, symbols } => {
                    (source, PricingIssueKindDto::NotCovered, symbols.clone())
                }
            };
            PricingIssueDto {
                source: quote_source_dto(*source),
                kind,
                symbols,
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

pub fn budget_summary_dto(summary: &BudgetSummary, categories: &[Category]) -> BudgetSummaryDto {
    BudgetSummaryDto {
        month: summary.month.to_string(),
        salary: summary.salary.amount(),
        one_off_incomes: summary.one_off_incomes.amount(),
        total_income: summary.total_income.amount(),
        fixed_charges: summary.fixed_charges.amount(),
        expenses: summary.expenses.amount(),
        average_expenses: summary.average_expenses.amount(),
        savings_capacity: summary.savings_capacity.amount(),
        external_out: summary.external_out.amount(),
        external_in: summary.external_in.amount(),
        allowance: allowance_dto(&summary.allowance),
        top_categories: summary
            .top_categories
            .iter()
            .map(|(id, amount)| CategoryAmountDto {
                id: id.to_string(),
                name: categories
                    .iter()
                    .find(|c| c.id() == *id)
                    .map_or_else(String::new, |c| c.name().to_string()),
                amount: amount.amount(),
            })
            .collect(),
    }
}

fn allowance_dto(allowance: &SpendingAllowance) -> AllowanceDto {
    let s = &allowance.sources;
    AllowanceDto {
        income: s.income.amount(),
        external_in: s.external_in.amount(),
        repatriated: s.repatriated.amount(),
        fixed_charges: s.fixed_charges.amount(),
        scheduled_savings: s.scheduled_savings.amount(),
        external_out: s.external_out.amount(),
        spent: s.spent.amount(),
        envelope: allowance.envelope.amount(),
        remaining: allowance.remaining.amount(),
        today: match allowance.period {
            Period::Current { day } => Some(day),
            Period::Past | Period::Future => None,
        },
        days_in_month: allowance.days_in_month,
        days_left: allowance.days_left,
        per_day: allowance.per_day.map(|m| m.amount()),
        daily_spending: allowance.daily_spending.map(|m| m.amount()),
        projected_end: allowance.projected_end.map(|m| m.amount()),
        pace: match allowance.pace {
            Pace::OnTrack => PaceDto::OnTrack,
            Pace::Fast => PaceDto::Fast,
            Pace::Exceeded => PaceDto::Exceeded,
        },
    }
}

pub fn dashboard_dto(
    dashboard: &Dashboard,
    categories: &[Category],
    export_reminder_due: bool,
) -> DashboardDto {
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
        budget: budget_summary_dto(&dashboard.budget, categories),
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

/// Noms des comptes suivis et hors patrimoine, pour afficher le compte d'en face d'un virement.
#[derive(Debug, Default)]
pub struct Names(HashMap<Endpoint, String>);

impl Names {
    pub fn new(accounts: &[Account], externals: &[ExternalAccount]) -> Self {
        let mut names = HashMap::new();
        for account in accounts {
            names.insert(Endpoint::Account(account.id()), account.name().to_string());
        }
        for external in externals {
            names.insert(
                Endpoint::External(external.id()),
                external.name().to_string(),
            );
        }
        Self(names)
    }

    pub fn of(&self, endpoint: Endpoint) -> String {
        self.0
            .get(&endpoint)
            .cloned()
            .unwrap_or_else(|| "Compte supprimé".into())
    }
}

pub fn movement_dto(movement: &Movement, names: &Names) -> MovementDto {
    MovementDto {
        kind: match movement.kind {
            MovementKind::Expense => MovementKindDto::Expense,
            MovementKind::Income => MovementKindDto::Income,
            MovementKind::Charge => MovementKindDto::Charge,
            MovementKind::TransferIn => MovementKindDto::TransferIn,
            MovementKind::TransferOut => MovementKindDto::TransferOut,
        },
        amount: movement.signed_amount().amount(),
        date: movement.date,
        label: movement.label.clone(),
        counterpart: movement.counterpart.map(|endpoint| names.of(endpoint)),
    }
}

pub fn account_dto(account: &Account, valuation: &AccountValuation, names: &Names) -> AccountDto {
    let movements = || {
        valuation
            .movements
            .iter()
            .map(|m| movement_dto(m, names))
            .collect()
    };
    let details = match account.kind() {
        AccountKind::Savings(d) => AccountDetailsDto::Savings {
            balance: valuation.value.amount(),
            reference_balance: d.balance().amount(),
            rate_percent: d.rate().as_percent(),
            deposit_ceiling: d.deposit_ceiling().map(|c| c.amount()),
            balance_updated_on: d.balance_updated_on(),
            ceiling_reached: d.is_ceiling_reached(valuation.value),
            movements: movements(),
        },
        AccountKind::Securities(d) => AccountDetailsDto::Securities {
            cash: (d.cash()
                + valuation
                    .movements
                    .iter()
                    .map(Movement::signed_amount)
                    .sum::<Money>())
            .amount(),
            positions: valuation.positions.iter().map(position_dto).collect(),
            movements: movements(),
        },
        AccountKind::Checking(d) => AccountDetailsDto::Checking {
            balance: valuation.value.amount(),
            reference_balance: d.reference_balance().amount(),
            reference_on: d.reference_on(),
            movements: movements(),
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
        scenario_rates: scenario_rates_dto(account.scenario_rates()),
        value: valuation.value.amount(),
        provenance: provenance_dto(valuation.provenance),
        details,
    }
}

pub fn endpoint(dto: &EndpointDto) -> CommandResult<Endpoint> {
    Ok(match dto {
        EndpointDto::Account(raw) => Endpoint::Account(id(raw, AccountId::from_uuid)?),
        EndpointDto::External(raw) => Endpoint::External(id(raw, ExternalId::from_uuid)?),
    })
}

pub fn endpoint_dto(endpoint: Endpoint) -> EndpointDto {
    match endpoint {
        Endpoint::Account(id) => EndpointDto::Account(id.to_string()),
        Endpoint::External(id) => EndpointDto::External(id.to_string()),
    }
}

pub fn schedule(dto: &ScheduleDto) -> CommandResult<Schedule> {
    Ok(match dto {
        ScheduleDto::Once { date } => Schedule::Once { date: *date },
        ScheduleDto::Monthly { day, from, until } => Schedule::Monthly {
            day: *day,
            from: month(from)?,
            until: until.as_deref().map(month).transpose()?,
        },
        ScheduleDto::Yearly {
            month,
            day,
            from_year,
            until_year,
        } => Schedule::Yearly {
            month: *month,
            day: *day,
            from_year: *from_year,
            until_year: *until_year,
        },
    })
}

pub fn transfer_dto(transfer: &Transfer, names: &Names) -> TransferDto {
    TransferDto {
        id: transfer.id().to_string(),
        from: endpoint_dto(transfer.from()),
        to: endpoint_dto(transfer.to()),
        from_name: names.of(transfer.from()),
        to_name: names.of(transfer.to()),
        sent: transfer.sent().amount(),
        received: transfer.received().amount(),
        label: transfer.label().map(ToString::to_string),
        schedule: match transfer.schedule() {
            Schedule::Once { date } => ScheduleDto::Once { date },
            Schedule::Monthly { day, from, until } => ScheduleDto::Monthly {
                day,
                from: from.to_string(),
                until: until.map(|m| m.to_string()),
            },
            Schedule::Yearly {
                month,
                day,
                from_year,
                until_year,
            } => ScheduleDto::Yearly {
                month,
                day,
                from_year,
                until_year,
            },
        },
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
        account_id: income.account().map(|id| id.to_string()),
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
        debit_account_id: charge.debit().map(|debit| debit.account.to_string()),
        debit_day: charge.debit().map(|debit| debit.day),
    }
}

pub fn expense_dto(expense: &Expense) -> ExpenseDto {
    ExpenseDto {
        id: expense.id().to_string(),
        amount: expense.amount().amount(),
        category_id: expense.category().to_string(),
        date: expense.date(),
        label: expense.label().map(|label| label.to_string()),
        account_id: expense.account().map(|id| id.to_string()),
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

pub fn goal_dto(progress: &GoalProgress, accounts: &[Account]) -> GoalDto {
    let goal = &progress.goal;
    GoalDto {
        id: goal.id().to_string(),
        name: goal.name().to_string(),
        target: goal.target().amount(),
        deadline: goal.deadline().map(|m| m.to_string()),
        account_ids: goal.accounts().iter().map(ToString::to_string).collect(),
        account_names: goal
            .accounts()
            .iter()
            .filter_map(|id| accounts.iter().find(|a| a.id() == *id))
            .map(|a| a.name().to_string())
            .collect(),
        current: progress.current.amount(),
        percent: progress.percent,
        remaining: progress.remaining.amount(),
        required_monthly: progress.required_monthly.map(|m| m.amount()),
        contributed_monthly: progress.contributed_monthly.amount(),
        arrivals: progress.arrivals.map(|m| m.map(|m| m.to_string())),
        status: match progress.status {
            GoalStatus::Reached => GoalStatusDto::Reached,
            GoalStatus::OnTrack => GoalStatusDto::OnTrack,
            GoalStatus::Late => GoalStatusDto::Late,
            GoalStatus::Open => GoalStatusDto::Open,
        },
        just_reached: progress.just_reached,
    }
}

fn category_name(categories: &[Category], id: CategoryId) -> String {
    categories
        .iter()
        .find(|c| c.id() == id)
        .map_or_else(String::new, |c| c.name().to_string())
}

pub fn month_report_dto(report: &MonthReport, categories: &[Category]) -> MonthReportDto {
    MonthReportDto {
        month: report.month.to_string(),
        saved: report.saved.amount(),
        previous_saved: report.previous_saved.map(|m| m.amount()),
        received: report.received.amount(),
        one_off_incomes: report.one_off_incomes.amount(),
        fixed_charges: report.fixed_charges.amount(),
        expenses: report.expenses.amount(),
        external_out: report.external_out.amount(),
        top_categories: report
            .top_categories
            .iter()
            .map(|(id, amount)| CategoryAmountDto {
                id: id.to_string(),
                name: category_name(categories, *id),
                amount: amount.amount(),
            })
            .collect(),
        biggest_increase: report
            .biggest_increase
            .map(|(id, percent)| (category_name(categories, id), percent)),
        wealth_end: report.wealth_end.map(|m| m.amount()),
        wealth_change: report.wealth_change.map(|m| m.amount()),
        wealth_change_percent: report.wealth_change_percent,
        goals: report
            .goals
            .iter()
            .map(|g| GoalMonthDto {
                name: g.name.to_string(),
                percent: g.percent,
                previous_percent: g.previous_percent,
            })
            .collect(),
    }
}

pub fn start_prompt_dto(prompt: &StartPrompt) -> StartPromptDto {
    StartPromptDto {
        account_name: prompt.account_name.to_string(),
        suggested: prompt.suggested.amount(),
        included: prompt
            .included
            .iter()
            .map(|t| IncludedTransferDto {
                counterpart: t.counterpart.clone(),
                amount: t.amount.amount(),
                date: t.date,
            })
            .collect(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use lc_domain::account::Position;
    use lc_domain::ledger::Ledger;
    use lc_domain::valuation::{PriceBook, Quote, QuoteFreshness, value_account};
    use lc_domain::{CategoryId, Label, NaiveDate, defaults};
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

        let dto = account_dto(
            &wallet,
            &value_account(&wallet, &prices, &Ledger::default()),
            &Names::default(),
        );

        assert_eq!(dto.value, dec!(9000));
        assert_eq!(dto.provenance, ProvenanceDto::Live);
        assert_eq!(dto.scenario_rates.pessimistic, dec!(-40));
        let AccountDetailsDto::Securities { positions, .. } = dto.details else {
            panic!("compte-titres attendu")
        };
        assert_eq!(positions[0].unrealized_gain, Some(dec!(3000)));
    }
}
