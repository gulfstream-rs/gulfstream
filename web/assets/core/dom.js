export const app = document.querySelector('#app');

export function escapeHtml(value) {
  return String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

export function query(selector, root = document) {
  const element = root.querySelector(selector);
  if (!element) throw new Error(`Required element not found: ${selector}`);
  return element;
}

export function setPage(html, title) {
  app.innerHTML = html;
  app.setAttribute('aria-busy', 'false');
  if (title) {
    const titleNode = document.querySelector('[data-page-title]');
    if (titleNode) titleNode.textContent = title;
  }
  app.focus({ preventScroll: true });
}

export function optionMarkup(values, selected, allLabel = null) {
  const all = allLabel == null ? '' : `<option value="">${escapeHtml(allLabel)}</option>`;
  return all + values.map((value) => {
    const isSelected = value === selected ? ' selected' : '';
    return `<option value="${escapeHtml(value)}"${isSelected}>${escapeHtml(labelize(value))}</option>`;
  }).join('');
}

export function labelize(value) {
  return String(value ?? '')
    .replaceAll('_', ' ')
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

export function allowedQuery(keys) {
  const source = new URLSearchParams(window.location.search);
  const target = new URLSearchParams();
  for (const key of keys) {
    const value = source.get(key);
    if (value) target.set(key, value);
  }
  return { source, target };
}
