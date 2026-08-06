import { escapeHtml } from '../core/dom.js';
import { formatNumber } from '../core/format.js';

export function barChart(points, valueKey, label) {
  if (!points?.length) return '<div class="empty-state"><strong>No chart data</strong><span>Choose a range with recorded activity.</span></div>';
  const maximum = Math.max(1, ...points.map((point) => Number(point[valueKey] || 0)));
  const bars = points.map((point) => {
    const value = Number(point[valueKey] || 0);
    const height = Math.max(value > 0 ? 3 : 0, (value / maximum) * 100);
    const tooltip = `${point.day}: ${formatNumber(value)} ${label}`;
    return `<span class="chart-bar" tabindex="0" style="--height:${height.toFixed(2)}%" data-label="${escapeHtml(tooltip)}" aria-label="${escapeHtml(tooltip)}"></span>`;
  }).join('');
  return `<div class="chart" style="--points:${points.length}" role="img" aria-label="${escapeHtml(label)} by day">${bars}</div>`;
}
