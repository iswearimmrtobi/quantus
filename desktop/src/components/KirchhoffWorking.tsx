import { useState } from 'react';
import { ApiError, type Kirchhoff } from '../api';
import { useT } from '../i18n';
import type { TextKey } from '../i18n/en';
import { matrixLatex } from '../linear';
import { copyText } from '../shell';
import { ErrorRow, Steps } from './AnalysisView';
import { Katex } from './Katex';
import type { Hover } from './KirchhoffOverlay';

interface Props {
  solution: Kirchhoff | null;
  /** Why there is no solution. */
  failure: string | null;
  empty: boolean;
  stepsHidden: boolean;
  hover: Hover;
  onHover(hover: Hover): void;
}

/**
 * The working under the canvas, in the order a student writes it: the junction and loop rules,
 * the matrix, its Gauss steps, the currents and the power balance.
 */
export function KirchhoffWorking({ solution, failure, empty, stepsHidden, hover, onHover }: Props) {
  const { t } = useT();
  const [copied, setCopied] = useState(false);

  if (!solution) {
    return failure ? (
      <ErrorRow error={new ApiError(failure)} soft />
    ) : (
      <p className="analysis-empty">{t(empty ? 'kirchhoff.empty' : 'kirchhoff.waiting')}</p>
    );
  }

  const copyAll = async () => {
    await copyText(solution.alignBlock);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };
  const active = (kind: 'loop' | 'junction', index: number) => hover?.kind === kind && hover.index === index;
  const hovering = (kind: 'loop' | 'junction', index: number) => ({
    onMouseEnter: () => onHover({ kind, index }),
    onMouseLeave: () => onHover(null),
  });

  return (
    <div className="kirchhoff-sheet">
      <div className="kirchhoff-head">
        <div className="kirchhoff-notes">
          {solution.notes.map((note) => (
            <p className="kirchhoff-note" key={note}>
              {note}
            </p>
          ))}
        </div>
        <button className="copy-btn" title={t('kirchhoff.copy.hint')} onClick={() => void copyAll()}>
          {copied ? t('copied') : t('copy')}
        </button>
      </div>

      <Section title="kirchhoff.junctions">
        {solution.junctions.map((junction, index) => (
          <div
            key={junction.name}
            className={['kirchhoff-rule', junction.dropped && 'dropped', active('junction', index) && 'active'].filter(Boolean).join(' ')}
            title={junction.dropped ? t('kirchhoff.dropped') : undefined}
            {...hovering('junction', index)}
          >
            <Katex className="kirchhoff-rule-name" latex={`${junction.name}:`} />
            <Katex latex={junction.symbolic} />
          </div>
        ))}
      </Section>

      <Section title="kirchhoff.loops">
        {solution.loops.map((loop, index) => (
          <div
            key={loop.name}
            className={active('loop', index) ? 'kirchhoff-rule active' : 'kirchhoff-rule'}
            {...hovering('loop', index)}
          >
            <Katex className="kirchhoff-rule-name" latex={`${loop.name}:`} />
            <div className="kirchhoff-rule-forms">
              <Katex latex={loop.symbolic} />
              <Katex className="kirchhoff-numeric" latex={loop.numeric} />
            </div>
          </div>
        ))}
      </Section>

      <Section title="kirchhoff.matrix">
        <Katex className="kirchhoff-unknowns" latex={solution.unknowns.join(',\\ ')} />
        {solution.matrix.length > 0 && <Katex latex={matrixLatex({ unknowns: solution.unknowns, rows: solution.matrix })} display />}
        <Steps steps={solution.steps} hidden={stepsHidden} />
      </Section>

      {solution.error && <ErrorRow error={new ApiError(solution.error)} />}

      {solution.result.length > 0 && (
        <Section title="kirchhoff.results">
          {solution.result.map((line) => (
            <Katex key={line} className="kirchhoff-result" latex={line} />
          ))}
        </Section>
      )}

      {solution.powerBalance && (
        <Section title="kirchhoff.power">
          <Katex latex={solution.powerBalance.latex} />
        </Section>
      )}
    </div>
  );
}

function Section({ title, children }: { title: TextKey; children: React.ReactNode }) {
  const { t } = useT();
  return (
    <section className="kirchhoff-section">
      <h3 className="kirchhoff-title">{t(title)}</h3>
      {children}
    </section>
  );
}
