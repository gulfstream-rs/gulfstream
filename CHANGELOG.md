# Changelog

All notable changes to Gulfstream are documented here. Releases follow Semantic Versioning.

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
