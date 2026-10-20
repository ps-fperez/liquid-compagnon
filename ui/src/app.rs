use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn App() -> impl IntoView {
    let backend_status = RwSignal::new(String::from("Connexion au backend…"));

    spawn_local(async move {
        let status = match crate::api::ping().await {
            Ok(response) => format!("Backend connecté · v{}", response.backend_version),
            Err(error) => format!("Backend injoignable : {error}"),
        };
        backend_status.set(status);
    });

    view! {
        <main class="mx-auto flex min-h-dvh max-w-md flex-col gap-6 px-4 py-8">
            <header class="flex items-center gap-3">
                <div class="flex size-9 items-center justify-center rounded-xl bg-primary font-semibold text-bg">
                    "LC"
                </div>
                <h1 class="text-lg font-medium">"Liquid Compagnon"</h1>
            </header>

            <section class="rounded-2xl border border-primary-strong bg-linear-to-br from-surface-raised to-surface p-5 shadow-glow-primary">
                <p class="text-sm text-muted">"Patrimoine total"</p>
                <p class="mt-1 text-3xl font-semibold tracking-tight">"— €"</p>
                <p class="mt-3 text-sm text-secondary-light">{move || backend_status.get()}</p>
            </section>

            <button class="mx-auto flex size-14 items-center justify-center rounded-full bg-secondary text-2xl text-ink animate-glow-pulse">
                "+"
            </button>
        </main>
    }
}
