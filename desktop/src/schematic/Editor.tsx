import { useMemo, useRef, type KeyboardEvent, type ReactNode } from 'react';
import { useT } from '../i18n';
import { useSchematicEditing } from './editing';
import { useHistory } from './history';
import { ExportButtons } from './ExportButtons';
import { boundsOf, connectedPointsOf, freeSpot, junctionDotsOf, wireEnds, type Placeable, type Rect } from './geometry';
import { InspectorPopover } from './inspector';
import { editorKeys, keyName, runKey } from './keys';
import type { PartKit } from './kit';
import { labelStyle, useLabelPlacements, useLabelSizes, type CanvasLabel } from './labels';
import { DragGhost, JunctionDots, LeaderLines, PlacedPart, WirePart } from './parts';
import { schematicPicture } from './picture';
import { StatusBar } from './StatusBar';
import { GridBackground, HOME, useCanvasView, useFitOnRequest, useFullscreen, viewTransform, ZoomControls, type Fullscreen } from './view';

/** What the inspector of the chosen part can do. */
export interface InspectorActions<E> {
  /** Changes some of the part's fields. */
  update(change: Partial<E>): void;
  /** Puts another part in its place. */
  replace(next: E): void;
  remove(): void;
  close(): void;
}

export interface SchematicEditorProps<E extends Placeable, K extends string, S> {
  kit: PartKit<E, K, S>;
  elements: E[];
  onChange(next: E[]): void;
  /** The labels on the canvas, for what is drawn (`preview` while there is one). */
  labels: CanvasLabel[];
  /** Places the labels must keep clear of, besides the parts, such as what `overlay` draws. */
  reserved?: Rect[];
  /** SVG drawn over the parts, in drawing coordinates. */
  overlay?: ReactNode;
  /**
   * Another drawing shown in place of `elements` for as long as it is set, such as the network
   * after a merge of resistors. It is only looked at: nothing on the canvas can be edited then.
   */
  preview?: E[];
  /** More classes of a part, such as `highlighted`, `error` or `critical`. */
  partClass?(element: E): string | undefined;
  /** The classes of a wire's line, such as the value it carries. */
  wireClass?(wire: E): string | undefined;
  /** What a part shows of the simulation. */
  partState?(element: E): S;
  /** A part pressed and let go without moving it, of those the kit says only tap (a switch). */
  onTap?(element: E): void;
  /** The mode's class on the editor, beside `schematic-editor`. */
  className?: string;
  /** How to draw a wire, in the palette column: from a terminal or a pin. */
  hint: ReactNode;
  /** More controls in the palette column, after the parts (load and save). */
  paletteExtras?: ReactNode;
  /** Controls in a bar above the canvas. */
  toolbar?: ReactNode;
  /** More over the canvas, in its top right corner. */
  stageExtras?: ReactNode;
  /** Shown in the middle of the canvas while it is empty. */
  emptyCanvas?: ReactNode;
  /** The bar under the canvas: figures, then a note or the errors. */
  status?: { corner?: ReactNode; note?: ReactNode; error?: boolean };
  /** Shown under the canvas and its bar. */
  below?: ReactNode;
  /** Changed to show the whole drawing, as after it was loaded, drawn or arranged anew. */
  fitKey?: number;
  /** The full-screen switch; the editor has its own without it. */
  fullscreen?: Fullscreen;
  /** The values of the chosen part. */
  inspector(element: E, actions: InspectorActions<E>): ReactNode;
}

/**
 * The schematic editor of every mode: the palette, the canvas that pans and zooms, the parts and
 * wires on the grid, choosing, moving, turning, deleting and wiring them, the labels laid out clear
 * of the drawing, the inspector beside the chosen part, the keys, full screen and the picture. What
 * the parts are comes from the mode's `kit`; what is computed from them, from the mode.
 */
