## ADDED Requirements

### Requirement: Salaire mensuel avec date d'effet
L'application SHALL permettre de saisir le salaire net mensuel (net à payer) avec une date d'effet (mois). Le salaire applicable à un mois donné MUST être celui dont la date d'effet est la plus récente sans dépasser ce mois.

#### Scenario: Première saisie
- **WHEN** l'utilisateur saisit un salaire de 3 200,00 € à effet de janvier 2026
- **THEN** le revenu mensuel de janvier 2026 et des mois suivants est 3 200,00 €

#### Scenario: Augmentation
- **WHEN** un salaire de 3 400,00 € à effet de septembre 2026 est ajouté
- **THEN** les mois jusqu'à août 2026 restent à 3 200,00 € et les mois à partir de septembre 2026 passent à 3 400,00 €

### Requirement: Historique des salaires
L'application SHALL afficher l'historique des salaires par date d'effet décroissante et permettre de modifier ou supprimer une entrée.

#### Scenario: Suppression d'une entrée
- **WHEN** l'utilisateur supprime l'entrée de septembre 2026
- **THEN** le salaire de janvier 2026 redevient applicable à partir de septembre 2026

### Requirement: Revenus ponctuels
L'application SHALL permettre de saisir des revenus ponctuels (prime, remboursement, cadeau) avec un libellé, un montant et une date. Ils MUST être comptés dans le revenu du mois concerné uniquement.

#### Scenario: Prime de fin d'année
- **WHEN** l'utilisateur saisit une prime de 1 500,00 € datée du 15 décembre 2026
- **THEN** le revenu total de décembre 2026 inclut ces 1 500,00 € et celui de janvier 2027 ne les inclut pas

### Requirement: Validation des montants
Tout montant de revenu MUST être strictement positif, en euros, avec au plus 2 décimales.

#### Scenario: Montant négatif
- **WHEN** l'utilisateur saisit -100 comme salaire
- **THEN** la saisie est refusée avec le message « Saisissez un montant positif »
