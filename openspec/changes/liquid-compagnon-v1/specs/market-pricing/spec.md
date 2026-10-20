## ADDED Requirements

### Requirement: Cours crypto via CoinGecko
L'application SHALL récupérer les cours des crypto-actifs en euros via l'API publique CoinGecko, en regroupant tous les actifs détenus dans une seule requête.

#### Scenario: Rafraîchissement groupé
- **WHEN** l'utilisateur détient BTC, ETH et SOL et le dashboard se rafraîchit
- **THEN** une seule requête CoinGecko est émise pour les trois actifs

### Requirement: Cours ETF et actions via Twelve Data
L'application SHALL récupérer les cours des ETF, actions et actifs de référence PEI via Twelve Data, avec une clé API saisie par l'utilisateur dans les réglages. Les cours en devise étrangère MUST être convertis en euros avec le taux de change fourni par la même source.

#### Scenario: Clé absente
- **WHEN** aucune clé Twelve Data n'est configurée et qu'un PEA contient des positions
- **THEN** les positions sont valorisées au dernier cours en cache ou au prix de revient, et un bandeau invite à saisir une clé dans les réglages

#### Scenario: ETF coté en dollars
- **WHEN** l'actif de référence est coté en USD
- **THEN** sa valeur est convertie en euros avant utilisation

### Requirement: Cache des cours et mode hors ligne
Chaque cours obtenu SHALL être enregistré localement avec son horodatage. Un cours de moins de 5 minutes MUST être réutilisé sans appel réseau. En cas d'échec réseau, l'application MUST utiliser le dernier cours en cache et afficher son âge.

#### Scenario: Hors ligne
- **WHEN** le téléphone est hors ligne
- **THEN** le dashboard affiche les valeurs calculées avec les derniers cours connus et l'indication « Cours du 20/10 à 09:42 »

#### Scenario: Cache frais
- **WHEN** l'utilisateur rouvre le dashboard 2 minutes après un rafraîchissement
- **THEN** aucune requête de cotation n'est émise

### Requirement: Rafraîchissement manuel et limites d'API
L'utilisateur SHALL pouvoir forcer un rafraîchissement (tirer pour rafraîchir). L'application MUST respecter les limites des API gratuites en espaçant les requêtes et MUST traiter une réponse HTTP 429 en conservant les cours en cache sans erreur bloquante.

#### Scenario: Limite atteinte
- **WHEN** Twelve Data répond 429
- **THEN** les cours en cache sont conservés et un message discret indique « Limite de requêtes atteinte, réessayez plus tard »

### Requirement: Valorisation PEI par indice de référence
Pour un fonds PEI rattaché à un actif de référence, la valeur de part estimée SHALL être : dernière valeur de part manuelle × (cours actuel de référence ÷ cours de référence à la date de cette valeur manuelle). Le cours de référence à la date d'ancrage MUST être enregistré au moment de la saisie manuelle.

#### Scenario: Estimation par l'indice
- **WHEN** la dernière valeur de part saisie est 25,00 € avec un cours de référence de 100,00 € à cette date, et que la référence cote maintenant 104,00 €
- **THEN** la valeur de part estimée est 26,00 € et elle est marquée « estimée »

### Requirement: Priorité de la saisie manuelle PEI
Une nouvelle valeur de part manuelle SHALL remplacer l'estimation et devenir le nouveau point d'ancrage de l'estimation.

#### Scenario: Recalage
- **WHEN** l'utilisateur saisit la valeur de part réelle de 25,80 € lue sur son espace d'épargne salariale
- **THEN** le fonds est valorisé avec 25,80 € marqué « réel », et les estimations suivantes partent de 25,80 € et du cours de référence du jour

### Requirement: Indication de la provenance des valeurs
Chaque valeur affichée SHALL indiquer sa provenance : « en direct », « en cache » (avec âge), « estimée » ou « saisie manuelle ».

#### Scenario: Valeur estimée
- **WHEN** un fonds PEI est valorisé par son indice
- **THEN** sa ligne affiche le badge « estimée »
