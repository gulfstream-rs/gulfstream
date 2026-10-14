import { currentSession, request } from '../core/api.js';
import { config, mediaPage } from '../core/config.js';
import { replaceContent, setPage } from '../core/dom.js';
import { html } from '../core/html.js';
import { formatBytes, formatDate, formatDuration, formatNumber, formatWatchTime, percent } from '../core/format.js';
import { barChart } from '../components/chart.js';
import { badge, balancedGrid, emptyState, metricCard, metricGrid, progressBar, statusBreakdown } from '../components/data.js';
import { icon } from '../components/icon.js';
import { pageHeader, sectionHeading } from '../components/layout.js';
import { autoRefresh, liveIndicator } from '../components/refresh.js';

const RECENT_COUNT = 5;

function quickActions() {
  const actions = [
    ['upload', config.routes.upload, 'Upload video', 'Send a file or import from a URL.'],
    ['media', config.routes.media, 'Media library', 'Search, edit, play, and share.'],
    ['jobs', config.routes.jobs, 'Processing queue', 'Track conversions and retries.'],
    ['analytics', config.routes.analytics, 'Audience analytics', 'Views, countries, delivery.'],
  ];
  return balancedGrid(actions.map(([iconName, href, title, text]) => html`<a class="quick-action" href="${href}">
    <span class="quick-action-icon">${icon(iconName)}</span>
    <span><strong>${title}</strong><span>${text}</span></span>
  </a>`), 'quick-actions');
}

function recentMedia(items) {
  const body = items?.length
    ? html`<div class="recent-list">${items.map((media) => html`<a class="recent-item" href="${mediaPage(media.id)}">
        <span class="recent-thumb">${icon('media')}</span>
        <span style="min-width:0"><span class="recent-title">${media.title}</span><span class="recent-meta">${formatDate(media.created_at)}${media.duration_ms != null && ` · ${formatDuration(media.duration_ms)}`}</span></span>
        ${badge(media.status)}
      </a>`)}</div>`
    : html`<div class="card-body">${emptyState('No videos yet', 'Upload your first video to get started.', { action: html`<a class="button button-primary" href="${config.routes.upload}">${icon('upload', 'icon-sm')}Upload video</a>` })}</div>`;
  return html`<section class="card">
    <header class="card-header card-header-flush"><div><h2>Recent uploads</h2><p>Latest additions to your library.</p></div><a class="button button-ghost button-sm" href="${config.routes.media}">View all${icon('chevron-right', 'icon-sm')}</a></header>
    ${body}
  </section>`;
}

/** "Last 30 days" style label for an inclusive ISO date range. */
function periodLabel(from, to) {
  const days = Math.round((Date.parse(to) - Date.parse(from)) / 86_400_000) + 1;
  return Number.isFinite(days) && days > 0 ? `Last ${days} day${days === 1 ? '' : 's'}` : `${from} – ${to}`;
}

function audienceCard(series) {
  if (!series) return null;
  const total = series.points.reduce((sum, point) => sum + Number(point.views || 0), 0);
  return html`<section class="card">
    <header class="card-header card-header-flush"><div><h2>Views by day</h2><p title="${series.from} – ${series.to}">${periodLabel(series.from, series.to)} · ${formatNumber(total)} view${total === 1 ? '' : 's'}</p></div><a class="button button-ghost button-sm" href="${config.routes.analytics}">Analytics${icon('chevron-right', 'icon-sm')}</a></header>
    <div class="card-body">${barChart(series.points, 'views', 'views')}</div>
  </section>`;
}

function storageCard(account) {
  const used = percent(account.storage_used_bytes, account.storage_quota_bytes);
  return html`<section class="card"><div class="card-body storage-meter">
    <div class="metric-label">${icon('storage')}Storage</div>
    <div class="storage-meter-figures"><strong>${formatBytes(account.storage_used_bytes)}</strong><span class="muted">of ${formatBytes(account.storage_quota_bytes)} · ${used.toFixed(1)}%</span></div>
    ${progressBar(account.storage_used_bytes, account.storage_quota_bytes, 'Storage used')}
    <div class="storage-meter-footer"><span>${formatBytes(Math.max(0, account.storage_quota_bytes - account.storage_used_bytes))} available</span><a href="${config.routes.media}">Manage media${icon('chevron-right', 'icon-sm')}</a></div>
  </div></section>`;
}

function libraryCard(dashboard) {
  return html`<section class="card"><div class="card-body library-summary">
    <div class="library-row">
      <div class="metric-label">${icon('media')}Media</div>
      <strong class="library-count">${formatNumber(dashboard.media_total)}</strong>
      ${statusBreakdown(dashboard.media_by_status)}
    </div>
    <div class="library-row">
      <div class="metric-label">${icon('jobs')}Processing jobs</div>
      <strong class="library-count">${formatNumber(dashboard.jobs_total)}</strong>
      ${statusBreakdown(dashboard.jobs_by_status)}
    </div>
  </div></section>`;
}

function kpis(dashboard) {
  const { analytics } = dashboard;
  const period = periodLabel(dashboard.analytics_from, dashboard.analytics_to);
  const perView = analytics.views > 0 ? analytics.watch_time_ms / analytics.views : 0;
  const bytesPerView = analytics.views > 0 ? analytics.bytes_served / analytics.views : 0;
  return metricGrid([
    metricCard('Views', formatNumber(analytics.views), { iconName: 'eye', detail: period, emphasis: true }),
    metricCard('Unique viewers', formatNumber(analytics.unique_viewers), { iconName: 'users', detail: `${formatNumber(analytics.qualified_views)} qualified views` }),
    metricCard('Watch time', formatWatchTime(analytics.watch_time_ms), { iconName: 'clock', detail: `${formatWatchTime(perView)} per view` }),
    metricCard('Data delivered', formatBytes(analytics.bytes_served), { iconName: 'transfer', detail: `${formatBytes(bytesPerView)} per view` }),
  ], { label: 'Audience summary' });
}

function content(dashboard, recent, series) {
  return html`${kpis(dashboard)}
    <div class="split page-section-tight">
      <div class="stack">
        ${audienceCard(series)}
        ${recentMedia(recent)}
      </div>
      <div class="stack split-aside">
        ${storageCard(dashboard.account)}
        ${libraryCard(dashboard)}
      </div>
    </div>
    <section class="page-section">${sectionHeading('Quick actions')}${quickActions()}</section>`;
}

async function load() {
  const [dashboard, recent] = await Promise.all([
    request(config.api.dashboard),
    request(`${config.api.media}?page=1&page_size=${RECENT_COUNT}`).then((result) => result.items).catch(() => []),
  ]);
  // The chart covers the same window as the dashboard totals; it is optional.
  const series = config.features.analytics
    ? await request(`${config.api.analytics_time_series}?from=${dashboard.analytics_from}&to=${dashboard.analytics_to}`).catch(() => null)
    : null;
  return { dashboard, recent, series };
}

export async function renderDashboard() {
  const session = await currentSession();
  const { dashboard, recent, series } = await load();
  const name = dashboard.account.display_name || session.account.display_name;
  setPage(html`${pageHeader(`Welcome back, ${name}`, 'Your library, processing pipeline, and audience at a glance.', {
    meta: liveIndicator(config.presentation.dashboard_refresh_seconds),
  })}<div id="dashboard-content">${content(dashboard, recent, series)}</div>`, 'Dashboard');

  autoRefresh(async () => {
    const next = await load();
    replaceContent(document.querySelector('#dashboard-content'), content(next.dashboard, next.recent, next.series));
  }, config.presentation.dashboard_refresh_seconds);
}
