//! This module defines agent-specific error types implementation.

use failure::{Backtrace, Context, Fail};
use log::info;
use std::{fmt, io, num, result, string};
use url;

use crate::ps::agent::{api, cache, cli, config, database, server, upload, version};

/// Type alias for handling errors throughout the agent
pub type Result<T> = result::Result<T, Error>;

/// An error that can occur while interacting with the agent
#[derive(Debug)]
pub struct Error {
    ctx: Context<ErrorKind>,
}

impl Error {
    fn _render<T: string::ToString>(&self, context: Option<T>) -> i32 {
        let kind = self.kind();
        let error_code = match kind {
            // user cancellations should be ignored and treated as
            // successful exits
            ErrorKind::UserCancelledError => 0,
            _ => 1,
        };

        if error_code > 0 {
            // Display the message to the user
            match context {
                Some(ctx) => eprintln!("{context}:{kind}", context = ctx.to_string(), kind = kind),
                None => eprintln!("{}", self.kind()),
            };

            // Print the message and backtrace to the log
            info!("ERROR: {}", kind);
            info!(
                "BACKTRACE: {}",
                self.backtrace()
                    .map(|bt| bt.to_string())
                    .unwrap_or_else(|| "None".to_string())
            );
        }

        error_code
    }

    /// This function will display an error to the user and return the
    /// code with which the program should exit.
    pub fn render(&self) -> i32 {
        self._render(None as Option<String>)
    }

    /// This function will display an error to the user and return the
    /// code with which the program should exit, along with a provided
    /// context string.
    pub fn render_with_context<T: string::ToString>(&self, context: T) -> i32 {
        self._render(Some(context))
    }

    /// Return the kind of this error.
    pub fn kind(&self) -> &ErrorKind {
        self.ctx.get_context()
    }

    pub fn malformed_hostname<S: Into<String>>(hostname: S) -> Error {
        ErrorKind::MalformedHostName {
            hostname: hostname.into(),
        }
        .into()
    }

    pub fn unsupported_scheme<S: Into<String>, T: Into<String>>(hostname: S, scheme: T) -> Error {
        ErrorKind::UnsupportedScheme {
            hostname: hostname.into(),
            scheme: scheme.into(),
        }
        .into()
    }

    pub fn output_format<S: Into<String>>(bad_format: S) -> Error {
        ErrorKind::OutputFormat {
            bad_format: bad_format.into(),
        }
        .into()
    }

    pub fn startup(err: io::Error) -> Error {
        ErrorKind::Startup {
            cause: err.to_string(),
        }
        .into()
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&dyn Fail> {
        self.ctx.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.ctx.backtrace()
    }
}

impl Clone for Error {
    fn clone(&self) -> Self {
        self.kind().clone().into()
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        self.kind() == other.kind()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ctx.fmt(f)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Fail)]
pub enum ErrorKind {
    #[fail(display = "malformed hostname {:?}", hostname)]
    MalformedHostName { hostname: String },

    #[fail(display = "invalid scheme: {}", scheme)]
    UnsupportedScheme { hostname: String, scheme: String },

    #[fail(display = "missing asset dir")]
    MissingAssetDir,

    #[fail(display = "no uploads")]
    NoUploads,

    #[fail(display = "unexpected output format: {}", bad_format)]
    OutputFormat { bad_format: String },

    #[fail(display = "error encountered during agent service startup: {}", cause)]
    Startup { cause: String },

    #[fail(display = "unexpected service termination: {}", error)]
    ServiceTermination { error: String },

    #[fail(display = "tokio timer error: {}", error)]
    TokioTimerError { error: String },

    #[fail(display = "timeout")]
    TimeoutError,

    #[fail(display = "url parse error: {}", error)]
    UrlParseError { error: String },

    #[fail(display = "hyper error: {}", error)]
    HyperError { error: String },

    #[fail(display = "protobuf error: {}", error)]
    ProtobufError { error: String },

    // this error means that the user explicitly cancelled an
    // operation (i.e. by hitting "no" in response to our prompt), so
    // it will be ignored and considered a successful exit
    #[fail(display = "cancelled")]
    UserCancelledError,

