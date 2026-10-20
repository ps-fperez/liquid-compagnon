//! Géométrie des courbes : échelles et tracés SVG. Fonctions pures, testées hors navigateur.

/// Plage de valeurs affichée sur l'axe vertical.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub min: f64,
    pub max: f64,
}

impl Bounds {
    /// Marge ajoutée au-dessus et au-dessous des valeurs extrêmes.
    const PADDING_RATIO: f64 = 0.08;

    /// Plage couvrant toutes les séries, avec une marge ; `None` sans valeur exploitable.
    pub fn of<'a>(series: impl IntoIterator<Item = &'a [f64]>) -> Option<Self> {
        let mut values = series
            .into_iter()
            .flatten()
            .copied()
            .filter(|v| v.is_finite());
        let first = values.next()?;
        let (min, max) = values.fold((first, first), |(lo, hi), v| (lo.min(v), hi.max(v)));
        let span = if max > min {
            max - min
        } else {
            first.abs().max(1.0)
        };
        let padding = span * Self::PADDING_RATIO;
        let (min, max) = if max > min {
            (min, max)
        } else {
            (min - span / 2.0, max + span / 2.0)
        };
        Some(Self {
            min: min - padding,
            max: max + padding,
        })
    }

    /// `count` graduations régulières, du bas vers le haut.
    pub fn ticks(&self, count: usize) -> Vec<f64> {
        match count {
            0 => Vec::new(),
            1 => vec![(self.min + self.max) / 2.0],
            _ => (0..count)
                .map(|i| self.min + (self.max - self.min) * i as f64 / (count - 1) as f64)
                .collect(),
        }
    }
}

/// Zone de dessin dans le `viewBox`, avec ses marges (libellés des axes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub width: f64,
    pub height: f64,
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

impl Frame {
    pub fn plot_bottom(&self) -> f64 {
        self.height - self.bottom
    }

    /// Abscisse du point `index` sur `count` (un point unique est centré).
    pub fn x(&self, index: usize, count: usize) -> f64 {
        let width = self.width - self.left - self.right;
        if count <= 1 {
            self.left + width / 2.0
        } else {
            self.left + width * index as f64 / (count - 1) as f64
        }
    }

    pub fn y(&self, value: f64, bounds: Bounds) -> f64 {
        let height = self.height - self.top - self.bottom;
        let ratio = if bounds.max > bounds.min {
            (value - bounds.min) / (bounds.max - bounds.min)
        } else {
            0.5
        };
        self.top + height * (1.0 - ratio)
    }
}

pub fn points(values: &[f64], frame: Frame, bounds: Bounds) -> Vec<(f64, f64)> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| (frame.x(index, values.len()), frame.y(*value, bounds)))
        .collect()
}

/// Tracé SVG reliant les points (`M x,y L x,y …`).
pub fn line_path(points: &[(f64, f64)]) -> String {
    points
        .iter()
        .enumerate()
        .map(|(i, (x, y))| format!("{}{x:.1},{y:.1}", if i == 0 { "M" } else { " L" }))
        .collect()
}

/// Surface sous la courbe, refermée sur la ligne de base.
pub fn area_path(points: &[(f64, f64)], baseline: f64) -> String {
    match (points.first(), points.last()) {
        (Some(first), Some(last)) => format!(
            "{} L{:.1},{baseline:.1} L{:.1},{baseline:.1} Z",
            line_path(points),
            last.0,
            first.0
        ),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: Frame = Frame {
        width: 300.0,
        height: 120.0,
        left: 40.0,
        right: 10.0,
        top: 10.0,
        bottom: 10.0,
    };

    #[test]
    fn bounds_cover_all_series_with_padding() {
        let a = [100.0, 200.0];
        let b = [150.0, 300.0];
        let bounds = Bounds::of([&a[..], &b[..]]).unwrap();
        assert!((bounds.min - 84.0).abs() < 1e-9);
        assert!((bounds.max - 316.0).abs() < 1e-9);
    }

    #[test]
    fn flat_series_gets_a_visible_range() {
        let flat = [1000.0, 1000.0];
        let bounds = Bounds::of([&flat[..]]).unwrap();
        assert!(bounds.min < 1000.0 && bounds.max > 1000.0);
        assert!(Bounds::of([&[f64::NAN][..]]).is_none());
        assert!(Bounds::of(std::iter::empty::<&[f64]>()).is_none());
    }

    #[test]
    fn points_span_the_plot_area() {
        let bounds = Bounds {
            min: 0.0,
            max: 100.0,
        };
        let pts = points(&[0.0, 50.0, 100.0], FRAME, bounds);
        assert_eq!(pts[0], (40.0, 110.0));
        assert_eq!(pts[1], (165.0, 60.0));
        assert_eq!(pts[2], (290.0, 10.0));
        assert_eq!(FRAME.x(0, 1), 165.0);
    }

    #[test]
    fn builds_svg_paths() {
        let pts = [(0.0, 10.0), (5.0, 2.25)];
        assert_eq!(line_path(&pts), "M0.0,10.0 L5.0,2.2");
        assert_eq!(
            area_path(&pts, 20.0),
            "M0.0,10.0 L5.0,2.2 L5.0,20.0 L0.0,20.0 Z"
        );
        assert_eq!(area_path(&[], 20.0), "");
    }

    #[test]
    fn ticks_are_evenly_spaced() {
        let bounds = Bounds {
            min: 0.0,
            max: 90.0,
        };
        assert_eq!(bounds.ticks(4), vec![0.0, 30.0, 60.0, 90.0]);
        assert_eq!(bounds.ticks(1), vec![45.0]);
    }
}
