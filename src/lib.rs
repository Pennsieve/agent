// According to https://github.com/chris-morgan/anymap/issues/31
// (dated 2018-09-17), it doesn't seem to be a hard-stopping issue and the core
// devs of rustc are aware.
#![allow(where_clauses_object_safety)]

mod ps;

pub use crate::ps::agent::api;
pub use crate::ps::agent::cache;
pub use crate::ps::agent::cli;
pub use crate::ps::agent::config;
pub use crate::ps::agent::database;
pub use crate::ps::agent::upload;
pub use crate::ps::agent::version;
pub use crate::ps::agent::{server, Agent};
pub use crate::ps::proto;
pub use crate::ps::util;
pub use crate::ps::{
    cache_dir, config_file, database_file, home_dir, messages, Error, ErrorKind, Future, HostName,
    OutputFormat, Result, Server, Service, ServiceId, WithProps, Worker,
};
