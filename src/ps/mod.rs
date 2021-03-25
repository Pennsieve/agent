//! Pennsieve top-level definitions go here:

use dirs;
use std::path;

pub mod agent;
pub mod proto;
pub mod util;

pub use self::agent::messages;
pub use self::agent::{
    Error, ErrorKind, Future, HostName, OutputFormat, Result, Server, Service, ServiceId,
    WithProps, Worker,
};

/// The home directory for Pennsieve configuration files, databases,
/// and cached data:
#[allow(dead_code)]
pub static PENNSIEVE_HOME: &str = ".pennsieve";

/// Gets the base directory used by the agent for Pennsieve-related assets
/// such configuration files, cached data, etc.
pub fn home_dir() -> Result<Box<path::Path>> {
    match dirs::home_dir() {
        Some(path) => {
            let mut ps_assets = path.clone();
            ps_assets.push(PENNSIEVE_HOME);
            Ok(ps_assets.into())
        }
        None => Err(ErrorKind::MissingAssetDir.into()),
    }
}

/// Gets the location of the Pennsieve agent configuration file.
/// By default, this file is located at "${home_dir()}/config.ini".
pub fn config_file() -> Result<Box<path::Path>> {
    home_dir().and_then(|dir| {
        let mut config_file = dir.to_path_buf();
        config_file.push("config");
        config_file.set_extension("ini");
        Ok(config_file.into())
    })
}

/// Gets the location of the Pennsieve agent database file.
/// By default, this file is located at "${home_dir()}/agent.db".
pub fn database_file() -> Result<Box<path::Path>> {
    home_dir().and_then(|dir| {
        let mut db_file = dir.to_path_buf();
        db_file.push("agent");
        db_file.set_extension("db");
        Ok(db_file.into())
    })
}

/// Gets the Pennsieve agent cache data directory.
/// By default, this file is located at "${home_dir()}/cache".
pub fn cache_dir() -> Result<Box<path::Path>> {
    home_dir().and_then(|dir| {
        let mut cache_dir = dir.to_path_buf();
        cache_dir.push("cache");
        Ok(cache_dir.into())
    })
}
