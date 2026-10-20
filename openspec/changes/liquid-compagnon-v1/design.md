## Context

Liquid Compagnon est une application Android personnelle, mono-utilisateur, sans backend. Le projet part de zéro (dépôt `liquid-compagnon/` vide). Contraintes structurantes issues du cadrage :

- APK natif installable, pas de PWA ; code en Rust, UI en WebAssembly + Tailwind.
- Données uniquement sur le téléphone (SQLite), sauvegarde par export JSON manuel.
- Valorisation temps réel via API publiques gratuites (CoinGecko, Twelve Data), le PEI n'ayant pas d'API.
- Clean Architecture + SOLID, sans sur-ingénierie (un seul utilisateur, un seul développeur).
- Poste de dev Windows 11 : Rust 1.94 et un JDK présents ; Android SDK/NDK à installer.

## Goals / Non-Goals

**Goals:**
- Un workspace Cargo où la règle de dépendance de la Clean Architecture est imposée par le compilateur.
- Un domaine pur, testable unitairement à 100 % sans I/O (valorisation, budget, projection).
- Des points d'extension (ports) là où la variabilité est réelle : sources de cours, stockage, et en V2 parseurs PDF.
- Un premier APK fonctionnel tôt (squelette + verrouillage + dashboard), puis enrichissement incrémental.

**Non-Goals:**
- Import / parsing PDF des fiches de paie et relevés (V2 ; le port `DocumentParser` sera ajouté à ce moment-là).
- Synchronisation, multi-appareils, multi-utilisateurs, comptes courants bancaires détaillés.
- Thème clair, iOS, version desktop (Tauri le permettrait mais hors périmètre).
- CQRS, bus d'événements, une crate par fonctionnalité.

## Decisions

