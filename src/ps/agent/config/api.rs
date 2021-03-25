use std::collections::HashMap;
use std::env::temp_dir;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use crate::ps;
use crate::ps::agent::cli::input::{confirm, user_input, user_input_with_default};
use crate::ps::agent::config::constants as c;
use crate::ps::agent::config::error::{Error, Result};

use pennsieve_rust::Environment as ApiEnvironment;

/// A key-value alias for a string-to-string hash map;
type Dict = HashMap<String, String>;

/// Objects that serve as a store of configuration options should
/// implement this interface:
pub trait ConfigStore: Default {
    type Key: ToString;
    type Value: ToString;

    //  Must implement
    /// Get the underlying key-value store backing this object.
    fn store(&mut self) -> &mut HashMap<String, String>;

    /// Merge two ConfigStore objects together.
    /// Note: If self contains a key, it will not be overwritten.
    fn merge<C: ConfigStore>(&mut self, other: &mut C) -> &Self {
        {
            let st = self.store();
            for (key, value) in other.store().iter() {
                if st.get(key).is_none() {
                    st.insert(key.clone(), value.clone());
                }
            }
        };
        self
    }

    /// Like `merge`, but merge this object with a default version.
    fn merge_default<T: ConfigStore>(&mut self) -> &Self {
        let mut default: T = Default::default();
        self.merge(&mut default);
        self
    }

    /// Get the value associated with a key, returning an error describing
    /// the missing key if no value is found.
    fn get_required<K>(&mut self, key: K) -> Result<String>
    where
        K: Into<String>,
    {
        let key = key.into();
        self.store()
            .get(&key)
            .ok_or_else(|| {
                Error::invalid_api_config(format!(
                    "missing required configuration option \"{key}\"",
                    key = key
                ))
            })
            .map(|s| s.clone())
    }

    /// Get the value associated with the key, returning the value if found.
    /// If no value is associated with the key, the provided default value
    /// will be inserted under the given key and returned.
    fn get_and_update<K>(&mut self, key: K, default: Self::Value) -> &String
    where
        K: Into<String>,
    {
        self.store()
            .entry(key.into())
            .or_insert_with(|| default.to_string())
    }

    /// Like `get_and_update`, but parses the stored value as type `T`. If
    /// a value is associated with the key, it will be parsed as type `T`. If
    /// parsing fails, a descriptive error will be returned. If no value is
    /// associated with the key, the default value will be inserted as a
    /// String, and the return value will attempt to be parsed as type `T`.
    fn get_as_and_update<K, T>(&mut self, key: K, default: T) -> Result<T>
    where
        K: Into<String>,
        T: FromStr + ToString,
        <T as FromStr>::Err: 'static + Send + std::error::Error,
    {
        let key = key.into();
        let key_inner = key.clone();
        self.store()
            .entry(key)
            .or_insert_with(|| default.to_string())
            .parse::<T>()
            .map_err(|_| {
                Error::invalid_api_config(format!(
                    "bad value for configuration option \"{key}\"",
                    key = key_inner
                ))
            })
    }
}

/// Global settings map:
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalSettings(Dict);

impl GlobalSettings {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn take(self) -> Dict {
        self.0
    }
}

impl From<Dict> for GlobalSettings {
    fn from(dict: Dict) -> Self {
        GlobalSettings(dict)
    }
}

impl From<GlobalSettings> for Dict {
    fn from(settings: GlobalSettings) -> Self {
        settings.0
    }
}

