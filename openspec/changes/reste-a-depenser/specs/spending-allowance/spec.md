## ADDED Requirements

### Requirement: Calcul du reste à dépenser
Le budget d'un mois SHALL calculer une enveloppe : revenus du mois (salaire et revenus ponctuels), plus les virements reçus de l'extérieur sur un compte courant et les rapatriements d'une épargne vers un compte courant, moins les charges fixes (équivalent mensuel), l'épargne programmée du mois (virements d'un compte courant ou de l'extérieur vers un compte suivi non courant) et les virements d'un compte courant vers l'extérieur. Toutes les occurrences du mois MUST compter. Le reste à dépenser est l'enveloppe moins les dépenses du mois.

#### Scenario: Mois type
- **WHEN** le salaire est de 3 340 €, les virements mensuels de 1 560 € vers le compte joint, 686 € vers le crédit et 850 € vers le LDDS partent du compte courant, et aucune charge ni dépense n'est saisie
- **THEN** l'enveloppe et le reste à dépenser valent 244 €

#### Scenario: Rapatriement
- **WHEN** 100 € sont virés du LDDS vers le compte courant dans le mois
- **THEN** l'enveloppe augmente de 100 €

#### Scenario: Virement payé par l'épargne
- **WHEN** 600 € partent du LDDS vers le compte joint
- **THEN** l'enveloppe ne change pas

#### Scenario: Épargne depuis un compte non suivi
- **WHEN** un virement mensuel de 200 € part de « Compte courant (non suivi) » vers le Livret A
- **THEN** l'enveloppe baisse de 200 € et la capacité d'épargne n'augmente pas

### Requirement: Rythme du mois
Pour le mois en cours, le reste à dépenser SHALL indiquer le montant par jour restant (aujourd'hui compris), le nombre de jours restants et un état : « Budget dépassé » si le reste est négatif, « Rythme élevé » si les dépenses dépassent la part de l'enveloppe correspondant aux jours écoulés de plus de 2 % de l'enveloppe, « Dans le rythme » sinon.

#### Scenario: Rythme élevé
- **WHEN** l'enveloppe est de 1 500 €, 1 088 € sont dépensés le 21 d'un mois de 31 jours
- **THEN** l'état est « Rythme élevé », il reste 412 €, soit 37,45 € par jour pendant 11 jours

#### Scenario: Dans le rythme
- **WHEN** l'enveloppe est de 250 € et 80 € sont dépensés le 15 d'un mois de 30 jours
- **THEN** l'état est « Dans le rythme »

### Requirement: Détail du reste à dépenser
Une feuille SHALL dérouler le calcul ligne à ligne, projeter le solde de fin de mois au rythme actuel des dépenses, indiquer le montant quotidien qui permet de finir à l'équilibre, et lister les trois catégories les plus dépensées du mois.

#### Scenario: Projection de fin de mois
- **WHEN** l'enveloppe est de 1 500 € et 1 088 € sont dépensés au 21 d'un mois de 31 jours
- **THEN** la feuille indique un rythme de 51,81 € par jour et une fin de mois projetée à −106,10 €