use std::collections::HashMap;
use std::default::Default;
use std::env;
use std::env::temp_dir;
use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path;
use std::str::{self, FromStr};

use ini::{self, Ini};
use serde_derive::Deserialize;

use crate::ps;
use crate::ps::agent::cli::input::confirm;
use crate::ps::agent::config::constants as c;

use pennsieve_rust::Environment as ApiEnvironment;

pub mod api;
pub mod constants;
mod error;

pub use self::api::{AgentSettings, ConfigStore, GlobalSettings};
pub use self::error::{Error, ErrorKind, Result};

// PS_HOME/config.ini file header:
const PS_HEADER: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/ps_header"));

/// A typeful representation of the pennsieve configuration file located at
/// `$HOME/.pennsieve/config.ini`.
///
/// This struct also includes an `environment_override` key which will
/// be populated if the user has included environment variables to
/// override their current profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub cache: CacheConfig,
    pub metrics: bool,
    services: Vec<Service>,
    pub api_settings: api::Settings,
    pub environment_override: bool,
    pub status_server_port: u16,
}

impl Config {
    /// Create a new Config object using the user's environment,
    /// falling back to the config file.
    ///
    /// If the user has provided a token and secret in their
    /// environment, the config file is not required. Defaults will be
    /// used for all other values.
    pub fn from_config_file_and_environment() -> Result<Self> {
        let environment_override = Self::get_environment_override();

        // If no configuration file exists, fall back to the default.
        let mut config = Self::from_config_file().unwrap_or_else(|_| Self::default());

        if let Some(environment_override) = environment_override {
            config.add_environment_override(environment_override)?
        }
        config.validate()?;
        Ok(config)
    }

    /// Get the environment override from the current process if it exists.
    fn get_environment_override() -> Option<api::ProfileConfig> {
        let token = match (
            env::var("PENNSIEVE_API_TOKEN").ok(),
            env::var("PENNSIEVE_API_KEY").ok(),
        ) {
            (k @ Some(_), _) => k,
            (None, k @ Some(_)) => k,
            _ => None,
        };

        let secret = match (
            env::var("PENNSIEVE_API_SECRET").ok(),
            env::var("PENNSIEVE_SECRET_KEY").ok(),
        ) {
            (s @ Some(_), _) => s,
            (None, s @ Some(_)) => s,
            _ => None,
        };

        token
            .and_then(|token| secret.map(|secret| (token, secret)))
            .map(|(token, secret)| {
                let environment: ApiEnvironment = env::var("PENNSIEVE_API_ENVIRONMENT")
                    .ok()
                    .and_then(|env| env.parse::<ApiEnvironment>().ok())
                    .unwrap_or(ApiEnvironment::Production);

                api::ProfileConfig::new("environment_override", token, secret)
                    .with_environment(environment)
            })
    }

    /// Add an environment override to this object
    fn add_environment_override(&mut self, environment_override: api::ProfileConfig) -> Result<()> {
        self.api_settings.add_profile(environment_override);
        self.environment_override = true;

        self.api_settings
            .set_default_profile(c::ENVIRONMENT_OVERRIDE_PROFILE)
    }

    /// Create a new Config object from the config file.
    fn from_config_file() -> Result<Self> {
        let mut file_contents = String::new();
        File::open(ps::config_file().map_err(|e| Error::config_file_not_found(e.to_string()))?)
            .map_err(|e| Error::config_file_not_found(e.to_string()))
            .and_then(|mut file| {
                file.read_to_string(&mut file_contents)
                    .map_err(Into::into)
                    .and_then(|_| file_contents.parse().map_err(Into::into))
            })
    }

    pub fn new(
        cache: CacheConfig,
        metrics: bool,
        services: Vec<Service>,
        api_settings: api::Settings,
        status_server_port: u16,
    ) -> Self {
        Self {
            cache,
            metrics,
            services,
            api_settings,
            environment_override: false,
            status_server_port,
        }
    }

    /// Get all services defined in the Pennsieve config.ini file.
    pub fn get_services(&self) -> &Vec<Service> {
        &self.services
    }

