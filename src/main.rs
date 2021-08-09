// Needed to suppress the warning message
//
// """
// warning: the trait `anymap::any::CloneToAny` cannot be made into an object
// """
//
// According to https://github.com/chris-morgan/anymap/issues/31
// (dated 2018-09-17), it doesn't seem to be a hard-stopping issue and the core
// devs of rustc are aware.
#![allow(where_clauses_object_safety)]

use std::cmp;
use std::env::{self, current_exe, var};
use std::mem;
use std::path::PathBuf;
use std::process::exit;
use std::sync::atomic;

use actix::prelude::*;
use chrono::Duration;
use clap::value_t;
use futures::{future, Future, IntoFuture};
// Set up human-panic for release build
#[cfg(not(debug_assertions))]
use human_panic::setup_panic;
use lazy_static::lazy_static;
use log::LevelFilter;
use log::*;
use log4rs::append::console::ConsoleAppender;
#[cfg(not(debug_assertions))]
use log4rs::append::rolling_file::{self, RollingFileAppender};
#[cfg(not(debug_assertions))]
use log4rs::config::Logger;
use log4rs::config::{Appender, Config as LogConfig, Root};
use log4rs::encode::pattern::PatternEncoder;
use timer::Timer;

use pennsieve::cache::{self, CachePageCollector};
use pennsieve::cli::upload::{StartMode, StopMode};
use pennsieve::cli::{self, Cli};
use pennsieve::config::constants as c;
use pennsieve::config::{self, Config, Service};
use pennsieve::database::{Database, Source, UserSettings};
use pennsieve::upload::{self, Uploader};
use pennsieve::util::futures::*;
use pennsieve::{self as ps, api, messages, server, Error, ErrorKind};
use pennsieve_macros::{strings, try_future};

///////////////////////////////////////////////////////////////////////////////
//
// --------------------
// Environment variables
// --------------------
//
// - PENNSIEVE_LOG_LEVEL=(debug|info|warn|error)?
//
//   Specify the debug-build console logging level (case-insensitive).
//   If omitted, "warn" will be used.
//
// - DISABLE_MIGRATIONS=true|1|yes
//
//   If given, no attempt to run database migration will occur. This is
//   useful if a user's agent.db SQLite database is in a bad state and we
//   need to manually repair it.
//
///////////////////////////////////////////////////////////////////////////////

// Wrap a Future to indicate main should exit following its execution.
macro_rules! run {
    // Run a future, then exit main().
    ($f:expr; exit) => {
        to_future_trait(($f).map(|_| {
            System::current().stop();
            ()
        }))
    };
    // Run a future, then exit main() with a specific exit code.
    ($f:expr; exit = $code:expr) => {
        to_future_trait(($f).map(|_| {
            System::current().stop_with_code($code);
            ()
        }))
    };
    // Run a future and continue blocking.
    ($f:expr) => {
        to_future_trait(($f).map(|_| ()).map_err(Into::<Error>::into))
    };
}

macro_rules! run_then_exit {
    // Generate a no-op leaf Future indicating main() should exit after it
    // is run.
    ($contents:block) => {
        to_future_trait(future::lazy(move || {
            $contents;
            System::current().stop();
            future::ok::<(), ps::Error>(())
        }))
    };
    // Run a future, then exit main().
    ($f:expr) => { run!($f; exit) };
    // Run a future, then exit main() with a specific exit code.
    ($f:expr; exit = $code:expr) => { run!($f; exit = $code) };
}

// Simplify working with a context CLI object by "Doing the Right Thing", i.e.
// not `unwrap()`ing
//
// For background on this macro technique:
//   https://users.rust-lang.org/t/how-to-use-self-or-any-variable-in-macro-generated-function-body/6264
macro_rules! with_cli {
    ($context:expr, $cli:ident, $run:block) => {
        match $context.cli() {
            Ok($cli) => to_future_trait($run),
            Err(e) => to_future_trait(future::err::<(), _>(e.into())),
        }
    };
}

// Defines the common arguments for an upload command.
// This applies to "append" and "upload".
//
// Note: the functionality to build a subcommand with `clap` must be
// implemented with a macro due to the lifetime specifier requirements of
// `clap::App<'_, '_>`. The `clap` library expects the user to construct
// a chain on subcommands and associated args in a single pass, without
// subdividing the task between builder functions. Most functions on `clap::Arg`
// that accept a string, take a `&str`, which is the cause of the complication.
macro_rules! build_upload_args {
    ($operation:expr, $about:expr, $fallback_dataset:ident) => {
        clap::SubCommand::with_name($operation)
            .about($about)
            .arg(
                clap::Arg::with_name("paths")
                    .value_name("paths")
                    .takes_value(true)
                    .multiple(true)
                    .min_values(1)
                    .required(true)
                    .validator(file_exists)
                    .help(concat!(
                        "Paths of the files to ",
                        $operation,
                        ".\n",
                        "If a single path is provided, it can be a directory from which to ",
                        $operation,
                        " files"
                    )),
            )
            .arg(
                clap::Arg::with_name("dataset")
                    .long("dataset")
                    .value_name("dataset")
                    .takes_value(true)
                    .default_value($fallback_dataset)
                    .validator(id_nonempty)
                    .help(concat!(
                        "The ID or name of the dataset to ",
                        $operation,
                        " to.\n",
                        "Example: --dataset=N:dataset:1234abcd-1234-abcd-efef-a0b1c2d3e4f5 or\n",
                        "         --dataset=\"My dataset\""
                    )),
            )
            .arg(
                clap::Arg::with_name("force")
                    .short("f")
                    .long("force")
                    .help("Bypass the file selection confirmation prompt"),
            )
            .arg(
                clap::Arg::with_name("legacy").long("legacy").help(
                    "Use the legacy S3 uploader instead of the Upload Service for this upload",
                ),
            )
            .arg(
                clap::Arg::with_name("recursive")
                    .short("r")
                    .long("recursive")
                    .help(concat!(
                        "Recursively upload all folders in a directory.\n",
                        "This argument will be ignored if multiple files are specified ",
                        "instead of a single directory"
                    )),
            )
            .arg(
                clap::Arg::with_name("parallelism")
                    .long("parallelism")
                    .value_name("parallelism")
                    .takes_value(true)
                    .hidden(true)
                    .help("Parallelism level; default is the number of CPUs"),
            )
    };
}

