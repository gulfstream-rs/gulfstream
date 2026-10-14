import { html } from '../core/html.js';

export function liveIndicator(seconds) {
  return html`<span class="live-indicator" data-live-indicator>Live · refreshes every ${seconds}s</span>`;
}

/**
 * Periodically runs `callback` while the tab is visible, and immediately when
 * it becomes visible again. Failures keep the last successful snapshot.
 */
export function autoRefresh(callback, seconds) {
  const period = Math.max(1, Number(seconds) || 0) * 1000;
  let running = false;
  const run = async () => {
    if (running || document.visibilityState !== 'visible') return;
    running = true;
    try {
      await callback();
    } catch { /* Keep the last successful snapshot. */ } finally {
      running = false;
    }
  };
  const interval = window.setInterval(run, period);
  const onVisibility = () => {
    const indicator = document.querySelector('[data-live-indicator]');
    if (indicator) indicator.dataset.state = document.visibilityState === 'visible' ? 'live' : 'paused';
    if (document.visibilityState === 'visible') run();
  };
  document.addEventListener('visibilitychange', onVisibility);
  window.addEventListener('pagehide', () => {
    window.clearInterval(interval);
    document.removeEventListener('visibilitychange', onVisibility);
  }, { once: true });
  return interval;
}
