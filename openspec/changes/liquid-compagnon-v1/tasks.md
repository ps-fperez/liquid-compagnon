## 1. Environnement et toolchain Android

- [x] 1.1 Installer le SDK Android en ligne de commande (sans Android Studio) : cmdline-tools, platform-tools, platform android-36, build-tools 36.1.0, NDK 29.0.14206865 ; définir `ANDROID_HOME`, `NDK_HOME` et `JAVA_HOME` (JDK 24 ; repli sur un JDK 21 si Gradle le refuse)
- [x] 1.2 Ajouter les cibles Rust Android (`aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android`, `i686-linux-android`) et `wasm32-unknown-unknown` ; installer `tauri-cli` 2 et `trunk` (wasm-bindgen-cli est téléchargé par Trunk à la bonne version)
- [x] 1.3 Choisir le canal de transfert de l'APK vers le téléphone sans liaison USB (la connexion téléphone ↔ PC est interdite) et autoriser « Installer des applis inconnues » pour l'application de ce canal
- [ ] 1.4 (Optionnel) Installer un émulateur Android si la virtualisation est autorisée sur le poste, pour tester biométrie et `FLAG_SECURE` sans téléphone

## 2. Squelette du workspace

- [x] 2.1 Créer le workspace Cargo (`crates/domain`, `crates/application`, `crates/infrastructure`, `crates/shared-dto`, `src-tauri`, `ui`) avec les dépendances inter-crates conformes à la règle de dépendance (D2)
- [x] 2.2 Initialiser Tauri 2 dans `src-tauri` (identifiant `com.liquidcompagnon.app`, nom « Liquid Compagnon ») et `cargo tauri android init`
- [x] 2.3 Initialiser l'UI Leptos CSR avec Trunk et Tailwind v4 ; câbler `beforeDevCommand` / `beforeBuildCommand` / `frontendDist` dans `tauri.conf.json`
- [x] 2.4 Commande Tauri `ping` appelée depuis l'UI ; vérifier le fonctionnement sur le PC avec le smoke test `scripts/smoke.ps1` (app de bureau, assets embarqués, fenêtre 412 × 892)
- [x] 2.5 Configurer les profils release (`opt-level="z"`, `lto`, `wasm-opt`), `rustfmt.toml`, clippy, `.gitignore`, et un script de vérification (`fmt` + `clippy -D warnings` + `test`)
- [x] 2.7 Pipeline GitHub Actions : qualité (fmt, clippy, tests) puis APK arm64 publié en release « dernier-build » (main) et `vX.Y.Z` (tags)
- [x] 2.6 Générer un premier APK signé (`cargo tauri android build --apk`, keystore hors dépôt documentée dans le README), le transférer par le canal retenu en 1.3 et l'installer sur le téléphone

## 3. Domaine — fondations

- [x] 3.1 Value objects `Money` (Decimal EUR, opérations checkées), `Rate`, `Month` (année-mois), identifiants (`uuid`) + tests unitaires et proptest
- [x] 3.2 Entités budget : `SalaryEntry`, `OneOffIncome`, `RecurringCharge` (périodicité, début/fin), `Expense`, `Category` + validations (montant > 0, 2 décimales) + tests
- [x] 3.3 Entités comptes : `Account`, `AccountKind` (Savings / Securities / EmployeeSavings), `Position`, `AssetRef`, `PeiFund`, `NavAnchor`, `ScenarioRates` + tests
- [x] 3.4 `Snapshot` et règles (un auto par mois, comptes archivés exclus du courant) + tests
- [x] 3.5 `domain::defaults` : catégories par défaut, taux et plafonds réglementés nommés, rendements par défaut par type de compte (D8) + tests

## 4. Domaine — services métier

- [x] 4.1 `BudgetCalculator` : salaire applicable à un mois, revenus ponctuels du mois, équivalent mensuel des charges actives + tests des scénarios `income-management` et `expense-tracking`
- [x] 4.2 `BudgetCalculator::savings_capacity` : moyenne des dépenses des 3 derniers mois complets avec replis (< 3 mois, aucun mois) + tests
- [x] 4.3 `PriceBook` et `Valuation` : valeur des comptes Savings / Securities, plus-value latente, provenance (Live / Cached / Estimated / Manual) + tests
- [x] 4.4 `PeiEstimator` : estimation par indice, fonds sans référence, recalage manuel + tests (scénario 25 € → 26 €)
- [x] 4.5 `ProjectionEngine` : capitalisation mensuelle `(1+r)^(1/12)` en décimal, versements programmés + tests (10 000 € à 3 % → 10 300 € ; 10 000 € + 6 × 300 € → 11 800 €)
- [x] 4.6 `ProjectionEngine` : plafonds livrets, épargne non affectée, avertissement de versements excessifs, 3 scénarios × 3/6/12 mois + tests