macro_rules! append_command {
    ($fallback_dataset:ident) => {
        build_upload_args!(
            "append",
            "Append data to a timeseries package",
            $fallback_dataset
        )
        .arg(
            clap::Arg::with_name("package")
                .long("package")
                .value_name("package")
                .takes_value(true)
                .validator(id_nonempty)
                .help(concat!(
                    "The ID or name of the timeseries package to append to\n",
                    "Example: --package=N:package:1234abcd-1234-abcd-efef-a0b1c2d3e4f5 or\n",
                    "         --package=\"My Timeseries Data\""
                )),
        )
    };
}

macro_rules! upload_command {
    ($fallback_dataset:ident) => {
        build_upload_args!("upload", "Upload files to the Pennsieve platform", $fallback_dataset)
            .arg(
                clap::Arg::with_name("folder")
                    .long("folder")
                    .value_name("folder")
                    .takes_value(true)
                    .validator(id_nonempty)
                    .help(concat!(
                        "The ID or name of the folder to upload to. If it doesn't exist, it will be created\n",
                        "Example: --folder=N:collection:1234abcd-1234-abcd-efef-a0b1c2d3e4f5 or\n",
                        "         --folder=\"My Samples\""
                        )
                    ),
            )
    };
}

lazy_static! {
    /// Set if the agent is running in server mode.
    static ref SERVER_MODE: atomic::AtomicBool = atomic::AtomicBool::new(false);
}

fn parallelism_level(raw_value: Option<&str>) -> usize {
    let max_parallelism: usize = num_cpus::get();
    match raw_value {
        Some(p) => cmp::min(p.parse::<usize>().unwrap(), max_parallelism),
        None => max_parallelism,
    }
}

/// A context for the CLI.
struct Context {
    agent: ps::Agent,
    db: Database,
    config: Option<Config>, // Empty until `get_config()` is called
    api: Option<api::Api>,  // Empty until `get_api()` is called
    output: ps::OutputFormat,
}

impl Context {
    fn new() -> ps::Result<Self> {
        let db = Database::new(&Source::File(ps::database_file()?.to_path_buf()))?;
        Ok(Self {
            agent: ps::Agent::new(),
            db,
            config: None,
            api: None,
            output: Default::default(),
        })
    }

    /// Lazily gets an instance of the Pennsieve API client.
    fn get_api(&mut self) -> ps::Result<api::Api> {
        match self.api {
            // If it exists, return a clone:
            Some(ref api) => Ok(api.clone()),
            None => {
                // Otherwise, create it based on the current profile:
                let config = self.get_config()?;

                let user_profile = self.get_current_profile()?;
                let user_profile = config
                    .api_settings
                    .get_profile(user_profile.clone())
                    .ok_or_else(|| {
                        Into::<Error>::into(api::Error::invalid_user_profile(user_profile))
                    })?;

                // if successful, memoize the result and return that in
                // subsequent calls:
                let api = api::Api::new(&self.db, &config, user_profile.environment);
                self.api = Some(api.clone());
                Ok(api)
            }
        }
    }

    /// Lazily reads the `config.ini` file in the Pennsieve home directory,
    /// returning a typed representation.
    fn get_config(&mut self) -> ps::Result<Config> {
        match self.config {
            // If it exists, return a clone:
            Some(ref config) => Ok(config.clone()),
            // Otherwise, attempt to read it from disk, then parse it:
            None => {
                let config = Config::from_config_file_and_environment()?;
                self.config = Some(config.clone());
                Ok(config)
            }
        }
    }

    /// Return the current output format.
    #[allow(dead_code)]
    fn output(&self) -> &ps::OutputFormat {
        &self.output
    }

    /// Sets the output format.
    fn set_output(&mut self, new_format: ps::OutputFormat) {
        self.output = new_format;
    }

    /// Adds the supplied service to the Pennsieve agent to run when it is
    /// started in server mode.
    fn add_service(&mut self, service: &Service, parallelism: usize) -> ps::Result<()> {
        let config = self.get_config()?;
        let api = self.get_api()?;

        match *service {
            // ----------------------------------------------------------------
            // SERVICE: Reverse proxy
            // ----------------------------------------------------------------
            Service::Proxy(config::ProxyService {
                local_port,
                ref remote_host,
                remote_port,
            }) => {
                let props = ps::server::rp::Props {
                    hostname: remote_host.parse::<ps::HostName>()?,
                    remote_port,
                };
                self.agent
                    .define_server(local_port, props, ps::server::ReverseProxyServer)
                    .map(|_| ())
            }

            // ----------------------------------------------------------------
            // SERVICE: Streaming timeseries data
            // ----------------------------------------------------------------
            Service::TimeSeries(config::TimeSeriesService {
                local_port,
                ref remote_host,
                remote_port,
            }) => {
                let cache_config = config.cache.clone();
                cache::create_page_template(&cache_config)?;

                // Define: cache collector
                {
                    let props = cache::Props {
                        db: self.db.clone(),
                        config: cache_config.clone(),
                    };
                    self.agent.define_worker(props, CachePageCollector)?;
                }

                // Define: streaming timeseries data server
                {
                    let props = ps::server::ts::Props {
                        hostname: remote_host.parse::<ps::HostName>()?,
                        port: remote_port,
                        config: cache_config,
                        db: self.db.clone(),
                    };
                    self.agent
                        .define_server(local_port, props, ps::server::TimeSeriesServer)
                        .map(|_| ())
                }
            }

            // ----------------------------------------------------------------
            // SERVICE: Uploader
            // ----------------------------------------------------------------
            Service::Uploader(_) => {
                let props = upload::Props {
                    api,
                    db: self.db.clone(),
                    parallelism,
                };
                self.agent.define_worker(props, Uploader).map(|_| ())
            }
        }
    }

    fn set_server_mode(mode: bool) {
        SERVER_MODE.store(mode, atomic::Ordering::SeqCst)
    }

    /// Tests if the agent is operating in server mode.
    pub fn in_server_mode() -> bool {
        SERVER_MODE.load(atomic::Ordering::SeqCst)
    }

