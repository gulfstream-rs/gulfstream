/**
 * Theme management. Dark is the default; the choice persists in localStorage
 * under a key shared with the watch page (templates/player.html) and is applied
 * before first paint by the inline script in web/shell.html.
 */
export const THEME_STORAGE_KEY = 'gulfstream:theme';
const THEMES = ['dark', 'light'];
const THEME_COLORS = { dark: '#0a0c10', light: '#ffffff' };

export function currentTheme() {
  const theme = document.documentElement.dataset.theme;
  return THEMES.includes(theme) ? theme : 'dark';
}

export function applyTheme(theme) {
  const next = THEMES.includes(theme) ? theme : 'dark';
  document.documentElement.dataset.theme = next;
  document.querySelector('meta[name="theme-color"]')?.setAttribute('content', THEME_COLORS[next]);
  for (const toggle of document.querySelectorAll('[data-theme-toggle]')) {
    const label = next === 'dark' ? 'Switch to light theme' : 'Switch to dark theme';
    toggle.setAttribute('aria-label', label);
    toggle.setAttribute('title', label);
  }
  return next;
}

export function setTheme(theme) {
  const next = applyTheme(theme);
  try {
    window.localStorage.setItem(THEME_STORAGE_KEY, next);
  } catch { /* Preference simply won't persist. */ }
  return next;
}

export function initializeTheme() {
  applyTheme(currentTheme());
  for (const toggle of document.querySelectorAll('[data-theme-toggle]')) {
    toggle.addEventListener('click', () => setTheme(currentTheme() === 'dark' ? 'light' : 'dark'));
  }
  // Keep multiple open tabs in sync.
  window.addEventListener('storage', (event) => {
    if (event.key === THEME_STORAGE_KEY && event.newValue) applyTheme(event.newValue);
  });
}
