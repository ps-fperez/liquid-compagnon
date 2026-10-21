use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::components::{LoadingBar, PageTransition, ToastHost, loading, toast};
use crate::pages::{
    AccountDetailPage, AccountsPage, BudgetPage, DashboardPage, Gallery, NotFound, ProjectionPage,
    SettingsPage,
};
use crate::shell::{
    BottomNav, Header, Onboarding, OnboardingOverlay, Review, ReviewOverlay, ReviewWatcher,
    SessionGate, UpdateWatcher, Updates,
};
use crate::state::{Privacy, Session, watch_visibility};

/// Racine : contextes globaux, porte de session, coque et routes (une route par écran pour que
/// le bouton retour d'Android remonte dans l'historique).
#[component]
pub fn App() -> impl IntoView {
    toast::provide_toasts();
    loading::provide_loading();
    Session::provide();
    Privacy::provide();
    Onboarding::provide();
    Updates::provide();
    Review::provide();
    watch_visibility();

    view! {
        <LoadingBar />
        <ToastHost />
        <Router>
            <SessionGate>
                <Header />
                <main class="mx-auto max-w-md px-4 pt-2 pb-32">
                    <PageTransition>
                    <Routes fallback=|| view! { <NotFound /> }>
                        <Route path=path!("/") view=DashboardPage />
                        <Route path=path!("/comptes") view=AccountsPage />
                        <Route path=path!("/comptes/:id") view=AccountDetailPage />
                        <Route path=path!("/budget") view=BudgetPage />
                        <Route path=path!("/projection") view=ProjectionPage />
                        <Route path=path!("/reglages") view=SettingsPage />
                        <Route path=path!("/galerie") view=Gallery />
                    </Routes>
                    </PageTransition>
                </main>
                <BottomNav />
                <UpdateWatcher />
                <ReviewWatcher />
                <ReviewOverlay />
                <OnboardingOverlay />
            </SessionGate>
        </Router>
    }
}
