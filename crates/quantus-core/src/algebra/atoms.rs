//! Non-polynomial parts in the rational pipeline.
//!
//! The pipeline only knows polynomials, so a root, a function, a constant like `\pi`, an absolute
//! value or a power with a non-integer exponent blocks every rule that touches it. Around the
//! pipeline each distinct such part is an atom, a variable of its own, and is put back into the
//! result, its steps and its domain afterwards. That alone collects `\sin x + \sin x = 2 \sin x`.
//!
//! Treating atoms as independent is always valid, if not always the simplest. What ties them
//! together (`\sqrt{u} \cdot \sqrt{u} = u`, `\ln a + \ln b = \ln(ab)`) are the identity rules,
//! which run on the atomized tree before each run of the pipeline. When one changes the
//! pipeline's result, it runs again.

use crate::algebra::domain::{extract_domain_restrictions, unwrap_group, DomainRestriction};
use crate::algebra::identities::{apply_rules, simplify_restrictions};
use crate::algebra::pipeline::{integer_exponent, unsupported_diagnostic};
use crate::algebra::poly::{Monomial, Poly, Term};
use crate::algebra::steps::{AlgebraStep, SimplificationResult};
use crate::settings::ConditionalIdentities;
use crate::latex::ast::Node;
use crate::latex::serializer;

/// The names of the atom variables; they cannot be typed, so they never clash with the input.
/// A polynomial orders its variables by name: constants (`#`) come before the letters, as in
/// `2 \pi x`, everything else (`◆`) after them, as in `x \sin(x)`.
const CONSTANT_ATOM: &str = "#";
const ATOM: &str = "◆";

/// Runs of the pipeline at most; each one after the first starts from a rewrite it could not see.
const MAX_ROUNDS: usize = 6;

/// The atoms of one simplification, in the order they were met. Each is stored atomized itself:
/// the atom for `\sqrt{\sin x}` holds the root of the atom for `\sin x`.
#[derive(Default)]
pub struct Atoms {
    nodes: Vec<Node>,
}

impl Atoms {
    /// The atom variable standing for `node`, a new one if `node` was not met before.
    pub fn intern(&mut self, node: Node) -> Node {
        let key = serializer::expression(&node);
        let index = match self.nodes.iter().position(|known| serializer::expression(known) == key) {
            Some(index) => index,
            None => {
                self.nodes.push(node);
                self.nodes.len() - 1
            }
        };
        self.var(index)
    }

    fn name(&self, index: usize) -> &'static str {
        if matches!(self.nodes[index], Node::Constant { .. }) {
            CONSTANT_ATOM
        } else {
            ATOM
        }
    }

    /// The variable standing for the atom with `index`.
    pub fn var(&self, index: usize) -> Node {
        Node::Variable { name: self.name(index).into(), index: (index + 1).to_string(), position: 0 }
    }

    /// How the atom with `index` is addressed; a polynomial rebuilds its variables from this key.
    pub fn key(&self, index: usize) -> String {
        Node::variable_key(self.name(index), &(index + 1).to_string())
    }

    /// Which atom a polynomial variable names, if any.
    pub fn index_of_key(&self, key: &str) -> Option<usize> {
        (0..self.nodes.len()).find(|&i| self.key(i) == key)
    }

    /// Which atom `node` is the variable of, if any.
    pub fn index_of(&self, node: &Node) -> Option<usize> {
        match unwrap_group(node) {
            Node::Variable { name, index, .. } => self.index_of_key(&Node::variable_key(name, index)),
            _ => None,
        }
    }

    /// The (atomized) part the atom with `index` stands for.
    pub fn node(&self, index: usize) -> &Node {
        &self.nodes[index]
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// `node` with every atom written out again, all the way down.
    pub fn restore(&self, node: Node) -> Node {
        if let Some(index) = self.index_of(&node) {
            return self.restore(self.nodes[index].clone());
        }
        node.map_children(&mut |child| self.restore(child))
    }

    /// The LaTeX of the part an atom stands for.
    pub fn latex(&self, index: usize) -> String {
        serializer::expression(&self.restore(self.var(index)))
    }

    /// An explanation names expressions in LaTeX; their atoms are written back, last first so
    /// that the 1st atom never matches inside the 10th.
    pub fn restore_text(&self, text: &str) -> String {
        (0..self.nodes.len()).rev().fold(text.to_string(), |text, index| {
            let latex = self.latex(index);
            text.replace(&serializer::expression(&self.var(index)), &latex).replace(&self.key(index), &latex)
        })
    }

    /// A restriction is a polynomial over variable names, so an atom in it is renamed to its LaTeX,
    /// written as in the result.
    fn restore_restriction(&self, restriction: DomainRestriction) -> DomainRestriction {
        let rename = |var: String| match self.index_of_key(&var) {
            Some(index) => serializer::algebraic_expression(&self.restore(self.var(index))),
            None => var,
        };
        let rename_poly = |poly: Poly| {
            Poly::from_terms(
                poly.terms
                    .into_iter()
                    .map(|term| {
                        let vars = term.monomial.vars.into_iter().map(|(var, exp)| (rename(var), exp)).collect();
                        Term::new(term.coeff, Monomial::from_map(vars))
                    })
                    .collect(),
            )
        };
        match restriction {
            DomainRestriction::VariableNotEqual { variable, value } => {
                DomainRestriction::VariableNotEqual { variable: rename(variable), value }
            }
            DomainRestriction::PolyNotZero(poly) => DomainRestriction::PolyNotZero(rename_poly(poly)),
            DomainRestriction::PolyNonNegative(poly) => DomainRestriction::PolyNonNegative(rename_poly(poly)),
            DomainRestriction::PolyPositive(poly) => DomainRestriction::PolyPositive(rename_poly(poly)),
        }
    }

    pub(crate) fn restore_step(&self, step: AlgebraStep) -> AlgebraStep {
        AlgebraStep {
            level: step.level,
            note: step.note,
            substeps: step.substeps.into_iter().map(|sub| self.restore_step(sub)).collect(),
            ..AlgebraStep::new(self.restore(step.before), self.restore(step.after), self.restore_text(&step.explanation), step.rule)
        }
    }
}

