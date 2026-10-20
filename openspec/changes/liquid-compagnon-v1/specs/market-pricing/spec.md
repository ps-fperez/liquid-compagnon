## ADDED Requirements

### Requirement: Cours crypto via CoinGecko
L'application SHALL récupérer les cours des crypto-actifs en euros via l'API publique CoinGecko, en regroupant tous les actifs détenus dans une seule requête.

#### Scenario: Rafraîchissement groupé
- **WHEN** l'utilisateur détient BTC, ETH et SOL et le dashboard se rafraîchit
- **THEN** une seule requête CoinGecko est émise pour les trois actifs

### Requirement: Cours ETF et actions via Yahoo Finance
L'application SHALL récupérer sans clé les cours des ETF, actions et actifs de référence PEI via Yahoo Finance, source proposée par défaut pour les PEA et les fonds PEI, places européennes comprises (symboles `TICKER.PLACE`, ex. `CW8.PA`). Les cours en devise étrangère MUST être convertis en euros avec le taux de change de la même source ; les cotations en pence (`GBp`) MUST être ramenées en livres avant conversion.

#### Scenario: ETF coté à Paris
- **WHEN** le PEA détient CW8 choisi via Yahoo Finance (`CW8.PA`)
- **THEN** sa position est valorisée en direct, sans clé API

#### Scenario: ETF coté à Londres en pence
- **WHEN** un actif est coté 10 960 GBp et que la livre vaut 1,15 €
- **THEN** son cours retenu est 126,04 €

### Requirement: Cours ETF et actions via Twelve Data (facultatif)
L'utilisateur SHALL pouvoir choisir Twelve Data comme source d'un actif, avec une clé API saisie dans les réglages. Les cours en devise étrangère MUST être convertis en euros avec le taux de change fourni par la même source. La recherche MUST signaler les cotations réservées à une offre payante, et la vérification de la clé MUST afficher l'offre associée.

#### Scenario: Clé absente
- **WHEN** aucune clé Twelve Data n'est configurée et qu'un PEA contient des positions Twelve Data
- **THEN** ces positions sont valorisées au dernier cours en cache ou au prix de revient, et un bandeau invite à vérifier la clé dans les réglages

#### Scenario: ETF coté en dollars
- **WHEN** l'actif de référence est coté en USD
- **THEN** sa valeur est convertie en euros avant utilisation

#### Scenario: Vérification de la clé
- **WHEN** l'utilisateur enregistre une clé de l'offre gratuite
- **THEN** l'application affiche « Clé vérifiée : offre basic » avec les crédits consommés du jour

### Requirement: Actifs non couverts par une source
Un actif que sa source ne cote pas (symbole inconnu, place réservée à une offre supérieure) SHALL être signalé nommément, et non ignoré silencieusement ; sa valeur MUST rester affichée au dernier cours en cache ou au prix de revient.

#### Scenario: Place hors de l'offre gratuite Twelve Data
- **WHEN** une position `CW8:Euronext` est demandée à Twelve Data avec une clé de l'offre gratuite
- **THEN** un bandeau indique que CW8:Euronext n'est pas couvert par l'offre Twelve Data et propose de choisir l'actif via Yahoo Finance

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
- **WHEN** Twelve Data ou Yahoo Finance répond 429
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
