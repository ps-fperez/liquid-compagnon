use std::fmt;

use uuid::Uuid;

macro_rules! define_id {
    ($($(#[$meta:meta])* $name:ident),+ $(,)?) => {$(
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Uuid);

        impl $name {
            /// Nouvel identifiant aléatoire (UUID v4).
            pub fn generate() -> Self {
                Self(Uuid::new_v4())
            }

            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            pub const fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    )+};
}

define_id!(
    /// Identifiant d'un compte d'épargne.
    AccountId,
    /// Identifiant d'une catégorie de dépenses.
    CategoryId,
    /// Identifiant d'une charge fixe récurrente.
    ChargeId,
    /// Identifiant d'une dépense ponctuelle.
    ExpenseId,
    /// Identifiant d'un fonds de PEI.
    FundId,
    /// Identifiant d'un revenu ponctuel.
    IncomeId,
    /// Identifiant d'une entrée de salaire.
    SalaryId,
    /// Identifiant d'un snapshot de patrimoine.
    SnapshotId,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_are_unique() {
        assert_ne!(AccountId::generate(), AccountId::generate());
    }

    #[test]
    fn id_roundtrips_through_uuid() {
        let uuid = Uuid::new_v4();
        assert_eq!(*ExpenseId::from_uuid(uuid).as_uuid(), uuid);
        assert_eq!(ExpenseId::from_uuid(uuid).to_string(), uuid.to_string());
    }
}
