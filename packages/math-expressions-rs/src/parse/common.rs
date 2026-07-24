//! Helpers shared by the text and LaTeX recursive descent parsers. Both JS
//! source files (`text-to-ast.js`, `latex-to-ast.js`) duplicate these exact
//! routines; here they live once. Everything is pure — no parser state.

use crate::expr::{Expr, MathConst};
use crate::sym::Sym;

// `statement` must be among the counted frames: its bar-fallback catches
// errors, rewinds, and re-descends, so budget freed by the failed descent's
// unwind would otherwise be re-spent each retry; `statement`'s own increment
// is held across both attempts and bounds the total.
//
// Sizing (measured, debug native): recursive descent costs ~4 budget units
// and ~40 KB of stack per bracket level. A 1 MB stack overflows near 25
// bracket levels in debug and ~60 in release; this cap fires at ~16 levels,
// which fits 1 MB in *both* profiles with margin. The entire fixture corpus
// nests at most 2 deep, so ~16 is ~8× real educational input. Lower than
// serde_json's 128 because our per-level frame is ~4× heavier; the real fix
// for a higher ceiling is an iterative parser (deferred, PORTING_PLAN.md §6e).
/// Maximum recursion budget for the parsers. Counts frames through the
/// self-recursive functions (`statement`, `relation`, `expression`,
/// `factor`, `base_factor`) — the ones untrusted input can drive to
/// unbounded stack depth via nesting (`((…))`) or prefix chains (`----x`,
/// `!!!!x`). Input that exceeds the cap yields a "too deeply nested"
/// `ParseError`, never a stack-overflow trap (which on wasm32 kills the
/// whole instance).
pub const MAX_PARSE_DEPTH: usize = 64;

/// Parse-time parameters, mirroring the JS options objects that are
/// re-destructured with defaults at every call site.
#[derive(Debug, Clone, Copy)]
pub struct P {
    pub inside_absolute_value: u32,
    pub parse_absolute_value: bool,
    pub allow_absolute_value_closing: bool,
    pub in_subsuperscript: bool,
}

impl Default for P {
    fn default() -> P {
        P {
            inside_absolute_value: 0,
            parse_absolute_value: true,
            allow_absolute_value_closing: false,
            in_subsuperscript: false,
        }
    }
}

/// JS: `typeof e === "string" && [...e].every(c => "+-".includes(c))`.
/// Sign-symbols are Syms whose name is entirely +/- characters.
pub fn sign_string(e: &Expr) -> Option<String> {
    if let Expr::Sym(s) = e {
        let name = s.name();
        if !name.is_empty() && name.chars().all(|c| c == '+' || c == '-') {
            return Some(name);
        }
    }
    None
}

/// JS: `typeof e === "number" || typeof e === "string"` with JS
/// stringification. The blank ("＿") and Infinity are a string/number in JS,
/// so they participate.
pub fn atom_string(e: &Expr) -> Option<String> {
    match e {
        Expr::Num(n) => Some(n.js_string()),
        Expr::Sym(s) => Some(s.name()),
        Expr::Blank => Some("\u{ff3f}".to_string()),
        Expr::Const(MathConst::Inf) => Some("Infinity".to_string()),
        Expr::Const(MathConst::NegInf) => Some("-Infinity".to_string()),
        _ => None,
    }
}

/// JS: `e > 0` (numbers and Infinity only; everything else coerces false).
pub fn is_positive_number(e: &Expr) -> bool {
    match e {
        Expr::Num(n) => n.is_positive(),
        Expr::Const(MathConst::Inf) => true,
        _ => false,
    }
}

/// JS unary numeric negation `-e` for the `e > 0` case.
pub fn negate_number(e: Expr) -> Expr {
    match e {
        Expr::Num(n) => Expr::Num(n.neg()),
        Expr::Const(MathConst::Inf) => Expr::Const(MathConst::NegInf),
        _ => unreachable!("negate_number only called when is_positive_number"),
    }
}

/// parseFloat for a NUMBER token (Rust's f64 parser rejects "1.E3" which
/// parseFloat accepts).
pub fn parse_js_float(text: &str) -> f64 {
    let t = text.trim();
    if let Ok(v) = t.parse::<f64>() {
        return v;
    }
    let fixed = t.replace(".E", "E").replace(".e", "e");
    if let Ok(v) = fixed.parse::<f64>() {
        return v;
    }
    t.trim_end_matches('.').parse::<f64>().unwrap_or(f64::NAN)
}

/// Build a generic `OtherOp` node from a string operator name.
pub fn other_op(name: &str, args: Vec<Expr>) -> Expr {
    Expr::OtherOp(Sym::new(name), args)
}
