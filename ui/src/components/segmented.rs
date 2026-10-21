use leptos::prelude::*;

/// Contrôle segmenté (maquette v0.3) : options côte à côte dans une même pastille, l'option
/// choisie en violet plein.
#[component]
pub fn Segmented<T>(
    #[prop(into)] label: String,
    options: Vec<(T, &'static str)>,
    selected: RwSignal<T>,
) -> impl IntoView
where
    T: Copy + PartialEq + Send + Sync + 'static,
{
    let columns = options.len();
    view! {
        <div
            class="grid rounded-xl bg-surface-raised p-[3px] text-sm"
            style=format!("grid-template-columns: repeat({columns}, minmax(0, 1fr))")
            role="radiogroup"
            aria-label=label
        >
            {options.into_iter().map(|(value, text)| view! {
                <button
                    role="radio"
                    aria-checked=move || (selected.get() == value).to_string()
                    class=move || format!(
                        "min-h-10 rounded-[9px] transition {}",
                        if selected.get() == value { "bg-primary font-semibold text-bg" } else { "text-muted" },
                    )
                    on:click=move |_| selected.set(value)
                >
                    {text}
                </button>
            }).collect_view()}
        </div>
    }
}
