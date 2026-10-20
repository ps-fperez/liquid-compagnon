## ADDED Requirements

### Requirement: Export JSON complet
L'application SHALL permettre d'exporter l'intégralité des données (revenus, charges, dépenses, catégories, comptes, positions, fonds PEI, snapshots, réglages de projection) dans un fichier JSON unique contenant un champ `schema_version` et la date d'export. Le hash du PIN et la clé API Twelve Data MUST être exclus de l'export.

#### Scenario: Export réussi
- **WHEN** l'utilisateur choisit « Exporter mes données » dans les réglages
- **THEN** le sélecteur de fichiers Android s'ouvre avec le nom proposé `liquid-compagnon-AAAA-MM-JJ.json` et le fichier est écrit à l'emplacement choisi

#### Scenario: Avertissement de confidentialité
- **WHEN** l'utilisateur lance un export
- **THEN** l'application l'informe que le fichier est en clair et doit être conservé dans un endroit sûr

### Requirement: Import JSON avec validation
L'application SHALL permettre de réimporter un fichier exporté. Avant toute écriture, le fichier MUST être validé (JSON valide, `schema_version` supportée, montants décimaux valides, références cohérentes). L'import SHALL remplacer toutes les données existantes, après confirmation explicite, dans une transaction unique.

#### Scenario: Import valide
- **WHEN** l'utilisateur sélectionne un export valide et confirme le remplacement
- **THEN** toutes les données sont remplacées et le dashboard reflète les données importées

#### Scenario: Fichier invalide
- **WHEN** le fichier n'est pas un export Liquid Compagnon valide
- **THEN** l'import est refusé avec un message indiquant la cause et aucune donnée existante n'est modifiée

#### Scenario: Version plus récente
- **WHEN** la `schema_version` du fichier est supérieure à celle supportée par l'application
- **THEN** l'import est refusé avec le message « Mettez à jour l'application pour importer ce fichier »

### Requirement: Migration des versions antérieures
L'import SHALL accepter les fichiers d'une `schema_version` antérieure en appliquant les migrations de données nécessaires.

#### Scenario: Ancien export
- **WHEN** l'utilisateur importe un fichier de `schema_version` 1 avec une application en version de schéma 2
- **THEN** les données sont migrées puis importées sans perte
