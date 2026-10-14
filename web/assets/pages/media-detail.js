import { currentSession, jsonRequest, request } from '../core/api.js';
import { config, embedPage, endpoint, mediaIdFromLocation, watchPage } from '../core/config.js';
import { copyText, labelize, query, replaceContent, setPage } from '../core/dom.js';
import { html } from '../core/html.js';
import { formatBitrate, formatBytes, formatDate, formatDuration } from '../core/format.js';
import { badge, cell, emptyState, table } from '../components/data.js';
import { confirmAction, notice, setBusy, setStatus, toast } from '../components/feedback.js';
import { field, options } from '../components/forms.js';
import { icon } from '../components/icon.js';
import { pageHeader, sectionHeading } from '../components/layout.js';
import { autoRefresh } from '../components/refresh.js';

const IN_PROGRESS = ['importing', 'queued', 'processing', 'deleting'];

/** Requests a short-lived signed playback token and returns a same-origin watch URL. */
async function signedWatchUrl(mediaId) {
  const token = await request(endpoint(config.api.media_playback_token_template, mediaId), { method: 'POST' });
  return { url: watchPage(mediaId, token.token), expiresIn: Number(token.expires_in_seconds) };
}

function headerMeta(media) {
  return html`${badge(media.status)}${badge(media.visibility)}${media.duration_ms != null && html`<span class="badge">${formatDuration(media.duration_ms)}</span>`}`;
}

function headerActions(media) {
  const ready = media.status === 'ready';
  return html`
    ${media.status === 'failed' && html`<button class="button" type="button" data-action="retry">${icon('retry', 'icon-sm')}Retry processing</button>`}
    <button class="button button-primary" type="button" data-action="open-player"${!ready && html` disabled`}>${icon('external', 'icon-sm')}Open player</button>`;
}

function statusNotice(media) {
  if (media.error_message) return notice(html`<span><strong>Processing failed.</strong> ${media.error_message}</span>`, 'error');
  if (IN_PROGRESS.includes(media.status)) {
    return notice(html`<span class="spinner" aria-hidden="true"></span><span>This video is <strong>${labelize(media.status).toLowerCase()}</strong>. This page updates automatically when it changes.</span>`);
  }
  return null;
}

function previewPanel(media) {
  const ready = media.status === 'ready';
  const label = ready ? 'Play preview' : media.status === 'failed' ? 'Playback unavailable' : 'Available when processing finishes';
  return html`<div class="player-preview" data-preview>
    <button class="player-preview-start" type="button" data-action="preview"${!ready && html` disabled`}>
      <span class="play-circle">${icon('play')}</span>
      <span>${label}</span>
    </button>
  </div>`;
}

/** Embed builder state; survives re-rendering of the share region. */
const embedState = { tab: 'link', layout: 'responsive', autoplay: false, loop: false, start: '' };

function aspectRatio(media) {
  return media.width > 0 && media.height > 0 ? media.width / media.height : 16 / 9;
}

function parseStart(value) {
  const text = String(value || '').trim();
  if (!text) return 0;
  if (/^\d+$/.test(text)) return Number(text);
  const match = /^(?:(\d+):)?(\d{1,2}):(\d{2})$/.exec(text);
  if (!match) return null;
  return Number(match[1] || 0) * 3600 + Number(match[2]) * 60 + Number(match[3]);
}

/** Builds the iframe snippet for third-party sites. Values are attribute-escaped. */
function embedCode(media, options) {
  const url = new URL(embedPage(media.id), window.location.origin);
  if (options.autoplay) url.searchParams.set('autoplay', '1');
  if (options.loop) url.searchParams.set('loop', '1');
  const start = parseStart(options.start);
  if (start) url.searchParams.set('start', String(start));
  const ratio = aspectRatio(media);
  const allow = 'autoplay; fullscreen; picture-in-picture; encrypted-media; clipboard-write';
  const title = String(html`${media.title}`);
  const src = String(html`${url.href}`);
  if (options.layout === 'fixed') {
    const width = config.embed.default_width;
    const height = Math.round(width / ratio);
    return `<iframe src="${src}" width="${width}" height="${height}" title="${title}" frameborder="0" allow="${allow}" allowfullscreen loading="lazy" referrerpolicy="strict-origin-when-cross-origin"></iframe>`;
  }
  const aspect = media.width > 0 && media.height > 0 ? `${media.width} / ${media.height}` : '16 / 9';
  return `<iframe src="${src}" title="${title}" style="display:block;width:100%;aspect-ratio:${aspect};border:0;border-radius:12px" allow="${allow}" allowfullscreen loading="lazy" referrerpolicy="strict-origin-when-cross-origin"></iframe>`;
}

