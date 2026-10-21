# Liquid Compagnon

Application Android personnelle de suivi de patrimoine et d'épargne : vue agrégée des comptes (compte courant, Livret A, LDDS, PEA, PEI, crypto), budget, cours en temps réel et projection à 3, 6 et 12 mois. Toutes les données restent sur le téléphone (SQLite local, aucune sauvegarde cloud) ; seuls les identifiants d'actifs partent vers CoinGecko, Yahoo Finance et Twelve Data pour les cours, et la vérification des mises à jour lit un fichier public sur github.com (désactivable).

Spécification : [`openspec/changes/liquid-compagnon-v1`](openspec/changes/liquid-compagnon-v1/), puis [`compte-courant-et-mises-a-jour`](openspec/changes/compte-courant-et-mises-a-jour/) (v0.2). Recette : [`docs/recette-v1.md`](docs/recette-v1.md).

## Stack

- Tauri 2 (APK Android), backend Rust ; l'UI dialogue avec lui par l'IPC Tauri (pas de serveur HTTP)
- UI Leptos (CSR) compilée en WebAssembly par Trunk, Tailwind CSS v4, graphiques SVG générés en Rust
- SQLite embarqué (`rusqlite`), PIN haché en Argon2id, cours via `reqwest` (rustls + ring)
- Clean Architecture : workspace Cargo

```
crates/domain          entités et règles métier pures (aucune I/O)
crates/application     cas d'usage + ports (traits)
crates/infrastructure  SQLite, fournisseurs de cours, horloge, sauvegarde JSON
crates/shared-dto      contrats échangés entre UI et backend
src-tauri/             composition root, session, commandes Tauri
ui/                    Leptos + Tailwind
```

## Installer sur le téléphone

Le téléphone n'est jamais relié au PC : l'APK est construit par GitHub Actions et téléchargé depuis le téléphone.

1. Depuis le téléphone, ouvrir la release voulue sur GitHub (connecté au compte) :
   - **dernier-build** : dernier commit de `main` ;
   - **vX.Y.Z** : versions publiées.
2. Télécharger le fichier `.apk`, autoriser « Installer des applis inconnues » pour le navigateur, ouvrir le fichier.

### Mises à jour sans perte de données

Une mise à jour s'installe par-dessus la version existante (données conservées) **seulement si les deux APK sont signés avec la même clé**. Sans clé configurée, la CI produit un APK de **debug** dont la signature change à chaque build : il faut alors désinstaller (et perdre les données) avant chaque mise à jour.

Créer la clé une seule fois, avant d'utiliser l'app avec de vraies données :

```bash
powershell -File scripts/new-keystore.ps1
```

Le script demande un mot de passe (jamais affiché ni écrit en clair), crée `%USERPROFILE%\.liquid-compagnon\release.jks` et enregistre les secrets `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD` et `ANDROID_KEY_ALIAS` dans le dépôt GitHub. Les builds suivants sont des APK de release signés. **Sauvegarder `release.jks` hors du PC** : sans lui, plus aucune mise à jour ne pourra s'installer par-dessus la version existante.

### Mises à jour depuis l'application (à partir de la v0.2)

Chaque APK publié par la CI est accompagné d'un `latest.json` (numéro de build, empreinte SHA-256). Au déverrouillage (au plus toutes les 6 h, désactivable dans Réglages › Mises à jour), l'application compare son build à celui publié et invite à mettre à jour :

1. « Mettre à jour » télécharge l'APK et vérifie sa taille et son empreinte ;
2. la première fois, Android demande d'autoriser Liquid Compagnon à installer des applications : autoriser, revenir, toucher de nouveau « Mettre à jour » ;
3. l'installeur Android propose « Mettre à jour » : les données sont conservées.

La version 0.2 elle-même s'installe une dernière fois à la main (depuis « dernier-build ») : les versions antérieures ne savent pas se mettre à jour. Un build fait sur le PC n'a pas de numéro et ne propose jamais de mise à jour.