    /// Validate this object:
    ///
    /// - Ensure the api settings are valid
    /// - If the api settings are invalid, ensure that a profile override was provided
    fn validate(&self) -> Result<()> {
        self.api_settings.validate().or_else(|_| {
            let missing_profile = !self.environment_override
                || self
                    .api_settings
                    .get_profile(c::ENVIRONMENT_OVERRIDE_PROFILE)
                    .is_none();

            if missing_profile {
                Err(ErrorKind::MissingProfile.into())
            } else {
                Ok(())
            }
        })
    }

    /// Write this object to a config file. Overwrite the existing
    /// config file if it exists, preserving any untouched keys or
    /// sections.
    pub fn write_to_config_file(&self) -> Result<()> {
        self.validate()?;
        overwrite_configuration_file(self.to_string())
    }
}

// Generate an instance of the configuration with sane default values:
impl Default for Config {
    fn default() -> Self {
        Self::new(
            CacheConfig::default(),
            true,
            vec![
                Service::Proxy(ProxyService::default()),
                Service::TimeSeries(TimeSeriesService::default()),
                Service::Uploader(UploaderService::default()),
            ],
            Default::default(),
            c::CONFIG_DEFAULT_STATUS_WEBSOCKET_PORT,
        )
    }
}

/// A typeful representation of the "[cache]" section of the agent's
/// configuration file.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct CacheConfig {
    base_path: path::PathBuf,
    page_size: u32,
    soft_cache_size: u64,
    hard_cache_size: u64,
}

impl CacheConfig {
    pub fn new<P>(base_path: P, page_size: u32, soft_cache_size: u64, hard_cache_size: u64) -> Self
    where
        P: AsRef<path::Path>,
    {
        let base_path = base_path.as_ref().to_path_buf();
        Self {
            base_path,
            page_size,
            soft_cache_size,
            hard_cache_size,
        }
    }

    /// Returns the given base_path as a value conforming to the path::Path
    /// interface.
    pub fn base_path(&self) -> &path::Path {
        &self.base_path
    }

    /// Returns a path that represents that location of where
    /// the template file should exist.
    pub fn get_template_path(&self) -> path::PathBuf {
        let mut template_path = path::PathBuf::from(&self.base_path);
        template_path.push("templates");
        template_path.push(self.page_size.to_string());
        template_path.set_extension("bin");
        template_path
    }

    pub fn page_size(&self) -> u32 {
        self.page_size
    }

    pub fn soft_cache_size(&self) -> u64 {
        self.soft_cache_size
    }

    pub fn hard_cache_size(&self) -> u64 {
        self.hard_cache_size
    }

