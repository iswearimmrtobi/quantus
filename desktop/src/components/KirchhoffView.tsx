import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { api, ApiError, type Kirchhoff, type Settings } from '../api';
import { IDEAL_FIELDS, toKirchhoffRequest, type EditorElement, type EditorKind, type Shown } from '../circuit';
import { useT } from '../i18n';
import { CircuitEditor } from './CircuitEditor';

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

  return (
    <CircuitEditor
      elements={elements}
      onChange={onChange}
      palette={PALETTE}
      fields={IDEAL_FIELDS}
      results={solution?.elements}
      failure={failure ?? solution?.error}
      note={elements.length === 0 ? t('kirchhoff.empty') : null}
      shown={shown}
      onShownChange={onShownChange}
      toolbar={toolbar}
    />
  );
}
