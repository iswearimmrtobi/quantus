import { useEffect, useImperativeHandle, useRef, type Ref } from 'react';
import { MathfieldElement } from 'mathlive';

// MathLive fetches its fonts by URL; the copy in public/fonts is what it finds (see
// scripts/copy-mathlive-fonts.mjs). Sounds are off — this is a calculator, not a game.
MathfieldElement.fontsDirectory = './fonts';
MathfieldElement.soundsDirectory = null;

export interface MathFieldHandle {
  focus(): void;
  setValue(latex: string): void;
  /** Puts LaTeX in at the cursor — how the variables panel writes `r_1` into the input. */
  insert(latex: string): void;
}

interface Props {
  value: string;
  onChange(latex: string): void;
  onEnter(): void;
  ref?: Ref<MathFieldHandle>;
}

/**
 * The MathLive editor (N2): LaTeX in the model, live typesetting on screen.
 *
 * The element is created imperatively rather than as JSX — it is a custom element with a
 * property-based API, and React would only ever set attributes.
 */
export function MathField({ value, onChange, onEnter, ref }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const field = useRef<MathfieldElement | null>(null);
  const callbacks = useRef({ onChange, onEnter });
  callbacks.current = { onChange, onEnter };

  useEffect(() => {
    const mathfield = new MathfieldElement();
    mathfield.className = 'math-input';
    // No virtual keyboard: the desktop workflow is physical keys and the numpad (F10).
    mathfield.mathVirtualKeyboardPolicy = 'manual';
    mathfield.smartMode = false;
    mathfield.value = value;

    const onInput = () => callbacks.current.onChange(mathfield.value);
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Enter') {
        event.preventDefault();
        callbacks.current.onEnter();
      }
    };
    mathfield.addEventListener('input', onInput);
    mathfield.addEventListener('keydown', onKeyDown);

    host.current?.appendChild(mathfield);
    field.current = mathfield;
    mathfield.focus();

    return () => {
      mathfield.removeEventListener('input', onInput);
      mathfield.removeEventListener('keydown', onKeyDown);
      mathfield.remove();
      field.current = null;
    };
    // Only on mount: the field owns its content from here on, `value` syncs below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // The input can also change from the outside — history, "new calculation", shortcuts.
  useEffect(() => {
    const mathfield = field.current;
    if (mathfield && mathfield.value !== value) mathfield.value = value;
  }, [value]);

  useImperativeHandle(ref, () => ({
    focus: () => field.current?.focus(),
    setValue: (latex: string) => {
      const mathfield = field.current;
      if (!mathfield) return;
      mathfield.value = latex;
      mathfield.focus();
      callbacks.current.onChange(mathfield.value);
    },
    insert: (latex: string) => {
      const mathfield = field.current;
      if (!mathfield) return;
      mathfield.insert(latex, { focus: true });
      callbacks.current.onChange(mathfield.value);
    },
  }));

  return <div className="math-host" ref={host} />;
}
