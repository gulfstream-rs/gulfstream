import { config } from './config.js';

const dateOptions = {
  dateStyle: 'medium',
  timeStyle: 'short',
  ...(config.presentation.time_zone ? { timeZone: config.presentation.time_zone } : {}),
};
const dateFormatter = new Intl.DateTimeFormat(config.presentation.date_locale, dateOptions);
const numberFormatter = new Intl.NumberFormat(config.presentation.date_locale);
const compactFormatter = new Intl.NumberFormat(config.presentation.date_locale, {
  notation: 'compact',
  maximumFractionDigits: 1,
});

export function formatNumber(value) {
  const number = Number(value || 0);
  return Number.isFinite(number) ? numberFormatter.format(number) : '—';
}

export function formatCompact(value) {
  const number = Number(value || 0);
  return Number.isFinite(number) ? compactFormatter.format(number) : '—';
}

export function formatBytes(value) {
  const bytes = Number(value || 0);
  if (!Number.isFinite(bytes) || bytes < 0) return '—';
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];
  let amount = bytes;
  let unit = 0;
  while (amount >= 1024 && unit < units.length - 1) {
    amount /= 1024;
    unit += 1;
  }
  return `${amount.toFixed(unit === 0 ? 0 : amount >= 10 ? 1 : 2)} ${units[unit]}`;
}

export function formatBitrate(bitsPerSecond) {
  const value = Number(bitsPerSecond || 0);
  if (!Number.isFinite(value) || value < 0) return '—';
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(2)} Mbps`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(0)} Kbps`;
  return `${value.toFixed(0)} bps`;
}

export function formatDuration(milliseconds) {
  const numeric = Number(milliseconds);
  if (!Number.isFinite(numeric) || numeric < 0) return '—';
  const totalSeconds = Math.floor(numeric / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return [hours, minutes, seconds].map((part) => String(part).padStart(2, '0')).join(':');
}

export function formatDate(value) {
  if (!value) return '—';
  const date = new Date(value);
  return Number.isNaN(date.valueOf()) ? String(value) : dateFormatter.format(date);
}

export function percent(value, total) {
  const numerator = Number(value || 0);
  const denominator = Number(total || 0);
  if (!Number.isFinite(numerator) || !Number.isFinite(denominator) || denominator <= 0) return 0;
  return Math.max(0, Math.min(100, (numerator / denominator) * 100));
}
