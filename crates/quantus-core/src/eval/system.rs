//! Systems of linear equations: `2x + y = 5; x - y = 1`.
//!
//! The unknowns are the free variables without a value, with or without a subscript (`x_1`), in
//! the order `x, y, z, a, b, … w` (see [`variable_order`]). Each
//! equation's coefficients are read off by evaluating its residual `left - right` at zero and at
//! each unit vector, as the single-equation solver does for polynomials, and checked at other
//! points: a residual that misses them is not linear. That also covers `\pi x`, fractions and
//! session variables as coefficients. The augmented matrix is then reduced exactly, so the
//! system has one solution, none, or infinitely many, with no tolerance in between.
//!
//! The setting `elimination` picks the method: the school Gauss method brings the matrix to
//! triangle form and solves from the bottom up, setting free unknowns to parameters `λ, μ, …`;
//! Gauss–Jordan reduces it fully and reads the solution off.

use super::evaluator::Evaluator;
use super::solver::collect_unknowns;
use crate::latex::Node;
use crate::linear::{self, Echelon, Elimination, Outcome};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::settings::Elimination as Method;
use crate::{QuantusError, Result};

/// A solved system: the unknowns, the augmented matrix and its reduction.
pub(crate) struct SystemSolution {
    pub unknowns: Vec<String>,
    /// `[A | b]` as read from the equations, one row per equation.
    pub matrix: Vec<Vec<Value>>,
    pub reduction: Reduction,
    pub solution: Solution,
}

/// How the matrix was reduced, by the method of the setting.
pub(crate) enum Reduction {
    /// The triangle form and the back-substitution from it, bottom row first; empty if the
    /// system contradicts itself.
    Gauss { echelon: Echelon, back: Vec<Back> },
    GaussJordan(Elimination),
}

/// One unknown found by back-substitution: the row of the echelon form solved for the unknown
/// of its pivot, with the unknowns below already put in.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Back {
    pub row: usize,
    pub column: usize,
    pub value: Affine,
}

/// `c + Σ k_j·p_j`: a constant plus multiples of the parameters that stand for the free
/// unknowns. `terms` has one factor per unknown; it is zero except at free columns.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Affine {
    pub constant: Value,
    pub terms: Vec<Value>,
}

pub(crate) enum Solution {
    /// The value of each unknown.
    Unique(Vec<Value>),
    /// Each unknown with a pivot as a constant minus multiples of the free unknowns, which
    /// can take any value: `x = c - Σ k·y`, given as `(x, c, [(y, k)])` by index. After the
    /// Gauss method the free unknowns are set to `parameters` (`λ, μ, …`); after Gauss–Jordan
    /// they stand for themselves and `parameters` is empty.
    Parametric { free: Vec<usize>, bound: Vec<(usize, Value, Vec<(usize, Value)>)>, parameters: Vec<String> },
    /// The row of the reduced matrix that reads `0 = c` with `c ≠ 0`.
    Inconsistent { row: usize },
}

