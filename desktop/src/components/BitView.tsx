import { useState } from 'react';
import type { BitView as Bits } from '../api';
import { useT } from '../i18n';
import { Segmented } from './SettingsDialog';
import { Katex } from './Katex';

/** Bits per row: 64 bits take two. */
const ROW = 32;

type Tab = 'integer' | 'ieee';

/**
 * The bits of a Basis-N result in groups of four, the index under each group's lowest bit, the
 * sign bit marked when the word is signed. A click (or Space on a focused bit) flips the bit. At
 * 32 and 64 bits a second tab reads the pattern as an IEEE-754 float: sign, exponent and mantissa
 * in their colours, each as a number, the exact value and the rounding error.
 */
export function BitView(props: { view: Bits; onFlip: (index: number) => void }) {
  const { t } = useT();
  const [tab, setTab] = useState<Tab>('integer');
  const { bits, width, signed, ieee } = props.view;
  const float = tab === 'ieee' && ieee ? ieee : null;
  const mantissaWidth = ieee?.mantissaBits.length ?? 0;
  const field = (index: number) => (index === width - 1 ? 'sign' : index >= mantissaWidth ? 'exponent' : 'mantissa');
  const rows: number[][] = [];
  for (let top = width - 1; top >= 0; top -= ROW) {
    const row: number[] = [];
    for (let index = top; index > top - Math.min(ROW, width) && index >= 0; index--) row.push(index);
    rows.push(row);
  }
  return (
    <div className="bit-panel">
      {ieee && (
        <Segmented
          choices={[
            { value: 'integer', label: t('basen.integer') },
            { value: 'ieee', label: 'IEEE 754' },
          ]}
          value={tab}
          onChange={setTab}
        />
      )}
      <div className="bit-view" role="group" aria-label={t('basen.bits')}>
        {rows.map((row) => (
          <div className="bit-row" key={row[0]}>
            {Array.from({ length: row.length / 4 }, (_, group) => {
              const indices = row.slice(group * 4, group * 4 + 4);
              const low = indices[indices.length - 1];
              return (
                <div className="bit-group" key={low}>
                  <div className="bit-cells">
                    {indices.map((index) => {
                      const one = bits[width - 1 - index] === '1';
                      const sign = signed && index === width - 1;
                      const label = t(sign ? 'basen.signBit' : 'basen.bit', { index });
                      const colour = float ? ` field-${field(index)}` : sign ? ' sign' : '';
                      return (
                        <button
                          key={index}
                          className={`bit${one ? ' one' : ''}${colour}`}
                          title={label}
                          aria-label={label}
                          aria-pressed={one}
                          onClick={() => props.onFlip(index)}
                        >
                          {one ? '1' : '0'}
                        </button>
                      );
                    })}
                  </div>
                  <span className="bit-index">{low}</span>
                </div>
              );
            })}
          </div>
        ))}
      </div>
      {float && (
        <dl className="ieee-fields">
          <dt className="field-sign">{t('basen.sign')}</dt>
          <dd>
            <code>{float.sign ? '1' : '0'}</code> {float.sign ? '−' : '+'}
          </dd>
          <dt className="field-exponent">{t('basen.exponent')}</dt>
          <dd>
            <code>{float.exponentBits}</code> {float.exponent != null && <Katex latex={`2^{${float.exponent}}`} />}
          </dd>
          <dt className="field-mantissa">{t('basen.mantissa')}</dt>
          <dd>
            <code>{float.mantissaBits}</code>
          </dd>
          <dt>{t('basen.class')}</dt>
          <dd>{t(`basen.class.${float.class}`)}</dd>
          <dt>{t('basen.exact')}</dt>
          <dd>
            <Katex latex={float.exact} />
          </dd>
          {float.typed && float.error && (
            <>
              <dt>{t('basen.typed')}</dt>
              <dd>
                <Katex latex={float.typed} />
              </dd>
              <dt>{t('basen.roundingError')}</dt>
              <dd>
                <Katex latex={float.error} />
              </dd>
            </>
          )}
        </dl>
      )}
    </div>
  );
}
