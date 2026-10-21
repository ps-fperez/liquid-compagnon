## Why

L'application dit combien on possède et combien il reste à dépenser, mais pas **vers quoi** on épargne ni **comment s'est passé le mois**. L'utilisateur suit de tête son apport immobilier ou un voyage, et refait chaque début de mois le même geste : regarder le mois écoulé, puis recaler son compte perso après les virements de début de mois.

Cette livraison 3 termine la v0.3 avec les objectifs d'épargne et le bilan du mois.

## What Changes

- **Objectifs d'épargne.**
  - Un objectif a un nom, un montant, une date facultative et suit des **comptes existants** (pas de double saisie) : sa progression est la valeur de ces comptes.
  - Effort mensuel nécessaire pour tenir la date, montant déjà versé chaque mois (virements récurrents vers ces comptes), date d'arrivée dans chaque scénario de projection (sur 10 ans), état « Dans les temps » ou « En retard ».
  - À 100 %, l'anneau s'illumine et une notification félicite l'utilisateur, une seule fois.
  - Liste et création dans l'onglet Projection ; aperçu en anneaux sur l'accueil.
- **Bilan du mois.**
  - S'ouvre tout seul au premier déverrouillage d'un mois, sur le mois qui vient de finir, en cartes qui défilent seules (5 s) comme des stories : le mois en un chiffre (mis de côté), entrées et sorties, où est parti l'argent, patrimoine, objectifs.
  - Chaque chiffre est comparé au mois précédent.
  - Dernière carte : « Votre compte courant repart à combien ? », pré-remplie avec le solde de départ du mois précédent, en rappelant les virements de début de mois déjà pris en compte. Valider recale le solde du compte courant.
  - Les bilans passés restent consultables depuis l'accueil (section Évolution).
- **BREAKING (sauvegarde)** : format v4 (objectifs et bilans vus). Les fichiers v1 à v3 restent importables.

## Capabilities

### New Capabilities
- `savings-goals` : objectifs d'épargne liés à des comptes, effort, versements, dates d'arrivée par scénario, objectif atteint.
- `monthly-review` : bilan du mois en cartes, comparaison au mois précédent, recalage du compte courant en début de mois, historique des bilans.

### Modified Capabilities
<!-- Spécifications principales non synchronisées : décrit dans les nouvelles capacités. -->

## Impact

- **Domaine.** `SavingsGoal` (progression, effort, date d'arrivée sur une projection) ; `MonthlyReview` (mois vu, solde de départ saisi).
- **Application.** Cas d'usage des objectifs (projection sur 120 mois des comptes suivis) et du bilan (synthèses budgétaires du mois et du précédent, snapshots de patrimoine, objectifs en fin de mois, recalage du compte courant).
- **Infrastructure.** Migration SQLite 004 (`savings_goals`, `goal_accounts`, `monthly_reviews`) ; sauvegarde v4.
- **UI.** Objectifs dans Projection et sur l'accueil ; bilan plein écran en cartes ; historique des bilans.
- **Aucune** donnée ne quitte le téléphone.