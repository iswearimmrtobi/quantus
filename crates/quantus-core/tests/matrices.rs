use quantus_core::api::{self, CalculationRequest};
use quantus_core::session::SessionStore;
use quantus_core::settings::{Mode, Settings, StepDetail};

fn store() -> SessionStore {
    let mut store = SessionStore::new();
    api::set_settings(&mut store, Settings { mode: Mode::Matrix, step_detail: StepDetail::Standard, ..Settings::default() }).unwrap();
    store
}

fn run(store: &mut SessionStore, latex: &str) -> String {
    let request = CalculationRequest { latex: latex.into(), ..Default::default() };
    api::evaluate(store, &request).map(|r| r.result).unwrap_or_else(|e| format!("error: {e}"))
}

#[test]
fn matrices_are_values_with_names() {
    let mut store = store();
    assert_eq!(run(&mut store, r"A = \begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}"), r"\begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}");
    run(&mut store, r"B = \begin{bmatrix} 0 & 1 \\ 1 & 0 \end{bmatrix}");
    assert_eq!(run(&mut store, "A + B"), r"\begin{pmatrix} 1 & 3 \\ 4 & 4 \end{pmatrix}");
    assert_eq!(run(&mut store, r"A \cdot B"), r"\begin{pmatrix} 2 & 1 \\ 4 & 3 \end{pmatrix}");
    assert_eq!(run(&mut store, "2A - A"), r"\begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}");
    assert_eq!(run(&mut store, "A^{T}"), r"\begin{pmatrix} 1 & 3 \\ 2 & 4 \end{pmatrix}");
    assert_eq!(run(&mut store, r"A^{\top}"), r"\begin{pmatrix} 1 & 3 \\ 2 & 4 \end{pmatrix}");
    assert_eq!(run(&mut store, "A^{2}"), r"\begin{pmatrix} 7 & 10 \\ 15 & 22 \end{pmatrix}");
    assert_eq!(run(&mut store, r"\det A"), "-2");
}

#[test]
fn inverse_identity_and_echelon_forms_are_exact() {
    let mut store = store();
    run(&mut store, r"M_1 = \begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}");
    assert_eq!(run(&mut store, "M_1^{-1}"), r"\begin{pmatrix} -2 & 1 \\ \frac{3}{2} & -\frac{1}{2} \end{pmatrix}");
    assert_eq!(run(&mut store, r"M_1 \cdot M_1^{-1}"), r"\begin{pmatrix} 1 & 0 \\ 0 & 1 \end{pmatrix}");
    assert_eq!(run(&mut store, r"\operatorname{Identity}(2)"), r"\begin{pmatrix} 1 & 0 \\ 0 & 1 \end{pmatrix}");
    assert_eq!(
        run(&mut store, r"\operatorname{rref}(\begin{pmatrix} 1 & 2 & 3 \\ 2 & 4 & 7 \end{pmatrix})"),
        r"\begin{pmatrix} 1 & 2 & 0 \\ 0 & 0 & 1 \end{pmatrix}"
    );
    // The school method: row 2 becomes row 2 - 3 row 1, then its common factor 2 is divided out.
    assert_eq!(run(&mut store, r"\operatorname{ref}(M_1)"), r"\begin{pmatrix} 1 & 2 \\ 0 & -1 \end{pmatrix}");
}

#[test]
fn sizes_and_singular_matrices_are_errors() {
    let mut store = store();
    run(&mut store, r"A = \begin{pmatrix} 1 & 2 \\ 2 & 4 \end{pmatrix}");
    assert!(run(&mut store, "A^{-1}").contains("singular"));
    assert!(run(&mut store, r"A + \begin{pmatrix} 1 \end{pmatrix}").contains("same size"));
    assert!(run(&mut store, "A + 1").starts_with("error"));
}

fn steps(latex: &str, detail: StepDetail) -> Vec<(String, String)> {
    let mut store = SessionStore::new();
    api::set_settings(&mut store, Settings { mode: Mode::Matrix, step_detail: detail, ..Settings::default() }).unwrap();
    let request = CalculationRequest { latex: latex.into(), ..Default::default() };
    api::evaluate(&mut store, &request)
        .unwrap()
        .steps
        .into_iter()
        .map(|step| (step.latex, step.explanation.unwrap_or_default()))
        .collect()
}

#[test]
fn the_inverse_shows_the_elimination_of_a_beside_i() {
    let steps = steps(r"\begin{pmatrix} 2 & 1 \\ 1 & 1 \end{pmatrix}^{-1}", StepDetail::Standard);
    // Symbolic, the stages, the end form, the result.
    assert!(steps.len() >= 4, "{steps:?}");
    assert!(steps[1].0.contains("{cc|cc|l}"), "{}", steps[1].0);
    assert!(steps.iter().any(|(_, text)| text.contains("right half is the inverse")));
    assert!(steps.last().unwrap().0.ends_with(r"\begin{pmatrix} 1 & -1 \\ -1 & 2 \end{pmatrix}"), "{:?}", steps.last());
}

#[test]
fn the_determinant_is_the_product_of_the_pivots() {
    let steps = steps(r"\det\begin{pmatrix} 0 & 2 \\ 3 & 1 \end{pmatrix}", StepDetail::Brief);
    let product = steps.iter().find(|(latex, _)| latex.starts_with(r"\det =")).unwrap();
    assert_eq!(product.0, r"\det = (-1) \cdot 3 \cdot 2 = -6");
}

#[test]
fn the_matrices_are_listed_in_their_order_and_deleted_like_a_variable() {
    let mut store = store();
    run(&mut store, r"B = \begin{pmatrix} 1 & 2 \end{pmatrix}");
    run(&mut store, r"A = \begin{pmatrix} 1 \\ 2 \end{pmatrix}");
    let keys = |store: &mut SessionStore| store.session(None).matrix_definitions().iter().map(|(key, _)| key.clone()).collect::<Vec<_>>();
    assert_eq!(keys(&mut store), ["B", "A"]);
    assert_eq!(store.session(None).matrix_definitions()[0].1, r"\begin{pmatrix} 1 & 2 \end{pmatrix}");
    api::delete_variable(&mut store, "B", None);
    assert_eq!(keys(&mut store), ["A"]);
    assert!(run(&mut store, "2B").starts_with("error"), "B is gone");
}
