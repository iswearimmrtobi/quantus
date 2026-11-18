import { useCallback, useEffect, useRef, useState } from 'react';
import type { CSSProperties } from 'react';
import { api, ApiError, type Geometry, type Settings } from '../api';
import { geometryRequest, type GeometryInput } from '../geometry';
import { useT } from '../i18n';
import { asError, ErrorRow } from './AnalysisView';
import { GeometryPlot } from './GeometryPlot';
import { GeometryTaskPanel } from './GeometryTask';
import { GeometrySidebar } from './GeometrySidebar';


/** The sidebar's width in pixels: at first, and the range dragging keeps it in. */
const SIDEBAR_WIDTH = 380;
const MIN_SIDEBAR = 240;
const MAX_SIDEBAR = 600;
/** Where the sidebar's width and whether it is folded are kept, per viewer. */
const SIDEBAR_KEY = 'quantus.geometry.sidebar';

interface Sidebar {
  width: number;
  collapsed: boolean;
}

function loadSidebar(): Sidebar {
  try {
    const stored = JSON.parse(localStorage.getItem(SIDEBAR_KEY) ?? 'null') as Partial<Sidebar> | null;
    const width = typeof stored?.width === 'number' ? Math.min(MAX_SIDEBAR, Math.max(MIN_SIDEBAR, stored.width)) : SIDEBAR_WIDTH;
    return { width, collapsed: stored?.collapsed === true };
  } catch {
    return { width: SIDEBAR_WIDTH, collapsed: false };
  }
}

function saveSidebar(sidebar: Sidebar) {
  try {
    localStorage.setItem(SIDEBAR_KEY, JSON.stringify(sidebar));
  } catch {
    // Without storage the width is only kept until the app closes.
  }
}

interface Props {
  settings: Settings;
  input: GeometryInput;
  onInputChange(input: GeometryInput): void;
  /** An example for the typed field. */
  placeholder?: string;
}

/**
 * The Geometry mode: a scene of points, vectors, lines, planes, spheres and circles, drawn in a
 * 3D view, and a task on them answered exactly with its steps.
 */
export function GeometryView({ settings, input, onInputChange, placeholder }: Props) {
  const { t } = useT();
  /** The row whose editor is open, and the one the pointer is over; both by place in the scene. */
  const [open, setOpen] = useState<number | null>(null);
  const [hovered, setHovered] = useState<number | null>(null);
  /**
   * The objects a task is looked for, at most two: the open row, or those Ctrl-clicked in the
   * sidebar or the view.
   */
  const [selected, setSelected] = useState<number[]>([]);
  const select = (index: number) =>
    setSelected((current) => (current.includes(index) ? current.filter((i) => i !== index) : [...current, index].slice(-2)));
  const openRow = (index: number | null) => {
    setOpen(index);
    setSelected(index === null ? [] : [index]);
  };
  // An object removed takes its place out of the selection.
  const count = input.objects.length;
  useEffect(() => {
    setSelected((current) => current.filter((index) => index < count));
  }, [count]);
  const [sidebar, setSidebar] = useState(loadSidebar);
  useEffect(() => saveSidebar(sidebar), [sidebar]);
  const resizing = useRef<{ x: number; width: number } | null>(null);
  const startResize = (event: React.PointerEvent) => {
    resizing.current = { x: event.clientX, width: sidebar.width };
    (event.currentTarget as Element).setPointerCapture(event.pointerId);
  };
  const resize = (event: React.PointerEvent) => {
    const start = resizing.current;
    if (!start) return;
    const width = Math.min(MAX_SIDEBAR, Math.max(MIN_SIDEBAR, start.width + event.clientX - start.x));
    setSidebar((s) => ({ ...s, width }));
  };
  const [answer, setAnswer] = useState<Geometry | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  /** The latest request; an answer to an older one is dropped. */
  const request = useRef(0);

  const run = useCallback(async (current: GeometryInput) => {
    const id = ++request.current;
    const next = geometryRequest(current);
    if (!next) {
      setAnswer(null);
      setError(null);
      return;
    }
    try {
      const result = await api.geometry(next);
      if (id !== request.current) return;
      setAnswer(result);
      setError(null);
    } catch (failure) {
      if (id !== request.current) return;
      setAnswer(null);
      setError(asError(failure));
    }
  }, []);

  // On opening the page, on every change and whenever a setting changes how the scene is written.
  const { language, precision, stepDetail, angleUnit } = settings;
  useEffect(() => {
    void run(input);
  }, [input, language, precision, stepDetail, angleUnit, run]);

  const columns = { '--sidebar-w': `${sidebar.width}px` } as CSSProperties;
  return (
    <main className="workspace geometry-workspace">
      <div className={`geometry-layout${sidebar.collapsed ? ' collapsed' : ''}`} style={columns}>
        {sidebar.collapsed ? (
          <aside className="geometry-strip">
            <button className="copy-btn" title={t('geometry.sidebar.expand')} aria-label={t('geometry.sidebar.expand')} onClick={() => setSidebar((s) => ({ ...s, collapsed: false }))}>
              »
            </button>
          </aside>
        ) : (
          <>
            <GeometrySidebar
              objects={input.objects}
              built={answer?.objects ?? null}
              onChange={(objects) => onInputChange({ ...input, objects })}
              open={open}
              onOpen={openRow}
              selected={selected}
              onSelect={select}
              onHover={setHovered}
              hideSteps={stepDetail === 'none'}
              placeholder={placeholder}
              headEnd={
                <button className="copy-btn" title={t('geometry.sidebar.collapse')} aria-label={t('geometry.sidebar.collapse')} onClick={() => setSidebar((s) => ({ ...s, collapsed: true }))}>
                  «
                </button>
              }
              error={error && <ErrorRow error={error} soft />}
              footer={
                input.objects.length > 0 && (
                  <GeometryTaskPanel
                    task={input.task}
                    built={answer?.objects ?? null}
                    answer={answer?.task ?? null}
                    onChange={(task) => onInputChange({ ...input, task })}
                    hideSteps={stepDetail === 'none'}
                    selected={selected.flatMap((index) => answer?.objects[index] ?? [])}
                  />
                )
              }
            />
            <div
              className="geometry-splitter"
              role="separator"
              aria-orientation="vertical"
              title={t('geometry.sidebar.resize')}
              onPointerDown={startResize}
              onPointerMove={resize}
              onPointerUp={() => (resizing.current = null)}
              onPointerCancel={() => (resizing.current = null)}
            />
          </>
        )}
        <section className="geometry-stage">
          {input.objects.length > 0 ? (
            <GeometryPlot
              objects={input.objects}
              highlight={hovered !== null ? [hovered] : selected}
              onPick={(index, add) => {
                setSidebar((s) => ({ ...s, collapsed: false }));
                if (add) select(index);
                else openRow(index);
              }}
            />
          ) : (
            <p className="analysis-empty">{t('geometry.empty')}</p>
          )}
        </section>
      </div>
    </main>
  );
}
