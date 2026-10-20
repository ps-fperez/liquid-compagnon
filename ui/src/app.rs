use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::pages::{
    AccountDetailPage, AccountsPage, BudgetPage, DashboardPage, Gallery, NotFound, ProjectionPage,
    SettingsPage,
};
use crate::shell::{BottomNav, Header, SessionGate};
use crate::state::{Privacy, Session, watch_visibility};

/// Racine : contextes globaux, porte de session, coque et routes (une route par écran pour que
/// le bouton retour d'Android remonte dans l'historique).
#[component]
pub fn App() -> impl IntoView {
    Session::provide();
    Privacy::provide();
    watch_visibility();

    view! {
        <Router>
            <SessionGate>
                <Header />
                <main class="mx-auto flex max-w-md flex-col gap-4 px-4 pt-2 pb-32">
                    <Routes fallback=|| view! { <NotFound /> }>
                        <Route path=path!("/") view=DashboardPage />
                        <Route path=path!("/comptes") view=AccountsPage />
                        <Route path=path!("/comptes/:id") view=AccountDetailPage />
                        <Route path=path!("/budget") view=BudgetPage />
                        <Route path=path!("/projection") view=ProjectionPage />
                        <Route path=path!("/reglages") view=SettingsPage />
                        <Route path=path!("/galerie") view=Gallery />
                    </Routes>
                </main>
                <BottomNav />
            </SessionGate>
        </Router>
    }
}
