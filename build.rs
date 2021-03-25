extern crate rustc_version;

use rustc_version::{version, Version};
use std::io::{self, Write};
use std::process::exit;

/// The minimum required version of rustc needed to build the Pennsieve agent.
const REQUIRED_MIN_VERSION: &str = "1.44.0";

// The Cargo.toml format does not (yet) support a directive
// specifying the minimum required rustc version; we need to check as part of
// the build process:
fn main() {
    let current = version().expect("Couldn't fetch rustc version");
    let required =
        Version::parse(REQUIRED_MIN_VERSION).expect("Couldn't parse required rustc version");
    if (current.major < required.major) || (current.minor < required.minor) {
        let _ = writeln!(
            &mut io::stderr(),
            "This crate requires rustc >= {} (found {})",
            required,
            current
        );
        exit(1);
    }
}
