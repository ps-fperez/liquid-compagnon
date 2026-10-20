## ADDED Requirements

### Requirement: Catégories de dépenses
L'application SHALL fournir les catégories par défaut Logement, Transport, Alimentation, Abonnements, Santé, Loisirs, Impôts et Autres, et MUST permettre d'en créer, renommer ou archiver. Une catégorie utilisée ne MUST NOT être supprimée, seulement archivée.

#### Scenario: Archivage d'une catégorie utilisée
- **WHEN** l'utilisateur archive la catégorie « Loisirs » qui contient des dépenses
- **THEN** la catégorie n'est plus proposée à la saisie mais les dépenses existantes la conservent

### Requirement: Charges fixes récurrentes
L'application SHALL permettre de saisir des charges fixes avec libellé, montant, catégorie, périodicité (mensuelle, trimestrielle, annuelle), date de début et date de fin optionnelle. L'équivalent mensuel d'une charge MUST être son montant divisé par 1, 3 ou 12 selon sa périodicité.

#### Scenario: Charge annuelle
- **WHEN** l'utilisateur saisit une assurance habitation annuelle de 240,00 €
- **THEN** son équivalent mensuel est 20,00 €

#### Scenario: Charge terminée
- **WHEN** une charge a une date de fin en mars 2026
- **THEN** elle n'est plus comptée dans les charges à partir d'avril 2026

### Requirement: Dépenses ponctuelles
L'application SHALL permettre de saisir une dépense ponctuelle avec montant, catégorie, date (aujourd'hui par défaut) et libellé optionnel, en 3 interactions maximum depuis le bouton de saisie rapide.

#### Scenario: Saisie rapide
- **WHEN** l'utilisateur appuie sur « + », saisit 42,50, choisit « Alimentation » et valide
- **THEN** une dépense de 42,50 € datée du jour est enregistrée dans « Alimentation »

### Requirement: Liste et filtre des dépenses
L'application SHALL afficher les dépenses ponctuelles par mois, groupées par jour, avec le total du mois et un filtre par catégorie, et MUST permettre de modifier ou supprimer une dépense.

#### Scenario: Total mensuel filtré
- **WHEN** l'utilisateur filtre octobre 2026 sur « Transport »
- **THEN** seules les dépenses Transport d'octobre sont listées avec leur total

### Requirement: Capacité d'épargne mensuelle
L'application SHALL calculer la capacité d'épargne mensuelle comme : salaire applicable − somme des équivalents mensuels des charges fixes actives − moyenne des dépenses ponctuelles des 3 derniers mois complets. Si moins de 3 mois complets existent, la moyenne MUST porter sur les mois complets disponibles, et sur le mois en cours s'il n'y en a aucun.

#### Scenario: Calcul nominal
- **WHEN** le salaire est 3 200,00 €, les charges fixes représentent 1 300,00 €/mois et les dépenses ponctuelles des 3 derniers mois sont 600, 750 et 450 €
- **THEN** la capacité d'épargne mensuelle est 1 300,00 €

#### Scenario: Capacité négative
- **WHEN** les charges et dépenses dépassent le salaire
- **THEN** la capacité d'épargne négative est affichée en couleur « perte » avec un avertissement

### Requirement: Libellés suggérés
La saisie d'une dépense SHALL proposer les libellés déjà utilisés, du plus fréquent au moins fréquent, chacun avec la catégorie de sa dernière utilisation. Toucher une suggestion MUST reprendre son libellé et sa catégorie ; pour une nouvelle dépense, elle MUST être enregistrée aussitôt.

#### Scenario: Dépense habituelle
- **WHEN** l'utilisateur a déjà saisi « Courses » en Alimentation, puis saisit 10 € et touche la suggestion « Courses »
- **THEN** une dépense « Courses » de 10 € est enregistrée en Alimentation

#### Scenario: Libellé en cours de frappe
- **WHEN** l'utilisateur tape « cou » dans le libellé
- **THEN** « Courses » est proposé (sans distinction de casse ni d'accents)
