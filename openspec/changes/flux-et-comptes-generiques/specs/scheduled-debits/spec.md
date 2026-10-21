## ADDED Requirements

### Requirement: Charge prélevée sur un compte courant
Une charge fixe SHALL pouvoir être prélevée sur un compte courant actif, à un jour du mois (1 à 31 ; un jour absent du mois est ramené au dernier jour). À chaque échéance de sa périodicité (mensuelle, trimestrielle depuis son mois de début, annuelle), le solde du compte courant MUST baisser du montant de la charge, selon la règle du solde de référence. Une charge sans compte reste comptée dans le budget seulement, comme aujourd'hui.

#### Scenario: Abonnement prélevé
- **WHEN** l'abonnement Sosh de 15,99 € est prélevé sur le compte courant le 5 de chaque mois et que le solde de 250 € a été saisi le 1er
- **THEN** à partir du 5, le compte courant affiche 234,01 € et le fil montre « Sosh, charge fixe, −15,99 € »

#### Scenario: Jour absent du mois
- **WHEN** une charge est prélevée le 31 et que le mois compte 30 jours
- **THEN** elle est prélevée le 30

### Requirement: Saisie du prélèvement
La feuille d'une charge fixe SHALL proposer « Prélevée sur » (comptes courants actifs ou « Aucun compte ») et, si un compte est choisi, « Jour du prélèvement ». Le premier compte courant MUST être proposé par défaut pour une nouvelle charge.

#### Scenario: Nouvelle charge
- **WHEN** l'utilisateur ajoute une charge de 7,20 € (« Ionos ») le 15 du mois avec un compte courant
- **THEN** la charge est prélevée sur ce compte le 15 de chaque mois
