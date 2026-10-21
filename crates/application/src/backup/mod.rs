//! Sauvegarde : export JSON complet et réimport validé (remplacement de toutes les données).

mod records;

use std::sync::Arc;

use chrono::Duration;

pub use records::*;

use crate::AppResult;
use crate::flows::convert_scheduled_contributions;
use crate::ports::{
    AccountRepository, BackupCodec, BudgetRepository, Clock, Dataset, DatasetStore,
    PlanningRepository, SettingsStore, SnapshotRepository, TransferRepository,
};

/// Version du format d'export produit par cette version de l'application.
pub const SCHEMA_VERSION: u32 = 4;

/// Résumé d'un import réussi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportSummary {
    pub accounts: usize,
    pub expenses: usize,
    pub snapshots: usize,
}

pub struct BackupUseCases {
    accounts: Arc<dyn AccountRepository>,
    budget: Arc<dyn BudgetRepository>,
    transfers: Arc<dyn TransferRepository>,
    planning: Arc<dyn PlanningRepository>,
    snapshots: Arc<dyn SnapshotRepository>,
    settings: Arc<dyn SettingsStore>,
    store: Arc<dyn DatasetStore>,
    codec: Arc<dyn BackupCodec>,
    clock: Arc<dyn Clock>,
}

impl BackupUseCases {
    /// Délai au-delà duquel un rappel d'export est affiché.
    pub const EXPORT_REMINDER_DAYS: i64 = 30;

