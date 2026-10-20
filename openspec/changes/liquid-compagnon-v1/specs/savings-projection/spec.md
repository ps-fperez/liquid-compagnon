## ADDED Requirements

### Requirement: Horizons de projection
L'application SHALL projeter la valeur de chaque compte et du patrimoine total à 3 mois, 6 mois et 12 mois à partir des valeurs actuelles.

#### Scenario: Sélection d'horizon
- **WHEN** l'utilisateur sélectionne l'horizon « 6 mois »
- **THEN** le patrimoine projeté à 6 mois est affiché pour chaque scénario

### Requirement: Rendements par scénario
Chaque compte SHALL avoir un rendement annuel par scénario (pessimiste, neutre, optimiste), pré-rempli par type de compte et modifiable. Pour Livret A et LDDS, le rendement MUST être par défaut le taux du compte dans les trois scénarios.

#### Scenario: Valeurs par défaut d'un PEA
- **WHEN** l'utilisateur crée un PEA
- **THEN** des rendements par défaut pessimiste, neutre et optimiste sont pré-remplis et visibles dans le paramétrage de projection

#### Scenario: Personnalisation
- **WHEN** l'utilisateur fixe le rendement optimiste de son portefeuille crypto à 40 %
- **THEN** la projection optimiste utilise 40 % pour ce compte

### Requirement: Calcul mensuel composé
La projection SHALL être calculée mois par mois : valeur(m+1) = valeur(m) × (1 + r)^(1/12) + versement programmé, où r est le rendement annuel du scénario. Le calcul MUST utiliser l'arithmétique décimale et arrondir au centime uniquement à l'affichage.

#### Scenario: Livret sans versement
- **WHEN** un livret de 10 000,00 € a un rendement de 3 % et aucun versement
- **THEN** sa valeur projetée à 12 mois est 10 300,00 €

#### Scenario: PEA avec versements
- **WHEN** un PEA de 10 000,00 € a un rendement de 0 % et un versement de 300,00 €/mois
- **THEN** sa valeur projetée à 6 mois est 11 800,00 €

### Requirement: Plafonds des livrets
Les versements projetés sur un Livret A ou un LDDS MUST s'arrêter lorsque le plafond de versement est atteint ; la part non versée SHALL être ajoutée à l'épargne non affectée.

#### Scenario: Plafond atteint
- **WHEN** un LDDS est à 11 900,00 € pour un plafond de 12 000,00 € avec un versement programmé de 200,00 €
- **THEN** le premier mois projeté reçoit 100,00 € et 100,00 € vont à l'épargne non affectée

### Requirement: Épargne non affectée
La différence entre la capacité d'épargne mensuelle et la somme des versements programmés SHALL être projetée comme « épargne non affectée » à rendement nul. Si la somme des versements dépasse la capacité d'épargne, l'application MUST afficher un avertissement.

#### Scenario: Reliquat
- **WHEN** la capacité d'épargne est 1 300,00 € et les versements programmés totalisent 900,00 €
- **THEN** 400,00 € par mois s'ajoutent à l'épargne non affectée

#### Scenario: Versements excessifs
- **WHEN** les versements programmés totalisent 1 500,00 € pour une capacité de 1 300,00 €
- **THEN** l'avertissement « Vos versements dépassent votre capacité d'épargne de 200 € » est affiché

### Requirement: Visualisation des scénarios
L'écran de projection SHALL afficher une courbe pour chaque scénario jusqu'à 12 mois, le scénario neutre mis en avant, ainsi que le tableau des valeurs projetées par compte pour l'horizon sélectionné.

#### Scenario: Lecture d'une projection
- **WHEN** l'utilisateur ouvre l'écran de projection
- **THEN** trois courbes sont visibles, la courbe neutre est mise en évidence, et les valeurs pessimiste, neutre et optimiste à l'horizon choisi sont affichées
