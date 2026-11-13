import { listen } from '@tauri-apps/api/event';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { save } from '@tauri-apps/plugin-dialog';
import { api } from './api';

/** Everything the native menu can trigger in the UI. Mirrored by `FORWARDED` in `src-tauri/src/shell.rs`. */
export type Shortcut =
  | 'new'
  | 'evaluate'
  | 'calc'
  | 'solve'
  | 'switch-form'
  | 'eng'
  | 'eng-back'
  | 'new-session'
  | 'toggle-sessions'
  | 'toggle-library'
  | 'toggle-writeup'
  | 'quick-search'
  | 'toggle-steps'
  | 'copy-result'
  | 'copy-all'
  | 'toggle-variables'
  | 'toggle-history'
  | 'toggle-latex'
  | 'toggle-settings';

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

/**
 * Saves `bytes` as a file the user picks, `name` offered; false when the dialog was cancelled.
 * In a plain browser the file is downloaded instead.
 */
export async function saveBytes(bytes: Uint8Array, name: string, filter: { name: string; extensions: string[] }): Promise<boolean> {
  if (!inShell) {
    const url = URL.createObjectURL(new Blob([bytes as BlobPart]));
    const link = document.createElement('a');
    link.href = url;
    link.download = name;
    link.click();
    URL.revokeObjectURL(url);
    return true;
  }
  const path = await save({ filters: [filter], defaultPath: name });
  if (!path) return false;
  await api.saveFile(path, bytes);
  return true;
}
