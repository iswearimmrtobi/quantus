import { useEffect, useImperativeHandle, useRef, type Ref } from 'react';

export interface LatexInputHandle {
  focus(): void;
  /** Fügt Text an der Einfügemarke ein — so schreibt das Variablen-Panel `r_1` hinein. */
  insert(latex: string): void;
}

interface Props {
  value: string;
  onChange(latex: string): void;
  onEnter(): void;
  /** Zurück zum Formeleditor (Esc). */
  onLeave(): void;
  /** What the empty field shows; a physics example unless the caller has a better one. */
  placeholder?: string;
  ref?: Ref<LatexInputHandle>;
}

/**
 * Die zweite Eingabeart: LaTeX-Quelltext, so getippt wie geschrieben (F2).
 *
 * Der Formeleditor daneben ist bequem, kann aber nur, was er als Struktur kennt — wer
 * `\frac{1}{2}` oder `\qty{12}{cm}` schreiben will, braucht ein Feld, das den Text
 * unverändert stehen lässt. Genau das ist hier: ein gewöhnliches Textfeld, dessen Inhalt
 * ungefiltert ans Backend geht.
 */
export function LatexInput({ value, onChange, onEnter, onLeave, placeholder = 'F = 2kg \\cdot 10\\frac{m}{s^2}', ref }: Props) {
  const input = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const field = input.current;
    if (!field) return;
    field.focus();
    // Beim Wechsel aus dem Formeleditor steht die Marke am Ende — dort tippt man weiter.
    field.setSelectionRange(field.value.length, field.value.length);
  }, []);

  useImperativeHandle(ref, () => ({
    focus: () => input.current?.focus(),
    insert: (latex: string) => {
      const field = input.current;
      if (!field) return;
      const start = field.selectionStart ?? field.value.length;
      const end = field.selectionEnd ?? start;
      const next = field.value.slice(0, start) + latex + field.value.slice(end);
      onChange(next);
      field.focus();
      // Nach dem Rendern steht der Cursor hinter dem Eingefügten.
      requestAnimationFrame(() => field.setSelectionRange(start + latex.length, start + latex.length));
    },
  }));

  return (
    <textarea
      ref={input}
      className="latex-input"
      value={value}
      spellCheck={false}
      autoComplete="off"
      rows={1}
      placeholder={placeholder}
      onChange={(event) => onChange(event.target.value)}
      onKeyDown={(event) => {
        if (event.key === 'Enter' && !event.shiftKey) {
          event.preventDefault();
          onEnter();
        } else if (event.key === 'Escape') {
          // Nur dieses Feld verlassen; das Escape der App schließt sonst gleich das Panel mit.
          event.preventDefault();
          event.stopPropagation();
          onLeave();
        }
      }}
    />
  );
}
