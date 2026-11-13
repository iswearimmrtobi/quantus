//! Minimisation (§6.4): the prime implicants by Quine–McCluskey, the prime implicant chart and
//! Petrick's method; all minimal covers, fewest terms first, then fewest literals.

use super::expr::{variable_latex, Expr, Notation};
use super::text::msg;
use super::working::{array, list, prime_name, Working};
use crate::settings::StepDetail::{Brief, Detailed, Maximal, Standard};
use super::{rows_of, Bits, Function, Pattern};
use crate::{QuantusError, Result};

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

/// The most products Petrick's method keeps after a multiplication before it gives up.
pub const PETRICK_LIMIT: usize = 5000;

/// Petrick's method over what the chart left: the product of one sum per column, the rows over
/// it, multiplied out with absorption, the sums with the fewest rows first.
///
/// When a step holds more than [`PETRICK_LIMIT`] products, those that already cost more than a
/// cover found greedily are dropped: more rows only cost more, so none of them leads to a
/// minimal cover. Only when that is not enough does the method give up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Petrick {
    /// Per column left, the rows over it, in the order multiplied.
    pub sums: Vec<Vec<usize>>,
    /// The products after each multiplication, each a sorted list of rows.
    pub steps: Vec<Vec<Vec<usize>>>,
    /// The cost of the greedy cover when products had to be dropped against it.
    pub bound: Option<(usize, usize)>,
}

impl Petrick {
    /// Multiplies out the chart's rest; an error when a step holds more than [`PETRICK_LIMIT`]
    /// products that might be minimal.
    pub fn of(chart: &Chart) -> Result<Petrick> {
        let mut sums: Vec<Vec<usize>> = chart.columns_left.iter().map(|c| chart.rows_over(*c)).collect();
        sums.sort_by_key(Vec::len);
        let literals = |product: &[usize]| product.iter().map(|r| chart.primes[*r].literals(chart.arity)).sum::<usize>();
        let mut products: Vec<Vec<usize>> = vec![Vec::new()];
        let mut steps = Vec::new();
        let mut bound = None;
        for sum in &sums {
            let mut next: Vec<Vec<usize>> = Vec::new();
            for product in &products {
                if product.iter().any(|r| sum.contains(r)) {
                    // (X)(X + Y) = X.
                    next.push(product.clone());
                    continue;
                }
                for row in sum {
                    let mut grown = product.clone();
                    let at = grown.partition_point(|r| r < row);
                    grown.insert(at, *row);
                    next.push(grown);
                }
            }
            if next.len() > PETRICK_LIMIT {
                let limit = *bound.get_or_insert_with(|| greedy(chart));
                next.retain(|p| (p.len(), literals(p)) <= limit);
            }
            products = absorb(next);
            if products.len() > PETRICK_LIMIT {
                return Err(QuantusError::other(msg!("core.logic.too_many_solutions")));
            }
            steps.push(products.clone());
        }
        Ok(Petrick { sums, steps, bound })
    }

    /// The products of the last step: every irredundant way to cover what was left (with
    /// [`Petrick::bound`], every one that might be minimal).
    pub fn products(&self) -> &[Vec<usize>] {
        self.steps.last().map_or(&[], |s| &s[..])
    }
}

/// The cost of a cover of the chart's rest, each time the row over the most columns still open,
/// then with the fewest literals.
fn greedy(chart: &Chart) -> (usize, usize) {
    let mut open = chart.columns_left.clone();
    let (mut terms, mut literals) = (0, 0);
    while !open.is_empty() {
        let row = chart.rows_left.iter().copied().max_by_key(|r| {
            let prime = chart.primes[*r];
            (open.iter().filter(|c| prime.covers(**c)).count(), std::cmp::Reverse(prime.literals(chart.arity)))
        });
        let prime = chart.primes[row.expect("every column has a row")];
        open.retain(|c| !prime.covers(*c));
        terms += 1;
        literals += prime.literals(chart.arity);
    }
    (terms, literals)
}

/// The products without those that contain another ($X + XY = X$) and without repeats, the
/// shortest first.
fn absorb(mut products: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
    products.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    products.dedup();
    let mut kept: Vec<Vec<usize>> = Vec::new();
    for product in products {
        if !kept.iter().any(|k| subset(k, &product)) {
            kept.push(product);
        }
    }
    kept
}

