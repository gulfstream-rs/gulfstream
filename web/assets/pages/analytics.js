import { currentSession, request } from '../core/api.js';
import { config } from '../core/config.js';
import { allowedQuery, setPage } from '../core/dom.js';
import { html } from '../core/html.js';
import { formatBytes, formatDuration, formatNumber, formatWatchTime, percent } from '../core/format.js';
import { barChart } from '../components/chart.js';
import { cell, emptyState, metricCard, metricGrid, table } from '../components/data.js';
import { notice } from '../components/feedback.js';
import { field } from '../components/forms.js';
import { pageHeader } from '../components/layout.js';

const PRESETS = [7, 30, 90];
const TOP_COUNTRIES = 8;

const regionNames = (() => {
  try { return new Intl.DisplayNames([config.presentation.date_locale, 'en'], { type: 'region' }); } catch { return null; }
})();

function countryName(code) {
  if (code === 'ZZ') return 'Unknown';
  try { return regionNames?.of(code) || code; } catch { return code; }
}

/** Regional-indicator flag emoji; a neutral globe for unknown locations. */
function countryFlag(code) {
  if (!/^[A-Z]{2}$/.test(code) || code === 'ZZ') return '🌐';
  return String.fromCodePoint(...[...code].map((letter) => 0x1f1e6 + letter.charCodeAt(0) - 65));
}

function countryRow(country, total) {
  const share = percent(country.views, total);
  return html`<li class="country-row">
    <span class="country-flag" aria-hidden="true">${countryFlag(country.country_code)}</span>
    <span class="country-name">${countryName(country.country_code)}</span>
    <span class="country-figures"><strong>${formatNumber(country.views)}</strong><span>${share.toFixed(1)}%</span></span>
    <span class="country-bar" aria-hidden="true"><span style="--share:${share.toFixed(2)}%"></span></span>
    <span class="country-meta">${formatNumber(country.qualified_views)} qualified · ${formatWatchTime(country.watch_time_ms)} watched</span>
  </li>`;
}

function countriesCard(report) {
  if (!report) return null;
  const total = report.countries.reduce((sum, country) => sum + Number(country.views || 0), 0);
  const top = report.countries.slice(0, TOP_COUNTRIES);
  const rest = report.countries.slice(TOP_COUNTRIES);
  const hint = report.geolocation_enabled
    ? 'Viewer locations resolved from IP addresses at playback start. Addresses are not stored.'
    : 'Country lookup is not configured, so views are reported as Unknown. Add a free GeoIP database (geolocation.database_path) or a trusted CDN country header.';
  const body = report.countries.length
    ? html`<ol class="country-list">${top.map((country) => countryRow(country, total))}</ol>
      ${rest.length > 0 && html`<details class="country-more"><summary>Show ${formatNumber(rest.length)} more countr${rest.length === 1 ? 'y' : 'ies'}</summary><ol class="country-list">${rest.map((country) => countryRow(country, total))}</ol></details>`}`
    : emptyState('No views in this range', 'Countries appear once viewers start playback.', { iconName: 'globe' });
  return html`<section class="card">
    <header class="card-header card-header-flush"><div><h2>Top countries</h2><p>${hint}</p></div></header>
    <div class="card-body">${body}</div>
  </section>`;
}

function isoDay(date) {
  return date.toISOString().slice(0, 10);
}

function presetQuery(days, mediaId) {
  const to = new Date();
  const from = new Date(to.getTime() - (days - 1) * 86_400_000);
  const query = new URLSearchParams({ from: isoDay(from), to: isoDay(to) });
  if (mediaId) query.set('media_id', mediaId);
  return `?${query}`;
}

function activePreset(from, to) {
  if (!from || !to) return null;
  const span = Math.round((Date.parse(to) - Date.parse(from)) / 86_400_000) + 1;
  return to === isoDay(new Date()) && PRESETS.includes(span) ? span : null;
}

function rangeBar(summary, source) {
  const mediaId = source.get('media_id') || '';
  const current = activePreset(summary.from, summary.to);
  return html`<div class="range-bar">
    <nav class="segmented" aria-label="Reporting range presets">
      ${PRESETS.map((days) => html`<a href="${presetQuery(days, mediaId)}"${current === days && html` aria-current="true"`}>${days} days</a>`)}
    </nav>
    <form method="get" class="range-form">
      ${field('From', html`<input name="from" type="date" value="${source.get('from') || summary.from}" max="${summary.to}">`)}
      ${field('To', html`<input name="to" type="date" value="${source.get('to') || summary.to}">`)}
      ${field('Media ID', html`<input name="media_id" value="${mediaId}" placeholder="All media" autocomplete="off">`)}
      <div class="toolbar-actions"><button class="button button-primary" type="submit">Apply</button><a class="button button-ghost" href="${config.routes.analytics}">Reset</a></div>
    </form>
  </div>`;
}