export function SchematicEditor<E extends Placeable, K extends string, S>(props: SchematicEditorProps<E, K, S>) {
  const { kit, elements, onChange, labels, reserved, overlay, preview, partClass, wireClass, partState, onTap, fitKey = 0 } = props;
  const { t } = useT();
  const canvas = useRef<SVGSVGElement>(null);
  const canvasView = useCanvasView(canvas);
  const { view, setView, toGrid } = canvasView;
  const history = useHistory(elements, onChange, fitKey);
  /** Whether a drag is on, so that its changes make one entry of the history. */
  const dragging = useRef(false);
  const editing = useSchematicEditing({
    elements,
    onChange: (next) => history.change(next, dragging.current ? 'pointer' : undefined),
    canvas,
    view: canvasView,
    pins: kit.pins,
    newWire: kit.newWire,
    turned: kit.turned,
    onTap,
    tapOnly: kit.tapOnly,
  });
  const { selection, inspected, drag, remove } = editing;
  dragging.current = drag !== null;
  const endDrag = () => {
    history.seal();
    dragging.current = false;
  };
  const ownFullscreen = useFullscreen();
  const fullscreen = props.fullscreen ?? ownFullscreen;

  /** What the canvas shows: the elements, or the preview while there is one. */
  const drawn = preview ?? elements;
  const ends = useMemo(() => drawn.flatMap(kit.pins), [drawn, kit]);
  const joining = useMemo(() => kit.joining(drawn), [drawn, kit]);
  const connected = useMemo(() => connectedPointsOf(ends, joining), [ends, joining]);
  const dots = useMemo(() => junctionDotsOf(kit.dotEnds?.(drawn) ?? ends, joining), [drawn, ends, joining, kit]);

  const { sizes, measure } = useLabelSizes();
  const obstacles = useMemo(() => [...drawn.flatMap(kit.obstacles), ...(reserved ?? [])], [drawn, reserved, kit]);
  const placements = useLabelPlacements(labels, sizes, obstacles, drag?.type === 'move' || drag?.type === 'end');

  /** Shows the whole drawing with its labels; the start view without one. */
  const fitAll = () => (drawn.length ? canvasView.fit(boundsOf([...obstacles, ...placements.values()])) : setView(HOME));
  useFitOnRequest(
    canvas,
    fitKey,
    labels.every((label) => sizes.has(label.id)),
    fitAll,
    placements,
  );

  const add = (kind: K, at?: ReturnType<typeof toGrid>) => {
    if (kit.canAdd && !kit.canAdd(elements, kind)) return;
    const part = kit.newPart(elements, kind, at ?? freeSpot(view, elements.length, kit.spacing));
    history.change([...elements, part]);
    editing.inspect(part.id);
    // The canvas takes the keys, so that Del removes the new part at once.
    canvas.current?.focus();
  };

  const bindings = [
    ...(kit.keys?.({ elements, selection, onChange: (next) => history.change(next) }) ?? []),
    ...editorKeys({
      removeSelection: () => remove(selection),
      rotateSelection: editing.rotateSelected,
      escape: () => {
        if (inspected) editing.closeInspector();
        else if (selection.length) editing.setSelection([]);
        // With nothing chosen, Esc is left to the full-screen view.
        else return false;
        return true;
      },
      fit: fitAll,
      undo: history.undo,
      redo: history.redo,
      view: canvasView,
    }),
  ];
  const onKeyDown = (event: KeyboardEvent) => void runKey(bindings, event);

  const chosen = (!preview && drag?.type !== 'move' && elements.find((e) => e.id === inspected)) || null;
  const picture = () => schematicPicture(kit, elements, labels, placements);

  return fullscreen.portal(
    <div className={['schematic-editor', props.className, fullscreen.full && 'fullscreen'].filter(Boolean).join(' ')}>
      <div className="schematic-palette">
        <div className="palette-title">{t('circuit.parts')}</div>
        {kit.palette.map((kind) => (
          <button
            key={kind}
            className="palette-item"
            draggable
            title={t('circuit.drag')}
            onDragStart={(event) => event.dataTransfer.setData('text/plain', kind)}
            onClick={() => add(kind)}
          >
            {kit.paletteIcon(kind)}
            <span>{kit.paletteLabel(kind, t)}</span>
          </button>
        ))}
        {props.paletteExtras}
        {kit.pictureStyle && <ExportButtons picture={picture} name={t('schematic.export.name')} disabled={!elements.length} />}
        <p className="palette-hint">{props.hint}</p>
        <p className="palette-hint">{t('circuit.pan')}</p>
        <details className="palette-keys">
          <summary>{t('schematic.keys')}</summary>
          <dl>
            {bindings.map((binding) => (
              <div key={binding.hint}>
                <dt>
                  {binding.keys.map((key) => (
                    <kbd key={key}>{keyName(key, t)}</kbd>
                  ))}
                </dt>
                <dd>{t(binding.hint)}</dd>
              </div>
            ))}
          </dl>
        </details>
      </div>

      <div className="schematic-centre">
        {props.toolbar && <div className="schematic-toolbar">{props.toolbar}</div>}
        <div className="schematic-stage">
          <ZoomControls view={canvasView} fit={fitAll} fullscreen={fullscreen} history={history} />
          <svg
            ref={canvas}
            className="schematic-canvas"
            tabIndex={0}
            onKeyDown={onKeyDown}
            onPointerMove={editing.onMove}
            onPointerUp={() => {
              editing.onUp();
              endDrag();
            }}
            onPointerCancel={() => {
              editing.setDrag(null);
              endDrag();
            }}
            onPointerDown={editing.startCanvas}
            onContextMenu={(event) => event.preventDefault()}
            onDragOver={(event) => event.preventDefault()}
            onDrop={(event) => {
              event.preventDefault();
              const kind = event.dataTransfer.getData('text/plain') as K;
              if (kit.palette.includes(kind)) add(kind, toGrid(event));
            }}
          >
            <GridBackground view={view} />
            <g transform={viewTransform(view)}>
              <g pointerEvents={preview ? 'none' : undefined}>
                {drawn.map((element) => {
                  const className = ['part', !preview && selection.includes(element.id) && 'selected', partClass?.(element)].filter(Boolean).join(' ');
                  if (element.kind === 'wire') {
                    const [a, b] = wireEnds(element);
                    return (
                      <WirePart
                        key={element.id}
                        a={a}
                        b={b}
                        className={className}
                        lineClassName={wireClass?.(element)}
                        joins={kit.wireJoins(element)}
                        connected={connected}
                        onPointerDown={(event) => editing.startMove(event, element)}
                        onEnd={(event, end) => editing.startEnd(event, element, end)}
                        onTerminal={(event, point) => editing.startDraw(event, point, element)}
                      />
                    );
                  }
                  return (
                    <PlacedPart
                      key={element.id}
                      element={element}
                      className={className}
                      symbol={kit.symbol(element, partState?.(element))}
                      hit={kit.hit(element)}
                      pins={kit.pins(element)}
                      decoration={kit.decoration?.(element)}
                      connected={connected}
                      onPointerDown={(event) => editing.startMove(event, element)}
                      onPin={(event, point) => (kit.pinMoves?.(element) ? editing.startMove(event, element) : editing.startDraw(event, point, element))}
                    />
                  );
                })}
              </g>
              <JunctionDots dots={dots} />
              {!preview && overlay}
              <LeaderLines labels={labels} placements={placements} />
              <DragGhost drag={drag} />
            </g>
          </svg>

          {/* The labels are HTML over the canvas, not foreignObject in it: WebKitGTK draws KaTeX's
              positioned spans inside a foreignObject at the origin of the SVG. */}
          {props.stageExtras}
          <div className="schematic-labels">
            <div className="schematic-labels-drawing" style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.zoom})` }}>
              {labels.map((label) => (
                <div key={label.id} className={label.className} style={labelStyle(placements.get(label.id), label.anchor)} ref={measure} data-label={label.id}>
                  {label.content}
                </div>
              ))}
            </div>
          </div>

          {!elements.length && !preview && props.emptyCanvas && <div className="schematic-empty">{props.emptyCanvas}</div>}

          {chosen && (
            <InspectorPopover box={kit.body(chosen)} view={view} canvas={canvas}>
              {props.inspector(chosen, {
                // Typing into one field is one entry of the history.
                update: (change) => history.change(elements.map((e) => (e.id === chosen.id ? { ...e, ...change } : e)), `field:${chosen.id}:${Object.keys(change).join()}`),
                replace: (next) => history.change(elements.map((e) => (e.id === next.id ? next : e))),
                remove: () => remove([chosen.id]),
                close: editing.closeInspector,
              })}
            </InspectorPopover>
          )}
        </div>
        <StatusBar {...props.status} />
        {props.below}
      </div>
    </div>,
  );
}
