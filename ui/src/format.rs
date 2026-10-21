//! Formatage à la française et lecture des saisies. Fonctions pures, testées hors navigateur.
//!
//! Montants : « 48 362,15 € » (espace fine insécable entre milliers, virgule décimale, espace
//! insécable avant €). Pourcentages : « 30,0 % ». Dates : « 20/10/2026 ».

use std::str::FromStr;

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use rust_decimal::{Decimal, RoundingStrategy};

/// Séparateur de milliers : espace insécable (l'espace fine U+202F est quasi invisible dans
/// certaines polices mobiles).
const THOUSANDS: char = '\u{00A0}';
/// Espace fine insécable, acceptée à la saisie.
const NARROW_NBSP: char = '\u{202F}';
/// Espace insécable avant une unité.
const NBSP: char = '\u{00A0}';
/// Signe moins typographique.
const MINUS: char = '\u{2212}';

/// Montant masqué en mode discret.
pub const HIDDEN_AMOUNT: &str = "••••\u{00A0}€";

const MONTHS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];
const MONTHS_SHORT: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];

fn round(value: Decimal, decimals: u32) -> Decimal {
    value.round_dp_with_strategy(decimals, RoundingStrategy::MidpointAwayFromZero)
}

/// Groupe les chiffres d'une partie entière par milliers.
fn group_thousands(digits: &str) -> String {
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(THOUSANDS);
        }
        grouped.push(digit);
    }
    grouped
}

/// Nombre avec exactement `decimals` décimales, au format français, sans signe.
fn unsigned_fixed(value: Decimal, decimals: u32) -> String {
    let text = format!("{:.*}", decimals as usize, round(value, decimals).abs());
    match text.split_once('.') {
        Some((integer, fraction)) => format!("{},{fraction}", group_thousands(integer)),
        None => group_thousands(&text),
    }
}

fn sign(value: Decimal, always: bool) -> &'static str {
    if value.is_sign_negative() && !value.is_zero() {
        "\u{2212}"
    } else if always && !value.is_zero() {
        "+"
    } else {
        ""
    }
}

/// « 48 362,15 € », « −300,00 € ».
pub fn amount(value: Decimal) -> String {
    let rounded = round(value, 2);
    format!(
        "{}{}{NBSP}€",
        sign(rounded, false),
        unsigned_fixed(rounded, 2)
    )
}

/// Variation signée : « +1 132,00 € », « −300,00 € ».
pub fn amount_signed(value: Decimal) -> String {
    let rounded = round(value, 2);
    format!(
        "{}{}{NBSP}€",
        sign(rounded, true),
        unsigned_fixed(rounded, 2)
    )
}

/// Montant arrondi à l'euro : « 48 362 € » (axes, totaux secondaires).
pub fn amount_whole(value: Decimal) -> String {
    let rounded = round(value, 0);
    format!(
        "{}{}{NBSP}€",
        sign(rounded, false),
        unsigned_fixed(rounded, 0)
    )
}

/// Euros entiers signés : « +190 € », « −106 € ».
pub fn amount_whole_signed(value: Decimal) -> String {
    let rounded = round(value, 0);
    format!(
        "{}{}{NBSP}€",
        sign(rounded, true),
        unsigned_fixed(rounded, 0)
    )
}

