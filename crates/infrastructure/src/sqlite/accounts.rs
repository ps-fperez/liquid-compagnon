use lc_application::ports::{AccountRepository, StorageResult};
use lc_domain::account::{
    Account, AccountKind, AssetRef, CheckingDetails, CurrencyCode, EmployeeSavingsDetails,
    NavAnchor, PeiFund, Position, SavingsDetails, SavingsProduct, ScenarioRates, SecuritiesDetails,
    SecuritiesProduct,
};
use lc_domain::{AccountId, Color, FundId, Label, Money, Rate};
use rusqlite::{Connection, Row, params};

use super::{
    Database, SqlResultExt, corrupt, date, datetime, decimal, domain, money, optional_money,
    source_from_sql, source_to_sql, uuid,
};

pub struct SqliteAccountRepository {
    db: Database,
}

impl SqliteAccountRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl AccountRepository for SqliteAccountRepository {
    fn list(&self) -> StorageResult<Vec<Account>> {
        self.db.read(|conn| load_accounts(conn, None))
    }

    fn get(&self, id: AccountId) -> StorageResult<Option<Account>> {
        self.db
            .read(|conn| Ok(load_accounts(conn, Some(id))?.into_iter().next()))
    }

    fn save(&self, account: &Account) -> StorageResult<()> {
        self.db.write(|tx| save_account(tx, account))
    }
}

// --- Écriture -----------------------------------------------------------------------------

/// Crée ou remplace le compte, ses positions et ses fonds.
pub(crate) fn save_account(conn: &Connection, account: &Account) -> StorageResult<()> {
    let id = account.id().to_string();
    let rates = account.scenario_rates();
    let (nature, product, balance, rate, ceiling, updated_on, cash, reference_at) =
        match account.kind() {
            AccountKind::Savings(d) => (
                "savings",
                Some(match d.product() {
                    SavingsProduct::LivretA => "livret_a",
                    SavingsProduct::Ldds => "ldds",
                }),
                Some(d.balance().amount().to_string()),
                Some(d.rate().as_fraction().to_string()),
                Some(d.deposit_ceiling().amount().to_string()),
                Some(d.balance_updated_on().to_string()),
                None,
                None,
            ),
            AccountKind::Securities(d) => (
                "securities",
                Some(match d.product() {
                    SecuritiesProduct::Pea => "pea",
                    SecuritiesProduct::Crypto => "crypto",
                }),
                None,
                None,
                None,
                None,
                Some(d.cash().amount().to_string()),
                None,
            ),
            AccountKind::EmployeeSavings(_) => {
                ("employee_savings", None, None, None, None, None, None, None)
            }
            AccountKind::Checking(d) => (
                "checking",
                None,
                Some(d.reference_balance().amount().to_string()),
                None,
                None,
                Some(d.reference_on().to_string()),
                None,
                Some(d.reference_at().to_rfc3339()),
            ),
        };
    conn.execute(
        "INSERT INTO accounts (id, name, color, nature, product, balance, rate, deposit_ceiling,
             balance_updated_on, cash, monthly_contribution, rate_pessimistic, rate_neutral,
             rate_optimistic, archived, reference_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name, color = excluded.color, nature = excluded.nature,
             product = excluded.product, balance = excluded.balance, rate = excluded.rate,
             deposit_ceiling = excluded.deposit_ceiling,
             balance_updated_on = excluded.balance_updated_on, cash = excluded.cash,
             monthly_contribution = excluded.monthly_contribution,
             rate_pessimistic = excluded.rate_pessimistic, rate_neutral = excluded.rate_neutral,
             rate_optimistic = excluded.rate_optimistic, archived = excluded.archived,
             reference_at = excluded.reference_at",
        params![
            id,
            account.name().as_str(),
            account.color().as_str(),
            nature,
            product,
            balance,
            rate,
            ceiling,
            updated_on,
            cash,
            account.monthly_contribution().amount().to_string(),
            rates.pessimistic.as_fraction().to_string(),
            rates.neutral.as_fraction().to_string(),
            rates.optimistic.as_fraction().to_string(),
            account.is_archived(),
            reference_at,
        ],
    )
    .db()?;

    conn.execute("DELETE FROM positions WHERE account_id = ?1", [&id])
        .db()?;
    conn.execute("DELETE FROM pei_funds WHERE account_id = ?1", [&id])
        .db()?;
    match account.kind() {
        AccountKind::Securities(details) => {
            for (ordinal, position) in details.positions().iter().enumerate() {
                let asset = position.asset();
                conn.execute(
                    "INSERT INTO positions (account_id, ordinal, source, symbol, display_name,
                         currency, quantity, unit_cost)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        id,
                        ordinal as i64,
                        source_to_sql(asset.source()),
                        asset.symbol(),
                        asset.display_name(),
                        asset.currency().as_str(),
                        position.quantity().to_string(),
                        position.unit_cost().map(|c| c.amount().to_string()),
                    ],
                )
                .db()?;
            }
        }
        AccountKind::EmployeeSavings(details) => {
            for (ordinal, fund) in details.funds().iter().enumerate() {
                let reference = fund.reference();
                let anchor = fund.anchor();
                conn.execute(
                    "INSERT INTO pei_funds (id, account_id, ordinal, name, units, ref_source,
                         ref_symbol, ref_display_name, ref_currency, nav, nav_date, reference_price)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        fund.id().to_string(),
                        id,
                        ordinal as i64,
                        fund.name().as_str(),
                        fund.units().to_string(),
                        reference.map(|r| source_to_sql(r.source())),
                        reference.map(AssetRef::symbol),
                        reference.map(AssetRef::display_name),
                        reference.map(|r| r.currency().as_str()),
                        anchor.nav().amount().to_string(),
                        anchor.date().to_string(),
                        anchor.reference_price().map(|p| p.amount().to_string()),
                    ],
                )
                .db()?;
            }
        }
        AccountKind::Savings(_) | AccountKind::Checking(_) => {}
    }
    Ok(())
}