### D1 — Tauri 2 + Leptos (CSR) plutôt que Dioxus ou Yew
Tauri 2 produit un APK Android avec un backend Rust natif et une WebView qui exécute l'UI Leptos compilée en WASM par Trunk. Leptos (0.8, mode CSR) offre une réactivité fine (signals) plus performante que Yew. Dioxus mobile ne compile pas l'UI en WASM et son support Android est moins mature.
*Alternatives* : Yew (plus ancien, VDOM moins performant), Dioxus (s'écarte de la contrainte WASM), Slint (pas de Tailwind).

### D2 — Workspace Cargo en 4 crates + 2 binaires
```
liquid-compagnon/
├── Cargo.toml                 # [workspace]
├── crates/
│   ├── domain/                # entités, value objects, services métier purs
│   ├── application/           # use cases + ports (traits) ; dépend de domain
│   ├── infrastructure/        # SQLite, HTTP, horloge ; implémente les ports
│   └── shared-dto/            # DTO serde partagés UI <-> backend
├── src-tauri/                 # composition root + #[tauri::command] ; dépend de tout sauf ui
└── ui/                        # Leptos CSR (wasm32) ; dépend uniquement de shared-dto
```
`domain` n'a comme dépendances que `rust_decimal`, `chrono`, `thiserror`, `uuid` (sans I/O). L'UI ne peut pas importer `domain` : les DTO sont la frontière. Les conversions domaine ↔ DTO vivent dans `src-tauri` (couche présentation).
*Alternative* : une seule crate avec des modules — rejetée car la règle de dépendance ne serait plus vérifiée par le compilateur.

### D3 — Modèle du domaine
- `Money` : newtype sur `Decimal`, EUR uniquement, opérations checkées ; jamais de `f64`.
- `Rate` : rendement / taux annuel en `Decimal` (0.03 = 3 %).
- `Account { id, name, color, kind: AccountKind, monthly_contribution: Money, scenario_rates: ScenarioRates, archived }`
- `AccountKind` (enum à données) :
  - `Savings { product: Livret A | LDDS, balance, rate, deposit_ceiling, balance_updated_at }`
  - `Securities { product: PEA | Crypto, cash: Money, positions: Vec<Position> }`
  - `EmployeeSavings { funds: Vec<PeiFund> }` (PEI)
- `Position { asset: AssetRef, quantity: Decimal, unit_cost: Option<Money> }`
- `AssetRef { provider: CoinGecko | TwelveData, symbol, display_name, currency }`
- `PeiFund { name, units, reference: Option<AssetRef>, anchor: NavAnchor { nav: Money, date, reference_price: Option<Money> } }`
- `SalaryEntry { effective_month, net_amount }`, `OneOffIncome`, `RecurringCharge { periodicity, start, end }`, `Expense`, `Category`.
- `Snapshot { month, kind: Auto | Manual, account_values: Vec<(AccountId, Money)>, total }`.
- `ScenarioRates { pessimistic, neutral, optimistic }`.

Services purs : `Valuation` (valeur d'un compte à partir d'un `PriceBook`), `PeiEstimator`, `BudgetCalculator` (salaire applicable, équivalent mensuel, capacité d'épargne), `ProjectionEngine`.

L'enum `AccountKind` (plutôt qu'un trait objet par type) est un choix délibéré : l'ensemble des types est fermé et connu, le `match` exhaustif garantit qu'un nouveau type est traité partout (OCP respecté au niveau des ports, pas des types fermés).

### D4 — Ports (application) — interfaces petites (ISP)
```rust
pub trait AccountRepository   { fn list(&self); fn get(&self, id); fn save(&self, account); }
pub trait BudgetRepository    { /* catégories, salaires, revenus ponctuels, charges, dépenses : liste, save, delete */ }
pub trait SnapshotRepository  { fn list(&self); fn save(&self, snapshot); }
pub trait PriceCache          { fn get(&self, a: &AssetRef) -> Option<Quote>; fn put(&self, q: &Quote); }
#[async_trait] pub trait PriceProvider { fn source(&self) -> QuoteSource; async fn quotes(&self, a: &[AssetRef]) -> Result<Vec<Quote>, PriceError>; async fn search(&self, q: &str) -> Result<Vec<AssetRef>, PriceError>; }
pub trait PinHasher           { fn hash(&self, pin); fn verify(&self, pin, hash) -> bool; }   // Argon2id
pub trait SettingsStore       { /* hash du PIN, compteur d'échecs, biométrie, clé Twelve Data, date du dernier export */ }
pub trait BackupCodec         { fn encode(&self, data: &FullDataset); fn decode(&self, bytes) -> Result<FullDataset, BackupError>; }
pub trait DatasetStore        { fn replace_all(&self, data: &Dataset); }   // import en une transaction
pub trait Clock               { fn now(&self) -> DateTime<Utc>; fn today(&self) -> NaiveDate; }
```
`Quote { asset, price (EUR), as_of, freshness: Live | Cached }`. Un `PricingService` (application) compose plusieurs `PriceProvider` + `PriceCache` : il route chaque actif vers le provider de sa `source`, applique le TTL de 5 min et le repli sur le cache. La provenance affichée (« en direct / en cache / estimée / saisie manuelle / prix de revient ») est calculée par le domaine (`valuation::Provenance`).

### D5 — Cas d'usage regroupés par domaine fonctionnel (SRP par méthode)
Une struct par domaine fonctionnel, une méthode par cas d'usage, dépendances injectées en `Arc<dyn Port>` :
- `SecurityUseCases` : `setup_pin`, `unlock` (5 essais puis blocage 30 s doublé), `change_pin`, biométrie, clé API.
- `BudgetUseCases` : catégories (création, renommage, archivage, suppression si inutilisée), salaires, revenus ponctuels, charges, dépenses, `summary`.
- `AccountUseCases` : création, modification, archivage, soldes, conditions des livrets, rendements de scénarios, positions, fonds PEI (ancrage avec le cours de référence du jour), recherche d'actifs.
- `PricingService` : cours groupés par source, cache, repli hors ligne, problèmes signalés (`PricingIssue`).
- `DashboardUseCases` : tableau de bord (+ snapshot automatique du mois), snapshot manuel.
- `ProjectionUseCases` : projection 12 mois sur 3 scénarios, alerte de versements excessifs.
- `BackupUseCases` : export (`FullDataset` v1, sans secrets), import validé, rappel d'export à 30 jours.

*Écart assumé par rapport à « une struct par cas d'usage »* : ~25 structs quasi vides n'apportaient que du boilerplate ; le regroupement garde une responsabilité par méthode, des dépendances explicites et testables (fakes en mémoire dans `application::testing`).

### D6 — Persistance : rusqlite + migrations embarquées
`rusqlite` (feature `bundled`, donc pas de dépendance au SQLite système Android) + migrations SQL versionnées dans `infrastructure/migrations/`, appliquées au démarrage selon `PRAGMA user_version` (une transaction par migration ; une base créée par une version plus récente est refusée). Choix d'une vingtaine de lignes maison plutôt que `rusqlite_migration`, dont la version doit suivre celle de `rusqlite`. Clés étrangères activées : une dépense ne peut référencer une catégorie absente. Montants stockés en `TEXT` (représentation décimale exacte), dates en ISO 8601. Une `Connection` unique derrière un `Mutex`, appelée via `tauri::async_runtime::spawn_blocking`.
Tables : `accounts`, `savings_details`, `positions`, `pei_funds`, `pei_nav_anchors`, `salaries`, `one_off_incomes`, `recurring_charges`, `expenses`, `categories`, `snapshots`, `snapshot_lines`, `price_cache`, `settings` (clé/valeur, dont hash PIN et clé API).
*Alternatives* : `sqlx` (async, macros compile-time lourdes à configurer pour Android), JSON pur (pas de requêtes, risque de corruption à l'écriture), SQLCipher (complexité de build NDK — écarté par choix utilisateur).

### D7 — Valorisation et cours
- **CoinGecko** : `GET /api/v3/simple/price?ids=…&vs_currencies=eur` (tous les actifs en une requête) et `/search` pour la recherche. Clé « demo » optionnelle si l'API publique devient restrictive.
- **Twelve Data** : `/price` (batch via `symbol=A,B`), `/symbol_search`, `/exchange_rate` pour la conversion en EUR. Symboles au format `TICKER:PLACE` (ex. `CW8:Euronext`) issus de `/symbol_search`. Offre gratuite : 8 crédits/min (1 par symbole et par conversion de devise), 800/jour → requête groupée + TTL 5 min + compteur local de crédits : seuls les actifs que le quota permet sont demandés, les autres restent servis par le cache (pas d'erreur 429 provoquée).
- Client HTTP : `reqwest` 0.12 avec `rustls-tls` (fournisseur `ring`, certificats racine `webpki-roots` embarqués) côté `infrastructure`. `reqwest` 0.13 a été écarté : son TLS par défaut impose `aws-lc` (C) et un vérificateur de plateforme qui exige une initialisation JNI sur Android. L'UI WASM ne fait **aucun** appel réseau.
- **PEI** : `PeiEstimator::estimate(anchor, current_ref_price) = anchor.nav × current / anchor.reference_price`. À chaque `RecordPeiNav`, le cours de référence du jour est capturé et devient le nouvel ancrage.

### D8 — Projection
`ProjectionEngine::project(accounts, budget, scenario, months) -> ProjectionResult` pur et déterministe. Facteur mensuel `(1 + r)^(1/12)` calculé avec `rust_decimal` (`powd`/`MathematicalOps`), arrondi seulement à l'affichage. Les plafonds Livret A / LDDS déroutent l'excédent vers « épargne non affectée » (rendement 0). Valeurs par défaut des rendements dans `domain::defaults` :

| Type | Pessimiste | Neutre | Optimiste |
|---|---|---|---|
| Livret A / LDDS | taux du compte | taux du compte | taux du compte |
| PEA / PEI actions | 0 % | 5 % | 8 % |
| Crypto | -40 % | 5 % | 50 % |

Les taux réglementés et plafonds par défaut (Livret A 22 950 €, LDDS 12 000 €) sont des constantes nommées, à vérifier à l'implémentation et modifiables par l'utilisateur.

### D9 — Sécurité
- PIN : `argon2` (Argon2id, sel aléatoire), stocké dans `settings`. Compteur d'échecs + délai exponentiel persistés (résiste au redémarrage).
- Biométrie : plugin officiel `tauri-plugin-biometric` (Android BiometricPrompt).
- Verrouillage après 60 s en arrière-plan : événement de cycle de vie Tauri (`RunEvent::Resumed` / pause) + horodatage.
- `FLAG_SECURE` et `android:allowBackup="false"` : modifications du projet Android généré (`src-tauri/gen/android`), `MainActivity.kt` et `AndroidManifest.xml`, versionnés.
- Clé Twelve Data : en clair dans `settings` (base privée de l'app, non sauvegardée) ; exclue de l'export.

### D10 — UI Leptos
- Routage `leptos_router` : `/lock`, `/`, `/accounts`, `/accounts/:id`, `/budget`, `/projection`, `/settings`.
- Appels backend via `tauri-sys`/`wasm-bindgen` `invoke`, encapsulés dans un module `ui::api` typé sur `shared-dto` (un seul endroit connaît les noms de commandes).
- État global léger : `Resource` par écran + un contexte `AppState` (verrouillé, mode discret).
- Graphiques en **SVG généré en Rust** (courbe patrimoine, projection, barre de répartition) — pas de librairie JS, poids WASM maîtrisé.
- Tailwind v4 via Trunk (`data-trunk rel="tailwind-css"`), tokens dans `ui/styles/theme.css` (`@theme`), utilitaires `shadow-glow-*`, `animate-glow-pulse`, respect de `prefers-reduced-motion`.
- Formatage FR (montants, dates) centralisé dans `ui::format`.

### D11 — Export JSON
`serde_json` sur un `FullDataset` versionné (`schema_version: 1`) défini dans `application` ; migrations d'import en chaîne (`v1 → v2 → …`). Écriture via `tauri-plugin-dialog` + `tauri-plugin-fs` (Storage Access Framework). Import en transaction unique, après validation complète en mémoire.

### D12 — Tests
- `domain` : tests unitaires exhaustifs + tests de propriété (`proptest`) sur `Money` et `ProjectionEngine`.
- `application` : use cases testés avec des fakes en mémoire des ports.
- `infrastructure` : SQLite `:memory:` + migrations ; clients HTTP testés contre `wiremock`.
- CI locale : `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`.

## Risks / Trade-offs

- [API publique CoinGecko restreinte ou modifiée] → port `PriceProvider` interchangeable, cache, clé demo optionnelle ; ajout possible d'un provider alternatif sans toucher au domaine.
- [Quota Twelve Data gratuit (800/j) insuffisant] → TTL 5 min, batch, pas de rafraîchissement automatique en arrière-plan ; l'usage typique (quelques ouvertures/jour, < 10 actifs) reste très en dessous.
- [Fonds PEI mal suivis par l'indice de référence (frais, couverture de change)] → valeur marquée « estimée », recalage manuel recommandé à chaque relevé ; écart affiché lors d'une nouvelle saisie.
- [Toolchain Android Tauri sur Windows (NDK, cibles rust, JDK 17)] → tâche dédiée de mise en place en premier, vérifiée par la génération d'un APK « hello world » avant tout développement métier.
- [Vérification des développeurs Android imposée par Google pour les APK hors Play Store (déploiement progressif annoncé 2026-2027), sans possibilité de passer par `adb`] → vérifier l'état de la règle avant chaque livraison ; options : parcours « avancé » prévu pour les utilisateurs expérimentés, ou compte développeur gratuit à diffusion limitée.
- [Pas de test direct sur le téléphone pendant le développement] → logique métier entièrement testée hors Android (domain/application), UI vérifiée en `cargo tauri dev` ; fonctions spécifiques Android (biométrie, `FLAG_SECURE`, cycle de vie) regroupées et validées sur un APK de recette.
- [Perte du téléphone = perte des données] → rappel d'export périodique (bandeau si dernier export > 30 jours).
- [Export JSON en clair] → avertissement explicite ; chiffrement de l'export envisagé (question ouverte).
- [Taille du bundle WASM] → `opt-level = "z"`, `lto = true`, `wasm-opt` via Trunk ; pas de lib de graphiques JS.
- [`rust_decimal` et exposant fractionnaire] → vérifier la précision de `powd` ; à défaut, précalcul du facteur mensuel par série de Taylor/Newton testée.

## Migration Plan

Pas de migration (nouvelle application). Le téléphone ne peut pas être relié au PC (ni USB, ni débogage ADB) : le développement courant se fait en `cargo tauri dev` sur le PC (fenêtre au format téléphone), et chaque version est livrée sous forme de fichier APK transféré sur le téléphone par un canal autorisé, puis installé (sources inconnues autorisées pour ce canal). Déploiement : `cargo tauri android build --apk` → transfert → installation. Signature avec une keystore personnelle conservée hors dépôt. Les mises à jour remplacent l'APK en conservant la base (même `applicationId` et même signature) ; les migrations SQLite s'appliquent au démarrage.

## Open Questions

- Chiffrer l'export JSON par mot de passe (Argon2 + AES-GCM) dès la V1, ou garder un export en clair ?
- Liste exacte des fonds du PEI et leurs indices de référence (à fournir par l'utilisateur).
- Taux réglementés en vigueur au moment de l'implémentation (Livret A, LDDS) pour les valeurs par défaut.
- Banque « autre » pour la V2 (Fortuneo, Hello bank ou autre) — à préciser avant la V2.
