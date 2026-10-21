## ADDED Requirements

### Requirement: Organisation de l'accueil
L'accueil SHALL afficher, dans cet ordre : le patrimoine total (variation du mois, répartition), le reste à dépenser du mois en cours, les comptes, puis l'évolution. Les tuiles revenus, charges, dépenses et capacité d'épargne MUST disparaître de l'accueil ; la synthèse de l'onglet Budget affiche le reste à dépenser du mois choisi.

#### Scenario: Chiffre du jour
- **WHEN** l'utilisateur ouvre l'application le 21 octobre
- **THEN** la carte « Reste à dépenser · octobre » apparaît sous le patrimoine, avec l'état du rythme, le montant restant, le montant par jour et la jauge

#### Scenario: Détail en un geste
- **WHEN** l'utilisateur touche la carte du reste à dépenser
- **THEN** la feuille de détail s'ouvre

#### Scenario: Aucun revenu saisi
- **WHEN** aucun salaire ni revenu n'est saisi
- **THEN** la carte propose de renseigner le salaire (lien vers le budget) au lieu d'afficher une enveloppe négative