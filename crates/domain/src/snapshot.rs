//! Snapshots mensuels du patrimoine : base de la courbe d'évolution et des variations.

use chrono::NaiveDate;

use crate::account::Account;
use crate::{AccountId, Money, Month, SnapshotId};

/// Origine d'un snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnapshotKind {
    /// Pris à la première ouverture de l'application dans le mois (un seul par mois).
    Auto,
    /// Demandé explicitement par l'utilisateur.
    Manual,
}

/// Valeur d'un compte au moment du snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotLine {
    pub account: AccountId,
    pub value: Money,
}

/// Photographie de la valeur des comptes à une date. Une fois pris, un snapshot ne change plus :
/// archiver ou supprimer un compte ensuite ne modifie pas l'historique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    id: SnapshotId,
    month: Month,
    taken_on: NaiveDate,
    kind: SnapshotKind,
    lines: Vec<SnapshotLine>,
    total: Money,
}

impl Snapshot {
    /// Prend un snapshot à partir des comptes et de leur valeur actuelle. Les comptes archivés
    /// sont exclus (ils ne font plus partie du patrimoine courant).
    pub fn capture<'a>(
        id: SnapshotId,
        taken_on: NaiveDate,
        kind: SnapshotKind,
        valued_accounts: impl IntoIterator<Item = (&'a Account, Money)>,
    ) -> Self {
        let lines = valued_accounts
            .into_iter()
            .filter(|(account, _)| !account.is_archived())
            .map(|(account, value)| SnapshotLine {
                account: account.id(),
                value,
            })
            .collect();
        Self::restore(id, Month::of(taken_on), taken_on, kind, lines)
    }

    /// Reconstitue un snapshot persisté ; le total est recalculé à partir des lignes.
    pub fn restore(
        id: SnapshotId,
        month: Month,
        taken_on: NaiveDate,
        kind: SnapshotKind,
        lines: Vec<SnapshotLine>,
    ) -> Self {
        let total = lines.iter().map(|line| line.value).sum();
        Self {
            id,
            month,
            taken_on,
            kind,
            lines,
            total,
        }
    }

    /// Vrai si aucun snapshot automatique n'existe encore pour `month`.
    pub fn is_auto_due(existing: &[Snapshot], month: Month) -> bool {
        !existing
            .iter()
            .any(|snapshot| snapshot.kind == SnapshotKind::Auto && snapshot.month == month)
    }

    pub fn id(&self) -> SnapshotId {
        self.id
    }

    pub fn month(&self) -> Month {
        self.month
    }

    pub fn taken_on(&self) -> NaiveDate {
        self.taken_on
    }

    pub fn kind(&self) -> SnapshotKind {
        self.kind
    }

    pub fn lines(&self) -> &[SnapshotLine] {
        &self.lines
    }

    pub fn total(&self) -> Money {
        self.total
    }

    pub fn value_of(&self, account: AccountId) -> Option<Money> {
        self.lines
            .iter()
            .find(|line| line.account == account)
            .map(|line| line.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::AccountType;
    use crate::{Label, defaults};
    use rust_decimal_macros::dec;

    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    fn account(account_type: AccountType) -> Account {
        Account::open(
            AccountId::generate(),
            Label::new(account_type.label()).unwrap(),
            defaults::color(account_type),
            account_type,
            date(10, 1),
        )
    }

    #[test]
    fn capture_sums_values_and_excludes_archived_accounts() {
        let livret = account(AccountType::LivretA);
        let pea = account(AccountType::Pea);
        let mut closed = account(AccountType::Crypto);
        closed.archive();

        let snapshot = Snapshot::capture(
            SnapshotId::generate(),
            date(11, 3),
            SnapshotKind::Auto,
            [
                (&livret, Money::new(dec!(10640))),
                (&pea, Money::new(dec!(14510))),
                (&closed, Money::new(dec!(8702))),
            ],
        );

        assert_eq!(snapshot.month(), Month::new(2026, 11).unwrap());
        assert_eq!(snapshot.total(), Money::new(dec!(25150)));
        assert_eq!(snapshot.value_of(pea.id()), Some(Money::new(dec!(14510))));
        assert_eq!(snapshot.value_of(closed.id()), None);
    }

    #[test]
    fn only_one_auto_snapshot_per_month() {
        let november = Month::new(2026, 11).unwrap();
        let manual = Snapshot::restore(
            SnapshotId::generate(),
            november,
            date(11, 2),
            SnapshotKind::Manual,
            vec![],
        );
        assert!(Snapshot::is_auto_due(
            std::slice::from_ref(&manual),
            november
        ));

        let auto = Snapshot::restore(
            SnapshotId::generate(),
            november,
            date(11, 3),
            SnapshotKind::Auto,
            vec![],
        );
        assert!(!Snapshot::is_auto_due(&[manual, auto.clone()], november));
        assert!(Snapshot::is_auto_due(&[auto], november.next()));
    }

    #[test]
    fn archiving_later_does_not_change_history() {
        let mut crypto = account(AccountType::Crypto);
        let snapshot = Snapshot::capture(
            SnapshotId::generate(),
            date(10, 1),
            SnapshotKind::Auto,
            [(&crypto, Money::new(dec!(9000)))],
        );

        crypto.archive();

        assert_eq!(snapshot.value_of(crypto.id()), Some(Money::new(dec!(9000))));
        assert_eq!(snapshot.total(), Money::new(dec!(9000)));
    }
}
