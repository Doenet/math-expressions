//! Internationalized number notation.
//!
//! The decimal and argument/tuple/list separators are an **explicit, declared**
//! input to every parse and print — never inferred from the string, because
//! under comma-decimal notation the argument separator moves to `;` and the two
//! meanings of `,` would otherwise collide (issue Doenet/DoenetML#1528).
//!
//! At [`NumberNotation::default`] the decimal is `.`, the argument separator is
//! `,`, and digits are Latin — byte-identical to the pre-i18n behavior, so
//! every existing fixture stays unchanged.
//!
//! The implemented notation covers `.`/`,` decimals paired with `,`/`;`
//! separators in Latin digits, and — because the lexer/printer are
//! char-generic — the Arabic `٫`/`؛` separators come along for free. The
//! [`Grouping`] / [`Digits`] fields are **present but not yet acted on**:
//! digit-set translation and thousands grouping are stubs so the option/JSON
//! shape is already stable.

use std::borrow::Cow;

/// Thousands-grouping style (not yet applied to output).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Grouping {
    /// No digit grouping (the default; today's behavior).
    #[default]
    None,
    /// Western groups of three (`1,000,000`).
    Western,
    /// Indian 3-2-2 grouping (`10,00,000`).
    Indian,
}

/// Digit set for scanning and emission (only [`Digits::Latin`] is acted on
/// today; the others are accepted and reserved).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Digits {
    /// Latin `0-9` (the default).
    #[default]
    Latin,
    /// Arabic-Indic `٠-٩`.
    Arabic,
    /// Devanagari `०-९`.
    Devanagari,
}

/// A declared number notation, shared by the parse-option and print-option
/// structs. Cheap to clone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberNotation {
    /// The decimal separator to scan and to emit (`.` | `,` | `٫`).
    pub decimal_separator: char,
    /// The argument/tuple/list/set separator (`,` | `;` | `؛`). Lexes to the
    /// same token the grammar reads regardless of glyph.
    pub argument_separator: char,
    /// Extra decimal separators also *accepted on input* (never emitted).
    /// `None` = strict: only [`Self::decimal_separator`] is a decimal.
    pub also_accept_decimal: Option<Vec<char>>,
    // ---- design-for (Phase 2; Latin/none defaults keep the A2 no-op) ----
    /// Thousands group separator accepted on input / emitted (not yet acted on).
    pub group_separator: Option<char>,
    /// Grouping style for emission (not yet acted on).
    pub grouping: Grouping,
    /// Digit set for scanning/emission (not yet acted on).
    pub digits: Digits,
}

impl Default for NumberNotation {
    /// `.` decimal, `,` argument separator, Latin digits, no grouping — the
    /// pre-i18n behavior, exactly.
    fn default() -> Self {
        NumberNotation {
            decimal_separator: '.',
            argument_separator: ',',
            also_accept_decimal: None,
            group_separator: None,
            grouping: Grouping::None,
            digits: Digits::Latin,
        }
    }
}

impl NumberNotation {
    /// The default period-decimal notation (`1.2`, `f(x,y)`).
    pub fn period() -> Self {
        NumberNotation::default()
    }

    /// Comma-decimal notation (`1,2`, `f(x;y)`): decimal `,`, argument `;`.
    pub fn comma() -> Self {
        NumberNotation {
            decimal_separator: ',',
            argument_separator: ';',
            ..NumberNotation::default()
        }
    }

    /// The conventional argument separator paired with a decimal separator:
    /// `.` → `,` and `,` → `;` (the two special-cased pairs). `None` for any
    /// other decimal, which has no established convention here.
    pub fn paired_argument_separator(decimal: char) -> Option<char> {
        match decimal {
            '.' => Some(','),
            ',' => Some(';'),
            _ => None,
        }
    }

    /// Notation from a decimal separator alone, filling in the conventional
    /// argument separator ([`paired_argument_separator`]): `from_decimal('.')`
    /// is [`period`], `from_decimal(',')` is [`comma`]. For a decimal with no
    /// convention the argument separator stays the default `,` (which
    /// [`validate`] will reject if it collides with the decimal).
    ///
    /// [`paired_argument_separator`]: Self::paired_argument_separator
    /// [`period`]: Self::period
    /// [`comma`]: Self::comma
    /// [`validate`]: Self::validate
    pub fn from_decimal(decimal: char) -> Self {
        NumberNotation {
            decimal_separator: decimal,
            argument_separator: Self::paired_argument_separator(decimal).unwrap_or(','),
            ..NumberNotation::default()
        }
    }

