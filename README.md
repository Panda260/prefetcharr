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

**Das Problem:** Standardmäßig weist `prefetcharr` Sonarr an, künftig **alle neuen Staffeln** einer Serie automatisch zu überwachen (`Monitor New Seasons = All`), sobald du die aktuell verfügbaren Episoden zu Ende geschaut hast. Das führt dazu, dass Sonarr sofort neue Staffeln herunterlädt, sobald sie angekündigt werden – selbst wenn du z. B. erst bei Staffel 1 bist und gerade Staffel 5 angekündigt wurde.

Mit dieser Einstellung kannst du dieses Verhalten exakt steuern und Festplattenplatz sparen.

#### Die 3 Einstellungs-Möglichkeiten:

1. **`"off"` (Standard-Einstellung)**
   - **Wie es funktioniert:** Alles bleibt beim Alten. `prefetcharr` ändert das Verhalten von Sonarr nicht.
   - **Ergebnis:** Wenn dir die Folgen ausgehen, stellt `prefetcharr` die Serie in Sonarr auf "Monitor All New Seasons". Jede künftige Staffel wird automatisch heruntergeladen.

2. **`"on_demand"` (Empfohlen für Platzsparer)**
   - **Wie es funktioniert:** `prefetcharr` sorgt dafür, dass immer nur **genau eine Staffel im Voraus** überwacht wird, basierend darauf, was du gerade schaust. 
   - **Beispiel:** Du schaust gerade *Staffel 2*. `prefetcharr` prüft, ob *Staffel 3* existiert. Wenn ja, sagt es Sonarr: *"Überwache keine weiteren neuen Staffeln mehr (Monitor New Seasons = None)"*, denn Staffel 3 reicht völlig aus. Staffel 4 oder 5 werden dann **nicht** mehr automatisch heruntergeladen.
   - **Hinweis:** Dies passiert "on demand", also immer nur für die Serien, die aktuell auch wirklich von jemandem geschaut werden.

3. **`"all"` (Der radikale Aufräumer)**
   - **Wie es funktioniert:** Macht genau dasselbe wie `"on_demand"`, aber mit einem extra Schritt beim Starten des Programms.
   - **Zusatz-Feature:** Jedes Mal, wenn der `prefetcharr`-Container neu startet, geht er **deine komplette Sonarr-Bibliothek** (alle Serien) durch und stellt "Monitor New Seasons" überall hart auf "None". 
   - **Ergebnis:** Keine einzige Serie in deinem Sonarr lädt mehr automatisch neue Staffeln herunter. Sobald du aber anfängst, eine Serie zu schauen, greift wieder die `"on_demand"`-Logik und lädt für diese eine Serie immer genau die nächste benötigte Staffel herunter.

**Konfiguration (z.B. in der `docker-compose.yml`):**
```toml
# In PREFETCHARR_CONFIG:
controlled_season_monitoring = "on_demand"
```

See [CHANGES_FORK.md](CHANGES_FORK.md) for a detailed technical explanation of the logic.

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
        log_level = "Debug"      # `Trace`, `Debug`, `Info`, `Warn` or `Error`
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
