//! Format d'échange de la sauvegarde (version 4 : objectifs d'épargne et bilans du mois ;
//! version 3 : virements, comptes hors patrimoine,
//! prélèvements des charges, livrets génériques ; version 2 : compte courant) et conversion depuis/vers le domaine.
//!
//! Les montants et taux sont sérialisés en chaînes décimales exactes, les dates en ISO 8601.
//! La conversion vers le domaine repasse par les constructeurs du domaine : toutes les règles
//! métier sont donc revérifiées à l'import.

use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Utc};
use lc_domain::account::{
    Account, AccountKind, AccountType, AssetRef, CheckingDetails, CurrencyCode,
    EmployeeSavingsDetails, NavAnchor, PeiFund, Position, QuoteSource, SavingsDetails,
    SavingsProduct, ScenarioRates, SecuritiesDetails, SecuritiesProduct,
};
use lc_domain::budget::{
    Category, Debit, Expense, OneOffIncome, Periodicity, RecurringCharge, SalaryEntry,
};
use lc_domain::goal::SavingsGoal;
use lc_domain::ledger::BalanceReference;
use lc_domain::review::MonthlyReview;
use lc_domain::transfer::{Endpoint, ExternalAccount, Schedule, Transfer};
use lc_domain::{
    AccountId, CategoryId, ChargeId, Color, Decimal, DomainError, ExpenseId, ExternalId, FundId,
    GoalId, IncomeId, Label, Money, Month, Rate, SalaryId, Snapshot, SnapshotId, SnapshotKind,
    SnapshotLine, TransferId,
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
    /// Absents avant la version 3.
    #[serde(default)]
    pub transfers: Vec<TransferRecord>,
    #[serde(default)]
    pub external_accounts: Vec<ExternalAccountRecord>,
    /// Absents avant la version 4.
    #[serde(default)]
    pub goals: Vec<GoalRecord>,
    #[serde(default)]
    pub reviews: Vec<ReviewRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalRecord {
    pub id: Uuid,
    pub name: String,
    pub target: Decimal,
    /// `AAAA-MM`
    pub deadline: Option<String>,
    pub accounts: Vec<Uuid>,
    pub reached_on: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRecord {
    /// `AAAA-MM`
    pub month: String,
    pub seen_at: DateTime<Utc>,
    pub next_start_balance: Option<Decimal>,
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
    /// Compte courant crédité (absent en version 1).
    #[serde(default)]
    pub account_id: Option<Uuid>,
    #[serde(default)]
    pub recorded_at: Option<DateTime<Utc>>,
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
    /// Prélèvement sur un compte courant (absent avant la version 3).
    #[serde(default)]
    pub debit: Option<DebitRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebitRecord {
    pub account_id: Uuid,
    pub day: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum EndpointRecord {
    Account(Uuid),
    External(Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScheduleRecord {
    Once {
        date: NaiveDate,
    },
    Monthly {
        day: u8,
        /// `AAAA-MM`
        from: String,
        until: Option<String>,
    },
    Yearly {
        month: u32,
        day: u8,
        from_year: i32,
        until_year: Option<i32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferRecord {
    pub id: Uuid,
    pub from: EndpointRecord,
    pub to: EndpointRecord,
    pub sent: Decimal,
    pub received: Decimal,
    pub label: Option<String>,
    pub schedule: ScheduleRecord,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalAccountRecord {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpenseRecord {
    pub id: Uuid,
    pub amount: Decimal,
    pub category_id: Uuid,
    pub date: NaiveDate,
    pub label: Option<String>,
    /// Compte courant débité (absent en version 1).
    #[serde(default)]
    pub account_id: Option<Uuid>,
    #[serde(default)]
    pub recorded_at: Option<DateTime<Utc>>,
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
    Livret,
    AssuranceVie,
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
        /// Absent pour un livret sans plafond (versions 3 et suivantes).
        deposit_ceiling: Option<Decimal>,
        balance_updated_on: NaiveDate,
        /// Instant du dernier solde saisi (absent avant la version 3 : fin de journée).
        #[serde(default)]
        reference_at: Option<DateTime<Utc>>,
    },
    Securities {
        product: SecuritiesProductRecord,
        cash: Decimal,
        positions: Vec<PositionRecord>,
        /// Date et instant du dernier solde espèces (absents avant la version 3 : date d'export).
        #[serde(default)]
        cash_on: Option<NaiveDate>,
        #[serde(default)]
        cash_at: Option<DateTime<Utc>>,
    },
    EmployeeSavings {
        funds: Vec<FundRecord>,
    },
    Checking {
        reference_balance: Decimal,
        reference_on: NaiveDate,
        reference_at: DateTime<Utc>,
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
                    account_id: i.account().map(|a| *a.as_uuid()),
                    recorded_at: i.recorded_at(),
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
                    debit: c.debit().map(|d| DebitRecord {
                        account_id: *d.account.as_uuid(),
                        day: d.day,
                    }),
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
                    account_id: e.account().map(|a| *a.as_uuid()),
                    recorded_at: e.recorded_at(),
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
            transfers: data.transfers.iter().map(transfer_record).collect(),
            external_accounts: data
                .external_accounts
                .iter()
                .map(|a| ExternalAccountRecord {
                    id: *a.id().as_uuid(),
                    name: a.name().to_string(),
                })
                .collect(),
            goals: data
                .goals
                .iter()
                .map(|g| GoalRecord {
                    id: *g.id().as_uuid(),
                    name: g.name().to_string(),
                    target: g.target().amount(),
                    deadline: g.deadline().map(|m| m.to_string()),
                    accounts: g.accounts().iter().map(|a| *a.as_uuid()).collect(),
                    reached_on: g.reached_on(),
                })
                .collect(),
            reviews: data
                .reviews
                .iter()
                .map(|r| ReviewRecord {
                    month: r.month().to_string(),
                    seen_at: r.seen_at(),
                    next_start_balance: r.next_start_balance().map(|m| m.amount()),
                })
                .collect(),
        }
    }
}

fn endpoint_record(endpoint: Endpoint) -> EndpointRecord {
    match endpoint {
        Endpoint::Account(id) => EndpointRecord::Account(*id.as_uuid()),
        Endpoint::External(id) => EndpointRecord::External(*id.as_uuid()),
    }
}

fn transfer_record(t: &Transfer) -> TransferRecord {
    TransferRecord {
        id: *t.id().as_uuid(),
        from: endpoint_record(t.from()),
        to: endpoint_record(t.to()),
        sent: t.sent().amount(),
        received: t.received().amount(),
        label: t.label().map(ToString::to_string),
        schedule: match t.schedule() {
            Schedule::Once { date } => ScheduleRecord::Once { date },
            Schedule::Monthly { day, from, until } => ScheduleRecord::Monthly {
                day,
                from: from.to_string(),
                until: until.map(|m| m.to_string()),
            },
            Schedule::Yearly {
                month,
                day,
                from_year,
                until_year,
            } => ScheduleRecord::Yearly {
                month,
                day,
                from_year,
                until_year,
            },
        },
        recorded_at: t.recorded_at(),
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
                    SavingsProduct::Livret => SavingsProductRecord::Livret,
                    SavingsProduct::AssuranceVie => SavingsProductRecord::AssuranceVie,
                },
                balance: d.balance().amount(),
                rate: d.rate().as_fraction(),
                deposit_ceiling: d.deposit_ceiling().map(|c| c.amount()),
                balance_updated_on: d.balance_updated_on(),
                reference_at: Some(d.reference().at),
            },
            AccountKind::Securities(d) => AccountKindRecord::Securities {
                product: match d.product() {
                    SecuritiesProduct::Pea => SecuritiesProductRecord::Pea,
                    SecuritiesProduct::Crypto => SecuritiesProductRecord::Crypto,
                },
                cash: d.cash().amount(),
                cash_on: Some(d.cash_reference().on),
                cash_at: Some(d.cash_reference().at),
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
            AccountKind::Checking(d) => AccountKindRecord::Checking {
                reference_balance: d.reference_balance().amount(),
                reference_on: d.reference_on(),
                reference_at: d.reference_at(),
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
        let exported_at = self.exported_at;
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
                .map(|income| {
                    income
                        .linked_to(r.account_id.map(AccountId::from_uuid))
                        .recorded(r.recorded_at)
                })
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
            .map_err(context("dépense"))?
            .linked_to(r.account_id.map(AccountId::from_uuid))
            .recorded(r.recorded_at);
            expenses.push(expense);
        }

        let accounts = self
            .accounts
            .into_iter()
            .map(|r| account_from_record(r, exported_at))
            .collect::<Result<Vec<_>, DomainError>>()
            .map_err(context("compte"))?;
        let account_ids: HashSet<AccountId> = accounts.iter().map(Account::id).collect();
        let checking_ids: HashSet<AccountId> = accounts
            .iter()
            .filter(|a| a.account_type() == AccountType::Checking)
            .map(Account::id)
            .collect();
        let mut linked = expenses
            .iter()
            .filter_map(Expense::account)
            .chain(one_off_incomes.iter().filter_map(OneOffIncome::account));
        if linked.any(|id| !checking_ids.contains(&id)) {
            return Err(invalid("mouvement rattaché à un compte courant inconnu"));
        }
        if charges
            .iter()
            .filter_map(RecurringCharge::debit)
            .any(|debit| !checking_ids.contains(&debit.account))
        {
            return Err(invalid("charge prélevée sur un compte courant inconnu"));
        }

        let external_accounts = self
            .external_accounts
            .into_iter()
            .map(|r| {
                Ok(ExternalAccount::new(
                    ExternalId::from_uuid(r.id),
                    Label::new(&r.name)?,
                ))
            })
            .collect::<Result<Vec<_>, DomainError>>()
            .map_err(context("compte hors patrimoine"))?;
        let external_ids: HashSet<ExternalId> =
            external_accounts.iter().map(ExternalAccount::id).collect();
        let endpoint = |record: EndpointRecord| match record {
            EndpointRecord::Account(id) => {
                let id = AccountId::from_uuid(id);
                account_ids.contains(&id).then_some(Endpoint::Account(id))
            }
            EndpointRecord::External(id) => {
                let id = ExternalId::from_uuid(id);
                external_ids.contains(&id).then_some(Endpoint::External(id))
            }
        };
        let mut transfers = Vec::with_capacity(self.transfers.len());
        for r in self.transfers {
            let (Some(from), Some(to)) = (endpoint(r.from), endpoint(r.to)) else {
                return Err(invalid("virement : compte inconnu"));
            };
            let transfer = transfer_from_record(r, from, to).map_err(context("virement"))?;
            transfers.push(transfer);
        }

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

        let mut goals = Vec::with_capacity(self.goals.len());
        for r in self.goals {
            let mut linked = Vec::with_capacity(r.accounts.len());
            for id in r.accounts {
                let id = AccountId::from_uuid(id);
                if !account_ids.contains(&id) {
                    return Err(invalid("objectif : référence de compte inconnue"));
                }
                linked.push(id);
            }
            let goal = (|| {
                Ok::<_, DomainError>(
                    SavingsGoal::new(
                        GoalId::from_uuid(r.id),
                        Label::new(&r.name)?,
                        Money::new(r.target),
                        r.deadline.as_deref().map(str::parse::<Month>).transpose()?,
                        linked,
                    )?
                    .with_reached_on(r.reached_on),
                )
            })()
            .map_err(context("objectif"))?;
            goals.push(goal);
        }
        let mut reviews = Vec::with_capacity(self.reviews.len());
        for r in self.reviews {
            let month: Month = r.month.parse().map_err(context("bilan"))?;
            reviews.push(
                MonthlyReview::new(month, r.seen_at)
                    .with_next_start_balance(r.next_start_balance.map(Money::new)),
            );
        }

        Ok(Dataset {
            categories,
            salaries,
            one_off_incomes,
            charges,
            expenses,
            accounts,
            snapshots,
            transfers,
            external_accounts,
            goals,
            reviews,
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
    )?
    .with_debit(r.debit.map(|d| Debit {
        account: AccountId::from_uuid(d.account_id),
        day: d.day,
    }))
}

fn transfer_from_record(
    r: TransferRecord,
    from: Endpoint,
    to: Endpoint,
) -> Result<Transfer, DomainError> {
    let schedule = match r.schedule {
        ScheduleRecord::Once { date } => Schedule::Once { date },
        ScheduleRecord::Monthly { day, from, until } => Schedule::Monthly {
            day,
            from: from.parse()?,
            until: until.as_deref().map(str::parse::<Month>).transpose()?,
        },
        ScheduleRecord::Yearly {
            month,
            day,
            from_year,
            until_year,
        } => Schedule::Yearly {
            month,
            day,
            from_year,
            until_year,
        },
    };
    Transfer::new(
        TransferId::from_uuid(r.id),
        from,
        to,
        Money::new(r.sent),
        Money::new(r.received),
        r.label.as_deref().map(Label::new).transpose()?,
        schedule,
        r.recorded_at,
    )
}

/// Un ancien export (avant la version 3) n'a pas l'instant du dernier solde : on retient la fin
/// de journée, pour que les virements de ce jour soient réputés déjà inclus.
fn end_of_day(date: NaiveDate) -> DateTime<Utc> {
    date.and_hms_opt(23, 59, 59)
        .expect("heure valide")
        .and_utc()
}

fn account_from_record(
    r: AccountRecord,
    exported_at: DateTime<Utc>,
) -> Result<Account, DomainError> {
    let kind = match r.kind {
        AccountKindRecord::Savings {
            product,
            balance,
            rate,
            deposit_ceiling,
            balance_updated_on,
            reference_at,
        } => AccountKind::Savings(SavingsDetails::new(
            match product {
                SavingsProductRecord::LivretA => SavingsProduct::LivretA,
                SavingsProductRecord::Ldds => SavingsProduct::Ldds,
                SavingsProductRecord::Livret => SavingsProduct::Livret,
                SavingsProductRecord::AssuranceVie => SavingsProduct::AssuranceVie,
            },
            BalanceReference::new(
                Money::new(balance),
                balance_updated_on,
                reference_at.unwrap_or_else(|| end_of_day(balance_updated_on)),
            ),
            Rate::from_fraction(rate)?,
            deposit_ceiling.map(Money::new),
        )?),
        AccountKindRecord::Securities {
            product,
            cash,
            positions,
            cash_on,
            cash_at,
        } => AccountKind::Securities(SecuritiesDetails::new(
            match product {
                SecuritiesProductRecord::Pea => SecuritiesProduct::Pea,
                SecuritiesProductRecord::Crypto => SecuritiesProduct::Crypto,
            },
            BalanceReference::new(
                Money::new(cash),
                cash_on.unwrap_or_else(|| exported_at.date_naive()),
                cash_at.unwrap_or(exported_at),
            ),
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
        AccountKindRecord::Checking {
            reference_balance,
            reference_on,
            reference_at,
        } => AccountKind::Checking(CheckingDetails::new(
            Money::new(reference_balance),
            reference_on,
            reference_at,
        )?),
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