function embedPanel(media) {
  if (!config.embed.enabled) return html`<p class="field-hint">Embedding is disabled by server configuration.</p>`;
  const startValid = parseStart(embedState.start) !== null;
  const code = embedCode(media, embedState);
  return html`<form class="embed-form" data-embed-form>
    <div class="segmented" role="radiogroup" aria-label="Embed size">
      <button type="button" role="radio" data-embed-layout="responsive" aria-selected="${embedState.layout === 'responsive'}" aria-checked="${embedState.layout === 'responsive'}">Responsive</button>
      <button type="button" role="radio" data-embed-layout="fixed" aria-selected="${embedState.layout === 'fixed'}" aria-checked="${embedState.layout === 'fixed'}">Fixed ${config.embed.default_width} px</button>
    </div>
    <div class="embed-options">
      <label class="check-row"><input type="checkbox" name="autoplay"${embedState.autoplay && html` checked`}><span>Autoplay <span class="muted">(starts muted)</span></span></label>
      <label class="check-row"><input type="checkbox" name="loop"${embedState.loop && html` checked`}><span>Loop</span></label>
      <label class="embed-start"><span>Start at</span><input name="start" value="${embedState.start}" placeholder="0:00" inputmode="numeric" autocomplete="off" aria-invalid="${!startValid}"></label>
    </div>
    <label class="sr-only" for="embed-code">Embed code</label>
    <textarea id="embed-code" class="embed-code" readonly rows="4" spellcheck="false">${code}</textarea>
    <div class="embed-actions">
      <button class="button button-primary" type="button" data-copy="${code}" data-copy-label="Embed code copied to clipboard.">${icon('copy', 'icon-sm')}Copy embed code</button>
      <a class="button button-ghost" href="${embedPage(media.id)}" target="_blank" rel="noopener">${icon('external', 'icon-sm')}Preview</a>
    </div>
    <span class="field-hint">Paste into any website or CMS. WordPress and other oEmbed-aware editors also accept the plain watch link.</span>
  </form>`;
}

function shareTabs() {
  return html`<div class="segmented share-tabs" role="tablist" aria-label="Share options">
    <button type="button" role="tab" data-share-tab="link" aria-selected="${embedState.tab === 'link'}">${icon('link', 'icon-sm')}Link</button>
    <button type="button" role="tab" data-share-tab="embed" aria-selected="${embedState.tab === 'embed'}">${icon('code', 'icon-sm')}Embed</button>
  </div>`;
}

function shareCard(media) {
  const ready = media.status === 'ready';
  if (media.visibility === 'private') {
    return html`<section class="card"><div class="card-body form-grid">
      <div><h2>Share</h2><p class="field-hint" style="margin-top: var(--space-1)">Private: only you can watch. Generate a temporary signed link to share access briefly. Set visibility to public or unlisted to embed it on other sites.</p></div>
      <button class="button" type="button" data-action="temporary-link"${!ready && html` disabled`}>${icon('link', 'icon-sm')}Create temporary link</button>
      <div data-temporary-link hidden></div>
    </div></section>`;
  }
  const url = new URL(watchPage(media.id), window.location.origin).href;
  const linkPanel = html`<div class="share-field">
      <label class="sr-only" for="share-url">Watch link</label>
      <input id="share-url" value="${url}" readonly>
      <button class="button" type="button" data-copy="${url}" aria-label="Copy watch link">${icon('copy', 'icon-sm')}</button>
    </div>`;
  return html`<section class="card"><div class="card-body form-grid">
    <div class="share-heading"><div><h2>Share</h2><p class="field-hint" style="margin-top: var(--space-1)">${media.visibility === 'public' ? 'Public: anyone can watch.' : 'Unlisted: anyone with the link can watch.'}</p></div>${shareTabs()}</div>
    <div data-share-panel>${embedState.tab === 'embed' ? embedPanel(media) : linkPanel}</div>
    ${!ready && html`<span class="field-hint">The link starts working once processing finishes.</span>`}
  </div></section>`;
}