export async function renderAnalytics() {
  await currentSession();
  if (!config.features.analytics) {
    setPage(html`${pageHeader('Analytics', 'Audience measurement is disabled by server configuration.')}${notice('Enable analytics in the server configuration to collect playback and delivery events.')}`, 'Analytics');
    return;
  }
  const { source, target } = allowedQuery(['from', 'to', 'media_id']);
  const suffix = target.toString() ? `?${target}` : '';
  const [summary, series, countries] = await Promise.all([
    request(`${config.api.analytics_summary}${suffix}`),
    request(`${config.api.analytics_time_series}${suffix}`),
    request(`${config.api.analytics_countries}${suffix}`).catch(() => null),
  ]);
  const { totals } = summary;
  const completion = percent(totals.completed_views, totals.play_starts);
  const qualifiedRate = percent(totals.qualified_views, totals.views);
  const perView = totals.views > 0 ? totals.watch_time_ms / totals.views : 0;
  const rows = [...series.points].reverse().map((point) => html`<tr>
    ${cell('Day', html`<span class="cell-primary">${point.day}</span>`, { primary: true })}
    ${cell('Views', formatNumber(point.views))}
    ${cell('Unique', formatNumber(point.unique_viewers))}
    ${cell('Starts', formatNumber(point.play_starts))}
    ${cell('Completed', formatNumber(point.completed_views))}
    ${cell('Qualified', formatNumber(point.qualified_views))}
    ${cell('Watch time', formatDuration(point.watch_time_ms), { className: 'cell-nowrap' })}
    ${cell('Delivered', formatBytes(point.bytes_served), { className: 'cell-nowrap' })}
  </tr>`);

  setPage(html`${pageHeader('Analytics', `Persisted playback activity from ${summary.from} to ${summary.to}.`)}
    ${rangeBar(summary, source)}
    ${metricGrid([
      metricCard('Views', formatNumber(totals.views), { iconName: 'eye', emphasis: true, detail: `${formatNumber(totals.play_starts)} play starts` }),
      metricCard('Qualified views', formatNumber(totals.qualified_views), { iconName: 'target', detail: `${qualifiedRate.toFixed(1)}% of views`, hint: `Views with at least ${config.features.qualified_view_seconds} seconds of measured watch time (or half of shorter videos), one per viewer per day. This is the monetizable unit.` }),
      metricCard('Unique viewers', formatNumber(totals.unique_viewers), { iconName: 'users', detail: totals.unique_viewers > 0 ? `${(totals.views / totals.unique_viewers).toFixed(1)} views per viewer` : 'No viewers yet' }),
      metricCard('Completion rate', `${completion.toFixed(1)}%`, { iconName: 'check', detail: `${formatNumber(totals.completed_views)} completed views` }),
      metricCard('Watch time', formatWatchTime(totals.watch_time_ms), { iconName: 'clock', detail: totals.watch_time_ms >= 3_600_000 ? `${(totals.watch_time_ms / 3_600_000).toFixed(1)} hours in total` : `across ${formatNumber(totals.views)} view${totals.views === 1 ? '' : 's'}` }),
      metricCard('Avg. view duration', formatWatchTime(perView), { iconName: 'hourglass', detail: 'per view' }),
      metricCard('Countries', formatNumber(countries?.countries.filter((country) => country.country_code !== 'ZZ').length ?? 0), { iconName: 'globe', detail: countries?.geolocation_enabled ? 'with views' : 'lookup not configured' }),
      metricCard('Data delivered', formatBytes(totals.bytes_served), { iconName: 'transfer', detail: `${formatBytes(totals.views > 0 ? totals.bytes_served / totals.views : 0)} per view` }),
    ], { label: 'Totals for the selected range' })}
    <div class="split split-even analytics-panels page-section-tight">
      <section class="card"><div class="card-body">
        <h2 style="margin-bottom: var(--space-2)">Views by day</h2>
        ${barChart(series.points, 'views', 'views')}
      </div></section>
      ${countriesCard(countries)}
    </div>
    <details class="page-section card collapsible">
      <summary>Daily breakdown</summary>
      ${table(['Day', 'Views', 'Unique', 'Starts', 'Completed', 'Qualified', 'Watch time', 'Delivered'], rows, { caption: 'Daily analytics' })}
    </details>`, 'Analytics');
}
