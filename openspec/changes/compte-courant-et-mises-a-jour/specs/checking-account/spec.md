## ADDED Requirements

### Requirement: Compte courant
L'utilisateur SHALL pouvoir créer un compte de type « Compte courant ». Ce compte a un solde de référence, saisi à une date, qui peut être négatif (découvert) : un montant au centime près. Mettre à jour le solde MUST remplacer la référence par le montant saisi, daté d'aujourd'hui.

#### Scenario: Création et premier solde
- **WHEN** l'utilisateur crée un compte courant puis saisit un solde de 1 250,00 €
- **THEN** le compte affiche 1 250,00 € et indique « Solde saisi le » suivi de la date du jour

#### Scenario: Découvert
- **WHEN** l'utilisateur saisit un solde de −85,40 €
- **THEN** le solde est accepté, affiché en négatif, et retranché du patrimoine total

### Requirement: Rattachement des mouvements
Une dépense ponctuelle ou un revenu ponctuel SHALL pouvoir être rattaché à un compte courant non archivé, ou à aucun compte. Lors d'une nouvelle saisie, le premier compte courant actif est proposé par défaut ; l'utilisateur MUST pouvoir choisir un autre compte courant ou « Aucun compte ». Rattacher un mouvement à un compte qui n'est pas un compte courant MUST être refusé.

#### Scenario: Saisie rapide
- **WHEN** l'utilisateur possède un compte courant et saisit une dépense de 42,50 € en trois gestes
- **THEN** la dépense est rattachée à ce compte courant sans geste supplémentaire

#### Scenario: Sans compte courant
- **WHEN** l'utilisateur ne possède aucun compte courant
- **THEN** la saisie d'une dépense ne propose pas de choix de compte et la dépense n'est rattachée à aucun compte

### Requirement: Solde courant calculé
Le solde courant d'un compte courant SHALL valoir : le solde de référence, moins les dépenses rattachées, plus les revenus ponctuels rattachés. Seuls comptent les mouvements enregistrés après la saisie de la référence et datés du jour de la référence ou après. Un mouvement saisi avant la référence est réputé déjà inclus dans le solde saisi ; un mouvement daté d'avant la référence aussi, même s'il a été saisi après. Modifier ou supprimer un mouvement compté MUST mettre le solde à jour.

#### Scenario: Dépenses après la référence
- **WHEN** le solde de référence est de 1 000,00 €, puis l'utilisateur saisit une dépense de 42,50 € et un revenu ponctuel de 100,00 €, tous deux rattachés au compte
- **THEN** le solde courant est de 1 057,50 €

#### Scenario: Recalage
- **WHEN** l'utilisateur met à jour le solde à 980,00 € après ces mouvements
- **THEN** le solde courant est de 980,00 € ; les mouvements déjà saisis ne sont plus retranchés

#### Scenario: Dépense oubliée d'avant la référence
- **WHEN** le solde de référence a été saisi le 20/10 et l'utilisateur saisit le 22/10 une dépense datée du 15/10
- **THEN** cette dépense ne modifie pas le solde courant

### Requirement: Détail du compte courant
Le détail d'un compte courant SHALL afficher le solde courant, le solde de référence et sa date, ainsi que la liste des mouvements comptés depuis la référence (date, libellé, montant signé). Le compte courant MUST apparaître dans la mise à jour rapide des soldes.

#### Scenario: Mouvements depuis la référence
- **WHEN** l'utilisateur ouvre un compte courant avec deux dépenses comptées
- **THEN** les deux dépenses apparaissent sous « Depuis le dernier solde saisi », avec leur montant en négatif

### Requirement: Compte courant dans le patrimoine et la projection
Le solde courant SHALL entrer dans le patrimoine total et dans l'historique. Dans la projection, le compte courant MUST rester à son solde courant, sans rendement et sans versement programmé. Un solde négatif MUST être retranché du total et ne pas apparaître dans la barre de répartition.

#### Scenario: Projection
- **WHEN** le compte courant affiche 1 057,50 €
- **THEN** la projection à 12 mois le maintient à 1 057,50 € dans les trois scénarios

### Requirement: Sauvegarde du compte courant
L'export SHALL inclure les comptes courants (solde, date et instant de la référence) et, pour chaque dépense et revenu ponctuel, son compte rattaché et son instant d'enregistrement. Le format passe en version 2. Un fichier en version 1 MUST rester importable : il n'a ni compte courant ni rattachement. Un rattachement à un compte absent du fichier ou qui n'est pas un compte courant MUST faire refuser l'import.

#### Scenario: Aller-retour
- **WHEN** l'utilisateur exporte puis réimporte ses données avec un compte courant et des dépenses rattachées
- **THEN** le solde courant affiché après l'import est identique

#### Scenario: Ancien fichier
- **WHEN** l'utilisateur importe une sauvegarde en version 1
- **THEN** l'import réussit et aucune dépense n'est rattachée à un compte
