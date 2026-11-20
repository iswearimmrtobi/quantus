/**
 * The names of the analysis view's functions. Each field holds one function; one typed with a
 * prefix (`g(x) = …`) keeps its name, the others get the next free letter, so every function
 * can be sent to the engine as a definition the others may use (`f(x - 1)`, `g'(x)`).
 */

/** Letters handed out in this order; e is Euler's number, i the imaginary unit. */
const LETTERS = ['f', 'g', 'h', 'k', 'p', 'q', 'r', 's', 'u', 'v', 'w'];

/** The name a function was typed with, `g` of `g(x) = …` or `f_{1}` of `f_{1}\left(t\right)=`. */
export function typedName(latex: string): string | null {
  const match = /^\s*([a-zA-Z](?:_\{[^{}]*\}|_[a-zA-Z0-9])?)\s*(?:\\left)?\(\s*[a-zA-Z]\s*(?:\\right)?\)\s*=/.exec(latex);
  return match ? match[1] : null;
}

/** A name for each function: the typed one, or the next letter no other function uses. */
export function assignNames(inputs: string[]): string[] {
  const typed = inputs.map(typedName);
  const taken = new Set(typed.filter((name): name is string => name !== null));
  const free = LETTERS.filter((letter) => !taken.has(letter));
  return typed.map((name) => name ?? free.shift() ?? `f_{${inputs.length}}`);
}

/** The function as a definition: with its prefix, `name(x) = …` added where it has none. */
export function asDefinition(latex: string, name: string): string {
  if (typedName(latex) !== null) return latex;
  const body = latex.replace(/^\s*y\s*=/, '');
  return `${name}(x) = ${body}`;
}
