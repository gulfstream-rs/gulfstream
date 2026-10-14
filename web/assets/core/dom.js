export const app = document.querySelector('#app');

export function query(selector, root = document) {
  const element = root.querySelector(selector);
  if (!element) throw new Error(`Required element not found: ${selector}`);
  return element;
}

/** Replaces the page content with trusted markup produced by `html`. */
export function setPage(content, title) {
  app.innerHTML = String(content);
  app.setAttribute('aria-busy', 'false');
  if (title) {
    const titleNode = document.querySelector('[data-page-title]');
    if (titleNode) titleNode.textContent = title;
    const siteName = window.GULFSTREAM_CONFIG?.site?.name;
    document.title = siteName ? `${title} · ${siteName}` : title;
  }
  app.focus({ preventScroll: true });
}

export function replaceContent(element, content) {
  if (element) element.innerHTML = String(content);
}

/** Copies only the allow-listed query parameters from the current URL. */
export function allowedQuery(keys) {
  const source = new URLSearchParams(window.location.search);
  const target = new URLSearchParams();
  for (const key of keys) {
    const value = source.get(key);
    if (value) target.set(key, value);
  }
  return { source, target };
}

export function labelize(value) {
  return String(value ?? '')
    .replaceAll('_', ' ')
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

export async function copyText(text) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  const area = document.createElement('textarea');
  area.value = text;
  area.setAttribute('readonly', '');
  area.style.position = 'fixed';
  area.style.opacity = '0';
  document.body.append(area);
  area.select();
  const copied = document.execCommand('copy');
  area.remove();
  if (!copied) throw new Error('Copying is not supported by this browser.');
}
