import { currentSession, request } from '../core/api.js';
import { config } from '../core/config.js';
import { allowedQuery, escapeHtml, setPage } from '../core/dom.js';
import { formatBytes, formatDuration, formatNumber, percent } from '../core/format.js';
import { metricCard, pageHeader, table } from '../core/ui.js';
import { barChart } from '../components/chart.js';

export async function renderAnalytics() {
  await currentSession();
  if (!config.features.analytics) {
    setPage(`${pageHeader('Analytics', 'Audience measurement is disabled by server configuration.')}<div class="notice">Enable analytics in the server configuration to collect playback and delivery events.</div>`, 'Analytics');
    return;
  }
  const { source, target } = allowedQuery(['from', 'to', 'media_id']);
  const suffix = target.toString() ? `?${target.toString()}` : '';
  const [summary, series] = await Promise.all([
    request(`${config.api.analytics_summary}${suffix}`),
    request(`${config.api.analytics_time_series}${suffix}`),
  ]);
  const completionRate = percent(summary.totals.completed_views, summary.totals.play_starts);
  const rows = series.points.map((point) => `<tr><td data-label="Day">${escapeHtml(point.day)}</td><td data-label="Views">${formatNumber(point.views)}</td><td data-label="Unique">${formatNumber(point.unique_viewers)}</td><td data-label="Starts">${formatNumber(point.play_starts)}</td><td data-label="Completed">${formatNumber(point.completed_views)}</td><td data-label="Watch time">${formatDuration(point.watch_time_ms)}</td><td data-label="Bytes">${formatBytes(point.bytes_served)}</td></tr>`);
  setPage(`${pageHeader('Analytics', 'Review persisted playback activity and bytes actually delivered.')}
    <section class="card"><div class="card-body"><form method="get" class="form-grid"><div class="form-row"><label class="field"><span>From</span><input name="from" type="date" value="${escapeHtml(source.get('from') || summary.from)}"></label><label class="field"><span>To</span><input name="to" type="date" value="${escapeHtml(source.get('to') || summary.to)}"></label></div><label class="field"><span>Media ID</span><input name="media_id" value="${escapeHtml(source.get('media_id') || '')}" placeholder="Leave empty for all media"></label><div class="form-actions"><button class="button button-primary" type="submit">Apply range</button><a class="button button-ghost" href="${escapeHtml(config.routes.analytics)}">Reset</a></div></form></div></section>
    <section class="page-section metric-grid">
      ${metricCard('Views', formatNumber(summary.totals.views))}
      ${metricCard('Unique viewers', formatNumber(summary.totals.unique_viewers))}
      ${metricCard('Play starts', formatNumber(summary.totals.play_starts))}
      ${metricCard('Completion rate', `${completionRate.toFixed(1)}%`, `${formatNumber(summary.totals.completed_views)} completed views`)}
      ${metricCard('Watch time', formatDuration(summary.totals.watch_time_ms))}
      ${metricCard('Bytes served', formatBytes(summary.totals.bytes_served))}
    </section>
    <section class="page-section card"><div class="card-header"><div><h2>Views by day</h2><p>${escapeHtml(summary.from)} through ${escapeHtml(summary.to)}</p></div></div><div class="card-body">${barChart(series.points, 'views', 'views')}</div></section>
    <section class="page-section"><div><h2>Daily totals</h2><p class="field-hint">Aggregated persisted metrics for the selected reporting range.</p></div>${table(['Day', 'Views', 'Unique', 'Starts', 'Completed', 'Watch time', 'Bytes'], rows, 'No analytics were recorded in this range.')}</section>`, 'Analytics');
}
