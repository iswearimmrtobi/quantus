import { useEffect, useImperativeHandle, useRef, type Ref } from 'react';
import { MathfieldElement } from 'mathlive';

// MathLive fetches its fonts by URL; the copy in public/fonts is what it finds (see
// scripts/copy-mathlive-fonts.mjs). Sounds are off — this is a calculator, not a game.
MathfieldElement.fontsDirectory = './fonts';
MathfieldElement.soundsDirectory = null;
// Deutsche Eingabe: das Komma ist ein Dezimaltrennzeichen, kein eigenes Zeichen. Sonst gilt
// `9,81` als zwei Zahlen — und `/` macht daraus einen Bruch über die 81 statt über 9,81.
MathfieldElement.decimalSeparator = ',';

/**
 * Befehle, die ohne ihre Argumente nichts anzeigen — mit den Skeletten, die sie stattdessen
 * bekommen. `#?` ist MathLives Platzhalter, zwischen denen Tab weiterspringt.
 */
const SKELETONS: Record<string, string> = {
  '\\frac': '\\frac{#?}{#?}',
  '\\dfrac': '\\frac{#?}{#?}',
  '\\tfrac': '\\frac{#?}{#?}',
  '\\sqrt': '\\sqrt{#?}',
  '\\mathrm': '\\mathrm{#?}',
  '\\text': '\\text{#?}',
  '\\operatorname': '\\operatorname{#?}',
  '\\qty': '\\qty{#?}{#?}',
};

/**
 * Ersetzt einen gerade abgeschlossenen, aber argumentlosen Befehl durch sein Skelett.
 *
 * Wer `\frac` tippt und Enter drückt, bekommt von MathLive ein nacktes `\frac` ins Feld —
 * ohne Klammern, ohne Platzhalter. Die nächsten Ziffern landen dann daneben statt darin, und
 * aus `\frac` + `1` + `2` wird `\frac12`. Hier wird daraus ein echter Bruch mit zwei
 * Platzhaltern, in dem der Cursor schon im Zähler steht.
 *
 * Ergänzt wird nur ein Befehl am Ende der Eingabe. Steht der Cursor mitten im Ausdruck, greift
 * die Regel bewusst nicht: Der abgeschnittene Text säße dann an der falschen Stelle, und ein
 * unvollständiger Befehl ist immer noch besser als eine zerschossene Formel.
 */
function completeCommand(mathfield: MathfieldElement): void {
  if (mathfield.mode !== 'math') return;

  const value = mathfield.value;
  const command = Object.keys(SKELETONS).find(
    (name) => value.endsWith(name) && !value.endsWith(`${name}{`),
  );
  if (!command) return;

  mathfield.value = value.slice(0, -command.length);
  mathfield.executeCommand(['moveToMathfieldEnd']);
  mathfield.insert(SKELETONS[command], { focus: true, selectionMode: 'placeholder' });
}

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
  /** Der Nutzer hat `\` getippt und will LaTeX schreiben — [seed] ist der Text für das Quelltextfeld. */
  onLatexRequested(seed: string): void;
  /** LaTeX the empty editor shows greyed out. */
  placeholder?: string;
  ref?: Ref<MathFieldHandle>;
}

/**
 * Was der Editor intern führt, in der Schreibweise, in der man es von Hand tippen würde.
 *
 * MathLive verpackt jeden mehrbuchstabigen Bezeichner in `\operatorname{}` — als Struktur
 * richtig, zum Weiterschreiben aber sperrig. Fürs Quelltextfeld wird daraus wieder das
 * `\mathrm{}`, das im Rest der Anwendung ohnehin die Einheiten-Auszeichnung ist. Für das
 * Backend sind beide Formen dasselbe, es ändert sich also nur, was der Nutzer liest.
 */
export function toReadableLatex(latex: string): string {
  return latex
    .replace(/\\operatorname\{\\mathrm\{([^{}]*)\}\}/g, '\\mathrm{$1}')
    .replace(/\\operatorname\{([^{}]*)\}/g, '\\mathrm{$1}');
}

/**
 * The MathLive editor (N2): LaTeX in the model, live typesetting on screen.
 *
 * The element is created imperatively rather than as JSX — it is a custom element with a
 * property-based API, and React would only ever set attributes.
 */
