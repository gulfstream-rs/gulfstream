import { config, isAuthPage } from './config.js';

const csrfStorageKey = `${config.site.name}:csrf`;

/** Fired on `window` whenever the current browser session is (re)loaded. */
export const SESSION_EVENT = 'gulfstream:session';

export class ApiError extends Error {
  constructor(message, status = 0, details = null) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.details = details;
  }
}

function storeCsrf(token) {
  try {
    if (token) sessionStorage.setItem(csrfStorageKey, token);
  } catch { /* Storage unavailable; mutating requests will fail with a clear error. */ }
}

function readCsrf() {
  try {
    return sessionStorage.getItem(csrfStorageKey);
  } catch {
    return null;
  }
}

export function clearSession() {
  try {
    sessionStorage.removeItem(csrfStorageKey);
  } catch { /* Nothing to clear. */ }
}

function withCsrf(headers, method) {
  const next = new Headers(headers || {});
  if (!['GET', 'HEAD', 'OPTIONS'].includes(method)) {
    const csrf = readCsrf();
    if (csrf) next.set(config.csrf_header_name, csrf);
  }
  return next;
}

function errorMessage(body, status, statusText) {
  return body?.error?.message || `${status} ${statusText}`.trim() || 'The request failed.';
}

function redirectToLogin() {
  clearSession();
  window.location.assign(config.routes.login);
}

export async function request(url, options = {}) {
  const method = (options.method || 'GET').toUpperCase();
  let response;
  try {
    response = await fetch(url, {
      ...options,
      method,
      headers: withCsrf(options.headers, method),
      credentials: 'same-origin',
    });
  } catch {
    throw new ApiError('The server could not be reached. Check your connection and try again.');
  }
  if (response.status === 401 && !isAuthPage) {
    redirectToLogin();
    throw new ApiError('Authentication is required.', 401);
  }
  if (!response.ok) {
    let body = null;
    try { body = await response.json(); } catch { /* Non-JSON error response. */ }
    throw new ApiError(errorMessage(body, response.status, response.statusText), response.status, body?.error ?? null);
  }
  if (response.status === 204) return null;
  const contentType = response.headers.get('content-type') || '';
  return contentType.includes('application/json') ? response.json() : response.text();
}

export function jsonRequest(method, body) {
  return {
    method,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  };
}

export async function currentSession(required = true) {
  try {
    const session = await request(config.api.session);
    storeCsrf(session.csrf_token);
    window.dispatchEvent(new CustomEvent(SESSION_EVENT, { detail: session }));
    return session;
  } catch (error) {
    if (required) throw error;
    return null;
  }
}

export async function login(email, password) {
  const session = await request(config.api.login, jsonRequest('POST', { email, password }));
  storeCsrf(session.csrf_token);
  return session;
}

export async function logout() {
  try {
    await request(config.api.logout, { method: 'POST' });
  } finally {
    clearSession();
  }
}

/**
 * Multipart upload with progress reporting. XHR is used because `fetch` cannot
 * report upload progress. Pass an `AbortSignal` to support cancellation.
 */
export function upload(url, formData, { onProgress, signal } = {}) {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) {
      reject(new ApiError('The upload was cancelled.'));
      return;
    }
    const xhr = new XMLHttpRequest();
    xhr.open('POST', url);
    xhr.withCredentials = true;
    const csrf = readCsrf();
    if (csrf) xhr.setRequestHeader(config.csrf_header_name, csrf);
    xhr.upload.addEventListener('progress', (event) => {
      if (event.lengthComputable) onProgress?.(event.loaded, event.total);
    });
    xhr.addEventListener('load', () => {
      if (xhr.status === 401 && !isAuthPage) {
        redirectToLogin();
        reject(new ApiError('Authentication is required.', 401));
        return;
      }
      let body = null;
      try { body = xhr.responseText ? JSON.parse(xhr.responseText) : null; } catch { body = null; }
      if (xhr.status < 200 || xhr.status >= 300) {
        reject(new ApiError(errorMessage(body, xhr.status, xhr.statusText), xhr.status, body?.error ?? null));
        return;
      }
      resolve(body);
    });
    xhr.addEventListener('error', () => reject(new ApiError('The upload connection failed.')));
    xhr.addEventListener('abort', () => reject(new ApiError('The upload was cancelled.')));
    signal?.addEventListener('abort', () => xhr.abort(), { once: true });
    xhr.send(formData);
  });
}