function detailsCard(media) {
  const rows = [
    ['Dimensions', media.width && media.height ? `${media.width} × ${media.height}` : '—'],
    ['Duration', media.duration_ms == null ? '—' : formatDuration(media.duration_ms)],
    ['Source size', formatBytes(media.source_size_bytes)],
    ['Stored size', formatBytes(media.storage_bytes)],
    ['Video codec', media.video_codec || '—'],
    ['Audio codec', media.audio_codec || '—'],
    ['Source file', media.source_filename],
    ['MIME type', media.source_mime_type || '—'],
    ['Created', formatDate(media.created_at)],
    ['Published', formatDate(media.published_at)],
  ];
  return html`<section class="card"><div class="card-body">
    <h2 style="margin-bottom: var(--space-3)">Details</h2>
    <dl class="detail-list">${rows.map(([term, value]) => html`<dt>${term}</dt><dd>${value}</dd>`)}
      <dt>SHA-256</dt><dd><code title="${media.sha256 || ''}">${media.sha256 ? `${media.sha256.slice(0, 16)}…` : '—'}</code></dd>
    </dl>
  </div></section>`;
}

function metadataForm(media) {
  const { limits } = config;
  return html`<section class="card"><form id="edit-form" class="card-body form-grid">
    <h2>Metadata</h2>
    ${field('Title', html`<input name="title" value="${media.title}" maxlength="${limits.maximum_text_field_bytes}" required>`)}
    ${field('Description', html`<textarea name="description" maxlength="${limits.maximum_text_field_bytes}" placeholder="Optional">${media.description}</textarea>`)}
    ${field('Visibility', html`<select name="visibility">${options(config.options.media_visibilities, media.visibility)}</select>`, { hint: 'Private media requires a signed link; unlisted media is reachable by link only.' })}
    <div class="form-actions">
      <button class="button button-primary" type="submit">Save changes</button>
      <p id="media-status" class="form-status" role="status" aria-live="polite"></p>
    </div>
  </form></section>`;
}

function renditionsSection(media) {
  const rows = media.variants.map((variant) => html`<tr>
    ${cell('Rendition', html`<span class="cell-primary">${variant.name}</span>`, { primary: true })}
    ${cell('Resolution', `${variant.width} × ${variant.height}`, { className: 'cell-nowrap' })}
    ${cell('Bandwidth', formatBitrate(variant.bandwidth_bps), { className: 'cell-nowrap' })}
    ${cell('Codecs', html`<span class="mono">${variant.codecs}</span>`)}
  </tr>`);
  return html`<section class="page-section">
    ${sectionHeading('Adaptive renditions', 'HLS variants generated by the configured processing profiles.')}
    ${table(['Rendition', 'Resolution', 'Bandwidth', 'Codecs'], rows, { empty: emptyState('No renditions yet', 'Renditions appear after conversion succeeds.', { iconName: 'media' }) })}
  </section>`;
}

function historySection(jobs) {
  const rows = jobs.map((job) => html`<tr>
    ${cell('Job', html`<span class="cell-primary">${labelize(job.kind)}</span>`, { primary: true })}
    ${cell('Status', badge(job.status))}
    ${cell('Attempts', `${job.attempts} / ${job.maximum_attempts}`, { className: 'cell-nowrap' })}
    ${cell('Updated', formatDate(job.updated_at), { className: 'cell-nowrap' })}
    ${job.error_message ? cell('Error', html`<span class="cell-error">${job.error_message}</span>`) : html`<td data-label="Error" data-optional-empty>—</td>`}
  </tr>`);
  return html`<section class="page-section">
    ${sectionHeading('Processing history', 'Durable job attempts and errors for this video.')}
    ${table(['Job', 'Status', 'Attempts', 'Updated', 'Error'], rows, { empty: emptyState('No jobs recorded', 'Processing jobs for this video will appear here.', { iconName: 'jobs' }) })}
  </section>`;
}

