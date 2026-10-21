use leptos::prelude::*;

/// Jauge d'un budget consommé (maquette v0.3) : barre en dégradé bleu → violet qui se remplit
/// depuis zéro, et repère blanc du jour du mois.
#[component]
pub fn Gauge(
    /// Part consommée, de 0 à 100.
    percent: f64,
    /// Position du repère du jour, de 0 à 100.
    #[prop(optional)]
    marker: Option<f64>,
    /// Couleur pleine à la place du dégradé (dépassement).
    #[prop(optional)]
    alert: bool,
) -> impl IntoView {
    let filled = RwSignal::new(false);
    request_animation_frame(move || {
        let _ = filled.try_set(true);
    });
    let fill = if alert {
        "bg-loss"
    } else {
        "bg-linear-to-r from-secondary to-primary"
    };
    view! {
        <span class="relative block h-2.5 rounded-full bg-surface-raised" role="presentation">
            <span
                class=format!("absolute inset-y-0 left-0 rounded-full transition-[width] duration-1000 ease-[cubic-bezier(0.2,0.8,0.2,1)] {fill}")
                style=move || format!("width: {:.1}%", if filled.get() { percent.clamp(0.0, 100.0) } else { 0.0 })
            ></span>
            {marker.map(|left| view! {
                <span class="absolute -inset-y-1 w-0.5 rounded-full bg-ink" style=format!("left: {:.1}%", left.clamp(0.0, 100.0))></span>
            })}
        </span>
    }
}
