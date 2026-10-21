//! Mises à jour (spec app-updates) : vérification au déverrouillage, invitation une fois par
//! build, pastille sur l'engrenage, installation avec barre de progression.

use lc_shared_dto::{
    InstallOutcomeDto, UpdateCheckDto, UpdateCheckInput, UpdateProgressDto, UpdateStatusDto,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::BottomSheet;
use crate::components::style::{BTN_GHOST, BTN_PRIMARY};

/// Préférences d'appareil (comme le mode discret).
const AUTO_CHECK_KEY: &str = "lc.maj-auto";
const POSTPONED_KEY: &str = "lc.maj-reportee";

/// État partagé : dernier résultat de vérification, invitation, installation en cours.
#[derive(Clone, Copy)]
pub struct Updates {
    pub check: RwSignal<Option<UpdateCheckDto>>,
    pub prompt: RwSignal<bool>,
    pub progress: RwSignal<Option<UpdateProgressDto>>,
    pub installing: RwSignal<bool>,
    pub message: RwSignal<Option<String>>,
}

impl Updates {
    /// À appeler une fois (racine de l'application) : écoute aussi la progression.
    pub fn provide() {
        let updates = Self {
            check: RwSignal::new(None),
            prompt: RwSignal::new(false),
            progress: RwSignal::new(None),
            installing: RwSignal::new(false),
            message: RwSignal::new(None),
        };
        provide_context(updates);
        spawn_local(async move {
            api::listen_forever("update-progress", move |progress: UpdateProgressDto| {
                updates.progress.set(Some(progress));
            })
            .await;
        });
    }

    /// Build proposé, s'il y en a un.
    pub fn available_build(self) -> Option<u32> {
        self.check
            .with(|check| match check.as_ref().map(|c| &c.status) {
                Some(UpdateStatusDto::Available { build, .. }) => Some(*build),
                _ => None,
            })
    }

    pub fn auto_check_enabled() -> bool {
        storage_get(AUTO_CHECK_KEY).as_deref() != Some("0")
    }

    pub fn set_auto_check(enabled: bool) {
        storage_set(AUTO_CHECK_KEY, if enabled { "1" } else { "0" });
    }

    /// Vérifie maintenant (`automatic` : limité à une requête toutes les 6 heures côté backend).
    pub async fn check_now(self, automatic: bool) -> Result<(), String> {
        let check = api::check_for_update(UpdateCheckInput { automatic })
            .await
            .map_err(|error| error.message)?;
        self.check.set(Some(check));
        Ok(())
    }

    pub fn install(self) {
        if self.installing.get_untracked() {
            return;
        }
        self.installing.set(true);
        self.progress.set(None);
        self.message.set(None);
        spawn_local(async move {
            let message = match api::install_update().await {
                Ok(InstallOutcomeDto::Started) => {
                    "L'installeur Android s'ouvre : touchez « Mettre à jour ». Vos données sont conservées."
                        .to_string()
                }
                Ok(InstallOutcomeDto::PermissionRequired) => {
                    "Autorisez Liquid Compagnon à installer des applications dans l'écran qui s'ouvre, puis revenez toucher « Mettre à jour »."
                        .to_string()
                }
                Ok(InstallOutcomeDto::Unsupported) => {
                    "La mise à jour depuis l'application n'est disponible que sur Android.".to_string()
                }
                Err(error) => error.message,
            };
            self.message.set(Some(message));
            self.installing.set(false);
        });
    }
}

pub fn use_updates() -> Updates {
    expect_context()
}

/// Vérification au déverrouillage (si l'option est active), puis invitation pour un build pas
/// encore reporté. Monté dans la porte de session : une fois par déverrouillage.
#[component]
pub fn UpdateWatcher() -> impl IntoView {
    let updates = use_updates();
    if Updates::auto_check_enabled() {
        spawn_local(async move {
            // Silencieux au démarrage : une erreur réseau n'interrompt rien.
            if updates.check_now(true).await.is_ok()
                && let Some(build) = updates.available_build()
                && storage_get(POSTPONED_KEY) != Some(build.to_string())
            {
                updates.prompt.set(true);
            }
        });
    }

    let postpone = move |_| {
        if let Some(build) = updates.available_build() {
            storage_set(POSTPONED_KEY, &build.to_string());
        }
        updates.prompt.set(false);
    };

    view! {
        <BottomSheet open=updates.prompt title="Nouvelle version disponible">
            <div class="flex flex-col gap-4">
                <AvailableUpdate />
                <InstallControls />
                <button class=BTN_GHOST disabled=move || updates.installing.get() on:click=postpone>"Plus tard"</button>
            </div>
        </BottomSheet>
    }
}

/// Description de la version proposée.
#[component]
pub fn AvailableUpdate() -> impl IntoView {
    let updates = use_updates();
    move || {
        updates.check.with(|check| match check.as_ref().map(|c| &c.status) {
            Some(UpdateStatusDto::Available { build, version, notes, size }) => Some(view! {
                <div class="flex flex-col gap-1">
                    <p class="font-medium">{format!("Version {version} · build {build}")}</p>
                    {(!notes.is_empty()).then(|| view! { <p class="text-sm text-muted">{notes.clone()}</p> })}
                    <p class="text-xs text-muted">{format!("{:.1} Mo à télécharger", *size as f64 / 1_048_576.0)}</p>
                </div>
            }),
            _ => None,
        })
    }
}

/// Bouton « Mettre à jour », progression et issue de l'installation.
#[component]
pub fn InstallControls() -> impl IntoView {
    let updates = use_updates();
    view! {
        <div class="flex flex-col gap-3">
            <button class=BTN_PRIMARY disabled=move || updates.installing.get() on:click=move |_| updates.install()>
                {move || if updates.installing.get() { "Téléchargement…" } else { "Mettre à jour" }}
            </button>
            {move || updates.installing.get().then(|| {
                let percent = updates.progress.with(|p| p.map_or(0, |p| p.received.saturating_mul(100) / p.total.max(1)));
                view! {
                    <div class="flex flex-col gap-1" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow=percent.to_string()>
                        <div class="h-2 overflow-hidden rounded-full bg-surface-raised">
                            <div class="h-full rounded-full bg-primary shadow-glow-primary transition-all" style=format!("width: {percent}%")></div>
                        </div>
                        <span class="text-xs text-muted">{format!("{percent} %")}</span>
                    </div>
                }
            })}
            {move || updates.message.get().map(|text| view! { <p class="text-sm text-secondary-light" role="status">{text}</p> })}
        </div>
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn storage_get(key: &str) -> Option<String> {
    storage()?.get_item(key).ok().flatten()
}

fn storage_set(key: &str, value: &str) {
    if let Some(storage) = storage() {
        let _ = storage.set_item(key, value);
    }
}
