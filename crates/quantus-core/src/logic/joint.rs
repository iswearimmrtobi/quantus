//! Joint minimisation of several outputs (§6.4.1): one set of terms for all outputs together,
//! a term that serves several outputs paid once.
//!
//! **Tagged Quine–McCluskey**: each row that is 1 or – in some output starts as a minterm tagged
//! with those outputs. Two terms combine as in [`QuineMcCluskey`](super::minimise::QuineMcCluskey)
//! when their tags share an output, the new term's tag the shared outputs; a term is ✓ only when
//! it combined into one with the same tag. The unchecked terms are the prime implicants of the
//! outputs together; a term's tag is every output it is an implicant of.
//!
//! **Cover**: the chart has a column per 1 of each output, a row over it when the term is an
//! implicant of that output and covers the row. Each term costs once (1 term, its literals), so
//! the chart's reductions and Petrick's method are those of one output. Of the cheapest covers
//! (fewest terms, then literals), those whose outputs need the fewest inputs to their ORs are
//! kept; each output takes the fewest of the cover's terms that give it.
//!
//! For the KNF the same runs over the 0s, each term then a sum by De Morgan.

use super::expr::{variable_latex, Expr, Notation};
use super::minimise::{chart_working, cost, form_expr, Chart, ChartLabels, Form, Minimisation, Petrick};
use super::text::msg;
use super::working::{array, list, prime_name, Working};
use super::{all_rows, contains, rows_of, Bits, Function, Pattern};
use crate::settings::StepDetail::{Brief, Detailed, Standard};
use crate::{QuantusError, Result};

/// The most outputs minimised together: a tag is a `u32`.
pub const MAX_JOINT_OUTPUTS: usize = 32;

/// A term of a tagged Quine–McCluskey table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JointTerm {
    pub pattern: Pattern,
    /// Bit `i` for output `i`: the outputs where every row of the term is 1 (0 for the KNF) or –.
    pub tag: u32,
    pub rows: Vec<usize>,
    /// Combined into a term with the same tag.
    pub combined: bool,
}

/// The terms of one round grouped by the number of 1s in their pattern, ascending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JointRound {
    pub groups: Vec<(u32, Vec<JointTerm>)>,
}

impl JointRound {
    pub fn terms(&self) -> impl Iterator<Item = &JointTerm> {
        self.groups.iter().flat_map(|(_, terms)| terms.iter())
    }
}

/// The cost of a joint cover: its distinct terms, their literals, and the inputs of the outputs'
/// ORs (ANDs for the KNF), a term of one output counted once each.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct JointCost {
    pub terms: usize,
    pub literals: usize,
    pub joins: usize,
}

/// A joint minimal cover: the distinct terms, in the order of the primes, and the terms each
/// output takes from them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JointCover {
    pub terms: Vec<Pattern>,
    pub outputs: Vec<Vec<Pattern>>,
}

impl JointCover {
    pub fn cost(&self, arity: usize) -> JointCost {
        let (terms, literals) = cost(&self.terms, arity);
        JointCost { terms, literals, joins: self.outputs.iter().map(Vec::len).sum() }
    }
}

/// The joint minimisation of several outputs over the same variables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JointMinimisation {
    pub form: Form,
    pub arity: usize,
    /// The 1s (0s for the KNF) and don't cares of each output.
    pub ones: Vec<Bits>,
    pub dont_cares: Vec<Bits>,
    pub rounds: Vec<JointRound>,
    /// The prime implicants with their tags, in the order of their first row, then the fewest
    /// literals; the chart's rows.
    pub primes: Vec<(Pattern, u32)>,
    pub chart: Chart,
    pub petrick: Option<Petrick>,
    /// Every joint minimal cover, in the order of their terms.
    pub covers: Vec<JointCover>,
    /// Each output minimised alone (its first minimal form), identical terms counted once.
    pub separate: Option<JointCost>,
}

/// The column of the chart for the row `row` of output `output`.
fn column_id(output: usize, row: usize) -> usize {
    output * 256 + row
}

