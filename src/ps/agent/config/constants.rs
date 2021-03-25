//! Program level configuration constants live here.

/// The maximum connection backlog limit:
pub const AGENT_CONNECTION_BACKLOG_LIMIT: i32 = 128;

/// CLI logging output format.
pub const AGENT_LOG_FORMAT: &str =
    "[{d(%Y-%m-%d %H:%M:%S %Z)(utc)}][{l}][{t}][{X(tid)(main)}] {m}{n}";

/// Shutdown the process after a timeout period.
pub const AGENT_MAX_SHUTDOWN_TIMEOUT_SECS: u64 = 5;

/// Config defaults:
pub const CONFIG_DEFAULT_PAGE_SIZE: u32 = 100_000; // 10k data points = 80 KB
pub const CONFIG_DEFAULT_HARD_CACHE_SIZE: u64 = 10_000_000_000; // 10 GB
pub const CONFIG_DEFAULT_SOFT_CACHE_SIZE: u64 = CONFIG_DEFAULT_HARD_CACHE_SIZE / 2;
pub const CONFIG_DEFAULT_PROXY_LOCAL_PORT: u16 = 8080;
pub const CONFIG_DEFAULT_PROXY_REMOTE_HOST: &str = "https://api.pennsieve.io";
pub const CONFIG_DEFAULT_PROXY_REMOTE_PORT: u16 = 443;
pub const CONFIG_DEFAULT_TIMESERIES_LOCAL_PORT: u16 = 9090;
//pub const CONFIG_DEFAULT_TIMESERIES_REMOTE_HOST: &str = "wss://streaming.pennsieve.io";
pub const CONFIG_DEFAULT_TIMESERIES_REMOTE_HOST: &str = "wss://streaming.dev.pennsieve.io";
pub const CONFIG_DEFAULT_TIMESERIES_REMOTE_PORT: u16 = 443;
pub const CONFIG_DEFAULT_STATUS_WEBSOCKET_PORT: u16 = 11235;

/// If true, the only way services will be disabled is by including
/// <service-name>=false in config.ini
pub const CONFIG_ENABLE_SERVICES_BY_DEFAULT: bool = true;

/// The collector run interval. A collection cycle will run every N ms.
pub const CACHE_COLLECTOR_RUN_INTERVAL_SECS: u64 = 60 * 15; // 15 minutes

/// CLI progress bar format.
pub const UPLOAD_PROGRESS_BAR_FORMAT: &str =
    "{prefix:8.bold.dim} {spinner} {bar:60.cyan/blue} {pos:>4}% {msg}";
pub const UPLOAD_ERROR_PROGRESS_BAR_FORMAT: &str =
    "{prefix:8.bold.dim} {spinner} {bar:60.red/red} {pos:>4}% {msg}";

/// CLI progress characters.
pub const UPLOAD_PROGRESS_CHARACTERS: &str = "#>-";

/// The refresh interval used when watching the progress of uploaded files.
pub const UPLOAD_PROGRESS_REFRESH_INTERVAL_MS: u64 = 500; // 1/2 second

/// The maximum amount of progress bars that we'll show while uploading
pub const UPLOAD_PROGRESS_MAX_BARS: u64 = 30;

/// The maximum amount of files we'll display to the user for each package in an upload preview.
pub const PREVIEW_DISPLAY_MAX_FILES: usize = 20;

/// The maximum amount of packages we'll display to the user for an upload preview.
pub const PREVIEW_DISPLAY_MAX_PACKAGES: usize = 30;

/// The upload refresh interval.
/// This will check files for upload status changes every N seconds.
pub const UPLOAD_WORKER_RUN_INTERVAL_SECS: u64 = 1;

/// Used for parsing and generating the config.ini file
pub const GLOBAL_SECTION: &str = "global";
pub const AGENT_SECTION: &str = "agent";
pub const DEFAULT_PROFILE_KEY: &str = "default_profile";
pub const API_TOKEN_KEY: &str = "api_token";
pub const API_SECRET_KEY: &str = "api_secret";
pub const ENVIRONMENT_KEY: &str = "environment";
pub const ENVIRONMENT_OVERRIDE_PROFILE: &str = "environment_override";
pub const RESERVED_PROFILE_NAMES: [&str; 3] =
    [GLOBAL_SECTION, AGENT_SECTION, ENVIRONMENT_OVERRIDE_PROFILE];

/// Frequency to check for new versions of the agent (daily
pub const AGENT_LATEST_RELEASE_CHECK_INTERVAL_SECS: u64 = 60 * 60 * 24;

/// URL to bucket that contains public Agent binaries
pub const VERSION_PATH: &str =
    "http://data.pennsieve.io.s3.amazonaws.com/public-downloads/agent/latest";

/// Path in bucket to version.txt for this platform
#[cfg(all(unix, target_os = "macos"))]
pub const VERSION_FILE: &str = "/x86_64-apple-darwin/version.txt";
#[cfg(all(unix, not(target_os = "macos")))]
pub const VERSION_FILE: &str = "/x86_64-unknown-linux-gnu/version.txt";
#[cfg(windows)]
pub const VERSION_FILE: &str = "/x86_64-pc-windows-msvc/version.txt";
