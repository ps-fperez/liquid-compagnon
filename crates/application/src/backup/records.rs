//! Format d'échange de la sauvegarde (version 1) et conversion depuis/vers le domaine.
//!
//! Les montants et taux sont sérialisés en chaînes décimales exactes, les dates en ISO 8601.
//! La conversion vers le domaine repasse par les constructeurs du domaine : toutes les règles
//! métier sont donc revérifiées à l'import.

use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Utc};
use lc_domain::account::{
    Account, AccountKind, AssetRef, CurrencyCode, EmployeeSavingsDetails, NavAnchor, PeiFund,
    Position, QuoteSource, SavingsDetails, SavingsProduct, ScenarioRates, SecuritiesDetails,
    SecuritiesProduct,
};
use lc_domain::budget::{
    Category, Expense, OneOffIncome, Periodicity, RecurringCharge, SalaryEntry,
};
use lc_domain::{
    AccountId, CategoryId, ChargeId, Color, Decimal, DomainError, ExpenseId, FundId, IncomeId,
    Label, Money, Month, Rate, SalaryId, Snapshot, SnapshotId, SnapshotKind, SnapshotLine,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::SCHEMA_VERSION;
use crate::ports::Dataset;
use crate::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullDataset {
    pub schema_version: u32,
    pub exported_at: DateTime<Utc>,
    pub categories: Vec<CategoryRecord>,
    pub salaries: Vec<SalaryRecord>,
    pub one_off_incomes: Vec<OneOffIncomeRecord>,
    pub charges: Vec<ChargeRecord>,
    pub expenses: Vec<ExpenseRecord>,
    pub accounts: Vec<AccountRecord>,
    pub snapshots: Vec<SnapshotRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryRecord {
    pub id: Uuid,
    pub name: String,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalaryRecord {
    pub id: Uuid,
    /// `AAAA-MM`
    pub effective_from: String,
    pub net_amount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneOffIncomeRecord {
    pub id: Uuid,
    pub label: String,
    pub amount: Decimal,
    pub date: NaiveDate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeriodicityRecord {
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeRecord {
    pub id: Uuid,
    pub label: String,
    pub amount: Decimal,
    pub category_id: Uuid,
    pub periodicity: PeriodicityRecord,
    pub start: String,
    pub end: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpenseRecord {
    pub id: Uuid,
    pub amount: Decimal,
    pub category_id: Uuid,
    pub date: NaiveDate,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioRatesRecord {
    pub pessimistic: Decimal,
    pub neutral: Decimal,
    pub optimistic: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuoteSourceRecord {
    CoinGecko,
    TwelveData,
    Yahoo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRecord {
    pub source: QuoteSourceRecord,
    pub symbol: String,
    pub display_name: String,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionRecord {
    pub asset: AssetRecord,
    pub quantity: Decimal,
    pub unit_cost: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FundRecord {
    pub id: Uuid,
    pub name: String,
    pub units: Decimal,
    pub reference: Option<AssetRecord>,
    pub nav: Decimal,
    pub nav_date: NaiveDate,
    pub reference_price: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavingsProductRecord {
    LivretA,
    Ldds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecuritiesProductRecord {
    Pea,
    Crypto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "nature", rename_all = "snake_case")]
pub enum AccountKindRecord {
    Savings {
        product: SavingsProductRecord,
        balance: Decimal,
        rate: Decimal,
        deposit_ceiling: Decimal,
        balance_updated_on: NaiveDate,
    },
    Securities {
        product: SecuritiesProductRecord,
        cash: Decimal,
        positions: Vec<PositionRecord>,
    },
    EmployeeSavings {
        funds: Vec<FundRecord>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRecord {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub monthly_contribution: Decimal,
    pub scenario_rates: ScenarioRatesRecord,
    pub archived: bool,
    pub kind: AccountKindRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotKindRecord {
    Auto,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotLineRecord {
    pub account_id: Uuid,
    pub value: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub id: Uuid,
    pub month: String,
    pub taken_on: NaiveDate,
    pub kind: SnapshotKindRecord,
    pub lines: Vec<SnapshotLineRecord>,
}

// --- Domaine -> enregistrements ------------------------------------------------------------

impl FullDataset {
    pub fn from_dataset(data: &Dataset, exported_at: DateTime<Utc>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            exported_at,
            categories: data
                .categories
                .iter()
                .map(|c| CategoryRecord {
                    id: *c.id().as_uuid(),
                    name: c.name().to_string(),
                    archived: c.is_archived(),
                })
                .collect(),
            salaries: data
                .salaries
                .iter()
                .map(|s| SalaryRecord {
                    id: *s.id().as_uuid(),
                    effective_from: s.effective_from().to_string(),
                    net_amount: s.net_amount().amount(),
                })
                .collect(),
            one_off_incomes: data
                .one_off_incomes
                .iter()
                .map(|i| OneOffIncomeRecord {
                    id: *i.id().as_uuid(),
                    label: i.label().to_string(),
                    amount: i.amount().amount(),
                    date: i.date(),
                })
                .collect(),
            charges: data
                .charges
                .iter()
                .map(|c| ChargeRecord {
                    id: *c.id().as_uuid(),
                    label: c.label().to_string(),
                    amount: c.amount().amount(),
                    category_id: *c.category().as_uuid(),
                    periodicity: match c.periodicity() {
                        Periodicity::Monthly => PeriodicityRecord::Monthly,
                        Periodicity::Quarterly => PeriodicityRecord::Quarterly,
                        Periodicity::Yearly => PeriodicityRecord::Yearly,
                    },
                    start: c.start().to_string(),
                    end: c.end().map(|m| m.to_string()),
                })
                .collect(),
            expenses: data
                .expenses
                .iter()
                .map(|e| ExpenseRecord {
                    id: *e.id().as_uuid(),
                    amount: e.amount().amount(),
                    category_id: *e.category().as_uuid(),
                    date: e.date(),
                    label: e.label().map(|l| l.to_string()),
                })
                .collect(),
            accounts: data.accounts.iter().map(account_record).collect(),
            snapshots: data
                .snapshots
                .iter()
                .map(|s| SnapshotRecord {
                    id: *s.id().as_uuid(),
                    month: s.month().to_string(),
                    taken_on: s.taken_on(),
                    kind: match s.kind() {
                        SnapshotKind::Auto => SnapshotKindRecord::Auto,
                        SnapshotKind::Manual => SnapshotKindRecord::Manual,
                    },
                    lines: s
                        .lines()
                        .iter()
                        .map(|l| SnapshotLineRecord {
                            account_id: *l.account.as_uuid(),
                            value: l.value.amount(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

fn account_record(account: &Account) -> AccountRecord {
    let rates = account.scenario_rates();
    AccountRecord {
        id: *account.id().as_uuid(),
        name: account.name().to_string(),
        color: account.color().to_string(),
        monthly_contribution: account.monthly_contribution().amount(),
        scenario_rates: ScenarioRatesRecord {
            pessimistic: rates.pessimistic.as_fraction(),
            neutral: rates.neutral.as_fraction(),
            optimistic: rates.optimistic.as_fraction(),
        },
        archived: account.is_archived(),
        kind: match account.kind() {
            AccountKind::Savings(d) => AccountKindRecord::Savings {
                product: match d.product() {
                    SavingsProduct::LivretA => SavingsProductRecord::LivretA,
                    SavingsProduct::Ldds => SavingsProductRecord::Ldds,
                },
                balance: d.balance().amount(),
                rate: d.rate().as_fraction(),
                deposit_ceiling: d.deposit_ceiling().amount(),
                balance_updated_on: d.balance_updated_on(),
            },
            AccountKind::Securities(d) => AccountKindRecord::Securities {
                product: match d.product() {
                    SecuritiesProduct::Pea => SecuritiesProductRecord::Pea,
                    SecuritiesProduct::Crypto => SecuritiesProductRecord::Crypto,
                },
                cash: d.cash().amount(),
                positions: d
                    .positions()
                    .iter()
                    .map(|p| PositionRecord {
                        asset: asset_record(p.asset()),
                        quantity: p.quantity(),
                        unit_cost: p.unit_cost().map(|c| c.amount()),
                    })
                    .collect(),
            },
            AccountKind::EmployeeSavings(d) => AccountKindRecord::EmployeeSavings {
                funds: d
                    .funds()
                    .iter()
                    .map(|f| FundRecord {
                        id: *f.id().as_uuid(),
                        name: f.name().to_string(),
                        units: f.units(),
                        reference: f.reference().map(asset_record),
                        nav: f.anchor().nav().amount(),
                        nav_date: f.anchor().date(),
                        reference_price: f.anchor().reference_price().map(|p| p.amount()),
                    })
                    .collect(),
            },
        },
    }
}

fn asset_record(asset: &AssetRef) -> AssetRecord {
    AssetRecord {
        source: match asset.source() {
            QuoteSource::CoinGecko => QuoteSourceRecord::CoinGecko,
            QuoteSource::TwelveData => QuoteSourceRecord::TwelveData,
            QuoteSource::Yahoo => QuoteSourceRecord::Yahoo,
        },
        symbol: asset.symbol().to_string(),
        display_name: asset.display_name().to_string(),
        currency: asset.currency().to_string(),
    }
}

// --- Enregistrements -> domaine (avec validation) ------------------------------------------

impl FullDataset {
    /// Convertit et valide l'ensemble des données. La première incohérence rencontrée est
    /// renvoyée avec son contexte ; rien n'est écrit tant que tout n'est pas valide.
    pub fn into_dataset(self) -> AppResult<Dataset> {
        let categories = self
            .categories
            .into_iter()
            .map(|r| {
                Ok(Category::restore(
                    CategoryId::from_uuid(r.id),
                    Label::new(&r.name)?,
                    r.archived,
                ))
            })
            .collect::<Result<Vec<_>, DomainError>>()
            .map_err(context("catégorie"))?;
        let category_ids: HashSet<CategoryId> = categories.iter().map(Category::id).collect();
        let known_category = |id: Uuid| {
            let id = CategoryId::from_uuid(id);
            if category_ids.contains(&id) {
                Ok(id)
            } else {
                Err(invalid("référence de catégorie inconnue"))
            }
        };

        let salaries = self
            .salaries
            .into_iter()
            .map(|r| {
                SalaryEntry::new(
                    SalaryId::from_uuid(r.id),
                    r.effective_from.parse()?,
                    Money::new(r.net_amount),
                )
            })
            .collect::<Result<Vec<_>, DomainError>>()
            .map_err(context("salaire"))?;

        let one_off_incomes = self
            .one_off_incomes
            .into_iter()
            .map(|r| {
                OneOffIncome::new(
                    IncomeId::from_uuid(r.id),
                    Label::new(&r.label)?,
                    Money::new(r.amount),
                    r.date,
                )
            })
            .collect::<Result<Vec<_>, DomainError>>()
            .map_err(context("revenu ponctuel"))?;

        let mut charges = Vec::with_capacity(self.charges.len());
        for r in self.charges {
            let category = known_category(r.category_id)?;
            charges.push(charge_from_record(r, category).map_err(context("charge fixe"))?);
        }

        let mut expenses = Vec::with_capacity(self.expenses.len());
        for r in self.expenses {
            let category = known_category(r.category_id)?;
            let label = r
                .label
                .as_deref()
                .map(Label::new)
                .transpose()
                .map_err(context("dépense"))?;
            let expense = Expense::new(
                ExpenseId::from_uuid(r.id),
                Money::new(r.amount),
                category,
                r.date,
                label,
            )
            .map_err(context("dépense"))?;
            expenses.push(expense);
        }

        let accounts = self
            .accounts
            .into_iter()
            .map(account_from_record)
            .collect::<Result<Vec<_>, DomainError>>()
            .map_err(context("compte"))?;
        let account_ids: HashSet<AccountId> = accounts.iter().map(Account::id).collect();

        let mut snapshots = Vec::with_capacity(self.snapshots.len());
        for r in self.snapshots {
            let month: Month = r.month.parse().map_err(context("snapshot"))?;
            let mut lines = Vec::with_capacity(r.lines.len());
            for line in r.lines {
                let account = AccountId::from_uuid(line.account_id);
                if !account_ids.contains(&account) {
                    return Err(invalid("snapshot : référence de compte inconnue"));
                }
                lines.push(SnapshotLine {
                    account,
                    value: Money::new(line.value),
                });
            }
            let kind = match r.kind {
                SnapshotKindRecord::Auto => SnapshotKind::Auto,
                SnapshotKindRecord::Manual => SnapshotKind::Manual,
            };
            snapshots.push(Snapshot::restore(
                SnapshotId::from_uuid(r.id),
                month,
                r.taken_on,
                kind,
                lines,
            ));
        }

        Ok(Dataset {
            categories,
            salaries,
            one_off_incomes,
            charges,
            expenses,
            accounts,
            snapshots,
        })
    }
}

fn charge_from_record(
    r: ChargeRecord,
    category: CategoryId,
) -> Result<RecurringCharge, DomainError> {
    RecurringCharge::new(
        ChargeId::from_uuid(r.id),
        Label::new(&r.label)?,
        Money::new(r.amount),
        category,
        match r.periodicity {
            PeriodicityRecord::Monthly => Periodicity::Monthly,
            PeriodicityRecord::Quarterly => Periodicity::Quarterly,
            PeriodicityRecord::Yearly => Periodicity::Yearly,
        },
        r.start.parse()?,
        r.end.as_deref().map(str::parse::<Month>).transpose()?,
    )
}

fn account_from_record(r: AccountRecord) -> Result<Account, DomainError> {
    let kind = match r.kind {
        AccountKindRecord::Savings {
            product,
            balance,
            rate,
            deposit_ceiling,
            balance_updated_on,
        } => AccountKind::Savings(SavingsDetails::new(
            match product {
                SavingsProductRecord::LivretA => SavingsProduct::LivretA,
                SavingsProductRecord::Ldds => SavingsProduct::Ldds,
            },
            Money::new(balance),
            Rate::from_fraction(rate)?,
            Money::new(deposit_ceiling),
            balance_updated_on,
        )?),
        AccountKindRecord::Securities {
            product,
            cash,
            positions,
        } => AccountKind::Securities(SecuritiesDetails::new(
            match product {
                SecuritiesProductRecord::Pea => SecuritiesProduct::Pea,
                SecuritiesProductRecord::Crypto => SecuritiesProduct::Crypto,
            },
            Money::new(cash),
            positions
                .into_iter()
                .map(|p| {
                    Position::new(
                        asset_from_record(p.asset)?,
                        p.quantity,
                        p.unit_cost.map(Money::new),
                    )
                })
                .collect::<Result<_, _>>()?,
        )?),
        AccountKindRecord::EmployeeSavings { funds } => {
            AccountKind::EmployeeSavings(EmployeeSavingsDetails::new(
                funds
                    .into_iter()
                    .map(|f| {
                        PeiFund::new(
                            FundId::from_uuid(f.id),
                            Label::new(&f.name)?,
                            f.units,
                            f.reference.map(asset_from_record).transpose()?,
                            NavAnchor::new(
                                Money::new(f.nav),
                                f.nav_date,
                                f.reference_price.map(Money::new),
                            )?,
                        )
                    })
                    .collect::<Result<_, _>>()?,
            ))
        }
    };
    let mut account = Account::restore(
        AccountId::from_uuid(r.id),
        Label::new(&r.name)?,
        Color::new(&r.color)?,
        kind,
        Money::ZERO,
        ScenarioRates {
            pessimistic: Rate::from_fraction(r.scenario_rates.pessimistic)?,
            neutral: Rate::from_fraction(r.scenario_rates.neutral)?,
            optimistic: Rate::from_fraction(r.scenario_rates.optimistic)?,
        },
        r.archived,
    );
    account.set_monthly_contribution(Money::new(r.monthly_contribution))?;
    Ok(account)
}

fn asset_from_record(r: AssetRecord) -> Result<AssetRef, DomainError> {
    let source = match r.source {
        QuoteSourceRecord::CoinGecko => QuoteSource::CoinGecko,
        QuoteSourceRecord::TwelveData => QuoteSource::TwelveData,
        QuoteSourceRecord::Yahoo => QuoteSource::Yahoo,
    };
    AssetRef::new(
        source,
        &r.symbol,
        &r.display_name,
        CurrencyCode::new(&r.currency)?,
    )
}

fn invalid(reason: &str) -> AppError {
    AppError::InvalidBackup(reason.to_string())
}

fn context(what: &'static str) -> impl Fn(DomainError) -> AppError {
    move |error| AppError::InvalidBackup(format!("{what} : {error}"))
}
