import { currentSession, login, request } from '../core/api.js';
import { config } from '../core/config.js';
import { query, setPage } from '../core/dom.js';
import { html } from '../core/html.js';
import { notice, setBusy, setStatus } from '../components/feedback.js';
import { field } from '../components/forms.js';
import { authCard, signedInCard } from '../components/auth.js';

export async function renderRegister() {
  const existing = await currentSession(false);
  if (existing) {
    setPage(signedInCard(existing), 'Create account');
    return;
  }
  const signIn = html`Already have an account? <a href="${config.routes.login}">Sign in</a>`;
  if (config.registration.mode === 'disabled') {
    setPage(authCard('Registration is closed', 'New accounts cannot be created on this server.', notice('Contact the server administrator for access.'), signIn), 'Create account');
    return;
  }

  const { limits } = config;
  const adminField = config.registration.mode === 'admin_token'
    ? field('Registration token', html`<input name="admin_token" type="password" autocomplete="off" required>`, { hint: 'Provided by the server administrator.' })
    : null;
  setPage(authCard('Create account', 'Start uploading, streaming, and measuring video.', html`<form id="registration-form" class="form-grid" novalidate>
      ${field('Display name', html`<input name="display_name" maxlength="${limits.maximum_display_name_bytes}" autocomplete="name" autofocus required>`)}
      ${field('Email', html`<input name="email" type="email" inputmode="email" autocomplete="email" required>`)}
      ${field('Password', html`<input name="password" type="password" autocomplete="new-password" minlength="${limits.minimum_password_bytes}" maxlength="${limits.maximum_password_bytes}" required>`, { hint: `At least ${limits.minimum_password_bytes} characters.` })}
      ${adminField}
      <p id="form-status" class="form-status" role="status" aria-live="polite"></p>
      <button class="button button-primary button-lg button-block" type="submit">Create account</button>
    </form>`, signIn), 'Create account');

  const form = query('#registration-form');
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const status = query('#form-status');
    if (!form.reportValidity()) return;
    const button = form.querySelector('button[type="submit"]');
    const data = new FormData(form);
    const headers = { 'content-type': 'application/json' };
    if (config.registration.mode === 'admin_token') headers[config.registration.admin_header_name] = data.get('admin_token');
    const email = String(data.get('email')).trim();
    try {
      setBusy(button, true, 'Creating account…');
      setStatus(status, '');
      await request(config.api.accounts, {
        method: 'POST',
        headers,
        body: JSON.stringify({
          email,
          display_name: String(data.get('display_name')).trim(),
          password: data.get('password'),
        }),
      });
      await login(email, data.get('password'));
      window.location.assign(config.routes.dashboard);
    } catch (error) {
      setStatus(status, error.message, 'error');
      setBusy(button, false);
    }
  });
}