/// Whether the pipeline has to see `node` as an atom.
fn is_atom(node: &Node) -> bool {
    match node {
        Node::Root { .. } | Node::Function { .. } | Node::Abs { .. } | Node::Constant { .. } => true,
        Node::Power { exponent, .. } => integer_exponent(exponent).is_none(),
        _ => false,
    }
}

/// Constructs the pipeline can neither simplify nor hold as an atom; they are left as they are.
fn is_opaque(node: &Node) -> bool {
    matches!(
        node,
        Node::Matrix { .. }
            | Node::Determinant { .. }
            | Node::Equation { .. }
            | Node::Sum { .. }
            | Node::Integral { .. }
            | Node::Limit { .. }
            | Node::Derivative { .. }
    )
}

fn contains_atom(node: &Node) -> bool {
    !is_opaque(node) && (is_atom(node) || node.children().into_iter().any(contains_atom))
}

/// `node` with each atom replaced by its variable, innermost first.
pub fn atomize(node: Node, atoms: &mut Atoms) -> Node {
    if is_opaque(&node) {
        return node;
    }
    let node = node.map_children(&mut |child| atomize(child, atoms));
    if is_atom(&node) {
        atoms.intern(node)
    } else {
        node
    }
}

/// Runs `simplify` on `node` with its non-polynomial parts held as atoms, or returns `None` when
/// `node` has none.
pub fn simplify_with_atoms(
    node: &Node,
    policy: ConditionalIdentities,
    mut simplify: impl FnMut(&Node) -> crate::Result<SimplificationResult>,
) -> Option<crate::Result<SimplificationResult>> {
    if !contains_atom(node) {
        return None;
    }
    let mut atoms = Atoms::default();
    let mut current = atomize(node.clone(), &mut atoms);
    let mut steps = Vec::new();
    let mut restrictions: Vec<DomainRestriction> = Vec::new();
    let mut diagnostic = None;

    for round in 0..MAX_ROUNDS {
        // An identity may remove a denominator before the pipeline has seen it (\log_a(a^{2}) = 2),
        // so the denominators of every state the rules start from are part of the domain.
        for restriction in extract_domain_restrictions(&current).unwrap_or_default() {
            if !restrictions.contains(&restriction) {
                restrictions.push(restriction);
            }
        }
        let rewritten = apply_rules(&mut current, &mut atoms, policy, &mut restrictions, &mut steps);
        // After the first run, the pipeline's result is final unless an identity changed it.
        if round > 0 && !rewritten {
            break;
        }
        let result = match simplify(&current) {
            Ok(result) => result,
            Err(error) => return Some(Err(error)),
        };
        steps.extend(result.steps);
        for restriction in result.domain_restrictions {
            if !restrictions.contains(&restriction) {
                restrictions.push(restriction);
            }
        }
        diagnostic = result.diagnostic;
        current = result.node;
    }

    // A step that only changed how an atom was written looks like no step once it is written out.
    let steps =
        steps.into_iter().map(|step| atoms.restore_step(step)).filter(|step| step.note || step.before_latex != step.after_latex).collect();
    let restrictions = simplify_restrictions(restrictions, &atoms).into_iter().map(|r| atoms.restore_restriction(r)).collect();
    let mut restored = SimplificationResult::new(atoms.restore(current), steps, restrictions);
    restored.diagnostic = diagnostic.map(|text| atoms.restore_text(&text)).or_else(|| unsupported_diagnostic(node));
    Some(Ok(restored))
}