/// Whether the sorted list `a` is part of the sorted list `b`.
fn subset(a: &[usize], b: &[usize]) -> bool {
    let mut rest = b.iter();
    a.iter().all(|x| rest.any(|y| y == x))
}

/// The cost of a cover: the number of terms, then the number of literals.
pub fn cost(terms: &[Pattern], arity: usize) -> (usize, usize) {
    (terms.len(), terms.iter().map(|t| t.literals(arity)).sum())
}

/// Which rows a minimisation covers: the 1s for the minimal DNF, the 0s for the minimal KNF.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Form {
    Dnf,
    Knf,
}

/// The whole minimisation of a function: the rounds, the chart, Petrick and every minimal cover.
/// For the KNF the terms cover the 0s, the minimal DNF of $\bar f$, and each is written negated
/// by De Morgan as a sum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Minimisation {
    pub form: Form,
    pub arity: usize,
    pub qm: QuineMcCluskey,
    pub chart: Chart,
    /// `None` when the chart left nothing.
    pub petrick: Option<Petrick>,
    /// Every minimal cover, its terms in the order of their first row, the covers in the order
    /// of their terms.
    pub covers: Vec<Vec<Pattern>>,
}

impl Minimisation {
    /// The minimal DNF: the 1s of `function` covered, with its don't cares.
    pub fn of(function: &Function) -> Result<Minimisation> {
        Minimisation::of_rows(Form::Dnf, function.arity(), function.ones(), function.dont_cares())
    }

    /// The minimal KNF: the 0s of `function` covered, with its don't cares.
    pub fn knf(function: &Function) -> Result<Minimisation> {
        Minimisation::of_rows(Form::Knf, function.arity(), &function.zeros(), function.dont_cares())
    }

    fn of_rows(form: Form, arity: usize, ones: &Bits, dont_cares: &Bits) -> Result<Minimisation> {
        let qm = QuineMcCluskey::of(arity, ones, dont_cares);
        let chart = Chart::of(arity, &qm.primes, ones);
        let petrick = (!chart.columns_left.is_empty()).then(|| Petrick::of(&chart)).transpose()?;
        let rests: Vec<Vec<usize>> = petrick.as_ref().map_or_else(|| vec![Vec::new()], |p| p.products().to_vec());
        let mut covers: Vec<Vec<usize>> = rests
            .into_iter()
            .map(|rest| {
                let mut rows: Vec<usize> = chart.taken.iter().copied().chain(rest).collect();
                rows.sort();
                rows
            })
            .collect();
        let cost_of = |rows: &Vec<usize>| cost(&rows.iter().map(|r| chart.primes[*r]).collect::<Vec<_>>(), arity);
        let best = covers.iter().map(cost_of).min().unwrap_or((0, 0));
        covers.retain(|rows| cost_of(rows) == best);
        covers.sort();
        let covers = covers.into_iter().map(|rows| rows.into_iter().map(|r| chart.primes[r]).collect()).collect();
        Ok(Minimisation { form, arity, qm, chart, petrick, covers })
    }

    /// The cost of the minimal covers.
    pub fn cost(&self) -> (usize, usize) {
        cost(&self.covers[0], self.arity)
    }

    /// Cover `index` over `names`: a sum of products for the DNF, no term the constant 0; a
    /// product of sums for the KNF, no term the constant 1.
    pub fn expr(&self, index: usize, names: &[String]) -> Expr {
        form_expr(self.form, &self.covers[index], names)
    }
}

/// The terms `terms` over `names` as a form: a sum of products for the DNF, no term the constant
/// 0; a product of sums for the KNF, no term the constant 1.
pub fn form_expr(form: Form, terms: &[Pattern], names: &[String]) -> Expr {
    let mut operands: Vec<Expr> = match form {
        Form::Dnf => terms.iter().map(|t| t.product(names)).collect(),
        Form::Knf => terms.iter().map(|t| t.sum(names)).collect(),
    };
    match (operands.len(), form) {
        (0, form) => Expr::Const(form == Form::Knf),
        (1, _) => operands.remove(0),
        (_, Form::Dnf) => Expr::Or(operands),
        (_, Form::Knf) => Expr::And(operands),
    }
}

