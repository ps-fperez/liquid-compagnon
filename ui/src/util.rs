//! Aides partagées par les écrans : chargement asynchrone, libellés, date du jour.

use std::future::Future;

use lc_shared_dto::{
    ErrorDto, IdInput, NaiveDate, PricingIssueDto, PricingIssueKindDto, QuoteSourceDto,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::toast;

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

/// Comme `load`, puis appelle `then` sur le résultat réussi (notification, suite d'un parcours).
pub fn load_then<T, Fut>(
    target: Loaded<T>,
    fetch: impl FnOnce() -> Fut + 'static,
    then: impl FnOnce(&T) + 'static,
) where
    T: Send + Sync + 'static,
    Fut: Future<Output = Result<T, ErrorDto>> + 'static,
{
    spawn_local(async move {
        let result = fetch().await;
        if let Ok(value) = &result {
            then(value);
        }
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
            Err(failure) => {
                // Visible même si le message sous le formulaire est hors de l'écran.
                toast::error(failure.message.clone());
                error.set(Some(failure.message));
            }
        }
        busy.set(false);
    });
}

/// Action « Annuler » d'une notification : supprime l'élément `id` par `delete`, confirme et
/// recharge l'écran s'il est encore affiché.
pub fn undo<Fut>(
    delete: fn(IdInput) -> Fut,
    id: String,
    done: &'static str,
    on_saved: Callback<()>,
) -> impl Fn() + Send + Sync + 'static
where
    Fut: Future<Output = Result<(), ErrorDto>> + 'static,
{
    move || {
        let id = id.clone();
        spawn_local(async move {
            match delete(IdInput { id }).await {
                Ok(()) => {
                    toast::info(done);
                    let _ = on_saved.try_run(());
                }
                Err(failure) => toast::error(failure.message),
            }
        });
    }
}

/// Couleurs proposées pour identifier un compte.
pub const ACCOUNT_COLORS: [&str; 8] = [
    "#A855F7", "#C084FC", "#7C3AED", "#E879F9", "#3B82F6", "#60A5FA", "#38BDF8", "#22D3EE",
];

pub fn source_label(source: QuoteSourceDto) -> &'static str {
    match source {
        QuoteSourceDto::CoinGecko => "CoinGecko",
        QuoteSourceDto::TwelveData => "Twelve Data",
        QuoteSourceDto::Yahoo => "Yahoo Finance",
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
            "Clé Twelve Data absente ou invalide : vérifiez-la dans les réglages".to_string()
        }
        PricingIssueKindDto::Unavailable => {
            format!("{source} injoignable : dernières valeurs connues affichées")
        }
        PricingIssueKindDto::NotCovered => {
            let symbols = issue.symbols.join(", ");
            match issue.source {
                QuoteSourceDto::TwelveData => format!(
                    "{symbols} : non couvert par votre offre Twelve Data. Choisissez l'actif via Yahoo Finance"
                ),
                _ => format!("{symbols} : cours introuvable chez {source}"),
            }
        }
    }
}

/// Fraction (0,25) en pourcentage (25) pour une largeur de jauge.
pub fn to_percent(fraction: lc_shared_dto::Decimal) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    (fraction * lc_shared_dto::Decimal::ONE_HUNDRED)
        .to_f64()
        .unwrap_or(0.0)
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