## 5. Application — ports et cas d'usage

- [x] 5.1 Définir les ports (D4) : `AccountRepository`, `BudgetRepository`, `SnapshotRepository`, `PriceCache`, `PriceProvider`, `SecretStore`, `BackupCodec`, `Clock` + fakes en mémoire pour les tests
- [x] 5.2 Cas d'usage sécurité : `SetupPin`, `Unlock` (compteur d'échecs, délai exponentiel à partir de 30 s), `ChangePin` + tests
- [x] 5.3 Cas d'usage budget : `AddSalary`, `AddOneOffIncome`, `UpsertRecurringCharge`, `AddExpense`, `ListExpenses` (mois, catégorie, total), `ManageCategories` (archivage si utilisée) + tests
- [x] 5.4 Cas d'usage comptes : `CreateAccount`, `UpdateAccount`, `UpdateSavingsBalance`, `UpsertPosition`, `UpsertPeiFund`, `RecordPeiNav` (capture du cours de référence), archivage + tests
- [x] 5.5 `PricingService` : routage par provider, TTL 5 min, repli cache, gestion 429, provenance ; `RefreshPrices` et `SearchAsset` + tests
- [x] 5.6 `GetDashboard` (total, répartition %, variation vs dernier snapshot, indicateurs budgétaires) et `TakeSnapshotIfDue` + tests
- [x] 5.7 `GetProjection` et `UpdateScenarioRates` + tests
- [x] 5.8 `FullDataset` versionné, `ExportData` (sans hash PIN ni clé API) et `ImportData` (validation complète avant écriture, refus des versions futures, chaîne de migrations) + tests

## 6. Infrastructure

- [x] 6.1 Connexion SQLite (`rusqlite` bundled) + migration initiale de toutes les tables (D6) via `PRAGMA user_version` + test de migration sur `:memory:`
- [x] 6.2 `SqliteAccountRepository` (comptes, détails livrets, positions, fonds PEI, ancrages) + tests d'intégration
- [x] 6.3 `SqliteBudgetRepository` (salaires, revenus ponctuels, charges, dépenses, catégories par défaut au premier lancement) + tests d'intégration
- [x] 6.4 `SqliteSnapshotRepository`, `SqlitePriceCache`, `SqliteSettingsStore`, `Argon2PinHasher` (Argon2id) + tests d'intégration
- [x] 6.5 `CoinGeckoProvider` (`/simple/price` groupé, `/search`) avec `reqwest` rustls + tests `wiremock`
- [x] 6.6 `TwelveDataProvider` (`/price` batch, `/symbol_search`, `/exchange_rate` → EUR, espacement des requêtes, 429) + tests `wiremock`
- [x] 6.7 `JsonBackupCodec` (serde_json, `schema_version`) + tests aller-retour export → import
- [x] 6.8 `SystemClock` et transaction d'import « tout remplacer » + test de non-modification en cas d'échec

## 7. Présentation Tauri (src-tauri)

- [x] 7.1 DTO dans `shared-dto` (dashboard, comptes, positions, budget, projection, réglages) et mappers domaine → DTO dans `src-tauri`
- [x] 7.2 Composition root : instanciation des adaptateurs, injection `Arc<dyn Port>` dans les use cases, état Tauri managé
- [x] 7.3 Commandes Tauri pour tous les use cases (appels DB en `spawn_blocking`), erreurs mappées en messages utilisateur français
- [x] 7.4 Garde de verrouillage côté backend : refus des commandes métier tant que la session n'est pas déverrouillée ; verrouillage après 60 s en arrière-plan (événements de cycle de vie)
- [x] 7.5 Plugins `tauri-plugin-biometric`, `tauri-plugin-dialog`, `tauri-plugin-fs` + permissions (capabilities) minimales
- [x] 7.6 Android : `FLAG_SECURE` dans `MainActivity.kt`, `android:allowBackup="false"` et seule la permission `INTERNET` déclarée par l'application (les plugins ajoutent les leurs, ex. `USE_BIOMETRIC`)

## 8. UI — fondations et thème