impl JointMinimisation {
    /// The joint minimal DNF (over the 1s) or KNF (over the 0s) of `functions`, all over the
    /// same variables.
    pub fn of(form: Form, functions: &[&Function]) -> Result<JointMinimisation> {
        if functions.len() > MAX_JOINT_OUTPUTS {
            let max = MAX_JOINT_OUTPUTS;
            return Err(QuantusError::other(msg!("core.logic.joint.too_many_outputs")));
        }
        let arity = functions.first().map_or(0, |f| f.arity());
        assert!(functions.iter().all(|f| f.names() == functions[0].names()), "functions of the same variables");
        let ones: Vec<Bits> = functions.iter().map(|f| if form == Form::Dnf { *f.ones() } else { f.zeros() }).collect();
        let dont_cares: Vec<Bits> = functions.iter().map(|f| *f.dont_cares()).collect();

        // Tagged Quine–McCluskey.
        let tag_of = |row: usize| (0..functions.len()).filter(|i| contains(&ones[*i], row) || contains(&dont_cares[*i], row)).fold(0u32, |tag, i| tag | 1 << i);
        let mut patterns: Vec<(Pattern, u32)> = rows_of(&all_rows(arity)).map(|row| (Pattern::minterm(row), tag_of(row))).filter(|(_, tag)| *tag != 0).collect();
        let mut rounds = Vec::new();
        while !patterns.is_empty() {
            let mut round = group(arity, &patterns);
            let mut next: Vec<(Pattern, u32)> = Vec::new();
            for g in 1..round.groups.len() {
                if round.groups[g].0 != round.groups[g - 1].0 + 1 {
                    continue;
                }
                let (lower, upper) = round.groups.split_at_mut(g);
                for a in lower[g - 1].1.iter_mut() {
                    for b in upper[0].1.iter_mut() {
                        let tag = a.tag & b.tag;
                        if tag == 0 {
                            continue;
                        }
                        if let Some(p) = a.pattern.combine(b.pattern) {
                            a.combined |= tag == a.tag;
                            b.combined |= tag == b.tag;
                            if !next.iter().any(|(q, _)| *q == p) {
                                next.push((p, tag));
                            }
                        }
                    }
                }
            }
            rounds.push(round);
            patterns = next;
        }
        let mut primes: Vec<(Pattern, u32)> = rounds.iter().flat_map(JointRound::terms).filter(|t| !t.combined).map(|t| (t.pattern, t.tag)).collect();
        primes.sort_by_key(|(p, _)| (p.value as usize, p.literals(arity), p.value, p.mask));

        // The chart over the 1s of every output.
        let columns: Vec<usize> = (0..functions.len()).flat_map(|i| rows_of(&ones[i]).map(move |row| column_id(i, row))).collect();
        let patterns: Vec<Pattern> = primes.iter().map(|(p, _)| *p).collect();
        let chart = Chart::with(arity, &patterns, columns, |r, c| primes[r].1 >> (c / 256) & 1 == 1 && primes[r].0.covers(c % 256));
        let petrick = (!chart.columns_left.is_empty()).then(|| Petrick::of(&chart)).transpose()?;
        let rests: Vec<Vec<usize>> = petrick.as_ref().map_or_else(|| vec![Vec::new()], |p| p.products().to_vec());

        // The cheapest covers, then the fewest inputs to the ORs.
        let mut candidates: Vec<(JointCost, Vec<usize>, JointCover)> = Vec::new();
        let best = rests
            .iter()
            .map(|rest| {
                let rows: Vec<&Pattern> = chart.taken.iter().chain(rest).map(|r| &patterns[*r]).collect();
                (rows.len(), rows.iter().map(|p| p.literals(arity)).sum::<usize>())
            })
            .min()
            .unwrap_or((0, 0));
        for rest in rests {
            let mut rows: Vec<usize> = chart.taken.iter().copied().chain(rest).collect();
            rows.sort();
            let terms: Vec<Pattern> = rows.iter().map(|r| patterns[*r]).collect();
            if cost(&terms, arity) != best {
                continue;
            }
            let outputs = (0..functions.len())
                .map(|i| {
                    let own: Vec<Pattern> = rows.iter().filter(|r| primes[**r].1 >> i & 1 == 1).map(|r| patterns[*r]).collect();
                    Minimisation::of_terms(arity, &own, &ones[i]).map(|covers| covers.into_iter().next().unwrap_or_default())
                })
                .collect::<Result<Vec<_>>>()?;
            let cover = JointCover { terms, outputs };
            candidates.push((cover.cost(arity), rows, cover));
        }
        let least = candidates.iter().map(|(c, ..)| *c).min();
        candidates.retain(|(c, ..)| Some(*c) == least);
        candidates.sort_by(|a, b| a.1.cmp(&b.1));
        let covers = candidates.into_iter().map(|(_, _, cover)| cover).collect();

        // Each output alone.
        let separate = (0..functions.len())
            .map(|i| Minimisation::of_rows(form, arity, &ones[i], &dont_cares[i]).map(|m| m.covers[0].clone()))
            .collect::<Result<Vec<_>>>()
            .ok()
            .map(|outputs| {
                let mut terms: Vec<Pattern> = outputs.iter().flatten().copied().collect();
                terms.sort();
                terms.dedup();
                JointCover { terms, outputs }.cost(arity)
            });
        Ok(JointMinimisation { form, arity, ones, dont_cares, rounds, primes, chart, petrick, covers, separate })
    }

