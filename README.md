# prefetcharr

Have [Sonarr][sonarr] automatically fetch the next episodes of the show you're
watching on [Jellyfin][jellyfin]/[Emby][emby]/[Plex][plex].

---

## ✨ Features added in this fork

> _This is a fork of [p-hueber/prefetcharr](https://github.com/p-hueber/prefetcharr)
> maintained by [panda260](https://github.com/panda260/prefetcharr).
> The additions below were implemented with AI assistance (Google Deepmind Antigravity)._

### Multi-Instance Sonarr Support

Run prefetcharr against **multiple Sonarr instances** at the same time (e.g. one
for 1080p, one for 4K). Each instance is matched to playback events by a
configurable `path` prefix.

```toml
[[sonarr]]
url     = "http://sonarr-hd:8989"
api_key = "<KEY>"
path    = "/nas/media/series/"

[[sonarr]]
url     = "http://sonarr-4k:8989"
api_key = "<KEY>"
path    = "/nas/media/series-4k/"
```

See [CHANGES_FORK.md](CHANGES_FORK.md) for the full documentation.

### Controlled Season Monitoring (`controlled_season_monitoring`)

**The Problem:** By default, `prefetcharr` tells Sonarr to automatically monitor **all new seasons** of a series (`Monitor New Seasons = All`) as soon as you finish watching the currently available episodes. This causes Sonarr to immediately download new seasons as soon as they are announced – even if you are only on Season 1 and Season 5 was just announced.

This setting allows you to strictly control this behavior and save disk space.

#### The 3 Configuration Options:

1. **`"off"` (Default Setting)**
   - **How it works:** Everything stays the same. `prefetcharr` does not change Sonarr's behavior.
   - **Result:** When you run out of episodes, `prefetcharr` sets the series in Sonarr to "Monitor All New Seasons". Every future season will be downloaded automatically.

2. **`"on_demand"` (Recommended for saving space)**
   - **How it works:** `prefetcharr` ensures that only **exactly one season in advance** is monitored, based on what you are currently watching. 
   - **Example:** You are currently watching *Season 2*. `prefetcharr` checks if *Season 3* exists. If yes, it tells Sonarr: *"Do not monitor any more new seasons (Monitor New Seasons = None)"*, because Season 3 is completely sufficient. Season 4 or 5 will then **not** be downloaded automatically anymore.
   - **Note:** This happens "on demand", meaning only for series that are currently actually being watched by someone.

3. **`"all"` (The radical cleaner)**
   - **How it works:** Does exactly the same as `"on_demand"`, but with an extra step when the program starts.
   - **Additional Feature:** Every time the `prefetcharr` container restarts, it goes through **your entire Sonarr library** (all series) and strictly sets "Monitor New Seasons" to "None" everywhere.
   - **Result:** Not a single series in your Sonarr will automatically download new seasons anymore. But as soon as you start watching a series, the `"on_demand"` logic kicks back in and always downloads exactly the next needed season for this one series.

**Configuration (e.g., in `docker-compose.yml`):**
```toml
# In PREFETCHARR_CONFIG:
controlled_season_monitoring = "on_demand"
```

#### The exact logic in the background (Step-by-Step):

So you understand exactly what the program decides under the hood, here is the precise flowchart when `"on_demand"` (or `"all"`) is activated:

1. **Someone watches a series (e.g., Season 2, Episode 9).**
   `prefetcharr` calculates that, for example, 3 episodes are needed next (S2E10, S3E1, S3E2).

2. **Are episodes missing?**
   - **No:** Everything is great. Sonarr has enough episodes ready. Nothing happens.
   - **Yes:** `prefetcharr` notices that it has reached the end of the available episodes. Now the logic triggers!

3. **Checking the highest monitored season:**
   `prefetcharr` looks into your Sonarr and asks: *"What is the highest season of this series that is currently monitored by Sonarr?"*
   *(In our example you are at Season 2, so Season 2 is the highest monitored season).*

4. **The decision for the *next* season (Season 3):**
   Now the program checks if Sonarr **even knows** the "next" season (Season 3) at all (regardless of whether it's already downloaded or not, it just has to be in the Sonarr overview).
   
   - **Condition A (The next season already exists in Sonarr):**
     - **Action:** `Monitor New Seasons` is set to `"None"`.
     - **Why?** Sonarr already knows about Season 3. That is completely sufficient for your next TV night. By setting it to "None", we prevent Sonarr from suddenly starting to automatically download Season 4 or Season 5 if they are announced somewhere on the internet.
   
   - **Condition B (The next season is completely unknown to Sonarr):**
     - **Action:** `Monitor New Seasons` is **temporarily** set to `"All"`.
     - **Storage (Docker-Safe):** So that `prefetcharr` doesn't forget this (even if you delete or restart the Docker container), it tags the series **in Sonarr** with `prefetcharr-awaiting-season`. The info is therefore safely stored in the Sonarr database.
     - **Why?** The series might currently only have 2 seasons. But we *want* Sonarr to notice if a 3rd season is published at some point. By using "All", Sonarr is allowed to look out for new seasons.
     - **The Climax (The Background Check):** `prefetcharr` asks Sonarr in the background every few minutes for series with this tag. As soon as TVDB adds the 3rd season, Sonarr finds it. Our background check notices this, triggers **Condition A** again, safely sets the whole thing back to `"None"` and deletes the tag. Even if you never watch the series again, Sonarr is guaranteed not to download any unwanted seasons 4, 5, etc.

See [CHANGES_FORK.md](CHANGES_FORK.md) for a technical summary.

---

## Details

_prefetcharr_ periodically polls your media server for active playback sessions
of TV shows.
It then checks whether a configured number of successive episodes is available.
If there are episodes missing, it asks _Sonarr_ to search for them.
Depending on the configuration, it searches the missing episodes individually or
tries to fetch all seasons that contain them.
If there are no more seasons left, the series is monitored for new seasons
instead.

## Build and install

To install, first ensure Rust is installed on your system by following the
instructions at [Install Rust][rust], then run:
```
cargo install --git https://github.com/p-hueber/prefetcharr
```

Or with docker compose:
```yml
services:
  prefetcharr:
    image: ghcr.io/panda260/prefetcharr:latest
    container_name: prefetcharr
    environment:
      - |
        PREFETCHARR_CONFIG=
        # Start of the configuration in TOML format.
       
        interval = 900           # Polling interval in seconds
        log_dir = "/log"         # Logging directory
        log_level = "debug"      # `none` or `debug`
        prefetch_num = 2         # Number of episodes to make available in advance
        request_seasons = true   # Always request full seasons to prefer season packs
        append_to_queue = false  # Experimental: Append upcoming episodes to the player's active queue.
                                 # Not supported by all clients. Not compatible with Tautulli.
        connection_retries = 6   # Number of retries for the initial connection probing

        [media_server]
        type = "Jellyfin"                       # `Jellyfin`, `Emby`, `Plex` or `Tautulli`
        url = "http://example.com/jellyfin"     # Jellyfin/Emby/Plex/Tautulli baseurl
        api_key = "<YOUR KEY HERE>"             # Jellyfin/Emby/Tautulli API key or plex server token
        # users = [ "John", "12345", "Axel F" ] # Optional: Only monitor sessions for specific user IDs or names
        # libraries = [ "TV Shows", "Anime" ]   # Optional: Only monitor sessions for specific libraries

        # Sonarr instance 1
        [[sonarr]]
        url = "http://example.com/sonarr" # Sonarr baseurl
        api_key = "<YOUR KEY HERE>"       # Sonarr API key
        # exclude_tag = "no_prefetch"     # Optional: Exclude series by tag
        # path = "/path/to/series/" # Optional: Match Jellyfin/Emby path prefix

        # Sonarr instance 2 (Optional)
        # [[sonarr]]
        # url = "http://example.com/sonarr-4k"
        # api_key = "<YOUR KEY HERE>"
        # path = "/path/to/series-4k/"

    volumes:
      - /path/to/log/dir:/log
      # Keep the config in a file instead of PREFETCHARR_CONFIG
      # - /path/to/config.toml:/config

```

## Configuration

The configuration is written in [TOML][toml] format. When running `prefetcharr`
directly, you can pass the path of the configuration file using the `--config`
command-line flag. For the Docker container, provide the entire configuration
via the `PREFETCHARR_CONFIG` environment variable.
A complete example can be found in the installation instructions for
`docker-compose` above.

### API keys

_prefetcharr_ needs two different API keys to do its job.

#### `sonarr.api_key`

Go to `Settings` -> `General` -> `Security` and copy the API key.

#### `media_server.api_key`

The key to use and how to obtain it differs on the type of media server you use:

#### Jellyfin

Log in as an administrator and go to `Administration` -> `Dashboard` ->
`Advanced` -> `Api Keys`. Add a new key or use an existing one.

#### Emby

Log in as an administrator, click on the gear on the top right and go to
`Advanced` -> `Api Keys`. Add a new key or use an existing one.

#### Plex

You need to [extract the server token][plex-token] from a configuration file and
use it as the API key.

#### Plex via Tautulli

Log in and go to `Settings` -> `Web Interface` -> API. Copy the key and make
sure `Enable API` is ticked.


### Upgrading pilots

If you want to store pilot episodes only, _prefetcharr_ can fetch the first
season for you on demand.  
This method works well for individual episodes but may encounter issues with
season packs.  
For this to function in _Sonarr_, grabbing the season pack must be considered
an upgrade of the pilot episode.
This can be achieved through a [custom format][custom-format].
[Import][format-import] the custom format and
[configure a quality profile][quality-profile] to prefer it.

## How to use

### Host installation

If you installed _prefetcharr_ through `cargo`, you can get a description of the
command-line interface by running `prefetcharr --help`.

### Docker installation

Users utilizing Docker only need to start the container, e.g. using `docker
compose up -d prefetcharr`.
Once the container is running, you may want to check the logs for errors. You
can do so by either calling `docker logs prefetcharr` or by checking the logging
directory you configured.


[sonarr]: <https://sonarr.tv>
[jellyfin]: <https://jellyfin.org>
[emby]: <https://emby.media>
[plex]: <https://www.plex.tv>
[tautulli]: <https://tautulli.com/>
[rust]: <https://www.rust-lang.org/tools/install>
[toml]: <https://toml.io/en/>
[plex-token]: <https://www.plexopedia.com/plex-media-server/general/plex-token/#plexservertoken>
[custom-format]: <https://trash-guides.info/Sonarr/sonarr-collection-of-custom-formats/#season-pack>
[format-import]: <https://trash-guides.info/Sonarr/sonarr-import-custom-formats/>
[quality-profile]: <https://trash-guides.info/Sonarr/sonarr-setup-quality-profiles/>