function dangerZone() {
  return html`<section class="card card-danger"><div class="card-body danger-zone">
    <div><h2>Delete video</h2><p>Removes the source and all generated streaming assets. This cannot be undone.</p></div>
    <button class="button button-danger-ghost" type="button" data-action="delete">${icon('trash', 'icon-sm')}Delete</button>
  </div></section>`;
}

function headerFor(media) {
  return pageHeader(media.title, media.description || null, {
    back: { href: config.routes.media, label: 'Media library' },
    meta: headerMeta(media),
    actions: headerActions(media),
  });
}

function layout(media, jobs) {
  return html`<div data-region="header">${headerFor(media)}</div>
    <div data-region="notice">${statusNotice(media)}</div>
    <div class="split" style="margin-top: var(--space-5)">
      <div class="stack">
        ${previewPanel(media)}
        ${metadataForm(media)}
      </div>
      <div class="stack">
        <div data-region="share">${shareCard(media)}</div>
        ${detailsCard(media)}
        ${dangerZone()}
      </div>
    </div>
    ${renditionsSection(media)}
    ${historySection(jobs)}`;
}

function startPreview(container, url) {
  const frame = document.createElement('iframe');
  frame.src = url;
  frame.title = 'Video player';
  frame.allow = 'autoplay; fullscreen; picture-in-picture; encrypted-media; clipboard-write';
  frame.allowFullscreen = true;
  frame.referrerPolicy = 'no-referrer';
  container.replaceChildren(frame);
  frame.focus();
}

