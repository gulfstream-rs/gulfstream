import { currentSession, jsonRequest, request, upload } from '../core/api.js';
import { config, mediaPage } from '../core/config.js';
import { query, replaceContent, setPage } from '../core/dom.js';
import { html } from '../core/html.js';
import { formatBytes, percent } from '../core/format.js';
import { notice, setBusy, setStatus } from '../components/feedback.js';
import { field, options } from '../components/forms.js';
import { icon } from '../components/icon.js';
import { pageHeader } from '../components/layout.js';

function titleFromFile(file) {
  return file.name.replace(/\.[^.]+$/, '').replaceAll(/[_-]+/g, ' ').trim();
}

function metadataFields(prefix) {
  const { limits } = config;
  return html`
    ${field('Title', html`<input id="${prefix}-title" name="title" maxlength="${limits.maximum_text_field_bytes}" required>`)}
    ${field('Description', html`<textarea name="description" maxlength="${limits.maximum_text_field_bytes}" placeholder="Optional"></textarea>`)}
    ${field('Visibility', html`<select name="visibility">${options(config.options.media_visibilities, 'private')}</select>`, { hint: 'You can change this at any time.' })}`;
}

function fileForm() {
  return html`<form id="upload-form" class="form-grid" novalidate>
    <label class="drop-zone" id="drop-zone">
      <input id="file-input" name="file" type="file" accept="video/*" required>
      <span class="drop-zone-icon">${icon('upload', 'icon-lg')}</span>
      <strong>Drop a video here</strong>
      <span>or</span>
      <span class="button button-sm">Browse files</span>
      <span class="field-hint">Up to ${formatBytes(config.limits.maximum_upload_bytes)}</span>
    </label>
    <div id="file-summary" hidden></div>
    ${metadataFields('upload')}
    <div id="upload-progress" class="upload-progress" hidden>
      <div class="progress" role="progressbar" aria-label="Upload progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0"><span></span></div>
      <div class="upload-progress-figures"><span data-progress-label>Preparing upload…</span><span data-progress-percent>0%</span></div>
    </div>
    <p id="upload-status" class="form-status" role="status" aria-live="polite"></p>
    <div class="form-actions">
      <button class="button button-primary button-lg" type="submit">${icon('upload', 'icon-sm')}Upload and process</button>
      <button class="button button-ghost" type="button" id="cancel-upload" hidden>Cancel</button>
    </div>
  </form>`;
}

function importForm() {
  if (!config.features.remote_imports) return notice('Remote imports are disabled by server configuration.');
  return html`<form id="import-form" class="form-grid" novalidate>
    ${field('Source URL', html`<input name="url" type="url" inputmode="url" placeholder="https://example.com/video.mp4" required>`, { hint: 'The server downloads the file through a protected import worker.' })}
    ${metadataFields('import')}
    <p id="import-status" class="form-status" role="status" aria-live="polite"></p>
    <div class="form-actions"><button class="button button-primary button-lg" type="submit">${icon('globe', 'icon-sm')}Start import</button></div>
  </form>`;
}

function helpCard() {
  return html`<aside class="card"><div class="card-body form-grid">
    <h2>What happens next</h2>
    <ol class="steps">
      <li><span><strong>Stored durably</strong>The source is checksummed and saved.</span></li>
      <li><span><strong>Processed</strong>${config.features.transcoding ? 'FFmpeg converts it into adaptive HLS renditions.' : 'The original file is published for playback.'}</span></li>
      <li><span><strong>Ready to stream</strong>Play, share, and measure audience activity.</span></li>
    </ol>
    <a class="button button-ghost button-sm" href="${config.routes.jobs}">View processing queue${icon('chevron-right', 'icon-sm')}</a>
  </div></aside>`;
}

function wireTabs() {
  const tabs = [...document.querySelectorAll('[role="tab"]')];
  const select = (selected) => {
    for (const tab of tabs) {
      const active = tab === selected;
      tab.setAttribute('aria-selected', String(active));
      tab.tabIndex = active ? 0 : -1;
      document.getElementById(tab.getAttribute('aria-controls')).hidden = !active;
    }
  };
  for (const tab of tabs) {
    tab.addEventListener('click', () => select(tab));
    tab.addEventListener('keydown', (event) => {
      if (!['ArrowLeft', 'ArrowRight'].includes(event.key)) return;
      const index = tabs.indexOf(tab);
      const next = tabs[(index + (event.key === 'ArrowRight' ? 1 : tabs.length - 1)) % tabs.length];
      select(next);
      next.focus();
    });
  }
}

