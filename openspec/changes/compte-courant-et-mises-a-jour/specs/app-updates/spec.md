## ADDED Requirements

### Requirement: Publication du manifeste de version
La CI SHALL publier, avec chaque APK de la release « dernier-build » ou d'une version taguée, un fichier `latest.json` contenant : le numéro de build, la version, le commit, l'URL de l'APK, sa taille, son empreinte SHA-256 et le titre du commit. Le `versionCode` Android MUST valoir 100000 + numéro de build, afin que chaque APK soit plus récent que le précédent.

#### Scenario: Build de main
- **WHEN** la CI construit le run n° 57 depuis main
- **THEN** la release « dernier-build » contient l'APK et un `latest.json` indiquant le build 57 et l'empreinte de cet APK, et l'APK a le versionCode 100057

### Requirement: Détection d'une nouvelle version
Au déverrouillage, si la vérification automatique est active, l'application SHALL lire le manifeste de son canal (« dernier-build » pour un build de main, dernière version publiée pour un build tagué), au plus une fois toutes les 6 heures. Une version est proposée MUST seulement si son numéro de build est supérieur à celui de l'application installée. Un build local, sans numéro, MUST ne jamais proposer de mise à jour.

#### Scenario: Version plus récente
- **WHEN** l'application installée est le build 57 et le manifeste annonce le build 60
- **THEN** l'application signale qu'une mise à jour est disponible, avec le titre de la modification

#### Scenario: Déjà à jour
- **WHEN** le manifeste annonce le build installé
- **THEN** aucune invitation n'est affichée et les réglages indiquent « Application à jour »

#### Scenario: Hors ligne
- **WHEN** le manifeste est injoignable
- **THEN** aucune erreur n'interrompt l'utilisation ; une vérification manuelle depuis les réglages affiche « Vérification impossible, réessayez plus tard »

### Requirement: Invitation à mettre à jour
Quand une nouvelle version est détectée, l'application SHALL afficher une invitation « Nouvelle version disponible » avec les choix « Mettre à jour » et « Plus tard ». Après « Plus tard », l'invitation MUST ne plus réapparaître pour ce build ; la mise à jour reste accessible depuis les réglages, signalée par une pastille sur l'icône des réglages.

#### Scenario: Report
- **WHEN** l'utilisateur touche « Plus tard » pour le build 60
- **THEN** l'invitation ne s'affiche plus pour le build 60, mais la pastille et la section « Mises à jour » des réglages restent visibles

### Requirement: Installation depuis l'application
« Mettre à jour » SHALL télécharger l'APK annoncé en affichant la progression. Avant toute installation, l'application MUST vérifier que la taille et l'empreinte SHA-256 correspondent au manifeste et que l'URL désigne une release du dépôt de l'application. Elle lance ensuite l'installeur Android, qui remplace l'application en conservant les données. Si Android n'autorise pas encore l'application à installer des applications, elle MUST ouvrir le réglage système correspondant et expliquer qu'il faut l'autoriser puis toucher de nouveau « Mettre à jour ».

#### Scenario: Mise à jour réussie
- **WHEN** l'utilisateur touche « Mettre à jour » et que l'APK téléchargé correspond à l'empreinte annoncée
- **THEN** l'installeur Android s'ouvre et propose de mettre à jour Liquid Compagnon ; après installation, les données sont intactes

#### Scenario: Fichier altéré
- **WHEN** l'empreinte de l'APK téléchargé diffère de celle du manifeste
- **THEN** le fichier est supprimé, rien n'est installé et le message « Téléchargement corrompu, réessayez » s'affiche

#### Scenario: Autorisation manquante
- **WHEN** l'application n'est pas autorisée à installer des applications inconnues
- **THEN** le réglage Android « Installer des applis inconnues » de Liquid Compagnon s'ouvre, avec l'explication de la marche à suivre

### Requirement: Contrôle par l'utilisateur
Les réglages SHALL afficher la version installée et son numéro de build, permettre une vérification manuelle et désactiver la vérification automatique. La vérification MUST ne transmettre aucune donnée personnelle : une simple lecture de fichier public sur github.com.

#### Scenario: Vérification automatique désactivée
- **WHEN** l'utilisateur désactive « Rechercher les mises à jour automatiquement »
- **THEN** aucune requête de mise à jour n'est émise au démarrage ; « Vérifier maintenant » reste disponible
