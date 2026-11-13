//! Minimisation (§6.4): the prime implicants by Quine–McCluskey, the prime implicant chart and
//! Petrick's method; all minimal covers, fewest terms first, then fewest literals.

use super::expr::Expr;
use super::{rows_of, Bits, Pattern};

/// A term of a Quine–McCluskey table: its pattern, the rows it covers (ones and don't cares) and
/// whether it combined with another (✓); a term that never combined is a prime implicant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    pub pattern: Pattern,
    pub rows: Vec<usize>,
    pub combined: bool,
}

/// The terms of one round grouped by the number of 1s in their pattern, ascending; empty groups
/// are left out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Round {
    pub groups: Vec<(u32, Vec<Term>)>,
}

impl Round {
    pub fn terms(&self) -> impl Iterator<Item = &Term> {
        self.groups.iter().flat_map(|(_, terms)| terms.iter())
    }
}

/// The rounds of Quine–McCluskey and the prime implicants they leave.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuineMcCluskey {
    /// The minterms first (the ones and the don't cares), then each round's combined terms, until
    /// nothing combines.
    pub rounds: Vec<Round>,
    /// The prime implicants in the order of their first row, then the fewest literals.
    pub primes: Vec<Pattern>,
}

impl QuineMcCluskey {
    /// The prime implicants of the rows `ones` and `dont_cares` among `arity` variables.
    pub fn of(arity: usize, ones: &Bits, dont_cares: &Bits) -> QuineMcCluskey {
        let start: Vec<Pattern> = rows_of(ones).chain(rows_of(dont_cares)).map(Pattern::minterm).collect();
        let mut rounds = Vec::new();
        let mut patterns = start;
        while !patterns.is_empty() {
            let mut round = group(arity, &patterns);
            let mut next: Vec<Pattern> = Vec::new();
            for g in 1..round.groups.len() {
                if round.groups[g].0 != round.groups[g - 1].0 + 1 {
                    continue;
                }
                let (lower, upper) = round.groups.split_at_mut(g);
                for a in lower[g - 1].1.iter_mut() {
                    for b in upper[0].1.iter_mut() {
                        if let Some(p) = a.pattern.combine(b.pattern) {
                            a.combined = true;
                            b.combined = true;
                            if !next.contains(&p) {
                                next.push(p);
                            }
                        }
                    }
                }
            }
            rounds.push(round);
            patterns = next;
        }
        let mut primes: Vec<Pattern> = rounds.iter().flat_map(Round::terms).filter(|t| !t.combined).map(|t| t.pattern).collect();
        primes.sort_by_key(|p| (first_row(*p), p.literals(arity), p.value, p.mask));
        QuineMcCluskey { rounds, primes }
    }
}

/// The patterns grouped by their number of 1s, each group in the order of the patterns' rows.
fn group(arity: usize, patterns: &[Pattern]) -> Round {
    let mut groups: Vec<(u32, Vec<Term>)> = Vec::new();
    let mut sorted: Vec<Pattern> = patterns.to_vec();
    sorted.sort_by_key(|p| (p.value.count_ones(), p.rows(arity).collect::<Vec<_>>()));
    for pattern in sorted {
        let ones = pattern.value.count_ones();
        let term = Term { pattern, rows: pattern.rows(arity).collect(), combined: false };
        match groups.last_mut() {
            Some((o, terms)) if *o == ones => terms.push(term),
            _ => groups.push((ones, vec![term])),
        }
    }
    Round { groups }
}

/// The smallest row a pattern covers: the dashes as 0.
pub fn first_row(pattern: Pattern) -> usize {
    pattern.value as usize
}

impl Pattern {
    /// The product of the term's literals over the variables `names`: `1-0` is $A \bar{C}$; no
    /// literal at all is the constant 1.
    pub fn product(self, names: &[String]) -> Expr {
        let literals = self.literal_exprs(names, false);
        match literals.len() {
            0 => Expr::Const(true),
            1 => literals.into_iter().next().unwrap(),
            _ => Expr::And(literals),
        }
    }

    /// The negated term by De Morgan, a sum: `1-0` gives $\bar{A} \lor C$; no literal at all is
    /// the constant 0.
    pub fn sum(self, names: &[String]) -> Expr {
        let literals = self.literal_exprs(names, true);
        match literals.len() {
            0 => Expr::Const(false),
            1 => literals.into_iter().next().unwrap(),
            _ => Expr::Or(literals),
        }
    }

    fn literal_exprs(self, names: &[String], negated: bool) -> Vec<Expr> {
        let n = names.len();
        (0..n)
            .filter(|k| self.mask >> (n - 1 - k) & 1 == 0)
            .map(|k| {
                let variable = Expr::Var(names[k].clone());
                if (self.value >> (n - 1 - k) & 1 == 1) == negated { Expr::negation(variable) } else { variable }
            })
            .collect()
    }
}