/// Nombre de jours du mois de `value`.
pub fn days_in_month(value: NaiveDate) -> u32 {
    let (year, month) = if value.month() == 12 {
        (value.year() + 1, 1)
    } else {
        (value.year(), value.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, 1)
        .and_then(|first| first.pred_opt())
        .map_or(31, |last| last.day())
}

/// « 21 oct. », « 1er oct. ».
pub fn day_month(value: NaiveDate) -> String {
    let day = if value.day() == 1 {
        "1er".to_string()
    } else {
        value.day().to_string()
    };
    format!("{day} {}", MONTHS_SHORT[value.month0() as usize])
}

/// « de septembre 2026 », « d'octobre 2026 » : élision devant une voyelle (avril, août, octobre).
pub fn of_month(iso_month: &str) -> String {
    elide(&month_label(iso_month))
}

/// « de septembre », « d'octobre ».
pub fn of_month_name(iso_month: &str) -> String {
    elide(&month_name(iso_month))
}

fn elide(word: &str) -> String {
    match word.chars().next() {
        Some('a' | 'e' | 'i' | 'o' | 'u' | 'é' | 'à') => format!("d'{word}"),
        _ => format!("de {word}"),
    }
}

/// Montant compact pour les axes de graphique : « 950 € », « 48 k€ », « 1,2 M€ ».
pub fn amount_compact(value: Decimal) -> String {
    let magnitude = value.abs();
    let (scaled, unit, decimals) = if magnitude >= Decimal::from(1_000_000) {
        (value / Decimal::from(1_000_000), "M€", 1)
    } else if magnitude >= Decimal::from(10_000) {
        (value / Decimal::from(1_000), "k€", 0)
    } else if magnitude >= Decimal::from(1_000) {
        (value / Decimal::from(1_000), "k€", 1)
    } else {
        (value, "€", 0)
    };
    let rounded = round(scaled, decimals);
    let text = unsigned_fixed(rounded, decimals);
    let text = text.strip_suffix(",0").unwrap_or(&text).to_string();
    format!("{}{text}{NBSP}{unit}", sign(rounded, false))
}

/// « 30,0 % ».
pub fn percent(value: Decimal, decimals: u32) -> String {
    let rounded = round(value, decimals);
    format!(
        "{}{}{NBSP}%",
        sign(rounded, false),
        unsigned_fixed(rounded, decimals)
    )
}

/// « +2,4 % ».
pub fn percent_signed(value: Decimal, decimals: u32) -> String {
    let rounded = round(value, decimals);
    format!(
        "{}{}{NBSP}%",
        sign(rounded, true),
        unsigned_fixed(rounded, decimals)
    )
}

/// Quantité sans zéros inutiles : « 0,15 », « 1 234,5678 », « 42 ».
pub fn quantity(value: Decimal) -> String {
    let normalized = value.normalize();
    let decimals = normalized.scale();
    format!(
        "{}{}",
        sign(normalized, false),
        unsigned_fixed(normalized, decimals)
    )
}

/// « 20/10/2026 ».
pub fn date(value: NaiveDate) -> String {
    value.format("%d/%m/%Y").to_string()
}

/// « octobre 2026 » à partir de « 2026-10 ».
pub fn month_label(iso_month: &str) -> String {
    parse_iso_month(iso_month)
        .map(|(year, month)| format!("{} {year}", MONTHS[month as usize - 1]))
        .unwrap_or_else(|| iso_month.to_string())
}

/// « octobre » à partir de « 2026-10 ».
pub fn month_name(iso_month: &str) -> String {
    parse_iso_month(iso_month)
        .map(|(_, month)| MONTHS[month as usize - 1].to_string())
        .unwrap_or_else(|| iso_month.to_string())
}

/// « oct. 26 » à partir de « 2026-10 ».
pub fn month_short(iso_month: &str) -> String {
    parse_iso_month(iso_month)
        .map(|(year, month)| format!("{} {:02}", MONTHS_SHORT[month as usize - 1], year % 100))
        .unwrap_or_else(|| iso_month.to_string())
}

/// « 2026-10 » → « 2026-11 » (décalage de `months` mois, négatif pour reculer).
pub fn shift_month(iso_month: &str, months: i32) -> Option<String> {
    let (year, month) = parse_iso_month(iso_month)?;
    let index = year * 12 + month as i32 - 1 + months;
    Some(format!(
        "{:04}-{:02}",
        index.div_euclid(12),
        index.rem_euclid(12) + 1
    ))
}

/// Nombre de mois de `from` à `to` (« 2026-10 » → « 2028-12 » : 26), si les deux sont valides.
pub fn months_between(from: &str, to: &str) -> Option<i32> {
    let (fy, fm) = parse_iso_month(from)?;
    let (ty, tm) = parse_iso_month(to)?;
    Some((ty * 12 + tm as i32) - (fy * 12 + fm as i32))
}

/// Mois ISO d'une date.
pub fn iso_month(value: NaiveDate) -> String {
    format!("{:04}-{:02}", value.year(), value.month())
}

fn parse_iso_month(raw: &str) -> Option<(i32, u32)> {
    let (year, month) = raw.split_once('-')?;
    let (year, month) = (year.parse().ok()?, month.parse().ok()?);
    (1..=12).contains(&month).then_some((year, month))
}

/// Date et heure locales courtes : « 20/10 à 09:42 ».
pub fn date_time_short(value: DateTime<Utc>) -> String {
    let local = value.naive_utc() - chrono::Duration::minutes(timezone_offset_minutes(value));
    local.format("%d/%m à %H:%M").to_string()
}

/// Décalage du fuseau local (convention JavaScript : minutes à retrancher de l'UTC).
#[cfg(target_arch = "wasm32")]
fn timezone_offset_minutes(value: DateTime<Utc>) -> i64 {
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(
        value.timestamp_millis() as f64
    ));
    date.get_timezone_offset() as i64
}

#[cfg(not(target_arch = "wasm32"))]
fn timezone_offset_minutes(_value: DateTime<Utc>) -> i64 {
    0
}

/// Nettoie une saisie numérique : espaces, symbole €, virgule décimale.
fn normalize_input(raw: &str) -> String {
    raw.trim()
        .trim_end_matches('€')
        .chars()
        .filter(|c| !c.is_whitespace() && *c != THOUSANDS && *c != NARROW_NBSP && *c != NBSP)
        .map(|c| {
            if c == ',' {
                '.'
            } else if c == MINUS {
                '-'
            } else {
                c
            }
        })
        .collect()
}

