//! KV diagrams (§6.3): the layout of 2 to 6 variables, its labels and the blocks.
//!
//! Variables beyond four choose the map: the first $n - 4$ of them. Of the other $m$, the first
//! $\lceil m/2 \rceil$ run along the columns and the rest along the rows, each axis in Gray
//! order (00, 01, 11, 10). Five variables give two maps side by side, the first variable 0 on the
//! left; six give a 2 × 2 grid of maps, the first variable along the grid's columns, the second
//! along its rows. The maps are copies, not mirrors: cells in the same place of neighbouring maps
//! are neighbours.

use super::minimise::Minimisation;
use super::{Function, Pattern, Value};

/// The fewest variables of a KV diagram.
pub const MIN_VARIABLES: usize = 2;
/// The most variables of a KV diagram; above, the Quine–McCluskey tables take its place.
pub const MAX_VARIABLES: usize = 6;

/// The code at position `k` of the Gray order: 0, 1, 3, 2, …
pub fn gray(k: usize) -> usize {
    k ^ (k >> 1)
}

/// The position of `code` in the Gray order, the inverse of [`gray`].
pub fn gray_position(code: usize) -> usize {
    let mut k = code;
    let mut shift = code >> 1;
    while shift != 0 {
        k ^= shift;
        shift >>= 1;
    }
    k
}

/// The place of a cell: its map and its row and column in the map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Place {
    /// The map, the value of the map variables (the first variable most significant).
    pub map: usize,
    pub row: usize,
    pub col: usize,
}

/// How the variables of a function are spread over maps, columns and rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub arity: usize,
    /// The number of variables choosing the map, the first ones.
    pub map_vars: usize,
    /// The number of variables along the columns, after the map variables.
    pub col_vars: usize,
    /// The number of variables along the rows, the last ones.
    pub row_vars: usize,
}

impl Layout {
    /// The layout of `arity` variables; `None` outside [`MIN_VARIABLES`] to [`MAX_VARIABLES`].
    pub fn new(arity: usize) -> Option<Layout> {
        if !(MIN_VARIABLES..=MAX_VARIABLES).contains(&arity) {
            return None;
        }
        let map_vars = arity.saturating_sub(4);
        let rest = arity - map_vars;
        let col_vars = rest.div_ceil(2);
        Some(Layout { arity, map_vars, col_vars, row_vars: rest - col_vars })
    }

    /// The number of maps, 1, 2 or 4.
    pub fn maps(&self) -> usize {
        1 << self.map_vars
    }

    /// The columns of one map.
    pub fn columns(&self) -> usize {
        1 << self.col_vars
    }

    /// The rows of one map.
    pub fn rows(&self) -> usize {
        1 << self.row_vars
    }

    /// The grid of maps: (columns, rows).
    pub fn map_grid(&self) -> (usize, usize) {
        match self.map_vars {
            0 => (1, 1),
            1 => (2, 1),
            _ => (2, 2),
        }
    }

    /// Where `map` sits in the grid of maps: (column, row). The first map variable chooses the
    /// column, the second the row.
    pub fn map_place(&self, map: usize) -> (usize, usize) {
        match self.map_vars {
            0 => (0, 0),
            1 => (map, 0),
            _ => (map >> 1, map & 1),
        }
    }

    /// The variables choosing the map, by index.
    pub fn map_variables(&self) -> std::ops::Range<usize> {
        0..self.map_vars
    }

    /// The variables along the columns, by index; the first is the most significant bit of a
    /// column's code.
    pub fn column_variables(&self) -> std::ops::Range<usize> {
        self.map_vars..self.map_vars + self.col_vars
    }

    /// The variables along the rows, by index.
    pub fn row_variables(&self) -> std::ops::Range<usize> {
        self.map_vars + self.col_vars..self.arity
    }

    /// The row of the truth table in the cell at `place`.
    pub fn row_number(&self, place: Place) -> usize {
        place.map << (self.col_vars + self.row_vars) | gray(place.col) << self.row_vars | gray(place.row)
    }

    /// The cell of the truth table's row `row`.
    pub fn place(&self, row: usize) -> Place {
        let mask = |bits: usize| (1 << bits) - 1;
        Place {
            map: row >> (self.col_vars + self.row_vars),
            col: gray_position(row >> self.row_vars & mask(self.col_vars)),
            row: gray_position(row & mask(self.row_vars)),
        }
    }

    /// The neighbours of the row `row`, one per variable in the order of the variables: the rows
    /// that differ from it in that variable only. In the diagram they are the cells next to it,
    /// over the edges of a map, and in the same place of the neighbouring maps.
    pub fn neighbours(&self, row: usize) -> Vec<usize> {
        (0..self.arity).map(|k| row ^ 1 << (self.arity - 1 - k)).collect()
    }
}

/// An edge of a map, or of the grid of maps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// A variable's label in German school style: a bar with its name along `edge`, over the
/// columns or rows where the variable is 1, `start` and `len` of them. With `maps`, the bar runs
/// along the grid of maps and counts maps; otherwise it is drawn at every map and counts cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bar {
    pub variable: usize,
    pub edge: Edge,
    pub start: usize,
    pub len: usize,
    pub maps: bool,
}

