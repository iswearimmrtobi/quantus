import { Katex } from './Katex';

/** A key of the bar: what it shows and the LaTeX it puts in at the cursor. */
export interface InsertKey {
  label: string;
  latex: string;
  /** Show the label as typeset maths rather than as text. */
  math?: boolean;
}

/** A row of keys above the input for what a mode needs and a keyboard lacks (Basis-N, Prüfen). */
export function InsertBar(props: { keys: InsertKey[]; onInsert: (latex: string) => void }) {
  return (
    <div className="insert-bar">
      {props.keys.map((key) => (
        <button key={key.label} className="insert-key" onClick={() => props.onInsert(key.latex)}>
          {key.math ? <Katex latex={key.label} /> : key.label}
        </button>
      ))}
    </div>
  );
}
