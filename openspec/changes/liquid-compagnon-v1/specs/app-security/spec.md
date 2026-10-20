## ADDED Requirements

### Requirement: Stockage exclusivement local
L'application SHALL stocker toutes les données utilisateur dans une base SQLite située dans le répertoire privé de l'application Android et MUST NOT transmettre de données personnelles (montants, soldes, quantités, revenus) à un service externe. Les seules requêtes réseau autorisées SHALL être les appels de cotation contenant uniquement des identifiants d'actifs.

#### Scenario: Requête de cotation
- **WHEN** l'application interroge CoinGecko ou Twelve Data
- **THEN** la requête ne contient que les identifiants d'actifs, la devise et la clé API éventuelle, sans aucun montant ni quantité

#### Scenario: Sauvegarde cloud Android désactivée
- **WHEN** l'APK est construit
- **THEN** le manifeste Android déclare `android:allowBackup="false"` afin que la base ne soit pas copiée dans la sauvegarde Google

### Requirement: Définition du code PIN au premier lancement
Au premier lancement, l'application SHALL exiger la création d'un code PIN de 4 à 6 chiffres, saisi deux fois, avant tout accès aux données. Le PIN MUST être stocké uniquement sous forme de hash Argon2id salé.

#### Scenario: Création réussie
- **WHEN** l'utilisateur saisit deux fois le même PIN de 6 chiffres
- **THEN** le hash du PIN est enregistré et l'utilisateur accède au dashboard

#### Scenario: Confirmation différente
- **WHEN** la seconde saisie diffère de la première
- **THEN** l'application affiche « Les codes ne correspondent pas » et redemande les deux saisies

### Requirement: Déverrouillage par PIN ou biométrie
À chaque ouverture et après 60 secondes passées en arrière-plan, l'application SHALL afficher l'écran de verrouillage. L'utilisateur MUST pouvoir déverrouiller avec son PIN, ou avec la biométrie Android s'il l'a activée dans les réglages.

#### Scenario: Déverrouillage biométrique
- **WHEN** la biométrie est activée et l'utilisateur présente une empreinte valide
- **THEN** l'application se déverrouille sans saisie du PIN

#### Scenario: Retour rapide au premier plan
- **WHEN** l'application revient au premier plan moins de 60 secondes après être passée en arrière-plan
- **THEN** aucun écran de verrouillage n'est affiché

#### Scenario: Biométrie indisponible
- **WHEN** l'appareil n'a pas de biométrie enregistrée
- **THEN** l'option biométrique est désactivée dans les réglages et seul le PIN est proposé

### Requirement: Protection contre les essais répétés
Après 5 saisies de PIN erronées consécutives, l'application SHALL imposer un délai d'attente doublé à chaque nouvel échec, en partant de 30 secondes.

#### Scenario: Cinquième échec
- **WHEN** l'utilisateur saisit un PIN erroné pour la cinquième fois consécutive
- **THEN** la saisie est bloquée pendant 30 secondes avec un compte à rebours visible

#### Scenario: Réinitialisation du compteur
- **WHEN** l'utilisateur saisit le bon PIN après des échecs
- **THEN** le compteur d'échecs est remis à zéro

### Requirement: Changement du PIN
L'application SHALL permettre de changer le PIN depuis les réglages après saisie du PIN actuel.

#### Scenario: Changement accepté
- **WHEN** l'utilisateur saisit le PIN actuel correct puis deux fois un nouveau PIN
- **THEN** le nouveau hash remplace l'ancien

### Requirement: Masquage dans le sélecteur d'applications
L'application SHALL empêcher les captures d'écran et masquer son contenu dans l'aperçu des applications récentes (`FLAG_SECURE`).

#### Scenario: Aperçu des applications récentes
- **WHEN** l'utilisateur ouvre la vue des applications récentes d'Android
- **THEN** la vignette de Liquid Compagnon n'affiche aucun montant
