//! Date and time related ultity functions live here.

use chrono;
use time::Timespec;

/// RFC3339 formatted timestamp
pub struct RFC3339(String);

impl From<RFC3339> for String {
    fn from(timestamp: RFC3339) -> Self {
        timestamp.0
    }
}

impl AsRef<String> for RFC3339 {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl AsRef<str> for RFC3339 {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

pub fn timespec_to_rfc3339(ts: Timespec) -> RFC3339 {
    let t = chrono::NaiveDateTime::from_timestamp(ts.sec as i64, ts.nsec as u32);
    RFC3339(chrono::DateTime::<chrono::Utc>::from_utc(t, chrono::Utc).to_rfc3339())
}