// --- Lecture ------------------------------------------------------------------------------

struct AccountRow {
    id: String,
    name: String,
    color: String,
    nature: String,
    product: Option<String>,
    balance: Option<String>,
    rate: Option<String>,
    deposit_ceiling: Option<String>,
    balance_updated_on: Option<String>,
    cash: Option<String>,
    reference_at: Option<String>,
    monthly_contribution: String,
    rates: [String; 3],
    archived: bool,
}

impl AccountRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            color: row.get("color")?,
            nature: row.get("nature")?,
            product: row.get("product")?,
            balance: row.get("balance")?,
            rate: row.get("rate")?,
            deposit_ceiling: row.get("deposit_ceiling")?,
            balance_updated_on: row.get("balance_updated_on")?,
            cash: row.get("cash")?,
            reference_at: row.get("reference_at")?,
            monthly_contribution: row.get("monthly_contribution")?,
            rates: [
                row.get("rate_pessimistic")?,
                row.get("rate_neutral")?,
                row.get("rate_optimistic")?,
            ],
            archived: row.get("archived")?,
        })
    }
}

fn load_accounts(conn: &Connection, only: Option<AccountId>) -> StorageResult<Vec<Account>> {
    let mut statement = conn
        .prepare("SELECT * FROM accounts WHERE ?1 IS NULL OR id = ?1 ORDER BY rowid")
        .db()?;
    let rows = statement
        .query_map([only.map(|id| id.to_string())], AccountRow::read)
        .db()?
        .collect::<rusqlite::Result<Vec<_>>>()
        .db()?;
    rows.into_iter()
        .map(|row| account_from_row(conn, row))
        .collect()
}

