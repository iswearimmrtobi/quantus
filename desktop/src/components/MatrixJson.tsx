import { useRef, useState } from 'react';
import { useT } from '../i18n';
import { MatrixJsonError, parseMatrixJson, type MatrixInput } from '../linear';

const EXAMPLE = '{"unknowns": ["x_1", "x_2"], "rows": [[2, 1, 5], [1, "\\\\frac{1}{2}", 1]]}';

/**
 * A matrix of any size as JSON, pasted or from a file. A broken input is reported here and
 * changes nothing.
 */
export function MatrixJson({ onLoad }: { onLoad(matrix: MatrixInput): void }) {
  const { t } = useT();
  const [text, setText] = useState('');
  const [error, setError] = useState<string | null>(null);
  const file = useRef<HTMLInputElement>(null);

  const load = (json: string) => {
    try {
      onLoad(parseMatrixJson(json));
      setError(null);
    } catch (failure) {
      setError(failure instanceof MatrixJsonError ? t(failure.key, failure.values) : String(failure));
    }
  };

  return (
    <details className="matrix-json">
      <summary>{t('linear.json')}</summary>
      <textarea
        className="latex-input json-text"
        value={text}
        spellCheck={false}
        rows={4}
        placeholder={EXAMPLE}
        onChange={(event) => setText(event.target.value)}
      />
      <div className="task-inputs">
        <button className="palette-item" disabled={!text.trim()} onClick={() => load(text)}>
          {t('linear.json.apply')}
        </button>
        <button className="palette-item" onClick={() => file.current?.click()}>
          {t('linear.json.file')}
        </button>
        <input
          ref={file}
          type="file"
          accept=".json,application/json"
          hidden
          onChange={async (event) => {
            const chosen = event.target.files?.[0];
            event.target.value = '';
            if (chosen) load(await chosen.text());
          }}
        />
      </div>
      {error && (
        <div className="error-row">
          <span className="error-label">{t('error')}</span>
          <span className="error-text">{error}</span>
        </div>
      )}
    </details>
  );
}
