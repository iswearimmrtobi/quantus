/**
 * The colour theme: sets the CSS variables of styles.css from a theme file (quantus-themes). Everything,
 * MathLive, KaTeX and the plots included, takes its colours from these variables.
 */
import { useCallback, useEffect, useState } from 'react';
import { api, type ThemeColors } from './api';

/** The setting that follows the system: the built-in light or dark theme. */
export const SYSTEM_THEME = 'system';

export function applyTheme(colors: ThemeColors): void {
  const root = document.documentElement;
  for (const [name, value] of colors.vars) root.style.setProperty(name, value);
  root.style.colorScheme = colors.dark ? 'dark' : 'light';
  root.dataset.theme = colors.id;
}

const darkQuery = (): MediaQueryList | null =>
  typeof window.matchMedia === 'function' ? window.matchMedia('(prefers-color-scheme: dark)') : null;

/** Whether the system asks for dark colours; dark if it does not say. */
function useSystemDark(): boolean {
  const [dark, setDark] = useState(() => darkQuery()?.matches ?? true);
  useEffect(() => {
    const query = darkQuery();
    if (!query) return;
    const changed = () => setDark(query.matches);
    query.addEventListener('change', changed);
    return () => query.removeEventListener('change', changed);
  }, []);
  return dark;
}

/**
 * Shows the theme `id` (a theme id or {@link SYSTEM_THEME}); `null` waits, so the saved theme is
 * shown first rather than the system's. Returns why the theme cannot be shown, if so, and a
 * function that reads its file again.
 */
export function useTheme(id: string | null): { error: string | null; reload: () => void } {
  const systemDark = useSystemDark();
  const [error, setError] = useState<string | null>(null);
  const [generation, setGeneration] = useState(0);
  const shown = id === SYSTEM_THEME ? (systemDark ? 'dark' : 'light') : id;

  useEffect(() => {
    if (shown === null) return;
    let current = true;
    api.theme(shown).then(
      (colors) => {
        if (!current) return;
        applyTheme(colors);
        setError(colors.error);
      },
      (failure) => current && setError(String(failure)),
    );
    return () => {
      current = false;
    };
  }, [shown, generation]);

  const reload = useCallback(() => setGeneration((n) => n + 1), []);
  return { error, reload };
}
