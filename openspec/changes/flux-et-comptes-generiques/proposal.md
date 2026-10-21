## Why

L'application décrit des soldes, pas les flux qui les font évoluer. Dans un mois réel :
- le salaire arrive sur le compte courant ;
- une partie repart par virements (épargne, compte joint, crédit) ;
- les abonnements sont prélevés au fil du mois ;
- on rapatrie parfois de l'épargne pour une grosse dépense.

Aujourd'hui, chaque solde se recale à la main et le « versement mensuel programmé » ne bouge aucun solde. Une épargne salariale versée une fois par an avec abondement ne se projette pas.

Les types de comptes sont en outre figés : Livret A, LDDS, PEA, PEI, crypto. Un autre utilisateur, avec un LEP, une assurance-vie ou un PEE, ne peut pas décrire sa situation.

Cette livraison 1 de la v0.3 pose les flux entre comptes et rend les comptes génériques, avant le reste à dépenser (livraison 2) et les objectifs et le bilan du mois (livraison 3).

## What Changes

- **Virements entre comptes.**
  - Ponctuels, mensuels (jour du mois) ou annuels (mois et jour).
  - Depuis ou vers un compte suivi, ou un **compte hors patrimoine** (compte joint, banque d'un crédit…) créé d'un simple nom.
  - Montant **reçu** distinct du montant **envoyé** (abondement net de prélèvements sociaux), sans calcul de frais.
  - Les occurrences font évoluer les soldes (compte courant, livrets, assurance-vie, espèces d'un PEA) selon la règle du solde de référence : seuls comptent les flux postérieurs au dernier solde saisi.
- **Le versement mensuel programmé disparaît au profit des virements récurrents.** Les versements déjà saisis sont convertis automatiquement en virements mensuels depuis le premier compte courant, ou depuis un compte hors patrimoine « Compte courant (non suivi) ».
- **Charges fixes prélevées sur un compte courant** à un jour du mois : elles font baisser son solde à chaque échéance.
- **Fil des mouvements** sur chaque compte : dépenses, revenus, prélèvements, virements entrants et sortants. Le compte affiche aussi ses virements programmés.
- **Projection.**
  - Les comptes reçoivent leurs virements récurrents, mensuels et annuels au bon mois. Les sorties récurrentes les diminuent.
  - Seuls les virements issus du compte courant ou de l'extérieur consomment la capacité d'épargne.
  - Les plafonds restent appliqués.
- **Budget.** Les virements avec l'extérieur comptent dans le mois : une sortie vers le compte joint est une dépense du foyer, une entrée extérieure est un revenu.
- **Comptes génériques.**
  - « Autre livret » (taux libre, plafond facultatif : LEP, Livret Jeune, livret bancaire…).
  - « Assurance-vie ou PER » (valeur saisie, rendements par scénario).
  - « Épargne salariale » remplace l'intitulé « PEI » (PEE, PEI, PER collectif).
  - Les modèles Livret A et LDDS restent pré-remplis.
- **BREAKING (sauvegarde)** : format v3 (virements, comptes hors patrimoine, prélèvements, nouveaux comptes). Les fichiers v1 et v2 restent importables, et leurs versements programmés sont convertis.

## Capabilities

### New Capabilities
- `account-transfers` : virements ponctuels et récurrents, comptes hors patrimoine, montant reçu distinct, effet sur les soldes, la projection et le budget, fil des mouvements.
- `generic-accounts` : types de comptes génériques (autre livret, assurance-vie ou PER, épargne salariale) et conversion des versements programmés.
- `scheduled-debits` : charges fixes prélevées sur un compte courant à date fixe.

### Modified Capabilities
<!-- Spécifications principales non synchronisées : décrit dans les nouvelles capacités. -->

## Impact

- **Domaine.**
  - `Transfer` et `Schedule` ; référence de solde généralisée aux livrets et aux espèces des comptes-titres.
  - Grand livre des mouvements (dépenses, revenus, prélèvements, virements) ; projection par flux datés.
  - Produits de livret génériques, plafond facultatif.
- **Application.** Cas d'usage des virements et des comptes hors patrimoine, conversion des versements programmés, prélèvements des charges, budget et projection avec flux.
- **Infrastructure.** Migration SQLite 003 (`transfers`, `external_accounts`, colonnes de prélèvement et de référence) ; sauvegarde v3.
- **UI.**
  - « Virement » dans le bouton + ; feuille de virement avec aperçu et contrôle du plafond.
  - Fil des mouvements et virements programmés sur chaque compte ; section « Hors patrimoine ».
  - Prélèvement sur les charges fixes ; nouveaux types de comptes à la création.
- **Aucune** donnée supplémentaire ne quitte le téléphone.
