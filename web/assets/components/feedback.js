import { app } from '../core/dom.js';
import { html } from '../core/html.js';
import { icon } from './icon.js';
import { pageHeader } from './layout.js';

export function notice(message, tone = '') {
  const toneClass = tone ? ` notice-${tone}` : '';
  return html`<div class="notice${toneClass}"${tone === 'error' ? html` role="alert"` : ''}>${message}</div>`;
}

export function setStatus(element, message, state = '') {
  if (!element) return;
  element.textContent = message;
  if (state) element.dataset.state = state;
  else delete element.dataset.state;
}

/** Disables a button and swaps its content for a spinner + label while busy. */
export function setBusy(button, busy, label = 'Working…') {
  if (!button) return;
  if (busy) {
    if (button.dataset.busy) return;
    button.dataset.busy = 'true';
    button.dataset.originalHtml = button.innerHTML;
    button.innerHTML = String(html`<span class="spinner" aria-hidden="true"></span><span>${label}</span>`);
    button.disabled = true;
  } else {
    if (button.dataset.originalHtml != null) button.innerHTML = button.dataset.originalHtml;
    button.disabled = false;
    delete button.dataset.busy;
    delete button.dataset.originalHtml;
  }
}

export function toast(message, tone = 'success') {
  const region = document.querySelector('#toast-region');
  if (!region) return;
  const node = document.createElement('div');
  node.className = 'toast';
  node.dataset.tone = tone;
  node.setAttribute('role', tone === 'error' ? 'alert' : 'status');
  node.textContent = message;
  region.append(node);
  window.setTimeout(() => node.remove(), tone === 'error' ? 7000 : 4500);
}

export function announce(message) {
  const region = document.querySelector('#live-region');
  if (region) region.textContent = message;
}

export function confirmAction(title, message, confirmLabel = 'Confirm') {
  const dialog = document.querySelector('#confirm-dialog');
  if (!(dialog instanceof HTMLDialogElement)) return Promise.resolve(window.confirm(message));
  dialog.querySelector('[data-dialog-title]').textContent = title;
  dialog.querySelector('[data-dialog-message]').textContent = message;
  const confirm = dialog.querySelector('[data-dialog-confirm]');
  if (confirm) confirm.textContent = confirmLabel;
  dialog.returnValue = '';
  return new Promise((resolve) => {
    dialog.addEventListener('close', () => resolve(dialog.returnValue === 'confirm'), { once: true });
    dialog.showModal();
  });
}

export function showFatal(error) {
  const message = error instanceof Error ? error.message : String(error);
  app.setAttribute('aria-busy', 'false');
  app.innerHTML = String(html`${pageHeader('Something went wrong', 'This page could not be loaded.')}
    ${notice(message, 'error')}
    <div class="actions" style="margin-top: var(--space-5)">
      <button class="button" type="button" data-reload>${icon('retry', 'icon-sm')}Try again</button>
    </div>`);
  app.querySelector('[data-reload]')?.addEventListener('click', () => window.location.reload());
}
