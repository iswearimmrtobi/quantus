import { app, BrowserWindow, clipboard, dialog, ipcMain, Menu, MenuItemConstructorOptions, shell } from 'electron';
import { join } from 'node:path';
import { Backend, waitFor } from './backend';

const DEV_SERVER_URL = 'http://localhost:5173';
/** `npm run dev` passes this flag; every other start loads the built renderer from dist/. */
const USE_DEV_SERVER = process.argv.includes('--dev-server');
const BACKEND_PORT = Number(process.env.QUANTUS_PORT ?? 8437);
const BACKEND_URL = `http://127.0.0.1:${BACKEND_PORT}`;

const backend = new Backend(BACKEND_URL, BACKEND_PORT);
let window: BrowserWindow | null = null;

/** Everything the menu can trigger in the renderer. Mirrored by `Shortcut` in the UI. */
type Shortcut =
  | 'new'
  | 'evaluate'
  | 'toggle-steps'
  | 'copy-result'
  | 'copy-all'
  | 'toggle-variables'
  | 'toggle-history';

function send(shortcut: Shortcut): void {
  window?.webContents.send('quantus:shortcut', shortcut);
}

function createWindow(): BrowserWindow {
  const created = new BrowserWindow({
    width: 1180,
    height: 780,
    minWidth: 820,
    minHeight: 560,
    show: false,
    backgroundColor: '#17161D',
    title: 'Quantus',
    webPreferences: {
      preload: join(__dirname, 'preload.js'),
      contextIsolation: true,
      sandbox: true,
      // The renderer learns the backend address without an extra round trip.
      additionalArguments: [`--quantus-backend-url=${BACKEND_URL}`],
    },
  });

  created.once('ready-to-show', () => created.show());
  // Links to the outside belong in the browser, not in the app window.
  created.webContents.setWindowOpenHandler(({ url }) => {
    shell.openExternal(url);
    return { action: 'deny' };
  });

  if (USE_DEV_SERVER) {
    // `npm run dev` starts Vite and Electron together; the server may still be booting.
    waitFor(() => fetch(DEV_SERVER_URL).then((response) => response.ok).catch(() => false), 30_000, 250)
      .then(() => created.loadURL(DEV_SERVER_URL));
  } else {
    created.loadFile(join(__dirname, '..', 'dist', 'index.html'));
  }
  return created;
}

/**
 * The keyboard workflow of F10: everything that is not typing runs through menu
 * accelerators, so the shortcuts work no matter what has focus.
 */
function buildMenu(): void {
  const template: MenuItemConstructorOptions[] = [
    {
      label: 'Rechnung',
      submenu: [
        { label: 'Neue Rechnung', accelerator: 'CmdOrCtrl+N', click: () => send('new') },
        { label: 'Auswerten', accelerator: 'CmdOrCtrl+Return', click: () => send('evaluate') },
        { type: 'separator' },
        { label: 'Ergebnis kopieren', accelerator: 'CmdOrCtrl+Shift+C', click: () => send('copy-result') },
        { label: 'Ganze Rechnung kopieren', accelerator: 'CmdOrCtrl+Alt+C', click: () => send('copy-all') },
        { type: 'separator' },
        { role: 'quit', label: 'Beenden' },
      ],
    },
    {
      label: 'Bearbeiten',
      submenu: [
        { role: 'undo', label: 'Rückgängig' },
        { role: 'redo', label: 'Wiederherstellen' },
        { type: 'separator' },
        { role: 'cut', label: 'Ausschneiden' },
        { role: 'copy', label: 'Kopieren' },
        { role: 'paste', label: 'Einfügen' },
        { role: 'selectAll', label: 'Alles auswählen' },
      ],
    },
    {
      label: 'Ansicht',
      submenu: [
        { label: 'Rechenweg', accelerator: 'CmdOrCtrl+J', click: () => send('toggle-steps') },
        { label: 'Variablen', accelerator: 'CmdOrCtrl+,', click: () => send('toggle-variables') },
        { label: 'Verlauf', accelerator: 'CmdOrCtrl+H', click: () => send('toggle-history') },
        { type: 'separator' },
        { role: 'resetZoom', label: 'Zoom zurücksetzen' },
        { role: 'zoomIn', label: 'Größer' },
        { role: 'zoomOut', label: 'Kleiner' },
        { role: 'togglefullscreen', label: 'Vollbild' },
        ...(app.isPackaged ? [] : [{ role: 'toggleDevTools' } as MenuItemConstructorOptions]),
      ],
    },
  ];
  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}

// A second instance would fight over the backend port; hand over to the first one instead.
if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on('second-instance', () => {
    if (window) {
      if (window.isMinimized()) window.restore();
      window.focus();
    }
  });

  app.whenReady().then(async () => {
    // Clipboard access stays in the main process: the sandboxed preload only forwards.
    ipcMain.handle('quantus:copy', (_event, text: string) => clipboard.writeText(text));

    try {
      await backend.start();
    } catch (error) {
      dialog.showErrorBox('Quantus: Backend startet nicht', String(error instanceof Error ? error.message : error));
      app.quit();
      return;
    }

    buildMenu();
    window = createWindow();

    app.on('activate', () => {
      if (BrowserWindow.getAllWindows().length === 0) window = createWindow();
    });
  });

  app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') app.quit();
  });

  // Quitting waits for the backend so no orphan java process is left behind.
  let shutdown = false;
  app.on('before-quit', (event) => {
    if (shutdown) return;
    event.preventDefault();
    shutdown = true;
    backend.stop().finally(() => app.quit());
  });
}
