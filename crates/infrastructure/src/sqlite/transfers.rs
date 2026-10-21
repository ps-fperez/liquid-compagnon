use lc_application::ports::{StorageResult, TransferRepository};
use lc_domain::transfer::{Endpoint, ExternalAccount, Schedule, Transfer};
use lc_domain::{AccountId, ExternalId, Label, TransferId};
use rusqlite::{Connection, Row, params};

use super::{Database, SqlResultExt, corrupt, date, datetime, domain, money, month, uuid};

/// Virements et comptes hors patrimoine.
pub struct SqliteTransferRepository {
    db: Database,
}

impl SqliteTransferRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl TransferRepository for SqliteTransferRepository {
    fn transfers(&self) -> StorageResult<Vec<Transfer>> {
        self.db.read(|conn| {
            let mut statement = conn
                .prepare("SELECT * FROM transfers ORDER BY rowid")
                .db()?;
            let rows = statement
                .query_map([], TransferRow::read)
                .db()?
                .collect::<rusqlite::Result<Vec<_>>>()
                .db()?;
            rows.into_iter().map(TransferRow::into_transfer).collect()
        })
    }

    fn save_transfer(&self, transfer: &Transfer) -> StorageResult<()> {
        self.db.write(|tx| save_transfer(tx, transfer))
    }

    fn delete_transfer(&self, id: TransferId) -> StorageResult<bool> {
        self.db.write(|tx| {
            tx.execute("DELETE FROM transfers WHERE id = ?1", [id.to_string()])
                .db()
                .map(|deleted| deleted > 0)
        })
    }

    fn external_accounts(&self) -> StorageResult<Vec<ExternalAccount>> {
        self.db.read(|conn| {
            let mut statement = conn
                .prepare("SELECT id, name FROM external_accounts ORDER BY rowid")
                .db()?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .db()?
                .collect::<rusqlite::Result<Vec<_>>>()
                .db()?;
            rows.into_iter()
                .map(|(id, name)| {
                    Ok(ExternalAccount::new(
                        ExternalId::from_uuid(uuid(&id)?),
                        domain(Label::new(&name))?,
                    ))
                })
                .collect()
        })
    }

    fn save_external_account(&self, account: &ExternalAccount) -> StorageResult<()> {
        self.db.write(|tx| save_external_account(tx, account))
    }

    fn delete_external_account(&self, id: ExternalId) -> StorageResult<bool> {
        self.db.write(|tx| {
            tx.execute(
                "DELETE FROM external_accounts WHERE id = ?1",
                [id.to_string()],
            )
            .db()
            .map(|deleted| deleted > 0)
        })
    }
}

pub(crate) fn save_external_account(
    conn: &Connection,
    account: &ExternalAccount,
) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO external_accounts (id, name) VALUES (?1, ?2)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name",
        params![account.id().to_string(), account.name().as_str()],
    )
    .db()
    .map(drop)
}

fn endpoint_to_sql(endpoint: Endpoint) -> (&'static str, String) {
    match endpoint {
        Endpoint::Account(id) => ("account", id.to_string()),
        Endpoint::External(id) => ("external", id.to_string()),
    }
}

fn endpoint_from_sql(kind: &str, id: &str) -> StorageResult<Endpoint> {
    let id = uuid(id)?;
    match kind {
        "account" => Ok(Endpoint::Account(AccountId::from_uuid(id))),
        "external" => Ok(Endpoint::External(ExternalId::from_uuid(id))),
        other => Err(corrupt(format!("côté de virement inconnu « {other} »"))),
    }
}

pub(crate) fn save_transfer(conn: &Connection, transfer: &Transfer) -> StorageResult<()> {
    let (from_kind, from_id) = endpoint_to_sql(transfer.from());
    let (to_kind, to_id) = endpoint_to_sql(transfer.to());
    let mut columns = ScheduleColumns::default();
    match transfer.schedule() {
        Schedule::Once { date } => {
            columns.kind = "once";
            columns.on_date = Some(date.to_string());
        }
        Schedule::Monthly { day, from, until } => {
            columns.kind = "monthly";
            columns.day = Some(day);
            columns.from_month = Some(from.to_string());
            columns.until_month = until.map(|m| m.to_string());
        }
        Schedule::Yearly {
            month,
            day,
            from_year,
            until_year,
        } => {
            columns.kind = "yearly";
            columns.day = Some(day);
            columns.month = Some(month);
            columns.from_year = Some(from_year);
            columns.until_year = until_year;
        }
    }
    conn.execute(
        "INSERT INTO transfers (id, from_kind, from_id, to_kind, to_id, sent, received, label,
             schedule, on_date, day, month, from_month, until_month, from_year, until_year,
             recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
         ON CONFLICT(id) DO UPDATE SET from_kind = excluded.from_kind,
             from_id = excluded.from_id, to_kind = excluded.to_kind, to_id = excluded.to_id,
             sent = excluded.sent, received = excluded.received, label = excluded.label,
             schedule = excluded.schedule, on_date = excluded.on_date, day = excluded.day,
             month = excluded.month, from_month = excluded.from_month,
             until_month = excluded.until_month, from_year = excluded.from_year,
             until_year = excluded.until_year, recorded_at = excluded.recorded_at",
        params![
            transfer.id().to_string(),
            from_kind,
            from_id,
            to_kind,
            to_id,
            transfer.sent().amount().to_string(),
            transfer.received().amount().to_string(),
            transfer.label().map(Label::as_str),
            columns.kind,
            columns.on_date,
            columns.day,
            columns.month,
            columns.from_month,
            columns.until_month,
            columns.from_year,
            columns.until_year,
            transfer.recorded_at().to_rfc3339(),
        ],
    )
    .db()
    .map(drop)
}

