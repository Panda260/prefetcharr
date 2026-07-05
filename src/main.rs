#![warn(clippy::pedantic)]

use std::{
    fmt::Display,
    fs::read_to_string,
    io::{IsTerminal, stderr},
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use anyhow::Context;
use clap::{Parser, ValueEnum};
use config::Config;
use futures::{StreamExt as _, TryStreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_util::sync::PollSender;
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{config::ControlledSeasonMonitoring, media_server::plex, util::once::Seen};

mod config;
#[cfg(test)]
mod fake_sonarr;
mod filter;
mod media_server;
mod process;
mod sonarr;
mod util;

use media_server::embyfin;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to config file
    #[arg(long)]
    config: PathBuf,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct LegacyArgs {
    /// Media server type
    #[arg(long, default_value = "jellyfin")]
    media_server_type: MediaServer,
    /// Jellyfin/Emby/Plex/Tautulli baseurl
    #[arg(long, value_name = "URL")]
    media_server_url: String,
    /// Jellyfin/Emby/Tautulli API key or Plex server token
    #[arg(long, value_name = "API_KEY", env = "MEDIA_SERVER_API_KEY")]
    media_server_api_key: String,
    /// Sonarr baseurl
    #[arg(long, value_name = "URL")]
    sonarr_url: String,
    /// Sonarr API key
    #[arg(long, value_name = "API_KEY", env = "SONARR_API_KEY")]
    sonarr_api_key: String,
    /// Polling interval
    #[arg(long, value_name = "SECONDS", default_value_t = 900)]
    interval: u64,
    /// Logging directory
    #[arg(long)]
    log_dir: Option<PathBuf>,
    /// The last <NUM> episodes trigger a search
    #[arg(long, value_name = "NUM", default_value_t = 2)]
    remaining_episodes: u8,
    /// User IDs or names to monitor episodes for (default: empty/all users)
    ///
    /// Each entry here is checked against the user's ID and name
    #[arg(long, value_name = "USER", value_delimiter = ',', num_args = 0..)]
    users: Vec<String>,
    /// Number of retries for the initial connection probing
    #[arg(long, value_name = "NUM", default_value_t = 0)]
    connection_retries: usize,
    /// Library names to monitor episodes for. (default: empty/all libraries)
    #[arg(long, value_name = "LIBRARY", value_delimiter = ',', num_args = 0..)]
    libraries: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, ValueEnum)]
enum MediaServer {
    Jellyfin,
    Emby,
    Plex,
    Tautulli,
}

impl Display for MediaServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            MediaServer::Jellyfin => "Jellyfin",
            MediaServer::Emby => "Emby",
            MediaServer::Plex => "Plex",
            MediaServer::Tautulli => "Tautulli",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Message {
    NowPlaying(media_server::NowPlaying),
}

fn config() -> anyhow::Result<Config> {
    let mut config = if let Ok(args) = LegacyArgs::try_parse() {
        Config::from(args)
    } else {
        let args = Args::parse();
        let toml = read_to_string(args.config.as_path())
            .with_context(|| format!("reading config from {}", args.config.to_string_lossy()))?;
        toml::from_str(&toml).context("parsing TOML config")?
    };

    let mut env_sonarrs = std::collections::BTreeMap::new();
    for (key, value) in std::env::vars() {
        if let Some(rest) = key.strip_prefix("SONARR_") {
            let parts: Vec<&str> = rest.splitn(2, '_').collect();
            if parts.len() == 2 {
                if let Ok(id) = parts[0].parse::<u32>() {
                    let prop = parts[1];
                    let entry = env_sonarrs.entry(id).or_insert_with(|| config::Sonarr {
                        url: String::new(),
                        api_key: String::new(),
                        exclude_tag: None,
                        path: None,
                    });
                    match prop {
                        "URL" => entry.url = value,
                        "API_KEY" => entry.api_key = value,
                        "PATH" => entry.path = Some(value),
                        _ => {}
                    }
                }
            }
        }
    }

    for (_, sonarr) in env_sonarrs {
        if !sonarr.url.is_empty() && !sonarr.api_key.is_empty() {
            config.sonarr.push(sonarr);
        }
    }

    Ok(config)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config()?;

    enable_logging(config.log_dir.as_ref(), config.log_level);

    info!("{NAME} {VERSION}");

    if config.legacy {
        warn!(
            "Legacy configuration method detected. \
            Migrate to the new TOML config to avoid future problems."
        );
    }

    if let Err(e) = run(config).await {
        error!("{e:#}");
        info!("{NAME} exits due to an error");
        return Err(e.into());
    }

    Ok(())
}

async fn run(config: Config) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel(1);

    let mut sonarr_clients = Vec::new();
    for s_conf in &config.sonarr {
        let client = sonarr::Client::new(&s_conf.url, &s_conf.api_key)
            .context("Invalid connection parameters for Sonarr")?;
        util::retry(config.connection_retries, async || {
            client.probe().await.context("Probing Sonarr failed")
        })
        .await?;
        let mut tag_id = None;
        if matches!(
            config.controlled_season_monitoring,
            ControlledSeasonMonitoring::OnDemand | ControlledSeasonMonitoring::All
        ) {
            match client.get_or_create_tag("prefetcharr-awaiting-season").await {
                Ok(id) => tag_id = Some(id),
                Err(e) => warn!("Failed to get or create tag for Sonarr: {e:#}"),
            }
        }

        sonarr_clients.push((s_conf.path.clone(), s_conf.exclude_tag.clone(), client, tag_id));
    }

    if sonarr_clients.is_empty() {
        return Err(anyhow::anyhow!("No Sonarr instances configured"));
    }

    // Startup sweep: when mode is "all", immediately set monitorNewItems = none
    // for every series across all Sonarr instances. Errors are non-fatal so a
    // single unreachable series doesn't abort startup.
    if config.controlled_season_monitoring == ControlledSeasonMonitoring::All {
        info!("controlled_season_monitoring=all: running startup sweep");
        for (_, _, client, _) in &sonarr_clients {
            match client.series().await {
                Ok(all_series) => {
                    for mut s in all_series {
                        if let Err(e) = client.set_monitor_new_items_none(&mut s).await {
                            warn!(
                                series_id = s.id,
                                title = ?s.title,
                                "startup sweep: failed to update series: {e:#}"
                            );
                        }
                    }
                }
                Err(e) => warn!("startup sweep: failed to fetch series: {e:#}"),
            }
        }
        info!("controlled_season_monitoring=all: startup sweep complete");
    }

    info!("Start watching {} sessions", config.media_server.r#type);
    let interval = Duration::from_secs(config.interval);
    let (client, queue): (
        Arc<dyn media_server::Client>,
        Option<Arc<dyn media_server::Queue + Send + Sync>>,
    ) = match config.media_server.r#type {
        MediaServer::Emby | MediaServer::Jellyfin => {
            let c = Arc::new(
                embyfin::Client::new(
                    &config.media_server.url,
                    &config.media_server.api_key,
                    config.media_server.r#type.try_into()?,
                )
                .context("Invalid connection parameters")?,
            );
            let queue: Option<Arc<dyn media_server::Queue + Send + Sync>> = config
                .append_to_queue
                .then(|| c.clone() as Arc<dyn media_server::Queue + Send + Sync>);
            (c as Arc<dyn media_server::Client>, queue)
        }
        MediaServer::Plex => {
            let c = Arc::new(
                plex::Client::new(&config.media_server.url, &config.media_server.api_key)
                    .context("Invalid connection parameters")?,
            );
            let queue: Option<Arc<dyn media_server::Queue + Send + Sync>> = config
                .append_to_queue
                .then(|| c.clone() as Arc<dyn media_server::Queue + Send + Sync>);
            (c as Arc<dyn media_server::Client>, queue)
        }
        MediaServer::Tautulli => {
            let c = Arc::new(
                media_server::tautulli::Client::new(
                    &config.media_server.url,
                    &config.media_server.api_key,
                )
                .context("Invalid connection parameters")?,
            );
            if config.append_to_queue {
                warn!(
                    "append_to_queue is not supported with Tautulli (read-only). \
                     The feature will be a no-op for this backend."
                );
            }
            (c as Arc<dyn media_server::Client>, None)
        }
    };

    client.probe_with_retry(config.connection_retries).await?;

    let has_pending = Arc::new(AtomicBool::new(false));
    let pending_ttl = interval.saturating_mul(2) + Duration::from_secs(60);
    let sink = PollSender::new(tx);
    let np_updates = client
        .now_playing_updates(interval, has_pending.clone())
        .inspect_err(|err| error!("Cannot fetch sessions from media server: {err}"))
        .filter_map(async |res| res.ok()) // remove errors
        .filter(filter::users(config.media_server.users.as_slice()))
        .filter(filter::libraries(config.media_server.libraries.as_slice()))
        .map(Message::NowPlaying)
        .map(Ok) // align with the error type of `PollSender`
        .forward(sink);

    let seen = Seen::default();
    let mut actor = process::Actor::new(
        rx,
        sonarr_clients,
        seen,
        config.prefetch_num,
        config.request_seasons,
        config.controlled_season_monitoring,
        queue,
        has_pending,
        pending_ttl,
    );

    let _ = tokio::join!(np_updates, actor.process(), client.run());

    Ok(())
}