fn account_from_row(conn: &Connection, row: AccountRow) -> StorageResult<Account> {
    let required = |value: Option<String>, column: &str| {
        value.ok_or_else(|| corrupt(format!("colonne {column} vide pour le compte {}", row.id)))
    };
    let kind = match (row.nature.as_str(), row.product.as_deref()) {
        ("savings", Some(product)) => AccountKind::Savings(domain(SavingsDetails::new(
            match product {
                "livret_a" => SavingsProduct::LivretA,
                "ldds" => SavingsProduct::Ldds,
                other => return Err(corrupt(format!("livret inconnu « {other} »"))),
            },
            money(&required(row.balance.clone(), "balance")?)?,
            domain(Rate::from_fraction(decimal(&required(
                row.rate.clone(),
                "rate",
            )?)?))?,
            money(&required(row.deposit_ceiling.clone(), "deposit_ceiling")?)?,
            date(&required(
                row.balance_updated_on.clone(),
                "balance_updated_on",
            )?)?,
        ))?),
        ("securities", Some(product)) => AccountKind::Securities(domain(SecuritiesDetails::new(
            match product {
                "pea" => SecuritiesProduct::Pea,
                "crypto" => SecuritiesProduct::Crypto,
                other => return Err(corrupt(format!("compte-titres inconnu « {other} »"))),
            },
            money(&required(row.cash.clone(), "cash")?)?,
            load_positions(conn, &row.id)?,
        ))?),
        ("checking", _) => AccountKind::Checking(domain(CheckingDetails::new(
            Money::new(decimal(&required(row.balance.clone(), "balance")?)?),
            date(&required(
                row.balance_updated_on.clone(),
                "balance_updated_on",
            )?)?,
            datetime(&required(row.reference_at.clone(), "reference_at")?)?,
        ))?),
        ("employee_savings", _) => {
            AccountKind::EmployeeSavings(EmployeeSavingsDetails::new(load_funds(conn, &row.id)?))
        }
        (nature, _) => return Err(corrupt(format!("nature de compte inconnue « {nature} »"))),
    };
    let rate = |raw: &str| decimal(raw).and_then(|value| domain(Rate::from_fraction(value)));
    let mut account = Account::restore(
        AccountId::from_uuid(uuid(&row.id)?),
        domain(Label::new(&row.name))?,
        domain(Color::new(&row.color))?,
        kind,
        Money::ZERO,
        ScenarioRates {
            pessimistic: rate(&row.rates[0])?,
            neutral: rate(&row.rates[1])?,
            optimistic: rate(&row.rates[2])?,
        },
        row.archived,
    );
    domain(account.set_monthly_contribution(money(&row.monthly_contribution)?))?;
    Ok(account)
}

fn asset(
    source: &str,
    symbol: &str,
    display_name: &str,
    currency: &str,
) -> StorageResult<AssetRef> {
    domain(AssetRef::new(
        source_from_sql(source)?,
        symbol,
        display_name,
        domain(CurrencyCode::new(currency))?,
    ))
}

