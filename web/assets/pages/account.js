import { clearSession, currentSession, jsonRequest, request } from '../core/api.js';
import { config } from '../core/config.js';
import { escapeHtml, query, setPage } from '../core/dom.js';
import { formatBytes, formatDate, percent } from '../core/format.js';
import { badge, confirmAction, pageHeader, setBusy, setStatus, table, toast } from '../core/ui.js';

export async function renderAccount() {
  const session = await currentSession();
  const keys = await request(config.api.api_keys);
  const account = session.account;
  const storagePercent = percent(account.storage_used_bytes, account.storage_quota_bytes);
  const rows = keys.map((key) => `<tr><td data-label="Name"><strong>${escapeHtml(key.name)}</strong></td><td data-label="Created">${formatDate(key.created_at)}</td><td data-label="Last used">${formatDate(key.last_used_at)}</td><td data-label="Status">${key.revoked_at ? badge('revoked') : badge('active')}</td><td data-label="Action">${key.revoked_at ? `<span class="field-hint">Revoked ${formatDate(key.revoked_at)}</span>` : `<button class="button button-danger" type="button" data-revoke="${escapeHtml(key.id)}">Revoke</button>`}</td></tr>`);

  setPage(`${pageHeader('Account', 'Update your profile, review quota usage, and manage revocable API credentials.')}
    <div class="two-column"><section class="card"><div class="card-header"><div><h2>Profile</h2><p>Browser and API requests share this account identity.</p></div></div><div class="card-body"><form id="profile-form" class="form-grid"><label class="field"><span>Email</span><input value="${escapeHtml(account.email)}" disabled></label><label class="field"><span>Display name</span><input name="display_name" value="${escapeHtml(account.display_name)}" maxlength="${config.limits.maximum_display_name_bytes}" required></label><div class="form-actions"><button class="button button-primary" type="submit">Save profile</button><button class="button" type="button" id="logout-button">Sign out</button></div><p id="profile-status" class="form-status" role="status"></p></form></div></section>
    <aside class="card"><div class="card-header"><div><h2>Storage quota</h2><p>${storagePercent.toFixed(1)}% used</p></div></div><div class="card-body"><div class="progress" role="progressbar" aria-label="Storage quota used" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${storagePercent.toFixed(1)}"><span style="--progress:${storagePercent.toFixed(2)}%"></span></div><p class="field-hint">${formatBytes(account.storage_used_bytes)} of ${formatBytes(account.storage_quota_bytes)}</p><dl class="detail-list"><dt>Account status</dt><dd>${badge(account.status)}</dd><dt>Created</dt><dd>${formatDate(account.created_at)}</dd><dt>Session expires</dt><dd>${formatDate(session.expires_at)}</dd></dl></div></aside></div>
    <section class="page-section card"><div class="card-header"><div><h2>Create API key</h2><p>The plaintext credential is displayed once and is never stored.</p></div></div><div class="card-body"><form id="key-form" class="form-grid"><label class="field"><span>Key name</span><input name="name" maxlength="${config.limits.maximum_api_key_name_bytes}" placeholder="Production uploader" required></label><div class="form-actions"><button class="button button-primary" type="submit">Create key</button></div><p id="key-status" class="form-status" role="status"></p><div id="key-display" hidden></div></form></div></section>
    <section class="page-section"><div><h2>API keys</h2><p class="field-hint">Revoke credentials that are no longer needed or may have been exposed.</p></div>${table(['Name', 'Created', 'Last used', 'Status', 'Action'], rows, 'No API keys have been created.')}</section>`, 'Account');

  const profileForm = query('#profile-form');
  profileForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    const button = profileForm.querySelector('button[type="submit"]');
    const status = query('#profile-status');
    try {
      setBusy(button, true, 'Saving…');
      await request(config.api.account, jsonRequest('PATCH', Object.fromEntries(new FormData(profileForm))));
      toast('Profile updated.');
      setStatus(status, 'Saved.', 'success');
      setBusy(button, false);
    } catch (error) {
      setStatus(status, error.message, 'error');
      setBusy(button, false);
    }
  });

  const keyForm = query('#key-form');
  keyForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    const button = keyForm.querySelector('button[type="submit"]');
    const status = query('#key-status');
    try {
      setBusy(button, true, 'Creating…');
      const key = await request(config.api.api_keys, jsonRequest('POST', Object.fromEntries(new FormData(keyForm))));
      const display = query('#key-display');
      display.hidden = false;
      display.className = 'key-display';
      display.innerHTML = `<strong>Copy this key now</strong><code>${escapeHtml(key.api_key)}</code><button class="button" type="button" id="copy-key">Copy key</button>`;
      query('#copy-key').addEventListener('click', async () => {
        await navigator.clipboard.writeText(key.api_key);
        toast('API key copied.');
      });
      keyForm.reset();
      setStatus(status, 'API key created.', 'success');
      setBusy(button, false);
    } catch (error) {
      setStatus(status, error.message, 'error');
      setBusy(button, false);
    }
  });

  for (const button of document.querySelectorAll('[data-revoke]')) {
    button.addEventListener('click', async () => {
      const confirmed = await confirmAction('Revoke API key', 'Clients using this key will lose access immediately.');
      if (!confirmed) return;
      await request(`${config.api.api_keys}/${encodeURIComponent(button.dataset.revoke)}`, { method: 'DELETE' });
      window.location.reload();
    });
  }

  query('#logout-button').addEventListener('click', async () => {
    await request(config.api.logout, { method: 'POST' });
    clearSession();
    window.location.assign(config.routes.login);
  });
}
