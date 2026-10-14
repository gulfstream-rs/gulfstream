import { html } from '../core/html.js';
import { icon } from './icon.js';

/**
 * @param {string} title
 * @param {string} description
 * @param {object} [options]
 * @param {unknown} [options.actions] trusted `html` fragment rendered on the right
 * @param {unknown} [options.meta] trusted `html` fragment rendered under the title
 * @param {{href: string, label: string}} [options.back] back link above the title
 */
export function pageHeader(title, description, { actions, meta, back } = {}) {
  return html`<header class="page-header">
    <div class="page-header-copy">
      ${back && html`<a class="page-back" href="${back.href}">${icon('chevron-left', 'icon-sm')}${back.label}</a>`}
      <h1>${title}</h1>
      ${description && html`<p>${description}</p>`}
      ${meta && html`<div class="page-header-meta">${meta}</div>`}
    </div>
    ${actions && html`<div class="actions">${actions}</div>`}
  </header>`;
}

export function sectionHeading(title, description = '', actions = null) {
  return html`<div class="section-heading">
    <div><h2>${title}</h2>${description && html`<p>${description}</p>`}</div>
    ${actions}
  </div>`;
}

export function card({ title, description, body, footer, className = '' }) {
  return html`<section class="card ${className}">
    ${title && html`<header class="card-header"><div><h2>${title}</h2>${description && html`<p>${description}</p>`}</div></header>`}
    <div class="card-body">${body}</div>
    ${footer && html`<footer class="card-footer">${footer}</footer>`}
  </section>`;
}
