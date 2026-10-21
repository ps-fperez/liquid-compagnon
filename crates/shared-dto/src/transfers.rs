use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Côté d'un virement : un compte suivi ou un compte hors patrimoine.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum EndpointDto {
    Account(String),
    External(String),
}

/// Fréquence d'un virement. Le jour est ramené au dernier jour d'un mois plus court.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScheduleDto {
    Once {
        date: NaiveDate,
    },
    Monthly {
        day: u8,
        /// `AAAA-MM`
        from: String,
        until: Option<String>,
    },
    Yearly {
        month: u32,
        day: u8,
        from_year: i32,
        until_year: Option<i32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferDto {
    pub id: String,
    pub from: EndpointDto,
    pub to: EndpointDto,
    pub from_name: String,
    pub to_name: String,
    pub sent: Decimal,
    pub received: Decimal,
    pub label: Option<String>,
    pub schedule: ScheduleDto,
}

/// Création (`id` absent) ou modification d'un virement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferInput {
    pub id: Option<String>,
    pub from: EndpointDto,
    pub to: EndpointDto,
    pub sent: Decimal,
    /// Montant crédité à l'arrivée ; égal au montant envoyé si absent.
    pub received: Option<Decimal>,
    pub label: Option<String>,
    pub schedule: ScheduleDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalAccountDto {
    pub id: String,
    pub name: String,
    /// Utilisé par au moins un virement (suppression impossible).
    pub used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameInput {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameInput {
    pub id: String,
    pub name: String,
}
