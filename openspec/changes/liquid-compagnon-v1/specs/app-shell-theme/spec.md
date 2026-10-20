## ADDED Requirements

### Requirement: Navigation mobile
L'application SHALL proposer une barre de navigation inférieure avec cinq entrées : Dashboard, Comptes, « + » (saisie rapide, centrale), Budget (revenus, charges, dépenses) et Projection. Les réglages MUST être accessibles depuis l'en-tête. Le bouton retour Android MUST revenir à l'écran précédent.

#### Scenario: Saisie rapide
- **WHEN** l'utilisateur appuie sur « + »
- **THEN** une feuille s'ouvre proposant « Dépense », « Revenu ponctuel » et « Mise à jour d'un solde »

#### Scenario: Retour Android
- **WHEN** l'utilisateur est sur le détail d'un compte et appuie sur retour
- **THEN** il revient à la liste des comptes

### Requirement: Thème sombre Purple Electro
L'application SHALL utiliser uniquement un thème sombre défini par des design tokens Tailwind : primary #A855F7, primary-strong #7C3AED, secondary #3B82F6, secondary-light #60A5FA, fond #0B0614, surface #160E26, gain #34D399, perte #F87171. Le vert et le rouge MUST être réservés aux variations et aux alertes.

#### Scenario: Ouverture de l'application
- **WHEN** le téléphone est en thème clair
- **THEN** l'application s'affiche quand même en thème sombre

### Requirement: Effets glow ciblés
L'application SHALL appliquer un effet glow uniquement sur : la carte du patrimoine total, le bouton « + », l'onglet actif, la courbe du scénario neutre, le champ ayant le focus et le flash de mise à jour d'un cours. Les listes et textes secondaires MUST NOT avoir d'effet glow.

#### Scenario: Mise à jour d'un cours
- **WHEN** le cours d'une position augmente lors d'un rafraîchissement
- **THEN** la ligne émet un bref halo vert qui s'éteint en moins d'une seconde

### Requirement: Réduction des animations
Lorsque l'option système de réduction des animations est active, les animations (pulsation du bouton « + », flashs) MUST être désactivées, les halos statiques restant affichés.

#### Scenario: Animations réduites
- **WHEN** l'option « Supprimer les animations » d'Android est activée
- **THEN** le bouton « + » ne pulse pas

### Requirement: Ergonomie mobile
Les cibles tactiles SHALL mesurer au moins 44 × 44 px, les actions principales MUST être atteignables dans la moitié inférieure de l'écran, et la saisie de montants MUST ouvrir le clavier numérique décimal. Le contraste du texte principal sur les surfaces MUST respecter WCAG AA.

#### Scenario: Saisie d'un montant
- **WHEN** l'utilisateur touche un champ de montant
- **THEN** le clavier numérique avec séparateur décimal s'ouvre

### Requirement: Format français
Les montants SHALL être affichés au format français (espace insécable comme séparateur de milliers, virgule décimale, symbole € après le montant) et les dates au format JJ/MM/AAAA. La saisie MUST accepter la virgule et le point comme séparateur décimal.

#### Scenario: Saisie avec point
- **WHEN** l'utilisateur saisit « 42.5 »
- **THEN** le montant est enregistré comme 42,50 €

### Requirement: Mode discret
L'application SHALL proposer un bouton dans l'en-tête masquant tous les montants (remplacés par « •••• ») jusqu'à un nouvel appui.

#### Scenario: Consultation en public
- **WHEN** l'utilisateur active le mode discret
- **THEN** aucun montant n'est lisible sur l'ensemble des écrans
