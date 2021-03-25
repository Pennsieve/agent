#[macro_use]
extern crate pennsieve_macros;

mod helpers;

use std::process::Command;

// Disable this for now
// #[test]
pub fn test_run_ts_proxy() {
    let runner = test_path!("timeseries", "run.sh");
    let status = Command::new(runner).status().unwrap();
    assert!(status.success());
}