export function MathField({ value, onChange, onEnter, onLatexRequested, placeholder, ref }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const field = useRef<MathfieldElement | null>(null);
  const callbacks = useRef({ onChange, onEnter, onLatexRequested });
  callbacks.current = { onChange, onEnter, onLatexRequested };
  /** Der zuletzt aus dem Feld gemeldete Wert — alles andere kommt von außen. */
  const reported = useRef(value);

  useEffect(() => {
    const mathfield = new MathfieldElement();
    mathfield.className = 'math-input';
    // No virtual keyboard: the desktop workflow is physical keys and the numpad (F10).
    mathfield.mathVirtualKeyboardPolicy = 'manual';
    mathfield.smartMode = false;
    if (placeholder) mathfield.placeholder = placeholder;
    mathfield.value = value;

    // Beim Verlassen des Latex-Modus meldet `mode-change` den neuen Modus, trägt den
    // fertigen Befehl aber noch nicht im Wert — der kommt erst mit dem nächsten `input`.
    // Genau dort muss die Ergänzung ansetzen.
    let commandJustCommitted = false;

    const onInput = () => {
      if (commandJustCommitted) {
        commandJustCommitted = false;
        completeCommand(mathfield);
      }
      reported.current = mathfield.value;
      callbacks.current.onChange(mathfield.value);
    };

    const onKeyDown = (event: KeyboardEvent) => {
      // Der Backslash ist die Tür zum Quelltext (F2). MathLive hätte hier seinen eigenen
      // Latex-Modus angeboten, in dem aber nur der Befehlsname steht und nicht die Formel —
      // Argumente, Klammern und alles Verschachtelte gehen dort nicht von Hand zu tippen.
      if (event.key === '\\' && mathfield.mode !== 'latex') {
        // stopPropagation, nicht nur preventDefault: MathLive verarbeitet die Taste im
        // Inneren des Elements und würde sonst trotzdem seinen eigenen Latex-Modus öffnen.
        event.preventDefault();
        event.stopPropagation();
        callbacks.current.onLatexRequested(`${toReadableLatex(mathfield.value)}\\`);
        return;
      }

      if (event.key !== 'Enter') return;
      // Beim Tippen eines Befehls (`\frac`) steht das Feld im Latex-Modus, und Enter ist dort
      // die Taste, die den Befehl abschließt. Nähme die App sie an sich, käme der Nutzer aus
      // dem Modus nie heraus — genau so ist aus `\frac{1}{2}` ein `\frac12` geworden.
      //
      // Nur dieser eine Modus ist gemeint: in einer Einheit steht der Cursor im Textmodus
      // (`\operatorname{kg}`), und dort muss Enter ganz normal rechnen.
      if (mathfield.mode === 'latex') return;
      event.preventDefault();
      event.stopPropagation();
      callbacks.current.onEnter();
    };

    // Der Latex-Modus meldet als Wert einen leeren String, solange der Befehl noch offen ist.
    // Ohne dieses Nachfassen bliebe die App auf diesem leeren Stand stehen und würde beim
    // nächsten Enter nichts oder Veraltetes auswerten.
    const onModeChange = () => {
      commandJustCommitted = mathfield.mode === 'math';
      callbacks.current.onChange(mathfield.value);
    };

    mathfield.addEventListener('input', onInput);
    // Capture-Phase: MathLive verarbeitet Tasten in seinem Inneren und ist sonst zuerst dran.
    mathfield.addEventListener('keydown', onKeyDown, true);
    mathfield.addEventListener('mode-change', onModeChange);

    host.current?.appendChild(mathfield);
    field.current = mathfield;
    mathfield.focus();

    return () => {
      mathfield.removeEventListener('input', onInput);
      mathfield.removeEventListener('keydown', onKeyDown, true);
      mathfield.removeEventListener('mode-change', onModeChange);
      mathfield.remove();
      field.current = null;
    };
    // Only on mount: the field owns its content from here on, `value` syncs below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /**
   * Änderungen von außen — Verlauf, neue Rechnung, eingefügte Variablen — ins Feld schreiben.
   *
   * Was aus dem Feld selbst kam, wird hier bewusst nicht zurückgeschrieben: Der Zustand von
   * React hinkt beim Tippen einen Rendervorgang hinterher, und ein Rückschreiben würde
   * mitten im Wort den vorherigen Stand wiederherstellen und den Cursor ans Ende werfen.
   */
  useEffect(() => {
    const mathfield = field.current;
    if (!mathfield || value === reported.current) return;
    if (mathfield.value !== value) mathfield.value = value;
    reported.current = value;
  }, [value]);

  useImperativeHandle(ref, () => ({
    focus: () => field.current?.focus(),
    setValue: (latex: string) => {
      const mathfield = field.current;
      if (!mathfield) return;
      mathfield.value = latex;
      mathfield.focus();
      reported.current = mathfield.value;
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
