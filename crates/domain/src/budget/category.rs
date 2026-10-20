use crate::{CategoryId, Label};

/// Catégorie de dépenses. Une catégorie utilisée n'est jamais supprimée : elle est archivée et
/// n'est alors plus proposée à la saisie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    id: CategoryId,
    name: Label,
    archived: bool,
}

impl Category {
    pub fn new(id: CategoryId, name: Label) -> Self {
        Self::restore(id, name, false)
    }

    pub fn restore(id: CategoryId, name: Label, archived: bool) -> Self {
        Self { id, name, archived }
    }

    pub fn id(&self) -> CategoryId {
        self.id
    }

    pub fn name(&self) -> &Label {
        &self.name
    }

    pub fn is_archived(&self) -> bool {
        self.archived
    }

    /// Vrai si la catégorie peut être choisie pour une nouvelle saisie.
    pub fn is_selectable(&self) -> bool {
        !self.archived
    }

    pub fn rename(&mut self, name: Label) {
        self.name = name;
    }

    pub fn archive(&mut self) {
        self.archived = true;
    }

    pub fn unarchive(&mut self) {
        self.archived = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archived_category_is_no_longer_selectable() {
        let mut leisure = Category::new(CategoryId::generate(), Label::new("Loisirs").unwrap());
        assert!(leisure.is_selectable());

        leisure.archive();

        assert!(leisure.is_archived());
        assert!(!leisure.is_selectable());
    }

    #[test]
    fn rename_keeps_identity() {
        let mut category = Category::new(CategoryId::generate(), Label::new("Loisir").unwrap());
        let id = category.id();

        category.rename(Label::new("Loisirs").unwrap());

        assert_eq!(category.id(), id);
        assert_eq!(category.name().as_str(), "Loisirs");
    }
}
