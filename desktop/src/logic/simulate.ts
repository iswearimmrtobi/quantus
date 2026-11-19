import type { LogicElement, Netlist } from '../api';

/**
 * Each net's value when the switches are set as `inputs` says (by name): what the Gates card's
 * picture colours. A net that a loop or an open pin leaves undecided has no value.
 */
export function simulate(netlist: Netlist, inputs: Map<string, boolean>): Map<number, boolean> {
  const driver = new Map<number, LogicElement>();
  for (const element of netlist.elements) if (element.output != null) driver.set(element.output, element);
  const values = new Map<number, boolean>();
  const busy = new Set<number>();
  const value = (net: number | null | undefined): boolean | null => {
    if (net == null) return null;
    const known = values.get(net);
    if (known !== undefined) return known;
    const element = driver.get(net);
    if (!element || busy.has(net)) return null;
    busy.add(net);
    const result = compute(element);
    busy.delete(net);
    if (result !== null) values.set(net, result);
    return result;
  };
  const compute = (element: LogicElement): boolean | null => {
    switch (element.kind) {
      case 'input':
        return inputs.get(element.name) ?? null;
      case 'constant':
        return element.value;
      case 'gate': {
        const ins = (element.inputs ?? []).map(value);
        if (ins.some((v) => v === null)) return null;
        const bits = ins as boolean[];
        const and = bits.every(Boolean);
        const or = bits.some(Boolean);
        const xor = bits.filter(Boolean).length % 2 === 1;
        switch (element.gate) {
          case 'and':
            return and;
          case 'nand':
            return !and;
          case 'or':
            return or;
          case 'nor':
            return !or;
          case 'xor':
            return xor;
          case 'xnor':
            return !xor;
          case 'not':
            return !bits[0];
          case 'buffer':
            return bits[0];
        }
        return null;
      }
      default:
        return null;
    }
  };
  for (const element of netlist.elements) {
    if (element.output != null) value(element.output);
    for (const net of element.inputs ?? []) value(net);
  }
  return values;
}

/** The switches' values in row `row` of the table: the first input is the most significant bit. */
export function rowInputs(names: string[], row: number): Map<string, boolean> {
  return new Map(names.map((name, i) => [name, ((row >> (names.length - 1 - i)) & 1) === 1]));
}
