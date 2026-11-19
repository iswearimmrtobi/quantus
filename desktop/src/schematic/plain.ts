/**
 * A result's LaTeX as plain text, for the labels of an exported picture, which has no KaTeX:
 * `33{,}75\,\mathrm{V}` is `33,75 V`, `4{,}7\cdot 10^{-3}\,\mathrm{A}` is `4,7·10⁻³ A`.
 */

const SUPERSCRIPT: Record<string, string> = {
  '0': '⁰', '1': '¹', '2': '²', '3': '³', '4': '⁴', '5': '⁵', '6': '⁶', '7': '⁷', '8': '⁸', '9': '⁹', '-': '⁻', '−': '⁻', '+': '⁺',
};

const SYMBOLS: Record<string, string> = {
  Omega: 'Ω', mu: 'µ', cdot: '·', times: '×', pi: 'π', varphi: 'φ', phi: 'φ', infty: '∞', degree: '°', circ: '°', approx: '≈', pm: '±',
};

const raised = (text: string) => [...text].map((c) => SUPERSCRIPT[c] ?? c).join('');

export function plainOf(latex: string): string {
  let text = latex
    .replace(/\{,\}/g, ',')
    .replace(/\\(?:left|right)/g, '')
    .replace(/\\[,;:! ]/g, ' ')
    .replace(/\\%/g, '%');
  // Innermost first, so that nested braces come apart.
  for (let i = 0; i < 4; i += 1) {
    text = text
      .replace(/\\(?:mathrm|text|operatorname|mathit|mathbf)\{([^{}]*)\}/g, '$1')
      .replace(/\\[dt]?frac\{([^{}]*)\}\{([^{}]*)\}/g, '$1/$2')
      .replace(/\^\{([^{}]*)\}/g, (_, power: string) => raised(power))
      .replace(/_\{([^{}]*)\}/g, '$1');
  }
  return text
    .replace(/\^\\circ/g, '°')
    .replace(/\^(\S)/g, (_, power: string) => raised(power))
    .replace(/\\([A-Za-z]+)/g, (_, name: string) => SYMBOLS[name] ?? name)
    .replace(/[{}]/g, '')
    .replace(/\s*·\s*/g, '·')
    .replace(/\s+/g, ' ')
    .trim();
}
