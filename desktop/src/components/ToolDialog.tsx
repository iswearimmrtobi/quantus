import { useEffect, useState, type KeyboardEvent } from 'react';
import { api, ApiError, type Calculation, type FreeVariable } from '../api';
import { useT } from '../i18n';
import { Katex } from './Katex';

export type ToolKind = 'calc' | 'solve';

/**
 * CALC and SOLVE as on a Casio. CALC asks for a value for every letter of the input that has
 * none and evaluates it with them; SOLVE asks which letter to solve for, takes values for the
 * others and an optional start value for Newton's method. The values become session variables.
 */
export function ToolDialog(props: {
  kind: ToolKind;
  latex: string;
  onDone: (result: Calculation) => void;
  onClose: () => void;
}) {
  const { t } = useT();
  const { kind, latex } = props;
  const [letters, setLetters] = useState<FreeVariable[] | null>(null);
  const [values, setValues] = useState<Record<string, string>>({});
  const [unknown, setUnknown] = useState('');
  const [start, setStart] = useState('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.freeVariables(latex).then(
      (found) => {
        setLetters(found);
        setUnknown(found[0]?.key ?? '');
      },
      (failure) => setError(failure instanceof Error ? failure.message : String(failure)),
    );
  }, [latex]);

  const run = async () => {
    const assignments = (letters ?? [])
      .filter((letter) => !(kind === 'solve' && letter.key === unknown))
      .filter((letter) => values[letter.key]?.trim())
      .map((letter) => ({ key: letter.key, latex: values[letter.key].trim() }));
    try {
      const result =
        kind === 'calc'
          ? await api.calc(latex, assignments)
          : await api.solveFor(latex, unknown, assignments, start.trim() || null);
      props.onDone(result);
    } catch (failure) {
      setError(failure instanceof ApiError ? failure.message : String(failure));
    }
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'Enter') {
      event.preventDefault();
      void run();
    } else if (event.key === 'Escape') {
      event.stopPropagation();
      props.onClose();
    }
  };

  return (
    <div className="dialog-backdrop" onMouseDown={props.onClose}>
      <div className="dialog" onMouseDown={(event) => event.stopPropagation()} onKeyDown={onKeyDown}>
        <h3>{t(kind === 'calc' ? 'tool.calc' : 'tool.solve')}</h3>
        <Katex className="dialog-input" latex={latex} />
        {letters === null && !error && <p className="setting-hint">…</p>}
        {letters?.length === 0 && <p className="setting-hint">{t('tool.noLetters')}</p>}
        {kind === 'solve' && letters && letters.length > 0 && (
          <p className="setting-hint">{t('tool.solve.hint')}</p>
        )}
        {letters?.map((letter, index) => {
          const isUnknown = kind === 'solve' && letter.key === unknown;
          return (
            <label className="dialog-row" key={letter.key}>
              {kind === 'solve' && (
                <input type="radio" name="unknown" checked={isUnknown} onChange={() => setUnknown(letter.key)} />
              )}
              <Katex className="dialog-letter" latex={`${letter.symbol} =`} />
              <input
                className="library-filter"
                autoFocus={index === (kind === 'solve' ? 1 : 0)}
                disabled={isUnknown}
                placeholder={isUnknown ? '?' : t('tool.fromSession')}
                value={isUnknown ? '' : values[letter.key] ?? ''}
                onChange={(event) => setValues({ ...values, [letter.key]: event.target.value })}
              />
            </label>
          );
        })}
        {kind === 'solve' && letters && letters.length > 0 && (
          <label className="dialog-row">
            <span className="dialog-letter">{t('tool.start')}</span>
            <input
              className="library-filter"
              placeholder={t('tool.start.hint')}
              value={start}
              onChange={(event) => setStart(event.target.value)}
            />
          </label>
        )}
        {error && <p className="setting-error">{error}</p>}
        <div className="dialog-buttons">
          <button className="setting-reset" onClick={props.onClose}>
            {t('sessions.cancel')}
          </button>
          <button
            className="setting-reset primary"
            disabled={kind === 'solve' && !unknown}
            onClick={() => void run()}
          >
            {t(kind === 'calc' ? 'tool.calc.run' : 'tool.solve.run')}
          </button>
        </div>
      </div>
    </div>
  );
}
