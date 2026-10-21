//! Reste à dépenser d'un mois (spec spending-allowance) : enveloppe calculée à partir des
//! revenus, des charges et des virements, dépenses déjà faites, rythme et fin de mois projetée.

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;

use crate::{Money, Month};

/// Composantes de l'enveloppe d'un mois.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllowanceSources {
    pub month: Month,
    /// Salaire et revenus ponctuels.
    pub income: Money,
    /// Virements reçus de l'extérieur sur un compte courant.
    pub external_in: Money,
    /// Virements d'une épargne vers un compte courant.
    pub repatriated: Money,
    /// Équivalent mensuel des charges fixes.
    pub fixed_charges: Money,
    /// Virements d'un compte courant (ou de l'extérieur) vers une épargne.
    pub scheduled_savings: Money,
    /// Virements d'un compte courant vers l'extérieur.
    pub external_out: Money,
    /// Dépenses du mois.
    pub spent: Money,
}

/// Rythme des dépenses par rapport au calendrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pace {
    OnTrack,
    Fast,
    Exceeded,
}

/// Position du mois par rapport à aujourd'hui, avec le jour courant pour le mois en cours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Past,
    Current { day: u32 },
    Future,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpendingAllowance {
    pub sources: AllowanceSources,
    /// Ce qui peut être dépensé dans le mois.
    pub envelope: Money,
    /// Enveloppe moins les dépenses du mois (négatif : dépassement).
    pub remaining: Money,
    pub period: Period,
    pub days_in_month: u32,
    /// Jours restants, aujourd'hui compris (0 pour un mois passé).
    pub days_left: u32,
    /// Montant par jour restant, s'il reste de l'argent et des jours.
    pub per_day: Option<Money>,
    /// Rythme actuel des dépenses par jour (mois en cours, dès la première dépense).
    pub daily_spending: Option<Money>,
    /// Solde de fin de mois au rythme actuel (mois en cours, dès la première dépense).
    pub projected_end: Option<Money>,
    pub pace: Pace,
}

impl SpendingAllowance {
    /// Tolérance avant « Rythme élevé », en part de l'enveloppe.
    const FAST_TOLERANCE: Decimal = Decimal::from_parts(2, 0, 0, false, 2);

    pub fn compute(sources: AllowanceSources, today: NaiveDate) -> Self {
        let s = sources;
        let envelope = s.income + s.external_in + s.repatriated
            - s.fixed_charges
            - s.scheduled_savings
            - s.external_out;
        let remaining = envelope - s.spent;
        let month = s.month;
        let days_in_month = month
            .next()
            .first_day()
            .pred_opt()
            .expect("veille du mois suivant")
            .day();
        let current = Month::of(today);
        let period = if month < current {
            Period::Past
        } else if month > current {
            Period::Future
        } else {
            Period::Current { day: today.day() }
        };
        let days_left = match period {
            Period::Past => 0,
            Period::Current { day } => days_in_month - day + 1,
            Period::Future => days_in_month,
        };
        let per_day = (remaining.is_positive() && days_left > 0)
            .then(|| remaining.divided_by(days_left))
            .flatten()
            .map(|m| m.round_to_cents());
        let (daily_spending, projected_end) = match period {
            Period::Current { day } if s.spent.is_positive() => {
                let daily = s.spent.divided_by(day).expect("jour du mois non nul");
                (
                    Some(daily.round_to_cents()),
                    Some((envelope - daily.times(Decimal::from(days_in_month))).round_to_cents()),
                )
            }
            _ => (None, None),
        };
        let pace = if remaining.is_negative() {
            Pace::Exceeded
        } else {
            match period {
                Period::Current { day } if envelope.is_positive() => {
                    let elapsed = Decimal::from(day) / Decimal::from(days_in_month);
                    let expected = envelope.times(elapsed);
                    let tolerance = envelope.times(Self::FAST_TOLERANCE);
                    if s.spent - expected > tolerance {
                        Pace::Fast
                    } else {
                        Pace::OnTrack
                    }
                }
                _ => Pace::OnTrack,
            }
        };
        Self {
            sources,
            envelope,
            remaining,
            period,
            days_in_month,
            days_left,
            per_day,
            daily_spending,
            projected_end,
            pace,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn eur(amount: Decimal) -> Money {
        Money::new(amount)
    }

    fn sources(envelope: Decimal, spent: Decimal, month: Month) -> AllowanceSources {
        AllowanceSources {
            month,
            income: eur(envelope),
            external_in: Money::ZERO,
            repatriated: Money::ZERO,
            fixed_charges: Money::ZERO,
            scheduled_savings: Money::ZERO,
            external_out: Money::ZERO,
            spent: eur(spent),
        }
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    // Spec spending-allowance : « Mois type »
    #[test]
    fn envelope_subtracts_charges_savings_and_external_outflows() {
        let allowance = SpendingAllowance::compute(
            AllowanceSources {
                month: Month::new(2026, 10).unwrap(),
                income: eur(dec!(3340)),
                external_in: Money::ZERO,
                repatriated: eur(dec!(100)),
                fixed_charges: Money::ZERO,
                scheduled_savings: eur(dec!(850)),
                external_out: eur(dec!(2246)),
                spent: Money::ZERO,
            },
            day(2026, 10, 1),
        );
        assert_eq!(allowance.envelope, eur(dec!(344)));
        assert_eq!(allowance.remaining, eur(dec!(344)));
        assert_eq!(allowance.days_left, 31);
        assert_eq!(allowance.pace, Pace::OnTrack);
        assert_eq!(allowance.projected_end, None);
    }

    // Spec spending-allowance : « Rythme élevé » et « Projection de fin de mois »
    #[test]
    fn fast_pace_with_daily_budget_and_projection() {
        let october = Month::new(2026, 10).unwrap();
        let allowance =
            SpendingAllowance::compute(sources(dec!(1500), dec!(1088), october), day(2026, 10, 21));
        assert_eq!(allowance.remaining, eur(dec!(412)));
        assert_eq!(allowance.days_left, 11);
        assert_eq!(allowance.per_day, Some(eur(dec!(37.45))));
        assert_eq!(allowance.pace, Pace::Fast);
        assert_eq!(allowance.daily_spending, Some(eur(dec!(51.81))));
        assert_eq!(allowance.projected_end, Some(eur(dec!(-106.10))));
    }

    // Spec spending-allowance : « Dans le rythme »
    #[test]
    fn on_track_and_exceeded() {
        let september = Month::new(2026, 9).unwrap();
        let on_track =
            SpendingAllowance::compute(sources(dec!(250), dec!(80), september), day(2026, 9, 15));
        assert_eq!(on_track.pace, Pace::OnTrack);
        let exceeded =
            SpendingAllowance::compute(sources(dec!(250), dec!(260), september), day(2026, 9, 15));
        assert_eq!(exceeded.pace, Pace::Exceeded);
        assert_eq!(exceeded.per_day, None);
    }

    #[test]
    fn past_and_future_months() {
        let past = SpendingAllowance::compute(
            sources(dec!(250), dec!(200), Month::new(2026, 9).unwrap()),
            day(2026, 10, 21),
        );
        assert_eq!((past.period, past.days_left), (Period::Past, 0));
        assert_eq!(past.per_day, None);
        let future = SpendingAllowance::compute(
            sources(dec!(250), Decimal::ZERO, Month::new(2026, 11).unwrap()),
            day(2026, 10, 21),
        );
        assert_eq!((future.period, future.days_left), (Period::Future, 30));
    }
}
