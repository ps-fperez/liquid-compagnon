## 1. Domaine — compte courant

- [x] 1.1 `AccountType::Checking`, `CheckingDetails` (référence signée, date, instant), `Account::open` et valeurs par défaut (couleur, rendements nuls), `Money::validate_signed`, mise à jour de la référence ; tests
- [x] 1.2 `Expense` / `OneOffIncome` : compte rattaché et instant d'enregistrement (`linked_to`, `recorded_at`) ; tests
- [x] 1.3 `CheckingLedger`, règle de comptage, `value_account` et `AccountValuation::movements`, projection sans rendement ; tests des scénarios de la spec checking-account

## 2. Application et stockage — compte courant

- [x] 2.1 Cas d'usage : rattachement validé (`InvalidAccountLink`), `recorded_at` horodaté à la création et conservé à l'édition, `update_balance` (livret ou compte courant), grand livre dans comptes, tableau de bord et projection ; tests
- [x] 2.2 Migration SQLite 002, lecture et écriture des nouveaux champs, `replace_all` ; tests
- [x] 2.3 Sauvegarde v2 (enregistrements, migration v1 → v2 neutre, contrôle des rattachements à l'import) ; tests aller-retour et import v1

## 3. Présentation — compte courant

- [x] 3.1 DTO et mapping (`AccountDetailsDto::Checking`, mouvements, `account_id` des saisies), commande `update_balance`
- [x] 3.2 UI : type à la création, détail du compte courant, mise à jour rapide des soldes, saisie d'un solde négatif
- [x] 3.3 UI : choix du compte dans les feuilles de dépense et de revenu ponctuel (défaut : premier compte courant actif), répartition sans solde négatif

## 4. Mises à jour

- [x] 4.1 `application::updates` : `BuildInfo`, `ReleaseManifest`, ports, `UpdateService::check` (comparaison de build, contrôle d'URL) ; tests
- [x] 4.2 Infrastructure : `GithubReleaseFeed`, `HttpPackageDownloader` (flux, taille, SHA-256, suppression si altéré) ; tests wiremock
- [x] 4.3 src-tauri : informations de build compilées, mini-plugin `apk-installer`, commandes `update_status`, `check_for_update` (limite de 6 h), `install_update` (progression par événement)
- [x] 4.4 Android : `ApkInstallerPlugin.kt`, permission `REQUEST_INSTALL_PACKAGES`, chemin `updates/` du FileProvider, règles R8
- [x] 4.5 CI : variables de build, `versionCode` = 100000 + run, génération et publication de `latest.json`
- [x] 4.6 UI : vérification au déverrouillage, feuille « Nouvelle version disponible » (une fois par build), pastille sur l'engrenage, section « Mises à jour » des réglages avec progression

## 5. Vérification et livraison

- [x] 5.1 Parcours e2e étendu (compte courant, dépense rattachée, solde courant, section Mises à jour), `scripts/check.ps1`, `scripts/smoke.ps1`, `scripts/e2e.ps1` au vert
- [x] 5.2 README (compte courant, mises à jour), `docs/recette-v1.md` complétée, `openspec validate`
- [x] 5.3 Push, CI verte, APK à installer une dernière fois à la main, puis build N+1 pour valider la mise à jour depuis l'application sur le téléphone
