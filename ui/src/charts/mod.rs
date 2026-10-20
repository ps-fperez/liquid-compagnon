//! Graphiques en SVG généré en Rust (aucune bibliothèque JavaScript).

mod allocation;
pub mod geometry;
mod line;

pub use allocation::{AllocationBar, AllocationSegment};
pub use line::{LineChart, Series};

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// Conversion pour l'affichage uniquement (les calculs restent en décimal exact).
pub fn to_f64(value: Decimal) -> f64 {
    value.to_f64().unwrap_or(0.0)
}