impl Minimisation {
    /// The working of the minimisation (§8, *Quine–McCluskey* and *Cover*) of the output
    /// `output` over the variables `names`: the minimal forms at Brief; the prime implicants,
    /// the essential ones and the minimal covers at Standard; the grouping tables with ✓, the
    /// chart and each reduction at Detailed; the pairs that did not combine and Petrick's
    /// product multiplied out at Maximal.
    pub fn working(&self, names: &[String], output: &str, notation: Notation) -> Working {
        let mut working = Working::default();
        let knf = self.form == Form::Knf;
        let arity = self.arity;
        let term = |p: Pattern| if knf { p.sum(names) } else { p.product(names) }.latex(notation);
        let column = |row: usize| format!("{}_{{{row}}}", if knf { "M" } else { "m" });
        let pattern = |p: Pattern| format!("\\texttt{{{}}}", p.text(arity));
        let or = if notation == Notation::Technical { " + " } else { " \\lor " };
        if knf {
            working.add(Standard, msg!("core.logic.step.knf_over_zeros"), String::new());
        }

        // Quine–McCluskey.
        for (index, round) in self.qm.rounds.iter().enumerate() {
            let mut rows: Vec<Option<Vec<String>>> = Vec::new();
            for (ones, terms) in &round.groups {
                if !rows.is_empty() {
                    rows.push(None);
                }
                for (i, t) in terms.iter().enumerate() {
                    let count = if i == 0 { ones.to_string() } else { String::new() };
                    let check = if t.combined { "\\checkmark" } else { "" };
                    // A don't care, in the first round, in brackets.
                    let covered = if index == 0 && !self.chart.minterms.contains(&t.rows[0]) { format!("({})", t.rows[0]) } else { list(&t.rows) };
                    rows.push(Some(vec![count, covered, pattern(t.pattern), check.into()]));
                }
            }
            let header = ["\\#1".to_string(), column(0).replace("{0}", "{i}"), String::new(), String::new()];
            let text = match (index, knf) {
                (0, false) => msg!("core.logic.step.qm_minterms"),
                (0, true) => msg!("core.logic.step.qm_maxterms"),
                _ => {
                    let round = index + 1;
                    msg!("core.logic.step.qm_round")
                }
            };
            working.add(Detailed, text, array("c|l|c|c", &header, &rows));
            let failed: Vec<String> = round
                .groups
                .windows(2)
                .filter(|pair| pair[1].0 == pair[0].0 + 1)
                .flat_map(|pair| {
                    pair[0].1.iter().flat_map(move |a| pair[1].1.iter().filter(move |b| a.pattern.mask == b.pattern.mask && a.pattern.combine(b.pattern).is_none()).map(move |b| (a.pattern, b.pattern)))
                })
                .map(|(a, b)| format!("{} \\,/\\, {}", pattern(a), pattern(b)))
                .collect();
            if !failed.is_empty() {
                working.add(Maximal, msg!("core.logic.step.qm_not_combined"), failed.join(",\\ "));
            }
        }
        if !self.qm.primes.is_empty() {
            let rows: Vec<Option<Vec<String>>> = self
                .qm
                .primes
                .iter()
                .enumerate()
                .map(|(i, p)| Some(vec![prime_name(i), pattern(*p), term(*p), list(&p.rows(arity).collect::<Vec<_>>())]))
                .collect();
            working.add(Standard, msg!("core.logic.step.primes"), array("c|c|l|l", &[String::new(), String::new(), String::new(), column(0).replace("{0}", "{i}")], &rows));
        }

        // The chart.
        let chart = &self.chart;
        if !chart.minterms.is_empty() {
            let header: Vec<String> = std::iter::once(String::new()).chain(chart.minterms.iter().map(|m| m.to_string())).collect();
            let rows: Vec<Option<Vec<String>>> = chart
                .primes
                .iter()
                .enumerate()
                .map(|(i, p)| Some(std::iter::once(prime_name(i)).chain(chart.minterms.iter().map(|m| if p.covers(*m) { "\\times".into() } else { String::new() })).collect()))
                .collect();
            let text = if knf { msg!("core.logic.step.chart_knf") } else { msg!("core.logic.step.chart") };
            working.add(Detailed, text, array(&format!("c|{}", "c".repeat(chart.minterms.len())), &header, &rows));
            let named = |row: usize| format!("{} = {}", prime_name(row), term(chart.primes[row]));
            if chart.essential.is_empty() {
                working.add_until(Standard, Detailed, msg!("core.logic.step.no_essentials"), String::new());
            } else {
                let latex = chart.essential.iter().map(|r| named(*r)).collect::<Vec<_>>().join(",\\ ");
                working.add_until(Standard, Detailed, msg!("core.logic.step.essentials"), latex);
            }
            // The reductions replayed, to name what was left at each.
            let mut rows_left: Vec<usize> = (0..chart.primes.len()).collect();
            let mut columns_left = chart.minterms.clone();
            let rows_over = |rows_left: &[usize], c: usize| rows_left.iter().filter(|r| chart.primes[**r].covers(c)).map(|r| prime_name(*r)).collect::<Vec<_>>().join(", ");
            let columns_under = |columns_left: &[usize], r: usize| columns_left.iter().filter(|c| chart.primes[r].covers(**c)).map(|c| column(*c)).collect::<Vec<_>>().join(", ");
            for reduction in &chart.reductions {
                match *reduction {
                    Reduction::Essential { row, column: c } => {
                        working.add(Detailed, msg!("core.logic.step.take"), format!("{}\\colon\\ {}", column(c), named(row)));
                        rows_left.retain(|r| *r != row);
                        columns_left.retain(|c| !chart.primes[row].covers(*c));
                    }
                    Reduction::Covered { row } => {
                        working.add(Detailed, msg!("core.logic.step.drop_covered"), named(row));
                        rows_left.retain(|r| *r != row);
                    }
                    Reduction::ColumnDominates { column: c, over } => {
                        let latex = format!("{}\\colon\\ {} \\supseteq {}\\colon\\ {}", column(c), rows_over(&rows_left, c), column(over), rows_over(&rows_left, over));
                        working.add(Detailed, msg!("core.logic.step.drop_column"), latex);
                        columns_left.retain(|x| *x != c);
                    }
                    Reduction::RowDominated { row, by } => {
                        let latex = format!("{}\\colon\\ {} \\subseteq {}\\colon\\ {}", prime_name(row), columns_under(&columns_left, row), prime_name(by), columns_under(&columns_left, by));
                        working.add(Detailed, msg!("core.logic.step.drop_row"), latex);
                        rows_left.retain(|r| *r != row);
                    }
                }
            }
        }

        // Petrick's method.
        if let Some(petrick) = &self.petrick {
            let product = |rows: &[usize]| if rows.is_empty() { "1".to_string() } else { rows.iter().map(|r| prime_name(*r)).collect::<Vec<_>>().join(" ") };
            let sum = |products: &[Vec<usize>]| products.iter().map(|p| product(p)).collect::<Vec<_>>().join(or);
            let sums: String = petrick
                .sums
                .iter()
                .map(|s| if s.len() == 1 { prime_name(s[0]) } else { format!("({})", s.iter().map(|r| prime_name(*r)).collect::<Vec<_>>().join(or)) })
                .collect();
            working.add(Detailed, msg!("core.logic.step.petrick_sums"), sums);
            for step in &petrick.steps {
                working.add(Maximal, msg!("core.logic.step.petrick_step"), format!("= {}", sum(step)));
            }
            if let Some((terms, literals)) = petrick.bound {
                working.add(Maximal, msg!("core.logic.step.petrick_bound"), String::new());
            }
        }

        // The result.
        if !self.chart.minterms.is_empty() {
            let (terms, literals) = self.cost();
            let covers = self.covers.iter().map(|c| format!("\\{{{}\\}}", c.iter().map(|p| prime_name(self.chart.primes.iter().position(|q| q == p).expect("a cover is made of primes"))).collect::<Vec<_>>().join(", ")));
            working.add(Standard, msg!("core.logic.step.covers"), covers.collect::<Vec<_>>().join(",\\ "));
        }
        let (text, latex) = self.result(names, output, notation);
        working.add(Brief, text, latex);
        working
    }

    /// The line of the result: every minimal form of the output `output`.
    pub(crate) fn result(&self, names: &[String], output: &str, notation: Notation) -> (String, String) {
        let f = variable_latex(output);
        let count = self.covers.len();
        let forms: Vec<String> = (0..count).map(|i| format!("{f} = {}", self.expr(i, names).latex(notation))).collect();
        let text = match (self.form, count) {
            (Form::Dnf, 1) => msg!("core.logic.step.minimal_dnf"),
            (Form::Dnf, _) => msg!("core.logic.step.minimal_dnfs"),
            (Form::Knf, 1) => msg!("core.logic.step.minimal_knf"),
            (Form::Knf, _) => msg!("core.logic.step.minimal_knfs"),
        };
        let latex = if count == 1 { forms.join("") } else { format!("\\begin{{gathered}}{}\\end{{gathered}}", forms.join(" \\\\ ")) };
        (text, latex)
    }
}
