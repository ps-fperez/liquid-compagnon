//! Tableau de bord : patrimoine total, détail par compte, variations, historique, budget.

use std::sync::Arc;

use lc_domain::account::{Account, AccountType};
use lc_domain::valuation::AccountValuation;
use lc_domain::{
    AccountId, Color, Decimal, Label, Money, Month, Snapshot, SnapshotId, SnapshotKind,
};

use crate::AppResult;
use crate::budget::{BudgetSummary, summarize};
use crate::ports::{AccountRepository, BudgetRepository, Clock, SnapshotRepository};
use crate::pricing::{PricingIssue, PricingService, checking_ledger, value_active_accounts};

/// Variation par rapport au dernier snapshot automatique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variation {
    pub amount: Money,
    /// En pourcentage de la valeur de référence ; absent si celle-ci est nulle.
    pub percent: Option<Decimal>,
}

impl Variation {
    fn between(reference: Money, current: Money) -> Self {
        let amount = current - reference;
        let percent = reference
            .is_positive()
            .then(|| amount.amount() / reference.amount() * Decimal::ONE_HUNDRED);
        Self { amount, percent }
    }
}

/// Ligne d'un compte sur le tableau de bord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountLine {
    pub account_id: AccountId,
    pub name: Label,
    pub color: Color,
    pub account_type: AccountType,
    pub value: Money,
    /// Part du patrimoine total, en pourcentage.
    pub share_percent: Decimal,
    pub variation: Option<Variation>,
    pub valuation: AccountValuation,
}

/// Point de la courbe d'évolution (dernier snapshot de chaque mois).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryPoint {
    pub month: Month,
    pub total: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dashboard {
    pub total: Money,
    pub variation: Option<Variation>,
    pub accounts: Vec<AccountLine>,
    pub budget: BudgetSummary,
    pub history: Vec<HistoryPoint>,
    pub pricing_issues: Vec<PricingIssue>,
}

pub struct DashboardUseCases {
    accounts: Arc<dyn AccountRepository>,
    budget: Arc<dyn BudgetRepository>,
    snapshots: Arc<dyn SnapshotRepository>,
    pricing: Arc<PricingService>,
    clock: Arc<dyn Clock>,
}

impl DashboardUseCases {
    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        budget: Arc<dyn BudgetRepository>,
        snapshots: Arc<dyn SnapshotRepository>,
        pricing: Arc<PricingService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            accounts,
            budget,
            snapshots,
            pricing,
            clock,
        }
    }

    /// Construit le tableau de bord. Prend au passage le snapshot automatique du mois s'il
    /// n'existe pas encore (première ouverture du mois).
    pub async fn dashboard(&self, force_refresh: bool) -> AppResult<Dashboard> {
        let (valued, pricing_issues) = value_active_accounts(
            self.accounts.list()?,
            &self.pricing,
            &checking_ledger(self.budget.as_ref())?,
            force_refresh,
        )
        .await?;
        let mut snapshots = self.snapshots.list()?;
        let today = self.clock.today();

        if Snapshot::is_auto_due(&snapshots, Month::of(today)) {
            let snapshot = capture(&valued, SnapshotKind::Auto, today);
            self.snapshots.save(&snapshot)?;
            snapshots.push(snapshot);
        }

        let total: Money = valued.iter().map(|(_, valuation)| valuation.value).sum();
        let reference = snapshots
            .iter()
            .filter(|s| s.kind() == SnapshotKind::Auto)
            .max_by_key(|s| s.taken_on());
        let accounts = valued
            .iter()
            .map(|(account, valuation)| AccountLine {
                account_id: account.id(),
                name: account.name().clone(),
                color: account.color().clone(),
                account_type: account.account_type(),
                value: valuation.value,
                share_percent: share(valuation.value, total),
                variation: reference
                    .and_then(|s| s.value_of(account.id()))
                    .map(|previous| Variation::between(previous, valuation.value)),
                valuation: valuation.clone(),
            })
            .collect();

        Ok(Dashboard {
            total,
            variation: reference.map(|s| Variation::between(s.total(), total)),
            accounts,
            budget: summarize(self.budget.as_ref(), Month::of(today))?,
            history: history(&snapshots),
            pricing_issues,
        })
    }

    /// Snapshot demandé par l'utilisateur.
    pub async fn take_manual_snapshot(&self) -> AppResult<Snapshot> {
        let (valued, _) = value_active_accounts(
            self.accounts.list()?,
            &self.pricing,
            &checking_ledger(self.budget.as_ref())?,
            false,
        )
        .await?;
        let snapshot = capture(&valued, SnapshotKind::Manual, self.clock.today());
        self.snapshots.save(&snapshot)?;
        Ok(snapshot)
    }
}

