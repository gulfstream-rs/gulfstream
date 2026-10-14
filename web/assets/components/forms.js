import { config } from '../core/config.js';
import { labelize } from '../core/dom.js';
import { html } from '../core/html.js';
import { icon } from './icon.js';

export function options(values, selected, allLabel = null) {
  return html`${allLabel != null && html`<option value="">${allLabel}</option>`}${values.map((value) => html`<option value="${value}"${value === selected && html` selected`}>${labelize(value)}</option>`)}`;
}

export function pageSizeOptions(selected) {
  const values = new Set(config.presentation.page_size_options.map(Number));
  values.add(Number(selected));
  return [...values]
    .filter((value) => Number.isFinite(value) && value > 0)
    .sort((left, right) => left - right)
    .map((value) => html`<option value="${value}"${Number(selected) === value && html` selected`}>${value} per page</option>`);
}

/** Labelled form control wrapper. `control` must be a trusted `html` fragment. */
export function field(label, control, { hint, className = '' } = {}) {
  return html`<label class="field ${className}"><span class="field-label">${label}</span>${control}${hint && html`<span class="field-hint">${hint}</span>`}</label>`;
}

export function searchField(name, value, placeholder) {
  return html`<label class="field field-grow search-field"><span class="field-label">Search</span>${icon('search', 'icon-sm')}<input name="${name}" type="search" value="${value}" placeholder="${placeholder}" autocomplete="off"></label>`;
}

/**
 * GET filter toolbar. The primary control (search) stays visible; secondary
 * filters collapse behind a "Filters" disclosure on phones.
 */
export function filterToolbar({ primary, filters, resetHref, activeCount = 0 }) {
  return html`<form method="get" class="toolbar" role="search">
    ${primary}
    <details class="filter-disclosure"${activeCount > 0 && html` open`}>
      <summary>${icon('filter', 'icon-sm')}Filters${activeCount > 0 && html` (${activeCount})`}</summary>
      <div class="filter-fields">
        ${filters}
        <div class="toolbar-actions">
          <button class="button button-primary" type="submit">Apply</button>
          <a class="button button-ghost" href="${resetHref}">Reset</a>
        </div>
      </div>
    </details>
  </form>`;
}

/** Submits GET filter forms automatically when a select changes. */
export function autoSubmitSelects(form) {
  for (const select of form?.querySelectorAll('select') ?? []) {
    select.addEventListener('change', () => form.requestSubmit());
  }
}