/// One reduction of the prime implicant chart, rows by their index in [`Chart::primes`],
/// columns by their minterm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reduction {
    /// `row` is the only prime implicant left over `column`, so it is taken.
    Essential { row: usize, column: usize },
    /// `row` covers no column that is left.
    Covered { row: usize },
    /// Every row over `over` is also over `column`, which is therefore covered anyway.
    ColumnDominates { column: usize, over: usize },
    /// `by` covers every column left that `row` covers, with fewer literals.
    RowDominated { row: usize, by: usize },
}

/// The prime implicant chart (§6.4): rows the prime implicants, columns the minterms with value
/// 1, reduced by essential rows and dominance until nothing changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chart {
    pub arity: usize,
    pub primes: Vec<Pattern>,
    pub minterms: Vec<usize>,
    /// The essential prime implicants: the rows taken before any dominance.
    pub essential: Vec<usize>,
    /// Every row taken because it alone covered a column, the essential ones first.
    pub taken: Vec<usize>,
    pub reductions: Vec<Reduction>,
    /// What is left for Petrick's method.
    pub rows_left: Vec<usize>,
    pub columns_left: Vec<usize>,
}

impl Chart {
    /// The chart of the prime implicants `primes` over the minterms `ones`, reduced.
    ///
    /// A row is only dropped as dominated when the dominating row has fewer literals: with as
    /// many, both can be part of different minimal covers, all of which are listed.
    pub fn of(arity: usize, primes: &[Pattern], ones: &Bits) -> Chart {
        let minterms: Vec<usize> = rows_of(ones).collect();
        let mut chart = Chart {
            arity,
            primes: primes.to_vec(),
            minterms: minterms.clone(),
            essential: Vec::new(),
            taken: Vec::new(),
            reductions: Vec::new(),
            rows_left: (0..primes.len()).collect(),
            columns_left: minterms,
        };
        chart.drop_empty_rows();
        chart.take_essential();
        chart.essential = chart.taken.clone();
        while chart.drop_dominating_columns() | chart.drop_dominated_rows() | chart.take_essential() {}
        chart
    }

    /// The rows left over `column`.
    pub fn rows_over(&self, column: usize) -> Vec<usize> {
        self.rows_left.iter().copied().filter(|r| self.primes[*r].covers(column)).collect()
    }

    /// The columns left under `row`.
    pub fn columns_under(&self, row: usize) -> Vec<usize> {
        self.columns_left.iter().copied().filter(|c| self.primes[row].covers(*c)).collect()
    }

    /// Takes every row that alone covers a column left; whether any was.
    fn take_essential(&mut self) -> bool {
        let mut changed = false;
        let mut c = 0;
        while c < self.columns_left.len() {
            let column = self.columns_left[c];
            if let [row] = self.rows_over(column)[..] {
                self.reductions.push(Reduction::Essential { row, column });
                self.taken.push(row);
                self.rows_left.retain(|r| *r != row);
                let prime = self.primes[row];
                self.columns_left.retain(|c| !prime.covers(*c));
                self.drop_empty_rows();
                changed = true;
                c = 0;
            } else {
                c += 1;
            }
        }
        changed
    }

    fn drop_empty_rows(&mut self) {
        for row in self.rows_left.clone() {
            if self.columns_under(row).is_empty() {
                self.reductions.push(Reduction::Covered { row });
                self.rows_left.retain(|r| *r != row);
            }
        }
    }

    /// Drops each column whose rows include all rows of another column left.
    fn drop_dominating_columns(&mut self) -> bool {
        let mut changed = false;
        let mut c = 0;
        while c < self.columns_left.len() {
            let column = self.columns_left[c];
            let rows = self.rows_over(column);
            let over = self.columns_left.iter().copied().find(|other| {
                let theirs = self.rows_over(*other);
                // Of two equal columns the later one goes.
                *other != column && theirs.iter().all(|r| rows.contains(r)) && (theirs.len() < rows.len() || *other < column)
            });
            if let Some(over) = over {
                self.reductions.push(Reduction::ColumnDominates { column, over });
                self.columns_left.remove(c);
                changed = true;
            } else {
                c += 1;
            }
        }
        changed
    }

    /// Drops each row whose columns another row with fewer literals covers too.
    fn drop_dominated_rows(&mut self) -> bool {
        let mut changed = false;
        for row in self.rows_left.clone() {
            let mine = self.columns_under(row);
            let literals = self.primes[row].literals(self.arity);
            let by = self.rows_left.iter().copied().find(|other| {
                self.primes[*other].literals(self.arity) < literals && mine.iter().all(|c| self.primes[*other].covers(*c))
            });
            if let Some(by) = by {
                self.reductions.push(Reduction::RowDominated { row, by });
                self.rows_left.retain(|r| *r != row);
                changed = true;
            }
        }
        if changed {
            self.drop_empty_rows();
        }
        changed
    }
}
