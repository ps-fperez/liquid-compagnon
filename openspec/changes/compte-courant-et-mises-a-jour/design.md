## Context

La V1 (change `liquid-compagnon-v1`, non archivé) est en usage sur le téléphone. Elle reste structurée en Clean Architecture : domaine pur, cas d'usage et ports, adaptateurs, composition root Tauri, UI Leptos.

Contraintes utilisateur :
- jamais de câble entre le téléphone et le PC : les APK sont construits par GitHub Actions, sur un dépôt public ;
- aucun build Android local : le code natif n'est compilé et testé qu'en CI, puis sur le téléphone ;
- les choix produit ont été tranchés par l'utilisateur :
  - le solde du compte courant évolue avec les dépenses et revenus ponctuels seulement ;
  - l'installation de la mise à jour se fait directement depuis l'application.

## Goals / Non-Goals

**Goals:**
- Un compte courant au solde vivant, sans saisie supplémentaire lors de la dépense rapide.
- Des mises à jour en deux gestes (« Mettre à jour » puis la confirmation d'Android), sûres et sans perte de données.

**Non-Goals:**
- Salaire et charges fixes ne sont pas imputés automatiquement au compte courant. Ni rapprochement bancaire, ni import de relevés (prévu en V2).
- Pas de virements entre comptes, ni de rattachement aux livrets ou au PEA.
- Pas de notification système Android ni de vérification en arrière-plan quand l'app est fermée : il faudrait WorkManager et une permission de notification.
- Pas de mise à jour silencieuse : Android impose la confirmation de l'utilisateur.

## Decisions

### D1 — Compte courant : solde de référence + mouvements comptés
- `AccountKind::Checking(CheckingDetails)` porte les champs suivants :
  - `reference_balance` : solde de référence, signé ;
  - `reference_on` : date de la référence, en `NaiveDate` ;
  - `reference_at` : instant de la référence, en `DateTime<Utc>`.
- `Expense` et `OneOffIncome` gagnent `account: Option<AccountId>` et `recorded_at: Option<DateTime<Utc>>`. Ils sont ajoutés par des méthodes chaînables (`linked_to`, `recorded_at`) pour ne pas casser les quinze appels existants à `new`.
- Règle de comptage : un mouvement compte si `recorded_at > reference_at` **et** `date >= reference_on`.
  - Les mouvements antérieurs à l'arrivée de la fonctionnalité (`recorded_at` absent) ne comptent jamais.
  - En éditant une dépense, on conserve son `recorded_at`.
  - *Alternative écartée — comparaison sur la seule date.* Une dépense du jour saisie après le solde du matin ne serait jamais comptée, et une dépense du jour déjà incluse dans le solde saisi le soir serait comptée deux fois.
  - *Alternative écartée — comparaison sur le seul horodatage.* Une dépense ancienne oubliée puis saisie serait retranchée alors que la banque l'a déjà débitée.
- `CheckingLedger`, côté domaine, regroupe par compte les mouvements rattachés. Il est construit à partir des dépenses et des revenus.
  - `value_account(account, prices, ledger)` valorise le compte courant à partir de la référence et des mouvements comptés.
  - `AccountValuation` gagne `movements` : les mouvements comptés, pour l'écran de détail.
- Les soldes négatifs passent par un nouveau `Money::validate_signed` (centimes seulement). La part dans la répartition est nulle pour un solde négatif.
- La projection garde le compte courant à son solde courant : taux nul dans les trois scénarios, sans plafond ni versement programmé.

### D2 — Rattachement validé dans l'application
- `BudgetUseCases` reçoit l'`AccountRepository`. Le compte rattaché doit exister, être un compte courant et ne pas être archivé (`AppError::InvalidAccountLink`).
- Pas de clé étrangère SQLite : l'import `replace_all` insère les dépenses avant les comptes, et l'intégrité est vérifiée à l'import de la sauvegarde.
- `AccountUseCases` reçoit le `BudgetRepository` pour construire le grand livre. `DashboardUseCases` et `ProjectionUseCases` l'ont déjà.
- La commande `update_savings_balance` devient `update_balance`. Elle accepte les livrets (solde positif ou nul) et les comptes courants (solde signé).

### D3 — Stockage et sauvegarde
- La migration SQLite `002_checking_account.sql` ajoute :
  - `accounts.reference_at` (la nature `checking` réutilise `balance` et `balance_updated_on`) ;
  - `expenses.account_id`, `expenses.recorded_at`, `one_off_incomes.account_id` et `one_off_incomes.recorded_at` ;
  - un index sur `expenses.account_id`.
- La sauvegarde passe en `schema_version` 2 :
  - les nouveaux champs sont optionnels (`#[serde(default)]`) ;
  - la migration v1 → v2 du codec JSON est neutre ;
  - l'import vérifie que chaque rattachement désigne un compte courant du fichier.

### D4 — Mises à jour : manifeste publié par la CI
- La CI construit l'APK avec les valeurs suivantes :
  - `LC_BUILD_NUMBER` = `github.run_number` ;
  - `LC_UPDATE_CHANNEL` = `dernier-build` (branche main) ou `stable` (tag) ;
  - `LC_COMMIT` ;
  - `--config '{"bundle":{"android":{"versionCode":100000+run}}}'`.
- Le `versionCode` actuel est 1000 (dérivé de 0.1.0) ; le nouveau sera toujours plus grand et strictement croissant.
- `latest.json` est publié comme asset de la release : `{build, version, commit, channel, url, size, sha256, notes, published_at}`. Les URL fixes sont :
  - `…/releases/download/dernier-build/latest.json` ;
  - `…/releases/latest/download/latest.json` pour les versions taguées.
- *Alternative écartée — l'API GitHub.* Elle impose une limite de 60 requêtes par heure, et le format des noms d'asset est fragile.

### D5 — Ports et adaptateurs de mise à jour
- Le module `application::updates` contient :
  - `BuildInfo` (version, build, canal, commit) ;
  - `ReleaseManifest` ;
  - les ports `ReleaseFeed::latest(channel)` et `PackageDownloader::download(manifest, dest, progress)` ;
  - `UpdateService::check()`, qui renvoie `UpToDate`, `Available(manifest)` ou `Unsupported` (build local sans numéro). C'est une comparaison pure, testée.
  - Le service refuse une URL hors de `https://github.com/<dépôt>/releases/download/`.
- L'infrastructure fournit `GithubReleaseFeed` (reqwest) et `HttpPackageDownloader` :
  - téléchargement en flux dans un fichier temporaire ;
  - contrôle de la taille et du SHA-256 (crate `sha2`), puis renommage ;
  - suppression du fichier si le contrôle échoue.
- Throttling dans `src-tauri` : 6 heures entre deux vérifications automatiques, mémorisé en mémoire (session).
  - L'option « vérifier automatiquement » est une préférence d'appareil de l'UI (`localStorage`), comme le mode discret.
  - Le report « Plus tard » est mémorisé par build, de la même façon.

### D6 — Installation : plugin Kotlin dans le module app
- `ApkInstallerPlugin.kt` vit dans `gen/android/app` (package `com.liquidcompagnon.app`). Il est enregistré côté Rust par un mini-plugin Tauri (`register_android_plugin`). Pas de crate séparée : un seul fichier Kotlin.
- Commande `install(path)` :
  1. si `canRequestPackageInstalls()` est faux, ouvrir `ACTION_MANAGE_UNKNOWN_APP_SOURCES` et répondre `permission_required` ;
  2. sinon, `FileProvider.getUriForFile` (fournisseur existant `${applicationId}.fileprovider`, chemin `cache/updates/`) et `ACTION_VIEW` sur `application/vnd.android.package-archive`.
- Le manifeste déclare `REQUEST_INSTALL_PACKAGES`. Les règles R8 conservent la classe du plugin, chargée par réflexion.
- L'APK est téléchargé dans `app_cache_dir()/updates/`.
- Sécurité :
  - HTTPS, empreinte SHA-256 publiée par la même CI que l'APK ;
  - Android refuse d'installer par-dessus un APK signé avec une autre clé.
- Sur bureau (développement, e2e), l'installation renvoie `unsupported`.

### D7 — UI
- Compte courant :
  - type ajouté à la création ;
  - écran de détail (solde, référence, mouvements, « Mettre à jour le solde » acceptant un montant négatif) ;
  - mise à jour rapide des soldes étendue ;
  - choix « Payé depuis » / « Versé sur » (puces) dans les feuilles de dépense et de revenu, compte par défaut présélectionné.
- Mises à jour :
  - la vérification part au déverrouillage ;
  - une feuille « Nouvelle version disponible » s'affiche une fois par build ;
  - une pastille apparaît sur l'engrenage ;
  - la section « Mises à jour » des réglages affiche la version et le build, « Vérifier maintenant », l'interrupteur de vérification automatique, le bouton « Mettre à jour » et une barre de progression alimentée par l'événement Tauri `update-progress`.

## Risks / Trade-offs

- **Kotlin non compilable en local.**
  - Risque : une erreur n'apparaît qu'en CI ; un comportement n'apparaît que sur le téléphone.
  - Parade : code minimal calqué sur les plugins officiels, règles R8 explicites.
  - Validation : installer le build N, puis déclencher un build N+1 vide (`workflow_dispatch`) pour tester la mise à jour réelle.
- **Première installation manuelle.** L'APK actuel n'a pas de module de mise à jour. Parade : le README et le message de livraison le disent ; c'est la dernière installation manuelle.
- **Solde courant faux si l'utilisateur oublie des dépenses.** Parade : la référence est datée et affichée, et le recalage se fait en un geste (« Mettre à jour le solde »).
- **Release « dernier-build » recréée à chaque push.** Risque : courte fenêtre où `latest.json` est absent. Parade : traitée comme « vérification impossible », silencieuse au démarrage.
- **Dérogation à « rien ne quitte le téléphone ».** Parade : une lecture publique sans donnée personnelle, désactivable, documentée.

## Migration Plan

1. Migration SQLite 002 appliquée au démarrage (transaction). Les données V1 restent valides : aucun rattachement.
2. Sauvegarde : v2 en écriture, v1 et v2 en lecture.
3. Livraison :
   - push sur main, puis APK « dernier-build » à installer manuellement une dernière fois ;
   - les builds suivants se proposent depuis l'application.
4. Retour arrière : réinstaller un APK plus ancien est impossible par-dessus, car le versionCode serait inférieur. Il faut corriger en avant (nouveau build).

## Open Questions

- Aucune. Les deux choix produit ont été tranchés par l'utilisateur le 21/10/2026.
