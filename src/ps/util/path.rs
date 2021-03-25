//! Test file functions.

use std::io;
use std::path::{Path, PathBuf};
use tempfile;

/// Create a temporary file path.
pub fn temp(prefix: &str, suffix: &str) -> io::Result<PathBuf> {
    let temp_file = tempfile::Builder::new()
        .prefix(prefix)
        .suffix(suffix)
        .tempfile()?;
    Ok(temp_file.into_temp_path().to_path_buf())
}

/// Like `temp`, but creates the path relative to the directory `dir`.
pub fn temp_in<P>(dir: P, prefix: &str, suffix: &str) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
{
    let temp_file = tempfile::Builder::new()
        .prefix(prefix)
        .suffix(suffix)
        .tempfile_in(dir)?;
    Ok(temp_file.into_temp_path().to_path_buf())
}