/// One cell of the diagram: its place, the row of the truth table $i$, the inputs of that row
/// (`1-0-` style, the first variable first; the tooltip with $i$) and the value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub place: Place,
    pub index: usize,
    pub code: String,
    pub value: Value,
}

impl Layout {
    /// The bars of every variable: the map variables first (the first on top of the grid, the
    /// second on its left), then the columns' (the first on top, the second at the bottom) and
    /// the rows' (the first on the left, the second on the right).
    pub fn bars(&self) -> Vec<Bar> {
        let mut bars = Vec::new();
        for (j, variable) in self.map_variables().enumerate() {
            // The maps where the variable is 1: the right column, the bottom row of the grid.
            let edge = if j == 0 { Edge::Top } else { Edge::Left };
            bars.push(Bar { variable, edge, start: 1, len: 1, maps: true });
        }
        let axis = |bars: &mut Vec<Bar>, variables: std::ops::Range<usize>, edges: [Edge; 2]| {
            let bits = variables.len();
            for (j, variable) in variables.enumerate() {
                let ones: Vec<usize> = (0..1 << bits).filter(|k| gray(*k) >> (bits - 1 - j) & 1 == 1).collect();
                // With at most two variables per axis, the 1s are always side by side.
                bars.push(Bar { variable, edge: edges[j], start: ones[0], len: ones.len(), maps: false });
            }
        };
        axis(&mut bars, self.column_variables(), [Edge::Top, Edge::Bottom]);
        axis(&mut bars, self.row_variables(), [Edge::Left, Edge::Right]);
        bars
    }

    /// The cells of `function`, map by map, each row by row from the left.
    pub fn cells(&self, function: &Function) -> Vec<Cell> {
        let mut cells = Vec::with_capacity(1 << self.arity);
        for map in 0..self.maps() {
            for row in 0..self.rows() {
                for col in 0..self.columns() {
                    let place = Place { map, row, col };
                    let index = self.row_number(place);
                    let code = Pattern::minterm(index).text(self.arity);
                    cells.push(Cell { place, index, code, value: function.value(index) });
                }
            }
        }
        cells
    }
}

/// One rectangle of a block within one map, in cells. A block that wraps over an edge is drawn
/// as pieces at both edges, each open towards the edge it continues over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub map: usize,
    pub col: usize,
    pub row: usize,
    pub width: usize,
    pub height: usize,
    pub open: Open,
}

/// The sides of a piece where the block goes on over the map's edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Open {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

/// A term of a minimal form as a block: the rows of the truth table it covers and the pieces it
/// is drawn as, map by map, then row by row and column by column. For the KNF the term covers 0s.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub term: Pattern,
    pub cells: Vec<usize>,
    pub pieces: Vec<Piece>,
}

/// A run of positions along one axis: start, length, open at the start, open at the end.
type Run = (usize, usize, bool, bool);

/// The positions `0..len` that `keep` keeps, as runs side by side; when the first run starts at
/// 0 and the last ends at `len` they are one run over the edge, and open there.
fn runs(len: usize, keep: impl Fn(usize) -> bool) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for k in (0..len).filter(|k| keep(*k)) {
        match runs.last_mut() {
            Some((start, length, _, _)) if *start + *length == k => *length += 1,
            _ => runs.push((k, 1, false, false)),
        }
    }
    if runs.len() > 1 && runs[0].0 == 0 && runs.last().is_some_and(|r| r.0 + r.1 == len) {
        runs[0].2 = true;
        runs.last_mut().expect("more than one run").3 = true;
    }
    runs
}

impl Layout {
    /// The block of `term`.
    pub fn block(&self, term: Pattern) -> Block {
        let (c, r) = (self.col_vars, self.row_vars);
        // Whether `code` of `bits` bits, `shift` bits up in a row number, fits the term.
        let fits = |code: usize, bits: usize, shift: usize| {
            let part = |x: u8| (x as usize >> shift) & ((1 << bits) - 1);
            code & !part(term.mask) == part(term.value)
        };
        let cols = runs(self.columns(), |k| fits(gray(k), c, r));
        let rows = runs(self.rows(), |k| fits(gray(k), r, 0));
        let mut pieces = Vec::new();
        for map in (0..self.maps()).filter(|m| fits(*m, self.map_vars, c + r)) {
            for &(row, height, top, bottom) in &rows {
                for &(col, width, left, right) in &cols {
                    pieces.push(Piece { map, col, row, width, height, open: Open { left, right, top, bottom } });
                }
            }
        }
        Block { term, cells: term.rows(self.arity).collect(), pieces }
    }

    /// The blocks of the minimal cover `cover` of `minimisation`: over the 1s for the DNF, over
    /// the 0s for the KNF.
    pub fn blocks(&self, minimisation: &Minimisation, cover: usize) -> Vec<Block> {
        minimisation.covers[cover].iter().map(|term| self.block(*term)).collect()
    }
}