    #[fail(display = "{}", kind)]
    Pennsieve { kind: pennsieve_rust::ErrorKind },

    #[fail(display = "number parse error: {}", error)]
    ParseIntError { error: String },

    #[fail(display = "io error: {}", error)]
    IoError { error: String },

    #[fail(display = "error setting up logger: {}", error)]
    SetLoggerError { error: String },

    #[fail(display = "from utf8 error: {}", error)]
    FromUtf8Error { error: String },

    #[fail(display = "json error: {}", error)]
    JsonError { error: String },

    #[fail(display = "semver error: {}", error)]
    SemVerError { error: String },

    // links to other modules
    #[fail(display = "api error: {}", kind)]
    ApiError { kind: api::ErrorKind },

    #[fail(display = "database error: {}", kind)]
    DatabaseError { kind: database::ErrorKind },

    #[fail(display = "config error: {}", kind)]
    ConfigError { kind: config::ErrorKind },

    #[fail(display = "upload error: {}", kind)]
    UploadError { kind: upload::ErrorKind },

    #[fail(display = "cache error: {}", kind)]
    CacheError { kind: cache::ErrorKind },

    #[fail(display = "cache error: {}", kind)]
    ServerError { kind: server::ErrorKind },

    #[fail(display = "cli error: {}", kind)]
    CliError { kind: cli::ErrorKind },

    #[fail(display = "version error: {}", kind)]
    VersionError { kind: version::ErrorKind },
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error::from(Context::new(kind))
    }
}
impl From<Context<ErrorKind>> for Error {
    fn from(ctx: Context<ErrorKind>) -> Error {
        Error { ctx }
    }
}

/// map from api errors
impl From<api::ErrorKind> for Error {
    fn from(kind: api::ErrorKind) -> Error {
        let new_kind = match kind {
            api::ErrorKind::UserCancelledError => ErrorKind::UserCancelledError,
            _ => ErrorKind::ApiError { kind },
        };
        Error::from(Context::new(new_kind))
    }
}
impl From<api::Error> for Error {
    fn from(error: api::Error) -> Error {
        error.kind().clone().into()
    }
}

/// map from database errors
impl From<database::ErrorKind> for Error {
    fn from(kind: database::ErrorKind) -> Error {
        Error::from(Context::new(ErrorKind::DatabaseError { kind }))
    }
}
impl From<database::Error> for Error {
    fn from(error: database::Error) -> Error {
        error.kind().clone().into()
    }
}

/// map from cache errors
impl From<cache::ErrorKind> for Error {
    fn from(kind: cache::ErrorKind) -> Error {
        Error::from(Context::new(ErrorKind::CacheError { kind }))
    }
}
impl From<cache::Error> for Error {
    fn from(error: cache::Error) -> Error {
        error.kind().clone().into()
    }
}

/// map from server errors
impl From<server::ErrorKind> for Error {
    fn from(kind: server::ErrorKind) -> Error {
        Error::from(Context::new(ErrorKind::ServerError { kind }))
    }
}
impl From<server::Error> for Error {
    fn from(error: server::Error) -> Error {
        error.kind().clone().into()
    }
}

/// map from cli errors
impl From<cli::ErrorKind> for Error {
    fn from(kind: cli::ErrorKind) -> Error {
        Error::from(Context::new(ErrorKind::CliError { kind }))
    }
}
impl From<cli::Error> for Error {
    fn from(error: cli::Error) -> Error {
        error.kind().clone().into()
    }
}

/// map from config errors
impl From<config::ErrorKind> for Error {
    fn from(kind: config::ErrorKind) -> Error {
        let new_kind = match kind {
            config::ErrorKind::UserCancelledError => ErrorKind::UserCancelledError,
            _ => ErrorKind::ConfigError { kind },
        };
        Error::from(Context::new(new_kind))
    }
}
impl From<config::Error> for Error {
    fn from(error: config::Error) -> Error {
        error.kind().clone().into()
    }
}

