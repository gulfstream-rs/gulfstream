import { currentSession, request } from '../core/api.js';
import { config, mediaPage } from '../core/config.js';
import { allowedQuery, escapeHtml, optionMarkup, setPage } from '../core/dom.js';
import { formatBytes, formatDate, formatDuration } from '../core/format.js';
import { badge, pageHeader, pagination, table } from '../core/ui.js';

function pageSizeOptions(selected) {
  const values = new Set(config.presentation.page_size_options.map(Number));
  values.add(Number(selected));
  return [...values].sort((left, right) => left - right).map((value) => `<option value="${value}"${Number(selected) === value ? ' selected' : ''}>${value}</option>`).join('');
}

export async function renderMediaList() {
  await currentSession();
  const { source, target } = allowedQuery(['status', 'visibility', 'search', 'page', 'page_size']);
  const result = await request(`${config.api.media}?${target.toString()}`);
  const rows = result.items.map((media) => `<tr>
    <td data-label="Title"><div class="table-primary"><a href="${escapeHtml(mediaPage(media.id))}">${escapeHtml(media.title)}</a></div><div class="table-secondary">${escapeHtml(media.source_filename)}</div></td>
    <td data-label="Status">${badge(media.status)}</td>
    <td data-label="Visibility">${badge(media.visibility)}</td>
    <td data-label="Duration">${media.duration_ms == null ? '—' : formatDuration(media.duration_ms)}</td>
    <td data-label="Storage">${formatBytes(media.storage_bytes)}</td>
    <td data-label="Created">${formatDate(media.created_at)}</td>
  </tr>`);

  setPage(`${pageHeader('Media library', 'Search, filter, inspect, edit, play, retry, or remove media.', `<a class="button button-primary" href="${escapeHtml(config.routes.upload)}">Upload video</a>`)}
    <section class="card"><div class="card-body"><form method="get" class="form-grid">
      <div class="form-row"><label class="field"><span>Search</span><input name="search" type="search" value="${escapeHtml(source.get('search') || '')}" placeholder="Title contains…"></label><label class="field"><span>Page size</span><select name="page_size">${pageSizeOptions(result.page_size)}</select></label></div>
      <div class="form-row"><label class="field"><span>Status</span><select name="status">${optionMarkup(config.options.media_statuses, source.get('status'), 'All statuses')}</select></label><label class="field"><span>Visibility</span><select name="visibility">${optionMarkup(config.options.media_visibilities, source.get('visibility'), 'All visibility')}</select></label></div>
      <div class="form-actions"><button class="button button-primary" type="submit">Apply filters</button><a class="button button-ghost" href="${escapeHtml(config.routes.media)}">Clear</a></div>
    </form></div></section>
    <section class="page-section">${table(['Title', 'Status', 'Visibility', 'Duration', 'Storage', 'Created'], rows, 'Upload your first video to populate the library.')}${pagination(result)}</section>`, 'Media');
}
