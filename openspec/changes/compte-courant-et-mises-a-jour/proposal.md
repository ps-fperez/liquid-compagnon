## Why

Après la recette de la V1, deux manques ressortent de l'usage réel :

- **Pas de compte courant.** Les dépenses ponctuelles sont en réalité payées depuis un compte courant. L'application ne sait ni le suivre ni y rattacher ces dépenses : le patrimoine affiché ignore l'argent disponible au quotidien.
- **Mises à jour manuelles.** Il faut surveiller GitHub, télécharger l'APK depuis le navigateur puis l'installer. Rien n'indique qu'une nouvelle version existe, et la procédure laisse croire qu'il faut désinstaller (avec perte des données).

## What Changes

- Nouveau type de compte **« Compte courant »** :
  - un solde de référence saisi par l'utilisateur, qui peut être négatif (découvert) ;
  - le solde courant est calculé : solde de référence − dépenses rattachées + revenus ponctuels rattachés, pour les mouvements notés après la saisie de la référence ;
  - mettre à jour le solde fixe une nouvelle référence ;
  - le compte entre dans le patrimoine total et apparaît dans la projection (sans rendement).
- Une **dépense ponctuelle** ou un **revenu ponctuel** peut être rattaché à un compte courant. Le compte courant principal est proposé par défaut lors de la saisie rapide.
- Le salaire et les charges fixes ne modifient pas le solde du compte courant : l'utilisateur recale le solde quand il le souhaite.
- **Mises à jour depuis l'application** :
  - la CI publie avec chaque APK un manifeste `latest.json` (numéro de build, empreinte SHA-256, notes) ;
  - au démarrage, l'application compare son numéro de build au manifeste, au plus une fois toutes les 6 heures et seulement si l'option est active ;
  - si une version plus récente existe, elle invite à mettre à jour ;
  - sur demande, elle télécharge l'APK, vérifie son empreinte, puis lance l'installeur Android, qui remplace l'application sans perte de données.
- Le `versionCode` Android devient croissant à chaque build : un APK plus récent s'installe toujours par-dessus l'ancien.
- **BREAKING (sauvegarde)** : le format d'export passe en version 2 (compte courant, rattachement des mouvements). Les fichiers en version 1 restent importables.
- Exception à la règle « rien ne quitte le téléphone hormis les cours » : la vérification des mises à jour interroge `github.com`. Elle n'envoie aucune donnée personnelle et se désactive dans les réglages.

## Capabilities

### New Capabilities
- `checking-account` : compte courant à solde de référence, rattachement des dépenses et revenus ponctuels, calcul du solde courant, intégration au patrimoine, à la projection et à la sauvegarde.
- `app-updates` : détection d'une nouvelle version, invitation à mettre à jour, téléchargement vérifié et installation par l'installeur Android, publication du manifeste par la CI.

### Modified Capabilities
<!-- Aucune spécification principale n'est encore synchronisée (la V1 n'est pas archivée) : les évolutions sont décrites dans les deux nouvelles capacités. -->

## Impact

- **Domaine.**
  - `AccountType::Checking` et ses détails : référence, date et instant de la référence.
  - `Expense` et `OneOffIncome` gagnent un compte rattaché et un horodatage d'enregistrement.
  - Calcul du solde courant, valorisation et projection mis à jour.
- **Application.**
  - Cas d'usage comptes et budget : rattachement, mise à jour de la référence.
  - Nouveau module `updates` avec les ports `ReleaseFeed` et `PackageDownloader`.
  - Sauvegarde en version 2, avec migration depuis la version 1.
- **Infrastructure.**
  - Migration SQLite : colonnes de rattachement et d'horodatage, détails du compte courant.
  - Adaptateurs HTTP : manifeste GitHub, téléchargement avec contrôle SHA-256 (crate `sha2`).
- **src-tauri.**
  - Commandes de mise à jour, et informations de build (`LC_BUILD_NUMBER`, `LC_UPDATE_CHANNEL`) injectées à la compilation.
  - Petit plugin Android natif en Kotlin (`ApkInstallerPlugin`) : permission `REQUEST_INSTALL_PACKAGES`, partage de l'APK via le FileProvider, lancement de l'installeur.
- **UI.**
  - Création et détail du compte courant.
  - Choix du compte dans les saisies de dépense et de revenu.
  - Invitation à mettre à jour, section « Mises à jour » des réglages, progression du téléchargement.
- **CI.** `versionCode` = 100000 + numéro de run, génération et publication de `latest.json` avec chaque APK.
- **Limite.** La première installation de cette version reste manuelle : l'APK actuel ne sait pas se mettre à jour.
