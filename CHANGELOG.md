# Changelog

All notable changes to Gulfstream are documented here. Releases follow Semantic Versioning.

## 5.0.1 — 2026-10-19

- Updated `rustls` to 0.23.46 (RUSTSEC-2026-0346, RUSTSEC-2026-0347, RUSTSEC-2026-0348, RUSTSEC-2026-0349).

## 5.0.0 — 2026-10-14

Breaking changes (no in-place upgrade from 4.x):

- The database schema is a single initial migration again. Databases created by 4.x are rejected at startup (modified migration checksum); start with a new database file.
- `[protection]`, `[embed]`, and `[geolocation]`, `routes.embed`, `routes.oembed`, `analytics.qualified_view_seconds`, and `analytics.qualified_view_minimum_fraction` are required configuration. Start from `config/gulfstream.example.toml`.
- Protection defaults now follow the recommended values: originals are paced at 2.5 MB/s after a 16 MB burst, cross-site media requests are rejected, and viewers no longer receive the original file once HLS renditions exist.
- The player template must contain the `{{FALLBACK_URL_JSON}}`, `{{SITE_NAME}}`, `{{PLAYER_OPTIONS_JSON}}`, and `{{HEAD_LINKS}}` markers.
- The watch page now sends `Content-Security-Policy: frame-ancestors 'self'`; use the embed route to show videos on other sites.

Changes:

- Redesigned the management interface: dark theme by default with a persisted light theme, desktop sidebar, tablet icon rail, mobile bottom tab bar, global media search, account menu, collapsible filters, recent uploads, and cancellable uploads.
- Added an inline player preview, temporary signed links for private media, share-link copying, and automatic status refresh while media is processing.
- Redesigned the watch page player with custom accessible controls, quality and speed selection, keyboard and touch gestures, picture-in-picture, fullscreen, buffering and error states, hls.js error recovery, and automatic fallback to the original file.
- Fixed reloading a private watch page after token redemption, watch links failing with extra query parameters, and dashboard playback depending on `server.public_base_url`.
- Web interface assets are now served with `Cache-Control: no-cache`, so browsers revalidate them instead of mixing stale and updated JavaScript modules after an upgrade.
- Introduced auto-escaping HTML templating in the browser code and JSON escaping for inline-script values on the server; single-pass template substitution prevents marker injection through media metadata.
- Split `util` and HTTP playback modules into focused submodules and extended web validation to icons and the player template.
- Added third-party embedding: a chromeless `/embed/{id}` player (public and unlisted media) with `autoplay`, `muted`, `loop`, and `start` options, `frame-ancestors` control through `embed.allowed_origins`, a `postMessage` API, an oEmbed provider (`/oembed`) with discovery links, and an embed-code builder on the media page. Analytics events from embeds are authorized by a signed session token, so they work without third-party cookies.
- Improved the player: share dialog (link at current time and embed code), title bar in embeds and full screen, "Tap to unmute" for muted autoplay, resume position with "Start over", end screen, `?t=` start offsets on watch links, and canonical, Open Graph, and `noindex` metadata.
- Added playback protection (`[protection]`): per-client stream request and session rate limits with `429 Retry-After`, concurrent-stream limits, per-response pacing for segments and originals after an initial burst, a global egress cap, cross-site hotlink blocking, an option to withhold originals from viewers when HLS exists, and trusted proxy client-address resolution.
- Added country analytics from a trusted CDN header or a MaxMind DB country database (DB-IP Lite, GeoLite2, IPinfo Lite), `GET /analytics/countries`, and a top-countries report.
- Added qualified views (server-measured watch time ≥ `analytics.qualified_view_seconds` or a fraction of short videos, once per viewer, media, and day) as the monetization unit, reported in all analytics totals.
- Redesigned dashboard and analytics metrics with count-aware grids that never leave a single card on a row, a views chart on the dashboard, and human-readable watch time.
- Added `maxminddb` 0.32.0; updated `h2` to 0.4.20 and `rustls` to 0.23.45 (RUSTSEC-2026-0258, RUSTSEC-2026-0285) and replaced the yanked `chacha20` 0.10.1 with 0.10.2.

## 4.0.1 — 2026-08-08

- Updated `base64` to 0.23.0 and `toml` to 1.1.4+spec-1.1.0.
- Updated GitHub Actions: `actions/checkout` to v7, `actions/configure-pages` to v6, `actions/deploy-pages` to v5, and `docker/login-action` to v4.6.0.

## 4.0.0 — 2026-08-06

- Reorganized backend code into private domain, application, infrastructure, HTTP, worker, presentation, and runtime boundaries.
- Reduced the public Rust API to configuration loading and the documented server runtime entry point.
- Added typed media/job states and shared pagination contracts and normalization.
- Rebuilt the management interface with responsive navigation, drag-and-drop upload progress, configurable pagination, automatic refresh, status breakdowns, analytics charts, accessible feedback, and safe destructive actions.
- Added locale, time-zone, brand-color, refresh-interval, and page-size presentation configuration with startup validation.
- Added deterministic crate include rules, docs.rs metadata, rustfmt policy, cargo-deny policy, contributor guidance, pull-request and issue templates, and stronger CI/release gates.
- Expanded architecture, dashboard, configuration, contributor, and release documentation.

## 3.0.0 — 2026-08-06

- Added configurable account registration, password authentication, browser sessions, CSRF protection, and revocable API keys.
- Added direct uploads, protected remote imports, durable processing jobs, FFprobe inspection, and adaptive HLS generation through FFmpeg.
- Added public, unlisted, and private playback with signed authorization, byte-range delivery, and real persisted analytics.
- Added the browser application for registration, login, dashboard, uploads, media management, processing, analytics, and account management.
- Added the OpenAPI 3.1 contract, VitePress documentation, Docker deployment, CI, documentation publishing, crates.io publishing, and dependency updates.
