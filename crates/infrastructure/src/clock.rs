use chrono::{DateTime, Local, NaiveDate, Utc};
use lc_application::ports::Clock;

/// Horloge système ; la date du jour est celle du fuseau du téléphone.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn today(&self) -> NaiveDate {
        Local::now().date_naive()
    }
}
