## 1. Domaine

- [x] 1.1 `SavingsGoal` (validation, progression, effort, versé, arrivée sur une projection, atteinte) ; `GoalId` ; tests des scénarios de spec
- [x] 1.2 `MonthlyReview` (mois, vu le, solde de départ) ; tests

## 2. Application et stockage

- [x] 2.1 Ports `GoalRepository`, `ReviewRepository` ; cas d'usage des objectifs (création, modification, suppression, rapport avec projection sur 120 mois, atteinte) ; tests
- [x] 2.2 Cas d'usage du bilan (bilan en attente, rapport d'un mois, recalage du compte courant, historique) ; tests des scénarios de spec
- [x] 2.3 Migration SQLite 004 et dépôts ; `replace_all` ; tests
- [x] 2.4 Sauvegarde v4 (enregistrements, contrôles d'import, migration v3 → v4) ; tests

## 3. Présentation

- [x] 3.1 DTO, mapping et commandes Tauri (objectifs, bilan)
- [x] 3.2 UI : objectifs dans Projection (liste, feuille de création avec effort en direct, anneau, objectif atteint) et anneaux sur l'accueil
- [x] 3.3 UI : bilan plein écran en cartes (défilement, navigation, recalage), ouverture au déverrouillage, historique dans Évolution

## 4. Vérification et livraison

- [x] 4.1 e2e : objectif (création, progression, effort), bilan (ouverture, cartes, recalage) ; check, smoke, e2e au vert
- [x] 4.2 README et recette, `openspec validate`, push, APK, mise à jour depuis l'application