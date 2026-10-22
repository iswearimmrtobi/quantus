/** What the preload script bridges into the renderer. */
interface QuantusBridge {
  /** Clipboard via Electron, so copying works without the DOM clipboard permission dance. */
  copy(text: string): Promise<void>;
  /** Menu accelerators; returns the unsubscribe function. */
  onShortcut(handler: (shortcut: string) => void): () => void;
}

interface Window {
  /** Absent when the UI runs in a plain browser (`npm run dev:renderer`). */
  quantus?: QuantusBridge;
}
