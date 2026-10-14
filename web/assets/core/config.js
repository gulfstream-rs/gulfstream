/** Server-provided runtime configuration (see src/http/web/runtime.rs). */
export const config = Object.freeze(window.GULFSTREAM_CONFIG);
export const page = window.GULFSTREAM_PAGE;

export const isAuthPage = ['login', 'register'].includes(page);

export function endpoint(template, mediaId) {
  return template.replace('{media_id}', encodeURIComponent(mediaId));
}

export function mediaPage(mediaId) {
  return `${config.routes.media}/${encodeURIComponent(mediaId)}`;
}

/** Same-origin watch page URL; independent of `server.public_base_url`. */
export function watchPage(mediaId, token = '') {
  const base = `${config.routes.watch}/${encodeURIComponent(mediaId)}`;
  return token ? `${base}?token=${encodeURIComponent(token)}` : base;
}

/** Same-origin embed player URL (public and unlisted media only). */
export function embedPage(mediaId) {
  return `${config.routes.embed}/${encodeURIComponent(mediaId)}`;
}

export function mediaIdFromLocation() {
  const prefix = `${config.routes.media}/`;
  if (!window.location.pathname.startsWith(prefix)) return '';
  return decodeURIComponent(window.location.pathname.slice(prefix.length));
}