export async function renderMediaDetail() {
  await currentSession();
  const mediaId = mediaIdFromLocation();
  if (!mediaId) throw new Error('Media ID is missing.');
  const itemUrl = endpoint(config.api.media_item_template, mediaId);
  let [media, jobs] = await Promise.all([
    request(itemUrl),
    request(endpoint(config.api.media_jobs_template, mediaId)),
  ]);

  setPage(layout(media, jobs), media.title);
  const app = document.querySelector('#app');

  const refreshSummary = (next) => {
    media = next;
    replaceContent(app.querySelector('[data-region="header"]'), headerFor(media));
    replaceContent(app.querySelector('[data-region="share"]'), shareCard(media));
    document.title = `${media.title} · ${config.site.name}`;
    const titleNode = document.querySelector('[data-page-title]');
    if (titleNode) titleNode.textContent = media.title;
  };

  // All actions use delegation so re-rendered regions keep working.
  app.addEventListener('click', async (event) => {
    const shareTab = event.target instanceof Element ? event.target.closest('[data-share-tab]') : null;
    if (shareTab) {
      embedState.tab = shareTab.dataset.shareTab;
      replaceContent(app.querySelector('[data-region="share"]'), shareCard(media));
      app.querySelector(`[data-share-tab="${embedState.tab}"]`)?.focus();
      return;
    }
    const layoutButton = event.target instanceof Element ? event.target.closest('[data-embed-layout]') : null;
    if (layoutButton) {
      embedState.layout = layoutButton.dataset.embedLayout;
      replaceContent(app.querySelector('[data-region="share"]'), shareCard(media));
      app.querySelector(`[data-embed-layout="${embedState.layout}"]`)?.focus();
      return;
    }
    const target = event.target instanceof Element ? event.target.closest('[data-action], [data-copy]') : null;
    if (!target || target.disabled) return;

    if (target.dataset.copy) {
      try {
        await copyText(target.dataset.copy);
        toast(target.dataset.copyLabel || 'Link copied to clipboard.');
      } catch (error) { toast(error.message, 'error'); }
      return;
    }

    switch (target.dataset.action) {
      case 'preview': {
        const container = app.querySelector('[data-preview]');
        setBusy(target, true, 'Preparing secure playback…');
        try {
          const { url } = await signedWatchUrl(mediaId);
          startPreview(container, url);
        } catch (error) {
          setBusy(target, false);
          toast(error.message, 'error');
        }
        break;
      }
      case 'open-player': {
        // Open synchronously to avoid popup blockers, then navigate once signed.
        const popup = window.open('about:blank', '_blank');
        try {
          const { url } = await signedWatchUrl(mediaId);
          if (popup) {
            popup.opener = null;
            popup.location.replace(url);
          } else {
            window.location.assign(url);
          }
        } catch (error) {
          popup?.close();
          toast(error.message, 'error');
        }
        break;
      }
      case 'temporary-link': {
        setBusy(target, true, 'Signing link…');
        try {
          const { url, expiresIn } = await signedWatchUrl(mediaId);
          const absolute = new URL(url, window.location.origin).href;
          const minutes = Math.max(1, Math.round(expiresIn / 60));
          const output = app.querySelector('[data-temporary-link]');
          output.hidden = false;
          replaceContent(output, html`<div class="share-field">
              <label class="sr-only" for="temporary-url">Temporary watch link</label>
              <input id="temporary-url" value="${absolute}" readonly>
              <button class="button" type="button" data-copy="${absolute}" aria-label="Copy temporary link">${icon('copy', 'icon-sm')}</button>
            </div>
            <span class="field-hint">Expires in about ${minutes} minute${minutes === 1 ? '' : 's'}. Anyone with this link can watch until then.</span>`);
        } catch (error) {
          toast(error.message, 'error');
        } finally {
          setBusy(target, false);
        }
        break;
      }
      case 'retry': {
        setBusy(target, true, 'Queueing…');
        try {
          await request(endpoint(config.api.media_retry_template, mediaId), { method: 'POST' });
          toast('Processing retry queued.');
          window.location.reload();
        } catch (error) {
          setBusy(target, false);
          toast(error.message, 'error');
        }
        break;
      }
      case 'delete': {
        const confirmed = await confirmAction('Delete this video?', `“${media.title}” and all generated streaming assets will be permanently removed.`, 'Delete video');
        if (!confirmed) return;
        setBusy(target, true, 'Deleting…');
        try {
          await request(itemUrl, { method: 'DELETE' });
          window.location.assign(config.routes.media);
        } catch (error) {
          setBusy(target, false);
          toast(error.message, 'error');
        }
        break;
      }
      default:
        break;
    }
  });

  // Embed options update the snippet in place (no re-render, so focus and
  // the caret stay in the "Start at" field while typing).
  app.addEventListener('input', (event) => {
    const form = event.target instanceof Element ? event.target.closest('[data-embed-form]') : null;
    if (!form) return;
    embedState.autoplay = form.elements.autoplay.checked;
    embedState.loop = form.elements.loop.checked;
    embedState.start = form.elements.start.value;
    const valid = parseStart(embedState.start) !== null;
    form.elements.start.setAttribute('aria-invalid', String(!valid));
    const code = embedCode(media, embedState);
    form.querySelector('#embed-code').value = code;
    form.querySelector('[data-copy]').dataset.copy = code;
  });

  const editForm = query('#edit-form');
  editForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    if (!editForm.reportValidity()) return;
    const button = editForm.querySelector('button[type="submit"]');
    const status = query('#media-status');
    try {
      setBusy(button, true, 'Saving…');
      setStatus(status, '');
      const updated = await request(itemUrl, jsonRequest('PATCH', Object.fromEntries(new FormData(editForm))));
      refreshSummary(updated && typeof updated === 'object' && updated.id ? updated : await request(itemUrl));
      toast('Changes saved.');
      setStatus(status, 'Saved.', 'success');
    } catch (error) {
      setStatus(status, error.message, 'error');
    } finally {
      setBusy(button, false);
    }
  });

  if (IN_PROGRESS.includes(media.status)) {
    const initialStatus = media.status;
    autoRefresh(async () => {
      const next = await request(itemUrl);
      if (next.status !== initialStatus) window.location.reload();
    }, config.presentation.jobs_refresh_seconds);
  }
}