impl Evaluator<'_> {
    pub(crate) fn solve_system(&self, system: &Node, method: Method) -> Result<SystemSolution> {
        let Node::System { equations, .. } = system else {
            return Err(QuantusError::other("Only a system of equations can be solved as one."));
        };
        let mut unknowns = Vec::new();
        collect_unknowns(system, true, &mut Vec::new(), self, &mut unknowns);
        if unknowns.is_empty() {
            return Err(QuantusError::other(
                "The system has no unknown to solve for. Use letters such as x and y without a value.",
            ));
        }
        unknowns.sort_by_cached_key(|key| variable_order(key));

        let mut matrix = Vec::new();
        for equation in equations {
            matrix.push(self.coefficients(equation, &unknowns)?);
        }
        let n = unknowns.len();
        let (reduction, solution) = match method {
            Method::Gauss => {
                let echelon = linear::gauss(matrix.clone(), n)?;
                if let Some(row) = echelon.contradiction() {
                    (Reduction::Gauss { echelon, back: Vec::new() }, Solution::Inconsistent { row })
                } else {
                    let back = back_substitute(&echelon, n)?;
                    let solution = gauss_solution(&echelon, &back, &unknowns);
                    (Reduction::Gauss { echelon, back }, solution)
                }
            }
            Method::GaussJordan => {
                let elimination = linear::eliminate(matrix.clone(), n, true)?;
                let solution = jordan_solution(&elimination, n);
                (Reduction::GaussJordan(elimination), solution)
            }
        };
        Ok(SystemSolution { unknowns, matrix, reduction, solution })
    }

    /// The row `a_1 … a_n | b` of a linear equation `a·x = b` in the given unknowns.
    fn coefficients(&self, equation: &Node, unknowns: &[String]) -> Result<Vec<Value>> {
        let Node::Equation { left, right, .. } = equation else {
            return Err(QuantusError::other("Every line of a system must be an equation."));
        };
        let mut scope = self.variables.clone();
        let mut residual = |point: &[Value]| -> Result<Value> {
            for (name, value) in unknowns.iter().zip(point) {
                scope.insert(name.clone(), PhysicalQuantity::dimensionless(value.clone()));
            }
            let evaluator = self.scoped_over(&scope);
            Ok(evaluator.evaluate(left)?.sub(&evaluator.evaluate(right)?)?.value)
        };
        let not_linear = || {
            QuantusError::other(format!(
                "The system is not linear in {}; only linear systems can be solved.",
                unknowns.join(", ")
            ))
        };
        let n = unknowns.len();
        let at = |residual: &mut dyn FnMut(&[Value]) -> Result<Value>, point: &[Value]| match residual(point) {
            Ok(value) => Ok(value),
            Err(e @ (QuantusError::DimensionMismatch(_) | QuantusError::UnresolvedVariable(_))) => Err(e),
            // Undefined somewhere, like 1/x at 0: no linear residual is.
            Err(_) => Err(not_linear()),
        };

        let origin = vec![Value::zero(); n];
        let constant = at(&mut residual, &origin)?;
        let mut row = Vec::with_capacity(n + 1);
        for j in 0..n {
            let mut unit = origin.clone();
            unit[j] = Value::one();
            row.push(at(&mut residual, &unit)?.sub(&constant));
        }

        // A linear residual through these points must also hit others, including points far
        // out and between the integers, which catch `|x|` or `x^2` that only agree near 0.
        let probes = [Value::int(2), Value::ratio(-7, 3), Value::int(5), Value::ratio(11, 2), Value::int(-1000)];
        for shift in 0..3 {
            let point: Vec<Value> = (0..n).map(|j| probes[(j + shift) % probes.len()].clone()).collect();
            let expected = row.iter().zip(&point).fold(constant.clone(), |sum, (a, x)| sum.add(&a.mul(x)));
            if at(&mut residual, &point)? != expected {
                return Err(not_linear());
            }
        }
        row.push(constant.neg());
        Ok(row)
    }
}

/// The solution read off the reduced row echelon form.
fn jordan_solution(elimination: &Elimination, n: usize) -> Solution {
    match elimination.outcome() {
        Outcome::Unique(values) => Solution::Unique(values),
        Outcome::Inconsistent { row } => Solution::Inconsistent { row },
        Outcome::Infinite { free } => {
            let bound = elimination
                .pivots
                .iter()
                .enumerate()
                .map(|(row, &column)| {
                    let terms = free
                        .iter()
                        .filter(|&&f| !elimination.matrix[row][f].is_zero())
                        .map(|&f| (f, elimination.matrix[row][f].clone()))
                        .collect();
                    (column, elimination.matrix[row][n].clone(), terms)
                })
                .collect();
            Solution::Parametric { free, bound, parameters: Vec::new() }
        }
    }
}