    // Un dépôt par famille de données exportées : les regrouper masquerait les dépendances.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        budget: Arc<dyn BudgetRepository>,
        transfers: Arc<dyn TransferRepository>,
        planning: Arc<dyn PlanningRepository>,
        snapshots: Arc<dyn SnapshotRepository>,
        settings: Arc<dyn SettingsStore>,
        store: Arc<dyn DatasetStore>,
        codec: Arc<dyn BackupCodec>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            accounts,
            budget,
            transfers,
            planning,
            snapshots,
            settings,
            store,
            codec,
            clock,
        }
    }

    /// Nom de fichier proposé : `liquid-compagnon-AAAA-MM-JJ.json`.
    pub fn suggested_file_name(&self) -> String {
        format!(
            "liquid-compagnon-{}.json",
            self.clock.today().format("%Y-%m-%d")
        )
    }

    /// Exporte toutes les données métier. Le hash du PIN et la clé API n'en font pas partie.
    pub fn export(&self) -> AppResult<Vec<u8>> {
        let dataset = Dataset {
            categories: self.budget.categories()?,
            salaries: self.budget.salaries()?,
            one_off_incomes: self.budget.one_off_incomes()?,
            charges: self.budget.charges()?,
            expenses: self.budget.expenses()?,
            accounts: self.accounts.list()?,
            snapshots: self.snapshots.list()?,
            transfers: self.transfers.transfers()?,
            external_accounts: self.transfers.external_accounts()?,
            goals: self.planning.goals()?,
            reviews: self.planning.reviews()?,
        };
        let now = self.clock.now();
        let bytes = self
            .codec
            .encode(&FullDataset::from_dataset(&dataset, now))?;
        self.settings.set_last_export_at(now)?;
        Ok(bytes)
    }

    /// Importe un export : le fichier est décodé et entièrement validé avant toute écriture,
    /// puis remplace toutes les données en une transaction. En cas d'erreur, rien ne change.
    pub fn import(&self, bytes: &[u8]) -> AppResult<ImportSummary> {
        let dataset = self.codec.decode(bytes)?.into_dataset()?;
        self.store.replace_all(&dataset)?;
        // Les versements programmés d'un ancien export deviennent des virements mensuels.
        convert_scheduled_contributions(
            self.accounts.as_ref(),
            self.transfers.as_ref(),
            self.clock.today(),
            self.clock.now(),
        )?;
        Ok(ImportSummary {
            accounts: dataset.accounts.len(),
            expenses: dataset.expenses.len(),
            snapshots: dataset.snapshots.len(),
        })
    }

    pub fn last_export_at(&self) -> AppResult<Option<chrono::DateTime<chrono::Utc>>> {
        Ok(self.settings.last_export_at()?)
    }

    /// Vrai si des comptes existent et qu'aucun export n'a eu lieu depuis 30 jours.
    pub fn export_reminder_due(&self) -> AppResult<bool> {
        if self.accounts.list()?.is_empty() {
            return Ok(false);
        }
        let due = match self.settings.last_export_at()? {
            Some(at) => self.clock.now() - at > Duration::days(Self::EXPORT_REMINDER_DAYS),
            None => true,
        };
        Ok(due)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppError;
    use crate::accounts::{FundInput, NewAccount};
    use crate::budget::{ChargeInput, ExpenseInput};
    use crate::testing::{TestEnv, asset};
    use crate::transfers::TransferInput;
    use lc_domain::account::{AccountType, QuoteSource};
    use lc_domain::budget::Debit;
    use lc_domain::budget::Periodicity;
    use lc_domain::transfer::{Endpoint, Schedule};
    use lc_domain::{Money, Month};
    use pollster::block_on;
    use rust_decimal_macros::dec;

    /// Remplit l'environnement avec un jeu de données couvrant tous les types d'entités.
    fn populate(env: &TestEnv) {
        let budget = env.budget();
        budget.ensure_default_categories().unwrap();
        let category = budget.categories(false).unwrap()[0].id();
        budget
            .upsert_salary(None, Month::new(2026, 1).unwrap(), Money::new(dec!(3200)))
            .unwrap();
        budget
            .upsert_one_off_income(
                None,
                "Prime",
                Money::new(dec!(1500)),
                env.clock.today(),
                None,
            )
            .unwrap();
        budget
            .upsert_charge(
                None,
                ChargeInput {
                    label: "Assurance".into(),
                    amount: Money::new(dec!(240)),
                    category,
                    periodicity: Periodicity::Yearly,
                    start: Month::new(2026, 1).unwrap(),
                    end: Some(Month::new(2027, 12).unwrap()),
                    debit: None,
                },
            )
            .unwrap();
        budget
            .add_expense(ExpenseInput {
                amount: Money::new(dec!(42.50)),
                category,
                date: None,
                label: Some("Courses".into()),
                account: None,
            })
            .unwrap();

        let accounts = env.accounts();
        let new = |name: &str, account_type| NewAccount {
            name: name.into(),
            account_type,
            color: None,
        };
        let livret = accounts
            .create_account(new("Livret A", AccountType::LivretA))
            .unwrap();
        accounts
            .update_balance(livret, Money::new(dec!(10640)))
            .unwrap();
        let wallet = accounts
            .create_account(new("Crypto", AccountType::Crypto))
            .unwrap();
        accounts
            .upsert_position(
                wallet,
                asset(QuoteSource::CoinGecko, "bitcoin"),
                dec!(0.15),
                Some(Money::new(dec!(40000))),
            )
            .unwrap();
        let pei = accounts
            .create_account(new("PEI", AccountType::Pei))
            .unwrap();
        env.twelve_data.set_price("IWDA", dec!(100));
        let fund = FundInput {
            name: "Monde".into(),
            units: dec!(120),
            reference: Some(asset(QuoteSource::TwelveData, "IWDA")),
        };
        block_on(accounts.create_pei_fund(pei, fund, Money::new(dec!(25)))).unwrap();
        // Compte courant avec une dépense et un revenu rattachés (format v2).
        let checking = accounts
            .create_account(new("Compte courant", AccountType::Checking))
            .unwrap();
        accounts
            .update_balance(checking, Money::new(dec!(-85.40)))
            .unwrap();
        env.clock.advance(chrono::Duration::minutes(1));
        budget
            .add_expense(ExpenseInput {
                amount: Money::new(dec!(12)),
                category,
                date: None,
                label: Some("Boulangerie".into()),
                account: Some(checking),
            })
            .unwrap();
        budget
            .upsert_one_off_income(
                None,
                "Remboursement",
                Money::new(dec!(30)),
                env.clock.today(),
                Some(checking),
            )
            .unwrap();
        // Format v3 : virements mensuel, ponctuel vers l'extérieur, annuel abondé ; charge
        // prélevée sur le compte courant.
        let transfers = env.transfers();
        let joint = transfers.create_external_account("Compte joint").unwrap();
        let monthly = |day| Schedule::Monthly {
            day,
            from: Month::of(env.clock.today()),
            until: None,
        };
        for input in [
            TransferInput {
                from: Endpoint::Account(checking),
                to: Endpoint::Account(livret),
                sent: Money::new(dec!(850)),
                received: None,
                label: None,
                schedule: monthly(1),
            },
            TransferInput {
                from: Endpoint::Account(livret),
                to: Endpoint::External(joint),
                sent: Money::new(dec!(600)),
                received: None,
                label: Some("Vacances".into()),
                schedule: Schedule::Once {
                    date: env.clock.today(),
                },
            },
            TransferInput {
                from: Endpoint::Account(checking),
                to: Endpoint::Account(pei),
                sent: Money::new(dec!(500)),
                received: Some(Money::new(dec!(1493.30))),
                label: None,
                schedule: Schedule::Yearly {
                    month: 1,
                    day: 15,
                    from_year: 2027,
                    until_year: None,
                },
            },
        ] {
            transfers.create_transfer(input).unwrap();
        }
        budget
            .upsert_charge(
                None,
                ChargeInput {
                    label: "Sosh".into(),
                    amount: Money::new(dec!(15.99)),
                    category,
                    periodicity: Periodicity::Monthly,
                    start: Month::new(2026, 1).unwrap(),
                    end: None,
                    debit: Some(Debit {
                        account: checking,
                        day: 5,
                    }),
                },
            )
            .unwrap();
        let closed = accounts
            .create_account(new("Ancien PEA", AccountType::Pea))
            .unwrap();
        block_on(env.dashboard().dashboard(false)).unwrap(); // snapshot automatique
        env.goals()
            .create_goal(crate::goals::GoalInput {
                name: "Apport immobilier".into(),
                target: Money::new(dec!(30000)),
                deadline: Some(Month::new(2028, 12).unwrap()),
                accounts: vec![checking],
            })
            .unwrap();
        let reviews = env.reviews();
        reviews.open_review(Month::new(2026, 9).unwrap()).unwrap();
        reviews.start_month(Money::new(dec!(250))).unwrap();
        accounts.archive_account(closed).unwrap();
    }

    // Spec data-backup : « Export réussi » et « Import valide »
    #[test]
    fn export_then_import_restores_identical_data() {
        let source = TestEnv::new();
        populate(&source);
        let bytes = source.backup().export().unwrap();

        let target = TestEnv::new();
        let summary = target.backup().import(&bytes).unwrap();

        assert_eq!(
            summary,
            ImportSummary {
                accounts: 5,
                expenses: 2,
                snapshots: 1
            }
        );
        assert_eq!(
            target.accounts.list().unwrap(),
            source.accounts.list().unwrap()
        );
        assert_eq!(
            target.budget_repo.expenses().unwrap(),
            source.budget_repo.expenses().unwrap()
        );
        assert_eq!(
            target.budget_repo.charges().unwrap(),
            source.budget_repo.charges().unwrap()
        );
        assert_eq!(
            target.budget_repo.categories().unwrap(),
            source.budget_repo.categories().unwrap()
        );
        assert_eq!(
            target.snapshots.list().unwrap(),
            source.snapshots.list().unwrap()
        );
        // Spec generic-accounts : « Aller-retour v3 »
        assert_eq!(
            target.planning_repo.goals().unwrap(),
            source.planning_repo.goals().unwrap()
        );
        assert_eq!(
            target.planning_repo.reviews().unwrap(),
            source.planning_repo.reviews().unwrap()
        );
        assert!(!target.planning_repo.reviews().unwrap().is_empty());
        assert_eq!(
            target.transfers_repo.transfers().unwrap(),
            source.transfers_repo.transfers().unwrap()
        );
        assert_eq!(
            target.transfers_repo.external_accounts().unwrap(),
            source.transfers_repo.external_accounts().unwrap()
        );
        let values = |env: &TestEnv| {
            block_on(env.accounts().valued_accounts(true))
                .unwrap()
                .accounts
                .into_iter()
                .map(|(account, valuation)| (account.id(), valuation.value))
                .collect::<Vec<_>>()
        };
        assert_eq!(values(&target), values(&source));
    }

    #[test]
    fn export_excludes_secrets_and_records_date() {
        let env = TestEnv::new();
        env.security().setup_pin("123456", "123456").unwrap();
        env.security()
            .set_twelve_data_api_key(Some("secret-key"))
            .unwrap();
        populate(&env);

        let json = String::from_utf8(env.backup().export().unwrap()).unwrap();

        assert!(!json.contains("secret-key"));
        assert!(!json.contains(&env.settings.pin_hash().unwrap().unwrap()));
        assert!(json.contains("\"schema_version\":4"));
        assert_eq!(
            env.settings.last_export_at().unwrap(),
            Some(env.clock.now())
        );
        assert_eq!(
            env.backup().suggested_file_name(),
            "liquid-compagnon-2026-10-20.json"
        );
    }

    // Spec data-backup : « Fichier invalide »
    #[test]
    fn invalid_file_changes_nothing() {
        let env = TestEnv::new();
        populate(&env);
        let before = env.accounts.list().unwrap();

        let result = env.backup().import(b"{ pas du json");

        assert!(matches!(result, Err(AppError::InvalidBackup(_))));
        assert_eq!(env.accounts.list().unwrap(), before);
    }

    #[test]
    fn inconsistent_references_are_rejected_before_writing() {
        let source = TestEnv::new();
        populate(&source);
        let mut dataset: FullDataset =
            serde_json::from_slice(&source.backup().export().unwrap()).unwrap();
        dataset.expenses[0].category_id = uuid::Uuid::new_v4();
        let bytes = serde_json::to_vec(&dataset).unwrap();

        let target = TestEnv::new();
        let result = target.backup().import(&bytes);

        assert!(
            matches!(result, Err(AppError::InvalidBackup(reason)) if reason.contains("catégorie"))
        );
        assert!(target.accounts.list().unwrap().is_empty());
    }

    #[test]
    fn invalid_amounts_are_rejected() {
        let source = TestEnv::new();
        populate(&source);
        let mut dataset: FullDataset =
            serde_json::from_slice(&source.backup().export().unwrap()).unwrap();
        dataset.salaries[0].net_amount = dec!(-3200);

        let result = TestEnv::new()
            .backup()
            .import(&serde_json::to_vec(&dataset).unwrap());

        assert!(
            matches!(result, Err(AppError::InvalidBackup(reason)) if reason.contains("montant positif"))
        );
    }

    // Spec data-backup : « Version plus récente »
    #[test]
    fn newer_schema_version_is_refused() {
        let source = TestEnv::new();
        populate(&source);
        let mut dataset: FullDataset =
            serde_json::from_slice(&source.backup().export().unwrap()).unwrap();
        dataset.schema_version = SCHEMA_VERSION + 1;

        let result = TestEnv::new()
            .backup()
            .import(&serde_json::to_vec(&dataset).unwrap());

        assert_eq!(result, Err(AppError::BackupFromNewerVersion));
    }

    #[test]
    fn export_reminder_after_thirty_days() {
        let env = TestEnv::new();
        let backup = env.backup();
        assert!(!backup.export_reminder_due().unwrap()); // aucune donnée

        populate(&env);
        assert!(backup.export_reminder_due().unwrap()); // jamais exporté

        backup.export().unwrap();
        env.clock.advance(Duration::days(29));
        assert!(!backup.export_reminder_due().unwrap());
        env.clock.advance(Duration::days(2));
        assert!(backup.export_reminder_due().unwrap());
    }
}
