-- Virements entre comptes, comptes hors patrimoine, prélèvement des charges, référence de solde
-- des livrets et des espèces des comptes-titres.

-- Livrets : l'instant du dernier solde saisi est la fin de sa journée (les virements de ce jour
-- sont réputés déjà inclus). La nature `checking` utilise déjà `reference_at`.
UPDATE accounts
SET reference_at = balance_updated_on || 'T23:59:59+00:00'
WHERE nature = 'savings' AND reference_at IS NULL;

-- Espèces des PEA et portefeuilles crypto : référence à la date de la migration.
ALTER TABLE accounts ADD COLUMN cash_on TEXT;
ALTER TABLE accounts ADD COLUMN cash_at TEXT;
UPDATE accounts
SET cash_on = date('now'), cash_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
WHERE nature = 'securities';

ALTER TABLE recurring_charges ADD COLUMN debit_account_id TEXT;
ALTER TABLE recurring_charges ADD COLUMN debit_day INTEGER;

CREATE TABLE external_accounts (
    id    TEXT PRIMARY KEY,
    name  TEXT NOT NULL
);

-- Un côté est `account` (compte suivi) ou `external` (compte hors patrimoine). Pas de clé
-- étrangère : l'intégrité est contrôlée par les cas d'usage et à l'import d'une sauvegarde.
CREATE TABLE transfers (
    id           TEXT PRIMARY KEY,
    from_kind    TEXT NOT NULL,
    from_id      TEXT NOT NULL,
    to_kind      TEXT NOT NULL,
    to_id        TEXT NOT NULL,
    sent         TEXT NOT NULL,
    received     TEXT NOT NULL,
    label        TEXT,
    schedule     TEXT NOT NULL,      -- once | monthly | yearly
    on_date      TEXT,               -- once : AAAA-MM-JJ
    day          INTEGER,            -- monthly, yearly : jour du mois
    month        INTEGER,            -- yearly : mois (1 à 12)
    from_month   TEXT,               -- monthly : AAAA-MM
    until_month  TEXT,
    from_year    INTEGER,            -- yearly
    until_year   INTEGER,
    recorded_at  TEXT NOT NULL
);