    /// The cost of the joint minimal covers.
    pub fn cost(&self) -> JointCost {
        self.covers[0].cost(self.arity)
    }

    /// Output `output` of cover `cover` over `names`.
    pub fn expr(&self, cover: usize, output: usize, names: &[String]) -> Expr {
        form_expr(self.form, &self.covers[cover].outputs[output], names)
    }

    /// The working (§8) of the joint minimisation of the outputs `outputs` over the variables
    /// `names`, cover `chosen` as the result: each output's form at Brief; the prime
    /// implicants with their outputs, the essential ones, the covers and the cost against each
    /// output alone at Standard; the tagged tables, the chart and its reductions at Detailed;
    /// Petrick's product multiplied out at Maximal.
    pub fn working(&self, names: &[String], outputs: &[String], chosen: usize, notation: Notation) -> Working {
        let mut working = Working::default();
        let knf = self.form == Form::Knf;
        let arity = self.arity;
        let term = |p: Pattern| if knf { p.sum(names) } else { p.product(names) }.latex(notation);
        let pattern = |p: Pattern| format!("\\texttt{{{}}}", p.text(arity));
        let tag = |tag: u32| (0..outputs.len()).filter(|i| tag >> i & 1 == 1).map(|i| variable_latex(&outputs[i])).collect::<Vec<_>>().join(", ");
        let or = if notation == Notation::Technical { " + " } else { " \\lor " };
        working.add(Standard, if knf { msg!("core.logic.step.joint_over_knf") } else { msg!("core.logic.step.joint_over") }, String::new());

        // Tagged Quine–McCluskey.
        for (index, round) in self.rounds.iter().enumerate() {
            let mut rows: Vec<Option<Vec<String>>> = Vec::new();
            for (ones, terms) in &round.groups {
                if !rows.is_empty() {
                    rows.push(None);
                }
                for (i, t) in terms.iter().enumerate() {
                    let count = if i == 0 { ones.to_string() } else { String::new() };
                    let check = if t.combined { "\\checkmark" } else { "" };
                    rows.push(Some(vec![count, list(&t.rows), pattern(t.pattern), tag(t.tag), check.into()]));
                }
            }
            let header = ["\\#1".to_string(), "i".into(), String::new(), String::new(), String::new()];
            let text = match (index, knf) {
                (0, false) => msg!("core.logic.step.joint_qm_minterms"),
                (0, true) => msg!("core.logic.step.joint_qm_maxterms"),
                _ => {
                    let round = index + 1;
                    msg!("core.logic.step.joint_qm_round")
                }
            };
            working.add(Detailed, text, array("c|l|c|l|c", &header, &rows));
        }
        if !self.primes.is_empty() {
            let rows: Vec<Option<Vec<String>>> =
                self.primes.iter().enumerate().map(|(i, (p, t))| Some(vec![prime_name(i), pattern(*p), term(*p), tag(*t)])).collect();
            working.add(Standard, msg!("core.logic.step.joint_primes"), array("c|c|l|l", &[String::new(), String::new(), String::new(), String::new()], &rows));
        }

        // The chart and Petrick's method.
        let letter = if knf { "M" } else { "m" };
        let column = |c: usize| format!("{letter}_{{{}}}^{{({})}}", c % 256, variable_latex(&outputs[c / 256]));
        let heading = |c: usize| format!("{}^{{({})}}", c % 256, variable_latex(&outputs[c / 256]));
        let row_term = |row: usize| term(self.primes[row].0);
        let text = if knf { msg!("core.logic.step.joint_chart_knf") } else { msg!("core.logic.step.joint_chart") };
        chart_working(&mut working, &self.chart, self.petrick.as_ref(), text, &ChartLabels { term: &row_term, column: &column, heading: &heading, or });

        // The covers and their cost.
        if !self.chart.columns.is_empty() {
            let JointCost { terms, literals, .. } = self.cost();
            let name = |p: &Pattern| prime_name(self.primes.iter().position(|(q, _)| q == p).expect("a cover is made of primes"));
            let covers = self.covers.iter().map(|c| format!("\\{{{}\\}}", c.terms.iter().map(name).collect::<Vec<_>>().join(", ")));
            working.add(Standard, msg!("core.logic.step.joint_covers"), covers.collect::<Vec<_>>().join(",\\ "));
        }
        if let Some(alone) = self.separate {
            let JointCost { terms, literals, joins } = self.cost();
            let (alone_terms, alone_literals) = (alone.terms, alone.literals);
            let text = if knf { msg!("core.logic.step.joint_cost_knf") } else { msg!("core.logic.step.joint_cost") };
            working.add(Standard, text, String::new());
        }
        let (text, latex) = self.result(names, outputs, chosen, notation);
        working.add(Brief, text, latex);
        working
    }

