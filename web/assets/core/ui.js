import { app, escapeHtml } from './dom.js';
import { formatNumber } from './format.js';

export function pageHeader(title, description, actions = '') {
  return `<header class="page-header"><div class="page-header-copy"><h1>${escapeHtml(title)}</h1><p>${escapeHtml(description)}</p></div>${actions ? `<div class="actions">${actions}</div>` : ''}</header>`;
}

export function metricCard(label, value, detail = '') {
  return `<section class="metric-card"><span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong>${detail ? `<small>${detail}</small>` : ''}</section>`;
}

export function statusTone(status) {
  if (['ready', 'succeeded', 'public'].includes(status)) return 'success';
  if (['failed', 'cancelled', 'deleting'].includes(status)) return 'danger';
  if (['queued', 'importing', 'processing', 'running'].includes(status)) return 'warning';
  return 'info';
}

export function badge(status) {
  return `<span class="badge" data-tone="${statusTone(status)}">${escapeHtml(String(status).replaceAll('_', ' '))}</span>`;
}

export function statusBreakdown(items) {
  if (!items?.length) return '<span class="field-hint">No activity</span>';
  return `<div class="status-breakdown">${items.map((item) => `${badge(item.status)} <span>${formatNumber(item.count)}</span>`).join('')}</div>`;
}

export function emptyState(title, message, action = '') {
  return `<div class="empty-state"><strong>${escapeHtml(title)}</strong><span>${escapeHtml(message)}</span>${action}</div>`;
}

export function table(headers, rows, emptyMessage = 'No records found.') {
  if (!rows.length) return emptyState('Nothing here yet', emptyMessage);
  return `<div class="table-wrap"><table><thead><tr>${headers.map((header) => `<th scope="col">${escapeHtml(header)}</th>`).join('')}</tr></thead><tbody>${rows.join('')}</tbody></table></div>`;
}

export function pagination(result) {
  const current = Number(result.page);
  const pageSize = Number(result.page_size);
  const total = Number(result.total);
  const pages = Math.max(1, Math.ceil(total / pageSize));
  if (pages <= 1) return `<p class="pagination-meta">${formatNumber(total)} result${total === 1 ? '' : 's'}</p>`;
  const link = (target, label) => {
    const query = new URLSearchParams(window.location.search);
    query.set('page', String(target));
    query.set('page_size', String(pageSize));
    return `<a class="button" href="?${escapeHtml(query.toString())}">${escapeHtml(label)}</a>`;
  };
  return `<nav class="pagination" aria-label="Pagination"><span class="pagination-meta">Page ${formatNumber(current)} of ${formatNumber(pages)} · ${formatNumber(total)} results</span><div class="actions">${current > 1 ? link(current - 1, 'Previous') : ''}${current < pages ? link(current + 1, 'Next') : ''}</div></nav>`;
}

export function setStatus(element, message, state = '') {
  element.textContent = message;
  if (state) element.dataset.state = state;
  else delete element.dataset.state;
}

export function setBusy(button, busy, label = 'Working…') {
  if (busy) {
    button.dataset.originalLabel = button.textContent;
    button.textContent = label;
    button.disabled = true;
  } else {
    button.textContent = button.dataset.originalLabel || button.textContent;
    button.disabled = false;
    delete button.dataset.originalLabel;
  }
}

export function toast(message, tone = 'success') {
  const region = document.querySelector('#toast-region');
  const node = document.createElement('div');
  node.className = 'toast';
  node.dataset.tone = tone;
  node.textContent = message;
  region?.append(node);
  window.setTimeout(() => node.remove(), 5000);
}

export function announce(message) {
  const region = document.querySelector('#live-region');
  if (region) region.textContent = message;
}

export function confirmAction(title, message) {
  const dialog = document.querySelector('#confirm-dialog');
  if (!(dialog instanceof HTMLDialogElement)) return Promise.resolve(window.confirm(message));
  dialog.querySelector('[data-dialog-title]').textContent = title;
  dialog.querySelector('[data-dialog-message]').textContent = message;
  return new Promise((resolve) => {
    const close = () => {
      dialog.removeEventListener('close', close);
      resolve(dialog.returnValue === 'confirm');
    };
    dialog.addEventListener('close', close);
    dialog.showModal();
  });
}

export function showFatal(error) {
  const message = error instanceof Error ? error.message : String(error);
  app.setAttribute('aria-busy', 'false');
  app.innerHTML = `${pageHeader('Something went wrong', 'The page could not be loaded.')}<div class="notice notice-error" role="alert">${escapeHtml(message)}</div>`;
}

export function autoRefresh(callback, seconds) {
  const interval = window.setInterval(() => {
    if (document.visibilityState === 'visible') callback();
  }, Number(seconds) * 1000);
  window.addEventListener('pagehide', () => window.clearInterval(interval), { once: true });
  return interval;
}
