import { describe, expect, it } from 'vitest';
import { circuitKit, LabelText } from '../circuit/kit';
import type { EditorElement } from '../circuit/model';
import { newPart } from '../logic/circuit';
import { staticPicture } from '../logic/kit';
import { placeLabels, type CanvasLabel } from './labels';
import { schematicPicture } from './picture';

describe('pictures', () => {
  it('draws a circuit with its labels as plain SVG', () => {
    const elements: EditorElement[] = [
      { id: 'R1', kind: 'resistor', x: 0, y: 0, rotation: 0 },
      { id: 'W1', kind: 'wire', x: 40, y: 0, x2: 80, y2: 0, rotation: 0 },
      { id: 'A', kind: 'terminal', x: 80, y: 0, rotation: 0 },
    ];
    const label: CanvasLabel = { id: 'R1', anchor: { x: 0, y: 0 }, sides: ['below'], gap: 18, className: 'part-label', content: 'R1', svg: <LabelText id="R1" lines={['U = 5 V']} /> };
    const kit = circuitKit(['resistor', 'wire', 'terminal']);
    const placements = placeLabels([{ ...label, w: 40, h: 30 }], elements.flatMap(kit.obstacles));
    const { svg, width, height } = schematicPicture(kit, elements, [label], placements);
    expect(svg).toMatch(/^<svg xmlns="http:\/\/www.w3.org\/2000\/svg"/);
    expect(svg).toContain('<line class="wire" x1="40" y1="0" x2="80" y2="0"');
    expect(svg).toContain('U = 5 V');
    expect(svg).toContain('class="terminal-name"');
    expect(svg).toContain('.terminal-name');
    expect(width).toBeGreaterThan(80);
    expect(height).toBeGreaterThan(30);
  });

  it('draws a Logic circuit without an editor', () => {
    const a = newPart([], 'input', { x: 0, y: 0 });
    const gate = newPart([a], 'not', { x: 80, y: 0 });
    const { svg } = staticPicture([a, gate], 'iec', 'export');
    expect(svg).toContain('class="name"');
    expect(svg).toContain('G1');
    expect(svg).toContain('fill="#ffffff"');
  });
});