    pub fn set_page_size(&mut self, size: u32) {
        self.page_size = size;
    }
    pub fn set_soft_cache_size(&mut self, size: u64) {
        self.soft_cache_size = size;
    }
    pub fn set_hard_cache_size(&mut self, size: u64) {
        self.hard_cache_size = size;
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::new(
            ps::cache_dir().unwrap_or_else(|_| temp_dir().into_boxed_path()),
            c::CONFIG_DEFAULT_PAGE_SIZE,
            c::CONFIG_DEFAULT_SOFT_CACHE_SIZE,
            c::CONFIG_DEFAULT_HARD_CACHE_SIZE,
        )
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct ProxyService {
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}
impl Default for ProxyService {
    fn default() -> Self {
        Self {
            local_port: c::CONFIG_DEFAULT_PROXY_LOCAL_PORT,
            remote_port: c::CONFIG_DEFAULT_PROXY_REMOTE_PORT,
            remote_host: c::CONFIG_DEFAULT_PROXY_REMOTE_HOST.to_string(),
        }
    }
}
impl ProxyService {
    pub fn set_local_port(&mut self, local_port: u16) {
        self.local_port = local_port;
    }
    pub fn set_remote_port(&mut self, remote_port: u16) {
        self.remote_port = remote_port;
    }
    pub fn set_remote_host<S: Into<String>>(&mut self, remote_host: S) {
        self.remote_host = remote_host.into();
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct TimeSeriesService {
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}
impl Default for TimeSeriesService {
    fn default() -> Self {
        Self {
            local_port: c::CONFIG_DEFAULT_TIMESERIES_LOCAL_PORT,
            remote_port: c::CONFIG_DEFAULT_TIMESERIES_REMOTE_PORT,
            remote_host: c::CONFIG_DEFAULT_TIMESERIES_REMOTE_HOST.to_string(),
        }
    }
}
impl TimeSeriesService {
    pub fn set_local_port(&mut self, local_port: u16) {
        self.local_port = local_port;
    }
    pub fn set_remote_port(&mut self, remote_port: u16) {
        self.remote_port = remote_port;
    }
    pub fn set_remote_host<S: Into<String>>(&mut self, remote_host: S) {
        self.remote_host = remote_host.into();
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct UploaderService {}
impl Default for UploaderService {
    fn default() -> Self {
        Self {}
    }
}

/// Types of services that the agent can spawn
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type")]
pub enum Service {
    Proxy(ProxyService),
    TimeSeries(TimeSeriesService),
    Uploader(UploaderService),
}

// consts for parsing

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ini = Ini::new();

        // global api settings
        for (k, v) in &(*self.api_settings.global_settings) {
            ini.with_section(Some(c::GLOBAL_SECTION))
                .set(k.clone(), v.clone());
        }

        fn agent_section(ini: &mut Ini) -> ini::ini::SectionSetter<'_> {
            ini.with_section(Some(c::AGENT_SECTION))
        }

        // global agent settings
        agent_section(&mut ini).set("metrics", if self.metrics { "true" } else { "false" });

        // cache settings
        agent_section(&mut ini)
            .set("cache_base_path", self.cache.base_path.to_str().unwrap())
            .set("cache_page_size", self.cache.page_size.to_string())
            .set(
                "cache_soft_cache_size",
                self.cache.soft_cache_size.to_string(),
            )
            .set(
                "cache_hard_cache_size",
                self.cache.hard_cache_size.to_string(),
            );

        // services
        // Note that we don't expose the ability to configure remote
        // host/port to users
        for service in &self.services {
            let mut agent_section = agent_section(&mut ini);
            match service {
                Service::Proxy(ProxyService { local_port, .. }) => agent_section
                    .set("proxy", "true")
                    .set("proxy_local_port", local_port.to_string().clone()),
                Service::TimeSeries(TimeSeriesService { local_port, .. }) => agent_section
                    .set("timeseries", "true")
                    .set("timeseries_local_port", local_port.to_string().clone()),
                Service::Uploader(_) => agent_section.set("uploader", "true"),
            };
        }

        // status server:
        agent_section(&mut ini).set("status_port", self.status_server_port.to_string());

        // profiles
        for (profile_name, profile) in &self.api_settings.profiles {
            ini.with_section(Some(profile_name.clone()))
                .set(c::API_TOKEN_KEY, profile.token.clone())
                .set(c::API_SECRET_KEY, profile.secret.clone());

            if profile.environment != ApiEnvironment::Production {
                ini.with_section(Some(profile_name.clone()))
                    .set(c::ENVIRONMENT_KEY, profile.environment.to_string());
            }
        }

        let mut bytes: Vec<u8> = vec![];
        ini.write_to(&mut bytes).unwrap();
        let ini_str = str::from_utf8(&bytes).unwrap();
        write!(f, "{}", ini_str)
    }
}

// Parse the agent's configuration file into a publicly consumeable
// representation with sane default values.
impl FromStr for Config {
    type Err = Error;

    fn from_str(raw_ini: &str) -> Result<Self> {
        let ini = Ini::load_from_str(raw_ini)?;

        let global_settings: HashMap<_, _> = ini
            .section(Some(c::GLOBAL_SECTION))
            .cloned()
            .ok_or_else(|| {
                Error::invalid_api_config(format!("section not found: {}", c::GLOBAL_SECTION))
            })?;

        let global_settings: GlobalSettings = global_settings.into();

        // Create an agent settings object, merging in keys for default values
        // that are missing from reading in $PS_HOME/config.ini
        let mut agent_settings: AgentSettings = ini
            .section(Some(c::AGENT_SECTION))
            .cloned()
            .map(AgentSettings::from)
            .unwrap_or_default();
        agent_settings.merge_default::<AgentSettings>();

        // global agent settings
        let metrics = agent_settings
            .get_as_and_update::<_, bool>("metrics", c::CONFIG_ENABLE_SERVICES_BY_DEFAULT)?;

        // cache
        let cache_base_path = agent_settings.get_required("cache_base_path")?;

        let cache_page_size = agent_settings
            .get_as_and_update::<_, u32>("cache_page_size", c::CONFIG_DEFAULT_PAGE_SIZE)?;
        let cache_soft_cache_size = agent_settings.get_as_and_update::<_, u64>(
            "cache_soft_cache_size",
            c::CONFIG_DEFAULT_SOFT_CACHE_SIZE,
        )?;
        let cache_hard_cache_size = agent_settings.get_as_and_update::<_, u64>(
            "cache_hard_cache_size",
            c::CONFIG_DEFAULT_HARD_CACHE_SIZE,
        )?;

        let cache_config = CacheConfig::new(
            cache_base_path,
            cache_page_size,
            cache_soft_cache_size,
            cache_hard_cache_size,
        );

        // status server port:
        let status_server_port = agent_settings
            .get_as_and_update::<_, u16>("status_port", c::CONFIG_DEFAULT_STATUS_WEBSOCKET_PORT)?;

        // services
        let mut services: Vec<Service> = vec![];

        // proxy service -- only disable if timeseries=false is explicitly
        // provided in config.ini
        {
            let proxy_enabled = agent_settings
                .get_as_and_update::<_, bool>("proxy", c::CONFIG_ENABLE_SERVICES_BY_DEFAULT)?;
            let proxy_local_port = agent_settings.get_as_and_update::<_, u16>(
                "proxy_local_port",
                c::CONFIG_DEFAULT_PROXY_LOCAL_PORT,
            )?;
            let proxy_remote_port = agent_settings.get_as_and_update::<_, u16>(
                "proxy_remote_port",
                c::CONFIG_DEFAULT_PROXY_REMOTE_PORT,
            )?;
            let proxy_remote_host = agent_settings.get_and_update(
                "proxy_remote_host",
                c::CONFIG_DEFAULT_PROXY_REMOTE_HOST.to_string(),
            );

            if proxy_enabled {
                let mut service = ProxyService::default();
                service.set_local_port(proxy_local_port);
                service.set_remote_port(proxy_remote_port);
                service.set_remote_host(proxy_remote_host.clone());
                services.push(Service::Proxy(service));
            }
        }

        // timeseries service -- only disable if timeseries=false is explicitly
        // provided in config.ini
        {
            let timeseries_enabled = agent_settings
                .get_as_and_update::<_, bool>("timeseries", c::CONFIG_ENABLE_SERVICES_BY_DEFAULT)?;
            let timeseries_local_port = agent_settings.get_as_and_update::<_, u16>(
                "timeseries_local_port",
                c::CONFIG_DEFAULT_TIMESERIES_LOCAL_PORT,
            )?;
            let timeseries_remote_port = agent_settings.get_as_and_update::<_, u16>(
                "timeseries_remote_port",
                c::CONFIG_DEFAULT_TIMESERIES_REMOTE_PORT,
            )?;
            let timeseries_remote_host = agent_settings.get_and_update(
                "timeseries_remote_host",
                c::CONFIG_DEFAULT_TIMESERIES_REMOTE_HOST.to_string(),
            );

            if timeseries_enabled {
                let mut service = TimeSeriesService::default();
                service.set_local_port(timeseries_local_port);
                service.set_remote_port(timeseries_remote_port);
                service.set_remote_host(timeseries_remote_host.clone());
                services.push(Service::TimeSeries(service));
            }
        }

        // uploader worker -- only disable if timeseries=false is explicitly
        // provided in config.ini
        {
            let uploaded_enabled = agent_settings
                .get_as_and_update::<_, bool>("uploader", c::CONFIG_ENABLE_SERVICES_BY_DEFAULT)?;

            if uploaded_enabled {
                services.push(Service::Uploader(UploaderService {}));
            }
        }

        // profiles
        let profiles: Result<Vec<(String, api::ProfileConfig)>> = ini
            .into_iter()
            .filter(|item| {
                item.0.is_some()
                    && item.0 != Some(c::GLOBAL_SECTION.into())
                    && item.1.contains_key(c::API_TOKEN_KEY)
                    && item.1.contains_key(c::API_SECRET_KEY)
            })
            .map(|item| {
                let profile_name = item.0.unwrap();
                api::ProfileConfig::from_ini_item(profile_name.clone(), &item.1)
                    .map(|conf| (profile_name, conf))
            })
            .collect();

        let profiles: HashMap<String, api::ProfileConfig> =
            profiles.map(|profiles| profiles.iter().cloned().collect())?;

        let api_settings = api::Settings::new(profiles, global_settings, agent_settings)?;

        Ok(Config::new(
            cache_config,
            metrics,
            services,
            api_settings,
            status_server_port,
        ))
    }
}

/// merge two INI objects
///
/// only keep sections that are in the new config. within
/// those sections, keep all keys from the old config in order
/// to persist configurations that are not specific to the
/// agent.
fn merge_ini(old: &Ini, new: &mut Ini) {
    for (section_name, section_props) in new.iter_mut() {
        if let Some(existing_section) = old.section(section_name.as_ref().cloned()) {
            for (k, v) in existing_section {
                if !section_props.contains_key(k) {
                    section_props.insert(k.to_string(), v.to_string());
                }
            }
        }
    }
}

/// Overwrite the configuration file with the given new contents. A
/// warning will be presented to the user if the old file existed and
/// did not start with the PS_HEADER
fn overwrite_configuration_file<S: Into<String>>(new_contents: S) -> Result<()> {
    // get the string representation of this object
    let mut new_config = Ini::load_from_str(&new_contents.into())?;

    let path = ps::config_file().map_err(|e| Error::config_file_not_found(e.to_string()))?;

    if path.exists() {
        let mut old_contents = String::new();
        let mut file = File::open(path.clone())?;
        file.read_to_string(&mut old_contents)?;
        let old_contents = old_contents.trim();

        if !old_contents.starts_with(PS_HEADER)
            && !confirm("Continue and write changes?".to_string())?
        {
            println!("Operation aborted, new configurations were not saved.");
            return Ok(());
        }

        // remove the old config file
        fs::remove_file(path.clone())?;

        let existing_config = Ini::load_from_str(&old_contents)?;
        merge_ini(&existing_config, &mut new_config);
    }

    // recreate the config file
    let mut file = File::create(path)?;

    // convert the new_config to a string
    let mut bytes: Vec<u8> = vec![];
    new_config.write_to(&mut bytes).unwrap();
    let managed_config = str::from_utf8(&bytes).unwrap();

    // write the managed config string to the config file, prepended
    // with PS_HEADER
    write!(file, "{}\n{}", PS_HEADER, managed_config)?;
    Ok(())
}

/// Start an interactive wizard to create a new configuration and profile
pub fn start_config_wizard() -> Result<Config> {
    let path = ps::config_file().map_err(|e| Error::config_file_not_found(e.to_string()))?;

    let confirmation_message = format!(
        "Existing configuration file found at {:?}.

Would you like to overwrite your existing configuration?",
        path
    );

    if (path.exists() && confirm(confirmation_message)?) || !path.exists() {
        let path = ps::config_file().map_err(|e| Error::config_file_not_found(e.to_string()))?;

        println!(
            "Creating new configuration file at {}",
            path.to_str().unwrap()
        );

        let mut config = Config::default();
        api::create_profile_prompt(&mut config.api_settings)
            .and_then(|_| {
                config.write_to_config_file()?;
                Ok(config)
            })
            .map_err(Into::into)
    } else {
        Err(ErrorKind::UserCancelledError.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fail_to_parse_invalid_ini_string() {
        let ini_str = "";
        let config = ini_str.parse::<Config>();

        assert!(config.is_err());
    }

    #[test]
    fn fail_to_parse_invalid_boolean() {
        let ini_str = test_ini_with_agent_settings(
            r#"
            uploader = foo
        "#,
        );
        let config = (&ini_str).parse::<Config>();
        assert!(config.is_err());
        let config = config.err().unwrap();
        let message = config.to_string();
        assert!(message.contains("bad value for configuration option \"uploader\""));
    }

    #[test]
    fn fail_to_parse_invalid_integer() {
        let ini_str = test_ini_with_agent_settings(
            r#"
            cache_page_size = foo
        "#,
        );
        let config = (&ini_str).parse::<Config>();
        assert!(config.is_err());
        let config = config.err().unwrap();
        let message = config.to_string();
        assert!(message.contains("bad value for configuration option \"cache_page_size\""));
    }

    fn test_ini_with_agent_settings(agent_settings: &str) -> String {
        format!(
            r#"
            [global]
            default_profile=default

            [default]
            api_token={}
            api_secret={}

            [agent]
            {}
        "#,
            env!("PENNSIEVE_API_KEY"),
            env!("PENNSIEVE_SECRET_KEY"),
            agent_settings
        )
    }

    #[test]
    fn valid_proxy_raw_config() {
        let ini_str = test_ini_with_agent_settings(
            r#"
            cache_page_size = 10000
            cache_soft_cache_size = 20000
            cache_hard_cache_size = 50000

            proxy = true
            proxy_local_port = 8000
            proxy_remote_host = "https://www.google.com"
            proxy_remote_port = 443

            timeseries = true
            timeseries_local_port = 8001
            timeseries_remote_host = "wss://echo.websocket.org"
            timeseries_remote_port = 443

            uploader = true
            metric = false
        "#,
        );
        let mut cache_cfg = CacheConfig::default();
        cache_cfg.set_page_size(10000);
        cache_cfg.set_soft_cache_size(20000);
        cache_cfg.set_hard_cache_size(50000);

        let proxy = Service::Proxy(ProxyService {
            local_port: 8000,
            remote_host: "https://www.google.com".to_string(),
            remote_port: 443,
        });
        let websocket = Service::TimeSeries(TimeSeriesService {
            local_port: 8001,
            remote_host: "wss://echo.websocket.org".to_string(),
            remote_port: 443,
        });
        let uploader = Service::Uploader(UploaderService {});
        let config = &ini_str.parse::<Config>().unwrap();
        let cache = config.clone().cache;
        let services = config.clone().services;

        assert_eq!(cache, cache_cfg);
        assert_eq!(services.len(), 3);
        assert_eq!(services, vec![proxy, websocket, uploader]);
    }

    #[test]
    fn valid_metrics() {
        let ini_str = test_ini_with_agent_settings(
            r#"
            metrics = true
        "#,
        );
        let config = (&ini_str).parse::<Config>().unwrap();

        assert!(config.metrics);
    }

    #[test]
    fn valid_public_cache_config() {
        let ini_str = test_ini_with_agent_settings(
            r#"
            cache_page_size = 500
            cache_soft_cache_size = 600
            cache_hard_cache_size = 700
            cache_base_path = "/path/to/data"
        "#,
        );
        let cache_cfg = CacheConfig::new("/path/to/data", 500, 600, 700);
        let config = (&ini_str).parse::<Config>().unwrap();

        assert_eq!(config.cache, cache_cfg);
        assert!(config.services.len() > 0);
    }

    #[test]
    fn valid_public_cache_config_omitted_page_size() {
        let ini_str = test_ini_with_agent_settings(
            r#"
            cache_base_path = "/path/to/data"
        "#,
        );
        let cache_cfg = CacheConfig::new(
            "/path/to/data",
            c::CONFIG_DEFAULT_PAGE_SIZE,
            c::CONFIG_DEFAULT_HARD_CACHE_SIZE / 2,
            c::CONFIG_DEFAULT_HARD_CACHE_SIZE,
        );
        let config = (&ini_str).parse::<Config>().unwrap();
        assert_eq!(config.cache, cache_cfg);
        assert!(config.services.len() > 0);
    }

    #[test]
    fn valid_public_cache_config_omitted_base_path() {
        let ini_str = test_ini_with_agent_settings(
            r#"
            cache_page_size = 500
            cache_soft_cache_size = 600
            cache_hard_cache_size = 700
        "#,
        );
        let cache_cfg = CacheConfig::new(ps::cache_dir().unwrap(), 500, 600, 700);
        let config = (&ini_str).parse::<Config>().unwrap();
        assert_eq!(config.cache, cache_cfg);
        assert!(config.services.len() > 0);
    }

    #[test]
    fn invalid_proxy_config() {
        let ini_str = r#"
            cache_page_size = 10000

            proxy = true
            proxy_local_port = invalidport
            proxy_remote_host = "https://www.google.com"
            proxy_remote_port = 443
        "#;
        assert!((&ini_str).parse::<Config>().is_err());
    }

    #[test]
    fn serde_no_modifications() {
        let ini_str = test_ini_with_agent_settings(
            r#"
            metrics = true
            cache_page_size = 10000
            cache_base_path = "~/.pennsieve/cache"
            cache_soft_cache_size = 5000000000
            cache_hard_cache_size = 10000000000
            proxy = true
            proxy_local_port = 8080
            timeseries = true
            timeseries_local_port = 9500
            uploader = true
            status_port = 11235
        "#,
        );
        let expected = Ini::load_from_str(&ini_str).unwrap();
        let config: Config = ini_str.parse().unwrap();

        let written_settings = Ini::load_from_str(&config.to_string()).unwrap();

        for (section_name, properties) in written_settings.clone() {
            let expected_properties = expected.section(section_name);
            assert!(expected_properties.is_some());
            assert_eq!(expected_properties.unwrap(), &properties);
        }
        for (section_name, expected_properties) in expected {
            let properties = written_settings.section(section_name);
            assert!(properties.is_some());
            assert_eq!(properties.unwrap(), &expected_properties);
        }
    }

    #[test]
    fn serde_with_modifications() {
        let ini_str = test_ini_with_agent_settings(
            r#"
            metrics = true
            cache_page_size = 10000
            cache_base_path = "~/.pennsieve/cache"
            cache_soft_cache_size = 5000000000
            cache_hard_cache_size = 10000000000
            proxy = true
            proxy_local_port = 8080
            timeseries = true
            timeseries_local_port = 9500
            uploader = true
            status_port = 11235
        "#,
        );
        let expected = Ini::load_from_str(&ini_str).unwrap();

        let new_profile_key = "new_profile";

        let mut config: Config = ini_str.parse().unwrap();
        config.api_settings.add_profile(api::ProfileConfig::new(
            new_profile_key.clone(),
            "token",
            "secret",
        ));
        let written_settings = Ini::load_from_str(&config.to_string()).unwrap();

        let mut contains_new_key = false;
        for (section_name, properties) in written_settings.clone() {
            let expected_properties = expected.section(section_name.clone());

            if section_name == Some(new_profile_key.to_string()) {
                contains_new_key = true;
                assert!(expected_properties.is_none());
            } else {
                assert!(expected_properties.is_some());
                assert_eq!(expected_properties.unwrap(), &properties);
            }
        }
        for (section_name, expected_properties) in expected {
            let properties = written_settings.section(section_name);
            assert!(properties.is_some());
            assert_eq!(properties.unwrap(), &expected_properties);
        }

        assert!(contains_new_key);
    }

    #[test]
    fn ini_merge() {
        let old = Ini::load_from_str(
            r#"
            [global]
            default_profile=default

            [default]
            api_token=token
            api_secret=secret

            [deleted_profile]
            api_token=token
            api_secret=secret

            [agent]
            agent_setting=value
            unrelated_setting=value
        "#,
        )
        .unwrap();

        let mut new = Ini::load_from_str(
            r#"
            [global]
            default_profile=new_profile

            [default]
            api_token=token
            api_secret=secret

            [new_profile]
            api_token=token
            api_secret=secret

            [agent]
            agent_setting=new_value
        "#,
        )
        .unwrap();

        merge_ini(&old, &mut new);

        assert!(new.section(Some("new_profile")).is_some());
        assert!(new.section(Some("deleted_profile")).is_none());

        assert_eq!(
            new.section(Some("agent")).unwrap()["unrelated_setting"],
            "value"
        );
        assert_eq!(
            new.section(Some("agent")).unwrap()["agent_setting"],
            "new_value"
        );
        assert_eq!(
            new.section(Some("global")).unwrap()["default_profile"],
            "new_profile"
        );
    }
}
