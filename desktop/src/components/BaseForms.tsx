import type { NumberBase, ResultForm } from '../api';
import { useT } from '../i18n';

const ORDER: NumberBase[] = ['hex', 'dec', 'oct', 'bin'];

/** The digits of a Basis-N result's LaTeX: `\mathrm{FF}` → `FF`, `-5` stays. */
export function plainDigits(latex: string): string {
  return latex.replace(/\\mathrm\{([^{}]*)\}/g, '$1').replace(/\s+/g, '');
}

/** Binary digits in groups of four from the right: `101101` → `10 1101`. */
export function groupedBinary(digits: string): string {
  const groups: string[] = [];
  for (let end = digits.length; end > 0; end -= 4) groups.unshift(digits.slice(Math.max(0, end - 4), end));
  return groups.join(' ');
}

/**
 * A Basis-N result in all four bases at once, one line each with Copy: the set base marked, BIN
 * in groups of four. The engine sends the other bases as the result's forms.
 */
export function BaseForms(props: {
  result: string;
  base: NumberBase;
  forms: ResultForm[];
  copied: string | null;
  onCopy(text: string, marker: string): void;
}) {
  const { t } = useT();
  return (
    <dl className="base-forms">
      {ORDER.map((base) => {
        const latex = base === props.base ? props.result : props.forms.find((form) => form.kind === base)?.latex;
        if (latex == null) return null;
        const digits = plainDigits(latex);
        const marker = `base-${base}`;
        return (
          <div key={base} className={base === props.base ? 'base-form active' : 'base-form'}>
            <dt>{t(`form.${base}`)}</dt>
            <dd>
              <code>{base === 'bin' ? groupedBinary(digits) : digits.replace('-', '−')}</code>
            </dd>
            <button className="copy-btn" onClick={() => props.onCopy(digits, marker)}>
              {props.copied === marker ? t('copied') : t('copy')}
            </button>
          </div>
        );
      })}
    </dl>
  );
}
