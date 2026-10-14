import { config } from '../core/config.js';
import { html } from '../core/html.js';

export function authCard(title, intro, body, footer = null) {
  return html`<section class="card auth-card">
    <div class="auth-intro"><h1>${title}</h1>${intro && html`<p>${intro}</p>`}</div>
    ${body}
    ${footer && html`<p class="auth-switch">${footer}</p>`}
  </section>`;
}

export function signedInCard(session) {
  return authCard(
    'You are signed in',
    null,
    html`<p class="muted" style="margin-bottom: var(--space-5)">Signed in as <strong>${session.account.email}</strong>.</p>
      <a class="button button-primary button-lg button-block" href="${config.routes.dashboard}">Open dashboard</a>`,
  );
}
