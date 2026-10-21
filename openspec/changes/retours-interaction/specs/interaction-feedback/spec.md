## ADDED Requirements

### Requirement: Champs de saisie fiables
Un champ de saisie SHALL toujours transmettre la valeur affichée, quel que soit le clavier (saisie prédictive Android comprise). Sa valeur MUST être synchronisée à la frappe, à la validation et à la sortie du champ. L'application MUST ne jamais réécrire un champ en cours de frappe si sa valeur n'a pas changé.

#### Scenario: Dépense avec libellé
- **WHEN** l'utilisateur ouvre « Date et libellé », saisit le libellé « Courses », un montant de 20 et choisit une catégorie
- **THEN** la dépense de 20,00 € « Courses » peut être enregistrée sans message d'erreur

### Requirement: Aide et erreurs par champ
Les champs de montant SHALL afficher sous le champ le montant reconnu (« 20,00 € ») ou l'erreur de saisie. Une erreur de validation MUST s'afficher sous le champ concerné et disparaître dès que l'utilisateur corrige la saisie.

#### Scenario: Montant invalide
- **WHEN** l'utilisateur saisit « 12,345 » puis valide
- **THEN** le message « Un montant a au plus 2 décimales » s'affiche sous le champ Montant

### Requirement: Notifications d'issue
Chaque enregistrement, suppression, import ou export SHALL être confirmé par une notification de succès. Chaque échec MUST produire une notification d'erreur. Les notifications de succès et d'information disparaissent après quelques secondes ; toutes MUST pouvoir être fermées d'un geste.

#### Scenario: Dépense enregistrée
- **WHEN** l'utilisateur enregistre une dépense de 20,00 €
- **THEN** la notification « Dépense de 20,00 € enregistrée » s'affiche puis disparaît

### Requirement: Indication de chargement
Pendant un échange avec le cœur de l'application de plus de 150 ms, une barre de progression SHALL apparaître en haut de l'écran. Un bouton qui lance une action MUST afficher une roue de chargement et rester désactivé jusqu'à la fin de l'action.

#### Scenario: Enregistrement en cours
- **WHEN** l'utilisateur touche « Enregistrer »
- **THEN** le bouton affiche une roue de chargement et ne peut pas être touché une seconde fois

### Requirement: Avertissements refermables
Les bandeaux d'avertissement non bloquants SHALL pouvoir être fermés : rappel d'export, cours indisponibles, versements supérieurs à la capacité. La fermeture est mémorisée sur l'appareil, et l'avertissement MUST réapparaître si la situation persiste à l'échéance : 7 jours pour le rappel d'export, le lendemain pour les autres.

#### Scenario: Rappel d'export fermé
- **WHEN** l'utilisateur ferme le rappel « Aucun export depuis 30 jours »
- **THEN** le rappel ne s'affiche plus pendant 7 jours

### Requirement: État de la clé Twelve Data
Les réglages SHALL indiquer clairement si une clé Twelve Data est enregistrée. Si elle l'est, ils affichent l'offre associée et les crédits consommés du jour, ou l'erreur de vérification.

#### Scenario: Clé enregistrée
- **WHEN** l'utilisateur ouvre les réglages avec une clé enregistrée
- **THEN** une pastille « Clé enregistrée » s'affiche avec « offre basic, 3/800 crédits aujourd'hui »