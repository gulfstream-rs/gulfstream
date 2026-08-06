export const config = Object.freeze(window.GULFSTREAM_CONFIG);
export const page = window.GULFSTREAM_PAGE;

export function endpoint(template, mediaId) {
  return template.replace('{media_id}', encodeURIComponent(mediaId));
}

export function mediaPage(mediaId) {
  return `${config.routes.media}/${encodeURIComponent(mediaId)}`;
}

export function mediaIdFromLocation() {
  const prefix = `${config.routes.media}/`;
  if (!window.location.pathname.startsWith(prefix)) return '';
  return decodeURIComponent(window.location.pathname.slice(prefix.length));
}

export function initializeShell() {
  document.documentElement.style.setProperty('--brand', config.presentation.brand_color);
  document.body.dataset.page = page;
  document.body.classList.toggle('auth-page', ['login', 'register'].includes(page));

  for (const link of document.querySelectorAll('[data-route]')) {
    const target = config.routes[link.dataset.route];
    if (target) link.href = target;
    const pages = (link.dataset.page || '').split(/\s+/).filter(Boolean);
    if (pages.includes(page)) link.setAttribute('aria-current', 'page');
  }

  for (const link of document.querySelectorAll('[data-link]')) {
    const target = config.links[link.dataset.link];
    if (target) {
      link.href = target;
      link.hidden = false;
    }
  }

  const sidebar = document.querySelector('#sidebar');
  const toggle = document.querySelector('[data-sidebar-toggle]');
  toggle?.addEventListener('click', () => {
    const open = sidebar?.dataset.open !== 'true';
    if (sidebar) sidebar.dataset.open = String(open);
    toggle.setAttribute('aria-expanded', String(open));
  });

  document.addEventListener('click', (event) => {
    if (window.matchMedia('(min-width: 52.01rem)').matches) return;
    if (!sidebar?.contains(event.target) && !toggle?.contains(event.target)) {
      sidebar?.removeAttribute('data-open');
      toggle?.setAttribute('aria-expanded', 'false');
    }
  });
}
