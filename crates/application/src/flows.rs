//! Flux entre comptes : grand livre des mouvements, flux récurrents de la projection et
//! conversion des anciens versements programmés en virements.

use chrono::{DateTime, NaiveDate, Utc};
use lc_domain::account::{Account, AccountType};
use lc_domain::ledger::{Ledger, LedgerSources};
use lc_domain::projection::{FlowTiming, ProjectedFlow, ProjectionInput};
use lc_domain::transfer::{Endpoint, ExternalAccount, Schedule, Transfer};
use lc_domain::{ExternalId, Label, Money, Month, TransferId};

use crate::AppResult;
use crate::ports::{AccountRepository, BudgetRepository, TransferRepository};

/// Grand livre à la date `today` : dépenses et revenus rattachés, charges prélevées, virements.
pub(crate) fn build_ledger(
    budget: &dyn BudgetRepository,
    transfers: &dyn TransferRepository,
    today: NaiveDate,
) -> AppResult<Ledger> {
    let (expenses, incomes, charges, transfers) = (
        budget.expenses()?,
        budget.one_off_incomes()?,
        budget.charges()?,
        transfers.transfers()?,
    );
    Ok(Ledger::build(&LedgerSources {
        expenses: &expenses,
        incomes: &incomes,
        charges: &charges,
        transfers: &transfers,
        today,
    }))
}

/// Ajoute à la projection les virements récurrents : reçus en entrée et envoyés en sortie des
/// comptes projetés (le compte courant reste constant), et prélevés sur la capacité d'épargne
/// quand ils partent d'un compte courant ou d'un compte hors patrimoine vers l'épargne.
pub(crate) fn add_projection_flows(
    input: &mut ProjectionInput,
    accounts: &[Account],
    transfers: &[Transfer],
) {
    let type_of = |endpoint: Endpoint| {
        endpoint.account().and_then(|id| {
            accounts
                .iter()
                .find(|a| a.id() == id)
                .map(Account::account_type)
        })
    };
    let is_savings =
        |endpoint: Endpoint| type_of(endpoint).is_some_and(|t| t != AccountType::Checking);
    for transfer in transfers {
        let Some((timing, from, until)) = recurrence(transfer.schedule()) else {
            continue;
        };
        let flow = |amount: Money| ProjectedFlow {
            amount,
            timing,
            from,
            until,
        };
        if is_savings(transfer.to()) {
            let to = transfer.to().account().expect("compte suivi");
            input.add_flow(to, flow(transfer.received()));
            let funded_by_income = transfer.from().is_external()
                || type_of(transfer.from()) == Some(AccountType::Checking);
            if funded_by_income {
                input.use_capacity(flow(transfer.sent()));
            }
        }
        if is_savings(transfer.from()) {
            let from_account = transfer.from().account().expect("compte suivi");
            input.add_flow(from_account, flow(-transfer.sent()));
        }
    }
}

fn recurrence(schedule: Schedule) -> Option<(FlowTiming, Month, Option<Month>)> {
    match schedule {
        Schedule::Once { .. } => None,
        Schedule::Monthly { from, until, .. } => Some((FlowTiming::EveryMonth, from, until)),
        Schedule::Yearly {
            month,
            from_year,
            until_year,
            ..
        } => Some((
            FlowTiming::EveryYear(month),
            Month::new(from_year, 1).ok()?,
            until_year.and_then(|year| Month::new(year, 12).ok()),
        )),
    }
}

/// Nom du compte hors patrimoine créé quand un versement programmé n'a pas de compte courant
/// d'origine.
pub const UNTRACKED_CHECKING: &str = "Compte courant (non suivi)";

/// Convertit chaque versement mensuel programmé en virement mensuel le 1er du mois, depuis le
/// premier compte courant actif (ou un compte hors patrimoine « Compte courant (non suivi) »),
/// puis remet le versement à zéro. Sans effet si aucun versement n'est programmé.
pub(crate) fn convert_scheduled_contributions(
    accounts: &dyn AccountRepository,
    transfers: &dyn TransferRepository,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> AppResult<usize> {
    let all = accounts.list()?;
    let pending: Vec<Account> = all
        .iter()
        .filter(|a| a.monthly_contribution().is_positive())
        .cloned()
        .collect();
    if pending.is_empty() {
        return Ok(0);
    }
    let source = match all
        .iter()
        .find(|a| a.account_type() == AccountType::Checking && !a.is_archived())
    {
        Some(checking) => Endpoint::Account(checking.id()),
        None => Endpoint::External(untracked_checking(transfers)?),
    };
    for mut account in pending {
        let amount = account.monthly_contribution();
        if Endpoint::Account(account.id()) != source {
            let transfer = Transfer::new(
                TransferId::generate(),
                source,
                Endpoint::Account(account.id()),
                amount,
                amount,
                None,
                Schedule::Monthly {
                    day: 1,
                    from: Month::of(today),
                    until: None,
                },
                now,
            )?;
            transfers.save_transfer(&transfer)?;
        }
        account.set_monthly_contribution(Money::ZERO)?;
        accounts.save(&account)?;
    }
    Ok(1)
}

fn untracked_checking(transfers: &dyn TransferRepository) -> AppResult<ExternalId> {
    if let Some(existing) = transfers
        .external_accounts()?
        .into_iter()
        .find(|a| a.name().as_str() == UNTRACKED_CHECKING)
    {
        return Ok(existing.id());
    }
    let created = ExternalAccount::new(ExternalId::generate(), Label::new(UNTRACKED_CHECKING)?);
    transfers.save_external_account(&created)?;
    Ok(created.id())
}
