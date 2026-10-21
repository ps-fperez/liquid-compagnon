## Why

À l'usage sur le téléphone, l'application manque de retours :
- une dépense saisie avec son libellé est refusée (« Saisissez un montant valide ») alors que le montant est affiché ;
- aucun message ne confirme un enregistrement ;
- rien ne montre qu'un chargement est en cours ;
- on ne voit pas si une clé Twelve Data est enregistrée ;
- les avertissements (rappel d'export, cours indisponibles) ne se ferment pas.

## What Changes

- **Correctif — champs de saisie fiables sous Android.** La valeur n'est plus réécrite dans le champ pendant la frappe (conflit avec la saisie prédictive du clavier). Elle est synchronisée à la frappe, à la validation et à la sortie du champ.
- **Aide et erreurs sous les champs.** Texte d'aide, erreur propre à chaque champ, aperçu du montant reconnu.
- **Notifications (toasts).** Succès, erreur, avertissement, information. Elles disparaissent seules et se ferment d'un geste.
- **Chargement visible.**
  - une barre de progression en haut de l'écran pendant les échanges avec le cœur de l'application ;
  - une roue de chargement dans les boutons en cours d'action.
- **Bandeaux d'avertissement refermables.** Rappel d'export, cours indisponibles : le choix est mémorisé sur l'appareil.
- **État de la clé Twelve Data.** Une pastille « Clé enregistrée » ou « Aucune clé », avec l'offre et les crédits du jour.
- **Saisie détaillée d'une dépense.** Quand « Date et libellé » est ouvert, toucher une catégorie la sélectionne et un bouton « Enregistrer » valide la saisie. La saisie rapide en trois gestes reste inchangée.

## Capabilities

### New Capabilities
- `interaction-feedback` : retours visuels des actions (notifications, chargements, erreurs par champ, bandeaux refermables) et fiabilité des champs de saisie.

### Modified Capabilities
<!-- Aucune spécification principale synchronisée : décrit dans la nouvelle capacité. -->

## Impact

- **UI uniquement.**
  - composants `Field`, `Toast`, `Notice`, `Spinner` et `LoadingBar` ;
  - `api::call` compte les appels en cours ;
  - `run_action` notifie les erreurs ;
  - formulaires de dépense, de revenu, de solde, de clé, de sauvegarde.
- **Aucun changement** du backend ni du format des données.