    /// Every character accepted as a decimal separator on input (the primary
    /// plus any leniency chars), primary first.
    pub fn accepted_decimals(&self) -> Vec<char> {
        let mut v = vec![self.decimal_separator];
        if let Some(extra) = &self.also_accept_decimal {
            for c in extra {
                if !v.contains(c) {
                    v.push(*c);
                }
            }
        }
        v
    }

    /// Rewrite a matched NUMBER token into the canonical `.`-decimal form the
    /// number parsers expect: every accepted decimal char → `.`, and any group
    /// separator stripped. A no-op (borrowed) under the default notation.
    pub fn normalize_number<'a>(&self, text: &'a str) -> Cow<'a, str> {
        let decimals = self.accepted_decimals();
        let needs_decimal = decimals.iter().any(|&c| c != '.' && text.contains(c));
        let needs_group = self.group_separator.is_some_and(|g| text.contains(g));
        // The LaTeX printer wraps a decimal comma as `{,}` (A5); numbers never
        // otherwise contain braces, so stripping them here is safe.
        let needs_brace = text.contains(['{', '}']);
        if !needs_decimal && !needs_group && !needs_brace {
            return Cow::Borrowed(text);
        }
        let group = self.group_separator;
        let out: String = text
            .chars()
            .filter(|c| group != Some(*c) && *c != '{' && *c != '}')
            .map(|c| if decimals.contains(&c) { '.' } else { c })
            .collect();
        Cow::Owned(out)
    }

    /// Reject incoherent separator combinations. Because separators can be set
    /// independently (and defaults fill in the unspecified ones), a partial
    /// specification can leave, e.g., the decimal and argument separators both
    /// `,` — which parses ambiguously. Callers at a trust boundary (the wasm
    /// JSON entry points) should `validate()` before use; because notation is
    /// explicit and never inferred, we error rather than silently repair.
    pub fn validate(&self) -> Result<(), String> {
        /// Glyphs the lexers already assign meaning to. A separator set to one
        /// of these silently shadows that token: the argument-separator
        /// intercept runs *before* the static rule table, and the decimal is
        /// consumed inside number scanning (so decimal `'-'` would lex `1-2`
        /// as the number 1.2). Reject them all up front.
        const RESERVED_GLYPHS: &[char] = &[
            '+', '-', '*', '/', '^', '=', '<', '>', '!', '&', '~', '|', '(', ')', '[', ']', '{',
            '}', ':', '%', '$', '_', '\\', '\'', '"', '±', '⟨', '⟩', '〈', '〉', '‸', 'ʌ', '′',
        ];
        let usable = |c: char, what: &str| -> Result<(), String> {
            if c.is_alphanumeric() || c.is_whitespace() {
                // Alphanumeric rules out digits and `E`/`e` (number scanning);
                // whitespace would never be seen by the scanner.
                return Err(format!(
                    "{what} must not be a letter, digit, or whitespace (got {c:?})"
                ));
            }
            if RESERVED_GLYPHS.contains(&c) {
                return Err(format!(
                    "{what} {c:?} collides with an operator/bracket the parser already uses"
                ));
            }
            Ok(())
        };
        usable(self.decimal_separator, "decimal separator")?;
        usable(self.argument_separator, "argument separator")?;
        if self.argument_separator == '.' {
            // `.` as the argument separator would shadow `...` (Ldots) and
            // leading-decimal numbers; only its decimal role is coherent.
            return Err("argument separator must not be '.' (conflicts with '...' and decimals)"
                .to_string());
        }
        if self.decimal_separator == self.argument_separator {
            return Err(format!(
                "decimal and argument separators must differ (both are {:?}) — \
                 set both when using a non-default decimal",
                self.decimal_separator
            ));
        }
        if let Some(extra) = &self.also_accept_decimal {
            if extra.contains(&self.argument_separator) {
                return Err(format!(
                    "also_accept_decimal must not include the argument separator {:?}",
                    self.argument_separator
                ));
            }
        }
        if let Some(g) = self.group_separator {
            usable(g, "group separator")?;
            if g == self.decimal_separator || g == self.argument_separator {
                return Err(format!(
                    "group separator {g:?} must differ from the decimal and argument separators"
                ));
            }
        }
        Ok(())
    }
}
