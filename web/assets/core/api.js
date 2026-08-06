import { config, page } from './config.js';

const csrfStorageKey = `${config.site.name}:csrf`;

export class ApiError extends Error {
  constructor(message, status = 0, details = null) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.details = details;
  }
}

function csrfHeaders(headers, method) {
  const next = new Headers(headers || {});
  if (!['GET', 'HEAD', 'OPTIONS'].includes(method)) {
    const csrf = sessionStorage.getItem(csrfStorageKey);
    if (csrf) next.set(config.csrf_header_name, csrf);
  }
  return next;
}

async function errorFromResponse(response) {
  let body = null;
  try { body = await response.json(); } catch { /* Non-JSON error response. */ }
  return new ApiError(body?.error?.message || `${response.status} ${response.statusText}`, response.status, body?.error ?? null);
}

function redirectForAuthentication(response) {
  if (response.status !== 401 || ['login', 'register'].includes(page)) return false;
  sessionStorage.removeItem(csrfStorageKey);
  window.location.assign(config.routes.login);
  return true;
}

export async function request(url, options = {}) {
  const method = (options.method || 'GET').toUpperCase();
  const response = await fetch(url, {
    ...options,
    method,
    headers: csrfHeaders(options.headers, method),
    credentials: 'same-origin',
  });
  if (redirectForAuthentication(response)) throw new ApiError('Authentication is required.', 401);
  if (!response.ok) throw await errorFromResponse(response);
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
    sessionStorage.setItem(csrfStorageKey, session.csrf_token);
    return session;
  } catch (error) {
    if (required) throw error;
    return null;
  }
}

export async function login(email, password) {
  const session = await request(config.api.login, jsonRequest('POST', { email, password }));
  sessionStorage.setItem(csrfStorageKey, session.csrf_token);
  return session;
}

export function clearSession() {
  sessionStorage.removeItem(csrfStorageKey);
}

export function upload(url, formData, onProgress) {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open('POST', url);
    xhr.withCredentials = true;
    const csrf = sessionStorage.getItem(csrfStorageKey);
    if (csrf) xhr.setRequestHeader(config.csrf_header_name, csrf);
    xhr.upload.addEventListener('progress', (event) => {
      if (event.lengthComputable) onProgress?.(event.loaded, event.total);
    });
    xhr.addEventListener('load', () => {
      if (xhr.status === 401 && !['login', 'register'].includes(page)) {
        clearSession();
        window.location.assign(config.routes.login);
        reject(new ApiError('Authentication is required.', 401));
        return;
      }
      let body = null;
      try { body = xhr.responseText ? JSON.parse(xhr.responseText) : null; } catch { body = xhr.responseText; }
      if (xhr.status < 200 || xhr.status >= 300) {
        reject(new ApiError(body?.error?.message || `${xhr.status} ${xhr.statusText}`, xhr.status, body?.error ?? null));
        return;
      }
      resolve(body);
    });
    xhr.addEventListener('error', () => reject(new ApiError('The upload connection failed.')));
    xhr.addEventListener('abort', () => reject(new ApiError('The upload was cancelled.')));
    xhr.send(formData);
  });
}
