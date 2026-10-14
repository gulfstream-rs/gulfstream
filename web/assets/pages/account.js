import { currentSession, jsonRequest, logout, request } from '../core/api.js';
import { config } from '../core/config.js';
import { copyText, query, replaceContent, setPage } from '../core/dom.js';
import { html } from '../core/html.js';
import { formatBytes, formatDate, percent } from '../core/format.js';
import { badge, cell, emptyState, progressBar, table } from '../components/data.js';
import { confirmAction, setBusy, setStatus, toast } from '../components/feedback.js';
import { field } from '../components/forms.js';
import { icon } from '../components/icon.js';
import { pageHeader, sectionHeading } from '../components/layout.js';

function keyRow(key) {
  return html`<tr>
    ${cell('Name', html`<span class="cell-primary">${key.name}</span>`, { primary: true })}
    ${cell('Status', key.revoked_at ? badge('revoked') : badge('active'))}
    ${cell('Created', formatDate(key.created_at), { className: 'cell-nowrap' })}
    ${cell('Last used', key.last_used_at ? formatDate(key.last_used_at) : 'Never', { className: 'cell-nowrap' })}
    ${cell('', key.revoked_at
      ? html`<span class="field-hint">Revoked ${formatDate(key.revoked_at)}</span>`
      : html`<button class="button button-danger-ghost button-sm" type="button" data-revoke="${key.id}" data-name="${key.name}">Revoke</button>`)}
  </tr>`;
}

function keysTable(keys) {
  return table(['Name', 'Status', 'Created', 'Last used', ''], keys.map(keyRow), {
    caption: 'API keys',
    empty: emptyState('No API keys', 'Create a key to automate uploads and management through the API.', { iconName: 'key' }),
  });
}

export async function renderAccount() {
  const session = await currentSession();
  const keys = await request(config.api.api_keys);
  const { account } = session;
  const { limits } = config;
  const used = percent(account.storage_used_bytes, account.storage_quota_bytes);

  setPage(html`${pageHeader('Account', 'Manage your profile, storage, sessions, and API credentials.', {
    actions: html`<button class="button" type="button" id="logout-button">${icon('logout', 'icon-sm')}Sign out</button>`,
  })}
    <div class="split">
      <section class="card"><form id="profile-form" class="card-body form-grid">
        <h2>Profile</h2>
        ${field('Email', html`<input value="${account.email}" disabled>`, { hint: 'Email changes are not supported.' })}
        ${field('Display name', html`<input name="display_name" value="${account.display_name}" maxlength="${limits.maximum_display_name_bytes}" autocomplete="name" required>`)}
        <div class="form-actions"><button class="button button-primary" type="submit">Save profile</button><p id="profile-status" class="form-status" role="status" aria-live="polite"></p></div>
      </form></section>
      <aside class="card"><div class="card-body form-grid">
        <div class="metric-label">${icon('storage')}Storage</div>
        <div class="storage-meter-figures"><strong>${formatBytes(account.storage_used_bytes)}</strong><span class="muted">of ${formatBytes(account.storage_quota_bytes)} · ${used.toFixed(1)}%</span></div>
        ${progressBar(account.storage_used_bytes, account.storage_quota_bytes, 'Storage quota used')}
        <dl class="detail-list">
          <dt>Status</dt><dd>${badge(account.status)}</dd>
          <dt>Member since</dt><dd>${formatDate(account.created_at)}</dd>
          <dt>Session expires</dt><dd>${formatDate(session.expires_at)}</dd>
        </dl>
      </div></aside>
    </div>

    <section class="page-section">
      ${sectionHeading('API keys', 'Bearer credentials for automation. Revoke keys that are unused or may be exposed.')}
      <section class="card"><form id="key-form" class="card-body form-grid">
        <div class="form-row" style="align-items:end">
          ${field('New key name', html`<input name="name" maxlength="${limits.maximum_api_key_name_bytes}" placeholder="e.g. Production uploader" required>`)}
          <div class="form-actions"><button class="button button-primary" type="submit">${icon('key', 'icon-sm')}Create key</button></div>
        </div>
        <p id="key-status" class="form-status" role="status" aria-live="polite"></p>
        <div id="key-display" class="key-display" hidden></div>
      </form></section>
      <div id="keys-table">${keysTable(keys)}</div>
    </section>`, 'Account');

  const profileForm = query('#profile-form');
  profileForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    if (!profileForm.reportValidity()) return;
    const button = profileForm.querySelector('button[type="submit"]');
    const status = query('#profile-status');
    try {
      setBusy(button, true, 'Saving…');
      setStatus(status, '');
      await request(config.api.account, jsonRequest('PATCH', Object.fromEntries(new FormData(profileForm))));
      await currentSession(false);
      toast('Profile updated.');
      setStatus(status, 'Saved.', 'success');
    } catch (error) {
      setStatus(status, error.message, 'error');
    } finally {
      setBusy(button, false);
    }
  });

  const reloadKeys = async () => replaceContent(query('#keys-table'), keysTable(await request(config.api.api_keys)));

  const keyForm = query('#key-form');
  keyForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    if (!keyForm.reportValidity()) return;
    const button = keyForm.querySelector('button[type="submit"]');
    const status = query('#key-status');
    try {
      setBusy(button, true, 'Creating…');
      setStatus(status, '');
      const key = await request(config.api.api_keys, jsonRequest('POST', Object.fromEntries(new FormData(keyForm))));
      const display = query('#key-display');
      display.hidden = false;
      replaceContent(display, html`<strong>Copy this key now — it will not be shown again.</strong>
        <code>${key.api_key}</code>
        <div class="actions"><button class="button" type="button" id="copy-key">${icon('copy', 'icon-sm')}Copy key</button></div>`);
      query('#copy-key').addEventListener('click', async () => {
        try {
          await copyText(key.api_key);
          toast('API key copied.');
        } catch (error) { toast(error.message, 'error'); }
      });
      keyForm.reset();
      await reloadKeys();
    } catch (error) {
      setStatus(status, error.message, 'error');
    } finally {
      setBusy(button, false);
    }
  });

  query('#keys-table').addEventListener('click', async (event) => {
    const button = event.target instanceof Element ? event.target.closest('[data-revoke]') : null;
    if (!button) return;
    const confirmed = await confirmAction('Revoke API key?', `Clients using “${button.dataset.name}” will lose access immediately.`, 'Revoke key');
    if (!confirmed) return;
    setBusy(button, true, 'Revoking…');
    try {
      await request(`${config.api.api_keys}/${encodeURIComponent(button.dataset.revoke)}`, { method: 'DELETE' });
      toast('API key revoked.');
      await reloadKeys();
    } catch (error) {
      setBusy(button, false);
      toast(error.message, 'error');
    }
  });

  query('#logout-button').addEventListener('click', async () => {
    try {
      await logout();
    } finally {
      window.location.assign(config.routes.login);
    }
  });
}