fn enable_logging(log_dir: Option<&PathBuf>, level: Option<config::LogLevel>) {
    // SubscriberBuilder defaults to a baked-in INFO max-level cap; without
    // this override, a Targets/EnvFilter that allows DEBUG would still be
    // gated by the inner cap and DEBUG events would never reach the writer.
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_ansi(stderr().is_terminal())
        .with_writer(stderr)
        .with_max_level(tracing_subscriber::filter::LevelFilter::TRACE)
        .finish();

    let is_none = matches!(level, Some(config::LogLevel::None));

    let filter = if is_none {
        tracing_subscriber::filter::Targets::new().boxed()
    } else if let Some(config::LogLevel::Debug) = level {
        tracing_subscriber::filter::Targets::new()
            .with_target(env!("CARGO_PKG_NAME"), tracing::Level::TRACE)
            .boxed()
    } else {
        EnvFilter::builder().from_env_lossy().boxed()
    };

    let rolling_layer = log_dir.as_ref().map(|log_dir| {
        let file_appender = tracing_appender::rolling::daily(log_dir, "prefetcharr.log");
        tracing_subscriber::fmt::layer()
            .compact()
            .with_ansi(false)
            .with_writer(file_appender)
    });

    subscriber
        .with(filter)
        .with(rolling_layer)
        .try_init()
        .expect("setting the default subscriber");
}
