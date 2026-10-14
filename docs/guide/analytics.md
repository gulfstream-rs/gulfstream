# Analytics

Analytics are derived from persisted playback activity, not generated sample data.

## Metrics

- **Views**: first accepted `play` event for a playback session.
- **Unique viewers**: exact per-account, per-media, per-day deduplication using a keyed visitor hash.
- **Play starts**: first play transition.
- **Completed views**: first completion transition.
- **Watch time**: accepted heartbeat deltas, bounded by the configured maximum.
- **Bytes served**: bytes actually emitted by source, playlist, and segment response streams.
- **Qualified views**: sessions whose server-measured watch time reached `analytics.qualified_view_seconds` (default 30 s) or `qualified_view_minimum_fraction` of shorter videos, counted once per viewer, media, and day. This is the unit intended for monetization; see [Monetization](/guide/monetization).
- **Countries**: views, qualified views, and watch time per ISO country code (`ZZ` = unknown).

The player creates a playback session when the watch page is opened, but that alone does not count as a view. Events are serialized per session so duplicate play/completion transitions are not double-counted.

## Countries

The country is resolved once, when a playback session starts, and only the two-letter code is stored. Sources, in order:

1. A trusted CDN header named by `geolocation.country_header` (for example Cloudflare `CF-IPCountry`).
2. A local MaxMind DB file at `geolocation.database_path`. Free options:
   - [DB-IP IP to Country Lite](https://db-ip.com/db/download/ip-to-country-lite) — CC BY 4.0, no account, monthly; attribution ("IP Geolocation by DB-IP") is required.
   - [MaxMind GeoLite2 Country](https://dev.maxmind.com/geoip/geolite2-free-geolocation-data) — free account and license key, GeoLite2 EULA; databases must be refreshed within 30 days of a new release.
   - [IPinfo Lite](https://ipinfo.io/lite) — CC BY-SA 4.0, free token, country and ASN.

Download the `.mmdb` file monthly (cron or `geoipupdate` for GeoLite2) and restart Gulfstream to load the new file. Country accuracy for these databases is typically above 98 %. VPN and proxy traffic is attributed to the exit country.

## Queries

The summary, time-series, and countries endpoints accept optional `from`, `to`, and `media_id` parameters. Reporting length is bounded by configuration. Daily time series include zero-value dates for convenient charting.

## Retention

Raw events and old playback sessions have independent retention periods. Daily aggregates and exact daily unique counts remain available after raw-data cleanup. Set a retention period to zero to retain that class indefinitely.
