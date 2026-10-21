## 1. Domaine et application

- [x] 1.1 Calculateur : comptes courants (`with_checking`), épargne programmée, rapatriements, entrées et sorties extérieures d'un compte courant ; capacité d'épargne corrigée ; tests
- [x] 1.2 `SpendingAllowance` : enveloppe, reste, jours restants, par jour, rythme, projection ; tests des scénarios de spec
- [x] 1.3 Synthèse budgétaire avec reste à dépenser et trois premières catégories (budget et tableau de bord) ; tests

## 2. Présentation

- [x] 2.1 DTO et mapping du reste à dépenser
- [x] 2.2 UI : carte et feuille du reste à dépenser ; accueil réorganisé ; ligne dans la synthèse du budget
- [x] 2.3 UI : notification unique en bas avec action « Annuler » (dépense, revenu ponctuel, virement)
- [x] 2.4 UI : animations (montants, jauge, cascades, transitions d'onglets, halo du bouton +), réduction des animations

## 3. Vérification et livraison

- [x] 3.1 e2e : reste à dépenser (enveloppe, rythme, détail), annulation d'une dépense ; check, smoke, e2e au vert
- [x] 3.2 README et recette, `openspec validate`, push, APK, mise à jour depuis l'application