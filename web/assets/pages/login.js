import { currentSession, login } from '../core/api.js';
import { config } from '../core/config.js';
import { escapeHtml, query, setPage } from '../core/dom.js';
import { setBusy, setStatus } from '../core/ui.js';

export async function renderLogin() {
  const existing = await currentSession(false);
  if (existing) {
    setPage(`<div class="auth-layout"><section class="card auth-card"><div class="card-header"><div><h1>Welcome back</h1><p>Your session is already active.</p></div></div><div class="card-body"><p>Signed in as <strong>${escapeHtml(existing.account.email)}</strong>.</p><a class="button button-primary" href="${escapeHtml(config.routes.dashboard)}">Open dashboard</a></div></section></div>`, 'Login');
    return;
  }

  setPage(`<div class="auth-layout"><section class="card auth-card"><div class="card-header"><div><h1>Sign in</h1><p>Manage uploads, processing, playback, and analytics.</p></div></div><div class="card-body"><form id="login-form" class="form-grid">
    <label class="field"><span>Email</span><input name="email" type="email" autocomplete="email" autofocus required></label>
    <label class="field"><span>Password</span><input name="password" type="password" autocomplete="current-password" required></label>
    <div class="form-actions"><button class="button button-primary" type="submit">Sign in</button><a class="button button-ghost" href="${escapeHtml(config.routes.register)}">Create account</a></div>
    <p id="form-status" class="form-status" role="status"></p>
  </form></div></section></div>`, 'Login');

  const form = query('#login-form');
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const data = new FormData(form);
    const status = query('#form-status');
    const button = form.querySelector('button[type="submit"]');
    try {
      setBusy(button, true, 'Signing in…');
      setStatus(status, 'Verifying your account…');
      await login(data.get('email'), data.get('password'));
      window.location.assign(config.routes.dashboard);
    } catch (error) {
      setStatus(status, error.message, 'error');
      setBusy(button, false);
    }
  });
}
