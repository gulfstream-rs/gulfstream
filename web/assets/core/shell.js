import { SESSION_EVENT, logout } from './api.js';
import { config, isAuthPage, page } from './config.js';
import { initializeTheme } from './theme.js';

function wireRoutes() {
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
  const brand = document.querySelector('.topbar-brand');
  brand?.setAttribute('data-site-name', config.site.name);
}

function wireSearch() {
  const form = document.querySelector('[data-search-form]');
  if (!form) return;
  form.action = config.routes.media;
  const input = form.querySelector('input[name="search"]');
  if (input && page === 'media') input.value = new URLSearchParams(window.location.search).get('search') || '';
  // "/" focuses search, as in most management consoles.
  document.addEventListener('keydown', (event) => {
    if (event.key !== '/' || event.metaKey || event.ctrlKey || event.altKey) return;
    const target = event.target;
    if (target instanceof HTMLElement && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))) return;
    if (!input || input.offsetParent === null) return;
    event.preventDefault();
    input.focus();
  });
}

function wireUserMenu() {
  const menu = document.querySelector('[data-user-menu]');
  if (!menu) return;
  document.addEventListener('click', (event) => {
    if (menu.open && !menu.contains(event.target)) menu.open = false;
  });
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && menu.open) {
      menu.open = false;
      menu.querySelector('summary')?.focus();
    }
  });
  window.addEventListener(SESSION_EVENT, (event) => {
    const account = event.detail?.account;
    if (!account) return;
    const name = account.display_name || account.email || '';
    menu.querySelector('[data-user-name]').textContent = name;
    menu.querySelector('[data-user-email]').textContent = account.email || '';
    menu.querySelector('[data-user-initial]').textContent = (name.trim()[0] || '?').toUpperCase();
  });
  menu.querySelector('[data-logout]')?.addEventListener('click', async () => {
    try {
      await logout();
    } finally {
      window.location.assign(config.routes.login);
    }
  });
}

export function initializeShell() {
  document.body.dataset.page = page;
  document.body.classList.toggle('auth-page', isAuthPage);
  initializeTheme();
  wireRoutes();
  wireSearch();
  wireUserMenu();
}
