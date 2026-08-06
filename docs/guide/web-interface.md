# Management interface

Gulfstream includes a responsive management application built with semantic HTML, modular CSS, and browser-native ES modules. It has no frontend framework, build step, independent application server, or mock state. Every view calls the same authenticated API available to external clients.

## Pages

Pages are mounted below the configured `routes.web` prefix:

| Page | Management workflow |
|---|---|
| Dashboard | Storage quota, media and job status breakdowns, views, viewers, watch time, and recent activity with automatic refresh |
| Register | Account creation under the configured registration policy |
| Login | Email/password browser-session creation |
| Upload | Drag-and-drop direct upload with progress, metadata, visibility, and protected URL import |
| Media | Search, status/visibility filters, configurable page sizes, responsive result cards/tables, and pagination |
| Media details | Edit metadata, inspect source and renditions, review jobs, play, retry failed work, or safely delete |
| Processing | Filter durable jobs by status or kind and monitor active work with automatic refresh |
| Analytics | Date-range and per-media totals, completion rate, daily activity chart, watch time, and emitted bytes |
| Account | Edit profile, create/copy/revoke API keys, inspect quota, and end the browser session |

## Interaction model

- Browser sessions use an HttpOnly cookie.
- The rotated CSRF token is kept only in `sessionStorage` and sent on state-changing requests.
- Upload progress comes from the browser upload stream rather than an estimated timer.
- Destructive operations use a native, keyboard-accessible confirmation dialog.
- Success and failure feedback is announced through accessible live regions.
- Navigation collapses for narrow screens, tables become cards where appropriate, and focus remains visible.
- Dashboard and job refresh intervals, locale, time zone, brand color, and page-size choices come from validated server configuration.

The server injects runtime configuration into the shell, so route prefixes, links, feature availability, limits, registration policy, status values, and CSRF header names are not duplicated in browser code.

## Customization

Change operational presentation without editing JavaScript:

```toml
[web]
site_name = "Gulfstream"
tagline = "Video operations"
brand_color = "#2563eb"
date_locale = "en-US"
time_zone = "UTC"
dashboard_refresh_seconds = 30
jobs_refresh_seconds = 10
page_size_options = [10, 25, 50, 100]
```

For deeper customization, modify `web/shell.html` or the modular files under `web/assets/`. The server validates required shell markers at startup and refuses incomplete templates.
