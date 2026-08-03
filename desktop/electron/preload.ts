import { contextBridge, ipcRenderer } from 'electron';

/** The backend address the main process picked, passed in as a launch argument. */
const backendUrl =
  process.argv.find((argument) => argument.startsWith('--quantus-backend-url='))?.split('=')[1] ??
  'http://127.0.0.1:8437';

/**
 * The whole bridge into the renderer — deliberately three functions wide. The renderer has
 * no Node access; anything it needs from the system passes through here.
 */
contextBridge.exposeInMainWorld('quantus', {
  backendUrl,

  copy: (text: string): Promise<void> => ipcRenderer.invoke('quantus:copy', text),

  /** Menu accelerators (F10). Returns the unsubscribe function. */
  onShortcut: (handler: (shortcut: string) => void): (() => void) => {
    const listener = (_event: unknown, shortcut: string) => handler(shortcut);
    ipcRenderer.on('quantus:shortcut', listener);
    return () => ipcRenderer.off('quantus:shortcut', listener);
  },
});
