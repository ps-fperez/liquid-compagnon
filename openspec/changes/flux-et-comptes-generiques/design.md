## Context

La v0.2 a introduit le compte courant et sa règle de référence : un solde saisi, puis les dépenses et les revenus rattachés saisis ensuite. Les livrets et les espèces des comptes-titres ne sont encore que des soldes saisis. Le « versement mensuel programmé » n'existe que dans la projection.

Cette livraison généralise la règle de référence et en fait le socle des flux. Les deux livraisons suivantes de la v0.3 (reste à dépenser, objectifs et bilan) s'appuient sur ce grand livre.

## Goals / Non-Goals

**Goals :**
- Décrire un mois réel par des flux saisis une fois : virements récurrents, prélèvements.
- Des types de comptes qui conviennent à d'autres utilisateurs que l'auteur.

**Non-Goals :**
- Calcul de frais ou de prélèvements sociaux : l'utilisateur saisit le montant reçu.
- Achat automatique de parts dans un PEA ou une épargne salariale : les parts restent saisies.
- Import de relevés bancaires (prévu en V2).

## Decisions

### D1 — Référence de solde généralisée
- `BalanceReference { amount, on: NaiveDate, at: DateTime<Utc> }` sert au compte courant (signé), aux livrets et à l'assurance-vie (montant positif ou nul), ainsi qu'aux espèces des comptes-titres.
- Migration :
  - livrets : `at` = fin de la journée de `balance_updated_on` ;
  - espèces : `on` et `at` = date de la migration.
- Mettre à jour un solde remplace la référence, comme pour le compte courant.

### D2 — Grand livre des mouvements
- `Ledger` remplace `CheckingLedger`. Il regroupe par compte les mouvements :
  - dépenses et revenus rattachés ;
  - occurrences des charges prélevées ;
  - occurrences des virements, en entrée et en sortie.
- Seules les occurrences datées d'aujourd'hui ou avant sont générées.
- Chaque mouvement porte sa **temporalité** :
  - `Recorded(instant)` pour une saisie (dépense, revenu, virement ponctuel) : il compte si `instant > référence.at` et `date >= référence.on`, la règle de la v0.2 ;
  - `Scheduled` pour une échéance (charge prélevée, occurrence récurrente) : il compte si `date > référence.on`. Cela évite toute ambiguïté de fuseau horaire : une échéance du jour de la référence est réputée déjà passée sur le relevé.
- Les noms des comptes d'en face (fil des mouvements) sont résolus par l'application, le domaine restant pur.
- *Alternative écartée — matérialiser les occurrences en base.* Il faudrait une tâche de fond, et modifier un virement récurrent obligerait à réécrire l'historique. Le calcul à la volée reste instantané pour des centaines de mouvements.

### D3 — Virements et comptes hors patrimoine
- `Transfer { id, from: Endpoint, to: Endpoint, sent: Money, received: Money, label, schedule, recorded_at }`.
  - `Endpoint` = `Account(AccountId)` | `External(ExternalId)`.
  - `Schedule` = `Once { date }` | `Monthly { day, from: Month, until: Option<Month> }` | `Yearly { month, day, from_year, until_year: Option<i32> }`.
- Le jour est compris entre 1 et 31 et ramené au dernier jour du mois si besoin.
- Invariants du domaine :
  - les deux côtés sont différents, et au moins un est un compte suivi ;
  - les montants sont strictement positifs.
- Le cas d'usage vérifie en plus que les comptes suivis existent et ne sont pas archivés.
- `ExternalAccount { id, name }` est sa propre table. Sa suppression est refusée s'il est utilisé.
- Effet par nature de compte :
  - compte courant, livrets, assurance-vie : solde ;
  - PEA et crypto : espèces ;
  - épargne salariale : projection seulement.

### D4 — Projection par flux datés
- `ProjectionInput` connaît le mois de départ.
- `ProjectedAccount.flows` liste des flux signés : `EveryMonth(montant)` ou `EveryYear(mois, montant)`. Ils sont dérivés des virements récurrents : reçus en entrée, envoyés en sortie.
- La capacité d'épargne consommée au mois *m* = somme des montants **envoyés** par les virements récurrents dont la source est un compte courant ou un compte hors patrimoine, vers un compte suivi d'une autre nature.
- L'épargne non affectée du mois = capacité − consommation, jamais négative.
- Le dépassement signalé compare la capacité à la consommation mensuelle moyenne : mensuels + annuels ÷ 12.
- Les plafonds limitent les flux entrants des livrets, comme les versements aujourd'hui. L'excédent va à l'épargne non affectée.
- Le compte courant reste constant.
- `monthly_contribution` disparaît du calcul.

