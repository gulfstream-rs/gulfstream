import { currentSession, request } from '../core/api.js';
import { config, mediaPage } from '../core/config.js';
import { allowedQuery, setPage } from '../core/dom.js';
import { html } from '../core/html.js';
import { formatBytes, formatDate, formatDuration } from '../core/format.js';
import { badge, cell, emptyState, pagination, table } from '../components/data.js';
import { autoSubmitSelects, field, filterToolbar, options, pageSizeOptions, searchField } from '../components/forms.js';
import { icon } from '../components/icon.js';
import { pageHeader } from '../components/layout.js';

const FILTER_KEYS = ['status', 'visibility', 'search', 'page', 'page_size'];

function row(media) {
  return html`<tr class="row-link">
    ${cell('Title', html`<a class="cell-primary stretched" href="${mediaPage(media.id)}">${media.title}</a><div class="cell-secondary">${media.source_filename}</div>`, { primary: true })}
    ${cell('Status', badge(media.status))}
    ${cell('Visibility', badge(media.visibility))}
    ${cell('Duration', media.duration_ms == null ? '—' : formatDuration(media.duration_ms), { className: 'cell-nowrap' })}
    ${cell('Size', formatBytes(media.storage_bytes), { className: 'cell-nowrap' })}
    ${cell('Created', formatDate(media.created_at), { className: 'cell-nowrap' })}
  </tr>`;
}

export async function renderMediaList() {
  await currentSession();
  const { source, target } = allowedQuery(FILTER_KEYS);
  const result = await request(`${config.api.media}?${target}`);
  const filtered = ['status', 'visibility', 'search'].some((key) => source.get(key));
  const activeCount = ['status', 'visibility'].filter((key) => source.get(key)).length;
  const upload = html`<a class="button button-primary" href="${config.routes.upload}">${icon('upload', 'icon-sm')}Upload video</a>`;
  const empty = filtered
    ? emptyState('No matching media', 'Try a different search or reset the filters.', { iconName: 'search', action: html`<a class="button" href="${config.routes.media}">Reset filters</a>` })
    : emptyState('Your library is empty', 'Upload a video or import one from a URL to get started.', { action: upload });

  setPage(html`${pageHeader('Media library', 'Search, inspect, play, and manage every video in your account.', { actions: upload })}
    ${filterToolbar({
      primary: searchField('search', source.get('search') || '', 'Search titles…'),
      activeCount,
      resetHref: config.routes.media,
      filters: html`
        ${field('Status', html`<select name="status">${options(config.options.media_statuses, source.get('status'), 'All statuses')}</select>`)}
        ${field('Visibility', html`<select name="visibility">${options(config.options.media_visibilities, source.get('visibility'), 'Any visibility')}</select>`)}
        ${field('Page size', html`<select name="page_size">${pageSizeOptions(result.page_size)}</select>`)}`,
    })}
    ${table(['Title', 'Status', 'Visibility', 'Duration', 'Size', 'Created'], result.items.map(row), { empty, caption: 'Media' })}
    ${result.items.length > 0 && pagination(result)}`, 'Media');

  autoSubmitSelects(document.querySelector('form.toolbar'));
}
