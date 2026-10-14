import { currentSession, login } from '../core/api.js';
import { config } from '../core/config.js';
import { query, setPage } from '../core/dom.js';
import { html } from '../core/html.js';
import { setBusy, setStatus } from '../components/feedback.js';
import { field } from '../components/forms.js';
import { authCard, signedInCard } from '../components/auth.js';

export async function renderLogin() {
  const existing = await currentSession(false);
  if (existing) {
    setPage(signedInCard(existing), 'Sign in');
    return;
  }

  const registration = config.registration.mode !== 'disabled'
    ? html`New here? <a href="${config.routes.register}">Create an account</a>`
    : null;
  setPage(authCard('Sign in', `Welcome back to ${config.site.name}.`, html`<form id="login-form" class="form-grid" novalidate>
      ${field('Email', html`<input name="email" type="email" autocomplete="username" inputmode="email" autofocus required>`)}
      ${field('Password', html`<input name="password" type="password" autocomplete="current-password" required>`)}
      <p id="form-status" class="form-status" role="status" aria-live="polite"></p>
      <button class="button button-primary button-lg button-block" type="submit">Sign in</button>
    </form>`, registration), 'Sign in');

  const form = query('#login-form');
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const status = query('#form-status');
    if (!form.reportValidity()) return;
    const data = new FormData(form);
    const button = form.querySelector('button[type="submit"]');
    try {
      setBusy(button, true, 'Signing in…');
      setStatus(status, '');
      await login(String(data.get('email')).trim(), data.get('password'));
      window.location.assign(config.routes.dashboard);
    } catch (error) {
      setStatus(status, error.message, 'error');
      setBusy(button, false);
      form.elements.password.select();
    }
  });
}
