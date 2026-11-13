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
