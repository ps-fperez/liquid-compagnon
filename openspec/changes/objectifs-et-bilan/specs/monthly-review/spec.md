## ADDED Requirements

### Requirement: Ouverture du bilan
Au premier déverrouillage d'un mois, l'application SHALL ouvrir le bilan du mois précédent si elle contient des données d'avant le mois en cours et que ce bilan n'a pas encore été ouvert. Le bilan MUST s'afficher en plein écran, en cartes qui défilent seules toutes les 5 secondes ; toucher la droite ou la gauche avance ou revient ; une croix ferme le bilan.

#### Scenario: Début de mois
- **WHEN** l'utilisateur déverrouille l'application le 1er novembre et n'a pas vu le bilan d'octobre
- **THEN** le bilan d'octobre s'ouvre ; au déverrouillage suivant, il ne s'ouvre plus

#### Scenario: Première utilisation
- **WHEN** l'application vient d'être installée ce mois-ci
- **THEN** aucun bilan ne s'ouvre

### Requirement: Contenu du bilan
Le bilan d'un mois SHALL présenter : le montant mis de côté (revenus et entrées extérieures sur un compte courant, moins charges fixes, dépenses et virements d'un compte courant vers l'extérieur) comparé au mois précédent ; les entrées et sorties du mois (revenus ponctuels signalés, poids des charges fixes et des dépenses) ; les trois catégories les plus dépensées et la plus forte hausse sur un mois ; le patrimoine en fin de mois et sa variation sur le mois si les snapshots le permettent ; la progression de chaque objectif comparée au mois précédent. Une carte sans donnée MUST être omise.

#### Scenario: Mis de côté
- **WHEN** octobre compte 3 450 € de revenus, 1 300 € de charges fixes et 1 540 € de dépenses, et septembre 420 € mis de côté
- **THEN** la première carte indique 610 € mis de côté, « +190 € par rapport à septembre »

#### Scenario: Hausse d'une catégorie
- **WHEN** les loisirs passent de 230 € en septembre à 310 € en octobre
- **THEN** la carte des catégories signale « Loisirs en hausse de 35 % sur un mois »

### Requirement: Recalage du compte courant
La dernière carte du bilan ouvert en début de mois SHALL demander « Votre compte courant repart à combien ? » pour le premier compte courant actif, pré-remplie avec le solde de départ saisi le mois précédent (sinon le solde actuel), et lister les virements sortants de ce compte déjà échus dans le mois. Valider MUST mettre à jour le solde du compte et enregistrer ce solde de départ.

#### Scenario: Compte perso à 250 €
- **WHEN** l'utilisateur valide 250 € le 1er novembre, après les virements du 1er vers le LDDS (850 €) et le compte joint (1 560 €)
- **THEN** le compte courant vaut 250 € et ces virements, déjà échus, ne sont pas déduits une seconde fois

#### Scenario: Mois suivant
- **WHEN** le bilan de novembre s'ouvre le 1er décembre
- **THEN** le champ est pré-rempli à 250 €

### Requirement: Historique des bilans
L'accueil SHALL proposer, dans la section Évolution, les bilans des mois passés qui contiennent des données ; en ouvrir un affiche ses cartes, sans la carte de recalage.

#### Scenario: Revoir septembre
- **WHEN** l'utilisateur touche « Septembre 2026 » dans les bilans
- **THEN** le bilan de septembre s'affiche