### D5 — Budget
- `BudgetSummary` gagne `external_out` et `external_in` du mois : occurrences et virements ponctuels datés du mois, avec l'extérieur.
- La capacité d'épargne retranche l'équivalent mensuel net des virements récurrents avec l'extérieur : mensuels, plus annuels ÷ 12.

### D6 — Conversion des versements programmés
- `TransferUseCases::convert_scheduled_contributions()` est idempotent. Pour chaque compte dont `monthly_contribution > 0` :
  - il crée un virement mensuel le 1er, à partir du mois courant ;
  - la source est le premier compte courant actif, sinon un compte hors patrimoine « Compte courant (non suivi) », créé une seule fois ;
  - il remet la contribution à 0.
- Appels : au démarrage (composition) et après chaque import.
- Le champ reste stocké, toujours à 0, pour lire les anciennes données. Il est retiré des saisies (création, paramètres du compte).

### D7 — Comptes génériques
- `SavingsProduct` = `LivretA | Ldds | Livret | AssuranceVie` ; `deposit_ceiling: Option<Money>`.
- `AccountType` gagne `Livret` et `AssuranceVie`.
- L'intitulé de `Pei` devient « Épargne salariale ». L'identifiant technique `pei` est conservé : base, sauvegarde, DTO.
- Valeurs par défaut :
  - Livret : taux 0 %, sans plafond ;
  - Assurance-vie : rendements 1,5 / 2,5 / 4 %, sans plafond.
- L'UI groupe les types : Courant, Livrets, Placements, Épargne salariale.

### D8 — Charges prélevées
- `RecurringCharge` gagne `debit: Option<Debit { account: AccountId, day: u8 }>`.
- Échéances :
  - mensuelle : chaque mois actif ;
  - trimestrielle : tous les 3 mois depuis le début ;
  - annuelle : tous les 12 mois.

### D9 — UI
- La feuille `TransferSheet` est partagée :
  - De / Vers : comptes suivis et hors patrimoine, avec « + Nouveau compte hors patrimoine » ;
  - montant envoyé, puis « Montant reçu différent » qui affiche l'abondement net ;
  - fréquence : Ponctuel, Mensuel, Annuel, avec la date, le jour ou le mois et le jour selon le cas ;
  - libellé ;
  - aperçu du solde avant et après, et avertissement de plafond.
- La feuille s'ouvre depuis :
  - « + » › Virement (`/comptes?saisie=virement`) ;
  - la fiche d'un compte, pré-remplie avec ce compte ;
  - la section « Virements programmés ».
- La fiche d'un compte à solde affiche :
  - « Depuis le dernier solde saisi » : le fil, avec une icône par nature ;
  - « Virements programmés » : modifier ou supprimer.
- La page Comptes gagne une section « Hors patrimoine » (renommer, supprimer).
- La feuille de charge gagne « Prélevée sur » et « Jour du prélèvement ».

### D10 — Sauvegarde v3
- Le format v3 ajoute `transfers`, `external_accounts`, `debit` sur les charges, `reference_at` sur les livrets, la référence des espèces et les nouveaux produits.
- La migration v2 → v3 est neutre : nouveaux champs facultatifs.
- L'import contrôle que chaque côté de virement et chaque prélèvement désigne un compte présent, puis lance D6.

## Risks / Trade-offs

- **Ampleur du changement (domaine à UI).**
  - Parade : livraison par couches testées (domaine, application, stockage, présentation).
  - Le parcours e2e couvre le mois type : virement mensuel, prélèvement, rapatriement, compte joint, abondement.
- **Double comptage d'une charge et d'un virement vers le même compte joint.** L'utilisateur choisit l'un ou l'autre. Parade : l'aide de la feuille de virement le rappelle quand une charge porte le même montant.
- **Solde de livret calculé, donc faux si un virement n'a pas eu lieu.** Parade : le solde reste recalable en un geste, et le fil montre chaque occurrence comptée.

## Migration Plan

1. Migration SQLite 003 :
   - tables `transfers` et `external_accounts` ;
   - colonnes `debit_account_id` et `debit_day` sur les charges ;
   - `reference_at` sur les livrets (fin de journée de `balance_updated_on`) ;
   - `cash_on` et `cash_at` sur les espèces (date de la migration).
2. Au démarrage, conversion des versements programmés (D6).
3. Sauvegarde : v3 en écriture, v1 à v3 en lecture.
