use std::path::PathBuf;

use serde::{Deserialize, Deserializer};

use crate::LegacyArgs;

#[derive(Deserialize)]
pub struct MediaServer {
    /// Media server type
    pub r#type: crate::MediaServer,
    /// Jellyfin/Emby/Plex/Tautulli baseurl
    pub url: String,
    /// Jellyfin/Emby/Tautulli API key or Plex server token
    pub api_key: String,
    /// User IDs or names to monitor episodes for (default: empty/all users)
    #[serde(default)]
    pub users: Vec<String>,
    /// Library names to monitor episodes for. (default: empty/all libraries)
    #[serde(default)]
    pub libraries: Vec<String>,
}

#[derive(Deserialize)]
pub struct Sonarr {
    /// Sonarr baseurl
    pub url: String,
    /// Sonarr API key
    pub api_key: String,
    /// Exclude series by tag
    pub exclude_tag: Option<String>,
    /// Optional prefix to match media paths to this Sonarr instance
    pub path: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SingleOrVec<T> {
    Single(T),
    Vec(Vec<T>),
}

fn deserialize_single_or_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    match SingleOrVec::<T>::deserialize(deserializer)? {
        SingleOrVec::Single(s) => Ok(vec![s]),
        SingleOrVec::Vec(v) => Ok(v),
    }
}

#[derive(Clone, Copy, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for tracing::Level {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

/// Controls whether prefetcharr limits Sonarr to monitoring only one season
/// ahead, preventing automatic monitoring of all future seasons.
///
/// - `"off"` (default): disabled, original Sonarr behaviour is untouched.
/// - `"on_demand"`: applied only when prefetcharr actively processes a series
///   (i.e. a user is currently watching it).
/// - `"all"`: on startup, all series in Sonarr are set to `monitorNewItems =
///   none` immediately; afterwards `on_demand` rules apply.
#[derive(Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ControlledSeasonMonitoring {
    /// Disabled — original behaviour, `monitorNewItems` is never changed by
    /// this feature.
    #[default]
    Off,
    /// Only applied when prefetcharr actively processes a series.
    OnDemand,
    /// On startup, all series are immediately set to `monitorNewItems = none`,
    /// then `on_demand` rules apply for future scans.
    All,
}

#[derive(Deserialize)]
pub struct Config {
    pub media_server: MediaServer,
    #[serde(deserialize_with = "deserialize_single_or_vec")]
    pub sonarr: Vec<Sonarr>,
    /// Polling interval
    pub interval: u64,
    /// Logging directory
    pub log_dir: Option<PathBuf>,
    /// Log level
    pub log_level: Option<LogLevel>,
    /// Number of episodes to make available in advance
    pub prefetch_num: usize,
    /// Always request full seasons to prefer season packs
    pub request_seasons: bool,
    /// Number of retries for the initial connection probing
    pub connection_retries: usize,
    /// Append upcoming episodes to the active player queue
    #[serde(default)]
    pub append_to_queue: bool,
    /// Controls season-ahead monitoring behaviour. See `ControlledSeasonMonitoring`.
    #[serde(default)]
    pub controlled_season_monitoring: ControlledSeasonMonitoring,
    #[serde(default)]
    pub legacy: bool,
}

impl From<LegacyArgs> for Config {
    fn from(
        LegacyArgs {
            media_server_type,
            media_server_url,
            media_server_api_key,
            sonarr_url,
            sonarr_api_key,
            interval,
            log_dir,
            remaining_episodes,
            users,
            connection_retries,
            libraries,
        }: LegacyArgs,
    ) -> Self {
        let media_server = MediaServer {
            r#type: media_server_type,
            url: media_server_url,
            api_key: media_server_api_key,
            users,
            libraries,
        };
        let sonarr = Sonarr {
            url: sonarr_url,
            api_key: sonarr_api_key,
            exclude_tag: None,
            path: None,
        };
        Config {
            media_server,
            sonarr: vec![sonarr],
            interval,
            log_dir,
            log_level: None,
            prefetch_num: remaining_episodes.into(),
            request_seasons: true,
            connection_retries,
            append_to_queue: false,
            controlled_season_monitoring: ControlledSeasonMonitoring::Off,
            legacy: true,
        }
    }
}