### Compte courant

Créer un compte « Compte courant », puis saisir son solde (case « découvert » pour un solde négatif). Les dépenses et revenus ponctuels saisis ensuite sont rattachés par défaut au premier compte courant (choix « Payé depuis » / « Versé sur ») et font évoluer son solde. Le salaire et les charges fixes ne sont pas imputés : recaler le solde de temps en temps (« Mettre à jour le solde ») ; les mouvements déjà saisis sont alors réputés inclus.

### Sauvegarde des données

Réglages › **Exporter mes données** écrit un fichier JSON (sélecteur de fichiers Android). Il contient toutes les données en clair (format v2 depuis la v0.2 ; les exports v1 restent importables), sans le code PIN ni la clé Twelve Data : le conserver dans un endroit sûr. Réglages › **Importer une sauvegarde** le restaure (fichier entièrement vérifié avant tout remplacement). Un rappel s'affiche si aucun export n'a eu lieu depuis 30 jours.

### Cours des ETF

Les crypto-actifs (CoinGecko) et les ETF et actions (Yahoo Finance, places européennes comprises : `CW8.PA`, `IWDA.AS`…) ne demandent aucune clé. Yahoo Finance est la source proposée par défaut pour le PEA et les indices de référence du PEI.

Twelve Data reste disponible au choix dans la recherche d'actif, avec une clé saisie dans Réglages › Cours des ETF (la vérification affiche l'offre associée). Son offre gratuite ne couvre pas Euronext ni les autres places européennes : la recherche signale « offre Grow requise » et un bandeau nomme les actifs non couverts.

Yahoo Finance est une API non officielle : si son format change, les valeurs restent affichées depuis le cache. Contrôle manuel contre l'API réelle :

```bash
cargo test -p lc-infrastructure yahoo -- --ignored
```

## Développement (PC)

Prérequis : Rust stable, `tauri-cli` 2 (`cargo install tauri-cli --locked`), `trunk` (`cargo install trunk --locked`), cible `wasm32-unknown-unknown`, Node.js (scripts de test).

```bash
cargo tauri dev
```

Ouvre l'application dans une fenêtre au format téléphone (412 × 892), avec rechargement à chaud de l'UI. En développement, la galerie des composants est disponible sur `/galerie`.

### Vérifications

Formatage, clippy (avertissements bloquants) et tests :

```bash
powershell -File scripts/check.ps1
```

Smoke test de l'application complète, **obligatoire avant tout push qui produit un APK** : construit l'app de bureau avec les assets embarqués (même configuration que l'APK : CSP, protocole `tauri.localhost`), la lance, vérifie que l'UI démarre sans erreur et que les commandes métier sont refusées tant que la session est verrouillée :

```bash
powershell -File scripts/smoke.ps1
```

Parcours utilisateur de bout en bout en 11 étapes (PIN, comptes, budget, saisie rapide, projection, réglages, robustesse), piloté dans l'app de bureau sur une base temporaire isolée (`LC_DATA_DIR`, builds de développement uniquement). Les captures d'écran sont écrites dans `target/e2e-captures` :

```bash
powershell -File scripts/e2e.ps1
```

Avec `LC_E2E_LIVE=1`, le parcours ajoute une étape réseau : autocomplétion réelle d'un ETF par son ISIN via Yahoo Finance.

## Publier une version

1. Vérifications au vert (`check`, `smoke`, `e2e`), version mise à jour dans `src-tauri/tauri.conf.json`.
2. Taguer et pousser : `git tag v0.1.0` puis `git push origin v0.1.0`.
3. GitHub Actions crée la release `v0.1.0` avec l'APK (signé si la clé est configurée).
4. Dérouler la recette [`docs/recette-v1.md`](docs/recette-v1.md) sur le téléphone.

Le build Android local n'est pas nécessaire (il exige le mode développeur Windows pour les liens symboliques) : la CI s'en charge ([`.github/workflows/android.yml`](.github/workflows/android.yml)).
