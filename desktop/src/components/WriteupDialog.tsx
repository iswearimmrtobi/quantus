import { useEffect, useState, type KeyboardEvent } from 'react';
import { api, type Writeup } from '../api';
import { useT } from '../i18n';
import { copyText } from '../shell';
import { displayStyle, Katex } from './Katex';

/**
 * The session written up the way school physics wants it: given values top left, wanted
 * quantities top right, and below them the solution, one block per quantity with the law, the
 * rearrangement, the values put in, the result and the unit check. Built by the engine from the
 * whole history of the session; the button copies it as LaTeX.
 */
export function WriteupDialog(props: { onClose: () => void }) {
  const { t, language } = useT();
  const [writeup, setWriteup] = useState<Writeup | null>(null);
  const [failed, setFailed] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    api.writeup().then(setWriteup, () => setFailed(true));
  }, [language]);

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      event.stopPropagation();
      props.onClose();
    }
  };

  const copy = async () => {
    if (!writeup) return;
    await copyText(writeup.latex);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };

  const empty = writeup !== null && writeup.blocks.length === 0;

  return (
    <div className="dialog-backdrop" onMouseDown={props.onClose}>
      <div
        className="dialog settings-dialog writeup-dialog"
        role="dialog"
        aria-label={t('writeup')}
        onMouseDown={(event) => event.stopPropagation()}
        onKeyDown={onKeyDown}
        tabIndex={-1}
      >
        <div className="settings-head">
          <h3>{t('writeup')}</h3>
          <div className="writeup-actions">
            <button className="copy-btn" disabled={!writeup || empty} onClick={() => void copy()}>
              {copied ? t('copied') : t('writeup.copy')}
            </button>
            <button className="settings-close" title={t('settings.close')} onClick={props.onClose}>
              ✕
            </button>
          </div>
        </div>
        <div className="settings-body writeup">
          {failed && <p className="panel-empty">{t('writeup.failed')}</p>}
          {empty && <p className="panel-empty">{t('writeup.empty')}</p>}
          {writeup && !empty && (
            <>
              <div className="writeup-top">
                <section>
                  <h4>{t('writeup.given')}</h4>
                  {writeup.given.length === 0 && <p className="writeup-none">–</p>}
                  {writeup.given.map((given) => (
                    <Katex key={given} latex={given} className="writeup-row" />
                  ))}
                </section>
                <section>
                  <h4>{t('writeup.wanted')}</h4>
                  {writeup.wanted.length === 0 && <p className="writeup-none">–</p>}
                  {writeup.wanted.map((wanted) => (
                    <Katex key={wanted} latex={wanted} className="writeup-row" />
                  ))}
                </section>
              </div>
              <section className="writeup-solution">
                <h4>{t('writeup.solution')}</h4>
                {writeup.blocks.map((block, index) => (
                  <div key={index} className="writeup-block">
                    {block.lines.map((line, row) => (
                      <div key={row} className={`writeup-line writeup-${line.kind}`}>
                        {line.text && <p className="writeup-text">{line.text}</p>}
                        <Katex latex={displayStyle(line.latex)} display className="writeup-math" />
                      </div>
                    ))}
                  </div>
                ))}
              </section>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
