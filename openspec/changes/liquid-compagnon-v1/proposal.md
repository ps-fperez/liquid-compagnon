## Why

Mon épargne est éclatée entre plusieurs supports (Livret A, LDDS, PEA, PEI, crypto) sans vue consolidée ni projection fiable, et les applications existantes hébergent les données chez un tiers. Liquid Compagnon fournit une vue agrégée et prévisionnelle de mon patrimoine dans une application Android native où toutes les données restent sur le téléphone.

## What Changes

- Création d'une application Android (APK) "Liquid Compagnon" en Rust : Tauri 2 + Leptos (WebAssembly) + Tailwind CSS v4.
- Base SQLite locale, verrouillage de l'app par code PIN / biométrie, export/import JSON manuel pour les sauvegardes.
- Saisie du revenu mensuel (salaire) avec historique des évolutions.
- Saisie des charges fixes récurrentes (mensuelles, trimestrielles, annuelles) et des dépenses ponctuelles, catégorisées.
- Gestion des comptes d'épargne : Livret A, LDDS (solde + taux), PEA et crypto (positions en quantités), PEI (fonds rattachés à un indice de référence + valeur de part manuelle).
- Valorisation temps réel : CoinGecko pour la crypto, Twelve Data pour les ETF/actions et les indices de référence du PEI ; cache local et mode hors ligne.
- Dashboard : patrimoine total, répartition par compte, variation, courbe d'évolution basée sur des snapshots mensuels.
- Projection de l'épargne à 3 mois, 6 mois et 1 an selon trois scénarios (pessimiste, neutre, optimiste) avec rendement paramétrable par compte.
- Shell UI mobile-first, thème sombre "Purple Electro" (violet / bleu, glow ciblé), navigation par barre inférieure.

Hors périmètre V1 (prévu en V2) : import et parsing des fiches de paie PDF et des relevés bancaires PDF (BoursoBank, Fortuneo), catégorisation automatique des opérations, thème clair, synchronisation multi-appareils.

## Capabilities

### New Capabilities

- `app-security`: verrouillage de l'application (PIN + biométrie Android), stockage local privé, aucune donnée transmise hors appareil hormis les identifiants d'actifs pour les cours.
- `data-backup`: export complet des données en fichier JSON versionné et réimport avec validation.
- `income-management`: saisie du salaire net mensuel avec date d'effet et historique.
- `expense-tracking`: charges fixes récurrentes et dépenses ponctuelles, catégories, calcul de l'épargne mensuelle disponible.
- `account-management`: création et édition des comptes (Livret A, LDDS, PEA, PEI, crypto), soldes, taux, positions et fonds.
- `market-pricing`: récupération des cours temps réel (CoinGecko, Twelve Data), valorisation PEI par indice de référence avec surcharge manuelle, cache et fonctionnement hors ligne.
- `wealth-dashboard`: patrimoine total agrégé, découpage par compte, variations, snapshots mensuels et courbe d'évolution.
- `savings-projection`: projection 3 / 6 / 12 mois, trois scénarios, rendements paramétrables par compte.
- `app-shell-theme`: navigation mobile, thème sombre Purple Electro, design tokens Tailwind, effets glow et accessibilité (réduction des animations).

### Modified Capabilities

_Aucune — projet nouveau._

## Impact

- Nouveau dépôt `liquid-compagnon/` : workspace Cargo (`crates/domain`, `crates/application`, `crates/infrastructure`, `crates/shared-dto`, `src-tauri`, `ui`).
- Dépendances principales : `tauri` 2 (+ plugins `biometric`, `dialog`, `fs`, `http` ou `reqwest`), `leptos` (CSR), `trunk`, `tailwindcss` v4, `rusqlite` (bundled), `rust_decimal`, `serde`, `chrono`, `thiserror`, `async-trait`, `argon2` (hash du PIN).
- Services externes : API CoinGecko (publique) et API Twelve Data (clé gratuite fournie par l'utilisateur, stockée localement).
- Environnement de build : Rust + cibles Android, Android SDK/NDK, JDK 17 — l'Android SDK n'est pas encore installé sur le poste de développement.
