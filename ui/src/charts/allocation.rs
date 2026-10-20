use leptos::prelude::*;

/// Part d'un compte dans le patrimoine.
#[derive(Debug, Clone, PartialEq)]
pub struct AllocationSegment {
    pub color: String,
    /// Pourcentage du total (0 à 100).
    pub share: f64,
}

/// Barre de répartition du patrimoine, une couleur par compte.
#[component]
pub fn AllocationBar(segments: Vec<AllocationSegment>) -> impl IntoView {
    let visible: Vec<_> = segments.into_iter().filter(|s| s.share > 0.0).collect();
    view! {
        <div class="flex h-2.5 w-full gap-0.5 overflow-hidden rounded-full bg-surface-raised" aria-hidden="true">
            {visible
                .into_iter()
                .map(|segment| {
                    view! {
                        <div
                            class="h-full min-w-1 first:rounded-l-full last:rounded-r-full"
                            style=format!("flex: {} 1 0%; background-color: {}", segment.share, segment.color)
                        ></div>
                    }
                })
                .collect_view()}
        </div>
    }
}
