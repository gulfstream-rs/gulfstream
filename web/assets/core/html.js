/**
 * Auto-escaping HTML templating.
 *
 * `html` is a tagged template: every interpolated value is HTML-escaped unless
 * it is itself the result of `html` (or explicitly wrapped with `raw`). Arrays
 * are flattened, and `null`, `undefined` and `false` render nothing, which makes
 * conditional fragments and lists safe and concise:
 *
 *   html`<li>${item.title}</li>`
 *   html`<ul>${items.map((item) => html`<li>${item}</li>`)}</ul>`
 *   html`${error && html`<p>${error.message}</p>`}`
 */
export class SafeHtml {
  constructor(value) {
    this.value = value;
  }

  toString() {
    return this.value;
  }
}

export function escapeHtml(value) {
  return String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

/** Marks trusted, already-escaped markup. Never pass user-controlled input. */
export function raw(value) {
  return new SafeHtml(String(value ?? ''));
}

function serialize(value) {
  if (value == null || value === false) return '';
  if (value instanceof SafeHtml) return value.value;
  if (Array.isArray(value)) return value.map(serialize).join('');
  return escapeHtml(value);
}

export function html(strings, ...values) {
  let output = strings[0];
  values.forEach((value, index) => {
    output += serialize(value) + strings[index + 1];
  });
  return new SafeHtml(output);
}
