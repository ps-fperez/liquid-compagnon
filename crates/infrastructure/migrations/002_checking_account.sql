-- Compte courant : la nature `checking` réutilise `balance` (solde de référence, signé) et
-- `balance_updated_on` (date de la référence) ; `reference_at` en donne l'instant (RFC 3339).
ALTER TABLE accounts ADD COLUMN reference_at TEXT;

-- Rattachement des mouvements à un compte courant et instant de leur saisie. Pas de clé
-- étrangère : l'intégrité est contrôlée par les cas d'usage et à l'import d'une sauvegarde.
ALTER TABLE expenses ADD COLUMN account_id TEXT;
ALTER TABLE expenses ADD COLUMN recorded_at TEXT;
ALTER TABLE one_off_incomes ADD COLUMN account_id TEXT;
ALTER TABLE one_off_incomes ADD COLUMN recorded_at TEXT;

CREATE INDEX expenses_by_account ON expenses(account_id);
