//! Animations (spec motion-feedback : « Animations ») : montants qui défilent, entrées en
//! cascade, transitions d'onglets. Les animations CSS sont coupées par la règle
//! `prefers-reduced-motion` de la feuille de style ; le défilement des montants vérifie la même
//! préférence.

use leptos::html::Div;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};

/// Durée du défilement d'un montant.
const COUNT_UP_MS: f64 = 700.0;
/// Écart entre deux lignes d'une liste en cascade.
const CASCADE_STEP_MS: usize = 60;
/// Au-delà, les lignes entrent ensemble : une longue liste ne fait pas attendre.
const CASCADE_MAX_STEPS: usize = 10;

/// Vrai si le système demande de réduire les animations.
pub fn reduced_motion() -> bool {
    window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .is_some_and(|query| query.matches())
}

/// Style d'une ligne de liste entrant en cascade (avec la classe `animate-rise`).
pub fn cascade(index: usize) -> String {
    format!(
        "animation-delay: {}ms",
        index.min(CASCADE_MAX_STEPS) * CASCADE_STEP_MS
    )
}

/// Valeur affichée d'un montant qui défile de zéro jusqu'à `value` (une fois, à l'affichage).
pub fn count_up(value: Decimal) -> RwSignal<Decimal> {
    if reduced_motion() || value.is_zero() {
        return RwSignal::new(value);
    }
    let shown = RwSignal::new(Decimal::ZERO);
    let target = value.to_f64().unwrap_or_default();
    step(js_sys::Date::now(), target, value, shown);
    shown
}

fn step(start: f64, target: f64, value: Decimal, shown: RwSignal<Decimal>) {
    request_animation_frame(move || {
        let progress = ((js_sys::Date::now() - start) / COUNT_UP_MS).min(1.0);
        let eased = 1.0 - (1.0 - progress).powi(3);
        let current = if progress >= 1.0 {
            value
        } else {
            Decimal::from_f64(target * eased)
                .unwrap_or(value)
                .round_dp(2)
        };
        // Composant retiré entre-temps : on s'arrête.
        if shown.try_set(current).is_none() && progress < 1.0 {
            step(start, target, value, shown);
        }
    });
}

/// Rang d'un écran dans la navigation, pour le sens du glissement : onglets de gauche à droite,
/// la fiche d'un compte après la liste des comptes.
fn rank(path: &str) -> (u8, u8) {
    let depth = u8::from(path.trim_end_matches('/').matches('/').count() > 1);
    let tab = match path.split('/').nth(1).unwrap_or_default() {
        "" => 0,
        "comptes" => 1,
        "budget" => 2,
        "projection" => 3,
        _ => 4,
    };
    (tab, depth)
}

/// Enveloppe des écrans : fondu avec un léger glissement dans le sens de la navigation.
#[component]
pub fn PageTransition(children: Children) -> impl IntoView {
    let location = use_location();
    let node = NodeRef::<Div>::new();
    Effect::new(move |previous: Option<String>| {
        let path = location.pathname.get();
        if let (Some(previous), Some(element)) = (previous.as_deref(), node.get_untracked())
            && previous != path
        {
            let class = if rank(&path) >= rank(previous) {
                "animate-page-forward"
            } else {
                "animate-page-back"
            };
            let classes = element.class_list();
            let _ = classes.remove_2("animate-page-forward", "animate-page-back");
            // Relance l'animation même si la classe est la même que la précédente.
            let _ = element.offset_width();
            let _ = classes.add_1(class);
        }
        path
    });
    view! { <div node_ref=node class="flex flex-col gap-4">{children()}</div> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_rank_follows_tabs_then_depth() {
        assert!(rank("/comptes") > rank("/"));
        assert!(rank("/comptes/42") > rank("/comptes"));
        assert!(rank("/budget") > rank("/comptes/42"));
        assert!(rank("/") < rank("/projection"));
    }
}
