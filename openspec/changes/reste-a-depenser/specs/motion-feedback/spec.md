## ADDED Requirements

### Requirement: Notification unique avec action
L'application SHALL afficher au plus une notification, en bas de l'écran au-dessus de la barre de navigation (en haut si une feuille est ouverte). Une nouvelle notification MUST remplacer la précédente. Les succès restent 5 s, les erreurs 7 s. Après l'ajout d'une dépense, d'un revenu ponctuel ou d'un virement, la notification SHALL proposer « Annuler », qui supprime l'élément ajouté et met l'écran à jour.

#### Scenario: Annuler une dépense
- **WHEN** l'utilisateur ajoute une dépense de 18,40 € puis touche « Annuler » dans la notification
- **THEN** la dépense est supprimée, les totaux reviennent à leur valeur et une notification confirme l'annulation

#### Scenario: Une seule notification
- **WHEN** deux enregistrements se suivent
- **THEN** seule la notification du second reste affichée

### Requirement: Animations
Les montants du patrimoine total et du reste à dépenser SHALL défiler jusqu'à leur valeur à l'affichage, la jauge du reste à dépenser se remplir depuis zéro, les listes de comptes et de mouvements entrer en cascade (60 ms entre les lignes), les onglets s'enchaîner par un fondu avec un léger glissement dans le sens de la navigation, et le bouton + afficher un halo au toucher. Toutes les animations MUST être désactivées quand le système demande de réduire les animations.

#### Scenario: Animations réduites
- **WHEN** l'option « Supprimer les animations » d'Android est active
- **THEN** les montants s'affichent directement à leur valeur et aucune animation ne joue