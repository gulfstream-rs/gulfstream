import { html } from '../core/html.js';
import { formatNumber } from '../core/format.js';
import { emptyState } from './data.js';

/** Dependency-free daily bar chart with keyboard-focusable bars. */
export function barChart(points, valueKey, label) {
  if (!points?.length) return emptyState('No chart data', 'Choose a range with recorded activity.', { iconName: 'analytics' });
  const maximum = Math.max(1, ...points.map((point) => Number(point[valueKey] || 0)));
  const edge = Math.max(1, Math.round(points.length / 5));
  const bars = points.map((point, index) => {
    const value = Number(point[valueKey] || 0);
    const height = value > 0 ? Math.max(2, (value / maximum) * 100) : 0;
    const tooltip = `${point.day}: ${formatNumber(value)} ${label}`;
    return html`<span class="chart-bar" tabindex="0" role="listitem" style="--height:${height.toFixed(2)}%"${value === 0 && html` data-empty`} data-label="${tooltip}" aria-label="${tooltip}"${index < edge ? html` data-edge="start"` : index >= points.length - edge ? html` data-edge="end"` : ''}></span>`;
  });
  const first = points[0]?.day ?? '';
  const last = points.at(-1)?.day ?? '';
  return html`<div class="chart">
    <div class="chart-plot" style="--points:${points.length}" role="list" aria-label="${label} by day, peak ${formatNumber(maximum)}">${bars}</div>
    <div class="chart-axis" aria-hidden="true"><span>${first}</span><span>Peak ${formatNumber(maximum)}</span><span>${last}</span></div>
  </div>`;
}