    /// Note: this function is not intended to be called directly.
    ///
    /// Runs the agent in server mode, passing the Agent instance to a callback
    /// before its `start()` method is invoked.
    fn custom_server_mode<F>(mut self, before_start: F, parallelism: usize) -> ps::Result<()>
    where
        F: Fn(&mut ps::Agent) -> ps::Result<()>,
    {
        let config = self.get_config()?;

        // Given a `config.ini` file, find all services and configure the
        // agent to run them.
        let services = config.get_services().clone();

        if services.is_empty() {
            return Err(Into::<ps::Error>::into(
                config::ErrorKind::NoServicesDefined,
            ));
        }

        for service in services {
            self.add_service(&service, parallelism)?;
        }

        // Apply any mutations to the agent instance before its started:
        before_start(&mut self.agent)?;

        let mut handle = self.agent.setup()?;

        Self::set_server_mode(true);

        install_sigint_handler(System::current());

        handle.run().expect("start in server mode");

        Self::set_server_mode(false);

        Ok(())
    }

    /// Starts the agent in server mode.
    fn start_server_mode(mut self, parallelism: usize) -> ps::Result<()> {
        let config = self.get_config()?;

        self.custom_server_mode(
            |ref mut agent| {
                // Set the status server port:
                agent.set_status_port(config.status_server_port);

                Ok(())
            },
            parallelism,
        )
    }

    /// Sets up logging.
    fn setup_logging() -> ps::Result<()> {
        // Get log level from the environment, falling back to the provided default
        // PENNSIEVE_LOG_LEVEL is preferred for compatibility with the Python client,
        // but LOGLEVEL is also supported.
        fn get_log_level(default_level: LevelFilter) -> LevelFilter {
            if let Ok(loglevel) = env::var("PENNSIEVE_LOG_LEVEL").or_else(|_| env::var("LOGLEVEL"))
            {
                match loglevel.to_lowercase().as_str() {
                    "debug" => LevelFilter::Debug,
                    "info" => LevelFilter::Info,
                    "warn" => LevelFilter::Warn,
                    "error" => LevelFilter::Error,
                    level => {
                        eprintln!("not a valid logging level: {}", level);
                        default_level
                    }
                }
            } else {
                default_level
            }
        }

        // === DEBUG BUILD ====================================================
        #[cfg(debug_assertions)]
        let config: LogConfig = {
            let stdout = ConsoleAppender::builder()
                .encoder(Box::new(PatternEncoder::new(
                    config::constants::AGENT_LOG_FORMAT,
                )))
                .build();

            LogConfig::builder()
                .appender(Appender::builder().build("stdout", Box::new(stdout)))
                .build(
                    Root::builder()
                        .appender("stdout")
                        .build(get_log_level(LevelFilter::Info)),
                )
                .expect("ps:main:context:logging:init ~ couldn't initialize the console logger")
        };

        // === RELEASE BUILD ==================================================
        #[cfg(not(debug_assertions))]
        let config: LogConfig = {
            // Build the log output path:
            let mut log_path = PathBuf::from(ps::home_dir().expect(
                "ps:main:context:logging:init ~ couldn't get the Pennsieve asset directory",
            ));
            log_path.push("out");
            log_path.set_extension("log");

            let trigger =
                rolling_file::policy::compound::trigger::size::SizeTrigger::new(10 * 1000 * 1000); // ~ 10 MB
            let roller =
                rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller::builder()
                    .base(1)
                    .build(&format!("{}{{}}", log_path.to_string_lossy()), 5)
                    .expect("ps:main:context:logging:init ~ couldn't initialize logger");
            let policy = rolling_file::policy::compound::CompoundPolicy::new(
                Box::new(trigger),
                Box::new(roller),
            );

            let file = RollingFileAppender::builder()
                .encoder(Box::new(PatternEncoder::new(
                    config::constants::AGENT_LOG_FORMAT,
                )))
                .build(&log_path, Box::new(policy))
                .expect("ps:main:context:logging:init ~ couldn't build the file logger");
            let stdout = ConsoleAppender::builder()
                .encoder(Box::new(PatternEncoder::new(
                    config::constants::AGENT_LOG_FORMAT,
                )))
                .build();

            LogConfig::builder()
                .appender(Appender::builder().build("rolling_file", Box::new(file)))
                .appender(Appender::builder().build("stdout", Box::new(stdout)))
                .logger(
                    Logger::builder()
                        .appender("rolling_file")
                        .additive(false)
                        .build("pennsieve::ps", get_log_level(LevelFilter::Info)),
                )
                .logger(
                    Logger::builder()
                        .appender("stdout")
                        .build("pennsieve", get_log_level(LevelFilter::Warn)),
                )
                .build(
                    Root::builder()
                        .appender("stdout")
                        .build(get_log_level(LevelFilter::Warn)),
                )
                .expect("ps:main:context:logging:init ~ couldn't build the console logger")
        };

        log4rs::init_config(config).map(|_| ()).map_err(Into::into)
    }

    // NOTE:
    // Sometimes it is necessary to defer the reading of the `config.ini` file
    // until it is actually used by an action. This is needed specifically for the
    // case of calling the Pennsieve agent "config" command, which generates a
    // sample configuration file. It should succeed even if a valid configuration
    // file cannot be found (as it is needed to bootstrap a new configuration).
    /// Create a new CLI instance.
    fn cli(&mut self) -> ps::Result<Cli> {
        let api = self.get_api()?;
        let config = self.get_config()?;
        Ok(Cli::new(&self.db, &api, self.output, &config.api_settings))
    }