impl Deref for GlobalSettings {
    type Target = Dict;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GlobalSettings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for GlobalSettings {
    fn default() -> Self {
        GlobalSettings(HashMap::new())
    }
}

impl ConfigStore for GlobalSettings {
    type Key = String;
    type Value = String;
    fn store(&mut self) -> &mut Dict {
        &mut self.0
    }
}

/// Agent settings map:
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSettings(Dict);

impl AgentSettings {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn take(self) -> Dict {
        self.0
    }
}

impl From<Dict> for AgentSettings {
    fn from(kv: Dict) -> Self {
        AgentSettings(kv)
    }
}

impl From<AgentSettings> for Dict {
    fn from(settings: AgentSettings) -> Self {
        settings.0
    }
}

impl ConfigStore for AgentSettings {
    type Key = String;
    type Value = String;
    fn store(&mut self) -> &mut Dict {
        &mut self.0
    }
}

impl Default for AgentSettings {
    fn default() -> Self {
        let mut settings = HashMap::new();
        settings.insert(
            "cache_base_path".to_string(),
            ps::cache_dir()
                .unwrap_or_else(|_| temp_dir().into_boxed_path())
                .to_str()
                .unwrap()
                .to_string(),
        );
        settings.insert(
            "cache_hard_cache_size".to_string(),
            c::CONFIG_DEFAULT_HARD_CACHE_SIZE.to_string(),
        );
        settings.insert(
            "cache_page_size".to_string(),
            c::CONFIG_DEFAULT_PAGE_SIZE.to_string(),
        );
        settings.insert(
            "cache_soft_cache_size".to_string(),
            c::CONFIG_DEFAULT_SOFT_CACHE_SIZE.to_string(),
        );
        settings.insert(
            "metrics".to_string(),
            c::CONFIG_ENABLE_SERVICES_BY_DEFAULT.to_string(),
        );
        settings.insert(
            "proxy".to_string(),
            c::CONFIG_ENABLE_SERVICES_BY_DEFAULT.to_string(),
        );
        settings.insert(
            "proxy_local_port".to_string(),
            c::CONFIG_DEFAULT_PROXY_LOCAL_PORT.to_string(),
        );
        settings.insert(
            "proxy_remote_host".to_string(),
            c::CONFIG_DEFAULT_PROXY_REMOTE_HOST.to_string(),
        );
        settings.insert(
            "proxy_remote_port".to_string(),
            c::CONFIG_DEFAULT_PROXY_REMOTE_PORT.to_string(),
        );
        settings.insert(
            "status_port".to_string(),
            c::CONFIG_DEFAULT_STATUS_WEBSOCKET_PORT.to_string(),
        );
        settings.insert(
            "timeseries".to_string(),
            c::CONFIG_ENABLE_SERVICES_BY_DEFAULT.to_string(),
        );
        settings.insert(
            "timeseries_local_port".to_string(),
            c::CONFIG_DEFAULT_TIMESERIES_LOCAL_PORT.to_string(),
        );
        settings.insert(
            "timeseries_remote_host".to_string(),
            c::CONFIG_DEFAULT_TIMESERIES_REMOTE_HOST.to_string(),
        );
        settings.insert(
            "timeseries_remote_port".to_string(),
            c::CONFIG_DEFAULT_TIMESERIES_REMOTE_PORT.to_string(),
        );
        settings.insert(
            "uploader".to_string(),
            c::CONFIG_ENABLE_SERVICES_BY_DEFAULT.to_string(),
        );
        AgentSettings(settings)
    }
}

impl Deref for AgentSettings {
    type Target = Dict;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AgentSettings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// The configuration for a single profile
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileConfig {
    pub profile: String,
    pub token: String,
    pub secret: String,
    pub environment: ApiEnvironment,
}

impl ProfileConfig {
    pub fn new<S, T, U>(profile: S, token: T, secret: U) -> Self
    where
        S: Into<String>,
        T: Into<String>,
        U: Into<String>,
    {
        Self {
            profile: profile.into(),
            token: token.into(),
            secret: secret.into(),
            environment: ApiEnvironment::Production,
        }
    }

    pub fn from_ini_item<S: Into<String>>(
        section_header: S,
        section_properties: &HashMap<String, String>,
    ) -> Result<Self> {
        let section_header: String = section_header.into();
        let api_token = section_properties.get(c::API_TOKEN_KEY).ok_or_else(|| {
            Error::invalid_api_config(format!(
                "key not found: {}:{}",
                section_header,
                c::API_TOKEN_KEY
            ))
        })?;
        let api_secret = section_properties.get(c::API_SECRET_KEY).ok_or_else(|| {
            Error::invalid_api_config(format!(
                "key not found: {}:{}",
                section_header,
                c::API_SECRET_KEY
            ))
        })?;

        let config = Self::new(
            section_header.clone(),
            api_token.to_string(),
            api_secret.to_string(),
        );

        match section_properties.get(c::ENVIRONMENT_KEY) {
            Some(environment) => environment
                .parse::<ApiEnvironment>()
                .or_else(|_| {
                    Err(Error::invalid_api_config(format!(
                        "invalid environment: {}:{}",
                        section_header, environment
                    )))
                })
                .map(|environment| config.with_environment(environment)),
            None => Ok(config),
        }
    }

