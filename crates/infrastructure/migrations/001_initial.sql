-- Schéma initial de Liquid Compagnon.
-- Montants, taux et quantités : TEXT (décimal exact) ; dates : TEXT ISO 8601.

CREATE TABLE categories (
    id        TEXT PRIMARY KEY,
    name      TEXT NOT NULL,
    archived  INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE salaries (
    id              TEXT PRIMARY KEY,
    effective_from  TEXT NOT NULL,
    net_amount      TEXT NOT NULL
);

CREATE TABLE one_off_incomes (
    id      TEXT PRIMARY KEY,
    label   TEXT NOT NULL,
    amount  TEXT NOT NULL,
    date    TEXT NOT NULL
);

CREATE TABLE recurring_charges (
    id           TEXT PRIMARY KEY,
    label        TEXT NOT NULL,
    amount       TEXT NOT NULL,
    category_id  TEXT NOT NULL REFERENCES categories(id),
    periodicity  TEXT NOT NULL,
    start_month  TEXT NOT NULL,
    end_month    TEXT
);

CREATE TABLE expenses (
    id           TEXT PRIMARY KEY,
    amount       TEXT NOT NULL,
    category_id  TEXT NOT NULL REFERENCES categories(id),
    date         TEXT NOT NULL,
    label        TEXT
);
CREATE INDEX expenses_by_date ON expenses(date);

CREATE TABLE accounts (
    id                    TEXT PRIMARY KEY,
    name                  TEXT NOT NULL,
    color                 TEXT NOT NULL,
    nature                TEXT NOT NULL,      -- savings | securities | employee_savings
    product               TEXT,               -- livret_a | ldds | pea | crypto
    balance               TEXT,
    rate                  TEXT,
    deposit_ceiling       TEXT,
    balance_updated_on    TEXT,
    cash                  TEXT,
    monthly_contribution  TEXT NOT NULL,
    rate_pessimistic      TEXT NOT NULL,
    rate_neutral          TEXT NOT NULL,
    rate_optimistic       TEXT NOT NULL,
    archived              INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE positions (
    account_id    TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    ordinal       INTEGER NOT NULL,
    source        TEXT NOT NULL,
    symbol        TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    currency      TEXT NOT NULL,
    quantity      TEXT NOT NULL,
    unit_cost     TEXT,
    PRIMARY KEY (account_id, source, symbol)
);

CREATE TABLE pei_funds (
    id                TEXT PRIMARY KEY,
    account_id        TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    ordinal           INTEGER NOT NULL,
    name              TEXT NOT NULL,
    units             TEXT NOT NULL,
    ref_source        TEXT,
    ref_symbol        TEXT,
    ref_display_name  TEXT,
    ref_currency      TEXT,
    nav               TEXT NOT NULL,
    nav_date          TEXT NOT NULL,
    reference_price   TEXT
);

CREATE TABLE snapshots (
    id        TEXT PRIMARY KEY,
    month     TEXT NOT NULL,
    taken_on  TEXT NOT NULL,
    kind      TEXT NOT NULL                   -- auto | manual
);

CREATE TABLE snapshot_lines (
    snapshot_id  TEXT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    ordinal      INTEGER NOT NULL,
    account_id   TEXT NOT NULL,
    value        TEXT NOT NULL,
    PRIMARY KEY (snapshot_id, account_id)
);

CREATE TABLE price_cache (
    source        TEXT NOT NULL,
    symbol        TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    currency      TEXT NOT NULL,
    price         TEXT NOT NULL,
    as_of         TEXT NOT NULL,
    PRIMARY KEY (source, symbol)
);

CREATE TABLE settings (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);