    /// Toggles file uploading watch mode.
    ///
    /// In this mode, the file upload progress indicator will be rendered on
    /// the CLI.  If the Pennsieve agent is not running in server mode, it is
    /// started before upload watching occurs.
    fn uploading(
        self,
        _cli: Cli,
        start_mode: StartMode,
        stop_mode: StopMode,
        parallelism: usize,
    ) -> ps::Future<()> {
        let active_uploads = try_future!(self.db.get_active_uploads());

        // If there are no active uploads and the start mode doesn't allow for
        // an empty upload queue, don't try to start the agent in
        // server mode, or really do anything; just return immediately.
        if start_mode.no_empty_queue() && active_uploads.is_empty() {
            // Returning an error causes the event loop to exit early:
            return future::err(ErrorKind::NoUploads.into()).into_trait();
        }

        let watcher: cli::UploadWatcher = Default::default();

        // If in server mode, an upload worker is already running and the
        // upload worker(s) will pick up any file changes:
        if Self::in_server_mode() {
            watcher.watch().into_trait()
        } else {
            let db = self.db.clone();
            let output = self.output;

            // The agent is not running. Start the server alongside the an
            // upload watcher worker.
            self.custom_server_mode(
                |ref mut agent| {
                    let db = db.clone();
                    let props = cli::upload::Props {
                        db,
                        output,
                        interval_ms: config::constants::UPLOAD_PROGRESS_REFRESH_INTERVAL_MS,
                        parallelism,
                        start_mode,
                        stop_mode,
                    };

                    // If a port is given, use that to set the status port:
                    if let StartMode::AllowEmptyQueue(port) = start_mode {
                        if let Some(status_port) = port {
                            agent.set_status_port(status_port);
                        }
                    }

                    // Suppress extraneous output when running a second instance:
                    agent.quiet();

                    // Set up the upload worker to run:
                    agent.define_worker(props, watcher).map(|_| ())
                },
                parallelism,
            )
            .into_future()
            .into_trait()
        }
    }

    /// Initialize a new profile.
    fn with_new_api_profile<S: Into<String>>(mut self, profile: S) -> ps::Future<Self> {
        let profile: String = profile.into();
        let config = try_future!(self.get_config());
        config
            .api_settings
            .get_profile(profile.clone())
            .ok_or_else(|| api::Error::invalid_user_profile(profile).into())
            .into_future()
            .and_then(move |new_profile| {
                let api = api::Api::new(&self.db, &config, new_profile.environment);
                api.login_with_profile(new_profile.profile).map(|_| Self {
                    agent: self.agent,
                    db: self.db,
                    config: self.config,
                    api: Some(api),
                    output: self.output,
                })
            })
            .into_trait()
    }

    /// Gets the currently set profile.
    fn get_current_profile(&mut self) -> ps::Result<String> {
        let config = self.get_config()?;
        if config.environment_override {
            Ok(c::ENVIRONMENT_OVERRIDE_PROFILE.to_string())
        } else {
            let default_profile = config.api_settings.default_profile().profile;

            if let Some(user) = self.db.get_user()? {
                if config.api_settings.profile_names().contains(&user.profile) {
                    Ok(user.profile)
                } else {
                    error!(
                        "User's current profile was not found: {}. Falling back to default: {}.",
                        user.profile, default_profile
                    );
                    self.db.delete_user()?;
                    Ok(default_profile)
                }
            } else {
                Ok(default_profile)
            }
        }
    }

    /// Gets the current user's settings.
    fn get_user_settings(&mut self) -> ps::Result<UserSettings> {
        let user = self
            .db
            .get_user()?
            .ok_or_else(|| Into::<Error>::into(api::ErrorKind::NoUserError))?;
        let profile = self.get_current_profile()?;
        self.db
            .get_or_create_user_settings(&user.id, &profile)
            .map_err(Into::into)
    }
}

/// Sets up cross-platform SIGINT (ctrl+c) handling for the Pennsieve agent
/// when running in server mode.
///
/// This function returns a future that will not resolve until a SIGINT
/// signal is received.
fn install_sigint_handler(system: System) {
    ctrlc::set_handler(move || {
        info!("received SIGINT");
        #[cfg(not(debug_assertions))]
        println!("Shutting down");

        // Shutdown the actix system:
        system
            .registry()
            .get::<server::StatusServer>()
            .do_send(messages::SystemShutdown);

        // Kick off a watchdog timer to kill the process if shutdown takes
        // too long.
        let timer = Timer::new();
        timer
            .schedule_with_delay(
                Duration::seconds(config::constants::AGENT_MAX_SHUTDOWN_TIMEOUT_SECS as i64),
                move || {
                    info!("shutdown timeout exceeded");
                    exit(0);
                },
            )
            .ignore();
        mem::forget(timer);
    })
    .expect("couldn't install SIGINT handler");
}

/// Function to validate whether a given profile_name exists.
fn profile_exists<S: Into<String>>(profile_name: S) -> Result<(), String> {
    let profile_name: String = profile_name.into();
    Config::from_config_file_and_environment()
        .or_else(|e| Err(format!("error building the configuration: {}", e)))
        .and_then(|config| {
            if config.api_settings.contains_profile(profile_name.clone()) {
                Ok(())
            } else {
                Err(format!(
                    "Invalid profile: {}. These are valid profiles: {}",
                    profile_name,
                    config.api_settings.profile_names().join(", ")
                ))
            }
        })
}

/// Function to validate if an identifier is non-empty.
fn id_nonempty<S: Into<String>>(id: S) -> Result<(), String> {
    let id = id.into();
    if id.trim().is_empty() {
        Err("an ID is required".into())
    } else {
        Ok(())
    }
}

/// Function to validate if a file exists.
fn file_exists<S: Into<String>>(filepath: S) -> Result<(), String> {
    let filepath = filepath.into();
    if !PathBuf::from(filepath.clone()).as_path().exists() {
        Err(format!("file not found: {:?}", filepath))
    } else {
        Ok(())
    }
}

/// Function to validate if a given argument is numeric.
fn is_numeric<S: Into<String>>(argument: S) -> Result<(), String> {
    let argument = argument.into();
    if argument.parse::<usize>().is_ok() {
        Ok(())
    } else {
        Err(format!("received non-numeric value: {}", argument))
    }
}

