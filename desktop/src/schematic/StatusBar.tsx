import type { ReactNode } from 'react';

/**
 * The bar under an editor's canvas: figures on the left (`corner`), then a note, or the errors in
 * red. Left out when there is neither.
 */
export function StatusBar({ corner, note, error = false }: { corner?: ReactNode; note?: ReactNode; error?: boolean }) {
  if (!corner && !note) return null;
  return (
    <div className="schematic-bar">
      {corner}
      {note && <div className={error ? 'schematic-bar-note error' : 'schematic-bar-note'}>{note}</div>}
    </div>
  );
}
