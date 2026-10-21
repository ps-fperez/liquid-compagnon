## Context

Retours d'usage de la v0.2 sur le téléphone. La couche UI (Leptos) est seule concernée : le backend renvoie déjà les erreurs en français et les données nécessaires.

## Goals / Non-Goals

**Goals:** fiabiliser la saisie sous Android ; donner un retour visible à chaque action (succès, erreur, chargement, avertissement).

**Non-Goals:** notifications système Android ; refonte visuelle des écrans.

## Decisions

### D1 — Champ non contrôlé pendant la frappe
- Le `prop:value` réactif réécrivait `input.value` à chaque frappe. Sous Android, une écriture pendant une composition du clavier (saisie prédictive) peut désynchroniser la valeur affichée de celle de l'application.
- `Field` garde un `NodeRef` et n'écrit dans le DOM que si la valeur du signal **diffère** de celle du champ, c'est-à-dire à une réinitialisation ou à un pré-remplissage.
- La valeur est relue sur `input`, `change` et `blur`.
- Le même principe s'applique aux champs de la recherche d'actif et de la clé.

### D2 — Notifications globales
- Le contexte `Toasts` (liste réactive) et `ToastHost` affichent les notifications en haut de l'écran, sous l'en-tête, avec `aria-live`.
- `toast::success / error / warning / info` sont appelables depuis n'importe quel gestionnaire.
- `run_action` publie l'erreur en notification ; le message reste aussi sous le formulaire.

### D3 — Chargement
- `api::call` incrémente un compteur global d'appels en cours. `LoadingBar` (barre indéterminée de 2 px) s'affiche si le compteur est positif depuis plus de 150 ms, pour éviter un clignotement.
- Composant `Spinner` (SVG), utilisé dans les boutons occupés via `<Busy busy=… />`.

### D4 — Avertissements refermables
- `Notice` (info / avertissement ; les erreurs bloquantes restent des `ErrorBanner`) avec un bouton « Fermer ».
- La fermeture est mémorisée dans `localStorage` sous une clé propre à l'avertissement, avec une échéance (`dismiss_days`).
- C'est une préférence d'appareil, comme le mode discret.

### D5 — Saisie détaillée d'une dépense
- Le mode rapide est conservé : toucher une catégorie enregistre.
- Une fois « Date et libellé » ouvert, toucher une catégorie la sélectionne, et « Enregistrer la dépense » valide la saisie.
- Cela supprime l'ambiguïté « la catégorie enregistre-t-elle déjà ? ».

## Risks / Trade-offs

- **Cause Android non reproductible sur PC.**
  - Parade : la correction supprime la classe de défauts (écritures pendant la frappe).
  - L'aperçu du montant reconnu sous le champ rend tout écart résiduel visible immédiatement sur le téléphone.