/// Lit un montant saisi (« 42,5 », « 42.50 », « 1 234,56 € »), au plus 2 décimales.
pub fn parse_amount(raw: &str) -> Result<Decimal, String> {
    let value = parse_decimal(raw).map_err(|_| "Saisissez un montant valide".to_string())?;
    if value.normalize().scale() > 2 {
        return Err("Un montant a au plus 2 décimales".into());
    }
    Ok(value)
}

/// Lit un nombre décimal (quantité, taux, valeur de part) sans limite de décimales.
pub fn parse_decimal(raw: &str) -> Result<Decimal, String> {
    let cleaned = normalize_input(raw);
    if cleaned.is_empty() {
        return Err("Saisissez un nombre".into());
    }
    Decimal::from_str(&cleaned).map_err(|_| "Saisissez un nombre valide".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn plain(text: String) -> String {
        text.replace([THOUSANDS, NBSP], " ")
    }

    #[test]
    fn formats_amounts_the_french_way() {
        assert_eq!(plain(amount(dec!(48362.15))), "48 362,15 €");
        assert_eq!(plain(amount(dec!(42.5))), "42,50 €");
        assert_eq!(plain(amount(dec!(0.005))), "0,01 €");
        assert_eq!(plain(amount(dec!(1234567.891))), "1 234 567,89 €");
        assert_eq!(plain(amount(dec!(-300))), "−300,00 €");
        assert_eq!(plain(amount(dec!(0))), "0,00 €");
        assert_eq!(plain(amount(dec!(-0.001))), "0,00 €");
    }

    #[test]
    fn uses_non_breaking_spaces() {
        assert_eq!(amount(dec!(1234)), "1\u{00A0}234,00\u{00A0}€");
    }

    #[test]
    fn signed_amounts_and_percents() {
        assert_eq!(plain(amount_signed(dec!(1132))), "+1 132,00 €");
        assert_eq!(plain(amount_signed(dec!(-3.2))), "−3,20 €");
        assert_eq!(plain(amount_signed(dec!(0))), "0,00 €");
        assert_eq!(plain(percent(dec!(30.003), 1)), "30,0 %");
        assert_eq!(plain(percent_signed(dec!(2.396), 1)), "+2,4 %");
        assert_eq!(plain(percent_signed(dec!(-3.2), 1)), "−3,2 %");
    }

    #[test]
    fn compact_and_whole_amounts() {
        assert_eq!(plain(amount_compact(dec!(950))), "950 €");
        assert_eq!(plain(amount_compact(dec!(1500))), "1,5 k€");
        assert_eq!(plain(amount_compact(dec!(2000))), "2 k€");
        assert_eq!(plain(amount_compact(dec!(48362))), "48 k€");
        assert_eq!(plain(amount_compact(dec!(1250000))), "1,3 M€");
        assert_eq!(plain(amount_whole(dec!(48362.5))), "48 363 €");
    }

    #[test]
    fn quantities_drop_useless_zeros() {
        assert_eq!(quantity(dec!(0.1500)), "0,15");
        assert_eq!(plain(quantity(dec!(1234.5678))), "1 234,5678");
        assert_eq!(quantity(dec!(42.000)), "42");
    }

    #[test]
    fn dates_and_months() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        assert_eq!(date(day), "20/10/2026");
        assert_eq!(iso_month(day), "2026-10");
        assert_eq!(month_label("2026-10"), "octobre 2026");
        assert_eq!(month_short("2027-02"), "févr. 27");
        assert_eq!(shift_month("2026-12", 1).as_deref(), Some("2027-01"));
        assert_eq!(shift_month("2026-01", -1).as_deref(), Some("2025-12"));
        assert_eq!(month_label("n'importe quoi"), "n'importe quoi");
        let at = day.and_hms_opt(9, 42, 0).unwrap().and_utc();
        assert_eq!(date_time_short(at), "20/10 à 09:42");
    }

    // Spec app-shell-theme : « Saisie avec point »
    #[test]
    fn parses_comma_point_and_spaces() {
        assert_eq!(parse_amount("42.5"), Ok(dec!(42.5)));
        assert_eq!(parse_amount("42,50"), Ok(dec!(42.50)));
        assert_eq!(parse_amount(" 1 234,56 € "), Ok(dec!(1234.56)));
        assert_eq!(parse_amount("1\u{202F}234,5"), Ok(dec!(1234.5)));
        assert_eq!(
            parse_amount("12,345"),
            Err("Un montant a au plus 2 décimales".into())
        );
        assert_eq!(
            parse_amount("abc"),
            Err("Saisissez un montant valide".into())
        );
        assert_eq!(parse_amount(""), Err("Saisissez un montant valide".into()));
        assert_eq!(parse_decimal("0,000123"), Ok(dec!(0.000123)));
        assert_eq!(parse_decimal("−40"), Ok(dec!(-40)));
    }
}