/// Solves the echelon form from the bottom up: each pivot row gives its unknown once the
/// unknowns to its right are known, the free ones as parameters.
fn back_substitute(echelon: &Echelon, n: usize) -> Result<Vec<Back>> {
    let mut values: Vec<Option<Affine>> = vec![None; n];
    for column in (0..n).filter(|c| !echelon.pivots.contains(c)) {
        let mut terms = vec![Value::zero(); n];
        terms[column] = Value::one();
        values[column] = Some(Affine { constant: Value::zero(), terms });
    }
    let mut back = Vec::new();
    for (row, &column) in echelon.pivots.iter().enumerate().rev() {
        let entries = &echelon.matrix[row];
        let mut constant = entries[n].clone();
        let mut terms = vec![Value::zero(); n];
        for (j, a) in entries.iter().enumerate().take(n).skip(column + 1) {
            if a.is_zero() {
                continue;
            }
            let known = values[j].as_ref().expect("the unknowns right of a pivot are known");
            constant = constant.sub(&a.mul(&known.constant));
            for (term, k) in terms.iter_mut().zip(&known.terms) {
                *term = term.sub(&a.mul(k));
            }
        }
        let pivot = &entries[column];
        let mut constant = constant.div(pivot)?;
        linear::settle(&mut constant);
        for term in &mut terms {
            *term = term.div(pivot)?;
            linear::settle(term);
        }
        let value = Affine { constant, terms };
        values[column] = Some(value.clone());
        back.push(Back { row, column, value });
    }
    Ok(back)
}

fn gauss_solution(echelon: &Echelon, back: &[Back], unknowns: &[String]) -> Solution {
    let n = unknowns.len();
    if echelon.pivots.len() == n {
        let mut values = vec![Value::zero(); n];
        for b in back {
            values[b.column] = b.value.constant.clone();
        }
        return Solution::Unique(values);
    }
    let free: Vec<usize> = (0..n).filter(|c| !echelon.pivots.contains(c)).collect();
    let mut bound: Vec<_> = back
        .iter()
        .map(|b| {
            // x = c + Σ k·p is x - Σ k·p = c.
            let terms = free.iter().filter(|&&f| !b.value.terms[f].is_zero()).map(|&f| (f, b.value.terms[f].neg())).collect();
            (b.column, b.value.constant.clone(), terms)
        })
        .collect();
    bound.sort_by_key(|(column, ..)| *column);
    Solution::Parametric { parameters: parameter_names(unknowns, free.len()), free, bound }
}

/// Greek letters for the free unknowns, `λ, μ, ν, …`, skipping any the system uses as an
/// unknown; past the list, `λ_1, λ_2, …`.
pub(crate) fn parameter_names(unknowns: &[String], count: usize) -> Vec<String> {
    const GREEK: [&str; 8] = ["\\lambda", "\\mu", "\\nu", "\\rho", "\\sigma", "\\tau", "\\kappa", "\\eta"];
    let numbered = (1..).map(|k| format!("\\lambda_{k}"));
    GREEK
        .iter()
        .map(|name| name.to_string())
        .chain(numbered)
        .filter(|name| !unknowns.contains(name))
        .take(count)
        .collect()
}

