## Context

La livraison 1 a posé les virements et le grand livre. Le budget d'un mois connaît déjà le salaire, les revenus ponctuels, les charges fixes (équivalent mensuel), les dépenses et les virements avec l'extérieur, mais sans savoir quels comptes sont des comptes courants. Les maquettes v0.3 (accueil, détail du reste à dépenser, notification en bas) ont été validées.

## Goals / Non-Goals

**Goals :** un chiffre du jour juste et expliqué ; un accueil qui le met en avant ; des retours d'action discrets et réversibles ; des animations sobres, désactivables.

**Non-Goals :** objectifs d'épargne et bilan du mois (livraison 3) ; enveloppes par catégorie ; alertes push.

## Decisions

### D1 — Le reste à dépenser est une enveloppe mensuelle calculée, pas saisie
`enveloppe = revenus + entrées extérieures sur un compte courant + rapatriements − charges fixes − épargne programmée − sorties hors patrimoine d'un compte courant`, puis `reste = enveloppe − dépenses du mois`. Les virements comptent pour **toutes leurs occurrences du mois**, même à venir : l'enveloppe décrit le mois entier, comme le salaire. Les charges comptent pour leur équivalent mensuel (une charge annuelle est provisionnée). Alternative écartée : une enveloppe saisie à la main ; elle dériverait des virements réels.

### D2 — Classement des virements par nature du compte
- Épargne programmée : d'un compte courant ou de l'extérieur vers un compte suivi qui n'est pas un compte courant (montant envoyé).
- Rapatriement : d'un compte suivi non courant vers un compte courant (montant reçu).
- Sortie hors patrimoine : d'un compte courant vers l'extérieur (montant envoyé). Un virement d'un livret vers le compte joint ne touche pas l'enveloppe : il est payé par l'épargne.
- Entrée extérieure : de l'extérieur vers un compte courant (montant reçu), ajoutée aux revenus.
Le calculateur reçoit la liste des comptes courants (`with_checking`). La même règle corrige la capacité d'épargne : une entrée extérieure vers une épargne n'est plus un revenu récurrent.

### D3 — Rythme
Pour le mois en cours, jour `d` sur `D` : dépense attendue `enveloppe × d / D`. « Budget dépassé » si le reste est négatif ; « Rythme élevé » si les dépenses dépassent l'attendu de plus de 2 % de l'enveloppe ; sinon « Dans le rythme ». Par jour : `reste / (D − d + 1)` (aujourd'hui compris). Projection : `enveloppe − dépenses / d × D`. Mois passé : pas de rythme, seulement le résultat ; mois futur : enveloppe prévue.

### D4 — Données dans la synthèse budgétaire
`BudgetSummary` porte le reste à dépenser et les trois premières catégories du mois ; le tableau de bord l'expose déjà (`dashboard.budget`). Aucune commande nouvelle.

### D5 — Notification unique
Une seule notification à la fois (la nouvelle remplace l'ancienne), en bas au-dessus de la barre, 5 s (7 s pour une erreur), avec action facultative. L'action est une fermeture `Arc<dyn Fn() + Send + Sync>` : elle doit survivre à la fermeture de la feuille qui l'a créée ; le rechargement de l'écran utilise `Callback::try_run`. Si une feuille est ouverte au moment de l'affichage, la notification se place en haut.

### D6 — Animations en CSS, défilement des montants en JS
Cascade, jauge, transition d'onglet et halo sont des animations CSS (désactivées par la règle `prefers-reduced-motion` existante). Le défilement d'un montant utilise `requestAnimationFrame` (700 ms, easing out-cubic) et vérifie `matchMedia("(prefers-reduced-motion: reduce)")`. Il ne concerne que le patrimoine total et le reste à dépenser, une fois par affichage.

## Risks / Trade-offs

- **Enveloppe négative avant toute dépense** (virements supérieurs aux revenus) → l'état « Budget dépassé » et la feuille expliquent la ligne en cause.
- **Comptes courants multiples** → tous forment le même « porte-monnaie » ; suffisant pour un foyer avec un compte perso.
- **Annuler après navigation** → l'action supprime l'élément même si l'écran a changé ; le rechargement est ignoré si l'écran n'existe plus.