#[derive(Default)]
struct ScheduleColumns {
    kind: &'static str,
    on_date: Option<String>,
    day: Option<u8>,
    month: Option<u32>,
    from_month: Option<String>,
    until_month: Option<String>,
    from_year: Option<i32>,
    until_year: Option<i32>,
}

struct TransferRow {
    id: String,
    from: (String, String),
    to: (String, String),
    sent: String,
    received: String,
    label: Option<String>,
    schedule: String,
    on_date: Option<String>,
    day: Option<u8>,
    month: Option<u32>,
    from_month: Option<String>,
    until_month: Option<String>,
    from_year: Option<i32>,
    until_year: Option<i32>,
    recorded_at: String,
}

impl TransferRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            from: (row.get("from_kind")?, row.get("from_id")?),
            to: (row.get("to_kind")?, row.get("to_id")?),
            sent: row.get("sent")?,
            received: row.get("received")?,
            label: row.get("label")?,
            schedule: row.get("schedule")?,
            on_date: row.get("on_date")?,
            day: row.get("day")?,
            month: row.get("month")?,
            from_month: row.get("from_month")?,
            until_month: row.get("until_month")?,
            from_year: row.get("from_year")?,
            until_year: row.get("until_year")?,
            recorded_at: row.get("recorded_at")?,
        })
    }

    fn into_transfer(self) -> StorageResult<Transfer> {
        let missing = |column: &str| {
            corrupt(format!(
                "colonne {column} vide pour le virement {}",
                self.id
            ))
        };
        let schedule = match self.schedule.as_str() {
            "once" => Schedule::Once {
                date: date(self.on_date.as_deref().ok_or_else(|| missing("on_date"))?)?,
            },
            "monthly" => Schedule::Monthly {
                day: self.day.ok_or_else(|| missing("day"))?,
                from: month(
                    self.from_month
                        .as_deref()
                        .ok_or_else(|| missing("from_month"))?,
                )?,
                until: self.until_month.as_deref().map(month).transpose()?,
            },
            "yearly" => Schedule::Yearly {
                month: self.month.ok_or_else(|| missing("month"))?,
                day: self.day.ok_or_else(|| missing("day"))?,
                from_year: self.from_year.ok_or_else(|| missing("from_year"))?,
                until_year: self.until_year,
            },
            other => return Err(corrupt(format!("fréquence inconnue « {other} »"))),
        };
        domain(Transfer::new(
            TransferId::from_uuid(uuid(&self.id)?),
            endpoint_from_sql(&self.from.0, &self.from.1)?,
            endpoint_from_sql(&self.to.0, &self.to.1)?,
            money(&self.sent)?,
            money(&self.received)?,
            self.label
                .as_deref()
                .map(Label::new)
                .transpose()
                .map_err(corrupt)?,
            schedule,
            datetime(&self.recorded_at)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use lc_domain::{Money, Month, NaiveDate};
    use rust_decimal_macros::dec;

    #[test]
    fn roundtrips_transfers_and_external_accounts() {
        let repo = SqliteTransferRepository::new(Database::open_in_memory().unwrap());
        let joint =
            ExternalAccount::new(ExternalId::generate(), Label::new("Compte joint").unwrap());
        repo.save_external_account(&joint).unwrap();
        let checking = Endpoint::Account(AccountId::generate());
        let pei = Endpoint::Account(AccountId::generate());
        let at = Utc.with_ymd_and_hms(2026, 10, 21, 9, 30, 0).unwrap();
        let transfers = [
            Transfer::new(
                TransferId::generate(),
                checking,
                Endpoint::External(joint.id()),
                Money::new(dec!(1560)),
                Money::new(dec!(1560)),
                None,
                Schedule::Monthly {
                    day: 1,
                    from: Month::new(2026, 10).unwrap(),
                    until: Some(Month::new(2027, 6).unwrap()),
                },
                at,
            )
            .unwrap(),
            Transfer::new(
                TransferId::generate(),
                checking,
                pei,
                Money::new(dec!(500)),
                Money::new(dec!(1493.30)),
                Some(Label::new("Abondement").unwrap()),
                Schedule::Yearly {
                    month: 1,
                    day: 15,
                    from_year: 2027,
                    until_year: None,
                },
                at,
            )
            .unwrap(),
            Transfer::new(
                TransferId::generate(),
                pei,
                checking,
                Money::new(dec!(100)),
                Money::new(dec!(100)),
                None,
                Schedule::Once {
                    date: NaiveDate::from_ymd_opt(2026, 10, 21).unwrap(),
                },
                at,
            )
            .unwrap(),
        ];
        for transfer in &transfers {
            repo.save_transfer(transfer).unwrap();
        }

        assert_eq!(repo.transfers().unwrap(), transfers);
        assert_eq!(repo.external_accounts().unwrap(), vec![joint.clone()]);
        assert!(repo.delete_transfer(transfers[0].id()).unwrap());
        assert!(repo.delete_external_account(joint.id()).unwrap());
        assert!(!repo.delete_external_account(joint.id()).unwrap());
    }
}