fn load_positions(conn: &Connection, account_id: &str) -> StorageResult<Vec<Position>> {
    let mut statement = conn
        .prepare(
            "SELECT source, symbol, display_name, currency, quantity, unit_cost
             FROM positions WHERE account_id = ?1 ORDER BY ordinal",
        )
        .db()?;
    let rows = statement
        .query_map([account_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .db()?
        .collect::<rusqlite::Result<Vec<_>>>()
        .db()?;
    rows.into_iter()
        .map(|(source, symbol, name, currency, quantity, unit_cost)| {
            domain(Position::new(
                asset(&source, &symbol, &name, &currency)?,
                decimal(&quantity)?,
                optional_money(unit_cost)?,
            ))
        })
        .collect()
}

struct FundRow {
    id: String,
    name: String,
    units: String,
    reference: Option<(String, String, String, String)>,
    nav: String,
    nav_date: String,
    reference_price: Option<String>,
}

fn load_funds(conn: &Connection, account_id: &str) -> StorageResult<Vec<PeiFund>> {
    let mut statement = conn
        .prepare("SELECT * FROM pei_funds WHERE account_id = ?1 ORDER BY ordinal")
        .db()?;
    let rows = statement
        .query_map([account_id], |row| {
            let source: Option<String> = row.get("ref_source")?;
            let reference = match source {
                Some(source) => Some((
                    source,
                    row.get("ref_symbol")?,
                    row.get("ref_display_name")?,
                    row.get("ref_currency")?,
                )),
                None => None,
            };
            Ok(FundRow {
                id: row.get("id")?,
                name: row.get("name")?,
                units: row.get("units")?,
                reference,
                nav: row.get("nav")?,
                nav_date: row.get("nav_date")?,
                reference_price: row.get("reference_price")?,
            })
        })
        .db()?
        .collect::<rusqlite::Result<Vec<_>>>()
        .db()?;
    rows.into_iter()
        .map(|row| {
            let reference = row
                .reference
                .map(|(source, symbol, name, currency)| asset(&source, &symbol, &name, &currency))
                .transpose()?;
            domain(PeiFund::new(
                FundId::from_uuid(uuid(&row.id)?),
                domain(Label::new(&row.name))?,
                decimal(&row.units)?,
                reference,
                domain(NavAnchor::new(
                    money(&row.nav)?,
                    date(&row.nav_date)?,
                    optional_money(row.reference_price)?,
                ))?,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc_domain::NaiveDate;
    use lc_domain::account::{AccountType, QuoteSource};
    use lc_domain::defaults;
    use rust_decimal_macros::dec;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()
    }

    fn open(account_type: AccountType) -> Account {
        Account::open(
            AccountId::generate(),
            Label::new(account_type.label()).unwrap(),
            defaults::color(account_type),
            account_type,
            today(),
        )
    }

    fn eur_asset(source: QuoteSource, symbol: &str, name: &str) -> AssetRef {
        AssetRef::new(source, symbol, name, CurrencyCode::eur()).unwrap()
    }

    fn repo() -> SqliteAccountRepository {
        SqliteAccountRepository::new(Database::open_in_memory().unwrap())
    }

    #[test]
    fn roundtrips_every_account_nature() {
        let repo = repo();
        let mut livret = open(AccountType::LivretA);
        livret
            .update_savings_balance(Money::new(dec!(10640.25)), today())
            .unwrap();
        livret
            .set_monthly_contribution(Money::new(dec!(200)))
            .unwrap();

        let mut wallet = open(AccountType::Crypto);
        wallet.set_cash(Money::new(dec!(12.5))).unwrap();
        wallet
            .upsert_position(
                Position::new(
                    eur_asset(QuoteSource::CoinGecko, "bitcoin", "Bitcoin"),
                    dec!(0.15),
                    Some(Money::new(dec!(40000.123456))),
                )
                .unwrap(),
            )
            .unwrap();
        wallet
            .upsert_position(
                Position::new(
                    eur_asset(QuoteSource::CoinGecko, "ethereum", ""),
                    dec!(2),
                    None,
                )
                .unwrap(),
            )
            .unwrap();

        let mut pei = open(AccountType::Pei);
        let usd = CurrencyCode::new("USD").unwrap();
        let reference =
            AssetRef::new(QuoteSource::TwelveData, "IWDA:Euronext", "iShares", usd).unwrap();
        pei.upsert_fund(
            PeiFund::new(
                FundId::generate(),
                Label::new("Amundi Actions Monde").unwrap(),
                dec!(120.5),
                Some(reference),
                NavAnchor::new(
                    Money::new(dec!(25.8012)),
                    today(),
                    Some(Money::new(dec!(104))),
                )
                .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        pei.upsert_fund(
            PeiFund::new(
                FundId::generate(),
                Label::new("Monétaire").unwrap(),
                dec!(10),
                None,
                NavAnchor::new(Money::new(dec!(10)), today(), None).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let mut closed = open(AccountType::Pea);
        closed.archive();

        for account in [&livret, &wallet, &pei, &closed] {
            repo.save(account).unwrap();
        }

        assert_eq!(
            repo.list().unwrap(),
            vec![livret, wallet.clone(), pei, closed]
        );
        assert_eq!(repo.get(wallet.id()).unwrap(), Some(wallet));
        assert_eq!(repo.get(AccountId::generate()).unwrap(), None);
    }

    #[test]
    fn saving_again_replaces_positions_and_fields() {
        let repo = repo();
        let mut pea = open(AccountType::Pea);
        let cw8 = eur_asset(QuoteSource::TwelveData, "CW8", "Amundi MSCI World");
        pea.upsert_position(Position::new(cw8.clone(), dec!(40), None).unwrap())
            .unwrap();
        repo.save(&pea).unwrap();

        pea.upsert_position(Position::new(cw8.clone(), dec!(42), None).unwrap())
            .unwrap();
        pea.rename(Label::new("PEA Bourso").unwrap());
        repo.save(&pea).unwrap();
        pea.remove_position(&cw8).unwrap();
        repo.save(&pea).unwrap();

        let stored = repo.get(pea.id()).unwrap().unwrap();
        assert_eq!(stored.name().as_str(), "PEA Bourso");
        assert_eq!(stored, pea);
        assert_eq!(repo.list().unwrap().len(), 1);
    }
}