fn capture(
    valued: &[(Account, AccountValuation)],
    kind: SnapshotKind,
    today: lc_domain::NaiveDate,
) -> Snapshot {
    Snapshot::capture(
        SnapshotId::generate(),
        today,
        kind,
        valued
            .iter()
            .map(|(account, valuation)| (account, valuation.value)),
    )
}

fn share(value: Money, total: Money) -> Decimal {
    if total.is_positive() {
        value.amount() / total.amount() * Decimal::ONE_HUNDRED
    } else {
        Decimal::ZERO
    }
}

/// Un point par mois (le snapshot le plus récent du mois), dans l'ordre chronologique. Un mois
/// sans ouverture de l'application n'a pas de point : rien n'est inventé.
fn history(snapshots: &[Snapshot]) -> Vec<HistoryPoint> {
    let mut points: Vec<HistoryPoint> = Vec::new();
    let mut sorted: Vec<&Snapshot> = snapshots.iter().collect();
    sorted.sort_by_key(|s| s.taken_on());
    for snapshot in sorted {
        match points.last_mut() {
            Some(last) if last.month == snapshot.month() => last.total = snapshot.total(),
            _ => points.push(HistoryPoint {
                month: snapshot.month(),
                total: snapshot.total(),
            }),
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::NewAccount;
    use crate::testing::{TestEnv, asset};
    use chrono::{Duration, NaiveDate};
    use lc_domain::account::QuoteSource;
    use pollster::block_on;
    use rust_decimal::RoundingStrategy;
    use rust_decimal_macros::dec;

    fn open(env: &TestEnv, name: &str, account_type: AccountType) -> AccountId {
        env.accounts()
            .create_account(NewAccount {
                name: name.into(),
                account_type,
                color: None,
                monthly_contribution: Money::ZERO,
            })
            .unwrap()
    }

    fn livret(env: &TestEnv, name: &str, account_type: AccountType, balance: Decimal) -> AccountId {
        let id = open(env, name, account_type);
        env.accounts()
            .update_balance(id, Money::new(balance))
            .unwrap();
        id
    }

    // Spec wealth-dashboard : « Agrégation » et « Répartition »
    #[test]
    fn total_and_shares() {
        let env = TestEnv::new();
        livret(&env, "Livret A", AccountType::LivretA, dec!(10640));
        livret(&env, "LDDS", AccountType::Ldds, dec!(5200));
        let pea = open(&env, "PEA", AccountType::Pea);
        env.accounts()
            .set_cash(pea, Money::new(dec!(14510)))
            .unwrap();
        livret(&env, "Livret A bis", AccountType::LivretA, dec!(9310));
        let wallet = open(&env, "Crypto", AccountType::Crypto);
        env.coingecko.set_price("bitcoin", dec!(87020));
        env.accounts()
            .upsert_position(
                wallet,
                asset(QuoteSource::CoinGecko, "bitcoin"),
                dec!(0.1),
                None,
            )
            .unwrap();

        let dashboard = block_on(env.dashboard().dashboard(false)).unwrap();

        assert_eq!(dashboard.total, Money::new(dec!(48362)));
        let pea_line = dashboard
            .accounts
            .iter()
            .find(|line| line.account_id == pea)
            .unwrap();
        assert_eq!(
            pea_line
                .share_percent
                .round_dp_with_strategy(1, RoundingStrategy::MidpointAwayFromZero),
            dec!(30.0)
        );
    }

    // Spec wealth-dashboard : « Premier lancement du mois »
    #[test]
    fn first_opening_of_month_takes_one_auto_snapshot() {
        let env = TestEnv::new();
        livret(&env, "Livret A", AccountType::LivretA, dec!(1000));
        let dashboard = env.dashboard();

        block_on(dashboard.dashboard(false)).unwrap();
        block_on(dashboard.dashboard(false)).unwrap();

        let snapshots = env.snapshots.list().unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].kind(), SnapshotKind::Auto);
        assert_eq!(snapshots[0].month(), Month::new(2026, 10).unwrap());
    }

    // Spec wealth-dashboard : « Hausse mensuelle »
    #[test]
    fn variation_since_last_auto_snapshot() {
        let env = TestEnv::new();
        let id = livret(&env, "Livret A", AccountType::LivretA, dec!(47230));
        let dashboard = env.dashboard();
        block_on(dashboard.dashboard(false)).unwrap();

        env.clock.advance(Duration::days(5));
        env.accounts()
            .update_balance(id, Money::new(dec!(48362)))
            .unwrap();
        let after = block_on(dashboard.dashboard(false)).unwrap();

        let variation = after.variation.unwrap();
        assert_eq!(variation.amount, Money::new(dec!(1132)));
        assert_eq!(variation.percent.unwrap().round_dp(1), dec!(2.4));
        assert_eq!(
            after.accounts[0].variation.unwrap().amount,
            Money::new(dec!(1132))
        );
    }

    // Spec wealth-dashboard : « Mois sans ouverture »
    #[test]
    fn history_has_one_point_per_opened_month() {
        let env = TestEnv::new();
        let id = livret(&env, "Livret A", AccountType::LivretA, dec!(1000));
        let dashboard = env.dashboard();
        block_on(dashboard.dashboard(false)).unwrap(); // octobre

        env.clock
            .set_date(NaiveDate::from_ymd_opt(2026, 11, 3).unwrap());
        env.accounts()
            .update_balance(id, Money::new(dec!(1100)))
            .unwrap();
        block_on(dashboard.dashboard(false)).unwrap(); // novembre
        block_on(dashboard.take_manual_snapshot()).unwrap();

        env.clock
            .set_date(NaiveDate::from_ymd_opt(2027, 1, 4).unwrap());
        let january = block_on(dashboard.dashboard(false)).unwrap(); // décembre sauté

        let months: Vec<String> = january
            .history
            .iter()
            .map(|p| p.month.to_string())
            .collect();
        assert_eq!(months, ["2026-10", "2026-11", "2027-01"]);
        assert_eq!(january.history[1].total, Money::new(dec!(1100)));
    }

    #[test]
    fn archived_accounts_are_excluded() {
        let env = TestEnv::new();
        livret(&env, "Livret A", AccountType::LivretA, dec!(1000));
        let closed = livret(&env, "LDDS", AccountType::Ldds, dec!(500));
        env.accounts().archive_account(closed).unwrap();

        let dashboard = block_on(env.dashboard().dashboard(false)).unwrap();

        assert_eq!(dashboard.total, Money::new(dec!(1000)));
        assert_eq!(dashboard.accounts.len(), 1);
    }

    #[test]
    fn empty_dashboard_has_zero_total_and_no_variation_percent() {
        let env = TestEnv::new();
        let dashboard = block_on(env.dashboard().dashboard(false)).unwrap();
        assert_eq!(dashboard.total, Money::ZERO);
        assert_eq!(dashboard.variation.unwrap().percent, None);
    }

    // Spec wealth-dashboard : « Synthèse du mois »
    #[test]
    fn includes_budget_summary_and_pricing_issues() {
        let env = TestEnv::new();
        env.budget()
            .upsert_salary(None, Month::new(2026, 1).unwrap(), Money::new(dec!(3200)))
            .unwrap();
        let pea = open(&env, "PEA", AccountType::Pea);
        env.accounts()
            .upsert_position(
                pea,
                asset(QuoteSource::TwelveData, "CW8"),
                dec!(42),
                Some(Money::new(dec!(500))),
            )
            .unwrap();
        env.twelve_data
            .fail_with(crate::ports::PriceError::MissingApiKey);

        let dashboard = block_on(env.dashboard().dashboard(false)).unwrap();

        assert_eq!(dashboard.budget.total_income, Money::new(dec!(3200)));
        assert_eq!(dashboard.total, Money::new(dec!(21000)));
        assert_eq!(
            dashboard.pricing_issues,
            vec![PricingIssue::MissingApiKey(QuoteSource::TwelveData)]
        );
    }
}
