//! Systems of linear equations: `2x + y = 5; x - y = 1`.
//!
//! The unknowns are the free variables without a subscript, in alphabetical order. Each
//! equation's coefficients are read off by evaluating its residual `left - right` at zero and at
//! each unit vector, as the single-equation solver does for polynomials, and checked at other
//! points: a residual that misses them is not linear. That also covers `\pi x`, fractions and
//! session variables as coefficients. The augmented matrix is then reduced exactly, so the
//! system has one solution, none, or infinitely many, with no tolerance in between.

use super::evaluator::Evaluator;
use super::solver::collect_unknowns;
use crate::latex::Node;
use crate::linear::{self, Elimination, Outcome};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::{QuantusError, Result};

/// A solved system: the unknowns, the augmented matrix and its reduction.
pub(crate) struct SystemSolution {
    pub unknowns: Vec<String>,
    /// `[A | b]` as read from the equations, one row per equation.
    pub matrix: Vec<Vec<Value>>,
    pub elimination: Elimination,
    pub solution: Solution,
}

pub(crate) enum Solution {
    /// The value of each unknown.
    Unique(Vec<Value>),
    /// Each unknown with a pivot as a constant minus multiples of the free unknowns, which
    /// can take any value: `x = c - Σ k·y`, given as `(x, c, [(y, k)])` by index.
    Parametric { free: Vec<usize>, bound: Vec<(usize, Value, Vec<(usize, Value)>)> },
}

impl Evaluator<'_> {
    pub(crate) fn solve_system(&self, system: &Node) -> Result<SystemSolution> {
        let Node::System { equations, .. } = system else {
            return Err(QuantusError::other("Only a system of equations can be solved as one."));
        };
        let mut unknowns = Vec::new();
        collect_unknowns(system, &mut Vec::new(), self, &mut unknowns);
        if unknowns.is_empty() {
            return Err(QuantusError::other(
                "The system has no unknown to solve for. Use letters such as x and y without a value.",
            ));
        }
        unknowns.sort();

        let mut matrix = Vec::new();
        for equation in equations {
            matrix.push(self.coefficients(equation, &unknowns)?);
        }
        let n = unknowns.len();
        let elimination = linear::eliminate(matrix.clone(), n, true)?;
        let solution = match elimination.outcome() {
            Outcome::Unique(values) => Solution::Unique(values),
            Outcome::Inconsistent { .. } => {
                return Err(QuantusError::other("The system has no solution: its equations contradict each other."));
            }
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
                Solution::Parametric { free, bound }
            }
        };
        Ok(SystemSolution { unknowns, matrix, elimination, solution })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Variables;
    use crate::latex::parser::parse;

    fn solve(input: &str) -> Result<SystemSolution> {
        let variables = Variables::new();
        Evaluator::new(&variables).solve_system(&parse(input)?.expression)
    }

    fn unique(input: &str) -> Vec<Value> {
        match solve(input).unwrap().solution {
            Solution::Unique(values) => values,
            Solution::Parametric { .. } => panic!("{input} has infinitely many solutions"),
        }
    }

    fn error(input: &str) -> String {
        solve(input).err().map(|e| e.to_string()).unwrap_or_default()
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
        let solution = Evaluator::new(&variables).solve_system(&system).unwrap();
        assert!(matches!(solution.solution, Solution::Unique(values) if values == [Value::int(2), Value::int(1)]));
    }

    #[test]
    fn no_solution_and_infinitely_many() {
        assert!(error("x + y = 1; 2x + 2y = 3").contains("no solution"));
        let Solution::Parametric { free, bound } = solve("x + y = 1; 2x + 2y = 2").unwrap().solution else {
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
