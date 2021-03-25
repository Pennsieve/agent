use std::result;

use chrono::Duration;
use futures::Future as _Future;
use futures::*;
use hyper::{Client, Uri};
use semver::Version;

use crate::ps::agent::config;
use crate::ps::agent::database::Database;
use crate::ps::agent::Future;
use crate::ps::util::futures::*;

mod error;
pub use self::error::{Error, ErrorKind, Result};

/// Check whether the user is using the latest version of the agent
pub fn check_for_new_version(db: Database) -> Future<()> {
    let db = db.clone();
    db.get_last_version_check()
        .map_err(|e| e.into())
        .into_future()
        .and_then(move |last_check| {
            if should_check_for_new_version(last_check) {
                validate_version_is_current()
                    // Always update that we checked the version, even in the case
                    // failures. The agent should not constantly  check for updates
                    // if e.g. something goes wrong with the S3 bucket
                    .then(move |r| match (db.add_version_check(), r) {
                        (Err(e), Ok(_)) => Err(e.into()),
                        (Ok(_), Err(e)) => Err(e),
                        _ => Ok(()),
                    })
                    .map(|_| ())
                    .into_trait()
            } else {
                Ok(()).into_future().into_trait()
            }
        })
        .into_trait()
}

/// The agent checks for updates at a predefined interval
pub fn should_check_for_new_version(last_check: Option<time::Timespec>) -> bool {
    match last_check {
        Some(last_check) => {
            (last_check
                + Duration::seconds(
                    config::constants::AGENT_LATEST_RELEASE_CHECK_INTERVAL_SECS as i64,
                ))
                < time::now().to_timespec()
        }
        None => true,
    }
}

pub fn validate_version_is_current() -> Future<()> {
    get_latest_version()
        .and_then(move |latest_version| {
            if latest_version > Version::parse(env!("CARGO_PKG_VERSION")).map_err(Into::<Error>::into)? {
                // Print to stderr so that consumers don't see this message in
                // the output of `version` or `config show`.
                eprintln!(
                    "\n\u{01F680} A new version ({}) of the Pennsieve Agent is available.\nVisit https://developer.pennsieve.io/agent to upgrade\n",
                    latest_version
                );
            }
            Ok(())
        })
        .into_trait()
}

/// Get the most recently released version of the agent
pub fn get_latest_version() -> Future<Version> {
    let maybe_uri = [
        config::constants::VERSION_PATH,
        config::constants::VERSION_FILE,
    ]
    .concat()
    .parse();

    maybe_uri
        .into_future()
        .map_err(Into::into)
        .and_then(|uri: Uri| {
            Client::new()
                .get(uri.clone())
                .map_err(Into::into)
                .and_then(move |resp| {
                    if resp.status() == 200 {
                        resp.into_body()
                            .fold(
                                Vec::new(),
                                |mut acc, chunk| -> result::Result<Vec<u8>, hyper::Error> {
                                    acc.extend_from_slice(&*chunk);
                                    Ok(acc)
                                },
                            )
                            .map_err(Into::<Error>::into)
                            .and_then(|v| {
                                String::from_utf8(v).map(String::from).map_err(Into::into)
                            })
                            .into_trait()
                    } else {
                        future::failed(Error::http_error(resp.status(), uri)).into_trait()
                    }
                })
                .map_err(Into::into)
                .and_then(|v| Version::parse(v.trim()).into_future().map_err(Into::into))
        })
        .into_trait()
}

//#[test]
//fn test_get_latest_version() {
//    let mut rt = tokio::runtime::Runtime::new().unwrap();
//    rt.block_on(get_latest_version()).unwrap();
//}

//#[test]
//fn test_should_check_for_new_version() {
//    let last_check = None;
//    assert!(should_check_for_new_version(last_check));
//    let last_check = Some(time::now().to_timespec());
//    assert!(!should_check_for_new_version(last_check));
//    let last_check = Some((time::now() - Duration::hours(4)).to_timespec());
//    assert!(!should_check_for_new_version(last_check));
//    let last_check = Some(
//        (time::now()
//            - Duration::seconds(
//                1 + config::constants::AGENT_LATEST_RELEASE_CHECK_INTERVAL_SECS as i64,
//            ))
//        .to_timespec(),
//    );
//    assert!(should_check_for_new_version(last_check));
//}
