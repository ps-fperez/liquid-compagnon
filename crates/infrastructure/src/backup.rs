//! Fichier de sauvegarde JSON versionné, avec migration des exports plus anciens.

use lc_application::backup::{FullDataset, SCHEMA_VERSION};
use lc_application::ports::{BackupCodec, BackupError};
use serde_json::Value;

/// Transforme un export de version N en version N + 1.
type Migration = fn(&mut Value) -> Result<(), String>;

/// `MIGRATIONS[i]` fait passer un export de la version `i + 1` à la version `i + 2`.
/// Il en faut exactement `SCHEMA_VERSION - 1`.
const MIGRATIONS: &[Migration] = &[v1_to_v2];

/// Version 2 : comptes courants et rattachement des mouvements. Les nouveaux champs sont
/// facultatifs : un export v1 est valide tel quel (aucun compte courant, aucun rattachement).
fn v1_to_v2(_: &mut Value) -> Result<(), String> {
    Ok(())
}

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonBackupCodec;

impl BackupCodec for JsonBackupCodec {
    fn encode(&self, data: &FullDataset) -> Result<Vec<u8>, BackupError> {
        serde_json::to_vec_pretty(data).map_err(|error| BackupError::Invalid(error.to_string()))
    }

    fn decode(&self, bytes: &[u8]) -> Result<FullDataset, BackupError> {
        let value: Value = serde_json::from_slice(bytes)
            .map_err(|error| BackupError::Invalid(format!("JSON illisible ({error})")))?;
        let upgraded = upgrade(value, SCHEMA_VERSION, MIGRATIONS)?;
        serde_json::from_value(upgraded)
            .map_err(|error| BackupError::Invalid(format!("contenu inattendu ({error})")))
    }
}

/// Amène un export à la version `target` en appliquant les migrations successives.
fn upgrade(mut value: Value, target: u32, migrations: &[Migration]) -> Result<Value, BackupError> {
    let version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .filter(|version| *version > 0)
        .ok_or_else(|| {
            BackupError::Invalid("ce fichier n'est pas un export Liquid Compagnon".into())
        })?;
    if version > target {
        return Err(BackupError::NewerVersion {
            found: version,
            supported: target,
        });
    }
    for from in version..target {
        let migration = migrations.get(from as usize - 1).ok_or_else(|| {
            BackupError::Invalid(format!("aucune migration depuis la version {from}"))
        })?;
        migration(&mut value).map_err(|reason| {
            BackupError::Invalid(format!("migration v{from} → v{} : {reason}", from + 1))
        })?;
        value["schema_version"] = Value::from(from + 1);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lc_application::ports::Dataset;
    use lc_domain::account::{Account, AccountType};
    use lc_domain::{AccountId, Label, defaults};
    use serde_json::json;

    fn sample() -> FullDataset {
        let today = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        let account = Account::open(
            AccountId::generate(),
            Label::new("PEA").unwrap(),
            defaults::color(AccountType::Pea),
            AccountType::Pea,
            today,
        );
        let dataset = Dataset {
            accounts: vec![account],
            ..Dataset::default()
        };
        FullDataset::from_dataset(&dataset, today.and_hms_opt(9, 42, 0).unwrap().and_utc())
    }

    #[test]
    fn migration_table_matches_schema_version() {
        assert_eq!(MIGRATIONS.len() + 1, SCHEMA_VERSION as usize);
    }

    #[test]
    fn encode_then_decode_roundtrips() {
        let codec = JsonBackupCodec;
        let original = sample();

        let bytes = codec.encode(&original).unwrap();

        assert!(String::from_utf8_lossy(&bytes).contains("\"schema_version\": 2"));
        assert_eq!(codec.decode(&bytes).unwrap(), original);
    }

    // Spec checking-account : « Ancien fichier »
    #[test]
    fn version_1_exports_are_still_readable() {
        let mut v1 = serde_json::to_value(sample()).unwrap();
        v1["schema_version"] = json!(1);
        v1["expenses"] = json!([{
            "id": "6f1c1a4e-8d0e-4a7b-9d43-2a2f0c6f0b11",
            "amount": "42.50",
            "category_id": "0b8f2d0e-1c3a-4e5f-8a9b-7c6d5e4f3a21",
            "date": "2026-10-20",
            "label": "Courses"
        }]);

        let decoded = JsonBackupCodec
            .decode(&serde_json::to_vec(&v1).unwrap())
            .unwrap();

        assert_eq!(decoded.schema_version, SCHEMA_VERSION);
        assert_eq!(decoded.expenses[0].account_id, None);
        assert_eq!(decoded.expenses[0].recorded_at, None);
    }

    // Spec data-backup : « Fichier invalide »
    #[test]
    fn rejects_garbage_and_foreign_json() {
        let codec = JsonBackupCodec;
        assert!(matches!(
            codec.decode(b"pas du json"),
            Err(BackupError::Invalid(_))
        ));
        assert!(matches!(
            codec.decode(br#"{"hello": "world"}"#),
            Err(BackupError::Invalid(_))
        ));
        assert!(matches!(
            codec.decode(br#"{"schema_version": 0}"#),
            Err(BackupError::Invalid(_))
        ));
    }

    // Spec data-backup : « Version plus récente »
    #[test]
    fn rejects_newer_versions() {
        let mut value = serde_json::to_value(sample()).unwrap();
        value["schema_version"] = json!(SCHEMA_VERSION + 1);
        let bytes = serde_json::to_vec(&value).unwrap();

        assert_eq!(
            JsonBackupCodec.decode(&bytes),
            Err(BackupError::NewerVersion {
                found: SCHEMA_VERSION + 1,
                supported: SCHEMA_VERSION
            })
        );
    }

    // Spec data-backup : « Ancien export » (mécanisme de migration en chaîne)
    #[test]
    fn upgrades_old_exports_step_by_step() {
        fn rename_comptes(value: &mut Value) -> Result<(), String> {
            let accounts = value
                .as_object_mut()
                .unwrap()
                .remove("comptes")
                .ok_or("champ comptes absent")?;
            value["accounts"] = accounts;
            Ok(())
        }
        fn add_snapshots(value: &mut Value) -> Result<(), String> {
            value["snapshots"] = json!([]);
            Ok(())
        }
        let v1 = json!({ "schema_version": 1, "comptes": [1, 2] });

        let v3 = upgrade(v1, 3, &[rename_comptes, add_snapshots]).unwrap();

        assert_eq!(
            v3,
            json!({ "schema_version": 3, "accounts": [1, 2], "snapshots": [] })
        );
        assert!(
            matches!(upgrade(json!({ "schema_version": 1 }), 3, &[rename_comptes, add_snapshots]), Err(BackupError::Invalid(reason)) if reason.contains("v1 → v2"))
        );
    }
}
