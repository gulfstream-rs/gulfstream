import { readdir, readFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { dirname, join, normalize, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const assetsRoot = join(root, 'web', 'assets');

async function filesBelow(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...await filesBelow(path));
    else files.push(path);
  }
  return files;
}

function fail(message) {
  console.error(`validation error: ${message}`);
  process.exitCode = 1;
}

const files = await filesBelow(assetsRoot);
const scripts = files.filter((path) => path.endsWith('.js'));
const styles = files.filter((path) => path.endsWith('.css'));

for (const script of scripts) {
  const result = spawnSync(process.execPath, ['--check', script], { encoding: 'utf8' });
  if (result.status !== 0) fail(`${script}\n${result.stderr}`);

  const source = await readFile(script, 'utf8');
  for (const match of source.matchAll(/from\s+['"](\.\.?\/[^'"]+)['"]/g)) {
    const imported = normalize(resolve(dirname(script), match[1]));
    const candidates = [imported, `${imported}.js`, join(imported, 'index.js')];
    const exists = candidates.some((candidate) => scripts.includes(candidate));
    if (!exists) fail(`${script} imports missing module ${match[1]}`);
  }
}

for (const style of styles) {
  const source = await readFile(style, 'utf8');
  let depth = 0;
  for (const character of source.replaceAll(/\/\*[\s\S]*?\*\//g, '')) {
    if (character === '{') depth += 1;
    if (character === '}') depth -= 1;
    if (depth < 0) break;
  }
  if (depth !== 0) fail(`${style} has unbalanced braces`);
}

const shell = await readFile(join(root, 'web', 'shell.html'), 'utf8');
for (const marker of [
  '{{PAGE_TITLE}}',
  '{{PAGE_ID_JSON}}',
  '{{SITE_NAME}}',
  '{{TAGLINE}}',
  '{{ASSET_BASE}}',
  '{{RUNTIME_CONFIG_JSON}}',
  'id="app"',
  'id="confirm-dialog"',
  'id="toast-region"',
  'data-theme-toggle',
]) {
  if (!shell.includes(marker)) fail(`web/shell.html is missing ${marker}`);
}

// Every icon referenced from JavaScript or the shell must exist in the sprite.
const sprite = new Set([...shell.matchAll(/<symbol id="i-([a-z-]+)"/g)].map((match) => match[1]));
const iconReferences = [...shell.matchAll(/href="#i-([a-z-]+)"/g)].map((match) => [join('web', 'shell.html'), match[1]]);
for (const script of scripts) {
  const source = await readFile(script, 'utf8');
  for (const match of source.matchAll(/(?:icon\(|iconName:\s*)'([a-z-]+)'/g)) iconReferences.push([script, match[1]]);
  for (const match of source.matchAll(/\['([a-z-]+)', config\.routes\./g)) iconReferences.push([script, match[1]]);
}
for (const [file, name] of iconReferences) {
  if (!sprite.has(name)) fail(`${file} references missing icon "${name}"`);
}

// Player template: required markers (see src/runtime.rs) and inline script syntax.
const player = await readFile(join(root, 'templates', 'player.html'), 'utf8');
const playerMarkers = {
  '{{TITLE}}': 'Title',
  '{{DESCRIPTION}}': 'Description',
  '{{HLS_SCRIPT}}': '',
  '{{STREAM_URL_JSON}}': '"/stream/id/master.m3u8"',
  '{{EVENT_URL_JSON}}': '"/api/v1/playback/session/events"',
  '{{SOURCE_MODE_JSON}}': '"hls"',
  '{{HEARTBEAT_MILLISECONDS}}': '10000',
  '{{FALLBACK_URL_JSON}}': 'null',
  '{{SITE_NAME}}': 'Site',
  '{{PLAYER_OPTIONS_JSON}}': '{"surface":"embed","autoplay":false,"muted":false,"loop":false,"start_seconds":0,"watch_url":null,"embed_url":null,"title":"Title","site_name":"Site"}',
  '{{HEAD_LINKS}}': '',
};
for (const marker of [...Object.keys(playerMarkers), '<video']) {
  if (!player.includes(marker)) fail(`templates/player.html is missing ${marker}`);
}
let renderedPlayer = player;
for (const [marker, value] of Object.entries(playerMarkers)) {
  renderedPlayer = renderedPlayer.replaceAll(marker, value);
}
const leftover = renderedPlayer.match(/\{\{[A-Z_]+\}\}/);
if (leftover) fail(`templates/player.html uses unknown marker ${leftover[0]}`);
const inlineScripts = [...renderedPlayer.matchAll(/<script>([\s\S]*?)<\/script>/g)].map((match) => match[1]);
if (inlineScripts.length === 0) fail('templates/player.html has no inline player script');
for (const [index, source] of inlineScripts.entries()) {
  const result = spawnSync(process.execPath, ['--check', '--input-type=commonjs', '-'], { input: source, encoding: 'utf8' });
  if (result.status !== 0) fail(`templates/player.html inline script #${index + 1}\n${result.stderr}`);
}

const application = await readFile(join(assetsRoot, 'app.js'), 'utf8');
for (const page of ['register', 'login', 'dashboard', 'upload', 'media', 'media_detail', 'jobs', 'analytics', 'account']) {
  if (!application.includes(`['${page}',`)) fail(`app.js does not register page ${page}`);
}

if (!process.exitCode) {
  console.log(`validated ${scripts.length} JavaScript modules, ${styles.length} stylesheets, ${sprite.size} icons, the HTML shell, and the player template`);
}
