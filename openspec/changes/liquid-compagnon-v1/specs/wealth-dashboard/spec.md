## ADDED Requirements

### Requirement: Patrimoine total
Le dashboard SHALL afficher le patrimoine total, égal à la somme des valeurs de tous les comptes non archivés, arrondi au centime et formaté en euros à la française (ex. « 48 362,15 € »).

#### Scenario: Agrégation
- **WHEN** les comptes valent 10 640,00 €, 5 200,00 €, 14 510,00 €, 9 310,00 € et 8 702,00 €
- **THEN** le patrimoine total affiché est 48 362,00 €

### Requirement: Découpage par compte
Le dashboard SHALL lister chaque compte avec son nom, sa valeur, sa part du total en pourcentage, sa variation et la provenance de sa valorisation, et MUST afficher une barre de répartition colorée selon la couleur de chaque compte. Un appui sur un compte MUST ouvrir son détail.

#### Scenario: Répartition
- **WHEN** le PEA vaut 14 510,00 € sur un total de 48 362,00 €
- **THEN** sa part affichée est 30,0 %

### Requirement: Variation sur la période
Le dashboard SHALL afficher la variation du patrimoine total et de chaque compte depuis le dernier snapshot mensuel, en euros et en pourcentage, en couleur « gain » si positive et « perte » si négative.

#### Scenario: Hausse mensuelle
- **WHEN** le snapshot du 1er octobre valait 47 230,00 € et le total actuel est 48 362,00 €
- **THEN** la variation affichée est +1 132,00 € (+2,4 %)

### Requirement: Snapshots mensuels automatiques
L'application SHALL enregistrer un snapshot (valeur de chaque compte et total) à la première ouverture de chaque mois, et l'utilisateur MUST pouvoir en enregistrer un manuellement. Un seul snapshot automatique MUST exister par mois.

#### Scenario: Premier lancement du mois
- **WHEN** l'utilisateur ouvre l'application le 3 novembre sans snapshot pour novembre
- **THEN** un snapshot daté de novembre est créé avec les valeurs du moment

#### Scenario: Mois sans ouverture
- **WHEN** l'application n'a pas été ouverte en décembre
- **THEN** aucun snapshot n'est inventé pour décembre et la courbe relie novembre à janvier

### Requirement: Courbe d'évolution
Le dashboard SHALL afficher une courbe d'évolution du patrimoine total à partir des snapshots, avec des périodes sélectionnables 6 mois, 1 an et tout l'historique.

#### Scenario: Historique insuffisant
- **WHEN** moins de 2 snapshots existent
- **THEN** la courbe est remplacée par le message « Votre courbe apparaîtra après votre deuxième mois »

### Requirement: Indicateurs budgétaires
Le dashboard SHALL afficher le revenu du mois, les charges fixes mensuelles, les dépenses ponctuelles du mois en cours et la capacité d'épargne mensuelle.

#### Scenario: Synthèse du mois
- **WHEN** l'utilisateur consulte le dashboard
- **THEN** les quatre indicateurs budgétaires sont visibles sous le découpage par compte
