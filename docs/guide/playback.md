# Playback and privacy

Ready media is played through the generated watch page. HLS-capable browsers use the master playlist directly; other supported browsers can use the configurable hls.js script URL. When no rendition exists and the original is retained, Gulfstream can serve the original source with byte ranges.

## Visibility

- `public`: discoverability is an application concern; playback requires no token.
- `unlisted`: playback requires no token, but the API does not expose other accounts' media lists.
- `private`: playback and stream assets require owner authorization or a signed playback token.

## Caching

Player, token redirect, playlists, segments, original sources, and private assets each have independent configurable `Cache-Control` values. Private defaults use `no-store`; immutable public segments can use long-lived caching.

## Byte ranges

Original sources and eligible assets support a single RFC-style `Range` request when enabled. Gulfstream returns `206` with `Content-Range`, or `416` for an invalid range.
