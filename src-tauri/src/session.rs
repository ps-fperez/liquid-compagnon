//! Session : l'application démarre verrouillée et se reverrouille après 60 s en arrière-plan.
//! L'état est tenu côté Rust ; l'UI ne peut que le consulter ou demander un verrouillage.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use lc_shared_dto::{ErrorCode, ErrorDto};

#[derive(Debug, Default)]
struct SessionState {
    unlocked: bool,
    backgrounded_at: Option<Instant>,
}

#[derive(Debug, Default)]
pub struct Session {
    state: Mutex<SessionState>,
}

impl Session {
    /// Durée en arrière-plan au-delà de laquelle l'application se reverrouille.
    pub const AUTO_LOCK_AFTER: Duration = Duration::from_secs(60);

    pub fn is_unlocked(&self) -> bool {
        self.with(|state| state.unlocked)
    }

    pub fn unlock(&self) {
        self.with(|state| {
            state.unlocked = true;
            state.backgrounded_at = None;
        });
    }

    pub fn lock(&self) {
        self.with(|state| state.unlocked = false);
    }

    /// Garde des commandes métier.
    pub fn require_unlocked(&self) -> Result<(), ErrorDto> {
        if self.is_unlocked() {
            Ok(())
        } else {
            Err(ErrorDto::new(
                ErrorCode::SessionLocked,
                "Déverrouillez l'application",
            ))
        }
    }

    /// L'application passe en arrière-plan (perte de focus, page masquée). Le premier instant
    /// est conservé si l'événement est reçu plusieurs fois.
    pub fn went_to_background(&self, at: Instant) {
        self.with(|state| {
            state.backgrounded_at.get_or_insert(at);
        });
    }

    /// L'application revient au premier plan. Renvoie `true` si elle vient d'être verrouillée
    /// parce qu'elle est restée trop longtemps en arrière-plan.
    pub fn came_to_foreground(&self, at: Instant) -> bool {
        self.with(|state| {
            let expired = state
                .backgrounded_at
                .take()
                .is_some_and(|since| at.saturating_duration_since(since) >= Self::AUTO_LOCK_AFTER);
            let locked_now = expired && state.unlocked;
            if expired {
                state.unlocked = false;
            }
            locked_now
        })
    }

    fn with<T>(&self, f: impl FnOnce(&mut SessionState) -> T) -> T {
        // Un verrou empoisonné ne doit jamais déverrouiller : on repart de l'état verrouillé.
        let mut guard = self.state.lock().unwrap_or_else(|poisoned| {
            let mut guard = poisoned.into_inner();
            guard.unlocked = false;
            guard
        });
        f(&mut guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_locked_and_guards_commands() {
        let session = Session::default();
        assert!(!session.is_unlocked());
        assert_eq!(
            session.require_unlocked().unwrap_err().code,
            ErrorCode::SessionLocked
        );

        session.unlock();
        assert!(session.require_unlocked().is_ok());
    }

    // Spec app-security : « Retour rapide au premier plan »
    #[test]
    fn short_background_keeps_session_open() {
        let session = Session::default();
        session.unlock();
        let start = Instant::now();

        session.went_to_background(start);
        let locked = session.came_to_foreground(start + Duration::from_secs(59));

        assert!(!locked);
        assert!(session.is_unlocked());
    }

    // Spec app-security : « Déverrouillage par PIN ou biométrie » (après 60 s en arrière-plan)
    #[test]
    fn long_background_locks_session() {
        let session = Session::default();
        session.unlock();
        let start = Instant::now();

        session.went_to_background(start);
        session.went_to_background(start + Duration::from_secs(30)); // événement en double
        let locked = session.came_to_foreground(start + Duration::from_secs(60));

        assert!(locked);
        assert!(!session.is_unlocked());
    }

    #[test]
    fn foreground_without_background_changes_nothing() {
        let session = Session::default();
        session.unlock();
        assert!(!session.came_to_foreground(Instant::now()));
        assert!(session.is_unlocked());
    }

    #[test]
    fn unlocking_clears_pending_background() {
        let session = Session::default();
        let start = Instant::now();
        session.went_to_background(start);
        session.unlock();
        assert!(!session.came_to_foreground(start + Duration::from_secs(120)));
        assert!(session.is_unlocked());
    }
}
