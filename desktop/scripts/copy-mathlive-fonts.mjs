// MathLive loads its fonts at runtime from a directory next to the page. Bundling them
// through Vite is not possible (they are fetched by URL), so they are copied into public/
// once after every install.
import { cp, access } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const source = resolve(here, '../node_modules/mathlive/fonts');
const target = resolve(here, '../public/fonts');

try {
  await access(source);
} catch {
  console.warn('[quantus] mathlive fonts not found — run npm install first.');
  process.exit(0);
}

await cp(source, target, { recursive: true });
console.log('[quantus] mathlive fonts copied to public/fonts');
