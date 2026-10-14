# Configuration reference

All runtime behavior is loaded from a validated TOML file. Unknown or missing required fields fail startup rather than silently selecting hidden behavior.

## `server`

Listener address, public base URL, maximum body size, in-flight request limit, shutdown grace period, CORS origins, and credential policy.

## `routes`

Prefixes or paths for API, web interface, watch page, embed player (`embed`, for example `/embed`), oEmbed endpoint (`oembed`, for example `/oembed`), streaming assets, liveness, readiness, and OpenAPI. Routes must be unique, absolute, and free of traversal components.

## `api`

Default and maximum page sizes.

## `web`

Enable flag, shell template, asset directory and mount route, site name, tagline, repository/documentation/support links, locale, time zone, brand color, dashboard reporting range, dashboard/job refresh intervals, page-size choices, and page cache policy.

## `database`

SQLite URL, pool size, busy timeout, journal mode, and synchronous mode.

## `storage`

Durable root, temporary root, original/stream/trash directory names, and deleted-media retention policy.

## `streaming`

Independent cache policies for players, token redirects, playlists, segments, sources, and private assets, plus range-request support.

## `registration`

Mode, optional administrator token/header, default quota, and account/API-key field limits.

## `browser_auth`

Password pepper and Argon2id parameters; password limits; session signing key; absolute and idle TTLs; maximum sessions; cookie name/path/domain/security/HttpOnly/SameSite attributes; CSRF header; cleanup interval.

## `security`

API-key prefix, pepper and Argon2 parameters; playback signing key and token lifetime.

## `uploads`

Minimum/maximum source size, text and filename limits, fallback names and content type, default visibility, and accepted content types.

## `remote_imports`

Enable flag, private-network policy, maximum size, request timeout, redirect cap, user agent, and system-proxy policy.

## `jobs`

Worker concurrency, poll interval, lease duration, lease-renewal interval, maximum attempts, and retry base delay.

## `transcoding`

FFmpeg/FFprobe paths, command timeout, HLS segment duration/format/version, output names and patterns, playlist flags, scaling, audio layout, thread count, original retention, upscale policy, extra arguments, and any number of rendition profiles.

## `analytics`

Enable flag, viewer hash key, visitor/playback/session cookie settings, heartbeat controls, reporting limits, raw/session retention, cleanup interval, optional byte-event retention, and the qualified-view rule (`qualified_view_seconds`, recommended 30; `qualified_view_minimum_fraction`, recommended 0.5).

## `player`

HTML template and optional hls.js URL.

## `observability`

Tracing filter and JSON output selection.

## `protection`

Per-client stream request rate and burst, playback-session rate, concurrent stream responses, per-response byte pacing for segments and originals, a global egress cap, cross-site (hotlink) blocking, whether viewers may fetch the original when HLS exists, and the trusted client-address header. Recommended values (example configuration): 30 requests/s with a burst of 300, 60 sessions/min, 16 concurrent streams, originals paced at 2.5 MB/s after a 16 MB burst, segments unpaced, hotlink blocking on, originals hidden once HLS renditions exist. `0` disables a limit. See [Playback protection](/guide/protection).

## `embed`

Enable flag, origins allowed to frame the embed player (`["*"]` allows any site), and the default embed size used by oEmbed and the embed-code builder.

## `geolocation`

Both fields are optional: `database_path` to a MaxMind DB country database (DB-IP Lite, GeoLite2 Country, or IPinfo Lite) and/or `country_header` naming a trusted CDN country header. Without either, countries are reported as unknown (`ZZ`).

See [`config/gulfstream.example.toml`](https://github.com/gulfstream-rs/gulfstream/blob/main/config/gulfstream.example.toml) for every field and a complete set of values.