    /// The line of the result: each output of cover `chosen`.
    pub(crate) fn result(&self, names: &[String], outputs: &[String], chosen: usize, notation: Notation) -> (String, String) {
        let count = self.covers.len();
        let number = chosen + 1;
        let forms: Vec<String> = (0..outputs.len()).map(|o| format!("{} = {}", variable_latex(&outputs[o]), self.expr(chosen, o, names).latex(notation))).collect();
        let text = match (self.form, count) {
            (Form::Dnf, 1) => msg!("core.logic.step.joint_dnf"),
            (Form::Dnf, _) => msg!("core.logic.step.joint_dnf_of"),
            (Form::Knf, 1) => msg!("core.logic.step.joint_knf"),
            (Form::Knf, _) => msg!("core.logic.step.joint_knf_of"),
        };
        (text, format!("\\begin{{gathered}}{}\\end{{gathered}}", forms.join(" \\\\ ")))
    }
}

/// The tagged patterns grouped by their number of 1s, each group in the order of their rows.
fn group(arity: usize, patterns: &[(Pattern, u32)]) -> JointRound {
    let mut sorted = patterns.to_vec();
    sorted.sort_by_key(|(p, _)| (p.value.count_ones(), p.rows(arity).collect::<Vec<_>>()));
    let mut groups: Vec<(u32, Vec<JointTerm>)> = Vec::new();
    for (pattern, tag) in sorted {
        let ones = pattern.value.count_ones();
        let term = JointTerm { pattern, tag, rows: pattern.rows(arity).collect(), combined: false };
        match groups.last_mut() {
            Some((o, terms)) if *o == ones => terms.push(term),
            _ => groups.push((ones, vec![term])),
        }
    }
    JointRound { groups }
}
