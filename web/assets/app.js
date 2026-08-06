import { initializeShell, page } from './core/config.js';
import { showFatal } from './core/ui.js';
import { renderAccount } from './pages/account.js';
import { renderAnalytics } from './pages/analytics.js';
import { renderDashboard } from './pages/dashboard.js';
import { renderJobs } from './pages/jobs.js';
import { renderLogin } from './pages/login.js';
import { renderMediaDetail } from './pages/media-detail.js';
import { renderMediaList } from './pages/media-list.js';
import { renderRegister } from './pages/register.js';
import { renderUpload } from './pages/upload.js';

const pages = new Map([
  ['register', renderRegister],
  ['login', renderLogin],
  ['dashboard', renderDashboard],
  ['upload', renderUpload],
  ['media', renderMediaList],
  ['media_detail', renderMediaDetail],
  ['jobs', renderJobs],
  ['analytics', renderAnalytics],
  ['account', renderAccount],
]);

initializeShell();

try {
  const render = pages.get(page);
  if (!render) throw new Error(`Unknown page: ${page}`);
  await render();
} catch (error) {
  showFatal(error);
}
