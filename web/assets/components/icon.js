import { html } from '../core/html.js';

/** References a symbol from the SVG sprite embedded in web/shell.html. */
export function icon(name, className = '') {
  const classes = className ? `icon ${className}` : 'icon';
  return html`<svg class="${classes}" aria-hidden="true" focusable="false"><use href="#i-${name}"></use></svg>`;
}
