//! Message types that can be sent between services.

use std::net::SocketAddr;
use std::path::Path;
use std::string::ToString;

use actix::prelude::*;
use actix_net::server as s;
use serde_derive::{Deserialize, Serialize};

use crate::ps::agent::server;

/// Signal that the system is shutting down.
#[derive(Clone, Debug, Message)]
pub struct SystemShutdown;

/// Signal that the agent's status server should start.
#[derive(Clone, Debug)]
pub struct StartStatusServer {
    pub port: u16,
}

impl StartStatusServer {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

impl Message for StartStatusServer {
    type Result = server::Result<Addr<s::Server>>;
}

/// A signal that is sent to initialize a "server" component of the agent.
#[derive(Clone, Debug, Message)]
pub struct ServerStartup {
    pub addr: SocketAddr,
}

impl ServerStartup {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
}

/// A signal that is sent to initialize a "worker" component of the agent.
#[derive(Clone, Debug, Message)]
pub struct WorkerStartup;

/// A message payload containing a request to upload files.
#[derive(Clone, Debug, Message, Serialize, Deserialize)]
pub struct QueueUpload {
    pub dataset: String,
    pub package: Option<String>,
    pub files: Vec<String>,
    pub recursive: Option<bool>, // if omitted, interpret recursive = false
    pub append: Option<bool>,    // if omitted, interpret append = false
}

/// An enum encoding request messages that the websocket can respond to.
#[derive(Clone, Debug, Message, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "message")]
pub enum Request {
    /// Enqueue files for upload.
    QueueUpload { body: QueueUpload },
}

impl Request {
    pub fn queue_upload(
        dataset: String,
        package: Option<String>,
        files: Vec<String>,
        recursive: Option<bool>,
        append: Option<bool>,
    ) -> Self {
        Request::QueueUpload {
            body: QueueUpload {
                dataset,
                package,
                files,
                recursive,
                append,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "message")]
pub enum Response {
    /// A generic error occurred
    Error { context: String },
    /// An error occured while uploading
    UploadError { context: String, import_id: String },
    /// An proxy request was made
    IncomingProxyRequest { url: String },
    /// A file was queued for upload
    FileQueuedForUpload { path: String, import_id: String },
    /// Update when upload progress is made
    UploadProgress {
        import_id: String,
        path: String,
        part_number: usize,
        bytes_sent: u64,
        size: u64,
        percent_done: i32,
        done: bool,
    },
    /// Update when uploads are completed
    UploadComplete { import_id: String },
}

impl Message for Response {
    type Result = server::Result<()>;
}

impl Response {
    pub fn error<T: ToString>(context: T) -> Self {
        Response::Error {
            context: context.to_string(),
        }
    }

    pub fn incoming_proxy_request(url: String) -> Self {
        Response::IncomingProxyRequest { url }
    }

    pub fn file_queued_for_upload<P: AsRef<Path>, I: Into<String>>(path: P, import_id: I) -> Self {
        // Bad unicode characters in the path don't matter as this is just an
        // update:
        let path_as_string: String = path.as_ref().to_string_lossy().to_string();
        Response::FileQueuedForUpload {
            path: path_as_string,
            import_id: import_id.into(),
        }
    }

    pub fn upload_error<T: ToString, I: Into<String>>(context: T, import_id: I) -> Self {
        Response::UploadError {
            context: context.to_string(),
            import_id: import_id.into(),
        }
    }

    pub fn upload_progress<P: AsRef<Path>>(
        import_id: String,
        path: P,
        part_number: usize,
        bytes_sent: u64,
        size: u64,
        percent_done: i32,
        done: bool,
    ) -> Self {
        // Bad unicode characters in the path don't matter as this is just an
        // update:
        let path_as_string: String = path.as_ref().to_string_lossy().to_string();
        Response::UploadProgress {
            import_id,
            path: path_as_string,
            part_number,
            bytes_sent,
            size,
            percent_done,
            done,
        }
    }

    pub fn upload_complete<I: Into<String>>(import_id: I) -> Self {
        Response::UploadComplete {
            import_id: import_id.into(),
        }
    }
}
