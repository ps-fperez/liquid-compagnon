## ADDED Requirements

### Requirement: Objectif lié à des comptes
L'utilisateur SHALL pouvoir créer, modifier et supprimer un objectif d'épargne : nom, montant cible positif, date d'échéance facultative (mois), au moins un compte suivi. La progression MUST être la somme des valeurs actuelles des comptes liés actifs, en pourcentage de la cible.

#### Scenario: Apport immobilier
- **WHEN** l'objectif « Apport immobilier » de 30 000 € suit le Livret A (12 840 €) et le PEA (5 760 €)
- **THEN** sa progression est de 18 600 €, soit 62 %

#### Scenario: Sans compte
- **WHEN** l'utilisateur enregistre un objectif sans compte
- **THEN** l'objectif est refusé avec le message « Choisissez au moins un compte »

### Requirement: Effort, versements et arrivée
Pour un objectif non atteint, l'application SHALL indiquer : l'effort mensuel nécessaire pour tenir l'échéance (reste à épargner divisé par le nombre de mois jusqu'à l'échéance, celui-ci compris, sans rendement), le montant versé chaque mois sur les comptes liés (équivalent mensuel net de leurs virements récurrents), et la date d'arrivée dans chacun des trois scénarios sur 10 ans. L'état MUST être « Dans les temps » si l'arrivée neutre ne dépasse pas l'échéance, « En retard » sinon.

#### Scenario: Effort mensuel
- **WHEN** il manque 11 400 € en octobre 2026 pour une échéance en décembre 2028
- **THEN** l'effort nécessaire est de 438,46 € par mois (26 mois)

#### Scenario: Dans les temps grâce aux rendements
- **WHEN** 400 € sont versés chaque mois et que la projection neutre atteint la cible en novembre 2028, avant l'échéance de décembre 2028
- **THEN** l'objectif est « Dans les temps » et affiche novembre 2028 en neutre

#### Scenario: Hors d'atteinte
- **WHEN** aucun scénario n'atteint la cible en 10 ans
- **THEN** la date d'arrivée affichée est « plus de 10 ans »

### Requirement: Objectif atteint
Quand la valeur des comptes liés atteint la cible pour la première fois, l'application SHALL enregistrer la date d'atteinte, illuminer l'anneau et féliciter l'utilisateur par une notification, une seule fois.

#### Scenario: Première atteinte
- **WHEN** le Livret A d'un objectif « Voyage au Japon » de 4 000 € passe à 4 050 €
- **THEN** l'objectif passe à « Atteint » et la notification « Objectif atteint : Voyage au Japon » s'affiche une fois

### Requirement: Objectifs à l'accueil et dans Projection
L'onglet Projection SHALL lister les objectifs (anneau, valeur et cible, échéance, état, effort, versé, dates d'arrivée) et permettre d'en créer. L'accueil MUST afficher les objectifs en anneaux défilant horizontalement, avec un accès « Tout voir » ; sans objectif, une carte propose d'en créer un.

#### Scenario: Création en une feuille
- **WHEN** l'utilisateur saisit un nom, un montant, une échéance et coche des comptes
- **THEN** l'effort mensuel s'affiche pendant la saisie et l'objectif apparaît dans la liste après l'enregistrement