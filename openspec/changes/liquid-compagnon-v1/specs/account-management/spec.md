## ADDED Requirements

### Requirement: Types de comptes
L'application SHALL gérer les types de comptes Livret A, LDDS, PEA, PEI et Portefeuille crypto. L'utilisateur MUST pouvoir créer plusieurs comptes d'un même type (ex. deux portefeuilles crypto), chacun avec un nom et une couleur d'identification.

#### Scenario: Création d'un compte
- **WHEN** l'utilisateur crée un compte de type PEA nommé « PEA Bourso »
- **THEN** le compte apparaît dans la liste des comptes et sur le dashboard avec une valeur de 0,00 €

### Requirement: Comptes à solde (Livret A, LDDS)
Les comptes Livret A et LDDS SHALL être définis par un solde, un taux annuel et un plafond de versement. Le taux et le plafond MUST être pré-remplis avec des valeurs par défaut modifiables, et l'utilisateur MUST pouvoir mettre à jour le solde à tout moment.

#### Scenario: Mise à jour du solde
- **WHEN** l'utilisateur met à jour le solde du Livret A à 10 640,00 €
- **THEN** la valeur du compte devient 10 640,00 € et la date de mise à jour est enregistrée

#### Scenario: Dépassement du plafond
- **WHEN** l'utilisateur saisit un solde supérieur au plafond de versement
- **THEN** le solde est accepté (intérêts capitalisés possibles) et une information indique que le plafond de versement est atteint

### Requirement: Positions des comptes PEA et crypto
Les comptes PEA et crypto SHALL contenir des positions définies par un actif (identifiant de cotation), une quantité décimale et un prix de revient unitaire optionnel. La valeur d'une position MUST être quantité × dernier cours connu, et la valeur du compte la somme de ses positions plus un solde espèces optionnel.

#### Scenario: Ajout d'une position crypto
- **WHEN** l'utilisateur ajoute 0,15 BTC à son portefeuille crypto, cours à 60 000,00 €
- **THEN** la position vaut 9 000,00 €

#### Scenario: Recherche d'actif
- **WHEN** l'utilisateur tape « CW8 » ou « bitcoin » dans la recherche d'actif
- **THEN** les actifs correspondants de la source de cotation sont proposés avec leur nom et leur marché

#### Scenario: Plus-value latente
- **WHEN** un prix de revient est renseigné
- **THEN** la plus ou moins-value latente en euros et en pourcentage est affichée sur la position

### Requirement: Fonds du PEI
Un compte PEI SHALL contenir des fonds définis par un nom, un nombre de parts, et un actif de référence coté (ex. ETF MSCI World) optionnel. L'utilisateur MUST pouvoir saisir une valeur de part manuelle datée pour chaque fonds.

#### Scenario: Fonds rattaché à un indice
- **WHEN** l'utilisateur crée le fonds « Amundi Actions Monde » avec 120 parts, une valeur de part de 25,00 € au 1er octobre 2026 et l'actif de référence IWDA
- **THEN** le fonds est valorisé selon les règles de valorisation PEI de la capacité `market-pricing`

#### Scenario: Fonds sans indice
- **WHEN** un fonds n'a pas d'actif de référence (fonds monétaire)
- **THEN** sa valeur est nombre de parts × dernière valeur de part manuelle

### Requirement: Versement mensuel programmé
Chaque compte SHALL pouvoir porter un montant de versement mensuel programmé, utilisé par la projection.

#### Scenario: Versements répartis
- **WHEN** l'utilisateur programme 300 € sur le PEA et 200 € sur le Livret A
- **THEN** la projection affecte ces montants à ces comptes chaque mois

### Requirement: Archivage d'un compte
L'utilisateur SHALL pouvoir archiver un compte clôturé ; un compte archivé MUST être exclu du patrimoine courant mais conservé dans les snapshots historiques.

#### Scenario: Clôture d'un compte
- **WHEN** l'utilisateur archive un portefeuille crypto
- **THEN** il disparaît du total actuel et la courbe historique reste inchangée
