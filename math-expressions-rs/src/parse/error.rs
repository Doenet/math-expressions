//! Parse errors carry a byte offset into the input (PORTING_PLAN.md §6).
//! Error message text matches the JS ParseError messages exactly — the
//! error fixtures assert on it.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub location: usize,
}

impl ParseError {
    pub fn new(message: impl Into<String>, location: usize) -> Self {
        ParseError {
            message: message.into(),
            location,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at {})", self.message, self.location)
    }
}

impl std::error::Error for ParseError {}
