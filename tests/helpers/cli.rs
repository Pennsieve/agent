//! Test utility functions, mostly for use with the hyper library.

use std::ffi::OsStr;
use std::process::{Command, ExitStatus};

#[allow(dead_code)]
pub fn ps_exec() -> String {
    release_binary!().display().to_string()
}

#[allow(dead_code)]
pub fn run_and_wait<I, S>(args: I) -> (ExitStatus, String)
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let exe = ps_exec();
    println!("{} - {:?}", exe, env!("CARGO_PKG_NAME"));
    let o = Command::new(exe).args(args).output().unwrap();
    let status = o.status;
    let output = String::from_utf8(o.stdout).unwrap();
    let collected = String::from(output.trim());
    (status, collected)
}

#[allow(dead_code)]
pub fn run_and_wait_ignore_status<I, S>(args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let (_, output) = run_and_wait(args);
    output
}
