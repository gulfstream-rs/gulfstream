# Management interface

Gulfstream includes a responsive management application built with semantic HTML, modular CSS, and browser-native ES modules. It has no frontend framework, build step, independent application server, or mock state. Every view calls the same authenticated API available to external clients.

## Themes and layout

- **Dark theme by default**, with a light theme available from the sun/moon toggle in the top bar. The choice is stored in `localStorage` (`gulfstream:theme`), applied before first paint, synchronized across open tabs, and shared with the watch page.
- **Desktop (≥ 64rem):** persistent sidebar, global media search (press `/`), upload shortcut, and account menu.
- **Tablet (48–64rem):** compact icon rail.
- **Phone (< 48rem):** thumb-reachable bottom tab bar (Home, Media, Upload, Jobs, Stats), filters collapse behind a *Filters* button, and tables become cards.

## Pages

Pages are mounted below the configured `routes.web` prefix:

| Page | Management workflow |
|---|---|
| Dashboard | Views, unique viewers, watch time, and delivered data for the reporting window; a daily views chart; recent uploads; storage quota; media and job status breakdowns; and quick actions, with automatic refresh |
| Register | Account creation under the configured registration policy |
| Login | Email/password browser-session creation |
| Upload | *From device* / *From URL* tabs, drag-and-drop with file validation, cancellable progress, metadata, and visibility |
| Media | Search, status/visibility filters (applied on change), configurable page sizes, responsive result cards/tables, and pagination |
| Media details | Inline player preview, open the full player, copy share links, build embed code (responsive or fixed size, autoplay, loop, start time) or create temporary signed links for private media, edit metadata, inspect source and renditions, review jobs, retry failed work, and delete; status updates automatically while processing |
| Processing | Filter durable jobs by status or kind and monitor active work with automatic refresh |
| Analytics | Date-range and per-media totals (views, qualified views, unique viewers, completion rate, watch time, average view duration, countries, delivered data), a daily views chart, top countries, and a daily breakdown |
| Account | Edit profile, create/copy/revoke API keys without a page reload, inspect quota, and end the browser session |

## Interaction model

- Browser sessions use an HttpOnly cookie.
- The rotated CSRF token is kept only in `sessionStorage` and sent on state-changing requests.
- Upload progress comes from the browser upload stream rather than an estimated timer.
- Destructive operations use a native, keyboard-accessible confirmation dialog.
- Success and failure feedback is announced through accessible live regions.
- Markup is produced with an auto-escaping `html` tagged template (`web/assets/core/html.js`), so interpolated API data is always HTML-escaped.
- Playback from the dashboard uses same-origin watch URLs, so it does not depend on `server.public_base_url` matching the address in the browser.
- Focus remains visible, motion respects `prefers-reduced-motion`, and touch targets are at least 44 px.
- Metric and tile grids choose their column count from both the available width and the number of tiles, so rows are always complete (for example 8 metrics lay out as 4 + 4 or 2 + 2 + 2 + 2, never 5 + 3 or 7 + 1).
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

## Source layout

```text
web/
├── shell.html            HTML shell, SVG icon sprite, pre-paint theme script
└── assets/
    ├── app.js            entry point and page registry
    ├── core/             runtime config, API client, html templating, DOM, formatting, theme, shell
    ├── components/       icons, layout, data display, forms, feedback, charts, auto-refresh
    ├── pages/            one module per page
    └── styles/           tokens (themes), base, layout, components, pages
```

`node scripts/validate-web.mjs` (also `make web`) syntax-checks every module, verifies imports and icon references, and validates the shell and player templates.
