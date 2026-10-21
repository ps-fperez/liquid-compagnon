//! Virements entre comptes et comptes hors patrimoine (spec account-transfers).

use std::sync::Arc;

use lc_domain::transfer::{Endpoint, ExternalAccount, Schedule, Transfer};
use lc_domain::{ExternalId, Label, Money, TransferId};

use crate::flows;
use crate::ports::{AccountRepository, Clock, TransferRepository};
use crate::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferInput {
    pub from: Endpoint,
    pub to: Endpoint,
    pub sent: Money,
    /// Montant crédité à l'arrivée ; égal au montant envoyé si absent.
    pub received: Option<Money>,
    pub label: Option<String>,
    pub schedule: Schedule,
}

pub struct TransferUseCases {
    repo: Arc<dyn TransferRepository>,
    accounts: Arc<dyn AccountRepository>,
    clock: Arc<dyn Clock>,
}

impl TransferUseCases {
    pub fn new(
        repo: Arc<dyn TransferRepository>,
        accounts: Arc<dyn AccountRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            accounts,
            clock,
        }
    }

    pub fn transfers(&self) -> AppResult<Vec<Transfer>> {
        Ok(self.repo.transfers()?)
    }

    /// Virements dont un côté est `endpoint`.
    pub fn transfers_of(&self, endpoint: Endpoint) -> AppResult<Vec<Transfer>> {
        let mut list = self.repo.transfers()?;
        list.retain(|t| t.involves(endpoint));
        Ok(list)
    }

    pub fn create_transfer(&self, input: TransferInput) -> AppResult<TransferId> {
        self.save(TransferId::generate(), input, self.clock.now())
    }

    pub fn update_transfer(&self, id: TransferId, input: TransferInput) -> AppResult<()> {
        let existing = self
            .repo
            .transfers()?
            .into_iter()
            .find(|t| t.id() == id)
            .ok_or(AppError::NotFound("Virement"))?;
        // L'instant de saisie d'origine est conservé (règle du solde de référence).
        self.save(id, input, existing.recorded_at()).map(|_| ())
    }

    pub fn delete_transfer(&self, id: TransferId) -> AppResult<()> {
        if self.repo.delete_transfer(id)? {
            Ok(())
        } else {
            Err(AppError::NotFound("Virement"))
        }
    }

    pub fn external_accounts(&self) -> AppResult<Vec<ExternalAccount>> {
        Ok(self.repo.external_accounts()?)
    }

    pub fn create_external_account(&self, name: &str) -> AppResult<ExternalId> {
        let label = Label::new(name)?;
        if self
            .repo
            .external_accounts()?
            .iter()
            .any(|a| a.name().as_str().eq_ignore_ascii_case(label.as_str()))
        {
            return Err(AppError::DuplicateExternalAccount);
        }
        let account = ExternalAccount::new(ExternalId::generate(), label);
        self.repo.save_external_account(&account)?;
        Ok(account.id())
    }

    pub fn rename_external_account(&self, id: ExternalId, name: &str) -> AppResult<()> {
        let mut account = self
            .repo
            .external_accounts()?
            .into_iter()
            .find(|a| a.id() == id)
            .ok_or(AppError::NotFound("Compte hors patrimoine"))?;
        account.rename(Label::new(name)?);
        Ok(self.repo.save_external_account(&account)?)
    }

    /// Spec account-transfers : « Suppression refusée » si un virement l'utilise.
    pub fn delete_external_account(&self, id: ExternalId) -> AppResult<()> {
        if self
            .repo
            .transfers()?
            .iter()
            .any(|t| t.involves(Endpoint::External(id)))
        {
            return Err(AppError::ExternalAccountInUse);
        }
        if self.repo.delete_external_account(id)? {
            Ok(())
        } else {
            Err(AppError::NotFound("Compte hors patrimoine"))
        }
    }

    /// Conversion des anciens versements programmés (au démarrage) ; idempotente.
    pub fn convert_scheduled_contributions(&self) -> AppResult<()> {
        flows::convert_scheduled_contributions(
            self.accounts.as_ref(),
            self.repo.as_ref(),
            self.clock.today(),
            self.clock.now(),
        )
        .map(|_| ())
    }

    fn save(
        &self,
        id: TransferId,
        input: TransferInput,
        recorded_at: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<TransferId> {
        self.require_endpoint(input.from)?;
        self.require_endpoint(input.to)?;
        let label = input
            .label
            .as_deref()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(Label::new)
            .transpose()?;
        let transfer = Transfer::new(
            id,
            input.from,
            input.to,
            input.sent,
            input.received.unwrap_or(input.sent),
            label,
            input.schedule,
            recorded_at,
        )?;
        self.repo.save_transfer(&transfer)?;
        Ok(transfer.id())
    }

    /// Un côté est un compte actif ou un compte hors patrimoine existant.
    fn require_endpoint(&self, endpoint: Endpoint) -> AppResult<()> {
        match endpoint {
            Endpoint::Account(id) => match self.accounts.get(id)? {
                Some(account) if !account.is_archived() => Ok(()),
                _ => Err(AppError::UnavailableAccount),
            },
            Endpoint::External(id) => {
                if self.repo.external_accounts()?.iter().any(|a| a.id() == id) {
                    Ok(())
                } else {
                    Err(AppError::UnavailableAccount)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::NewAccount;
    use crate::testing::TestEnv;
    use chrono::{Datelike, Duration};
    use lc_domain::account::AccountType;
    use lc_domain::{AccountId, DomainError, Month};
    use pollster::block_on;
    use rust_decimal_macros::dec;

    fn create(env: &TestEnv, name: &str, account_type: AccountType) -> AccountId {
        env.accounts()
            .create_account(NewAccount {
                name: name.into(),
                account_type,
                color: None,
            })
            .unwrap()
    }

    fn value(env: &TestEnv, id: AccountId) -> Money {
        block_on(env.accounts().valued_account(id)).unwrap().1.value
    }

    fn once(env: &TestEnv) -> Schedule {
        Schedule::Once {
            date: env.clock.today(),
        }
    }

    // Spec account-transfers : « Rapatriement ponctuel » et « Virement vers un compte hors patrimoine »
    #[test]
    fn one_off_transfers_move_balances() {
        let env = TestEnv::new();
        let transfers = env.transfers();
        let checking = create(&env, "Compte courant", AccountType::Checking);
        let ldds = create(&env, "LDDS", AccountType::Ldds);
        env.accounts()
            .update_balance(checking, Money::new(dec!(250)))
            .unwrap();
        env.accounts()
            .update_balance(ldds, Money::new(dec!(10840)))
            .unwrap();
        env.clock.advance(Duration::minutes(5));
        let joint = transfers.create_external_account("Compte joint").unwrap();

        transfers
            .create_transfer(TransferInput {
                from: Endpoint::Account(ldds),
                to: Endpoint::Account(checking),
                sent: Money::new(dec!(100)),
                received: None,
                label: None,
                schedule: once(&env),
            })
            .unwrap();
        transfers
            .create_transfer(TransferInput {
                from: Endpoint::Account(ldds),
                to: Endpoint::External(joint),
                sent: Money::new(dec!(600)),
                received: None,
                label: Some("Vacances à deux".into()),
                schedule: once(&env),
            })
            .unwrap();

        assert_eq!(value(&env, checking), Money::new(dec!(350)));
        assert_eq!(value(&env, ldds), Money::new(dec!(10140)));
        let movements = block_on(env.accounts().valued_account(ldds))
            .unwrap()
            .1
            .movements;
        assert_eq!(movements.len(), 2);
    }

    // Spec account-transfers : « Virement mensuel »
    #[test]
    fn monthly_transfer_applies_on_each_due_date() {
        let env = TestEnv::new();
        let checking = create(&env, "Compte courant", AccountType::Checking);
        let ldds = create(&env, "LDDS", AccountType::Ldds);
        env.accounts()
            .update_balance(ldds, Money::new(dec!(5000)))
            .unwrap();
        let today = env.clock.today();
        env.transfers()
            .create_transfer(TransferInput {
                from: Endpoint::Account(checking),
                to: Endpoint::Account(ldds),
                sent: Money::new(dec!(850)),
                received: None,
                label: None,
                schedule: Schedule::Monthly {
                    day: 1,
                    from: Month::of(today),
                    until: None,
                },
            })
            .unwrap();
        assert_eq!(
            value(&env, ldds),
            Money::new(dec!(5000)),
            "le 1er de ce mois est passé"
        );

        // Un mois plus tard (le 1er du mois suivant est dépassé).
        let next_first = Month::of(today).next().first_day();
        env.clock.set_date(next_first + Duration::days(1));
        assert_eq!(value(&env, ldds), Money::new(dec!(5850)));
        assert!(next_first.day() == 1);
    }

    // Spec account-transfers : « Côtés identiques » et compte archivé
    #[test]
    fn invalid_endpoints_are_refused() {
        let env = TestEnv::new();
        let transfers = env.transfers();
        let ldds = create(&env, "LDDS", AccountType::Ldds);
        let input = |to| TransferInput {
            from: Endpoint::Account(ldds),
            to,
            sent: Money::new(dec!(10)),
            received: None,
            label: None,
            schedule: once(&env),
        };
        assert_eq!(
            transfers.create_transfer(input(Endpoint::Account(ldds))),
            Err(AppError::Domain(DomainError::SameTransferEndpoints))
        );
        assert_eq!(
            transfers.create_transfer(input(Endpoint::External(ExternalId::generate()))),
            Err(AppError::UnavailableAccount)
        );
    }

    // Spec account-transfers : « Suppression refusée »
    #[test]
    fn external_account_in_use_cannot_be_deleted() {
        let env = TestEnv::new();
        let transfers = env.transfers();
        let ldds = create(&env, "LDDS", AccountType::Ldds);
        let joint = transfers.create_external_account("Compte joint").unwrap();
        assert_eq!(
            transfers.create_external_account("compte JOINT"),
            Err(AppError::DuplicateExternalAccount)
        );
        let id = transfers
            .create_transfer(TransferInput {
                from: Endpoint::Account(ldds),
                to: Endpoint::External(joint),
                sent: Money::new(dec!(50)),
                received: None,
                label: None,
                schedule: once(&env),
            })
            .unwrap();

        assert_eq!(
            transfers.delete_external_account(joint),
            Err(AppError::ExternalAccountInUse)
        );
        transfers.delete_transfer(id).unwrap();
        assert_eq!(transfers.delete_external_account(joint), Ok(()));
    }

    // Spec generic-accounts : « Mise à jour de l'application »
    #[test]
    fn scheduled_contributions_become_monthly_transfers() {
        let env = TestEnv::new();
        let checking = create(&env, "Compte courant", AccountType::Checking);
        let livret = create(&env, "Livret A", AccountType::LivretA);
        // Versement programmé d'une ancienne version.
        let mut stored = env.accounts().get_account(livret).unwrap();
        stored
            .set_monthly_contribution(Money::new(dec!(200)))
            .unwrap();
        env.accounts.save(&stored).unwrap();

        let transfers = env.transfers();
        transfers.convert_scheduled_contributions().unwrap();
        transfers.convert_scheduled_contributions().unwrap();

        let list = transfers.transfers().unwrap();
        assert_eq!(list.len(), 1, "conversion idempotente");
        assert_eq!(list[0].from(), Endpoint::Account(checking));
        assert_eq!(list[0].to(), Endpoint::Account(livret));
        assert_eq!(list[0].sent(), Money::new(dec!(200)));
        assert_eq!(
            env.accounts()
                .get_account(livret)
                .unwrap()
                .monthly_contribution(),
            Money::ZERO
        );
    }

    #[test]
    fn contributions_without_checking_account_come_from_an_untracked_one() {
        let env = TestEnv::new();
        let livret = create(&env, "Livret A", AccountType::LivretA);
        let mut stored = env.accounts().get_account(livret).unwrap();
        stored
            .set_monthly_contribution(Money::new(dec!(100)))
            .unwrap();
        env.accounts.save(&stored).unwrap();

        let transfers = env.transfers();
        transfers.convert_scheduled_contributions().unwrap();

        let externals = transfers.external_accounts().unwrap();
        assert_eq!(externals[0].name().as_str(), flows::UNTRACKED_CHECKING);
        assert!(transfers.transfers().unwrap()[0].from().is_external());
    }

    // Spec scheduled-debits : « Abonnement prélevé »
    #[test]
    fn debited_charge_lowers_the_checking_balance_after_its_day() {
        use crate::budget::ChargeInput;
        use lc_domain::budget::{Debit, Periodicity};

        let env = TestEnv::new();
        let budget = env.budget();
        budget.ensure_default_categories().unwrap();
        let category = budget.categories(false).unwrap()[3].id();
        let checking = create(&env, "Compte courant", AccountType::Checking);
        let first = Month::of(env.clock.today()).first_day();
        env.clock.set_date(first);
        env.accounts()
            .update_balance(checking, Money::new(dec!(250)))
            .unwrap();
        budget
            .upsert_charge(
                None,
                ChargeInput {
                    label: "Sosh".into(),
                    amount: Money::new(dec!(15.99)),
                    category,
                    periodicity: Periodicity::Monthly,
                    start: Month::of(first),
                    end: None,
                    debit: Some(Debit {
                        account: checking,
                        day: 5,
                    }),
                },
            )
            .unwrap();
        assert_eq!(value(&env, checking), Money::new(dec!(250)), "avant le 5");

        env.clock.set_date(first + Duration::days(5));
        assert_eq!(value(&env, checking), Money::new(dec!(234.01)));
    }

    // Spec account-transfers : « Sortie vers le compte joint » (budget) et projection annuelle
    #[test]
    fn external_outflows_show_in_the_budget_and_yearly_bonus_in_the_projection() {
        let env = TestEnv::new();
        let budget = env.budget();
        budget.ensure_default_categories().unwrap();
        budget
            .upsert_salary(None, Month::of(env.clock.today()), Money::new(dec!(3340)))
            .unwrap();
        let checking = create(&env, "Compte courant", AccountType::Checking);
        let ldds = create(&env, "LDDS", AccountType::Ldds);
        let pei = create(&env, "Épargne salariale", AccountType::Pei);
        let transfers = env.transfers();
        let joint = transfers.create_external_account("Compte joint").unwrap();
        let today = env.clock.today();
        transfers
            .create_transfer(TransferInput {
                from: Endpoint::Account(ldds),
                to: Endpoint::External(joint),
                sent: Money::new(dec!(600)),
                received: None,
                label: None,
                schedule: Schedule::Once { date: today },
            })
            .unwrap();
        transfers
            .create_transfer(TransferInput {
                from: Endpoint::Account(checking),
                to: Endpoint::Account(pei),
                sent: Money::new(dec!(500)),
                received: Some(Money::new(dec!(1493.30))),
                label: None,
                schedule: Schedule::Yearly {
                    month: 1,
                    day: 15,
                    from_year: today.year() + 1,
                    until_year: None,
                },
            })
            .unwrap();

        let summary = budget.summary(None).unwrap();
        assert_eq!(summary.external_out, Money::new(dec!(600)));

        let report = block_on(env.projection().projection()).unwrap();
        let neutral = report.scenario(lc_domain::account::Scenario::Neutral);
        // Versé en janvier (pas de rendement avant), puis rémunéré au taux du scénario neutre.
        let january =
            Month::of(today).months_until(Month::new(today.year() + 1, 1).unwrap()) as u32;
        assert_eq!(neutral.account_at(pei, january - 1), Some(Money::ZERO));
        assert_eq!(
            neutral.account_at(pei, january),
            Some(Money::new(dec!(1493.30)))
        );
        assert!(neutral.account_at(pei, 12).unwrap() > Money::new(dec!(1493.30)));
    }

    // Spec spending-allowance : « Mois type » (synthèse du budget) et catégories du mois
    #[test]
    fn budget_summary_carries_the_spending_allowance() {
        use crate::budget::ExpenseInput;

        let env = TestEnv::new();
        let budget = env.budget();
        budget.ensure_default_categories().unwrap();
        let today = env.clock.today();
        budget
            .upsert_salary(None, Month::of(today), Money::new(dec!(3340)))
            .unwrap();
        let checking = create(&env, "Compte courant", AccountType::Checking);
        let ldds = create(&env, "LDDS", AccountType::Ldds);
        let transfers = env.transfers();
        let joint = transfers.create_external_account("Compte joint").unwrap();
        let monthly = Schedule::Monthly {
            day: 1,
            from: Month::of(today),
            until: None,
        };
        for (to, amount) in [
            (Endpoint::External(joint), dec!(1560)),
            (Endpoint::Account(ldds), dec!(850)),
        ] {
            transfers
                .create_transfer(TransferInput {
                    from: Endpoint::Account(checking),
                    to,
                    sent: Money::new(amount),
                    received: None,
                    label: None,
                    schedule: monthly,
                })
                .unwrap();
        }
        let categories = budget.categories(false).unwrap();
        for (category, amount) in [(0, dec!(20)), (1, dec!(34.50)), (0, dec!(22))] {
            budget
                .add_expense(ExpenseInput {
                    amount: Money::new(amount),
                    category: categories[category].id(),
                    date: None,
                    label: None,
                    account: None,
                })
                .unwrap();
        }

        let summary = budget.summary(None).unwrap();
        assert_eq!(summary.allowance.envelope, Money::new(dec!(930)));
        assert_eq!(summary.allowance.remaining, Money::new(dec!(853.50)));
        assert_eq!(
            summary.top_categories,
            vec![
                (categories[0].id(), Money::new(dec!(42))),
                (categories[1].id(), Money::new(dec!(34.50))),
            ]
        );
    }
}