- [x] 8.1 Design tokens Tailwind (`@theme` : couleurs, `shadow-glow-*`, `animate-glow-pulse`, `prefers-reduced-motion`), police, fond sombre forcé
- [x] 8.2 Module `ui::api` typé (wrapper `invoke` par commande) et module `ui::format` (montants FR, dates JJ/MM/AAAA, saisie virgule/point)
- [x] 8.3 Composants de base : `AmountInput` (clavier décimal), `Card`, `GlowCard`, `Badge` provenance, `BottomSheet`, `ListRow`, cibles ≥ 44 px
- [x] 8.4 Shell : routeur, barre de navigation inférieure (onglet actif avec glow, bouton « + » central pulsant), en-tête avec réglages et mode discret, gestion du bouton retour Android
- [x] 8.5 Graphiques SVG en Rust : barre de répartition, courbe d'évolution, courbes de projection (neutre mise en avant)

## 9. UI — écrans

- [x] 9.1 Écrans de création du PIN et de verrouillage (pavé numérique, biométrie, compte à rebours après 5 échecs)
- [x] 9.2 Dashboard : carte patrimoine (glow, dégradé), variation, barre de répartition, liste des comptes avec provenance, indicateurs budgétaires, courbe 6 mois / 1 an / tout, tirer pour rafraîchir, flash de mise à jour des cours
- [x] 9.3 Comptes : liste, création (type, nom, couleur, versement programmé), détail livret (solde, taux, plafond)
- [x] 9.4 Détail PEA / crypto : positions, recherche d'actif, quantité, prix de revient, plus-value latente, solde espèces
- [x] 9.5 Détail PEI : fonds, actif de référence, saisie de valeur de part (réel vs estimée)
- [x] 9.6 Budget : salaire et historique, revenus ponctuels, charges fixes, dépenses du mois (filtre catégorie, total), gestion des catégories, capacité d'épargne
- [x] 9.7 Saisie rapide « + » : dépense en 3 interactions, revenu ponctuel, mise à jour de solde
- [x] 9.8 Projection : sélecteur 3 / 6 / 12 mois, 3 courbes, tableau par compte, épargne non affectée, avertissements, édition des rendements par scénario
- [x] 9.9 Réglages : biométrie, changement de PIN, clé Twelve Data (+ test de la clé), export / import JSON avec avertissements, rappel d'export > 30 jours

## 10. Vérification et livraison

- [x] 10.1 Parcourir chaque scénario des specs sur le téléphone (checklist dans `docs/recette-v1.md`), corriger les écarts
- [x] 10.2 Tests hors ligne (mode avion), quota API atteint, clé absente, import invalide
- [x] 10.3 Vérifier `openspec validate liquid-compagnon-v1`, `cargo clippy -D warnings`, `cargo test --workspace` au vert
- [x] 10.4 Build APK release signé, installation sur le téléphone, README (build, installation, sauvegarde)

## 11. Correctif post-recette : cours des ETF européens

- [x] 11.1 Twelve Data : symboles hors offre (403 par symbole, 404 seul) signalés `NotCovered` au lieu d'être ignorés, 401 = clé absente ou invalide, offre requise affichée dans la recherche (`show_plan`), vérification de la clé par `/api_usage`.
- [x] 11.2 Source Yahoo Finance sans clé (`QuoteSource::Yahoo`, `CW8.PA`) : cours, conversion de devises (pence `GBp` compris), recherche ; source par défaut des PEA et références PEI, Twelve Data au choix dans la recherche d'actif.
- [x] 11.3 Bandeau « non couvert » nommant les actifs, sauvegarde JSON et SQLite étendues à la nouvelle source (format inchangé).

## 12. Expérience utilisateur

- [x] 12.1 Didacticiel de prise en main en cinq écrans (balayage, « Passer »), mémorisé dans les réglages locaux, « Revoir le didacticiel » dans les réglages
- [x] 12.2 Icône engrenage pour les réglages
- [x] 12.3 États vides actionnables : la carte vide ouvre directement l'ajout (comptes, positions, fonds, salaire, charges, dépenses, projection)
- [x] 12.4 Recherche d'actif avec autocomplétion (délai de 400 ms, réponses périmées ignorées), recherche par ISIN, Paris en tête, actifs populaires en un geste
- [x] 12.5 Libellés de dépense suggérés avec leur catégorie (« Habituelles », autocomplétion du libellé)
- [x] 12.6 Parcours e2e étendu (didacticiel, suggestions, cartes vides, contrôle d'encodage, étape réseau facultative `LC_E2E_LIVE`)
