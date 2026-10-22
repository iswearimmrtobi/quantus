import { listen } from '@tauri-apps/api/event';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

/** Everything the native menu can trigger in the UI. Mirrored by `FORWARDED` in `src-tauri/src/shell.rs`. */
export type Shortcut =
  | 'new'
  | 'evaluate'
  | 'toggle-steps'
  | 'copy-result'
  | 'copy-all'
  | 'toggle-variables'
  | 'toggle-history'
  | 'toggle-latex';

/** False in a plain browser (`npm run dev:renderer`), where there is no menu behind the UI. */
export const inShell = '__TAURI_INTERNALS__' in window;

/** Menu accelerators (F10). Returns the unsubscribe function. */
export function onShortcut(handler: (shortcut: Shortcut) => void): () => void {
  if (!inShell) return () => {};
  const subscription = listen<Shortcut>('quantus://shortcut', (event) => handler(event.payload));
  return () => void subscription.then((unlisten) => unlisten());
}

/** Writes through the native clipboard; the DOM clipboard needs a permission dance in a webview. */
export async function copyText(text: string): Promise<void> {
  if (inShell) {
    await writeText(text);
  } else {
    await navigator.clipboard.writeText(text);
  }
}
