// name: Counter
// name.de: Zähler
// kind: tool
// api: 1
// author: Quantus
// version: 1.0
// about: Counts up and down with the arrows.
// about.de: Zählt mit den Pfeilen auf und ab.
// license: MIT

fn init() { this.n = 0; }

fn key(k) {
    if k == "up"   { this.n += 1; }
    if k == "down" { this.n -= 1; }
}

fn draw() {
    clear("background");
    let label = if lang() == "de" { "Zähler" } else { "Count" };
    text(20, 40, `${label}: ${this.n}`, "text");
}
