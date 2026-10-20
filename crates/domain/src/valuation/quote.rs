use chrono::{DateTime, Utc};

use crate::Money;
use crate::account::AssetRef;

/// Fraîcheur d'un cours au moment où il est utilisé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuoteFreshness {
    /// Obtenu à l'instant auprès de la source de cotation.
    Live,
    /// Relu depuis le cache local (source injoignable ou cache encore valide).
    Cached,
}

/// Cours d'un actif, converti en euros.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quote {
    asset: AssetRef,
    price: Money,
    as_of: DateTime<Utc>,
    freshness: QuoteFreshness,
}

impl Quote {
    pub fn new(
        asset: AssetRef,
        price: Money,
        as_of: DateTime<Utc>,
        freshness: QuoteFreshness,
    ) -> Self {
        Self {
            asset,
            price,
            as_of,
            freshness,
        }
    }

    pub fn asset(&self) -> &AssetRef {
        &self.asset
    }

    pub fn price(&self) -> Money {
        self.price
    }

    pub fn as_of(&self) -> DateTime<Utc> {
        self.as_of
    }

    pub fn freshness(&self) -> QuoteFreshness {
        self.freshness
    }

    /// Le même cours, requalifié (ex. un cours relu du cache est `Cached`).
    pub fn with_freshness(self, freshness: QuoteFreshness) -> Self {
        Self { freshness, ..self }
    }

    fn provenance(&self) -> Provenance {
        match self.freshness {
            QuoteFreshness::Live => Provenance::Live,
            QuoteFreshness::Cached => Provenance::Cached { as_of: self.as_of },
        }
    }
}

/// Ensemble des cours disponibles pour une valorisation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PriceBook {
    quotes: Vec<Quote>,
}

impl PriceBook {
    pub fn new(quotes: impl IntoIterator<Item = Quote>) -> Self {
        let mut book = Self::default();
        for quote in quotes {
            book.insert(quote);
        }
        book
    }

    /// Ajoute un cours, ou remplace celui du même actif.
    pub fn insert(&mut self, quote: Quote) {
        match self
            .quotes
            .iter_mut()
            .find(|q| q.asset.is_same_asset(&quote.asset))
        {
            Some(existing) => *existing = quote,
            None => self.quotes.push(quote),
        }
    }

    pub fn get(&self, asset: &AssetRef) -> Option<&Quote> {
        self.quotes.iter().find(|q| q.asset.is_same_asset(asset))
    }

    pub(super) fn provenance_of(&self, asset: &AssetRef) -> Option<Provenance> {
        self.get(asset).map(Quote::provenance)
    }
}

/// Provenance d'une valeur affichée, de la plus fiable à la moins fiable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// Cours obtenu à l'instant.
    Live,
    /// Valeur saisie par l'utilisateur (solde de livret, valeur de part réelle).
    Manual,
    /// Dernier cours connu, avec sa date.
    Cached { as_of: DateTime<Utc> },
    /// Valeur de part PEI extrapolée par l'actif de référence.
    Estimated,
    /// Aucun cours : position valorisée à son prix de revient.
    CostBasis,
    /// Aucun cours ni prix de revient : valeur inconnue (comptée pour 0).
    Unavailable,
}

impl Provenance {
    fn rank(&self) -> u8 {
        match self {
            Provenance::Live => 0,
            Provenance::Manual => 1,
            Provenance::Cached { .. } => 2,
            Provenance::Estimated => 3,
            Provenance::CostBasis => 4,
            Provenance::Unavailable => 5,
        }
    }

    /// Provenance d'un agrégat : la moins fiable des deux (pour deux cours en cache, le plus
    /// ancien).
    pub fn least_reliable(self, other: Provenance) -> Provenance {
        match (self, other) {
            (Provenance::Cached { as_of: a }, Provenance::Cached { as_of: b }) => {
                Provenance::Cached { as_of: a.min(b) }
            }
            _ if other.rank() > self.rank() => other,
            _ => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{CurrencyCode, QuoteSource};
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn at(day: u32, hour: u32) -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(2026, 10, day)
            .unwrap()
            .and_hms_opt(hour, 0, 0)
            .unwrap()
            .and_utc()
    }

    fn btc() -> AssetRef {
        AssetRef::new(QuoteSource::CoinGecko, "bitcoin", "", CurrencyCode::eur()).unwrap()
    }

    #[test]
    fn price_book_replaces_quote_of_same_asset() {
        let mut book = PriceBook::default();
        book.insert(Quote::new(
            btc(),
            Money::new(dec!(59000)),
            at(19, 9),
            QuoteFreshness::Cached,
        ));
        book.insert(Quote::new(
            btc(),
            Money::new(dec!(60000)),
            at(20, 9),
            QuoteFreshness::Live,
        ));

        assert_eq!(book.get(&btc()).unwrap().price(), Money::new(dec!(60000)));
        assert_eq!(book.provenance_of(&btc()), Some(Provenance::Live));
    }

    #[test]
    fn least_reliable_provenance_wins() {
        assert_eq!(
            Provenance::Live.least_reliable(Provenance::Estimated),
            Provenance::Estimated
        );
        assert_eq!(
            Provenance::CostBasis.least_reliable(Provenance::Manual),
            Provenance::CostBasis
        );
        assert_eq!(
            Provenance::Cached { as_of: at(20, 9) }
                .least_reliable(Provenance::Cached { as_of: at(18, 9) }),
            Provenance::Cached { as_of: at(18, 9) }
        );
    }
}
