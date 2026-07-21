//! Matrix and vector operations (item 12) and the eigen-decomposition surface
//! (MATRIX_PLAN §1b–§3).

use super::Expression;
use math_expressions::{to_text, Assumptions, Expr};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Expression {
    pub fn transpose(&self) -> Expression {
        Expression(math_expressions::transpose(&self.0))
    }
    pub fn trace(&self) -> Expression {
        Expression(math_expressions::trace(&self.0))
    }
    pub fn matmul(&self, other: &Expression) -> Expression {
        Expression(math_expressions::matmul(&self.0, &other.0))
    }
    /// Matrix inverse (opaque when singular or symbolic without a nonzero-det
    /// proof under the default assumptions).
    pub fn matrix_inverse(&self) -> Expression {
        Expression(math_expressions::matrix_inverse(&self.0, &Assumptions::new()))
    }
    /// Reduced row-echelon form.
    pub fn rref(&self) -> Expression {
        Expression(math_expressions::rref(&self.0, &Assumptions::new()))
    }
    /// Matrix rank, or `undefined` when not a literal matrix under the caps.
    pub fn rank(&self) -> Option<u32> {
        math_expressions::rank(&self.0, &Assumptions::new())
    }
    /// A basis for the null space as a JSON array of column vectors (each an
    /// array of text-syntax entries), or `undefined` on refusal.
    pub fn nullspace(&self) -> Option<String> {
        let basis = math_expressions::nullspace(&self.0, &Assumptions::new())?;
        let rows: Vec<serde_json::Value> = basis
            .iter()
            .map(|v| {
                let entries = match v {
                    Expr::Matrix { entries, .. } => entries.clone(),
                    other => vec![other.clone()],
                };
                serde_json::Value::Array(
                    entries
                        .iter()
                        .map(|e| serde_json::json!(to_text(e, &Default::default())))
                        .collect(),
                )
            })
            .collect();
        Some(serde_json::Value::Array(rows).to_string())
    }

    pub fn vector_add(&self, other: &Expression) -> Expression {
        Expression(math_expressions::vector_add(&self.0, &other.0))
    }
    pub fn vector_sub(&self, other: &Expression) -> Expression {
        Expression(math_expressions::vector_sub(&self.0, &other.0))
    }
    pub fn dot_prod(&self, other: &Expression) -> Expression {
        Expression(math_expressions::dot_prod(&self.0, &other.0))
    }
    pub fn cross_prod(&self, other: &Expression) -> Expression {
        Expression(math_expressions::cross_prod(&self.0, &other.0))
    }

    /// Determinant (tiered — MATRIX_PLAN §1b). Always an expression: an
    /// opaque `det(…)` node when not decidable.
    pub fn determinant(&self) -> Expression {
        Expression(math_expressions::matrix::det(&self.0))
    }

    /// Characteristic polynomial in `var`, or `undefined` when the receiver
    /// is not a square literal matrix under the caps.
    pub fn char_poly(&self, var: &str) -> Option<Expression> {
        math_expressions::matrix::char_poly(&self.0, var).map(Expression)
    }

    /// Eigenvalues with algebraic multiplicities as JSON
    /// `[{"value": <text>, "multiplicity": n}, …]` in canonical order
    /// (MATRIX_PLAN §2c), or `undefined` on an honest refusal.
    pub fn eigenvalues(&self) -> Option<String> {
        let vals = math_expressions::matrix::eigenvalues(&self.0, &Assumptions::new())?;
        let rows: Vec<serde_json::Value> = vals
            .iter()
            .map(|(v, m)| {
                serde_json::json!({
                    "value": to_text(v, &Default::default()),
                    "multiplicity": m,
                })
            })
            .collect();
        Some(serde_json::Value::Array(rows).to_string())
    }

    /// Eigenvectors as JSON `[{"value", "multiplicity", "basis": [[…]]}, …]`
    /// (basis entries in text syntax; geometric multiplicity is the basis
    /// length), or `undefined` on an honest refusal (MATRIX_PLAN §3).
    pub fn eigenvectors(&self) -> Option<String> {
        let pairs = math_expressions::matrix::eigenvectors(&self.0, &Assumptions::new())?;
        let rows: Vec<serde_json::Value> = pairs
            .iter()
            .map(|p| {
                let basis: Vec<Vec<String>> = p
                    .basis
                    .iter()
                    .map(|v| v.iter().map(|e| to_text(e, &Default::default())).collect())
                    .collect();
                serde_json::json!({
                    "value": to_text(&p.value, &Default::default()),
                    "multiplicity": p.alg_mult,
                    "basis": basis,
                })
            })
            .collect();
        Some(serde_json::Value::Array(rows).to_string())
    }
}
