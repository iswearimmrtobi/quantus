// Writes THIRD-PARTY-LICENSES.md in the repository root: the Rust crates through cargo-about
// (about.toml, about.hbs), then the npm packages that end up in the renderer, which are the
// non-dev entries of package-lock.json. The installers ship the file next to the app.
import { execFileSync } from 'node:child_process';
import { readdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const desktop = resolve(here, '..');
const root = resolve(desktop, '..');

const crates = execFileSync('cargo', ['about', 'generate', 'about.hbs'], { cwd: root, encoding: 'utf8' });

const lock = JSON.parse(await readFile(join(desktop, 'package-lock.json'), 'utf8'));
const packages = Object.entries(lock.packages)
  .filter(([path, entry]) => path !== '' && !entry.dev)
  .map(([path]) => path)
  .sort((a, b) => a.localeCompare(b));

let npm = '## npm packages\n\n';
for (const path of packages) {
  const dir = join(desktop, path);
  const manifest = JSON.parse(await readFile(join(dir, 'package.json'), 'utf8'));
  const repository = typeof manifest.repository === 'string' ? manifest.repository : manifest.repository?.url;
  npm += `### ${manifest.name} ${manifest.version}\n\n`;
  npm += `License: ${manifest.license ?? 'see the package'}${repository ? ` (${repository})` : ''}\n\n`;
  const files = (await readdir(dir)).filter((name) => /^licen[cs]e/i.test(name) && !name.endsWith('.spdx')).sort();
  if (files.length === 0) {
    npm += 'The package ships no license text; its license is the one named above, whose text is in the Rust crates section.\n\n';
  }
  for (const file of files) {
    const text = (await readFile(join(dir, file), 'utf8')).trim();
    npm += '```text\n' + text + '\n```\n\n';
  }
}

const text = crates.trimEnd() + '\n\n' + npm.trimEnd() + '\n';
await writeFile(join(root, 'THIRD-PARTY-LICENSES.md'), text.replace(/\r\n?/g, '\n'));
console.log(`[quantus] THIRD-PARTY-LICENSES.md written (${packages.length} npm packages)`);
