//! Valeurs par défaut métier. Toutes restent modifiables par l'utilisateur.

use rust_decimal_macros::dec;

use crate::account::{AccountType, SavingsProduct, ScenarioRates};
use crate::{Color, Money, Rate};

/// Catégories de dépenses créées au premier lancement.
pub const CATEGORY_NAMES: [&str; 8] = [
    "Logement",
    "Transport",
    "Alimentation",
    "Abonnements",
    "Santé",
    "Loisirs",
    "Impôts",
    "Autres",
];

/// Taux du Livret A en vigueur depuis le 1er août 2026 (1,7 %).
pub fn livret_a_rate() -> Rate {
    percent(dec!(1.7))
}

/// Taux du LDDS, aligné sur le Livret A (1,7 % depuis le 1er août 2026).
pub fn ldds_rate() -> Rate {
    percent(dec!(1.7))
}

/// Plafond de versement du Livret A (personne physique).
pub fn livret_a_ceiling() -> Money {
    Money::new(dec!(22950))
}

/// Plafond de versement du LDDS.
pub fn ldds_ceiling() -> Money {
    Money::new(dec!(12000))
}

/// Taux et plafond par défaut : réglementés pour le Livret A et le LDDS ; libres (0 %, sans
/// plafond) pour un autre livret ou une assurance-vie.
pub fn savings_terms(product: SavingsProduct) -> (Rate, Option<Money>) {
    match product {
        SavingsProduct::LivretA => (livret_a_rate(), Some(livret_a_ceiling())),
        SavingsProduct::Ldds => (ldds_rate(), Some(ldds_ceiling())),
        SavingsProduct::Livret | SavingsProduct::AssuranceVie => (Rate::ZERO, None),
    }
}

/// Rendements de projection par défaut. Un livret suit son propre taux dans les trois scénarios
/// (`savings_rate`, ou le taux réglementé par défaut s'il n'est pas fourni).
pub fn scenario_rates(account_type: AccountType, savings_rate: Option<Rate>) -> ScenarioRates {
    match account_type {
        AccountType::LivretA => ScenarioRates::uniform(savings_rate.unwrap_or_else(livret_a_rate)),
        AccountType::Ldds => ScenarioRates::uniform(savings_rate.unwrap_or_else(ldds_rate)),
        AccountType::Pea | AccountType::Pei => ScenarioRates {
            pessimistic: percent(dec!(0)),
            neutral: percent(dec!(5)),
            optimistic: percent(dec!(8)),
        },
        AccountType::Crypto => ScenarioRates {
            pessimistic: percent(dec!(-40)),
            neutral: percent(dec!(5)),
            optimistic: percent(dec!(50)),
        },
        // Pas de rendement : le compte courant reste à son solde dans la projection.
        AccountType::Checking => ScenarioRates::uniform(Rate::ZERO),
        AccountType::Livret => ScenarioRates::uniform(savings_rate.unwrap_or(Rate::ZERO)),
        AccountType::AssuranceVie => ScenarioRates {
            pessimistic: percent(dec!(1.5)),
            neutral: percent(dec!(2.5)),
            optimistic: percent(dec!(4)),
        },
    }
}

/// Couleur proposée à la création d'un compte (palette « Purple Electro »).
pub fn color(account_type: AccountType) -> Color {
    let hex = match account_type {
        AccountType::LivretA => "#A855F7",
        AccountType::Ldds => "#C084FC",
        AccountType::Pea => "#3B82F6",
        AccountType::Pei => "#60A5FA",
        AccountType::Crypto => "#7C3AED",
        AccountType::Checking => "#22D3EE",
        AccountType::Livret => "#E879F9",
        AccountType::AssuranceVie => "#38BDF8",
    };
    Color::new(hex).expect("les couleurs par défaut sont valides")
}

fn percent(value: rust_decimal::Decimal) -> Rate {
    Rate::from_percent(value).expect("les taux par défaut sont supérieurs à -100 %")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Label;

    #[test]
    fn regulated_savings_defaults() {
        assert_eq!(livret_a_rate().as_percent(), dec!(1.7));
        assert_eq!(ldds_rate().as_percent(), dec!(1.7));
        assert_eq!(livret_a_ceiling(), Money::new(dec!(22950)));
        assert_eq!(ldds_ceiling(), Money::new(dec!(12000)));
    }

    #[test]
    fn savings_scenarios_follow_account_rate() {
        let custom = Rate::from_percent(dec!(2.4)).unwrap();
        assert!(scenario_rates(AccountType::LivretA, Some(custom)).is_uniform(custom));
        assert!(scenario_rates(AccountType::Ldds, None).is_uniform(ldds_rate()));
    }

    #[test]
    fn market_accounts_have_ordered_scenarios() {
        for account_type in [AccountType::Pea, AccountType::Pei, AccountType::Crypto] {
            let rates = scenario_rates(account_type, None);
            assert!(rates.pessimistic < rates.neutral, "{account_type:?}");
            assert!(rates.neutral < rates.optimistic, "{account_type:?}");
        }
        assert_eq!(
            scenario_rates(AccountType::Crypto, None)
                .pessimistic
                .as_percent(),
            dec!(-40)
        );
    }

    #[test]
    fn default_categories_are_valid_labels() {
        for name in CATEGORY_NAMES {
            assert!(Label::new(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn every_account_type_has_a_color() {
        for account_type in AccountType::ALL {
            assert!(color(account_type).as_str().starts_with('#'));
        }
    }
}
