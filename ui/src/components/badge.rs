use lc_shared_dto::ProvenanceDto;
use leptos::prelude::*;

use crate::format;

/// Indique d'où vient une valeur : en direct, en cache (avec sa date), estimée, saisie…
#[component]
pub fn ProvenanceBadge(provenance: ProvenanceDto) -> impl IntoView {
    let (label, style) = match provenance {
        ProvenanceDto::Live => (
            "en direct".to_string(),
            "bg-secondary/15 text-secondary-light",
        ),
        ProvenanceDto::Manual => (
            "saisie manuelle".to_string(),
            "bg-surface-raised text-muted",
        ),
        ProvenanceDto::Cached { as_of } => (
            format!("cours du {}", format::date_time_short(as_of)),
            "bg-surface-raised text-muted",
        ),
        ProvenanceDto::Estimated => ("estimée".to_string(), "bg-primary/15 text-primary-soft"),
        ProvenanceDto::CostBasis => (
            "prix de revient".to_string(),
            "bg-surface-raised text-muted",
        ),
        ProvenanceDto::Unavailable => ("cours indisponible".to_string(), "bg-loss/10 text-loss"),
    };
    view! {
        <span class=format!("inline-flex items-center rounded-full px-2 py-0.5 text-[11px] leading-4 {style}")>
            {label}
        </span>
    }
}
