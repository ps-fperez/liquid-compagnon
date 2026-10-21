-- Objectifs d'épargne (comptes suivis liés) et bilans du mois déjà présentés.

CREATE TABLE savings_goals (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    target      TEXT NOT NULL,
    deadline    TEXT,               -- AAAA-MM
    reached_on  TEXT                -- AAAA-MM-JJ, première atteinte
);

-- Ordre des comptes conservé par `position`. Pas de clé étrangère : un compte reste toujours
-- (les comptes sont clôturés, jamais supprimés) et l'import vérifie les références.
CREATE TABLE goal_accounts (
    goal_id     TEXT NOT NULL REFERENCES savings_goals(id) ON DELETE CASCADE,
    account_id  TEXT NOT NULL,
    position    INTEGER NOT NULL,
    PRIMARY KEY (goal_id, account_id)
);

CREATE TABLE monthly_reviews (
    month               TEXT PRIMARY KEY,   -- AAAA-MM, mois résumé
    seen_at             TEXT NOT NULL,
    next_start_balance  TEXT                -- solde de départ saisi pour le mois suivant
);