/// The sort key of an unknown, as textbooks write systems: `x, y, z`, then `a … w`, then Greek
/// letters and longer names alphabetically. A subscript sorts after the bare letter, numbers by
/// value (`x_2` before `x_10`) and other subscripts after them.
pub(crate) fn variable_order(key: &str) -> (u8, String, u8, u64, String) {
    let (name, index) = key.split_once('_').unwrap_or((key, ""));
    let group = match name.as_bytes() {
        [b'x' | b'y' | b'z'] => 0,
        [c] if c.is_ascii_alphabetic() => 1,
        _ => 2,
    };
    let (kind, number) = match index.parse::<u64>() {
        _ if index.is_empty() => (0, 0),
        Ok(number) => (1, number),
        Err(_) => (2, 0),
    };
    (group, name.to_string(), kind, number, index.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Variables;
    use crate::latex::parser::parse;

    fn solve_with(input: &str, method: Method) -> Result<SystemSolution> {
        let variables = Variables::new();
        Evaluator::new(&variables).solve_system(&parse(input)?.expression, method)
    }

    fn solve(input: &str) -> Result<SystemSolution> {
        solve_with(input, Method::Gauss)
    }

    fn unique(input: &str) -> Vec<Value> {
        match solve(input).unwrap().solution {
            Solution::Unique(values) => values,
            _ => panic!("{input} has no unique solution"),
        }
    }

    fn error(input: &str) -> String {
        solve(input).err().map(|e| e.to_string()).unwrap_or_default()
    }

    /// The systems of these tests that have a solution.
    const SYSTEMS: [&str; 14] = [
        "2x + y = 5; x - y = 1",
        "x + y + z = 6; x - y = 0; 2z = 6",
        "y = 2x; x + y = 3",
        "a + x = 3; a - x = 1",
        "b + a + y + x = 4; a = 1; b = 1; x - y = 0",
        "x_1 + x_2 = 3; x_1 - x_2 = 1",
        "\\frac{x}{2} + y = 1; x - y = \\frac{1}{2}",
        "\\pi x = \\pi; x + y = 3",
        "\\pi x + \\pi y = \\pi; x + y = 1",
        "\\pi x + \\pi y = 2\\pi; x - y = 0",
        "x + y = 1; 2x + 2y = 2",
        "x + z = 1; y - z = 2",
        "2x + 3y - z = 1; 4x + y + 2z = 3; 6x - 2y + z = 0",
        "y + z = 1; x + z = 2; x + y + 2z = 3",
    ];

    /// A solution as decimals, so `π⁻¹·π` and `1` compare equal.
    fn decimals(solution: &Solution) -> Vec<String> {
        let d = |v: &Value| v.decimal(30).unwrap().to_string();
        match solution {
            Solution::Unique(values) => values.iter().map(d).collect(),
            Solution::Parametric { free, bound, .. } => {
                let mut out = vec![format!("{free:?}")];
                for (column, constant, terms) in bound {
                    out.push(format!("{column}: {} {:?}", d(constant), terms.iter().map(|(f, k)| (f, d(k))).collect::<Vec<_>>()));
                }
                out
            }
            Solution::Inconsistent { row } => vec![format!("inconsistent {row}")],
        }
    }

    #[test]
    fn gauss_and_gauss_jordan_agree() {
        for input in SYSTEMS {
            let gauss = solve_with(input, Method::Gauss).unwrap().solution;
            let jordan = solve_with(input, Method::GaussJordan).unwrap().solution;
            assert_eq!(decimals(&gauss), decimals(&jordan), "{input}");
        }
    }

    #[test]
    fn back_substitution_goes_from_the_bottom_up() {
        let solution = solve("2x + y = 5; x - y = 1").unwrap();
        let Reduction::Gauss { echelon, back } = &solution.reduction else { panic!("Gauss is the default") };
        assert_eq!(echelon.pivots, [0, 1]);
        assert_eq!(back.iter().map(|b| (b.row, b.column)).collect::<Vec<_>>(), [(1, 1), (0, 0)]);
        assert_eq!(back[0].value.constant, Value::int(1));
        assert_eq!(back[1].value.constant, Value::int(2));
    }

    #[test]
    fn free_unknowns_become_greek_parameters() {
        let Solution::Parametric { parameters, .. } = solve("x + z = 1; y - z = 2").unwrap().solution else {
            panic!("expected infinitely many solutions");
        };
        assert_eq!(parameters, ["\\lambda"]);
        let unknowns = ["x".to_string(), "\\lambda".to_string(), "\\nu".to_string()];
        assert_eq!(parameter_names(&unknowns, 3), ["\\mu", "\\rho", "\\sigma"]);
        assert_eq!(parameter_names(&[], 10)[8..], ["\\lambda_1", "\\lambda_2"]);
    }

    #[test]
    fn two_and_three_unknowns() {
        assert_eq!(unique("2x + y = 5; x - y = 1"), [Value::int(2), Value::int(1)]);
        assert_eq!(unique("x + y + z = 6; x - y = 0; 2z = 6"), [Value::int(3).div(&Value::int(2)).unwrap(), Value::ratio(3, 2), Value::int(3)]);
    }

    #[test]
    fn both_syntaxes_give_the_same_system() {
        assert_eq!(unique("\\begin{cases} 2x + y = 5 \\\\ x - y = 1 \\end{cases}"), unique("2x + y = 5; x - y = 1"));
    }

    #[test]
    fn unknowns_are_sorted_and_may_stand_on_both_sides() {
        let solution = solve("y = 2x; x + y = 3").unwrap();
        assert_eq!(solution.unknowns, ["x", "y"]);
        assert_eq!(solution.matrix[0], [Value::int(-2), Value::int(1), Value::int(0)]);
        assert!(matches!(solution.solution, Solution::Unique(values) if values == [Value::int(1), Value::int(2)]));
    }

    #[test]
    fn unknowns_are_ordered_x_y_z_then_a_to_w() {
        assert_eq!(solve("a + x = 3; a - x = 1").unwrap().unknowns, ["x", "a"]);
        assert_eq!(solve("b + a + y + x = 4; a = 1; b = 1; x - y = 0").unwrap().unknowns, ["x", "y", "a", "b"]);
        let mut keys = ["\\alpha", "b", "x_10", "z", "x", "x_2", "x_i", "a"];
        keys.sort_by_cached_key(|key| variable_order(key));
        assert_eq!(keys, ["x", "x_2", "x_10", "x_i", "z", "a", "b", "\\alpha"]);
    }

    #[test]
    fn subscripted_variables_are_unknowns() {
        let solution = solve("x_1 + x_2 = 3; x_1 - x_2 = 1").unwrap();
        assert_eq!(solution.unknowns, ["x_1", "x_2"]);
        assert!(matches!(solution.solution, Solution::Unique(values) if values == [Value::int(2), Value::int(1)]));
        assert_eq!(solve("x_{10} + x_2 = 3; x_{10} - x_2 = 1").unwrap().unknowns, ["x_2", "x_10"]);
        assert_eq!(solve("b + x = 1; a + b = 2; x_1 + a = 3; x - x_1 = 0").unwrap().unknowns, ["x", "x_1", "a", "b"]);
    }

    #[test]
    fn fractional_and_symbolic_coefficients() {
        assert_eq!(unique("\\frac{x}{2} + y = 1; x - y = \\frac{1}{2}"), [Value::int(1), Value::ratio(1, 2)]);
        // π⁻¹·π does not fold to 1, but it is 1.
        let decimals: Vec<String> =
            unique("\\pi x = \\pi; x + y = 3").iter().map(|v| v.decimal(20).unwrap().to_string()).collect();
        assert_eq!(decimals, ["1", "2"]);
        // The same equation twice, once scaled by π: infinitely many solutions, not one.
        assert!(matches!(solve("\\pi x + \\pi y = \\pi; x + y = 1").unwrap().solution, Solution::Parametric { .. }));
    }

    #[test]
    fn session_variables_are_coefficients() {
        let mut variables = Variables::new();
        variables.insert("k_1".to_string(), PhysicalQuantity::dimensionless(Value::int(3)));
        let system = parse("k_1 x + y = 7; x - y = 1").unwrap().expression;
        let solution = Evaluator::new(&variables).solve_system(&system, Method::Gauss).unwrap();
        assert!(matches!(solution.solution, Solution::Unique(values) if values == [Value::int(2), Value::int(1)]));
    }

    #[test]
    fn no_solution_and_infinitely_many() {
        for method in [Method::Gauss, Method::GaussJordan] {
            let solution = solve_with("x + y = 1; 2x + 2y = 3", method).unwrap().solution;
            assert!(matches!(solution, Solution::Inconsistent { row: 1 }));
        }
        let Solution::Parametric { free, bound, .. } = solve("x + y = 1; 2x + 2y = 2").unwrap().solution else {
            panic!("expected infinitely many solutions");
        };
        assert_eq!(free, [1]);
        assert_eq!(bound, [(0, Value::int(1), vec![(1, Value::int(1))])]);
    }

    #[test]
    fn non_linear_systems_are_refused() {
        for input in ["x^2 + y = 3; x - y = 1", "xy = 2; x + y = 3", "|x| = 1; y = 2", "\\frac{1}{x} = 1; y = 2", "\\sin x = 0; y = 1"] {
            assert!(error(input).contains("not linear"), "{input}: {}", error(input));
        }
    }

    #[test]
    fn a_system_without_unknowns_is_an_error() {
        assert!(error("1 = 1; 2 = 2").contains("no unknown"));
    }
}
