import { currentSession, request } from '../core/api.js';
import { config } from '../core/config.js';
import { escapeHtml, setPage } from '../core/dom.js';
import { formatBytes, formatDuration, formatNumber, percent } from '../core/format.js';
import { autoRefresh, metricCard, pageHeader, statusBreakdown } from '../core/ui.js';

function content(dashboard) {
  const storagePercent = percent(dashboard.account.storage_used_bytes, dashboard.account.storage_quota_bytes);
  return `<div class="metric-grid">
    ${metricCard('Media', formatNumber(dashboard.media_total), statusBreakdown(dashboard.media_by_status))}
    ${metricCard('Processing jobs', formatNumber(dashboard.jobs_total), statusBreakdown(dashboard.jobs_by_status))}
    ${metricCard('Views', formatNumber(dashboard.analytics.views), `${escapeHtml(dashboard.analytics_from)} – ${escapeHtml(dashboard.analytics_to)}`)}
    ${metricCard('Unique viewers', formatNumber(dashboard.analytics.unique_viewers))}
    ${metricCard('Watch time', formatDuration(dashboard.analytics.watch_time_ms))}
    ${metricCard('Completed views', formatNumber(dashboard.analytics.completed_views))}
  </div>
  <section class="page-section card"><div class="card-header"><div><h2>Storage</h2><p>Account quota and current usage.</p></div><strong>${storagePercent.toFixed(1)}%</strong></div><div class="card-body"><div class="progress" aria-label="Storage used" aria-valuenow="${storagePercent.toFixed(1)}" aria-valuemin="0" aria-valuemax="100" role="progressbar"><span style="--progress:${storagePercent.toFixed(2)}%"></span></div><p class="field-hint">${formatBytes(dashboard.account.storage_used_bytes)} used of ${formatBytes(dashboard.account.storage_quota_bytes)}</p></div></section>
  <section class="page-section"><div><h2>Quick actions</h2><p class="field-hint">Common management workflows.</p></div><div class="quick-actions">
    <a class="quick-action" href="${escapeHtml(config.routes.upload)}"><strong>Upload video</strong><span>Send a local file or start a protected URL import.</span></a>
    <a class="quick-action" href="${escapeHtml(config.routes.media)}"><strong>Manage media</strong><span>Search, edit, play, retry, or delete media.</span></a>
    <a class="quick-action" href="${escapeHtml(config.routes.jobs)}"><strong>Review processing</strong><span>Inspect queue state, retries, and conversion failures.</span></a>
    <a class="quick-action" href="${escapeHtml(config.routes.analytics)}"><strong>Open analytics</strong><span>Review views, watch time, completion, and delivery.</span></a>
  </div></section>`;
}

export async function renderDashboard() {
  await currentSession();
  const dashboard = await request(config.api.dashboard);
  setPage(`${pageHeader(`Welcome, ${dashboard.account.display_name}`, 'Monitor your library, processing pipeline, storage, and audience activity.', `<a class="button button-primary" href="${escapeHtml(config.routes.upload)}">Upload video</a>`)}<div id="dashboard-content">${content(dashboard)}</div>`, 'Dashboard');

  autoRefresh(async () => {
    try {
      const next = await request(config.api.dashboard);
      const node = document.querySelector('#dashboard-content');
      if (node) node.innerHTML = content(next);
    } catch { /* Keep the last successful dashboard snapshot. */ }
  }, config.presentation.dashboard_refresh_seconds);
}
