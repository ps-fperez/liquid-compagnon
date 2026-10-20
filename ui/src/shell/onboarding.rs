//! Didacticiel de prise en main : affiché après le premier déverrouillage, puis à la demande
//! depuis les réglages (spec app-shell-theme : « Didacticiel de prise en main »).

use lc_shared_dto::ToggleInput;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use crate::api;
use crate::components::style::{BTN_PRIMARY, BTN_SECONDARY};
use crate::icons::{Icon, IconName};

/// Affichage du didacticiel, partagé entre la coque et les réglages (« Revoir le didacticiel »).
#[derive(Clone, Copy)]
pub struct Onboarding(pub RwSignal<bool>);

impl Onboarding {
    pub fn provide() {
        provide_context(Self(RwSignal::new(false)));
    }

    pub fn replay(self) {
        self.0.set(true);
    }
}

pub fn use_onboarding() -> Onboarding {
    expect_context()
}

struct Step {
    icon: IconName,
    title: &'static str,
    body: &'static str,
}

const STEPS: [Step; 5] = [
    Step {
        icon: IconName::Sparkles,
        title: "Bienvenue dans Liquid Compagnon",
        body: "Votre patrimoine et votre budget réunis au même endroit. Tout reste sur ce téléphone : aucun compte à créer, aucune donnée dans le cloud.",
    },
    Step {
        icon: IconName::Wallet,
        title: "Tous vos comptes",
        body: "Livret A, LDDS, PEA, PEI et crypto. Les cours des ETF et des cryptos se mettent à jour tout seuls ; tirez l'accueil vers le bas pour les rafraîchir.",
    },
    Step {
        icon: IconName::Plus,
        title: "La saisie rapide",
        body: "Le bouton + en bas de l'écran : une dépense en trois gestes (montant, catégorie, c'est enregistré), un revenu ponctuel ou un nouveau solde.",
    },
    Step {
        icon: IconName::Projection,
        title: "Votre épargne demain",
        body: "La projection estime vos comptes à 3, 6 et 12 mois selon trois scénarios, à partir de votre salaire, de vos charges et de vos placements.",
    },
    Step {
        icon: IconName::EyeOff,
        title: "Discret et protégé",
        body: "L'œil en haut masque les montants. L'application se verrouille après une minute en arrière-plan. Pensez à exporter une sauvegarde depuis les réglages.",
    },
];

/// Écran plein en surimpression. S'ouvre seul tant que le didacticiel n'a été ni parcouru ni
/// passé ; balayer vers la gauche ou la droite change d'étape.
#[component]
pub fn OnboardingOverlay() -> impl IntoView {
    let onboarding = use_onboarding();
    let step = RwSignal::new(0_usize);
    let touch_start = StoredValue::new(None::<i32>);
    let navigate = StoredValue::new_local(use_navigate());

    spawn_local(async move {
        if let Ok(settings) = api::get_settings().await
            && !settings.onboarding_done
        {
            step.set(0);
            onboarding.0.set(true);
        }
    });

    let last = STEPS.len() - 1;
    let finish = move |target: Option<&'static str>| {
        onboarding.0.set(false);
        step.set(0);
        spawn_local(async move {
            let _ = api::set_onboarding_done(ToggleInput { enabled: true }).await;
        });
        if let Some(target) = target {
            navigate.with_value(|navigate| navigate(target, Default::default()));
        }
    };
    let next = move || step.update(|s| *s = (*s + 1).min(last));
    let previous = move || step.update(|s| *s = s.saturating_sub(1));

    view! {
        <Show when=move || onboarding.0.get()>
            <div
                class="fixed inset-0 z-50 flex flex-col bg-bg px-6 pt-[calc(1rem+env(safe-area-inset-top))] pb-[calc(1.5rem+env(safe-area-inset-bottom))]"
                role="dialog"
                aria-modal="true"
                aria-label="Didacticiel"
                on:touchstart=move |event| {
                    touch_start.set_value(event.touches().get(0).map(|t| t.client_x()));
                }
                on:touchend=move |event| {
                    let Some(start) = touch_start.get_value() else { return };
                    let Some(end) = event.changed_touches().get(0).map(|t| t.client_x()) else { return };
                    match end - start {
                        dx if dx < -50 => next(),
                        dx if dx > 50 => previous(),
                        _ => {}
                    }
                }
            >
                <div class="flex min-h-11 justify-end">
                    <Show when=move || step.get() < last>
                        <button class="px-2 text-sm text-muted" on:click=move |_| finish(None)>"Passer"</button>
                    </Show>
                </div>
                {move || {
                    let current = &STEPS[step.get()];
                    view! {
                        <div class="flex flex-1 animate-slide-in flex-col items-center justify-center gap-6 text-center">
                            <span class="flex size-24 items-center justify-center rounded-full bg-primary/15 text-primary-soft shadow-glow-primary">
                                <Icon name=current.icon class="size-11" />
                            </span>
                            <h2 class="text-2xl font-semibold">{current.title}</h2>
                            <p class="max-w-sm text-base leading-relaxed text-muted">{current.body}</p>
                        </div>
                    }
                }}
                <div class="flex justify-center gap-2 py-6" aria-hidden="true">
                    {(0..STEPS.len()).map(|index| view! {
                        <span class=move || format!(
                            "h-2 rounded-full transition-all {}",
                            if step.get() == index { "w-6 bg-primary shadow-glow-primary" } else { "w-2 bg-line" },
                        )></span>
                    }).collect_view()}
                </div>
                <p class="sr-only" aria-live="polite">{move || format!("Étape {} sur {}", step.get() + 1, STEPS.len())}</p>
                {move || if step.get() < last {
                    view! {
                        <div class="flex gap-3">
                            <Show when=move || { step.get() > 0 }>
                                <button class=format!("{BTN_SECONDARY} flex-1") on:click=move |_| previous()>"Précédent"</button>
                            </Show>
                            <button class=format!("{BTN_PRIMARY} flex-1") on:click=move |_| next()>"Suivant"</button>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="flex flex-col gap-3">
                            <button class=BTN_PRIMARY on:click=move |_| finish(Some("/comptes?nouveau=1"))>"Ajouter mon premier compte"</button>
                            <button class=BTN_SECONDARY on:click=move |_| finish(None)>"Découvrir l'application"</button>
                        </div>
                    }.into_any()
                }}
            </div>
        </Show>
    }
}
