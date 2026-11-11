// name: 2048
// kind: game
// api: 1
// author: Quantus
// version: 1.0
// about: Slide the tiles and join equal ones.
// about.de: Schiebe die Kacheln und verbinde gleiche.
// license: MIT

// The board is an array of sixteen numbers, row by row, 0 for a free cell. The game is saved
// after every move, so switching off loses nothing.

fn init() {
    let saved = load();
    this.best = 0;
    if saved != () && type_of(saved) == "map" && saved.board != () && saved.board.len() == 16 {
        this.board = saved.board;
        this.score = saved.score;
        this.best = saved.best;
        this.reached = saved.reached;
        this.back = ();
        this.fresh = -1;
        this.over = !moves_left(this.board);
    } else {
        this.start();
    }
    this.labels();
}

fn de() { lang() == "de" }

fn labels() {
    let de = de();
    let back = if this.back == () { "" } else if de { "Zurück" } else { "Undo" };
    soft([if de { "Neu" } else { "New" }, back]);
}

// A new game; the best score stays.
fn start() {
    this.board = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    this.score = 0;
    this.reached = false;
    this.over = false;
    this.back = ();
    this.fresh = -1;
    this.add();
    this.add();
}

// A 2 (nine times in ten) or a 4 on a free cell; it is outlined until the next move.
fn add() {
    let free = [];
    for i in 0..16 {
        if this.board[i] == 0 { free.push(i); }
    }
    if free.is_empty() { return; }
    let i = free[random(free.len())];
    this.board[i] = if random(10) < 9 { 2 } else { 4 };
    this.fresh = i;
}

// The cells of line `k` in the order the tiles slide towards: the first is where they go.
fn cells(dir, k) {
    if dir == "left" { [k * 4, k * 4 + 1, k * 4 + 2, k * 4 + 3] }
    else if dir == "right" { [k * 4 + 3, k * 4 + 2, k * 4 + 1, k * 4] }
    else if dir == "up" { [k, k + 4, k + 8, k + 12] }
    else { [k + 12, k + 8, k + 4, k] }
}

// The numbers of a line slid to its start, equal neighbours joined once: the new line and the
// points the joins give. [2, 2, 2, 2] gives [4, 4, 0, 0] and 8.
fn slide(line) {
    let tiles = [];
    for v in line {
        if v != 0 { tiles.push(v); }
    }
    let out = [];
    let gained = 0;
    let i = 0;
    while i < tiles.len() {
        if i + 1 < tiles.len() && tiles[i] == tiles[i + 1] {
            out.push(tiles[i] * 2);
            gained += tiles[i] * 2;
            i += 2;
        } else {
            out.push(tiles[i]);
            i += 1;
        }
    }
    while out.len() < line.len() { out.push(0); }
    #{ line: out, gained: gained }
}

// The board after a move in `dir`, and the points; the same board if nothing moved.
fn moved(board, dir) {
    let next = board;
    let gained = 0;
    for k in 0..4 {
        let at = cells(dir, k);
        let line = [];
        for i in at { line.push(board[i]); }
        let result = slide(line);
        for j in 0..4 { next[at[j]] = result.line[j]; }
        gained += result.gained;
    }
    #{ board: next, gained: gained }
}

fn moves_left(board) {
    for dir in ["left", "right", "up", "down"] {
        if moved(board, dir).board != board { return true; }
    }
    false
}

fn turn(dir) {
    if this.over { return; }
    let result = moved(this.board, dir);
    // A move that changes nothing is ignored, and adds no tile.
    if result.board == this.board { return; }
    this.back = #{ board: this.board, score: this.score };
    this.board = result.board;
    this.score += result.gained;
    if this.score > this.best { this.best = this.score; }
    this.add();
    if !this.reached && this.board.contains(2048) { this.reached = true; this.just_reached = true; }
    this.over = !moves_left(this.board);
    this.keep();
    this.labels();
}

fn keep() {
    save(#{ board: this.board, score: this.score, best: this.best, reached: this.reached });
}

fn key(k) {
    this.just_reached = false;
    if k == "left" || k == "4" { this.turn("left"); }
    else if k == "right" || k == "6" { this.turn("right"); }
    else if k == "up" || k == "8" { this.turn("up"); }
    else if k == "down" || k == "2" { this.turn("down"); }
    else if k == "soft1" {
        // Asks first while a game is going on.
        let running = this.score > 0 && !this.over;
        let question = if de() { "Neues Spiel?" } else { "New game?" };
        if !running || confirm(question) {
            this.start();
            this.keep();
            this.labels();
        }
    } else if k == "soft2" && this.back != () {
        // One move back, once.
        this.board = this.back.board;
        this.score = this.back.score;
        this.back = ();
        this.fresh = -1;
        this.over = false;
        this.keep();
        this.labels();
    }
}

fn leave() { this.keep(); }

// The tile palette by the tile's power of two.
fn colour(v) {
    let n = 0;
    while v > 1 { v /= 2; n += 1; }
    `tile${(n - 1) % 14 + 1}`
}

fn draw() {
    clear("background");
    let g = grid(4, 4);
    let gap = g.cell / 20 + 2;
    round(g.x, g.y, g.w, g.h, 8, "surface");
    for i in 0..16 {
        let c = g.cell(i % 4, i / 4);
        let x = c.x + gap;
        let y = c.y + gap;
        let w = c.w - 2 * gap;
        let h = c.h - 2 * gap;
        let v = this.board[i];
        if v == 0 {
            round(x, y, w, h, 6, "hairline");
            continue;
        }
        round(x, y, w, h, 6, colour(v));
        if i == this.fresh { frame(x, y, w, h, 6, "text"); }
        let s = `${v}`;
        // Four digits and more in the small font.
        if s.len() >= 4 {
            text_small(x + (w - text_small_width(s)) / 2, y + (h - text_small_height()) / 2, s, "text");
        } else {
            text(x + (w - text_width(s)) / 2, y + (h - text_height()) / 2, s, "text");
        }
    }
    this.side_panel();
}

fn side_panel() {
    let s = side();
    let de = de();
    let room = s.w - 8;
    let x = s.x + 4;
    let y = s.y + 4;
    let small = text_small_height() + 4;
    let points = if de { "Punkte" } else { "Score" };
    let best = if de { "Bestwert" } else { "Best" };
    let wide = s.h >= 2 * (text_height() + small) + 8;
    if wide {
        // Beside the board: each number under its name.
        text_small(x, y, points, "muted");
        y += small;
        text(x, y, `${this.score}`, "text");
        y += text_height() + 6;
        text_small(x, y, best, "muted");
        y += small;
        text(x, y, `${this.best}`, "text");
        y += text_height() + 10;
    } else {
        // Below it: one line.
        let line = `${points} ${this.score}   ${best} ${this.best}`;
        if text_small_width(line) <= room { text_small(x, y, line, "text"); }
        y += small + 2;
    }
    let status = if this.over {
        if de { "Keine Züge mehr" } else { "No moves left" }
    } else if this.just_reached == true {
        if de { "2048 erreicht!" } else { "2048 reached!" }
    } else { "" };
    if status != "" && text_small_width(status) <= room && y + small <= s.y + s.h {
        text_small(x, y, status, "amber");
    }
}
