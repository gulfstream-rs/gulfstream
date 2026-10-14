# Playback protection

Anything a browser can play, a determined user can save: screen capture and
stream-ripping tools work against every web player, including those with DRM
(DRM only stops copying of the decrypted file, not recording the screen). The
realistic goals are to make bulk downloading and hotlinking unattractive, to
keep any single client from saturating the uplink, and to keep view counts
honest. Gulfstream implements these layers; all settings live in the
`[protection]` section.

## What Gulfstream enforces

| Layer | Setting | Effect |
| --- | --- | --- |
| Request rate | `stream_requests_per_second`, `stream_request_burst` | Token bucket per client for playlists, segments, and source ranges. Exceeding it returns `429` with `Retry-After`. Normal HLS playback uses well under 1 request/s; seeking bursts are absorbed by the burst. |
| Session rate | `sessions_per_minute` | Limits watch/embed page loads per client, which also limits fake view creation. |
| Concurrent streams | `max_concurrent_streams_per_client` | Caps simultaneous open responses (parallel downloaders open many). |
| Per-response pacing | `segment_bytes_per_second` / `segment_burst_bytes`, `source_bytes_per_second` / `source_burst_bytes` | Sends the first *burst* bytes at full speed (fast start), then paces the rest — the same model as nginx `limit_rate_after` + `limit_rate`. |
| Global egress cap | `global_bytes_per_second` | One shared budget for all stream bodies, so the server never exceeds the link capacity you reserve for video. |
| Hotlink blocking | `block_cross_site_requests` | Rejects browser requests marked `Sec-Fetch-Site: cross-site`, i.e. other sites pointing `<video>` or hls.js at your streams. The official watch page and the embed iframe are same-origin and unaffected. |
| Original file | `expose_original_with_hls` | When `false`, viewers can only fetch HLS renditions; the full-quality upload is available to the owner's API key only. |
| Private media | visibility `private` | Short-lived HMAC-signed tokens and HttpOnly cookies scoped to the media's stream path. |
| Honest analytics | built in | Watch time is capped by wall-clock time between events and by playback progress; qualified views count once per viewer, media and day. |

Clients are identified by IP address (IPv6 grouped by /64). Behind a reverse
proxy or CDN set `client_ip_header` (for example `x-forwarded-for` or
`cf-connecting-ip`); without a trusted proxy leave it unset, otherwise clients
can spoof their address. Limits are kept in memory per process.

### Choosing pacing values

Pace relative to your top rendition. For a 1080p ladder topping out at
6 Mbit/s (750 kB/s):

- `segment_bytes_per_second = 3000000` (4× the top bitrate) keeps adaptive
  bitrate selection working while preventing a client from pulling segments
  as fast as the disk can read them. Leave `segment_burst_bytes` at about one
  segment (`3000000`) so playback starts instantly.
- `source_bytes_per_second = 2500000` with `source_burst_bytes = 16000000`
  allows progressive playback of the fallback file but turns a "save as" of a
  2 GB original into a long download.
- `global_bytes_per_second` at roughly 80 % of your uplink keeps the admin UI
  and API responsive under load.

## Recommended edge setup

The in-process limits are a backstop. In production put a CDN or reverse proxy
in front of `/stream` and:

1. Cache segments at the edge (`segment_cache_control` is immutable by
   default), which removes most origin bandwidth.
2. Add edge rate limiting and bot management (Cloudflare, Fastly, nginx
   `limit_req`/`limit_conn`).
3. Use signed, expiring URLs or tokens at the CDN for premium content.
4. For studio-grade content, package with DRM (Widevine/FairPlay/PlayReady via
   CMAF + CENC) through a packager and license service; this is outside
   Gulfstream's scope.
5. Optional forensic watermarking (per-session visible or invisible marks)
   helps trace leaks; it requires per-viewer packaging.

## What it cannot do

No setting prevents screen recording, and determined users can still save HLS
segments at the paced rate. Treat these controls as cost and abuse reduction,
not as copy protection.
