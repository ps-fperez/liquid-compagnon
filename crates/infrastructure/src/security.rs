use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use lc_application::ports::{PinHasher, StorageError, StorageResult};

/// Hachage Argon2id (paramètres par défaut recommandés) avec sel aléatoire, au format PHC.
#[derive(Debug, Clone, Copy, Default)]
pub struct Argon2PinHasher;

impl PinHasher for Argon2PinHasher {
    fn hash(&self, pin: &str) -> StorageResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(pin.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| StorageError(format!("hachage du PIN impossible : {error}")))
    }

    fn verify(&self, pin: &str, hash: &str) -> bool {
        PasswordHash::new(hash).is_ok_and(|parsed| {
            Argon2::default()
                .verify_password(pin.as_bytes(), &parsed)
                .is_ok()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_only_the_right_pin() {
        let hasher = Argon2PinHasher;
        let hash = hasher.hash("123456").unwrap();

        assert!(hash.starts_with("$argon2id$"));
        assert!(!hash.contains("123456"));
        assert!(hasher.verify("123456", &hash));
        assert!(!hasher.verify("654321", &hash));
        assert!(!hasher.verify("123456", "pas-un-hash"));
    }

    #[test]
    fn same_pin_gives_different_salted_hashes() {
        let hasher = Argon2PinHasher;
        assert_ne!(hasher.hash("1234").unwrap(), hasher.hash("1234").unwrap());
    }
}
