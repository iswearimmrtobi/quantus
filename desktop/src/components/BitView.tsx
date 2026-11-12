import type { BitView as Bits } from '../api';
import { useT } from '../i18n';

/** Bits per row: 64 bits take two. */
const ROW = 32;

/**
 * The bits of a Basis-N result in groups of four, the index under each group's lowest bit, the
 * sign bit marked when the word is signed. A click (or Space on a focused bit) flips the bit.
 */
export function BitView(props: { view: Bits; onFlip: (index: number) => void }) {
  const { t } = useT();
  const { bits, width, signed } = props.view;
  const rows: number[][] = [];
  for (let top = width - 1; top >= 0; top -= ROW) {
    const row: number[] = [];
    for (let index = top; index > top - Math.min(ROW, width) && index >= 0; index--) row.push(index);
    rows.push(row);
  }
  return (
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
                    return (
                      <button
                        key={index}
                        className={`bit${one ? ' one' : ''}${sign ? ' sign' : ''}`}
                        title={t(sign ? 'basen.signBit' : 'basen.bit', { index })}
                        aria-label={t(sign ? 'basen.signBit' : 'basen.bit', { index })}
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
  );
}
