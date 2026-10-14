import { currentSession, request } from '../core/api.js';
import { config, mediaPage } from '../core/config.js';
import { allowedQuery, labelize, replaceContent, setPage } from '../core/dom.js';
import { html } from '../core/html.js';
import { formatDate } from '../core/format.js';
import { badge, cell, emptyState, pagination, table } from '../components/data.js';
import { autoSubmitSelects, field, filterToolbar, options, pageSizeOptions } from '../components/forms.js';
import { pageHeader } from '../components/layout.js';
import { autoRefresh, liveIndicator } from '../components/refresh.js';

const FILTER_KEYS = ['status', 'kind', 'page', 'page_size'];

function row(job) {
  return html`<tr class="row-link">
    ${cell('Media', html`<a class="cell-primary stretched" href="${mediaPage(job.media_id)}">${job.media_title}</a><div class="cell-secondary">${labelize(job.kind)}</div>`, { primary: true })}
    ${cell('Status', badge(job.status))}
    ${cell('Attempts', `${job.attempts} / ${job.maximum_attempts}`, { className: 'cell-nowrap' })}
    ${cell('Run after', formatDate(job.run_after), { className: 'cell-nowrap' })}
    ${cell('Updated', formatDate(job.updated_at), { className: 'cell-nowrap' })}
    ${job.error_message ? cell('Error', html`<span class="cell-error">${job.error_message}</span>`) : html`<td data-label="Error" data-optional-empty>—</td>`}
  </tr>`;
}

function jobsTable(result, filtered) {
  const empty = filtered
    ? emptyState('No matching jobs', 'No processing jobs match these filters.', { iconName: 'jobs', action: html`<a class="button" href="${config.routes.jobs}">Reset filters</a>` })
    : emptyState('The queue is empty', 'Imports, conversions, and deletions will appear here.', { iconName: 'jobs' });
  return html`${table(['Media', 'Status', 'Attempts', 'Run after', 'Updated', 'Error'], result.items.map(row), { empty, caption: 'Processing jobs' })}
    ${result.items.length > 0 && pagination(result)}`;
}

export async function renderJobs() {
  await currentSession();
  const { source, target } = allowedQuery(FILTER_KEYS);
  const url = `${config.api.jobs}?${target}`;
  const result = await request(url);
  const activeCount = ['status', 'kind'].filter((key) => source.get(key)).length;

  setPage(html`${pageHeader('Processing', 'Durable imports, conversions, and deletions with retries and failures.', {
    meta: liveIndicator(config.presentation.jobs_refresh_seconds),
  })}
    ${filterToolbar({
      activeCount,
      resetHref: config.routes.jobs,
      filters: html`
        ${field('Status', html`<select name="status">${options(config.options.job_statuses, source.get('status'), 'All statuses')}</select>`)}
        ${field('Type', html`<select name="kind">${options(config.options.job_kinds, source.get('kind'), 'All job types')}</select>`)}
        ${field('Page size', html`<select name="page_size">${pageSizeOptions(result.page_size)}</select>`)}`,
    })}
    <section id="jobs-content">${jobsTable(result, activeCount > 0)}</section>`, 'Processing');

  autoSubmitSelects(document.querySelector('form.toolbar'));
  autoRefresh(async () => {
    replaceContent(document.querySelector('#jobs-content'), jobsTable(await request(url), activeCount > 0));
  }, config.presentation.jobs_refresh_seconds);
}
