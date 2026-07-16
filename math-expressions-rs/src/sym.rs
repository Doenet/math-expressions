//! Symbol interning (PORTING_PLAN.md §4).
//!
//! Symbols are u32 indices into a thread-local interner; comparison and
//! hashing are O(1). WASM is single-threaded, so thread_local is free.

use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sym(u32);

thread_local! {
    static INTERNER: RefCell<Interner> = RefCell::new(Interner::default());
}

#[derive(Default)]
struct Interner {
    by_name: HashMap<String, u32>,
    names: Vec<String>,
}

impl Sym {
    pub fn new(name: &str) -> Sym {
        INTERNER.with(|i| {
            let mut i = i.borrow_mut();
            if let Some(&id) = i.by_name.get(name) {
                return Sym(id);
            }
            let id = i.names.len() as u32;
            i.names.push(name.to_string());
            i.by_name.insert(name.to_string(), id);
            Sym(id)
        })
    }

    /// The symbol's name. Returns an owned String because the interner is
    /// thread-local; symbol-heavy code paths should compare `Sym`s directly.
    pub fn name(self) -> String {
        INTERNER.with(|i| i.borrow().names[self.0 as usize].clone())
    }
}

impl std::fmt::Display for Sym {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