function wireFileUpload() {
  const form = query('#upload-form');
  const fileInput = query('#file-input');
  const dropZone = query('#drop-zone');
  const summary = query('#file-summary');
  const status = query('#upload-status');
  const titleInput = form.elements.title;
  const progress = query('#upload-progress');
  const progressBar = progress.querySelector('[role="progressbar"]');
  const progressFill = progress.querySelector('.progress > span');
  const progressLabel = progress.querySelector('[data-progress-label]');
  const progressPercent = progress.querySelector('[data-progress-percent]');
  const cancel = query('#cancel-upload');
  let autoTitle = '';
  let controller = null;

  const validateFile = (file) => {
    if (!file) return 'Choose a video file.';
    if (file.size === 0) return 'The selected file is empty.';
    if (file.size > config.limits.maximum_upload_bytes) return `The file exceeds the ${formatBytes(config.limits.maximum_upload_bytes)} limit.`;
    return '';
  };

  const showFile = () => {
    const file = fileInput.files?.[0];
    dropZone.hidden = Boolean(file);
    summary.hidden = !file;
    if (!file) return;
    const problem = validateFile(file);
    replaceContent(summary, html`<div class="file-card">
      <span class="recent-thumb">${icon('file')}</span>
      <span style="min-width:0"><span class="file-card-name">${file.name}</span><span class="file-card-meta">${formatBytes(file.size)} · ${file.type || 'Unknown type'}</span></span>
      <button class="icon-button" type="button" data-clear-file aria-label="Remove file">${icon('close')}</button>
    </div>`);
    setStatus(status, problem, problem ? 'error' : '');
    if (!titleInput.value.trim() || titleInput.value === autoTitle) {
      autoTitle = titleFromFile(file);
      titleInput.value = autoTitle;
    }
  };

  fileInput.addEventListener('change', showFile);
  summary.addEventListener('click', (event) => {
    if (!(event.target instanceof Element) || !event.target.closest('[data-clear-file]') || controller) return;
    fileInput.value = '';
    if (titleInput.value === autoTitle) titleInput.value = '';
    setStatus(status, '');
    showFile();
    fileInput.focus();
  });
  for (const name of ['dragenter', 'dragover']) dropZone.addEventListener(name, () => { dropZone.dataset.dragging = 'true'; });
  for (const name of ['dragleave', 'drop']) dropZone.addEventListener(name, () => { delete dropZone.dataset.dragging; });

  const setProgress = (loaded, total) => {
    const value = percent(loaded, total);
    progressBar.setAttribute('aria-valuenow', value.toFixed(1));
    progressFill.style.setProperty('--progress', `${value.toFixed(2)}%`);
    progressPercent.textContent = `${value.toFixed(0)}%`;
    progressLabel.textContent = value >= 100 ? 'Finalizing…' : `${formatBytes(loaded)} of ${formatBytes(total)}`;
  };

  cancel.addEventListener('click', () => controller?.abort());

  // Warn before leaving while an upload is in flight.
  window.addEventListener('beforeunload', (event) => {
    if (!controller) return;
    event.preventDefault();
    event.returnValue = '';
  });

  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const file = fileInput.files?.[0];
    const problem = validateFile(file);
    if (problem) {
      setStatus(status, problem, 'error');
      return;
    }
    if (!form.reportValidity()) return;
    const button = form.querySelector('button[type="submit"]');
    const data = new FormData(form);
    controller = new AbortController();
    try {
      setBusy(button, true, 'Uploading…');
      setStatus(status, '');
      cancel.hidden = false;
      progress.hidden = false;
      setProgress(0, file.size);
      const media = await upload(config.api.media, data, { onProgress: setProgress, signal: controller.signal });
      controller = null;
      setStatus(status, 'Upload complete. Opening video…', 'success');
      window.location.assign(mediaPage(media.id));
    } catch (error) {
      controller = null;
      setStatus(status, error.message, 'error');
      setBusy(button, false);
      cancel.hidden = true;
      progress.hidden = true;
    }
  });
}

function wireImport() {
  const form = document.querySelector('#import-form');
  form?.addEventListener('submit', async (event) => {
    event.preventDefault();
    if (!form.reportValidity()) return;
    const status = query('#import-status');
    const button = form.querySelector('button[type="submit"]');
    try {
      setBusy(button, true, 'Queueing…');
      setStatus(status, 'Validating and queueing the import…');
      const media = await request(config.api.media_imports, jsonRequest('POST', Object.fromEntries(new FormData(form))));
      window.location.assign(mediaPage(media.id));
    } catch (error) {
      setStatus(status, error.message, 'error');
      setBusy(button, false);
    }
  });
}

export async function renderUpload() {
  await currentSession();
  setPage(html`${pageHeader('Upload video', 'Add a video from your device or import it from a URL. Processing starts automatically.')}
    <div class="upload-layout">
      <section class="card"><div class="card-body">
        <div class="segmented upload-tabs" role="tablist" aria-label="Upload method">
          <button type="button" role="tab" id="tab-file" aria-controls="panel-file" aria-selected="true">${icon('file', 'icon-sm')}From device</button>
          <button type="button" role="tab" id="tab-url" aria-controls="panel-url" aria-selected="false" tabindex="-1">${icon('globe', 'icon-sm')}From URL</button>
        </div>
        <div id="panel-file" role="tabpanel" aria-labelledby="tab-file">${fileForm()}</div>
        <div id="panel-url" role="tabpanel" aria-labelledby="tab-url" hidden>${importForm()}</div>
      </div></section>
      ${helpCard()}
    </div>`, 'Upload');

  wireTabs();
  wireFileUpload();
  wireImport();
}
