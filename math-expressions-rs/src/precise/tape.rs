//! Expression → evaluation tape (ARBITRARY_PERCISION_PLAN §3).
//!
//! The tape is a flat postorder (RPN) program over a value stack; every tier
//! evaluates it with a single loop, so evaluation depth is O(1) in the
//! expression's tree depth. Compilation itself is an **iterative** postorder
//! walk (explicit work stack — requirement 1 of the plan: no recursion
//! anywhere on the evaluation path).

use crate::expr::{Expr, MathConst};
use crate::num::Number;

use super::kernels;

#[derive(Clone, Debug)]
pub enum Op {
    /// Push `consts[i]`.
    Const(u32),
    /// Push the binding for variable slot `i`.
    Var(u32),
    Pi,
    E,
    /// The imaginary unit (complex tier only; the real tiers escalate).
    I,
    /// Pop n, push their sum.
    Add(u32),
    /// Pop n, push their product.
    Mul(u32),
    /// Pop base, push base^k (integer k, possibly negative).
    PowInt(i64),
    /// Pop exponent then base, push base^exponent (general real power).
    Pow,
    /// Pop the argument, push `REGISTRY[id](arg)`.
    Call(u32),
    /// Push `roots[i]`: an abstract algebraic number (MATRIX_PLAN §2a leaf),
    /// refined per tier via the certified Newton machinery in `rootof`.
    Root(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// A non-numeric shape (relation, matrix, blank, …) or unknown function.
    NotNumeric(&'static str),
    /// ∞/NaN constant in the tree.
    NotFinite,
    /// Tape length exceeded `limits.max_tape_ops`.
    TooLarge,
}

pub struct CompiledExpr {
    pub(super) ops: Vec<Op>,
    pub(super) consts: Vec<Number>,
    pub(super) vars: Vec<String>,
    pub(super) roots: Vec<(Box<[Number]>, u32)>,
    pub(super) max_stack: usize,
}

impl CompiledExpr {
    pub fn vars(&self) -> &[String] {
        &self.vars
    }
    pub fn len(&self) -> usize {
        self.ops.len()
    }
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

/// How many values an op pops from the stack.
pub(super) fn arity(op: &Op) -> usize {
    match op {
        Op::Const(_) | Op::Var(_) | Op::Pi | Op::E | Op::I | Op::Root(_) => 0,
        Op::Add(n) | Op::Mul(n) => *n as usize,
        Op::PowInt(_) | Op::Call(_) => 1,
        Op::Pow => 2,
    }
}

/// Compile a **canonical** tree (no `Div`/`Neg`; flat `Add`/`Mul`) into a
/// tape. Iterative: an explicit visit/emit stack, never recursion.
pub fn compile(e: &Expr) -> Result<CompiledExpr, CompileError> {
    enum Task<'a> {
        Visit(&'a Expr),
        Emit(&'a Expr),
    }

    let mut ops: Vec<Op> = Vec::new();
    let mut consts: Vec<Number> = Vec::new();
    let mut vars: Vec<String> = Vec::new();
    let mut roots: Vec<(Box<[Number]>, u32)> = Vec::new();
    let max_ops = crate::limits::current().max_tape_ops;

    let mut stack = vec![Task::Visit(e)];
    while let Some(task) = stack.pop() {
        let node = match task {
            Task::Emit(n) => {
                emit(n, &mut ops, &mut consts, &mut vars, &mut roots)?;
                if ops.len() > max_ops {
                    return Err(CompileError::TooLarge);
                }
                continue;
            }
            Task::Visit(n) => n,
        };
        stack.push(Task::Emit(node));
        // Children pushed in reverse so they evaluate left-to-right.
        match node {
            Expr::Add(ts) | Expr::Mul(ts) => {
                for t in ts.iter().rev() {
                    stack.push(Task::Visit(t));
                }
            }
            Expr::Pow(b, x) => {
                // Integer exponents fold into the op itself (no child).
                if !matches!(&**x, Expr::Num(Number::Int(_))) {
                    stack.push(Task::Visit(x));
                }
                stack.push(Task::Visit(b));
            }
            Expr::Apply(head, args) => {
                let Expr::Sym(_) = &**head else {
                    return Err(CompileError::NotNumeric("non-symbol function head"));
                };
                let [arg] = args.as_slice() else {
                    return Err(CompileError::NotNumeric("multi-argument function"));
                };
                stack.push(Task::Visit(arg));
            }
            Expr::Num(_) | Expr::Sym(_) | Expr::Const(_) | Expr::RootOf { .. } => {}
            _ => return Err(CompileError::NotNumeric("non-numeric expression shape")),
        }
    }

    // Stack-height simulation: verifies well-formedness and sizes the value
    // stacks once for every later evaluation.
    let mut height = 0usize;
    let mut max_stack = 0usize;
    for op in &ops {
        let a = arity(op);
        debug_assert!(height >= a, "malformed tape");
        height = height - a + 1;
        max_stack = max_stack.max(height);
    }
    debug_assert_eq!(height, 1, "tape must leave exactly one value");

    Ok(CompiledExpr {
        ops,
        consts,
        vars,
        roots,
        max_stack,
    })
}

fn emit(
    node: &Expr,
    ops: &mut Vec<Op>,
    consts: &mut Vec<Number>,
    vars: &mut Vec<String>,
    roots: &mut Vec<(Box<[Number]>, u32)>,
) -> Result<(), CompileError> {
    match node {
        Expr::RootOf { poly, index } => {
            let id = match roots.iter().position(|(p, i)| p == poly && i == index) {
                Some(i) => i,
                None => {
                    roots.push((poly.clone(), *index));
                    roots.len() - 1
                }
            };
            ops.push(Op::Root(id as u32));
            return Ok(());
        }
        Expr::Num(n) => {
            if matches!(n, Number::Float(f) if !f.get().is_finite()) {
                return Err(CompileError::NotFinite);
            }
            consts.push(n.clone());
            ops.push(Op::Const((consts.len() - 1) as u32));
        }
        Expr::Const(MathConst::Pi) => ops.push(Op::Pi),
        Expr::Const(MathConst::E) => ops.push(Op::E),
        Expr::Const(MathConst::I) => ops.push(Op::I),
        Expr::Const(_) => return Err(CompileError::NotFinite),
        Expr::Sym(s) => {
            let name = s.name();
            match name.as_str() {
                "pi" => ops.push(Op::Pi),
                "e" => ops.push(Op::E),
                "i" => ops.push(Op::I),
                _ => {
                    let slot = match vars.iter().position(|v| *v == name) {
                        Some(i) => i,
                        None => {
                            vars.push(name);
                            vars.len() - 1
                        }
                    };
                    ops.push(Op::Var(slot as u32));
                }
            }
        }
        Expr::Add(ts) => ops.push(Op::Add(ts.len() as u32)),
        Expr::Mul(ts) => ops.push(Op::Mul(ts.len() as u32)),
        Expr::Pow(_, x) => match &**x {
            Expr::Num(Number::Int(k)) => ops.push(Op::PowInt(*k)),
            _ => ops.push(Op::Pow),
        },
        Expr::Apply(head, _) => {
            let Expr::Sym(f) = &**head else { unreachable!() };
            let Some(id) = kernels::lookup(&f.name()) else {
                return Err(CompileError::NotNumeric("unknown function"));
            };
            ops.push(Op::Call(id));
        }
        _ => unreachable!("filtered in Visit"),
    }
    Ok(())
}