/// map from upload errors
impl From<upload::ErrorKind> for Error {
    fn from(kind: upload::ErrorKind) -> Error {
        let new_kind = match kind {
            upload::ErrorKind::UserCancelledError => ErrorKind::UserCancelledError,
            _ => ErrorKind::UploadError { kind },
        };
        Error::from(Context::new(new_kind))
    }
}
impl From<upload::Error> for Error {
    fn from(error: upload::Error) -> Error {
        error.kind().clone().into()
    }
}

/// map from version errors
impl From<version::ErrorKind> for Error {
    fn from(kind: version::ErrorKind) -> Error {
        Error::from(Context::new(ErrorKind::VersionError { kind }))
    }
}
impl From<version::Error> for Error {
    fn from(error: version::Error) -> Error {
        error.kind().clone().into()
    }
}

// map from pennsieve errors
impl From<pennsieve_rust::ErrorKind> for Error {
    fn from(kind: pennsieve_rust::ErrorKind) -> Error {
        Error::from(Context::new(ErrorKind::Pennsieve { kind }))
    }
}
impl From<pennsieve_rust::Error> for Error {
    fn from(error: pennsieve_rust::Error) -> Error {
        error.kind().clone().into()
    }
}

// map from FromUtf8Error
impl From<string::FromUtf8Error> for Error {
    fn from(error: string::FromUtf8Error) -> Error {
        Error::from(Context::new(ErrorKind::FromUtf8Error {
            error: error.to_string(),
        }))
    }
}

/// map from ParseInt errors
impl From<num::ParseIntError> for Error {
    fn from(error: num::ParseIntError) -> Error {
        Error::from(Context::new(ErrorKind::ParseIntError {
            error: error.to_string(),
        }))
    }
}

/// map from IO errors
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::from(Context::new(ErrorKind::IoError {
            error: error.to_string(),
        }))
    }
}

/// map from tokio timer errors
impl From<tokio::timer::Error> for Error {
    fn from(error: tokio::timer::Error) -> Error {
        Error::from(Context::new(ErrorKind::TokioTimerError {
            error: error.to_string(),
        }))
    }
}
impl<T> From<tokio::timer::timeout::Error<T>> for Error {
    fn from(_error: tokio::timer::timeout::Error<T>) -> Error {
        Error::from(Context::new(ErrorKind::TimeoutError))
    }
}

/// map from url parse errors
impl From<url::ParseError> for Error {
    fn from(error: url::ParseError) -> Error {
        Error::from(Context::new(ErrorKind::UrlParseError {
            error: error.to_string(),
        }))
    }
}

/// map from serde_json errors
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Error {
        Error::from(Context::new(ErrorKind::JsonError {
            error: error.to_string(),
        }))
    }
}

/// map from hyper errors
impl From<hyper::Error> for Error {
    fn from(error: hyper::Error) -> Error {
        Error::from(Context::new(ErrorKind::HyperError {
            error: error.to_string(),
        }))
    }
}
impl From<hyper::http::uri::InvalidUri> for Error {
    fn from(error: hyper::http::uri::InvalidUri) -> Error {
        Error::from(Context::new(ErrorKind::HyperError {
            error: error.to_string(),
        }))
    }
}

/// map from semver errors
impl From<semver::SemVerError> for Error {
    fn from(error: semver::SemVerError) -> Error {
        Error::from(Context::new(ErrorKind::SemVerError {
            error: error.to_string(),
        }))
    }
}

/// map from protobuf errors
impl From<protobuf::ProtobufError> for Error {
    fn from(error: protobuf::ProtobufError) -> Error {
        Error::from(Context::new(ErrorKind::ProtobufError {
            error: error.to_string(),
        }))
    }
}

/// map from log errors
impl From<log::SetLoggerError> for Error {
    fn from(error: log::SetLoggerError) -> Error {
        Error::from(Context::new(ErrorKind::SetLoggerError {
            error: error.to_string(),
        }))
    }
}

/// map from futures::Canceled errors
impl From<futures::Canceled> for Error {
    fn from(error: futures::Canceled) -> Error {
        Error::from(Context::new(ErrorKind::ServiceTermination {
            error: error.to_string(),
        }))
    }
}