#[allow(clippy::cyclomatic_complexity)]
fn main() {
    // First, initialize all logging:
    Context::setup_logging().expect("couldn't initialize the logger");

    // Set up human-panic for release build
    #[cfg(not(debug_assertions))]
    setup_panic!();

    let mut context = Context::new().unwrap_or_else(|e| {
        eprintln!("Error creating command line context:");
        print!("    ");
        eprintln!("{}", e.to_string());
        exit(1)
    });

    // Reads the ID from the persistent dataset file, returning it if it exists.
    let user_settings = context.get_user_settings().unwrap_or_default();

    let fallback_dataset: &str = user_settings
        .use_dataset_id
        .as_ref()
        .map_or("", String::as_str);

    let mut app = clap::App::new(env!("CARGO_PKG_NAME"))
                .version(env!("CARGO_PKG_VERSION"))
                .author(env!("CARGO_PKG_AUTHORS"))
                .about("The official Pennsieve client")
                .setting(clap::AppSettings::UnifiedHelpMessage)
        .arg(clap::Arg::with_name("output")
             .short("O")
             .long("output")
             .takes_value(true)
             .global(true)
             .possible_value("simple")
             .possible_value("rich")
             .default_value("rich")
             //.possible_value("json")
             .help("Sets the output format"))
        .subcommand(append_command!(fallback_dataset))
        .subcommand(clap::SubCommand::with_name("config")
                    .about("Configure the Pennsieve Agent")
                    .long_about("Configure the Pennsieve Agent")
                    .subcommand(clap::SubCommand::with_name("show")
                                .about("Show a configuration value")
                                .arg(clap::Arg::with_name("key")
                                     .value_name("key")
                                     .takes_value(true)
                                     .required(false)))
                    .subcommand(clap::SubCommand::with_name("wizard")
                                .about("Create a new config file using the configuration wizard."))
                    .subcommand(clap::SubCommand::with_name("example")
                                .about("Print a template configuration file to standard output"))
                    .subcommand(clap::SubCommand::with_name("schema-version")
                                .about("Get/set the agent.db SQLite database schema version (user_version)")
                                .arg(clap::Arg::with_name("version")
                                    .value_name("version")
                                    .required(false)
                                    .hidden(cfg!(not(debug_assertions)))
                                    .takes_value(true))))
        .subcommand(clap::SubCommand::with_name("create-collection")
                    .about("Create a new collection")
                    .long_about("Create a new collection.")
                    .arg(clap::Arg::with_name("name")
                         .value_name("name")
                         .takes_value(true)
                         .required(true)
                         .index(1)
                         .help("A collection name"))
                    .arg(clap::Arg::with_name("dataset")
                         .long("dataset")
                         .value_name("dataset")
                         .takes_value(true)
                         .default_value(fallback_dataset)
                         .validator(id_nonempty)
                         .help(concat!(
                                 "A dataset ID or name.\n",
                                 "Example: --dataset=N:dataset:1234abcd-1234-abcd-efef-a0b1c2d3e4f5 or\n",
                                 "         --dataset=\"My Samples\""
                            ))))
        .subcommand(clap::SubCommand::with_name("clear")
                    .about("Clear the current working dataset")
                    .long_about("Clear the current working dataset.")
                    .display_order(2))
        .subcommand(clap::SubCommand::with_name("collaborators")
                    .about("List the collaborators of a dataset")
                    .long_about("List the collaborators of a dataset.")
                    .arg(clap::Arg::with_name("dataset")
                         .long("dataset")
                         .value_name("dataset")
                         .takes_value(true)
                         .global(true)
                         .default_value(fallback_dataset)
                         .validator(id_nonempty)
                         .help(concat!(
                                 "A dataset ID or name.\n",
                                 "Example: --dataset=N:dataset:1234abcd-1234-abcd-efef-a0b1c2d3e4f5 or\n",
                                 "         --dataset=\"My Samples\""
                            )))
                    .subcommand(clap::SubCommand::with_name("users")
                                .about("List all user collaborators.")
                                .long_about("List all users and their permission level on the given dataset."))
                    .subcommand(clap::SubCommand::with_name("teams")
                                .about("List all team collaborators.")
                                .long_about("List all teams and their permission level on the given dataset."))
                    .subcommand(clap::SubCommand::with_name("organization")
                                .about("Show the organization role.")
                                .long_about("Show the role of the user's preferred organization on the given dataset.")))
        .subcommand(clap::SubCommand::with_name("datasets")
                    .about("List your datasets")
                    .long_about("List your datasets.")
                    .alias("ds"))
        .subcommand(clap::SubCommand::with_name("create-dataset")
                    .about("Create a new dataset")
                    .long_about("Create a new dataset.")
                    .display_order(3)
                    .arg(clap::Arg::with_name("name")
                         .value_name("name")
                         .takes_value(true)
                         .required(true)
                         .index(1)
                         .help("A dataset name"))
                    .arg(clap::Arg::with_name("description")
                         .long("description")
                         .required(false)
                         .index(2)
                         .help("An optional description")))
        .subcommand(clap::SubCommand::with_name("ls")
                    .about("Provides navigation around datasets and collections")
                    .long_about("Provides navigation around datasets and collections.")
                    .arg(clap::Arg::with_name("dataset")
                         .long("dataset")
                         .value_name("dataset")
                         .takes_value(true)
                         .default_value(fallback_dataset)
                         .validator(id_nonempty)
                         .help(concat!(
                                 "A dataset ID or name.\n",
                                 "Example: --dataset=N:dataset:1234abcd-1234-abcd-efef-a0b1c2d3e4f5 or\n",
                                 "         --dataset=\"My Samples\""
                            )))
                    .arg(clap::Arg::with_name("collection")
                         .long("collection")
                         .value_name("collection")
                         .takes_value(true)
                         .help("A package ID.\nExample: --collection=N:collection:1234abcd-1234-abcd-efef-a0b1c2d3e4f5")))

        .subcommand(clap::SubCommand::with_name("move")
                    .alias("mv")
                    .about("Move packages and collections")
                    .long_about("Move packages and collections")
                    .arg(clap::Arg::with_name("source")
                         .long("source")
                         .value_name("source")
                         .required(true)
                         .index(1)
                         .help("The package or collection to move")
                    )
                    .arg(clap::Arg::with_name("destination")
                         .long("destination")
                         .value_name("destination")
                         .required(false)
                         .index(2)
                         .help("The destination collection. If not provided, the source will be moved to the root of the dataset")))

        .subcommand(clap::SubCommand::with_name("members")
                    .about("List the members that are part of the organization you belong to")
                    .long_about("List the members that are part of the organization you belong to."))
        .subcommand(clap::SubCommand::with_name("organizations")
                    .about("List the organizations you belong to")
                    .long_about("List the organizations you belong to.")
                    .alias("orgs"))
        .subcommand(clap::SubCommand::with_name("rename")
                    .about("Rename a package or dataset")
                    .long_about("Rename a package or dataset.")
                    .arg(clap::Arg::with_name("package_or_dataset_id")
                         .value_name("id")
                         .takes_value(true)
                         .required(true)
                         .index(1)
                         .help("A package ID, or dataset ID, or dataset name"))
                    .arg(clap::Arg::with_name("name")
                         .value_name("name")
                         .takes_value(true)
                         .required(true)
                         .index(2)
                         .help("A new name")))
        .subcommand(clap::SubCommand::with_name("server")
                    .about("Start the Pennsieve agent in server mode")
                    .long_about("Start the Pennsieve agent in server mode.")
                    .arg(
                         clap::Arg::with_name("parallelism")
                         .long("parallelism")
                         .value_name("parallelism")
                         .takes_value(true)
                         .hidden(true)
                         .help("Parallelism level; default is the number of CPUs")))
        .subcommand(clap::SubCommand::with_name("teams")
                    .about("List the teams that are part of the organization you belong to")
                    .long_about("List the teams that are part of the organization you belong to."))
        .subcommand(upload_command!(fallback_dataset))
        .subcommand(clap::SubCommand::with_name("profile")
                    .about("Manage profiles")
                    .long_about("Manage profiles.")
                    .subcommand(clap::SubCommand::with_name("show")
                                .about("Show current active profile"))
                    .subcommand(clap::SubCommand::with_name("switch")
                                .about("Switch to a new profile")
                                .arg(clap::Arg::with_name("profile")
                                     .value_name("profile")
                                     .required(true)
                                     .takes_value(true)
                                     .validator(profile_exists)
                                     .index(1)
                                     .help("The target profile")))
                    .subcommand(clap::SubCommand::with_name("create")
                                .about("Create a new profile"))
                    .subcommand(clap::SubCommand::with_name("delete")
                                .about("Delete a profile")
                                .arg(clap::Arg::with_name("profile")
                                     .value_name("profile")
                                     .required(true)
                                     .takes_value(true)
                                     .validator(profile_exists)
                                     .index(1)
                                     .help("The profile to be deleted")))
                    .subcommand(clap::SubCommand::with_name("set-default")
                                .about("Set a profile as the default")
                                .arg(clap::Arg::with_name("profile")
                                     .value_name("profile")
                                     .required(true)
                                     .takes_value(true)
                                     .validator(profile_exists)
                                     .index(1)
                                     .help("The profile to use as new default")))
                    .subcommand(clap::SubCommand::with_name("list")
                                .about("Display a list of available profiles")))
        .subcommand(clap::SubCommand::with_name("upload-status")
                    .about("Check the upload status of files")
                    .long_about("Check the upload status of files; resume and cancel uploads.")
                    .arg(clap::Arg::with_name("cancel")
                            .long("cancel")
                            .value_name("ID")
                            .multiple(true)
                            .takes_value(true)
                            .help("Cancel an upload by its ID"))
                    .arg(clap::Arg::with_name("cancel_pending")
                            .long("cancel-pending")
                            .value_name("cancel-pending")
                            .takes_value(false)
                            .help("Cancel all pending uploads"))
                    .arg(clap::Arg::with_name("cancel_all")
                            .long("cancel-all")
                            .value_name("cancel-all")
                            .takes_value(false)
                            .help("Cancel all uploads, regardless of status"))
                    .arg(clap::Arg::with_name("retry")
                            .long("retry")
                            .value_name("ID")
                            .multiple(true)
                            .takes_value(true)
                            .help("Retry an upload by ID"))
                    .arg(clap::Arg::with_name("resume")
                         .long("resume")
                         .help("Resume queued uploads"))
                    .arg(clap::Arg::with_name("failed")
                         .long("failed")
                         .help("View failed uploads"))
                    .arg(clap::Arg::with_name("completed")
                         .long("completed")
                         .value_name("completed")
                         .validator(is_numeric)
                         .takes_value(true)
                         .help("View last N completed uploads"))
                    .arg(clap::Arg::with_name("listen")
                         .long("listen")
                         .takes_value(false)
                         .help(concat!("Listens for incoming uploads and does not terminate upon upload completion.\n",
                                       "This mode is useful for scripting the upload behavior of the Pennsieve command line tool \n",
                                       "by sending files to be uploaded over a websocket.")))
                    .arg(clap::Arg::with_name("port")
                         .long("port")
                         .takes_value(true)
                         .requires("listen")
                         .help("The port to listen on"))
                    .arg(clap::Arg::with_name("parallelism")
                         .long("parallelism")
                         .value_name("parallelism")
                         .takes_value(true)
                         .hidden(true)
                         .help("Parallelism level; default is the number of CPUs")))
        .subcommand(clap::SubCommand::with_name("upload-verify")
                    .about("Verify the integrity of files on the platform")
                    .long_about(concat!("Verify that local files match uploaded files in the platform.\n",
                                        "If a local filepath is not specified, the local file that was ",
                                        "originally uploaded will be used to verify."))
                    .arg(clap::Arg::with_name("id")
                            .short("i")
                            .long("upload-id")
                            .value_name("ID")
                            .takes_value(true)
                            .validator(is_numeric)
                            .required(true)
                            .help("The ID of the uploaded file, as it appears in `upload-status --completed N`"))
                    .arg(clap::Arg::with_name("path")
                            .short("f")
                            .long("path")
                            .value_name("PATH")
                            .takes_value(true)
                            .validator(file_exists)
                            .help("An optional local file to check against the uploaded file.")))
        .subcommand(clap::SubCommand::with_name("use")
                    .about("Set your current working dataset")
                    .long_about("Set your current working dataset.")
                    .display_order(1)
                    .arg(clap::Arg::with_name("dataset")
                         .value_name("dataset")
                         .takes_value(true)
                         .index(1)
                         .help("A dataset's ID or name. If omitted, the current dataset will be printed.")))
        .subcommand(clap::SubCommand::with_name("version")
            .about("Print the current version number")
            .long_about("Print the current version number."))
        .subcommand(clap::SubCommand::with_name("where")
                    .about("Show the path to a package or dataset")
                    .long_about("Show the path to a package or dataset.")
                    .arg(clap::Arg::with_name("package_or_dataset_id")
                         .value_name("id")
                         .takes_value(true)
                         .default_value(fallback_dataset)
                         .validator(id_nonempty)
                         .index(1)
                         .help("A package or collection ID")))
        .subcommand(clap::SubCommand::with_name("whoami")
                    .about("Displays information about the logged in user")
                    .long_about("Displays information about the logged in user."));

    // Get the raw argument count:
    let raw_arg_count = env::args().count();
    if raw_arg_count <= 1 {
        app.print_help().expect("couldn't print help");
        exit(1);
    }

    // Pull out the global `--output` option:
    let parse = app.get_matches_from_safe_borrow(env::args());
    let args = match parse {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{}", e.message);
            exit(1);
        }
    };

    // What kind of output format do we want?
    let output: ps::OutputFormat = args
        .value_of("output")
        .map(|format| format.parse().unwrap_or_default())
        .unwrap_or_default();

    context.set_output(output);

    let matches = match app.get_matches_from_safe_borrow(&mut env::args()) {
        Ok(matches) => matches,
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    };

    let db = context.db.clone();

    let system = System::new("ps:main");

    let toplevel: ps::Future<()> = match matches.subcommand() {
        ("append", Some(args)) => with_cli!(context, cli, {
            let files = args
                .values_of("paths")
                .map(|p| p.collect())
                .unwrap_or_else(|| vec![]);
            let dataset = args.value_of("dataset");
            let package = args.value_of("package");
            let recursive = args.is_present("recursive");
            let force = args.is_present("force");
            let parallelism = parallelism_level(args.value_of("parallelism"));

            cli.queue_uploads(files, dataset, package, true, force, recursive)
                .and_then(move |_| {
                    context.uploading(
                        cli,
                        StartMode::NoEmptyQueue,
                        StopMode::OnFinish,
                        parallelism,
                    )
                })
        }),
        ("clear", _) => with_cli!(context, cli, {
            run_then_exit!(cli.clear_settings_dataset())
        }),
        ("create-collection", Some(args)) => with_cli!(context, cli, {
            run_then_exit!(cli.create_collection(
                args.value_of("name").unwrap(),
                args.value_of("dataset").unwrap(),
            ))
        }),
        ("config", Some(config_matches)) => match config_matches.subcommand() {
            ("show", Some(args)) => with_cli!(context, cli, {
                match args.value_of("key") {
                    Some(conf_key) => run_then_exit!(cli.print_settings_value(conf_key)),
                    None => run_then_exit!(cli.print_settings_key_values()),
                }
            }),
            ("example", _) => run_then_exit!(Cli::print_config_example()),
            ("wizard", _) => run_then_exit!(Cli::start_config_wizard(context.db)),
            ("schema-version", Some(args)) => match args.value_of("version") {
                Some(schema_version) => with_cli!(context, cli, {
                    match schema_version.parse::<usize>() {
                        Ok(version) => run_then_exit!(cli.set_schema_version(version)),
                        Err(e) => run_then_exit!(future::err::<(), _>(e.into()).into_trait()),
                    }
                }),
                None => with_cli!(context, cli, { run_then_exit!(cli.print_schema_version()) }),
            },
            _ => run_then_exit!(Cli::print_or_create_config(context.db)),
        },
        ("collaborators", Some(collab_matches)) => match collab_matches.subcommand() {
            ("teams", _) => with_cli!(context, cli, {
                run_then_exit!(cli
                    .print_dataset_team_collaborators(collab_matches.value_of("dataset").unwrap()))
            }),
            ("organization", _) => with_cli!(context, cli, {
                run_then_exit!(cli
                    .print_dataset_organization_role(collab_matches.value_of("dataset").unwrap()))
            }),
            ("users", _) => with_cli!(context, cli, {
                run_then_exit!(cli
                    .print_dataset_user_collaborators(collab_matches.value_of("dataset").unwrap()))
            }),
            _ => with_cli!(context, cli, {
                run_then_exit!(cli
                    .print_all_dataset_collaborators(collab_matches.value_of("dataset").unwrap()))
            }),
        },
        ("datasets", _) => with_cli!(context, cli, { run_then_exit!(cli.print_datasets()) }),
        ("create-dataset", Some(args)) => with_cli!(context, cli, {
            run_then_exit!(
                cli.create_dataset(args.value_of("name").unwrap(), args.value_of("description"))
            )
        }),
        ("ls", Some(ls_matches)) => {
            let dataset = ls_matches.value_of("dataset");
            let collection_id = ls_matches.value_of("collection");
            with_cli!(context, cli, {
                match (dataset, collection_id) {
                    (_, Some(collection_id)) => run_then_exit!(cli.print_collection(collection_id)),
                    (Some(dataset), _) => run_then_exit!(cli.print_dataset(dataset)),
                    _ => run_then_exit!(cli.print_datasets()),
                }
            })
        }
        ("members", _) => with_cli!(context, cli, { run_then_exit!(cli.print_members()) }),
        ("move", Some(mv_matches)) => {
            let source = mv_matches.value_of("source").unwrap();
            let destination = mv_matches.value_of("destination");
            with_cli!(context, cli, {
                run_then_exit!(cli.move_package(source, destination))
            })
        }
        ("organizations", _) => {
            with_cli!(context, cli, { run_then_exit!(cli.print_organizations()) })
        }
        ("profile", Some(profile_matches)) => match profile_matches.subcommand() {
            ("switch", Some(args)) => {
                let new_profile = args.value_of("profile").unwrap();
                let current_profile = context.get_current_profile().unwrap();
                if current_profile == new_profile {
                    run_then_exit!({
                        eprintln!(
                            "'{}' is already the current profile, no action taken.",
                            current_profile
                        );
                    })
                } else if current_profile == c::ENVIRONMENT_OVERRIDE_PROFILE {
                    let token_var = match var("PENNSIEVE_API_TOKEN").is_ok() {
                        true => "PENNSIEVE_API_TOKEN",
                        _ => "PENNSIEVE_API_KEY",
                    };
                    let secret_var = match var("PENNSIEVE_API_SECRET").is_ok() {
                        true => "PENNSIEVE_API_SECRET",
                        _ => "PENNSIEVE_SECRET_KEY",
                    };
                    eprintln!(
                        "Profile is currently overridden by environment variables.

Unset these variables in order to use profiles from config.ini:
{} and {}",
                        token_var, secret_var
                    );
                    future::err(config::Error::illegal_operation("switch").into()).into_trait()
                } else {
                    run_then_exit!(context.with_new_api_profile(new_profile))
                }
            }
            ("create", _) => run_then_exit!(Cli::create_profile_prompt(context.db)),
            ("delete", Some(args)) => {
                let profile_to_delete = args.value_of("profile").unwrap();
                run_then_exit!(Config::from_config_file_and_environment()
                    .and_then(|mut config| config::api::delete_profile(
                        &mut config.api_settings,
                        profile_to_delete
                    )
                    .and_then(|_| config.write_to_config_file()))
                    .map_err(Into::into)
                    .into_future())
            }
            ("set-default", Some(args)) => {
                let new_default = args.value_of("profile").unwrap();
                run_then_exit!(Config::from_config_file_and_environment()
                    .and_then(|mut config| config::api::set_default_profile(
                        &mut config.api_settings,
                        new_default
                    )
                    .and_then(|_| config.write_to_config_file()))
                    .map_err(Into::into)
                    .into_future())
            }
            ("list", _) => run_then_exit!(Config::from_config_file_and_environment()
                .map(|config| println!(
                    "Profiles: \n  {}",
                    config.api_settings.profile_names().join("\n  ")
                ))
                .map_err(Into::into)
                .into_future()),
            // any other subcommand will display the current profile
            (_, _) => run_then_exit!(context
                .get_current_profile()
                .map(|profile| println!("Current profile: {}", profile))
                .into_future()),
        },
        ("rename", Some(args)) => with_cli!(context, cli, {
            run_then_exit!(cli.rename(
                args.value_of("package_or_dataset_id").unwrap(),
                args.value_of("name").unwrap()
            ))
        }),
        ("server", Some(args)) => {
            let parallelism = parallelism_level(args.value_of("parallelism"));

            run!(context.start_server_mode(parallelism).into_future())
        }
        ("teams", _) => with_cli!(context, cli, { run_then_exit!(cli.print_teams()) }),
        ("upload", Some(args)) => with_cli!(context, cli, {
            let files = args
                .values_of("paths")
                .map(|p| p.collect())
                .unwrap_or_else(|| vec![]);
            let dataset = args.value_of("dataset");
            let package = args.value_of("folder"); // folder == package
            let recursive = args.is_present("recursive");
            let force = args.is_present("force");
            let parallelism = parallelism_level(args.value_of("parallelism"));

            // validate the upload args
            if recursive && files.len() > 1 {
                eprintln!("Recursive uploads can only contain one path argument");
                exit(1)
            }
            cli.queue_uploads(files, dataset, package, false, force, recursive)
                .and_then(move |_| {
                    context.uploading(
                        cli,
                        StartMode::NoEmptyQueue,
                        StopMode::OnFinish,
                        parallelism,
                    )
                })
        }),
        ("upload-status", Some(args)) => with_cli!(context, cli, {
            let parallelism = parallelism_level(args.value_of("parallelism"));

            if let Some(cancel_ids) = args.values_of("cancel") {
                run_then_exit!(cli.cancel_uploads(strings!(cancel_ids)))
            } else if let Some(retry_ids) = args.values_of("retry") {
                run_then_exit!(cli.requeue_failed_uploads(strings!(retry_ids)).and_then(
                    move |_| context.uploading(
                        cli,
                        StartMode::NoEmptyQueue,
                        StopMode::OnFinish,
                        parallelism
                    )
                ))
            } else if args.is_present("cancel_all") {
                run_then_exit!(cli.cancel_all_uploads())
            } else if args.is_present("cancel_pending") {
                run_then_exit!(cli.cancel_pending_uploads())
            } else if args.is_present("listen") {
                let port = value_t!(args.value_of("port"), u16).ok();
                run!(context.uploading(
                    cli,
                    StartMode::AllowEmptyQueue(port),
                    StopMode::Never,
                    parallelism
                ))
            } else if args.is_present("resume") {
                run!(context.uploading(
                    cli,
                    StartMode::NoEmptyQueue,
                    StopMode::OnFinish,
                    parallelism
                ))
            } else if let Some(num) = args.value_of("completed") {
                run_then_exit!(cli.most_recently_completed_uploads(num.parse::<usize>().unwrap()))
            } else if args.is_present("failed") {
                run_then_exit!(cli.failed_uploads())
            } else {
                run_then_exit!(cli.active_uploads())
            }
        }),
        ("upload-verify", Some(args)) => with_cli!(context, cli, {
            let upload_id = args.value_of("id").unwrap().parse::<usize>().unwrap();
            let file_path = args.value_of("path").map(PathBuf::from);

            run_then_exit!(cli
                .verify_upload(upload_id, file_path)
                .map(move |_| println!("Verified upload {}.", upload_id))
                .map_err(|e| match e.kind() {
                    ErrorKind::CliError {
                        kind: cli::ErrorKind::UploadDoesNotMatch { path: local_path },
                    } => {
                        eprintln!(
                            "Local file does not match file on the Pennsieve platform: {:?}",
                            local_path
                        );
                        exit(1)
                    }
                    _ => exit(e.render()),
                }))
        }),
        ("use", Some(args)) => with_cli!(context, cli, {
            match args.value_of("dataset") {
                Some(id) => run_then_exit!(cli.set_settings_dataset(id)),
                None => run_then_exit!(cli.print_settings_dataset()),
            }
        }),
        ("version", _) => run_then_exit!({ println!("{}", env!("CARGO_PKG_VERSION")) }),
        ("where", Some(args)) => with_cli!(context, cli, {
            run_then_exit!(cli.where_(args.value_of("package_or_dataset_id").unwrap()))
        }),
        ("whoami", Some(_)) => with_cli!(context, cli, { run_then_exit!(cli.print_whoami()) }),
        _ => {
            // Calling this will result in a panic. See clap issue
            // https://github.com/clap-rs/clap/issues/1356
            //app.print_help();
            run_then_exit!({
                println!(
                    "\nRun `{} --help` for available options.\n",
                    current_exe().expect("couldn't get program name").display()
                );
            })
        }
    };

    // Check for new agent version before anything else
    // Ignore any errors and log a warning
    let fut = ps::version::check_for_new_version(db)
        .then(|result| {
            if let Err(e) = result {
                info!("{}", e.kind());
            }
            Ok(())
        })
        .and_then(|_| toplevel);

    Arbiter::spawn(fut.map(|_| ()).map_err(|e| {
        let exit_code = e.render();
        System::current().stop_with_code(exit_code);
    }));

    let code = system.run();
    exit(code);
}
