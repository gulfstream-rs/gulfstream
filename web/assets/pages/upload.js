import { currentSession, jsonRequest, request, upload } from '../core/api.js';
import { config, mediaPage } from '../core/config.js';
import { escapeHtml, optionMarkup, query, setPage } from '../core/dom.js';
import { formatBytes, percent } from '../core/format.js';
import { pageHeader, setBusy, setStatus } from '../core/ui.js';

function titleFromFile(file) {
  return file.name.replace(/\.[^.]+$/, '').replaceAll(/[_-]+/g, ' ').trim();
}

export async function renderUpload() {
  await currentSession();
  const remote = config.features.remote_imports ? `<section class="card"><div class="card-header"><div><h2>Import from URL</h2><p>Fetch a remote video through the protected import worker.</p></div></div><div class="card-body"><form id="import-form" class="form-grid">
    <label class="field"><span>Source URL</span><input name="url" type="url" placeholder="https://example.com/video.mp4" required></label>
    <div class="form-row"><label class="field"><span>Title</span><input name="title" maxlength="${config.limits.maximum_text_field_bytes}" required></label><label class="field"><span>Visibility</span><select name="visibility">${optionMarkup(config.options.media_visibilities, 'private')}</select></label></div>
    <label class="field"><span>Description</span><textarea name="description" maxlength="${config.limits.maximum_text_field_bytes}"></textarea></label>
    <div class="form-actions"><button class="button button-primary" type="submit">Start import</button><p id="import-status" class="form-status" role="status"></p></div>
  </form></div></section>` : '<div class="notice">Remote imports are disabled by server configuration.</div>';

  setPage(`${pageHeader('Upload video', `Maximum source size: ${formatBytes(config.limits.maximum_upload_bytes)}. Conversion starts after durable storage succeeds.`)}<div class="two-column"><section class="card"><div class="card-header"><div><h2>Direct upload</h2><p>Select one video file and configure its initial metadata.</p></div></div><div class="card-body"><form id="upload-form" class="form-grid">
    <label class="drop-zone" id="drop-zone"><input id="file-input" name="file" type="file" accept="video/*" required><strong>Drop a video here</strong><span class="field-hint">or click to choose a file</span></label>
    <div id="file-summary" class="file-summary" hidden></div>
    <div class="form-row"><label class="field"><span>Title</span><input name="title" maxlength="${config.limits.maximum_text_field_bytes}" required></label><label class="field"><span>Visibility</span><select name="visibility">${optionMarkup(config.options.media_visibilities, 'private')}</select></label></div>
    <label class="field"><span>Description</span><textarea name="description" maxlength="${config.limits.maximum_text_field_bytes}"></textarea></label>
    <div id="upload-progress" hidden><div class="progress" role="progressbar" aria-label="Upload progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0"><span></span></div><p class="field-hint" data-progress-label>Preparing upload…</p></div>
    <div class="form-actions"><button class="button button-primary" type="submit">Upload and process</button><p id="upload-status" class="form-status" role="status"></p></div>
  </form></div></section><div>${remote}</div></div>`, 'Upload');

  const uploadForm = query('#upload-form');
  const fileInput = query('#file-input');
  const dropZone = query('#drop-zone');
  const fileSummary = query('#file-summary');
  const titleInput = uploadForm.elements.title;
  const updateFile = () => {
    const file = fileInput.files?.[0];
    if (!file) {
      fileSummary.hidden = true;
      return;
    }
    fileSummary.hidden = false;
    fileSummary.innerHTML = `<span><strong>${escapeHtml(file.name)}</strong><br><small>${escapeHtml(file.type || 'Unknown type')}</small></span><strong>${formatBytes(file.size)}</strong>`;
    if (!titleInput.value.trim()) titleInput.value = titleFromFile(file);
  };
  fileInput.addEventListener('change', updateFile);
  for (const eventName of ['dragenter', 'dragover']) dropZone.addEventListener(eventName, () => { dropZone.dataset.dragging = 'true'; });
  for (const eventName of ['dragleave', 'drop']) dropZone.addEventListener(eventName, () => { delete dropZone.dataset.dragging; });

  uploadForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    const status = query('#upload-status');
    const button = uploadForm.querySelector('button[type="submit"]');
    const progress = query('#upload-progress');
    const progressBar = progress.querySelector('[role="progressbar"]');
    const progressFill = progress.querySelector('.progress > span');
    const progressLabel = progress.querySelector('[data-progress-label]');
    const data = new FormData(uploadForm);
    const file = data.get('file');
    if (!(file instanceof File) || file.size === 0) {
      setStatus(status, 'Choose a video file.', 'error');
      return;
    }
    if (file.size > config.limits.maximum_upload_bytes) {
      setStatus(status, `The file exceeds the ${formatBytes(config.limits.maximum_upload_bytes)} limit.`, 'error');
      return;
    }
    try {
      setBusy(button, true, 'Uploading…');
      progress.hidden = false;
      const media = await upload(config.api.media, data, (loaded, total) => {
        const value = percent(loaded, total);
        progressBar.setAttribute('aria-valuenow', value.toFixed(1));
        progressFill.style.setProperty('--progress', `${value.toFixed(2)}%`);
        progressLabel.textContent = `${value.toFixed(1)}% · ${formatBytes(loaded)} of ${formatBytes(total)}`;
      });
      window.location.assign(mediaPage(media.id));
    } catch (error) {
      setStatus(status, error.message, 'error');
      setBusy(button, false);
    }
  });

  const importForm = document.querySelector('#import-form');
  importForm?.addEventListener('submit', async (event) => {
    event.preventDefault();
    const status = query('#import-status');
    const button = importForm.querySelector('button[type="submit"]');
    const data = Object.fromEntries(new FormData(importForm));
    try {
      setBusy(button, true, 'Queueing…');
      setStatus(status, 'Validating and queueing the import…');
      const media = await request(config.api.media_imports, jsonRequest('POST', data));
      window.location.assign(mediaPage(media.id));
    } catch (error) {
      setStatus(status, error.message, 'error');
      setBusy(button, false);
    }
  });
}
