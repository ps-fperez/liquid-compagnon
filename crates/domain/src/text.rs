use std::fmt;

use crate::DomainError;

/// Libellé saisi par l'utilisateur (nom de compte, catégorie, charge…) : non vide, espaces
/// superflus retirés, longueur bornée.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label(String);

impl Label {
    pub const MAX_LEN: usize = 60;

    pub fn new(raw: &str) -> Result<Self, DomainError> {
        let trimmed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        if trimmed.is_empty() {
            return Err(DomainError::EmptyLabel);
        }
        if trimmed.chars().count() > Self::MAX_LEN {
            return Err(DomainError::LabelTooLong { max: Self::MAX_LEN });
        }
        Ok(Self(trimmed))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Couleur d'identification d'un compte, au format `#RRGGBB` (normalisée en majuscules).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Color(String);

impl Color {
    pub fn new(raw: &str) -> Result<Self, DomainError> {
        let raw = raw.trim();
        let valid = raw.len() == 7
            && raw.starts_with('#')
            && raw[1..].chars().all(|c| c.is_ascii_hexdigit());
        if !valid {
            return Err(DomainError::InvalidColor(raw.to_string()));
        }
        Ok(Self(raw.to_ascii_uppercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_collapses_whitespace() {
        assert_eq!(
            Label::new("  PEA   Bourso ").unwrap().as_str(),
            "PEA Bourso"
        );
    }

    #[test]
    fn label_rejects_empty_and_too_long() {
        assert_eq!(Label::new("   "), Err(DomainError::EmptyLabel));
        let long = "é".repeat(Label::MAX_LEN + 1);
        assert_eq!(
            Label::new(&long),
            Err(DomainError::LabelTooLong {
                max: Label::MAX_LEN
            })
        );
        assert!(Label::new(&"é".repeat(Label::MAX_LEN)).is_ok());
    }

    #[test]
    fn color_is_normalized_and_validated() {
        assert_eq!(Color::new("#a855f7").unwrap().as_str(), "#A855F7");
        assert!(Color::new("a855f7").is_err());
        assert!(Color::new("#A855F").is_err());
        assert!(Color::new("#GG55F7").is_err());
    }
}
