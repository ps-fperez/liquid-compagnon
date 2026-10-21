use lc_application::ports::{PlanningRepository, StorageResult};
use lc_domain::goal::SavingsGoal;
use lc_domain::review::MonthlyReview;
use lc_domain::{AccountId, GoalId, Label};
use rusqlite::{Connection, params};

use super::{Database, SqlResultExt, date, datetime, domain, money, month, optional_money, uuid};

/// Objectifs d'épargne et bilans du mois.
pub struct SqlitePlanningRepository {
    db: Database,
}

impl SqlitePlanningRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

type GoalRow = (String, String, String, Option<String>, Option<String>);

impl PlanningRepository for SqlitePlanningRepository {
    fn goals(&self) -> StorageResult<Vec<SavingsGoal>> {
        self.db.read(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT id, name, target, deadline, reached_on FROM savings_goals
                     ORDER BY rowid",
                )
                .db()?;
            let rows: Vec<GoalRow> = statement
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                })
                .db()?
                .collect::<rusqlite::Result<_>>()
                .db()?;
            let mut accounts = conn
                .prepare(
                    "SELECT account_id FROM goal_accounts WHERE goal_id = ?1 ORDER BY position",
                )
                .db()?;
            rows.into_iter()
                .map(|(id, name, target, deadline, reached_on)| {
                    let linked = accounts
                        .query_map([&id], |row| row.get::<_, String>(0))
                        .db()?
                        .collect::<rusqlite::Result<Vec<_>>>()
                        .db()?
                        .iter()
                        .map(|raw| uuid(raw).map(AccountId::from_uuid))
                        .collect::<StorageResult<Vec<_>>>()?;
                    let goal = domain(SavingsGoal::new(
                        GoalId::from_uuid(uuid(&id)?),
                        domain(Label::new(&name))?,
                        money(&target)?,
                        deadline.as_deref().map(month).transpose()?,
                        linked,
                    ))?;
                    Ok(goal.with_reached_on(reached_on.as_deref().map(date).transpose()?))
                })
                .collect()
        })
    }

    fn save_goal(&self, goal: &SavingsGoal) -> StorageResult<()> {
        self.db.write(|tx| save_goal(tx, goal))
    }

    fn delete_goal(&self, id: GoalId) -> StorageResult<bool> {
        self.db.write(|tx| {
            tx.execute(
                "DELETE FROM goal_accounts WHERE goal_id = ?1",
                [id.to_string()],
            )
            .db()?;
            tx.execute("DELETE FROM savings_goals WHERE id = ?1", [id.to_string()])
                .db()
                .map(|deleted| deleted > 0)
        })
    }

    fn reviews(&self) -> StorageResult<Vec<MonthlyReview>> {
        self.db.read(|conn| {
            let mut statement = conn
                .prepare(
                    "SELECT month, seen_at, next_start_balance FROM monthly_reviews ORDER BY month",
                )
                .db()?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })
                .db()?
                .collect::<rusqlite::Result<Vec<_>>>()
                .db()?;
            rows.into_iter()
                .map(|(m, seen_at, balance)| {
                    Ok(MonthlyReview::new(month(&m)?, datetime(&seen_at)?)
                        .with_next_start_balance(optional_money(balance)?))
                })
                .collect()
        })
    }

    fn save_review(&self, review: &MonthlyReview) -> StorageResult<()> {
        self.db.write(|tx| save_review(tx, review))
    }
}

pub(crate) fn save_goal(conn: &Connection, goal: &SavingsGoal) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO savings_goals (id, name, target, deadline, reached_on)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, target = excluded.target,
             deadline = excluded.deadline, reached_on = excluded.reached_on",
        params![
            goal.id().to_string(),
            goal.name().as_str(),
            goal.target().amount().to_string(),
            goal.deadline().map(|m| m.to_string()),
            goal.reached_on().map(|d| d.to_string()),
        ],
    )
    .db()?;
    conn.execute(
        "DELETE FROM goal_accounts WHERE goal_id = ?1",
        [goal.id().to_string()],
    )
    .db()?;
    for (position, account) in goal.accounts().iter().enumerate() {
        conn.execute(
            "INSERT INTO goal_accounts (goal_id, account_id, position) VALUES (?1, ?2, ?3)",
            params![goal.id().to_string(), account.to_string(), position as i64],
        )
        .db()?;
    }
    Ok(())
}

pub(crate) fn save_review(conn: &Connection, review: &MonthlyReview) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO monthly_reviews (month, seen_at, next_start_balance) VALUES (?1, ?2, ?3)
         ON CONFLICT(month) DO UPDATE SET seen_at = excluded.seen_at,
             next_start_balance = excluded.next_start_balance",
        params![
            review.month().to_string(),
            review.seen_at().to_rfc3339(),
            review.next_start_balance().map(|m| m.amount().to_string()),
        ],
    )
    .db()
    .map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use lc_domain::{Money, Month};
    use rust_decimal_macros::dec;

    #[test]
    fn goals_and_reviews_roundtrip() {
        let repo = SqlitePlanningRepository::new(Database::open_in_memory().unwrap());
        let (a, b) = (AccountId::generate(), AccountId::generate());
        let goal = SavingsGoal::new(
            GoalId::generate(),
            Label::new("Apport immobilier").unwrap(),
            Money::new(dec!(30000)),
            Some(Month::new(2028, 12).unwrap()),
            vec![b, a],
        )
        .unwrap()
        .with_reached_on(Some(NaiveDate::from_ymd_opt(2026, 10, 21).unwrap()));
        repo.save_goal(&goal).unwrap();
        assert_eq!(repo.goals().unwrap(), vec![goal.clone()]);
        assert!(repo.delete_goal(goal.id()).unwrap());
        assert!(repo.goals().unwrap().is_empty());

        let review = MonthlyReview::new(
            Month::new(2026, 10).unwrap(),
            Utc.with_ymd_and_hms(2026, 11, 1, 8, 0, 0).unwrap(),
        )
        .with_next_start_balance(Some(Money::new(dec!(250))));
        repo.save_review(&review).unwrap();
        assert_eq!(repo.reviews().unwrap(), vec![review]);
    }
}
