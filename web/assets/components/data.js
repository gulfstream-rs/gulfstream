import { html } from '../core/html.js';
import { formatNumber, percent } from '../core/format.js';
import { icon } from './icon.js';

const TONES = {
  success: ['ready', 'succeeded', 'public', 'active'],
  danger: ['failed', 'cancelled', 'deleting', 'revoked', 'disabled', 'suspended'],
  warning: ['queued', 'importing', 'processing', 'running', 'uploading', 'pending'],
};

export function statusTone(status) {
  for (const [tone, values] of Object.entries(TONES)) {
    if (values.includes(status)) return tone;
  }
  return 'info';
}

export function badge(status) {
  return html`<span class="badge" data-tone="${statusTone(status)}">${String(status ?? '').replaceAll('_', ' ')}</span>`;
}

export function metricCard(label, value, { detail, iconName, emphasis = false, hint } = {}) {
  return html`<section class="metric-card"${emphasis && html` data-emphasis`}${hint && html` title="${hint}"`}>
    <span class="metric-label">${iconName && icon(iconName)}${label}</span>
    <strong class="metric-value">${value}</strong>
    ${detail && html`<span class="metric-detail">${detail}</span>`}
  </section>`;
}

/**
 * Grid of metric cards whose column count depends on both the available
 * width (container queries) and the number of cards, so every row is full:
 * a card never ends up alone on the last row.
 * @param {unknown[]} cards trusted `metricCard` fragments
 */
export function metricGrid(cards, { label } = {}) {
  const items = cards.filter(Boolean);
  return html`<div class="balanced"><div class="balanced-grid metric-grid" data-count="${items.length}"${label && html` role="group" aria-label="${label}"`}>${items}</div></div>`;
}

/** Same balancing rules for arbitrary tiles (quick actions, etc.). */
export function balancedGrid(items, className = '') {
  const list = items.filter(Boolean);
  return html`<div class="balanced"><div class="balanced-grid ${className}" data-count="${list.length}">${list}</div></div>`;
}

export function statusBreakdown(items) {
  if (!items?.length) return html`<span class="metric-detail">No activity yet</span>`;
  return html`<div class="status-breakdown">${items.map((item) => html`<span class="status-breakdown-item">${badge(item.status)}${formatNumber(item.count)}</span>`)}</div>`;
}

export function progressBar(value, total, label) {
  const amount = percent(value, total);
  const tone = amount >= 95 ? 'danger' : amount >= 80 ? 'warning' : '';
  return html`<div class="progress"${tone && html` data-tone="${tone}"`} role="progressbar" aria-label="${label}" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${amount.toFixed(1)}"><span style="--progress:${amount.toFixed(2)}%"></span></div>`;
}

export function emptyState(title, message, { action, iconName = 'media' } = {}) {
  return html`<div class="empty-state">${icon(iconName)}<strong>${title}</strong><span>${message}</span>${action}</div>`;
}

/**
 * Responsive table. On phones rows become cards; each cell needs a label.
 * @param {string[]} headers
 * @param {unknown[]} rows trusted `<tr>` fragments
 */
export function table(headers, rows, { empty, caption } = {}) {
  if (!rows.length) return empty ?? emptyState('Nothing here yet', 'No records found.');
  return html`<div class="table-wrap"><table>
    ${caption && html`<caption class="sr-only">${caption}</caption>`}
    <thead><tr>${headers.map((header) => html`<th scope="col">${header}</th>`)}</tr></thead>
    <tbody>${rows}</tbody>
  </table></div>`;
}

/** Table cell; `primary` cells become the card heading on phones. */
export function cell(label, content, { primary = false, className = '' } = {}) {
  return html`<td data-label="${label}"${primary && html` data-primary`}${className && html` class="${className}"`}>${content}</td>`;
}

export function pagination(result) {
  const current = Number(result.page);
  const pageSize = Number(result.page_size);
  const total = Number(result.total);
  const pages = Math.max(1, Math.ceil(total / pageSize));
  const summary = `${formatNumber(total)} result${total === 1 ? '' : 's'}`;
  if (pages <= 1) return html`<p class="pagination-meta">${summary}</p>`;
  const link = (target, label, iconName, rel) => {
    const query = new URLSearchParams(window.location.search);
    query.set('page', String(target));
    query.set('page_size', String(pageSize));
    return html`<a class="button button-sm" rel="${rel}" href="?${query.toString()}">${iconName === 'chevron-left' && icon(iconName, 'icon-sm')}${label}${iconName === 'chevron-right' && icon(iconName, 'icon-sm')}</a>`;
  };
  return html`<nav class="pagination" aria-label="Pagination">
    <span class="pagination-meta">Page ${formatNumber(current)} of ${formatNumber(pages)} · ${summary}</span>
    <div class="actions">
      ${current > 1 && link(current - 1, 'Previous', 'chevron-left', 'prev')}
      ${current < pages && link(current + 1, 'Next', 'chevron-right', 'next')}
    </div>
  </nav>`;
}

