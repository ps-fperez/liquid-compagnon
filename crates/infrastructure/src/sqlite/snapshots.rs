use lc_application::ports::{SnapshotRepository, StorageResult};
use lc_domain::{AccountId, Snapshot, SnapshotId, SnapshotKind, SnapshotLine};
use rusqlite::{Connection, params};

use super::{Database, SqlResultExt, corrupt, date, money, month, uuid};

pub struct SqliteSnapshotRepository {
    db: Database,
}

impl SqliteSnapshotRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl SnapshotRepository for SqliteSnapshotRepository {
    fn list(&self) -> StorageResult<Vec<Snapshot>> {
        self.db.read(load_snapshots)
    }

    fn save(&self, snapshot: &Snapshot) -> StorageResult<()> {
        self.db.write(|tx| save_snapshot(tx, snapshot))
    }
}

pub(crate) fn save_snapshot(conn: &Connection, snapshot: &Snapshot) -> StorageResult<()> {
    let id = snapshot.id().to_string();
    let kind = match snapshot.kind() {
        SnapshotKind::Auto => "auto",
        SnapshotKind::Manual => "manual",
    };
    conn.execute(
        "INSERT INTO snapshots (id, month, taken_on, kind) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET month = excluded.month, taken_on = excluded.taken_on,
             kind = excluded.kind",
        params![
            id,
            snapshot.month().to_string(),
            snapshot.taken_on().to_string(),
            kind
        ],
    )
    .db()?;
    conn.execute("DELETE FROM snapshot_lines WHERE snapshot_id = ?1", [&id])
        .db()?;
    for (ordinal, line) in snapshot.lines().iter().enumerate() {
        conn.execute(
            "INSERT INTO snapshot_lines (snapshot_id, ordinal, account_id, value)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                id,
                ordinal as i64,
                line.account.to_string(),
                line.value.amount().to_string()
            ],
        )
        .db()?;
    }
    Ok(())
}

fn load_snapshots(conn: &Connection) -> StorageResult<Vec<Snapshot>> {
    let mut statement = conn
        .prepare("SELECT id, month, taken_on, kind FROM snapshots ORDER BY taken_on, rowid")
        .db()?;
    let headers = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .db()?
        .collect::<rusqlite::Result<Vec<_>>>()
        .db()?;
    let mut lines_statement = conn
        .prepare(
            "SELECT account_id, value FROM snapshot_lines WHERE snapshot_id = ?1 ORDER BY ordinal",
        )
        .db()?;

    headers
        .into_iter()
        .map(|(id, snapshot_month, taken_on, kind)| {
            let raw_lines = lines_statement
                .query_map([&id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .db()?
                .collect::<rusqlite::Result<Vec<_>>>()
                .db()?;
            let lines = raw_lines
                .into_iter()
                .map(|(account, value)| {
                    Ok(SnapshotLine {
                        account: AccountId::from_uuid(uuid(&account)?),
                        value: money(&value)?,
                    })
                })
                .collect::<StorageResult<Vec<_>>>()?;
            let kind = match kind.as_str() {
                "auto" => SnapshotKind::Auto,
                "manual" => SnapshotKind::Manual,
                other => return Err(corrupt(format!("type de snapshot inconnu « {other} »"))),
            };
            Ok(Snapshot::restore(
                SnapshotId::from_uuid(uuid(&id)?),
                month(&snapshot_month)?,
                date(&taken_on)?,
                kind,
                lines,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc_domain::{Money, Month, NaiveDate};
    use rust_decimal_macros::dec;

    #[test]
    fn roundtrips_snapshots_in_chronological_order() {
        let repo = SqliteSnapshotRepository::new(Database::open_in_memory().unwrap());
        let lines = vec![
            SnapshotLine {
                account: AccountId::generate(),
                value: Money::new(dec!(10640)),
            },
            SnapshotLine {
                account: AccountId::generate(),
                value: Money::new(dec!(14510.37)),
            },
        ];
        let november = Snapshot::restore(
            SnapshotId::generate(),
            Month::new(2026, 11).unwrap(),
            NaiveDate::from_ymd_opt(2026, 11, 3).unwrap(),
            SnapshotKind::Manual,
            lines.clone(),
        );
        let october = Snapshot::restore(
            SnapshotId::generate(),
            Month::new(2026, 10).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            SnapshotKind::Auto,
            lines,
        );

        repo.save(&november).unwrap();
        repo.save(&october).unwrap();

        let stored = repo.list().unwrap();
        assert_eq!(stored, vec![october, november]);
        assert_eq!(stored[0].total(), Money::new(dec!(25150.37)));
    }
}
