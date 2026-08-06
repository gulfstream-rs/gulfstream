import { currentSession, request } from '../core/api.js';
import { config, mediaPage } from '../core/config.js';
import { allowedQuery, escapeHtml, optionMarkup, setPage } from '../core/dom.js';
import { formatDate } from '../core/format.js';
import { autoRefresh, badge, pageHeader, pagination, table } from '../core/ui.js';

function pageSizeOptions(selected) {
  const values = new Set(config.presentation.page_size_options.map(Number));
  values.add(Number(selected));
  return [...values].sort((left, right) => left - right).map((value) => `<option value="${value}"${Number(selected) === value ? ' selected' : ''}>${value}</option>`).join('');
}

function jobsTable(result) {
  const rows = result.items.map((job) => `<tr><td data-label="Media"><div class="table-primary"><a href="${escapeHtml(mediaPage(job.media_id))}">${escapeHtml(job.media_title)}</a></div><div class="table-secondary">${escapeHtml(job.kind.replaceAll('_', ' '))}</div></td><td data-label="Status">${badge(job.status)}</td><td data-label="Attempts">${job.attempts}/${job.maximum_attempts}</td><td data-label="Run after">${formatDate(job.run_after)}</td><td data-label="Updated">${formatDate(job.updated_at)}</td><td data-label="Error"><span class="processing-error">${escapeHtml(job.error_message || '—')}</span></td></tr>`);
  return `${table(['Media', 'Status', 'Attempts', 'Run after', 'Updated', 'Error'], rows, 'No processing jobs match these filters.')}${pagination(result)}`;
}

export async function renderJobs() {
  await currentSession();
  const { source, target } = allowedQuery(['status', 'kind', 'page', 'page_size']);
  const result = await request(`${config.api.jobs}?${target.toString()}`);
  setPage(`${pageHeader('Processing', 'Inspect durable imports, transcodes, deletion jobs, retries, and failures.')}
    <section class="card"><div class="card-body"><form method="get" class="form-grid"><div class="form-row"><label class="field"><span>Status</span><select name="status">${optionMarkup(config.options.job_statuses, source.get('status'), 'All statuses')}</select></label><label class="field"><span>Kind</span><select name="kind">${optionMarkup(config.options.job_kinds, source.get('kind'), 'All job types')}</select></label></div><div class="form-row"><label class="field"><span>Page size</span><select name="page_size">${pageSizeOptions(result.page_size)}</select></label><div></div></div><div class="form-actions"><button class="button button-primary" type="submit">Apply filters</button><a class="button button-ghost" href="${escapeHtml(config.routes.jobs)}">Clear</a></div></form></div></section>
    <section id="jobs-content" class="page-section">${jobsTable(result)}</section>`, 'Processing');

  autoRefresh(async () => {
    try {
      const next = await request(`${config.api.jobs}?${target.toString()}`);
      const content = document.querySelector('#jobs-content');
      if (content) content.innerHTML = jobsTable(next);
    } catch { /* Keep the last successful queue snapshot. */ }
  }, config.presentation.jobs_refresh_seconds);
}
