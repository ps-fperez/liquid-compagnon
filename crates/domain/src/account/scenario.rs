use crate::Rate;

/// Scénario de projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scenario {
    Pessimistic,
    Neutral,
    Optimistic,
}

impl Scenario {
    pub const ALL: [Scenario; 3] = [
        Scenario::Pessimistic,
        Scenario::Neutral,
        Scenario::Optimistic,
    ];
}

/// Rendement annuel d'un compte pour chaque scénario de projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenarioRates {
    pub pessimistic: Rate,
    pub neutral: Rate,
    pub optimistic: Rate,
}

impl ScenarioRates {
    /// Même rendement dans les trois scénarios (cas des livrets réglementés).
    pub const fn uniform(rate: Rate) -> Self {
        Self {
            pessimistic: rate,
            neutral: rate,
            optimistic: rate,
        }
    }

    pub fn rate(&self, scenario: Scenario) -> Rate {
        match scenario {
            Scenario::Pessimistic => self.pessimistic,
            Scenario::Neutral => self.neutral,
            Scenario::Optimistic => self.optimistic,
        }
    }

    pub fn is_uniform(&self, rate: Rate) -> bool {
        *self == Self::uniform(rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn picks_rate_per_scenario() {
        let rates = ScenarioRates {
            pessimistic: Rate::from_percent(dec!(-40)).unwrap(),
            neutral: Rate::from_percent(dec!(5)).unwrap(),
            optimistic: Rate::from_percent(dec!(50)).unwrap(),
        };
        assert_eq!(rates.rate(Scenario::Optimistic).as_percent(), dec!(50));
        assert_eq!(rates.rate(Scenario::Pessimistic).as_percent(), dec!(-40));
    }

    #[test]
    fn uniform_rates_are_detected() {
        let rate = Rate::from_percent(dec!(1.7)).unwrap();
        assert!(ScenarioRates::uniform(rate).is_uniform(rate));
        assert!(!ScenarioRates::uniform(rate).is_uniform(Rate::ZERO));
    }
}
