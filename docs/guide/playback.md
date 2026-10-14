# Playback and privacy

Ready media is played through the generated watch page (`templates/player.html`). The player prefers the configurable hls.js script (`player.hls_javascript_url`) for adaptive streaming, uses native HLS where the browser supports it, and falls back to the retained original file when adaptive playback is unavailable or fails. When no rendition exists and the original is retained, Gulfstream serves the original source with byte ranges.

## Player

The watch page is self-contained (it works when the management interface is disabled) and is progressively enhanced: native browser controls remain if its script cannot run.

- Custom controls: play/pause, ±10 s skip, seek bar with buffered range and hover preview, volume, playback speed, quality selection (hls.js), picture-in-picture, and fullscreen (element fullscreen, or native video fullscreen on iOS).
- Touch: tap shows or hides controls; double-tap the left or right side to seek.
- Keyboard: `Space`/`K` play/pause, `J`/`L` ±10 s, `←`/`→` ±5 s, `↑`/`↓` volume, `M` mute, `F` fullscreen, `0`–`9` jump to a percentage, `<`/`>` speed.
- Recovery: hls.js network and media errors are retried; unrecoverable errors switch to the original file when it is retained, otherwise a retry prompt is shown.
- Analytics: play, pause, seek, heartbeat, and ended events are reported; watched time is flushed with `sendBeacon` when the page is hidden or closed.
- Sharing: a share dialog copies the watch link (optionally at the current time, `?t=90`) and, for public and unlisted media, the embed code.
- Resume: the watch page remembers the position of partially watched videos in the browser and offers "Start over".
- End screen: replay, share, and (in embeds) a link to the full watch page.
- Embedding: inside a frame the player fills it. The dashboard preview starts automatically; third-party embeds only when `autoplay=1` is requested.
- Theme: the page chrome follows the dark/light preference stored by the management interface; the video area is always dark.

Required template markers are validated at startup. They include <span v-pre>`{{FALLBACK_URL_JSON}}`</span> (original-file URL or `null`), <span v-pre>`{{SITE_NAME}}`</span>, <span v-pre>`{{PLAYER_OPTIONS_JSON}}`</span> (surface, autoplay, start offset, share and embed URLs) and <span v-pre>`{{HEAD_LINKS}}`</span> (canonical link, oEmbed discovery, `noindex` for embeds and non-public media).

## Embedding

Public and unlisted media can be embedded on any website with an iframe pointing at the embed route (default `/embed/{media_id}`). The media page in the web interface has an **Embed** tab that generates the code (responsive or fixed size, autoplay, loop, start time):

```html
<iframe src="https://video.example.com/embed/MEDIA_ID?autoplay=1&start=30"
  title="My video" style="display:block;width:100%;aspect-ratio:16 / 9;border:0"
  allow="autoplay; fullscreen; picture-in-picture; encrypted-media; clipboard-write"
  allowfullscreen loading="lazy"></iframe>
```

- Query options: `autoplay=1` (muted by default, as browsers require; `muted=0` to try with sound), `muted=1`, `loop=1`, `start=90` (also `1m30s`, `1:30`, or `t=`).
- `embed.allowed_origins` restricts which sites may frame the player (`Content-Security-Policy: frame-ancestors`). The regular watch page can only be framed by Gulfstream itself.
- Analytics work without third-party cookies: each player page carries a signed session token for its events. Viewers without a first-party cookie are de-duplicated by a daily, keyed hash of address and user agent.
- oEmbed: watch pages advertise `/oembed`, so WordPress, Ghost, Notion, and other oEmbed consumers embed a pasted watch link automatically (`/oembed?url=...&maxwidth=...`).
- Private media cannot be embedded (browsers withhold the required cookies in third-party frames); use a temporary signed link instead.

### JavaScript API

The embedded player talks to its parent page with `postMessage`:

```js
const player = document.querySelector('iframe').contentWindow;
player.postMessage({ type: 'gulfstream:command', command: 'seek', value: 42 }, '*');
window.addEventListener('message', (event) => {
  if (event.data?.type === 'gulfstream:event') console.log(event.data.event, event.data.currentTime);
});
```

Commands: `play`, `pause`, `mute`, `unmute`, `seek` (`value` in seconds). Events: `ready`, `play`, `pause`, `seeked`, `volumechange`, `ended`, and `timeupdate` (at most once per second), each with `currentTime`, `duration`, `muted`, and `paused`.

See [Playback protection](/guide/protection) for rate limits, bandwidth pacing, and hotlink blocking.

## Visibility

- `public`: discoverability is an application concern; playback requires no token.
- `unlisted`: playback requires no token, but the API does not expose other accounts' media lists.
- `private`: playback and stream assets require owner authorization or a signed playback token. A redeemed token is stored as HttpOnly cookies scoped to the watch page and the media's stream assets, so reloading the watch page keeps working until the token expires.

Unrecognized query parameters on watch URLs (for example `utm_*` tracking parameters) are ignored.

## Caching

Player, token redirect, playlists, segments, original sources, and private assets each have independent configurable `Cache-Control` values. Private defaults use `no-store`; immutable public segments can use long-lived caching.

## Byte ranges

Original sources and eligible assets support a single RFC-style `Range` request when enabled. Gulfstream returns `206` with `Content-Range`, or `416` for an invalid range.
