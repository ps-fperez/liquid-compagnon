use lc_application::ports::{PriceCache, StorageResult};
use lc_domain::account::{AssetRef, CurrencyCode};
use lc_domain::valuation::{Quote, QuoteFreshness};
use rusqlite::{OptionalExtension, params};

use super::{Database, SqlResultExt, datetime, domain, money, source_to_sql};

/// Derniers cours connus, une ligne par actif. Un cours relu est toujours marqué « en cache ».
pub struct SqlitePriceCache {
    db: Database,
}

impl SqlitePriceCache {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl PriceCache for SqlitePriceCache {
    fn get(&self, asset: &AssetRef) -> StorageResult<Option<Quote>> {
        self.db.read(|conn| {
            let row = conn
                .query_row(
                    "SELECT display_name, currency, price, as_of FROM price_cache
                     WHERE source = ?1 AND symbol = ?2",
                    params![source_to_sql(asset.source()), asset.symbol()],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        ))
                    },
                )
                .optional()
                .db()?;
            row.map(|(name, currency, price, as_of)| {
                let stored = domain(AssetRef::new(
                    asset.source(),
                    asset.symbol(),
                    &name,
                    domain(CurrencyCode::new(&currency))?,
                ))?;
                Ok(Quote::new(
                    stored,
                    money(&price)?,
                    datetime(&as_of)?,
                    QuoteFreshness::Cached,
                ))
            })
            .transpose()
        })
    }

    fn put(&self, quote: &Quote) -> StorageResult<()> {
        let asset = quote.asset();
        self.db.write(|tx| {
            tx.execute(
                "INSERT INTO price_cache (source, symbol, display_name, currency, price, as_of)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(source, symbol) DO UPDATE SET display_name = excluded.display_name,
                     currency = excluded.currency, price = excluded.price, as_of = excluded.as_of",
                params![
                    source_to_sql(asset.source()),
                    asset.symbol(),
                    asset.display_name(),
                    asset.currency().as_str(),
                    quote.price().amount().to_string(),
                    quote.as_of().to_rfc3339(),
                ],
            )
            .db()
            .map(drop)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc_domain::Money;
    use lc_domain::NaiveDate;
    use lc_domain::account::QuoteSource;
    use rust_decimal_macros::dec;

    #[test]
    fn stores_latest_quote_and_reads_it_back_as_cached() {
        let cache = SqlitePriceCache::new(Database::open_in_memory().unwrap());
        let btc = AssetRef::new(
            QuoteSource::CoinGecko,
            "bitcoin",
            "Bitcoin",
            CurrencyCode::eur(),
        )
        .unwrap();
        let at = |hour| {
            NaiveDate::from_ymd_opt(2026, 10, 20)
                .unwrap()
                .and_hms_opt(hour, 42, 0)
                .unwrap()
                .and_utc()
        };
        assert_eq!(cache.get(&btc).unwrap(), None);

        cache
            .put(&Quote::new(
                btc.clone(),
                Money::new(dec!(59000)),
                at(8),
                QuoteFreshness::Live,
            ))
            .unwrap();
        cache
            .put(&Quote::new(
                btc.clone(),
                Money::new(dec!(60000.12)),
                at(9),
                QuoteFreshness::Live,
            ))
            .unwrap();

        let cached = cache.get(&btc).unwrap().unwrap();
        assert_eq!(cached.price(), Money::new(dec!(60000.12)));
        assert_eq!(cached.as_of(), at(9));
        assert_eq!(cached.freshness(), QuoteFreshness::Cached);
        assert_eq!(cached.asset().display_name(), "Bitcoin");
    }
}
