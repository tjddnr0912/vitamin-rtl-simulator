//! public parse entry — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

/// Public API — mirrors `hdl_lexer::lex`'s two-channel shape. Never panics; returns
/// a (partial) AST plus all recovered errors. The driver maps errors → diagnostics
/// (E-PARSE-UNEXPECTED-TOKEN / VITA-E2002) and enforces `--error-limit`.
/// Empty input ⇒ `(None, [])`.
pub fn parse(tokens: &[Spanned], src: &str) -> (Option<SourceUnit>, Vec<ParseError>) {
    let mut p = Parser::new(tokens, src);
    let unit = p.parse_source_unit();
    let su = if unit.items.is_empty() && p.errors.is_empty() {
        None
    } else {
        Some(unit)
    };
    (su, p.errors)
}
