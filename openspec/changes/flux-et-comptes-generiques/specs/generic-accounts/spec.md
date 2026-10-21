## ADDED Requirements

### Requirement: Types de comptes génériques
La création d'un compte SHALL proposer :
- **Courant** : compte courant ;
- **Livrets** : Livret A, LDDS, Autre livret ;
- **Placements** : PEA, Assurance-vie ou PER, Portefeuille crypto ;
- **Épargne salariale** : PEE, PEI, PER collectif.

L'« Autre livret » MUST avoir un taux libre et un plafond facultatif. L'« Assurance-vie ou PER » a une valeur saisie à la main, sans plafond, et des rendements par scénario (1,5 %, 2,5 % et 4 % par défaut). Le Livret A et le LDDS gardent leurs taux et plafonds réglementés pré-remplis, modifiables.

#### Scenario: LEP
- **WHEN** l'utilisateur crée un « Autre livret » nommé « LEP » avec un taux de 2,5 % et un plafond de 10 000 €
- **THEN** le compte se comporte comme un livret réglementé : plafond contrôlé, projection au taux saisi

#### Scenario: Livret sans plafond
- **WHEN** l'utilisateur crée un « Autre livret » sans plafond
- **THEN** aucun avertissement de plafond n'est jamais affiché pour ce compte

### Requirement: Conversion des versements programmés
Le versement mensuel programmé d'un compte SHALL être converti une seule fois en virement mensuel le 1er du mois, du même montant. La source est le premier compte courant actif ou, à défaut, un compte hors patrimoine « Compte courant (non suivi) » créé à cette occasion. La conversion MUST s'appliquer aussi aux sauvegardes v1 et v2 importées, et la projection MUST rester identique après la conversion.

#### Scenario: Mise à jour de l'application
- **WHEN** l'utilisateur met à jour l'application avec un Livret A à 200 €/mois programmés et un compte courant
- **THEN** un virement mensuel de 200 € du compte courant vers le Livret A apparaît, et la projection à 12 mois est inchangée

### Requirement: Sauvegarde v3
L'export SHALL contenir les virements, les comptes hors patrimoine, les prélèvements des charges, les nouveaux types de comptes et les références de solde. Le format passe en version 3. Les fichiers v1 et v2 MUST rester importables. Un virement qui désigne un compte absent du fichier MUST faire refuser l'import.

#### Scenario: Aller-retour v3
- **WHEN** l'utilisateur exporte puis réimporte des données avec des virements et un compte hors patrimoine
- **THEN** les soldes, les virements et la projection sont identiques après l'import
