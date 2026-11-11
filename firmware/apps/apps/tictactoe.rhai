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
    this.computer = true;
    this.hard = false;
    this.score = #{ x: 0, o: 0, draw: 0 };
    this.over = false;
    this.line = ();
    input("grid", #{ cols: 3, rows: 3, digits: true });
    this.labels();
}

fn de() { lang() == "de" }

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
        return;
    }
    if full(this.board) {
        this.over = true;
        this.score.draw += 1;
        return;
    }
    this.turn = other(this.turn);
}

// The computer's move, when it is its turn.
fn cpu() {
    if !this.computer || this.over || this.turn != "O" { return; }
    let cells = free(this.board);
    this.play(cells[random(cells.len())]);
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
    } else if k == "soft2" {
        this.computer = !this.computer;
        this.labels();
        this.cpu();
    } else if k == "soft3" {
        this.hard = !this.hard;
        this.labels();
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
    let y = s.y + 4;
    text(s.x + 4, y, status, if this.over { "amber" } else { "text" });
    y += text_height() + 6;
    let draws = if de { "Remis" } else { "Draws" };
    text_small(s.x + 4, y, `X ${this.score.x}   O ${this.score.o}   ${draws} ${this.score.draw}`, "muted");
    y += step;
    if y + step <= s.y + s.h {
        let hint = if this.over {
            if de { "Eine Taste: nächste Runde" } else { "Any key: next round" }
        } else if de { "Pfeile und OK, oder 1-9" } else { "Arrows and OK, or 1-9" };
        text_small(s.x + 4, y, hint, "muted");
    }
}
