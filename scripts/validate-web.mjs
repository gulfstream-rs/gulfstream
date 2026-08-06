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
]) {
  if (!shell.includes(marker)) fail(`web/shell.html is missing ${marker}`);
}

const application = await readFile(join(assetsRoot, 'app.js'), 'utf8');
for (const page of ['register', 'login', 'dashboard', 'upload', 'media', 'media_detail', 'jobs', 'analytics', 'account']) {
  if (!application.includes(`['${page}',`)) fail(`app.js does not register page ${page}`);
}

if (!process.exitCode) {
  console.log(`validated ${scripts.length} JavaScript modules, ${styles.length} stylesheets, and the HTML shell`);
}
