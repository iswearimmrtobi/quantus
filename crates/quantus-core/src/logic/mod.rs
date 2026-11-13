//! Logic mode: combinational digital logic (`design/digital-logic.md`).
//!
//! A function of up to [`MAX_INPUTS`] inputs is a pair of bitsets over its rows, the ones and the
//! don't cares; every other row is 0. Row `i` gives the first input the most significant bit of
//! `i`, so the rows are in the binary order of the truth table.

pub mod expr;
pub mod minimise;
pub mod parse;
pub mod table;
pub mod text;

use crate::{QuantusError, Result};
use text::msg;

/// The most inputs of a function: 256 rows.
pub const MAX_INPUTS: usize = 8;

/// A set of rows, bit `i` of the set for row `i`.
pub type Bits = [u64; 4];

/// The value of one row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    Zero,
    One,
    /// Don't care, written –.
    DontCare,
}

/// A function $\{0,1\}^n \to \{0, 1, –\}$ with its variables' names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    names: Vec<String>,
    ones: Bits,
    dont_cares: Bits,
}

impl Function {
    /// The function of the variables `names` that is 0 everywhere.
    pub fn new(names: Vec<String>) -> Result<Function> {
        check_names(&names)?;
        Ok(Function { names, ones: [0; 4], dont_cares: [0; 4] })
    }

    /// The function with the rows `ones` and `dont_cares` (rows outside the table are dropped).
    pub fn from_bits(names: Vec<String>, ones: Bits, dont_cares: Bits) -> Result<Function> {
        let mut function = Function::new(names)?;
        let all = function.all_rows();
        for word in 0..4 {
            function.dont_cares[word] = dont_cares[word] & all[word];
            function.ones[word] = ones[word] & all[word] & !function.dont_cares[word];
        }
        Ok(function)
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// The number of inputs.
    pub fn arity(&self) -> usize {
        self.names.len()
    }

    /// The number of rows, $2^n$.
    pub fn rows(&self) -> usize {
        1 << self.names.len()
    }

    pub fn ones(&self) -> &Bits {
        &self.ones
    }

    pub fn dont_cares(&self) -> &Bits {
        &self.dont_cares
    }

    /// The rows that are 0.
    pub fn zeros(&self) -> Bits {
        let all = self.all_rows();
        std::array::from_fn(|w| all[w] & !self.ones[w] & !self.dont_cares[w])
    }

    /// Every row of the table.
    pub fn all_rows(&self) -> Bits {
        all_rows(self.arity())
    }

    pub fn value(&self, row: usize) -> Value {
        if contains(&self.ones, row) {
            Value::One
        } else if contains(&self.dont_cares, row) {
            Value::DontCare
        } else {
            Value::Zero
        }
    }

    pub fn set(&mut self, row: usize, value: Value) {
        assert!(row < self.rows(), "row {row} outside a table of {} rows", self.rows());
        set_bit(&mut self.ones, row, value == Value::One);
        set_bit(&mut self.dont_cares, row, value == Value::DontCare);
    }

    /// The value of variable `index` (0 the first, most significant) in `row`.
    pub fn input(&self, row: usize, index: usize) -> bool {
        (row >> (self.arity() - 1 - index)) & 1 == 1
    }
}

/// The rows of a table of `arity` inputs.
pub fn all_rows(arity: usize) -> Bits {
    let rows = 1usize << arity;
    std::array::from_fn(|w| match rows.saturating_sub(64 * w) {
        0 => 0,
        n if n >= 64 => u64::MAX,
        n => (1u64 << n) - 1,
    })
}

pub fn contains(bits: &Bits, row: usize) -> bool {
    bits[row / 64] >> (row % 64) & 1 == 1
}

pub fn set_bit(bits: &mut Bits, row: usize, on: bool) {
    if on {
        bits[row / 64] |= 1 << (row % 64);
    } else {
        bits[row / 64] &= !(1 << (row % 64));
    }
}

/// The rows of `bits`, ascending.
pub fn rows_of(bits: &Bits) -> impl Iterator<Item = usize> + '_ {
    (0..256).filter(|row| contains(bits, *row))
}

/// At most [`MAX_INPUTS`] names, none twice.
pub fn check_names(names: &[String]) -> Result<()> {
    let count = names.len();
    if count > MAX_INPUTS {
        return Err(QuantusError::other(msg!("core.logic.too_many_variables")));
    }
    if let Some(name) = names.iter().enumerate().find_map(|(i, name)| names[..i].contains(name).then_some(name)) {
        return Err(QuantusError::other(msg!("core.logic.name_twice")));
    }
    Ok(())
}

/// A product term as a row pattern such as `1-0-`: `mask` holds the variables that are left out
/// (the dashes), `value` the others' values (0 under the dashes). Bit $n-1-k$ is variable $k$.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pattern {
    pub value: u8,
    pub mask: u8,
}

impl Pattern {
    /// The pattern of the single row `row`, a minterm.
    pub fn minterm(row: usize) -> Pattern {
        Pattern { value: row as u8, mask: 0 }
    }

    /// Whether the term is 1 in `row`.
    pub fn covers(self, row: usize) -> bool {
        (row as u8) & !self.mask == self.value
    }

    /// The pattern of the two terms together when they differ in one variable only.
    pub fn combine(self, other: Pattern) -> Option<Pattern> {
        let differ = self.value ^ other.value;
        (self.mask == other.mask && differ.count_ones() == 1)
            .then_some(Pattern { value: self.value & !differ, mask: self.mask | differ })
    }

    /// The number of literals of the term among `arity` variables.
    pub fn literals(self, arity: usize) -> usize {
        arity - self.mask.count_ones() as usize
    }

    /// The rows the term covers among `arity` variables.
    pub fn rows(self, arity: usize) -> impl Iterator<Item = usize> {
        (0..1usize << arity).filter(move |row| self.covers(*row))
    }

    /// `1-0-`: the first variable first.
    pub fn text(self, arity: usize) -> String {
        (0..arity)
            .rev()
            .map(|bit| match (self.mask >> bit & 1, self.value >> bit & 1) {
                (1, _) => '-',
                (_, 1) => '1',
                _ => '0',
            })
            .collect()
    }
}
