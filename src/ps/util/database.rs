//! Test database functions.

use crate::ps::agent::database;
use crate::ps::util;

/// Create a temporary file-backed database
pub fn temp() -> database::Result<database::Database> {
    util::path::temp("ps-temp-database", ".db")
        .map_err(Into::into)
        .and_then(|path| database::Database::new(&database::Source::File(path)))
}
