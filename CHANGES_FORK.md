# Fork Changes

> _Changes added in this fork — maintained by [panda260](https://github.com/panda260/prefetcharr)._
> _AI-assisted implementation (Google Deepmind Antigravity)._

---

## Multi-Instance Sonarr Support

Multiple Sonarr instances can now be configured simultaneously. Each instance
can be assigned a **path prefix** that prefetcharr uses to route playback events
to the correct Sonarr instance (e.g. one instance for 1080p, another for 4K).

### Configuration

```toml
# Sonarr Instance 1 (1080p)
[[sonarr]]
url     = "http://sonarr-hd:8989"
api_key = "<KEY>"
path    = "/nas/media/series/"

# Sonarr Instance 2 (4K)
[[sonarr]]
url     = "http://sonarr-4k:8989"
api_key = "<KEY>"
path    = "/nas/media/series-4k/"
```

> **Path matching**: When a media-server playback event carries a file path,
> prefetcharr selects the Sonarr instance whose `path` is the longest matching
> prefix of that file path. If no path is configured (or no match is found),
> the first instance is used as a fallback.

### Environment variable style (alternative)

```
SONARR_1_URL=http://sonarr-hd:8989
SONARR_1_API_KEY=<KEY>
SONARR_2_URL=http://sonarr-4k:8989
SONARR_2_API_KEY=<KEY>
```

---

## Controlled Season Monitoring (`controlled_season_monitoring`)

By default Sonarr's **Monitor New Items** setting is left on `all`, which means
every time a new season is announced it gets automatically monitored and
downloaded — even if the user has not yet watched the previous seasons.

This setting gives you control over that behaviour.

### Values

| Value | Behaviour |
|-------|-----------|
| `"off"` | **Default.** Original behaviour: `monitorNewItems` in Sonarr is never changed by prefetcharr. |
| `"on_demand"` | When prefetcharr detects a user is near the end of a season, it limits Sonarr to monitoring **only one season ahead**. No other seasons are auto-added. |
| `"all"` | Same as `on_demand`, **plus** a one-time startup sweep that immediately sets `monitorNewItems = none` on **all** series across all configured Sonarr instances. |

### How `on_demand` / `all` works at runtime

1. User watches episode N of season S.  
   prefetcharr checks the next `prefetch_num` episodes.
2. **Enough episodes exist** → nothing changes, normal search.
3. **Too few episodes** (end of a season):
   - Next season (S+1) **already in Sonarr** → `monitorNewItems = none`  
     _(one season ahead is enough; don't pull in S+2 automatically)_
   - Next season **not yet in Sonarr** → `monitorNewItems = all` temporarily, **and** the series gets tagged with `prefetcharr-awaiting-season` in Sonarr.  
     _(allows Sonarr to discover the season on its next metadata refresh)_  
     
#### Persistent Background Cleanup
Because users might stop watching a show before the next season is announced, leaving `monitorNewItems = all` indefinitely would cause all future seasons (S+2, S+3) to download automatically even years later.
To prevent this, prefetcharr uses **Sonarr Tags** to store state persistently across Docker restarts:
- Every `interval` (e.g. 300s), prefetcharr runs a background sweep.
- It asks Sonarr for all series containing the `prefetcharr-awaiting-season` tag.
- For each tagged series, it checks if the awaited season has finally appeared.
- If it has, it reverts `monitorNewItems = none` and removes the tag.
### Configuration

```toml
controlled_season_monitoring = "off"       # default, safe, no changes
# controlled_season_monitoring = "on_demand"  # active users only
# controlled_season_monitoring = "all"        # startup sweep + active users
```

In `docker-compose.yml` / `PREFETCHARR_CONFIG`:
```toml
controlled_season_monitoring = "on_demand"
```
