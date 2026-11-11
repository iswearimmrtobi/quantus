// name: Tic tac toe
// name.de: Tic Tac Toe
// kind: game
// api: 1
// author: Quantus
// version: 1.0
// about: Three in a row, against the calculator or a second player.
// about.de: Drei in einer Reihe, gegen den Rechner oder zu zweit.
// license: MIT

// The board is an array of nine cells, row by row: "", "X" or "O". X is the player, O the
// computer or the second player.

fn init() {
    this.board = ["", "", "", "", "", "", "", "", ""];
    this.starter = "X";
    this.turn = "X";
    // The mode, the level and the score as they were left.
    let saved = load();
    let kept = saved != () && type_of(saved) == "map";
    this.computer = if kept && type_of(saved.computer) == "bool" { saved.computer } else { true };
    this.hard = if kept && type_of(saved.hard) == "bool" { saved.hard } else { false };
    this.score = if kept && type_of(saved.score) == "map" { saved.score } else { #{ x: 0, o: 0, draw: 0 } };
    this.over = false;
    this.line = ();
    input("grid", #{ cols: 3, rows: 3, digits: true });
    this.labels();
}

fn de() { lang() == "de" }

fn keep() {
    save(#{ computer: this.computer, hard: this.hard, score: this.score });
}

fn leave() { this.keep(); }

// The soft keys: Neu, the mode, the level (only against the computer), each showing what is set.
fn labels() {
    let de = de();
    let mode = if this.computer { if de { "Rechner" } else { "CPU" } } else if de { "2 Spieler" } else { "2 players" };
    let level = if !this.computer { "" } else if this.hard {
        if de { "schwer" } else { "hard" }
    } else {
        if de { "leicht" } else { "easy" }
    };
    soft([if de { "Neu" } else { "New" }, mode, level]);
}

// The three cells of each line.
fn lines() {
    [[0, 1, 2], [3, 4, 5], [6, 7, 8], [0, 3, 6], [1, 4, 7], [2, 5, 8], [0, 4, 8], [2, 4, 6]]
}

// The line `board` has three of a kind in, or ().
fn won(board) {
    for line in lines() {
        let a = board[line[0]];
        if a != "" && a == board[line[1]] && a == board[line[2]] {
            return line;
        }
    }
    ()
}

fn full(board) {
    for cell in board {
        if cell == "" { return false; }
    }
    true
}

fn free(board) {
    let cells = [];
    for i in 0..9 {
        if board[i] == "" { cells.push(i); }
    }
    cells
}

fn other(mark) { if mark == "X" { "O" } else { "X" } }

// A new round, `first` beginning.
fn new_round(first) {
    this.board = ["", "", "", "", "", "", "", "", ""];
    this.starter = first;
    this.turn = first;
    this.over = false;
    this.line = ();
    this.cpu();
}

// Sets the mark of whose turn it is on cell `i`, and sees whether the round is over.
fn play(i) {
    if this.over || this.board[i] != "" { return; }
    this.board[i] = this.turn;
    let line = won(this.board);
    if line != () {
        this.over = true;
        this.line = line;
        if this.turn == "X" { this.score.x += 1; } else { this.score.o += 1; }
        this.keep();
        return;
    }
    if full(this.board) {
        this.over = true;
        this.score.draw += 1;
        this.keep();
        return;
    }
    this.turn = other(this.turn);
}

// The computer's move, when it is its turn: a free cell at random, or on *schwer* the best.
fn cpu() {
    if !this.computer || this.over || this.turn != "O" { return; }
    let cells = free(this.board);
    if this.hard {
        this.play(best(this.board, "O"));
    } else {
        this.play(cells[random(cells.len())]);
    }
}

// The free cells, the centre first, then the corners.
fn ordered(board) {
    let cells = [];
    for i in [4, 0, 2, 6, 8, 1, 3, 5, 7] {
        if board[i] == "" { cells.push(i); }
    }
    cells
}

// The free cell of a line with two of `m` in it, or -1.
fn open_end(b, line, m) {
    let a = b[line[0]];
    let c = b[line[1]];
    let d = b[line[2]];
    if a == m && c == m && d == "" { return line[2]; }
    if a == m && d == m && c == "" { return line[1]; }
    if c == m && d == m && a == "" { return line[0]; }
    -1
}

// A free cell that gives `m` a line, or -1.
fn completing(b, m) {
    for line in lines() {
        let cell = open_end(b, line, m);
        if cell >= 0 { return cell; }
    }
    -1
}

// How many lines `m` could complete with one more mark.
fn threats(b, m) {
    let n = 0;
    for line in lines() {
        if open_end(b, line, m) >= 0 { n += 1; }
    }
    n
}

// The free cells where a mark of `m` makes two threats at once.
fn forks(b, m) {
    let cells = [];
    for i in ordered(b) {
        b[i] = m;
        if threats(b, m) >= 2 { cells.push(i); }
        b[i] = "";
    }
    cells
}

// The best cell for `me`, by the rules that never lose: win, block, fork, block a fork, the
// centre, the corner opposite the other's, a corner, a side. A search of the game tree takes
// seconds on the handheld; the rules take a few thousand operations.
fn best(board, me) {
    let you = other(me);
    let cell = completing(board, me);
    if cell >= 0 { return cell; }
    cell = completing(board, you);
    if cell >= 0 { return cell; }
    let mine = forks(board, me);
    if !mine.is_empty() { return mine[0]; }
    let theirs = forks(board, you);
    if theirs.len() == 1 { return theirs[0]; }
    if theirs.len() > 1 {
        // A threat of my own, so that the other must block, where blocking makes no fork.
        for i in ordered(board) {
            board[i] = me;
            let block = completing(board, me);
            let good = false;
            if block >= 0 {
                board[block] = you;
                good = threats(board, you) < 2;
                board[block] = "";
            }
            board[i] = "";
            if good { return i; }
        }
    }
    if board[4] == "" { return 4; }
    for pair in [[0, 8], [8, 0], [2, 6], [6, 2]] {
        if board[pair[0]] == you && board[pair[1]] == "" { return pair[1]; }
    }
    ordered(board)[0]
}

fn pick(col, row) {
    // Any key after a round starts the next one, the other side beginning.
    if this.over {
        this.new_round(other(this.starter));
        return;
    }
    if this.computer && this.turn == "O" { return; }
    this.play(row * 3 + col);
    this.cpu();
}

fn key(k) {
    if k == "soft1" {
        this.score = #{ x: 0, o: 0, draw: 0 };
        this.new_round("X");
        this.keep();
    } else if k == "soft2" {
        this.computer = !this.computer;
        this.labels();
        this.keep();
        this.cpu();
    } else if k == "soft3" {
        this.hard = !this.hard;
        this.labels();
        this.keep();
    } else if this.over {
        this.new_round(other(this.starter));
    }
}

fn draw() {
    clear("background");
    let g = grid(3, 3);
    let gap = g.cell / 16 + 2;
    for row in 0..3 {
        for col in 0..3 {
            let c = g.cell(col, row);
            let mark = this.board[row * 3 + col];
            round(c.x + gap, c.y + gap, c.w - 2 * gap, c.h - 2 * gap, 8, "surface");
            let m = c.w / 4;
            if mark == "X" {
                line(c.x + m, c.y + m, c.x + c.w - m, c.y + c.h - m, "cyan", 6);
                line(c.x + c.w - m, c.y + m, c.x + m, c.y + c.h - m, "cyan", 6);
            } else if mark == "O" {
                circle(c.x + m, c.y + m, c.w - 2 * m, "amber");
                circle(c.x + m + 6, c.y + m + 6, c.w - 2 * m - 12, "surface");
            }
        }
    }
    if this.line != () {
        let a = g.cell(this.line[0] % 3, this.line[0] / 3);
        let b = g.cell(this.line[2] % 3, this.line[2] / 3);
        line(a.x + a.w / 2, a.y + a.h / 2, b.x + b.w / 2, b.y + b.h / 2, "text", 6);
    }
    this.side_panel();
}

// Whose turn it is, or how the round ended, and the score.
fn side_panel() {
    let s = side();
    let de = de();
    let status = if this.over {
        if this.line == () {
            if de { "Unentschieden" } else { "A draw" }
        } else {
            let winner = this.board[this.line[0]];
            if this.computer && winner == "O" {
                if de { "Der Rechner gewinnt" } else { "The CPU wins" }
            } else if de { `${winner} gewinnt` } else { `${winner} wins` }
        }
    } else if de { `Am Zug: ${this.turn}` } else { `To move: ${this.turn}` };
    let step = text_small_height() + 6;
    let room = s.w - 8;
    let y = s.y + 4;
    let color = if this.over { "amber" } else { "text" };
    // The status in the large font where it fits, else the small one.
    if text_width(status) <= room {
        text(s.x + 4, y, status, color);
        y += text_height() + 6;
    } else {
        text_small(s.x + 4, y, status, color);
        y += step;
    }
    let draws = if de { "Remis" } else { "Draws" };
    let score = `X ${this.score.x}   O ${this.score.o}   ${draws} ${this.score.draw}`;
    if text_small_width(score) <= room {
        text_small(s.x + 4, y, score, "muted");
    } else {
        text_small(s.x + 4, y, `${this.score.x} : ${this.score.o}`, "muted");
    }
    y += step;
    let hint = if this.over {
        if de { "Taste: nächste Runde" } else { "Any key: next round" }
    } else if de { "Pfeile und OK, oder 1-9" } else { "Arrows and OK, or 1-9" };
    if y + step <= s.y + s.h && text_small_width(hint) <= room {
        text_small(s.x + 4, y, hint, "muted");
    }
}
