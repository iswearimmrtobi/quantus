//! KV diagrams (§6.3): the layout of 2 to 6 variables.
//!
//! Variables beyond four choose the map: the first $n - 4$ of them. Of the other $m$, the first
//! $\lceil m/2 \rceil$ run along the columns and the rest along the rows, each axis in Gray
//! order (00, 01, 11, 10). Five variables give two maps side by side, the first variable 0 on the
//! left; six give a 2 × 2 grid of maps, the first variable along the grid's columns, the second
//! along its rows. The maps are copies, not mirrors: cells in the same place of neighbouring maps
//! are neighbours.

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
