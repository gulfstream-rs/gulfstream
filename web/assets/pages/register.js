import { currentSession, login, request } from '../core/api.js';
import { config } from '../core/config.js';
import { escapeHtml, query, setPage } from '../core/dom.js';
import { pageHeader, setBusy, setStatus } from '../core/ui.js';

export async function renderRegister() {
  const existing = await currentSession(false);
  if (existing) {
    setPage(`${pageHeader('Create account', 'A browser session is already active.')}<div class="card"><div class="card-body"><p>Signed in as <strong>${escapeHtml(existing.account.email)}</strong>.</p><a class="button button-primary" href="${escapeHtml(config.routes.dashboard)}">Open dashboard</a></div></div>`, 'Create account');
    return;
  }
  if (config.registration.mode === 'disabled') {
    setPage(`${pageHeader('Create account', 'Registration is not available on this server.')}<div class="notice">Contact the server administrator for access.</div>`, 'Create account');
    return;
  }

  const adminField = config.registration.mode === 'admin_token'
    ? `<label class="field"><span>Registration token</span><input name="admin_token" type="password" autocomplete="off" required><span class="field-hint">Provided by the server administrator.</span></label>`
    : '';
  setPage(`<div class="auth-layout"><section class="card auth-card"><div class="card-header"><div><h1>Create account</h1><p>Start uploading and processing video.</p></div></div><div class="card-body"><form id="registration-form" class="form-grid">
    <label class="field"><span>Email</span><input name="email" type="email" autocomplete="email" required></label>
    <label class="field"><span>Display name</span><input name="display_name" maxlength="${config.limits.maximum_display_name_bytes}" autocomplete="name" required></label>
    <label class="field"><span>Password</span><input name="password" type="password" autocomplete="new-password" minlength="${config.limits.minimum_password_bytes}" maxlength="${config.limits.maximum_password_bytes}" required><span class="field-hint">Use at least ${config.limits.minimum_password_bytes} characters.</span></label>
    ${adminField}
    <div class="form-actions"><button class="button button-primary" type="submit">Create account</button><a class="button button-ghost" href="${escapeHtml(config.routes.login)}">I already have an account</a></div>
    <p id="form-status" class="form-status" role="status"></p>
  </form></div></section></div>`, 'Create account');

  const form = query('#registration-form');
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const status = query('#form-status');
    const button = form.querySelector('button[type="submit"]');
    const data = new FormData(form);
    const headers = { 'content-type': 'application/json' };
    if (config.registration.mode === 'admin_token') headers[config.registration.admin_header_name] = data.get('admin_token');
    try {
      setBusy(button, true, 'Creating…');
      setStatus(status, 'Creating your account…');
      await request(config.api.accounts, {
        method: 'POST',
        headers,
        body: JSON.stringify({
          email: data.get('email'),
          display_name: data.get('display_name'),
          password: data.get('password'),
        }),
      });
      await login(data.get('email'), data.get('password'));
      window.location.assign(config.routes.dashboard);
    } catch (error) {
      setStatus(status, error.message, 'error');
      setBusy(button, false);
    }
  });
}
