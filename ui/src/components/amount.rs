use leptos::prelude::*;
use rust_decimal::Decimal;

use crate::format;
use crate::state::use_privacy;

/// Montant formaté à la française, masqué en mode discret.
///
/// `signed` affiche le signe (+/−) ; `colored` colore en vert / rouge selon le signe (réservé
/// aux variations) ; `animate` fait défiler le montant depuis zéro à l'affichage.
#[component]
pub fn Amount(
    value: Decimal,
    #[prop(optional)] signed: bool,
    #[prop(optional)] colored: bool,
    #[prop(optional, into)] class: String,
    #[prop(optional)] animate: bool,
    /// Euros entiers, pour les grands chiffres (patrimoine, reste à dépenser, bilan).
    #[prop(optional)]
    whole: bool,
) -> impl IntoView {
    let privacy = use_privacy();
    let color = match (
        colored,
        value.is_sign_negative() && !value.is_zero(),
        value.is_zero(),
    ) {
        (false, _, _) | (true, _, true) => "",
        (true, true, false) => "text-loss",
        (true, false, false) => "text-gain",
    };
    let shown = if animate {
        super::motion::count_up(value)
    } else {
        RwSignal::new(value)
    };
    let text = move || {
        let value = shown.get();
        match (signed, whole) {
            (true, true) => format::amount_whole_signed(value),
            (true, false) => format::amount_signed(value),
            (false, true) => format::amount_whole(value),
            (false, false) => format::amount(value),
        }
    };
    view! {
        <span class=format!("tabular-nums whitespace-nowrap {color} {class}")>
            {move || if privacy.hidden() { format::HIDDEN_AMOUNT.to_string() } else { text() }}
        </span>
    }
}
