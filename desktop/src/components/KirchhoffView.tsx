import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { api, ApiError, type Kirchhoff, type Settings } from '../api';
import {
  GRID,
  IDEAL_FIELDS,
  junctionPoint,
  numberNodes,
  toKirchhoffRequest,
  type EditorElement,
  type EditorKind,
  type Shown,
} from '../circuit';
import { useT } from '../i18n';
import { CircuitEditor } from './CircuitEditor';
import { currentArrows, KirchhoffOverlay, overlayLabels, overlayReserved, type Hover } from './KirchhoffOverlay';

const PALETTE: EditorKind[] = ['voltageSource', 'currentSource', 'resistor', 'wire'];

interface Props {
  elements: EditorElement[];
  onChange(next: EditorElement[]): void;
  /** The engine reads the stored settings; these are watched so a change shows at once. */
  settings: Settings;
  shown: Shown;
  onShownChange(next: Shown): void;
  toolbar?: ReactNode;
}

/**
 * The Kirchhoff mode: a resistor network drawn in the circuit editor, worked out by the junction
 * and loop rules.
 */
export function KirchhoffView({ elements, onChange, settings, shown, onShownChange, toolbar }: Props) {
  const { t } = useT();
  const [solution, setSolution] = useState<Kirchhoff | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [hover, setHover] = useState<Hover>(null);

  const request = useMemo(() => toKirchhoffRequest(elements), [elements]);

  // As in the electronics mode: every change recalculates after a short debounce, and an answer
  // to an older circuit is dropped. The step settings change the working, so they count too.
  const { language, precision, exactRoots, letterProducts, extraUnitLetters, stepDetail, cancelMarks, elimination } = settings;
  useEffect(() => {
    let current = true;
    const timer = window.setTimeout(() => {
      if (request.elements.length === 0) {
        setSolution(null);
        setFailure(null);
        return;
      }
      api.solveKirchhoff(request).then(
        (solved) => {
          if (!current) return;
          setSolution(solved);
          setFailure(null);
        },
        (error) => {
          if (!current) return;
          setSolution(null);
          setFailure(error instanceof ApiError ? error.message : String(error));
        },
      );
    }, 60);
    return () => {
      current = false;
      window.clearTimeout(timer);
    };
  }, [request, language, precision, exactRoots, letterProducts, extraUnitLetters, stepDetail, cancelMarks, elimination]);

  const nodes = useMemo(() => numberNodes(elements), [elements]);
  // The overlay belongs to the drawing it was solved for; while a change is being solved the
  // parts may have moved, which only shifts the marks until the answer comes.
  const marks = useMemo(() => {
    if (!solution) return null;
    const junctions = solution.junctions.map((junction, index) => ({
      index,
      point: junctionPoint(elements, nodes, { x: junction.anchor.x * GRID, y: junction.anchor.y * GRID }),
    }));
    const arrows = currentArrows(solution, elements);
    return { junctions, arrows, labels: overlayLabels(solution, junctions, arrows), reserved: overlayReserved(solution, arrows) };
  }, [solution, elements, nodes]);

  /** The parts of the loop under the pointer. */
  const highlight = useMemo(() => {
    if (hover?.kind !== 'loop' || !solution) return undefined;
    return solution.loops[hover.index]?.members.map((m) => m.id);
  }, [hover, solution]);

  const flip = (lead: string) => onChange(elements.map((e) => (e.id === lead ? { ...e, reversed: !e.reversed } : e)));

  return (
    <CircuitEditor
      elements={elements}
      onChange={onChange}
      palette={PALETTE}
      fields={IDEAL_FIELDS}
      results={solution?.elements}
      overlay={
        solution &&
        marks && (
          <KirchhoffOverlay
            solution={solution}
            junctions={marks.junctions}
            arrows={marks.arrows}
            hover={hover}
            onHover={setHover}
            onFlip={flip}
          />
        )
      }
      labels={marks?.labels}
      reserved={marks?.reserved}
      highlight={highlight}
      failure={failure ?? solution?.error}
      note={elements.length === 0 ? t('kirchhoff.empty') : null}
      shown={shown}
      onShownChange={onShownChange}
      toolbar={toolbar}
    />
  );
}
