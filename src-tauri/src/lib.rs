//! Backend Tauri de Liquid Compagnon : couche présentation et composition root.
//!
//! Les commandes Tauri convertissent les DTO de `lc-shared-dto` vers les cas d'usage de
//! `lc-application`, dont les ports sont branchés ici sur les adaptateurs de `lc-infrastructure`.

mod biometric;
mod commands;
mod error;
mod mapping;
mod session;
mod state;
mod updater;

use std::sync::Arc;
use std::time::Instant;

use tauri::{Emitter, Manager, WindowEvent};

use crate::state::Services;

/// Événement émis vers l'UI quand la session se verrouille après un passage en arrière-plan.
pub const SESSION_LOCKED_EVENT: &str = "session-locked";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(updater::plugin());
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_biometric::init());

    builder
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            // En développement, `LC_DATA_DIR` isole la base (tests de l'interface sur un dossier
            // temporaire) ; ignoré dans les builds de release.
            let data_dir = match std::env::var_os("LC_DATA_DIR").filter(|_| cfg!(debug_assertions))
            {
                Some(dir) => std::path::PathBuf::from(dir),
                None => app.path().app_data_dir()?,
            };
            let services = Services::compose(&data_dir)?;
            app.manage(Arc::new(services));
            Ok(())
        })
        .on_window_event(|window, event| {
            let WindowEvent::Focused(focused) = event else {
                return;
            };
            let Some(services) = window.try_state::<Arc<Services>>() else {
                return;
            };
            if !*focused {
                services.session.went_to_background(Instant::now());
            } else if services.session.came_to_foreground(Instant::now()) {
                let _ = window.emit(SESSION_LOCKED_EVENT, ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            // Session, sécurité, réglages, sauvegarde
            commands::session::session_status,
            commands::session::setup_pin,
            commands::session::unlock,
            commands::session::unlock_with_biometric,
            commands::session::lock,
            commands::session::notify_hidden,
            commands::session::notify_visible,
            commands::session::change_pin,
            commands::session::get_settings,
            commands::session::set_biometric_enabled,
            commands::session::set_onboarding_done,
            commands::session::set_twelve_data_api_key,
            commands::session::export_backup,
            commands::session::import_backup,
            // Tableau de bord et projection
            commands::overview::get_dashboard,
            commands::overview::take_manual_snapshot,
            commands::overview::get_projection,
            // Comptes
            commands::accounts::list_accounts,
            commands::accounts::get_account,
            commands::accounts::create_account,
            commands::accounts::update_account,
            commands::accounts::archive_account,
            commands::accounts::unarchive_account,
            commands::accounts::update_balance,
            commands::accounts::checking_accounts,
            // Mises à jour
            commands::updates::check_for_update,
            commands::updates::install_update,
            commands::accounts::set_savings_terms,
            commands::accounts::set_scenario_rates,
            commands::accounts::set_cash,
            commands::accounts::upsert_position,
            commands::accounts::remove_position,
            commands::accounts::search_assets,
            commands::accounts::price_source_status,
            commands::accounts::create_pei_fund,
            commands::accounts::update_pei_fund,
            commands::accounts::record_pei_nav,
            commands::accounts::remove_pei_fund,
            commands::planning::list_goals,
            commands::planning::save_goal,
            commands::planning::delete_goal,
            commands::planning::pending_review,
            commands::planning::get_review,
            commands::planning::review_months,
            commands::planning::start_month,
            commands::transfers::list_transfers,
            commands::transfers::save_transfer,
            commands::transfers::delete_transfer,
            commands::transfers::list_external_accounts,
            commands::transfers::create_external_account,
            commands::transfers::rename_external_account,
            commands::transfers::delete_external_account,
            // Budget
            commands::budget::budget_summary,
            commands::budget::list_categories,
            commands::budget::save_category,
            commands::budget::archive_category,
            commands::budget::unarchive_category,
            commands::budget::delete_category,
            commands::budget::salary_history,
            commands::budget::upsert_salary,
            commands::budget::delete_salary,
            commands::budget::list_one_off_incomes,
            commands::budget::upsert_one_off_income,
            commands::budget::delete_one_off_income,
            commands::budget::list_charges,
            commands::budget::expense_suggestions,
            commands::budget::upsert_charge,
            commands::budget::delete_charge,
            commands::budget::save_expense,
            commands::budget::delete_expense,
            commands::budget::list_expenses,
        ])
        .run(tauri::generate_context!())
        .expect("échec du démarrage de l'application Tauri");
}
