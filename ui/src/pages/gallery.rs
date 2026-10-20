//! Galerie des composants et graphiques (builds de développement) : vérification visuelle du
//! thème sans backend, ex. avec `trunk serve`.

use std::str::FromStr;

use lc_shared_dto::{DateTime, ProvenanceDto};
use leptos::prelude::*;
use rust_decimal::Decimal;

use crate::charts::{AllocationBar, AllocationSegment, LineChart, Series, to_f64};
use crate::components::style::{BTN_DANGER, BTN_GHOST, BTN_PRIMARY, BTN_SECONDARY, SECTION_TITLE};
use crate::components::{
    Amount, BottomSheet, Card, EmptyState, ErrorBanner, Field, GlowCard, ListRow, ProvenanceBadge,
    Skeleton,
};
use crate::format;
use crate::icons::{Icon, IconName};

/// Montant d'exemple.
fn d(text: &str) -> Decimal {
    Decimal::from_str(text).unwrap_or_default()
}

#[component]
pub fn Gallery() -> impl IntoView {
    if !cfg!(debug_assertions) {
        return view! { <EmptyState title="Indisponible" body="La galerie n'existe qu'en développement." /> }.into_any();
    }
    let sheet = RwSignal::new(false);
    let amount_input = RwSignal::new(String::from("42.5"));
    let parsed = move || format::parse_amount(&amount_input.get());

    let history = Series {
        label: "Patrimoine".into(),
        color: "#A855F7".into(),
        values: [
            "41200", "42050", "41700", "43900", "45100", "47230", "48362.15",
        ]
        .map(|value| to_f64(d(value)))
        .to_vec(),
        emphasized: true,
        dashed: false,
    };
    let scenario = |label: &str, color: &str, rate: f64, emphasized: bool| Series {
        label: label.into(),
        color: color.into(),
        values: (0..=12)
            .map(|m| 48_362.0 * (1.0 + rate).powf(m as f64 / 12.0) + 1_300.0 * m as f64)
            .collect(),
        emphasized,
        dashed: !emphasized,
    };
    let as_of = DateTime::parse_from_rfc3339("2026-10-20T09:42:00Z")
        .map(|d| d.to_utc())
        .unwrap_or_default();

    view! {
        <div class="flex flex-col gap-6">
            <h1 class="px-1 text-2xl font-medium">"Galerie"</h1>

            <GlowCard>
                <p class="text-sm text-muted">"Patrimoine total"</p>
                <Amount value=d("48362.15") class="mt-1 block text-3xl font-semibold tracking-tight" />
                <div class="mt-2 flex items-center gap-2 text-sm">
                    <Amount value=d("1132") signed=true colored=true />
                    <span class="text-gain">{format::percent_signed(d("2.396"), 1)}</span>
                    <span class="text-muted">"ce mois"</span>
                </div>
                <div class="mt-4">
                    <AllocationBar segments=vec![
                        AllocationSegment { color: "#A855F7".into(), share: 22.0 },
                        AllocationSegment { color: "#C084FC".into(), share: 10.8 },
                        AllocationSegment { color: "#3B82F6".into(), share: 30.0 },
                        AllocationSegment { color: "#60A5FA".into(), share: 19.2 },
                        AllocationSegment { color: "#7C3AED".into(), share: 18.0 },
                    ] />
                </div>
            </GlowCard>

            <section class="flex flex-col gap-2">
                <h2 class=SECTION_TITLE>"Provenance des valeurs"</h2>
                <div class="flex flex-wrap gap-2">
                    <ProvenanceBadge provenance=ProvenanceDto::Live />
                    <ProvenanceBadge provenance=ProvenanceDto::Cached { as_of } />
                    <ProvenanceBadge provenance=ProvenanceDto::Estimated />
                    <ProvenanceBadge provenance=ProvenanceDto::Manual />
                    <ProvenanceBadge provenance=ProvenanceDto::CostBasis />
                    <ProvenanceBadge provenance=ProvenanceDto::Unavailable />
                </div>
            </section>

            <Card>
                <h2 class="mb-2 text-sm text-muted">"Évolution sur 6 mois"</h2>
                <LineChart
                    series=vec![history]
                    x_labels=vec![format::month_short("2026-04"), format::month_short("2026-07"), format::month_short("2026-10")]
                    description="Évolution du patrimoine"
                />
            </Card>

            <Card>
                <h2 class="mb-2 text-sm text-muted">"Projection à 12 mois"</h2>
                <LineChart
                    series=vec![
                        scenario("Pessimiste", "#60A5FA", -0.05, false),
                        scenario("Optimiste", "#C084FC", 0.10, false),
                        scenario("Neutre", "#A855F7", 0.04, true),
                    ]
                    x_labels=vec!["Aujourd'hui".into(), "6 mois".into(), "12 mois".into()]
                    description="Projection selon trois scénarios"
                />
            </Card>

            <Card class="flex flex-col gap-4">
                <Field
                    label="Montant"
                    value=amount_input
                    input_mode="decimal"
                    suffix="€"
                    placeholder="0,00"
                    error=Signal::derive(move || parsed().err())
                />
                <p class="text-sm text-muted">
                    "Lu : "
                    {move || parsed().map(format::amount).unwrap_or_else(|_| "—".into())}
                </p>
            </Card>

            <section class="flex flex-col gap-3">
                <h2 class=SECTION_TITLE>"Boutons"</h2>
                <button class=BTN_PRIMARY on:click=move |_| sheet.set(true)>"Ouvrir une feuille"</button>
                <button class=BTN_SECONDARY>"Secondaire"</button>
                <button class=BTN_GHOST>"Discret"</button>
                <button class=BTN_DANGER>"Supprimer"</button>
            </section>

            <Card class="flex flex-col gap-1 text-sm">
                <h2 class=SECTION_TITLE>"Formats"</h2>
                <p>{format::percent(d("30.003"), 1)} " · " {format::quantity(d("0.1500"))} " · " {format::amount_whole(d("48362.5"))}</p>
                <p>
                    {format::month_label(&format::iso_month(as_of.date_naive()))}
                    " → "
                    {format::shift_month("2026-10", 3).map(|m| format::month_label(&m)).unwrap_or_default()}
                </p>
            </Card>

            <Card class="p-1">
                <ListRow href="/galerie">
                    <span class="size-2.5 shrink-0 rounded-full bg-primary"></span>
                    <span class="flex-1 truncate">"Livret A"</span>
                    <Amount value=d("10640") />
                </ListRow>
            </Card>

            <div class="flex flex-wrap gap-3 text-muted">
                {IconName::ALL.into_iter().map(|name| view! { <Icon name=name /> }).collect_view()}
            </div>

            <ErrorBanner message="Limite de requêtes atteinte, réessayez plus tard" />
            <Skeleton />
            <p class="text-sm text-muted">
                "Format : " {format::amount(Decimal::from(1234567))} " · " {format::date(as_of.date_naive())}
            </p>

            <BottomSheet open=sheet title="Exemple de feuille">
                <p class="text-sm text-muted">"Une feuille monte du bas de l'écran pour les saisies courtes."</p>
            </BottomSheet>
        </div>
    }
    .into_any()
}
