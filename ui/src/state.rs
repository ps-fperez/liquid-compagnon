//! État global de l'UI : session (fournie par le backend) et mode discret.

use std::cell::Cell;

use lc_shared_dto::SessionStatusDto;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

use crate::api;

thread_local! {
    /// Accès à la session depuis `api` (hors des composants) pour réagir à `session_locked`.
    static SESSION_SIGNAL: Cell<Option<RwSignal<Option<SessionStatusDto>>>> = const { Cell::new(None) };
}

/// État de la session tel que connu du backend.
#[derive(Clone, Copy)]
pub struct Session {
    pub status: RwSignal<Option<SessionStatusDto>>,
    pub error: RwSignal<Option<String>>,
}

impl Session {
    pub fn provide() -> Self {
        let session = Self {
            status: RwSignal::new(None),
            error: RwSignal::new(None),
        };
        SESSION_SIGNAL.with(|cell| cell.set(Some(session.status)));
        provide_context(session);
        session
    }

    /// Relit l'état de la session auprès du backend.
    pub fn refresh(self) {
        spawn_local(async move {
            match api::session_status().await {
                Ok(status) => {
                    self.error.set(None);
                    self.status.set(Some(status));
                }
                Err(error) => self.error.set(Some(error.message)),
            }
        });
    }
}

pub fn use_session() -> Session {
    expect_context()
}

/// Appelé quand le backend refuse une commande parce que la session est verrouillée.
pub fn session_locked() {
    SESSION_SIGNAL.with(|cell| {
        if let Some(status) = cell.get() {
            status.update(|status| {
                if let Some(status) = status {
                    status.unlocked = false;
                }
            });
        }
    });
}

/// Signale au backend les passages en arrière-plan (onglet masqué, application réduite). Au
/// retour, la session peut avoir été verrouillée (plus de 60 s).
pub fn watch_visibility() {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let watched = document.clone();
    let handler = Closure::<dyn FnMut()>::new(move || {
        if watched.hidden() {
            spawn_local(async {
                let _ = api::notify_hidden().await;
            });
        } else {
            spawn_local(async {
                if api::notify_visible().await == Ok(true) {
                    session_locked();
                }
            });
        }
    });
    let _ = document
        .add_event_listener_with_callback("visibilitychange", handler.as_ref().unchecked_ref());
    handler.forget();
}

/// Mode discret : tous les montants sont masqués. Mémorisé sur l'appareil.
#[derive(Clone, Copy)]
pub struct Privacy(pub RwSignal<bool>);

const PRIVACY_KEY: &str = "lc.mode-discret";

impl Privacy {
    pub fn provide() -> Self {
        let stored = local_storage()
            .and_then(|storage| storage.get_item(PRIVACY_KEY).ok().flatten())
            .is_some_and(|value| value == "1");
        let privacy = Self(RwSignal::new(stored));
        provide_context(privacy);
        privacy
    }

    pub fn hidden(self) -> bool {
        self.0.get()
    }

    pub fn toggle(self) {
        let hidden = !self.0.get_untracked();
        self.0.set(hidden);
        if let Some(storage) = local_storage() {
            let _ = storage.set_item(PRIVACY_KEY, if hidden { "1" } else { "0" });
        }
    }
}

pub fn use_privacy() -> Privacy {
    expect_context()
}

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}