    pub fn with_environment(self, environment: ApiEnvironment) -> Self {
        Self {
            profile: self.profile,
            token: self.token,
            secret: self.secret,
            environment,
        }
    }
}

/// This struct contains the relevant sections of a config.ini file
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub profiles: HashMap<String, ProfileConfig>,
    pub global_settings: GlobalSettings,
    pub agent_settings: AgentSettings,
}

impl Default for Settings {
    /// Create a new, empty Settings object
    fn default() -> Self {
        Self {
            profiles: HashMap::new(),
            global_settings: Default::default(),
            agent_settings: Default::default(),
        }
    }
}

impl Settings {
    /// Validate this object:
    ///
    /// - Ensure it has a default profile
    /// - Ensure the default profile has an associated configuration
    pub fn validate(&self) -> Result<()> {
        let default_profile_key = self
            .global_settings
            .get(c::DEFAULT_PROFILE_KEY)
            .cloned()
            .ok_or_else(|| {
                Error::invalid_api_config(format!("key not found: {}", c::DEFAULT_PROFILE_KEY))
            })?;
        if !self.profiles.contains_key(&default_profile_key) {
            return Err(Error::invalid_api_config(format!(
                "default profile not found: {}",
                default_profile_key
            )));
        }
        Ok(())
    }

    /// Create (and validate) a new settings object, given a profiles
    /// map and a global settings map.
    pub fn new(
        profiles: HashMap<String, ProfileConfig>,
        global_settings: GlobalSettings,
        agent_settings: AgentSettings,
    ) -> Result<Self> {
        // validations
        let new_settings = Self {
            profiles,
            global_settings,
            agent_settings,
        };
        new_settings.validate()?;
        Ok(new_settings)
    }

    /// Add the given profile to this instance. If it is the first
    /// profile, make it the default.
    pub fn add_profile(&mut self, profile_config: ProfileConfig) {
        let profile_name = profile_config.profile.clone();
        self.profiles.insert(profile_name.clone(), profile_config);
        if self.profile_names().len() == 1 {
            self.set_default_profile(profile_name).unwrap();
        }
    }

    /// Remove the specified profile from this Settings object
    pub fn remove_profile<S: Into<String>>(&mut self, profile_name: S) -> Result<()> {
        let profile_name: String = profile_name.into();
        if self.global_settings.get(c::DEFAULT_PROFILE_KEY) == Some(&profile_name) {
            Err(Error::illegal_operation(format!(
                "cannot remove the default profile '{}', set a new default first.",
                profile_name
            )))
        } else if !self.contains_profile(profile_name.clone()) {
            Err(Error::illegal_operation(format!(
                "profile not found: {}",
                profile_name
            )))
        } else {
            self.profiles.remove(&profile_name);
            Ok(())
        }
    }

    /// Set the default profile on this instance
    pub fn set_default_profile<S: Into<String>>(&mut self, profile_name: S) -> Result<()> {
        let profile_name: String = profile_name.into();
        if self.profiles.contains_key(&profile_name) {
            self.global_settings
                .insert(c::DEFAULT_PROFILE_KEY.into(), profile_name);
            Ok(())
        } else {
            Err(Error::illegal_operation(format!(
                "Could not set new default. Profile does not exist: {}",
                profile_name
            )))
        }
    }

    /// Get a list of profile names
    pub fn profile_names(&self) -> Vec<String> {
        self.profiles
            .keys()
            .map(|k| k.to_string())
            .filter(|k| k != c::ENVIRONMENT_OVERRIDE_PROFILE)
            .collect()
    }

    /// Check if this instance contains the given profile by name
    pub fn contains_profile<S: Into<String>>(&self, profile_name: S) -> bool {
        self.profile_names().contains(&profile_name.into())
    }

