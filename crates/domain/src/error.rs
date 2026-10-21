/// Violation d'une règle métier. Les messages sont destinés à l'utilisateur final.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    #[error("Saisissez un montant positif")]
    NonPositiveAmount,
    #[error("Le montant ne peut pas être négatif")]
    NegativeAmount,
    #[error("Un montant a au plus 2 décimales")]
    TooManyDecimals,
    #[error("Le libellé est obligatoire")]
    EmptyLabel,
    #[error("Le libellé dépasse {max} caractères")]
    LabelTooLong { max: usize },
    #[error("Mois invalide : {year}-{month:02}")]
    InvalidMonth { year: i32, month: u32 },
    #[error("La date de fin précède la date de début")]
    EndBeforeStart,
    #[error("Couleur invalide : {0} (format attendu #RRGGBB)")]
    InvalidColor(String),
    #[error("Code devise invalide : {0}")]
    InvalidCurrency(String),
    #[error("Le symbole de l'actif est obligatoire")]
    InvalidSymbol,
    #[error("La quantité doit être strictement positive")]
    NonPositiveQuantity,
    #[error("Un taux doit être supérieur à -100 %")]
    RateOutOfRange,
    #[error("Opération impossible pour ce type de compte")]
    WrongAccountKind,
    #[error("Fonds introuvable dans ce PEI")]
    UnknownFund,
    #[error("Position introuvable dans ce compte")]
    UnknownPosition,
    #[error("Choisissez deux comptes différents")]
    SameTransferEndpoints,
    #[error("Un virement doit partir d'un de vos comptes ou y arriver")]
    TransferWithoutTrackedAccount,
    #[error("Jour du mois invalide (1 à 31)")]
    InvalidDayOfMonth,
    #[error("Cette valeur de part est plus ancienne que la dernière saisie")]
    OutdatedNav,
}
