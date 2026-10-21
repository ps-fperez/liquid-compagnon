# Recette Liquid Compagnon V1

Chaque scénario de la spécification (`openspec/changes/liquid-compagnon-v1/specs`) est relié aux tests automatiques qui le couvrent. La colonne **Téléphone** liste ce qui ne peut être vérifié que sur l'appareil Android : à cocher pendant la recette.

Légende de couverture :

- **dom** : `crates/domain` (tests unitaires et de propriété)
- **app** : `crates/application` (cas d'usage avec ports simulés)
- **infra** : `crates/infrastructure` (SQLite réel, API simulées par serveur HTTP, test de bout en bout `tests/end_to_end.rs`)
- **tauri** : `src-tauri` (session, erreurs, conversions)
- **ui** : `ui` (formatage, géométrie des courbes, navigation)
- **e2e n** : étape *n* de `scripts/e2e.ps1` (vraie app de bureau, même UI et même backend que l'APK)
- **smoke** : `scripts/smoke.ps1`

## Procédure

1. Désinstaller toute version précédente, installer l'APK signé de la release `vX.Y.Z` (ou `dernier-build`).
2. Parcourir les sections ci-dessous dans l'ordre et cocher chaque case **Téléphone**.
3. Noter toute anomalie (capture d'écran + étapes) dans la section « Anomalies ».

## Sécurité (app-security)

| Scénario | Couverture automatique | Téléphone |
|---|---|---|
| Requête de cotation (identifiants d'actifs uniquement) | infra : `fetches_all_assets_in_one_request`, `batch_request_with_api_key_header` | — |
| Sauvegarde cloud Android désactivée | manifeste versionné (`allowBackup="false"`, `dataExtractionRules`) | [ ] Réglages Android › Google › Sauvegarde : l'app n'apparaît pas dans les données sauvegardées |
| Création réussie du PIN | app : `setup_stores_only_a_hash` · e2e 1 | [ ] Création du PIN au premier lancement |
| Confirmation différente | app : `setup_rejects_mismatch_and_bad_format` | [ ] Saisir deux codes différents : « Les codes ne correspondent pas » |
| Déverrouillage biométrique | — (Android uniquement) | [ ] Activer la biométrie dans les réglages, verrouiller, déverrouiller à l'empreinte |
| Retour rapide au premier plan (< 60 s) | tauri : `short_background_keeps_session_open` | [ ] Quitter l'app 30 s puis revenir : pas de PIN demandé |
| Verrouillage après 60 s en arrière-plan | tauri : `long_background_locks_session` · smoke (garde IPC) | [ ] Quitter l'app plus d'1 min puis revenir : PIN demandé |
| Biométrie indisponible | e2e 10 (case désactivée sur ordinateur) | [ ] Sans empreinte enregistrée : option grisée |
| Cinquième échec (blocage 30 s) | app : `fifth_failure_locks_for_thirty_seconds`, `lockout_doubles_with_each_new_failure` | [ ] 5 PIN faux : compte à rebours visible |
| Réinitialisation du compteur | app : `success_resets_failure_counter` | — |
| Changement du PIN | app : `change_pin_requires_current_pin` | [ ] Réglages › Changer le code PIN, puis déverrouiller avec le nouveau |
| Aperçu des applications récentes masqué | — (`FLAG_SECURE`, Android uniquement) | [ ] Vue des apps récentes : contenu masqué ; capture d'écran refusée |
| Verrouillage, mauvais PIN, déverrouillage | e2e 6 | — |

## Sauvegarde (data-backup)

| Scénario | Couverture automatique | Téléphone |
|---|---|---|
| Export réussi | app : `export_then_import_restores_identical_data` · infra : `full_journey_from_setup_to_restored_backup` · e2e 10 (contenu sans PIN) | [ ] Réglages › Exporter : le sélecteur Android s'ouvre, fichier `liquid-compagnon-AAAA-MM-JJ.json` créé |
| Avertissement de confidentialité | texte de la feuille d'export | [ ] Le message « fichier en clair » s'affiche avant l'export |
| Import valide | app : `export_then_import_restores_identical_data` · infra : test de bout en bout | [ ] Réimporter le fichier exporté : données identiques |
| Fichier invalide | app : `invalid_file_changes_nothing`, `inconsistent_references_are_rejected_before_writing` · infra : `failure_midway_leaves_previous_data_untouched` · e2e 11 | [ ] Importer un autre fichier JSON : refus, rien ne change |
| Version plus récente | app : `newer_schema_version_is_refused` · infra : `rejects_newer_versions` | — |
| Migration d'un ancien export | infra : `upgrades_old_exports_step_by_step` (mécanisme ; aucune version antérieure à la v1) | — |
| Exclusion du PIN et de la clé API | app : `export_excludes_secrets_and_records_date` · e2e 10 | — |
| Rappel d'export après 30 jours | app : `export_reminder_after_thirty_days` | — |

## Revenus (income-management)

| Scénario | Couverture automatique | Téléphone |
|---|---|---|
| Première saisie du salaire | dom : `raise_applies_from_its_effective_month` · e2e 7 | [ ] Budget › Nouveau salaire |
| Augmentation | dom : `raise_applies_from_its_effective_month` · app : `salary_history_and_summary` | — |
| Suppression d'une entrée | dom : `previous_salary_applies_again_after_deleting_raise` · app : `salary_history_and_summary` | — |
| Prime de fin d'année | dom : `one_off_income_counts_only_in_its_month` · app : `one_off_incomes_of_month` | [ ] « + » › Revenu ponctuel |
| Montant négatif refusé | dom : `salary_must_be_positive` · app : `negative_salary_is_rejected` | — |

## Budget (expense-tracking)

| Scénario | Couverture automatique | Téléphone |
|---|---|---|
| Archivage d'une catégorie utilisée | app : `used_category_can_only_be_archived` · infra : `category_in_use_cannot_be_deleted_at_database_level` | [ ] Budget › Gérer les catégories |
| Charge annuelle (équivalent mensuel) | dom : `yearly_charge_monthly_equivalent`, `fixed_charges_use_monthly_equivalents_of_active_charges` | — |
| Charge terminée | dom : `ended_charge_is_inactive_after_end_month` | — |
| Saisie rapide d'une dépense (3 interactions) | e2e 7 | [ ] « + » › Dépense › montant › catégorie |
| Total mensuel filtré | app : `expenses_filtered_by_month_and_category` | [ ] Filtrer par catégorie |
| Capacité d'épargne (calcul nominal) | dom : `savings_capacity_nominal_case` · app : `summary_computes_savings_capacity` · e2e 7 | — |
| Capacité négative | dom : `capacity_can_be_negative` | — |

## Comptes (account-management)

| Scénario | Couverture automatique | Téléphone |
|---|---|---|
| Création d'un compte | app : `creates_account_with_type_defaults` · e2e 3, 9 | [ ] Créer chaque type de compte |
| Mise à jour du solde | app : `savings_balance_update_is_dated_today` · e2e 4 | [ ] « + » › Mise à jour d'un solde |
| Dépassement du plafond | dom : `balance_above_ceiling_is_allowed_but_ceiling_reached` | — |
| Ajout d'une position crypto | dom : `position_value_and_unrealized_gain` · infra : CoinGecko simulé | [ ] Crypto › Ajouter une position › rechercher « bitcoin » (vraie API) |
| Recherche d'actif | infra : `search_returns_coin_ids_with_readable_names`, `search_builds_symbols_with_exchange` | [ ] PEA › rechercher « CW8 » (clé Twelve Data requise) |
| Plus-value latente | dom : `position_value_and_unrealized_gain` · tauri : `crypto_account_dto_carries_positions_and_provenance` | [ ] Saisir un prix de revient |
| Fonds rattaché à un indice | app : `pei_fund_anchor_captures_reference_price_of_the_day` | [ ] PEI › Ajouter un fonds avec l'indice « IWDA » |
| Fonds sans indice | dom : `pei_funds_are_estimated_through_reference` | — |
| Versements répartis | dom : `remaining_capacity_accumulates_as_unallocated` | — |
| Clôture d'un compte | app : `archived_accounts_are_hidden_by_default` · dom : `archiving_later_does_not_change_history` | [ ] Clôturer puis rouvrir un compte |

## Cours (market-pricing)

| Scénario | Couverture automatique | Téléphone |
|---|---|---|
| Rafraîchissement groupé | app : `assets_of_same_source_are_fetched_in_one_request` · infra : `fetches_all_assets_in_one_request` | — |
| Clé absente | app : `missing_api_key_is_reported` · infra : `missing_key_means_no_request` · e2e 11 | — |
| ETF coté en dollars | infra : `converts_foreign_currency_to_euros` | — |
| Hors ligne | app : `falls_back_to_cache_when_source_fails` | [ ] Mode avion : valeurs affichées avec « cours du … » |
| Cache frais (< 5 min) | app : `fresh_cache_avoids_network_calls` | — |
| Limite de requêtes atteinte | app : `falls_back_to_cache_when_source_fails` · infra : `api_errors_in_body_are_mapped`, `free_plan_credits_are_respected_per_minute` | — |
| Estimation par l'indice | dom : `extrapolates_nav_with_reference_variation` | [ ] Lendemain de la saisie d'un fonds : badge « estimée » |
| Recalage | dom : `freshly_recorded_nav_is_real` · app : `pei_fund_anchor_captures_reference_price_of_the_day` | [ ] Saisir la valeur de part réelle |
| Indication de la provenance | dom : `cached_quote_marks_account_as_cached` · e2e 11 (« prix de revient ») | — |
| Clé Twelve Data vérifiée | — | [ ] Réglages › Enregistrer la clé : « Clé enregistrée et vérifiée » |

## Tableau de bord (wealth-dashboard)

| Scénario | Couverture automatique | Téléphone |
|---|---|---|
| Agrégation | app : `total_and_shares` · e2e 5, 9, 11 | — |
| Répartition | app : `total_and_shares` | — |
| Hausse mensuelle | app : `variation_since_last_auto_snapshot` | [ ] Le mois suivant : variation affichée |
| Premier lancement du mois | app : `first_opening_of_month_takes_one_auto_snapshot` | — |
| Mois sans ouverture | app : `history_has_one_point_per_opened_month` | — |
| Historique insuffisant | e2e 5 (message affiché) | — |
| Synthèse du mois | app : `includes_budget_summary_and_pricing_issues` · e2e 11 | — |
| Snapshot manuel | app : `history_has_one_point_per_opened_month` | [ ] « Figer ce mois » |

## Projection (savings-projection)

| Scénario | Couverture automatique | Téléphone |
|---|---|---|
| Sélection d'horizon | e2e 8 | [ ] Basculer 3 / 6 / 12 mois |
| Valeurs par défaut d'un PEA | dom : `market_accounts_have_ordered_scenarios` | — |
| Personnalisation des rendements | app : `scenario_rates_are_saved` | [ ] Projection › Rendements d'un compte |
| Livret sans versement (10 000 € à 3 % → 10 300 €) | dom : `savings_without_contribution_compound_to_annual_rate` | — |
| PEA avec versements (→ 11 800 €) | dom : `contributions_are_added_monthly` | — |
| Plafond atteint | dom : `ceiling_diverts_excess_contribution_to_unallocated`, `savings_never_receive_deposits_above_ceiling` | — |
| Reliquat (épargne non affectée) | dom : `remaining_capacity_accumulates_as_unallocated` · app : `unprogrammed_capacity_goes_to_unallocated_savings` | — |
| Versements excessifs | dom : `detects_contributions_above_capacity` · app : `reports_contributions_above_capacity` | — |
| Lecture d'une projection | app : `three_scenarios_over_twelve_months` · e2e 8 | — |

## Interface (app-shell-theme)

| Scénario | Couverture automatique | Téléphone |
|---|---|---|
| Saisie rapide | e2e 7 | — |
| Retour Android | — | [ ] Fiche d'un compte › bouton retour : liste des comptes |
| Thème sombre forcé | `color-scheme: dark` dans le thème | [ ] Téléphone en thème clair : l'app reste sombre |
| Flash de mise à jour d'un cours | — | [ ] Actualiser les cours : halo vert/rouge bref sur le compte modifié |
| Animations réduites | `prefers-reduced-motion` dans le thème | [ ] Option « Supprimer les animations » : le bouton « + » ne pulse plus |
| Clavier numérique pour les montants | `inputmode="decimal"` sur les champs de montant | [ ] Toucher un champ de montant |
| Saisie avec point | ui : `parses_comma_point_and_spaces` | — |
| Mode discret | vérifié dans la galerie (tous les montants masqués) | [ ] Icône œil : plus aucun montant lisible |
| Cibles tactiles ≥ 44 px, contraste | classes `min-h-11` et plus, palette sombre | [ ] Utilisation à une main |

## Compte courant (checking-account, v0.2)

| Scénario | Couverture automatique | Téléphone |
|---|---|---|
| Création et premier solde | dom : `checking_account_is_valued_from_its_reference_and_ledger` · e2e 12 | [ ] Créer « Compte courant », saisir le solde du jour |
| Découvert | dom : `overdraft_is_allowed_but_not_sub_cent_amounts` · app : `checking_balance_follows_linked_movements_until_the_next_reference` · e2e 12 | [ ] Case « découvert » : solde négatif, retranché du patrimoine |
| Saisie rapide | e2e 12 (compte présélectionné, trois gestes) | [ ] « + » › Dépense : « Payé depuis » montre le compte courant |
| Sans compte courant | e2e 7 (aucun choix de compte affiché) | — |
| Rattachement refusé | app : `movements_link_only_to_an_active_checking_account` | — |
| Dépenses après la référence | dom : `movements_after_the_reference_change_the_balance` · app : `checking_balance_follows_linked_movements_until_the_next_reference` · e2e 12 | [ ] Le solde baisse après une dépense |
| Recalage | dom : `movements_already_included_in_the_reference_are_ignored` · e2e 12 | [ ] « Mettre à jour le solde » : la liste des mouvements se vide |
| Dépense oubliée d'avant la référence | dom : `movements_already_included_in_the_reference_are_ignored` | — |
| Mouvements depuis la référence | e2e 12 | — |
| Projection | dom : rendements nuls (`checking_account_is_valued_from_its_reference_and_ledger`) | [ ] Projection : compte courant constant |
| Aller-retour de sauvegarde | app : `export_then_import_restores_identical_data` · infra : `full_journey_from_setup_to_restored_backup` | — |
| Ancien fichier (v1) | infra : `version_1_exports_are_still_readable` | — |

## Mises à jour (app-updates, v0.2)

| Scénario | Couverture automatique | Téléphone |
|---|---|---|
| Build de main | CI : `latest.json` publié avec l'APK, versionCode 100000 + run | [ ] Release « dernier-build » : APK + `latest.json` |
| Version plus récente | app : `newer_build_is_proposed` | [ ] Installer le build N, publier N+1 : invitation au déverrouillage |
| Déjà à jour | app : `newer_build_is_proposed` | [ ] Réglages › Mises à jour : « Application à jour » |
| Build local | app : `local_builds_never_update` · e2e 10 | — |
| Hors ligne | app : `feed_errors_are_reported` | [ ] Mode avion : aucune erreur au démarrage |
| Report | — | [ ] « Plus tard » : plus d'invitation pour ce build, pastille sur l'engrenage |
| Mise à jour réussie | infra : `verified_download_reports_progress` | [ ] « Mettre à jour » : progression, installeur Android, données intactes |
| Fichier altéré | infra : `altered_download_is_deleted` · app : `apk_must_come_from_the_app_releases` | — |
| Autorisation manquante | — | [ ] Premier essai : réglage « Installer des applis inconnues » ouvert |
| Vérification automatique désactivée | src-tauri : `automatic_checks_are_spaced_by_six_hours` | [ ] Option désactivée : pas d'invitation |

## Anomalies
| # | Écran | Description | Statut |
|---|---|---|---|
| | | | |