    /// Get the default profile
    pub fn default_profile(&self) -> ProfileConfig {
        let default_profile_key = &self.global_settings[c::DEFAULT_PROFILE_KEY];
        self.profiles[default_profile_key].clone()
    }

    /// Get a profile by name
    pub fn get_profile<S: Into<String>>(&self, profile_name: S) -> Option<ProfileConfig> {
        self.profiles.get(&profile_name.into()).cloned()
    }
}

/// Get a new profile name from the user. Will default to 'default' if
/// no such profile already exists, and will be rejected if the user
/// inputs an existing name or if the user inputs a restricted name
/// (like 'global').
fn new_profile_name(settings: &Settings) -> Result<String> {
    let default_name = "default";
    let prompt = "  Profile name:";
    let name = if settings.contains_profile(default_name) {
        user_input(prompt)
    } else {
        user_input_with_default(prompt, default_name)
    }?;

    if name.is_empty() {
        println!("Profile name cannot be empty.");
        new_profile_name(settings)
    } else if c::RESERVED_PROFILE_NAMES.contains(&&name[..]) {
        println!(
            "Profile name '{}' reserved for system. Please try a different name.",
            name
        );
        new_profile_name(settings)
    } else if settings.contains_profile(name.clone()) {
        println!("Profile already exists!");
        new_profile_name(settings)
    } else {
        Ok(name.to_string())
    }
}

/// Get the name of the profile to be used as the new default from the
/// user
fn new_default_profile(settings: &Settings) -> Result<String> {
    let current_default = settings.default_profile().profile;
    let potential_new_default = settings
        .profile_names()
        .clone()
        .into_iter()
        .find(|n| n != &current_default)
        .ok_or_else(|| {
            Error::illegal_operation("could not select a new default, no other profiles found")
        })?;

    let prompt = format!("  New default profile [{}]:", potential_new_default);
    let name = user_input(prompt)?;

    if name.is_empty() {
        Ok(potential_new_default)
    } else if !settings.contains_profile(name.clone()) {
        println!("Profile does not exist: {}", name);
        new_default_profile(settings)
    } else {
        Ok(name.to_string())
    }
}

/// Add a new profile to a settings instance using input from the
/// user, return the new profile.
pub fn create_profile_prompt(settings: &mut Settings) -> Result<ProfileConfig> {
    println!("Create a new profile:");

    let profile_name: String = new_profile_name(&settings)?;
    let token = user_input("  API token:")?;
    let secret = user_input("  API secret:")?;

    println!("Creating new profile: '{}'", profile_name);

    let profile = ProfileConfig::new(profile_name.clone(), token, secret);
    settings.add_profile(profile.clone());

    Ok(profile)
}

/// Delete a profile specified by the user
///
/// This is a destructive operation. It will modify the underlying
/// config.ini file to ensure that the profile is completely deleted.
pub fn delete_profile<S: Into<String>>(settings: &mut Settings, profile_name: S) -> Result<()> {
    let profile_name: String = profile_name.into();

    if settings.default_profile().profile == profile_name {
        println!("Cannot remove the default profile '{}'.", profile_name);
        let confirmation = format!(
            "{} {}",
            "Would you like to choose an existing profile to use as the default?",
            "Otherwise, we'll create a new profile."
        );

        let new_default = if settings.profile_names().len() > 1 {
            if confirm(confirmation)? {
                new_default_profile(settings)
            } else {
                create_profile_prompt(settings).map(|profile| profile.profile.clone())
            }
        } else {
            println!("Must create a new profile to use as the default.");
            create_profile_prompt(settings).map(|profile| profile.profile.clone())
        }?;

        settings.set_default_profile(new_default)?;
    }

    // remove it from the settings object
    settings.remove_profile(profile_name)
}

/// Set the specified profile as the new default.
pub fn set_default_profile<S: Into<String>>(
    settings: &mut Settings,
    profile_name: S,
) -> Result<()> {
    let profile_name: String = profile_name.into();

    if settings.default_profile().profile == profile_name {
        println!(
            "'{}' is already the default, no action taken.",
            profile_name
        );
        Ok(())
    } else {
        settings.set_default_profile(profile_name)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::ps::agent::config::Config;

    #[test]
    fn parse_valid_ini_single_profile() {
        let ini_str = r#"
            [global]
            default_profile = dev

            [dev]
            api_token = token
            api_secret = secret
        "#;
        let config = ini_str.parse::<Config>();
        assert!(config.is_ok());

        let settings = config.unwrap().api_settings;

        assert!(settings.profile_names() == vec!["dev"]);

        let expected = ProfileConfig::new("dev", "token", "secret");
        assert_eq!(settings.get_profile("dev").unwrap(), expected);
    }

    #[test]
    fn parse_valid_ini_multiple_profiles() {
        let ini_str = r#"
            [global]
            default_profile = dev

            [dev]
            api_token = token
            api_secret = secret
            environment = development

            [prod]
            api_token = prod_token
            api_secret = prod_secret
        "#;
        let settings: Settings = ini_str.parse::<Config>().unwrap().api_settings;
        let expected = Settings {
            profiles: [
                (
                    "prod".into(),
                    ProfileConfig::new("prod", "prod_token", "prod_secret"),
                ),
                (
                    "dev".into(),
                    ProfileConfig::new("dev", "token", "secret")
                        .with_environment("development".parse().unwrap()),
                ),
            ]
            .iter()
            .cloned()
            .collect(),
            global_settings: {
                let kv: HashMap<String, String> = [("default_profile".into(), "dev".into())]
                    .iter()
                    .cloned()
                    .collect();
                GlobalSettings::from(kv)
            },
            agent_settings: Default::default(),
        };

        assert_eq!(settings, expected);
    }

    #[test]
    fn fail_to_parse_valid_ini_no_default() {
        let ini_str = r#"
            [global]

            [dev]
            api_token = token
            api_secret = secret
            environment = development

            [prod]
            api_token = prod_token
            api_secret = prod_secret
        "#;
        let config = ini_str.parse::<Config>();
        assert!(config.is_err());
        assert!(config
            .err()
            .unwrap()
            .to_string()
            .contains("key not found: default_profile"));
    }

    #[test]
    fn fail_to_parse_valid_ini_with_invalid_default() {
        let ini_str = r#"
            [global]
            default_profile = doesnt_exist

            [dev]
            api_token = token
            api_secret = secret
        "#;
        let config = ini_str.parse::<Config>();
        assert!(config.is_err());
        assert!(config
            .err()
            .unwrap()
            .to_string()
            .contains("default profile not found: doesnt_exist"));
    }

    #[test]
    fn find_profile_that_exists() {
        let ini_str = r#"
            [global]
            default_profile = dev

            [dev]
            api_token = token
            api_secret = secret
        "#;
        let config = ini_str.parse::<Config>().unwrap();

        assert!(config.api_settings.contains_profile("dev"));
    }

    #[test]
    fn fail_to_find_profile_that_does_not_exist() {
        let ini_str = r#"
            [global]
            default_profile = dev

            [dev]
            api_token = token
            api_secret = secret
        "#;
        let config = ini_str.parse::<Config>().unwrap();

        assert!(!config.api_settings.contains_profile("prod"));
    }

    #[test]
    fn find_the_correct_default_profile() {
        let ini_str = r#"
            [global]
            default_profile = dev

            [dev]
            api_token = token
            api_secret = secret
        "#;
        let config = ini_str.parse::<Config>().unwrap();

        assert_eq!(config.api_settings.default_profile().profile, "dev");
    }

    #[test]
    fn add_profile_to_empty_settings_object() {
        let mut settings: Settings = Default::default();
        let new_profile = ProfileConfig::new("test", "token", "secret");

        assert!(settings.profile_names().is_empty());

        settings.add_profile(new_profile.clone());

        assert_eq!(settings.profile_names(), vec!["test"]);
        assert_eq!(settings.default_profile(), new_profile);
    }

    #[test]
    fn add_profile_to_populated_settings_object() {
        let ini_str = r#"
            [global]
            default_profile = dev

            [dev]
            api_token = token
            api_secret = secret
        "#;
        let mut config: Config = ini_str.parse().unwrap();
        let new_profile = ProfileConfig::new("test", "token", "secret");

        config.api_settings.add_profile(new_profile.clone());

        assert!(config.api_settings.contains_profile("test"));
    }
}
