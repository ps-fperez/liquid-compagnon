## ADDED Requirements

### Requirement: Virement entre comptes
L'utilisateur SHALL pouvoir enregistrer un virement d'un compte vers un autre. Chacun des deux côtés est un compte suivi (non archivé) ou un compte hors patrimoine, et au moins un côté MUST être un compte suivi. Les deux côtés MUST être différents. Le virement a un montant envoyé, un montant reçu (égal au montant envoyé par défaut), un libellé facultatif et une fréquence : ponctuel (date), mensuel (jour du mois, à partir d'un mois, fin facultative) ou annuel (mois et jour, à partir d'une année, fin facultative).

#### Scenario: Rapatriement ponctuel
- **WHEN** l'utilisateur enregistre un virement ponctuel de 100 € du LDDS vers son compte courant
- **THEN** le LDDS baisse de 100 € et le compte courant monte de 100 €

#### Scenario: Virement vers un compte hors patrimoine
- **WHEN** l'utilisateur vire 600 € du LDDS vers « Compte joint », créé à la volée comme compte hors patrimoine
- **THEN** le LDDS baisse de 600 € et le patrimoine total baisse de 600 €

#### Scenario: Virement mensuel
- **WHEN** un virement mensuel de 850 € du compte courant vers le LDDS est programmé le 1er du mois
- **THEN** à chaque 1er du mois, le LDDS monte de 850 € et le compte courant baisse de 850 €, sans saisie

#### Scenario: Côtés identiques
- **WHEN** l'utilisateur choisit le même compte au départ et à l'arrivée
- **THEN** le virement est refusé avec le message « Choisissez deux comptes différents »

### Requirement: Montant reçu différent
Un virement SHALL pouvoir créditer un montant différent du montant envoyé, par exemple un abondement net de prélèvements sociaux. L'application MUST utiliser les deux montants tels que saisis, sans calcul de frais. Le compte de départ baisse du montant envoyé, celui d'arrivée monte du montant reçu.

#### Scenario: Épargne salariale abondée
- **WHEN** l'utilisateur programme un virement annuel en janvier de 500 € envoyés depuis son compte courant et 1 493,30 € reçus sur son épargne salariale
- **THEN** la feuille indique « Abondement net : +993,30 € » et la projection ajoute 1 493,30 € à l'épargne salariale chaque mois de janvier

### Requirement: Effet des virements sur les soldes
Les comptes à solde (compte courant, livrets, assurance-vie ou PER, espèces d'un PEA ou d'un portefeuille crypto) SHALL avoir un solde de référence daté et horodaté. Leur solde courant MUST valoir la référence plus les flux entrants moins les flux sortants postérieurs à la référence :
- un virement ponctuel compte s'il a été saisi après la référence et qu'il est daté du jour de la référence ou après ;
- une occurrence de virement récurrent compte si sa date est postérieure au jour de la référence.

Un virement vers une épargne salariale MUST seulement alimenter la projection : sa valeur reste celle des fonds.

#### Scenario: Virement déjà inclus dans le solde saisi
- **WHEN** le virement mensuel du 1er a eu lieu et l'utilisateur saisit le 3 le solde de son LDDS
- **THEN** ce solde n'est pas augmenté du virement du 1er ; le suivant, le 1er du mois prochain, s'y ajoute

### Requirement: Contrôle du plafond
Lors de la saisie d'un virement vers un livret à plafond, l'application SHALL afficher le solde avant et après la prochaine occurrence. Elle MUST signaler un dépassement du plafond (« Ce virement dépasserait le plafond de 12 000 € »), sans bloquer la saisie.

#### Scenario: Plafond dépassé
- **WHEN** le LDDS est à 11 600 € et l'utilisateur programme un virement de 850 €
- **THEN** l'avertissement de dépassement du plafond s'affiche dans la feuille

### Requirement: Comptes hors patrimoine
Un compte hors patrimoine SHALL n'avoir qu'un nom. Il n'entre ni dans le patrimoine ni dans la projection. Il MUST pouvoir être renommé, et supprimé seulement s'il n'est utilisé par aucun virement.

#### Scenario: Suppression refusée
- **WHEN** l'utilisateur supprime « Compte joint » alors qu'un virement l'utilise
- **THEN** la suppression est refusée avec le message « Ce compte est utilisé par des virements »

### Requirement: Virements dans le budget et la projection
Dans le budget du mois, les virements vers un compte hors patrimoine SHALL compter comme des sorties, et ceux qui en proviennent comme des entrées. Les virements entre comptes suivis MUST être neutres. Dans la projection, chaque compte reçoit ses virements récurrents au mois de leur échéance (mensuel : chaque mois ; annuel : le mois choisi) et perd ses virements récurrents sortants. Seuls les virements issus d'un compte courant ou d'un compte hors patrimoine consomment la capacité d'épargne ; leur dépassement de la capacité reste signalé.

#### Scenario: Sortie vers le compte joint
- **WHEN** l'utilisateur vire 600 € du LDDS vers le compte joint en octobre
- **THEN** le budget d'octobre affiche 600 € de « Virements hors patrimoine », retranchés de la capacité d'épargne du mois

### Requirement: Fil des mouvements
La fiche d'un compte à solde SHALL lister les mouvements comptés depuis le dernier solde saisi, du plus récent au plus ancien : dépenses, revenus, prélèvements, virements entrants et sortants. Chaque ligne affiche sa nature, sa date, son libellé ou le compte d'en face, et son montant signé. La fiche MUST aussi lister les virements programmés du compte, modifiables et supprimables, et proposer « Nouveau virement » pré-rempli avec ce compte.

#### Scenario: Fil du compte courant
- **WHEN** l'utilisateur ouvre son compte courant après un prélèvement de 15,99 € et un rapatriement de 100 € depuis le LDDS
- **THEN** les deux mouvements apparaissent avec leur nature (« Charge fixe », « Virement »), et le solde en tient compte
