# Liquid Compagnon

Application Android personnelle de suivi de patrimoine et d'épargne : vue agrégée des comptes (Livret A, LDDS, PEA, PEI, crypto), budget, cours en temps réel et projection à 3, 6 et 12 mois. Toutes les données restent sur le téléphone (SQLite local).

Spécification : [`openspec/changes/liquid-compagnon-v1`](openspec/changes/liquid-compagnon-v1/).

## Stack

- Tauri 2 (APK Android), backend Rust
- UI Leptos (CSR) compilée en WebAssembly par Trunk, Tailwind CSS v4
- Clean Architecture : workspace Cargo

```
crates/domain          entités et règles métier pures
crates/application     cas d'usage + ports (traits)
crates/infrastructure  SQLite, fournisseurs de cours, horloge
crates/shared-dto      DTO échangés entre UI et backend
src-tauri/             composition root + commandes Tauri
ui/                    Leptos + Tailwind
```

## Développement (PC)

Prérequis : Rust stable, `tauri-cli` 2 (`cargo install tauri-cli --locked`), `trunk` (`cargo install trunk --locked`), cible `wasm32-unknown-unknown`.

```bash
cargo tauri dev
```

Ouvre l'application dans une fenêtre au format téléphone (412 × 892), avec rechargement à chaud de l'UI.

Vérifications (formatage, clippy, tests) :

```bash
powershell -File scripts/check.ps1
```

Smoke test de l'application complète, **obligatoire avant tout push qui produit un APK**. Il construit l'app de bureau avec les assets embarqués (même configuration que l'APK : CSP, protocole `tauri.localhost`), la lance, puis vérifie que l'UI démarre sans erreur et dialogue avec le backend (nécessite Node.js) :

```bash
powershell -File scripts/smoke.ps1
```

## APK Android

L'APK est construit par GitHub Actions ([`.github/workflows/android.yml`](.github/workflows/android.yml)) à chaque push sur `main` et à chaque tag `v*`. Le build local n'est pas nécessaire (il exige le mode développeur Windows pour les liens symboliques).

- Dernier build de `main` : release **dernier-build** du dépôt.
- Versions : releases `vX.Y.Z` (créer un tag `v0.1.0` puis le pousser).

### Installer sur le téléphone

1. Depuis le téléphone, ouvrir la release sur GitHub (connecté au compte) et télécharger le fichier `.apk`.
2. Autoriser « Installer des applis inconnues » pour le navigateur utilisé.
3. Ouvrir le fichier téléchargé et installer.

### Signature

Sans clé configurée, la CI produit un APK de **debug** dont la signature change à chaque build : il faut désinstaller l'application avant chaque mise à jour (les données sont perdues).

Pour des mises à jour qui conservent les données, créer une fois la clé de signature :

```bash
powershell -File scripts/new-keystore.ps1
```

Le script demande un mot de passe, crée `%USERPROFILE%\.liquid-compagnon\release.jks` et enregistre les secrets `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD` et `ANDROID_KEY_ALIAS` dans le dépôt GitHub. **Sauvegarder ce fichier hors du PC** : sans lui, aucune mise à jour ne pourra s'installer par-dessus la version existante.
