## 1. Fiabilité et aide à la saisie

- [x] 1.1 `Field` : écriture DOM seulement si la valeur diffère, synchronisation sur input/change/blur, texte d'aide, erreur par champ ; idem recherche d'actif
- [x] 1.2 Montants : aperçu du montant reconnu ou erreur sous le champ (dépense, revenu, solde, charge, position)
- [x] 1.3 Dépense : en saisie détaillée, catégorie sélectionnée puis « Enregistrer la dépense » ; erreurs par champ

## 2. Retours globaux

- [x] 2.1 Notifications : contexte `Toasts`, `ToastHost`, icônes, `run_action` qui notifie les erreurs, succès sur les enregistrements principaux
- [x] 2.2 Chargement : compteur d'appels dans `api::call`, `LoadingBar`, `Spinner` et `Busy` dans les boutons d'action
- [x] 2.3 `Notice` refermable (échéance mémorisée) : rappel d'export, cours indisponibles, versements excessifs
- [x] 2.4 Réglages : état de la clé Twelve Data (pastille, offre, crédits)

## 3. Vérification

- [x] 3.1 e2e : dépense avec libellé (frappe réelle), notification de succès, aide sous le montant, bandeau refermé ; check, smoke, e2e au vert ; push et APK