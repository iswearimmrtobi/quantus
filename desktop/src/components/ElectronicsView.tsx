import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { api, ApiError, type CircuitResponse, type Settings } from '../api';
import { numberNodes, toRequest, type EditorElement, type Shown } from '../circuit';
import { useT } from '../i18n';
import { CircuitEditor } from './CircuitEditor';

interface Props {
  elements: EditorElement[];
  onChange(next: EditorElement[]): void;
  /** The solver reads the stored settings; these are watched so a change shows at once. */
  settings: Settings;
  shown: Shown;
  onShownChange(next: Shown): void;
  toolbar?: ReactNode;
}

/** The electronics mode: the circuit editor with the node-voltage solver behind it. */
export function ElectronicsView({ elements, onChange, settings, shown, onShownChange, toolbar }: Props) {
  const { t } = useT();
  const [response, setResponse] = useState<CircuitResponse | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const nodes = useMemo(() => numberNodes(elements), [elements]);
  const request = useMemo(() => toRequest(elements, nodes), [elements, nodes]);

  // Every change recalculates, and so does a changed language, precision or output setting; the
  // engine runs in the app, so a short debounce is enough to keep dragging smooth. A slow answer
  // to an older circuit must not overwrite a newer one.
  const { language, precision, exactRoots, letterProducts, extraUnitLetters, mode } = settings;
  useEffect(() => {
    let current = true;
    const timer = window.setTimeout(() => {
      if (request.elements.length === 0) {
        setResponse(null);
        setFailure(null);
        return;
      }
      api.solveCircuit(request).then(
        (solved) => {
          if (!current) return;
          setResponse(solved);
          setFailure(null);
        },
        (error) => {
          if (!current) return;
          setResponse(null);
          setFailure(error instanceof ApiError ? error.message : String(error));
        },
      );
    }, 60);
    return () => {
      current = false;
      window.clearTimeout(timer);
    };
  }, [request, language, precision, exactRoots, letterProducts, extraUnitLetters, mode]);

  const voltages = useMemo(
    () =>
      response?.nodes.flatMap(({ id, voltage }) => {
        const point = nodes.pointOf.get(id);
        return point ? [{ point, voltage }] : [];
      }),
    [response, nodes],
  );

  const ungrounded = elements.length > 0 && !nodes.grounded && nodes.byPoint.size > 0;
  return (
    <CircuitEditor
      elements={elements}
      onChange={onChange}
      results={response?.elements}
      voltages={voltages}
      failure={failure}
      note={ungrounded ? t('circuit.noGround') : null}
      shown={shown}
      onShownChange={onShownChange}
      toolbar={toolbar}
    />
  );
}
