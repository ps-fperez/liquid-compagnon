## Why

L'accueil répond à « combien je possède », pas à la question du quotidien : « combien je peux encore dépenser ce mois-ci ? ». Le calcul existe en morceaux (revenus, charges, virements programmés, dépenses), mais l'utilisateur le refait de tête : il garde par exemple 250 € sur son compte perso après les virements de début de mois, et vérifie à la main que ses abonnements et ses petites dépenses tiennent dedans.

Les quatre tuiles budget de l'accueil répètent l'onglet Budget sans dire quoi faire. Les notifications s'empilent en haut et masquent le patrimoine, et l'application manque des petites animations qui rendent les chiffres vivants.

Cette livraison 2 de la v0.3 ajoute le **reste à dépenser**, réorganise l'accueil autour de lui, et refond les notifications et les animations. Les objectifs et le bilan du mois suivent en livraison 3.

## What Changes

- **Reste à dépenser du mois.**
  - Revenus du mois, plus les rapatriements d'épargne vers un compte courant, moins les charges fixes, l'épargne programmée du mois et les virements hors patrimoine partis d'un compte courant, moins les dépenses du mois.
  - Montant par jour restant, jauge « dépensé / disponible » avec le repère du jour, état du rythme : « Dans le rythme », « Rythme élevé », « Budget dépassé ».
  - Feuille de détail qui déroule le calcul, projette la fin du mois au rythme actuel, dit combien dépenser par jour pour finir à l'équilibre, et montre où part l'argent (trois premières catégories).
- **Accueil réorganisé** : patrimoine, puis reste à dépenser, puis comptes et évolution. Les quatre tuiles budget disparaissent ; la synthèse de l'onglet Budget gagne la ligne « Reste à dépenser ».
- **Budget plus juste avec les comptes hors patrimoine** : un virement reçu de l'extérieur n'est un revenu que s'il arrive sur un compte courant ; un virement de l'extérieur vers une épargne (dont « Compte courant (non suivi) ») est de l'épargne financée par les revenus et n'augmente plus la capacité d'épargne.
- **Une seule notification, en bas**, au-dessus de la barre de navigation, avec une action : « Annuler » pendant 5 s après l'ajout d'une dépense, d'un revenu ponctuel ou d'un virement. Elle passe en haut quand une feuille est ouverte, pour ne pas masquer son bouton.
- **Animations** : montants clés qui défilent jusqu'à leur valeur, jauge qui se remplit, listes qui entrent en cascade, fondu glissé entre onglets, halo sur le bouton +. Toutes désactivées si le système demande de réduire les animations.

## Capabilities

### New Capabilities
- `spending-allowance` : calcul du reste à dépenser, rythme, projection de fin de mois, détail.
- `home-dashboard` : organisation de l'accueil autour du patrimoine et du reste à dépenser.
- `motion-feedback` : notification unique avec action d'annulation, animations et réduction des animations.

### Modified Capabilities
<!-- Spécifications principales non synchronisées : décrit dans les nouvelles capacités. -->

## Impact

- **Domaine.** `SpendingAllowance` (calcul, rythme, projection) ; le calculateur de budget distingue les comptes courants pour les flux de virements.
- **Application.** Synthèse budgétaire avec le reste à dépenser et les catégories du mois ; tableau de bord inchangé par ailleurs.
- **UI.** Carte et feuille du reste à dépenser ; accueil ; notification unique avec action ; animations (montants, jauges, cascades, transitions d'onglets).
- **Aucune** donnée stockée en plus, aucun changement de sauvegarde ; rien ne quitte le téléphone.