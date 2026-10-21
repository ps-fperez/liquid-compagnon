## 1. Domaine

- [x] 1.1 Comptes génériques : `SavingsProduct` (Livret, AssuranceVie), plafond facultatif, `AccountType` et intitulés, valeurs par défaut ; tests
- [x] 1.2 `BalanceReference` pour livrets et espèces des comptes-titres, mise à jour de solde ; tests
- [x] 1.3 `Transfer`, `Endpoint`, `Schedule` (occurrences jusqu'à une date, jour ramené au dernier jour du mois), `ExternalAccount` ; tests
- [x] 1.4 `RecurringCharge::debit` et échéances par périodicité ; tests
- [x] 1.5 `Ledger` (dépenses, revenus, prélèvements, virements ; temporalité saisie ou échéance), valorisation de tous les comptes à solde ; tests des scénarios de spec
- [x] 1.6 Projection par flux datés (mensuels, annuels, consommation de la capacité, plafonds) ; tests (projection inchangée après conversion)
- [x] 1.7 Budget : virements avec l'extérieur dans le mois et dans la capacité ; tests

## 2. Application et stockage

- [x] 2.1 Ports `TransferRepository`, `ExternalAccountRepository` ; cas d'usage des virements (création, modification, suppression, validations), des comptes hors patrimoine et du prélèvement des charges ; tests
- [x] 2.2 Grand livre dans comptes, tableau de bord, projection et budget ; conversion des versements programmés (démarrage, import) ; retrait de `monthly_contribution` des saisies ; tests
- [x] 2.3 Migration SQLite 003 et dépôts ; `replace_all` ; tests
- [x] 2.4 Sauvegarde v3 (enregistrements, contrôles d'import, migration v2 → v3) ; tests aller-retour et import v2

## 3. Présentation

- [x] 3.1 DTO, mapping et commandes Tauri (virements, comptes hors patrimoine, prélèvement, mouvements de tous les comptes, nouveaux types)
- [x] 3.2 UI : feuille de virement (aperçu, plafond, montant reçu, fréquences), « Virement » dans le bouton +
- [x] 3.3 UI : fiche de compte (fil des mouvements, virements programmés), section « Hors patrimoine », fiche des livrets et de l'assurance-vie
- [x] 3.4 UI : création des comptes groupée par famille, plafond facultatif, prélèvement des charges, retrait du versement programmé des formulaires

## 4. Vérification et livraison

- [x] 4.1 e2e : mois type (virement mensuel, prélèvement, rapatriement, compte joint, abondement, autre livret) ; check, smoke, e2e au vert
- [x] 4.2 README et recette, `openspec validate`, push, APK, mise à jour depuis l'application
