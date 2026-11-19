import type { LogicElement, LogicNotation, Netlist } from '../api';
import { nameLatex } from '../logic';

/**
 * A circuit gate by gate, with a named signal per gate: `G_1 = \overline{A \land B}`, …, and
 * `f = G_4` for each lamp. Easier to read than a NAND or NOR formula with its overlines stacked
 * three deep, and it is the circuit as drawn. Gates are numbered from the inputs on, in the order
 * their signals are needed. One `aligned` block.
 */
export function gateByGate(netlist: Netlist, notation: LogicNotation): string {
  const technical = notation === 'technical';
  const and = technical ? ' \\cdot ' : ' \\land ';
  const or = technical ? ' + ' : ' \\lor ';
  const driver = new Map<number, LogicElement>();
  for (const element of netlist.elements) if (element.output != null) driver.set(element.output, element);

  const names = new Map<string, string>();
  const lines: string[] = [];
  const signal = (net: number | null | undefined): string => {
    const element = net == null ? undefined : driver.get(net);
    if (!element) return '?';
    switch (element.kind) {
      case 'input':
        return nameLatex(element.name);
      case 'constant':
        return element.value ? '1' : '0';
      case 'gate':
        return gate(element);
      default:
        return '?';
    }
  };
  const gate = (element: LogicElement & { kind: 'gate' }): string => {
    const known = names.get(element.id);
    if (known) return known;
    // Named before its inputs are, so that a loop cannot recurse for ever; renamed below.
    names.set(element.id, '?');
    const operands = (element.inputs ?? []).map(signal);
    const joined = (op: string) => operands.join(op);
    const formula = (() => {
      switch (element.gate) {
        case 'not':
          return `\\overline{${operands[0] ?? '?'}}`;
        case 'buffer':
          return operands[0] ?? '?';
        case 'and':
          return joined(and);
        case 'or':
          return joined(or);
        case 'nand':
          return `\\overline{${joined(and)}}`;
        case 'nor':
          return `\\overline{${joined(or)}}`;
        case 'xor':
          return joined(' \\oplus ');
        case 'xnor':
          return `\\overline{${joined(' \\oplus ')}}`;
      }
    })();
    const name = `G_{${lines.length + 1}}`;
    names.set(element.id, name);
    lines.push(`${name} &= ${formula}`);
    return name;
  };

  const lamps = netlist.elements.filter((e): e is LogicElement & { kind: 'output' } => e.kind === 'output');
  const outputs = lamps.map((lamp) => `${nameLatex(lamp.name)} &= ${signal(lamp.inputs?.[0])}`);
  return `\\begin{aligned} ${[...lines, ...outputs].join(' \\\\ ')} \\end{aligned}`;
}
