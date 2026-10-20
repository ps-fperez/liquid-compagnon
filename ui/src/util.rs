//! Aides partagées par les écrans : chargement asynchrone, libellés, date du jour.

use std::future::Future;

use lc_shared_dto::{ErrorDto, NaiveDate, PricingIssueDto, PricingIssueKindDto, QuoteSourceDto};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Résultat d'un chargement : `None` tant qu'il est en cours.
pub type Loaded<T> = RwSignal<Option<Result<T, ErrorDto>>>;

/// Lance (ou relance) un chargement et range son résultat dans `target`.
pub fn load<T, Fut>(target: Loaded<T>, fetch: impl FnOnce() -> Fut + 'static)
where
    T: Send + Sync + 'static,
    Fut: Future<Output = Result<T, ErrorDto>> + 'static,
{
    spawn_local(async move {
        let result = fetch().await;
        target.set(Some(result));
    });
}

/// Exécute une action (enregistrement…) : empêche le double envoi, affiche l'erreur éventuelle,
/// puis appelle `on_success`.
pub fn run_action<T, Fut>(
    busy: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    action: Fut,
    on_success: impl FnOnce(T) + 'static,
) where
    T: 'static,
    Fut: Future<Output = Result<T, ErrorDto>> + 'static,
{
    if busy.get_untracked() {
        return;
    }
    busy.set(true);
    error.set(None);
    spawn_local(async move {
        match action.await {
            Ok(value) => on_success(value),
            Err(failure) => error.set(Some(failure.message)),
        }
        busy.set(false);
    });
}

/// Couleurs proposées pour identifier un compte.
pub const ACCOUNT_COLORS: [&str; 8] = [
    "#A855F7", "#C084FC", "#7C3AED", "#E879F9", "#3B82F6", "#60A5FA", "#38BDF8", "#22D3EE",
];

pub fn source_label(source: QuoteSourceDto) -> &'static str {
    match source {
        QuoteSourceDto::CoinGecko => "CoinGecko",
        QuoteSourceDto::TwelveData => "Twelve Data",
    }
}

/// Message affiché pour un problème de cotation (les valeurs restent visibles).
pub fn pricing_issue_message(issue: &PricingIssueDto) -> String {
    let source = source_label(issue.source);
    match issue.kind {
        PricingIssueKindDto::RateLimited => {
            format!("{source} : limite de requêtes atteinte, réessayez plus tard")
        }
        PricingIssueKindDto::MissingApiKey => {
            "Ajoutez une clé Twelve Data dans les réglages pour suivre les ETF".to_string()
        }
        PricingIssueKindDto::Unavailable => {
            format!("{source} injoignable : dernières valeurs connues affichées")
        }
    }
}

/// Date du jour (fuseau du téléphone).
pub fn today() -> NaiveDate {
    let now = js_sys::Date::new_0();
    NaiveDate::from_ymd_opt(
        now.get_full_year() as i32,
        now.get_month() + 1,
        now.get_date(),
    )
    .unwrap_or_default()
}

/// Mois en cours au format `AAAA-MM`.
pub fn current_month() -> String {
    crate::format::iso_month(today())
}
