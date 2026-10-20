use leptos::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

use super::geometry::{Bounds, Frame, area_path, line_path, points};
use crate::format;
use crate::state::use_privacy;

/// Série d'une courbe. La série mise en avant (`emphasized`) est plus épaisse, lumineuse et
/// remplie d'un dégradé ; les autres sont fines (et pointillées si `dashed`).
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub label: String,
    pub color: String,
    pub values: Vec<f64>,
    pub emphasized: bool,
    pub dashed: bool,
}

const FRAME: Frame = Frame {
    width: 340.0,
    height: 180.0,
    left: 46.0,
    right: 10.0,
    top: 12.0,
    bottom: 24.0,
};
const Y_TICKS: usize = 4;

/// Courbe(s) d'évolution en SVG. `x_labels` : libellés placés sous le premier, le milieu et le
/// dernier point.
#[component]
pub fn LineChart(
    series: Vec<Series>,
    #[prop(optional)] x_labels: Vec<String>,
    #[prop(into)] description: String,
) -> impl IntoView {
    let privacy = use_privacy();
    let Some(bounds) = Bounds::of(series.iter().map(|s| s.values.as_slice())) else {
        return view! { <p class="py-8 text-center text-sm text-muted">"Pas encore de données"</p> }.into_any();
    };
    let ticks = bounds.ticks(Y_TICKS);
    let count = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
    let baseline = FRAME.plot_bottom();

    let grid = ticks
        .iter()
        .map(|tick| {
            let y = FRAME.y(*tick, bounds);
            let label = Decimal::from_f64(*tick).map(format::amount_compact).unwrap_or_default();
            view! {
                <line x1=FRAME.left x2=FRAME.width - FRAME.right y1=y y2=y stroke="#2C2047" stroke-width="1" />
                <text x=FRAME.left - 6.0 y=y + 3.5 text-anchor="end" font-size="10" fill="#A7A3C2">
                    {move || if privacy.hidden() { "••".to_string() } else { label.clone() }}
                </text>
            }
        })
        .collect_view();

    let label_positions: Vec<(usize, String)> = match x_labels.len() {
        0 => Vec::new(),
        n => {
            let last = count.saturating_sub(1);
            let indexes = [0, last / 2, last];
            indexes.into_iter().zip(x_labels).take(n.min(3)).collect()
        }
    };
    let x_axis = label_positions
        .into_iter()
        .map(|(index, label)| {
            let x = FRAME.x(index, count);
            let anchor = if index == 0 { "start" } else if index + 1 >= count { "end" } else { "middle" };
            view! { <text x=x y=FRAME.height - 6.0 text-anchor=anchor font-size="10" fill="#A7A3C2">{label}</text> }
        })
        .collect_view();

    // Séries secondaires d'abord, la série mise en avant par-dessus.
    let mut ordered = series;
    ordered.sort_by_key(|s| s.emphasized);
    let curves = ordered
        .into_iter()
        .map(|serie| {
            let pts = points(&serie.values, FRAME, bounds);
            let gradient_id = format!("lc-area-{}", serie.color.trim_start_matches('#'));
            let area = serie.emphasized.then(|| {
                view! {
                    <defs>
                        <linearGradient id=gradient_id.clone() x1="0" y1="0" x2="0" y2="1">
                            <stop offset="0%" stop-color=serie.color.clone() stop-opacity="0.32" />
                            <stop offset="100%" stop-color=serie.color.clone() stop-opacity="0" />
                        </linearGradient>
                    </defs>
                    <path d=area_path(&pts, baseline) fill=format!("url(#{gradient_id})") />
                }
            });
            let end_dot = serie.emphasized.then(|| {
                pts.last().map(|(x, y)| {
                    view! { <circle cx=*x cy=*y r="3.5" fill=serie.color.clone() class="chart-glow" /> }
                })
            });
            view! {
                {area}
                <path
                    d=line_path(&pts)
                    fill="none"
                    stroke=serie.color.clone()
                    stroke-width=if serie.emphasized { "2.5" } else { "1.5" }
                    stroke-dasharray=if serie.dashed { "4 4" } else { "" }
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class=if serie.emphasized { "chart-glow" } else { "" }
                    opacity=if serie.emphasized { "1" } else { "0.8" }
                >
                    <title>{serie.label}</title>
                </path>
                {end_dot}
            }
        })
        .collect_view();

    view! {
        <svg
            viewBox=format!("0 0 {} {}", FRAME.width, FRAME.height)
            class="h-auto w-full overflow-visible"
            role="img"
            aria-label=description
        >
            {grid}
            {curves}
            {x_axis}
        </svg>
    }
    .into_any()
}
