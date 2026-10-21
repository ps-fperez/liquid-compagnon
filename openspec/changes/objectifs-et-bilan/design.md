## Context

Les livraisons 1 et 2 de la v0.3 ont posé les virements, le grand livre et le reste à dépenser. La projection existe sur 12 mois et trois scénarios ; les snapshots mensuels gardent la valeur de chaque compte (un snapshot automatique à la première ouverture de chaque mois). Les maquettes « Objectif d'épargne » et « Bilan du mois » ont été validées.

## Goals / Non-Goals

**Goals :** suivre un objectif sans double saisie, avec une date d'arrivée réaliste ; un rituel de début de mois court, qui se termine par le recalage du compte courant.

**Non-Goals :** objectifs alimentés par un virement dédié créé automatiquement ; effet de marché isolé dans la variation du patrimoine ; notifications système en dehors de l'application.

## Decisions

### D1 — Un objectif suit des comptes, sa progression est leur valeur
Pas de « versement sur l'objectif » : la progression est la somme des valeurs actuelles des comptes liés. Deux objectifs peuvent partager un compte (chacun voit la même valeur) : c'est assumé et signalé à la création. Un compte clôturé sort de l'objectif ; un objectif sans compte actif affiche zéro.

### D2 — Effort, versements et dates d'arrivée
- Effort nécessaire (si une date est fixée) : `(cible − valeur actuelle) / mois restants`, sans rendement (repère simple et prudent) ; le mois de l'échéance compte.
- Versé chaque mois : équivalent mensuel net des flux récurrents des comptes liés (virements entrants moins sortants, un annuel pour 1/12) ; un virement entre deux comptes liés est neutre.
- Date d'arrivée : premier mois où la somme des comptes liés atteint la cible dans la projection du scénario, sur **120 mois** (la projection de l'écran reste à 12 mois). Au-delà : « plus de 10 ans ».
- État : « Atteint » si la valeur actuelle atteint la cible ; sinon « Dans les temps » si l'arrivée neutre est au plus tard le mois de l'échéance, « En retard » sinon ; sans date : pas d'état.

### D3 — Objectif atteint, une seule fois
Le rapport des objectifs enregistre la date d'atteinte la première fois que la valeur atteint la cible et la signale (`just_reached`) : l'UI félicite une fois. Redescendre sous la cible n'efface pas la date.

### D4 — Bilan du mois `M`, calculé à la demande
- Mis de côté : revenus + entrées extérieures sur un compte courant − charges fixes − dépenses − sorties hors patrimoine d'un compte courant (ce qui reste de l'enveloppe plus l'épargne programmée, moins les rapatriements). Comparé à `M − 1`.
- Entrées et sorties : reçus (revenus et entrées extérieures), dépensés (charges, dépenses, sorties hors patrimoine) ; revenus ponctuels signalés.
- Catégories : trois premières, et la plus forte hausse sur un mois (catégorie dépensée les deux mois).
- Patrimoine : valeur au début de `M + 1` (snapshot automatique de `M + 1`, sinon dernier snapshot de `M`) comparée au début de `M` (snapshot automatique de `M`) ; la variation n'est affichée que si les deux existent.
- Objectifs : progression en fin de mois à partir des valeurs par compte du même snapshot, comparée à la fin de `M − 1`.

### D5 — Ouverture automatique et recalage
`pending_review` renvoie le mois précédent si aucun bilan n'est enregistré pour lui et que l'application contient des données d'avant le mois en cours (compte créé ou dépense saisie). Ouvrir le bilan l'enregistre comme vu. La dernière carte propose le premier compte courant actif : valeur pré-remplie = solde de départ saisi au mois précédent (enregistré dans le bilan), sinon solde actuel ; elle liste les virements sortants du compte courant déjà échus dans le mois. Valider met à jour le solde (référence) et enregistre le solde de départ du mois.

### D6 — Persistance et sauvegarde
Tables `savings_goals`, `goal_accounts`, `monthly_reviews` (migration 004). Sauvegarde v4 : objectifs et bilans ; migration v3 → v4 neutre (listes vides). L'import vérifie que les comptes liés existent.

## Risks / Trade-offs

- **Patrimoine sans snapshot** (application non ouverte un mois) → la carte affiche la valeur connue sans variation ; rien n'est inventé.
- **Projection sur 10 ans** → coût de calcul négligeable (comptes × 120 mois), mais les rendements de scénario dominent les longues durées : la date neutre est un ordre de grandeur, l'effort sans rendement reste le repère.
- **Bilan ouvert pendant une saisie** → il ne s'ouvre qu'au déverrouillage, après la porte de session, jamais par-dessus une feuille.