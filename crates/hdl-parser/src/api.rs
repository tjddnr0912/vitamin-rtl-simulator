//! public parse entry — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

/// Public API — mirrors `hdl_lexer::lex`'s two-channel shape. Never panics; returns
/// a (partial) AST plus all recovered errors. The driver maps errors → diagnostics
/// (E-PARSE-UNEXPECTED-TOKEN / VITA-E2002) and enforces `--error-limit`.
/// Empty input ⇒ `(None, [])`.
///
/// Non-fatal observations ([`ParseWarn`]) are dropped here. The arity stays at two on
/// purpose: forty-four call sites destructure this pair, and widening it for the one
/// caller that renders warnings would edit all of them to write `_`. Use
/// [`parse_with_warnings`] where the third channel is actually consumed.
pub fn parse(tokens: &[Spanned], src: &str) -> (Option<SourceUnit>, Vec<ParseError>) {
    let (su, errs, _) = parse_with_warnings(tokens, src);
    (su, errs)
}

/// [`parse`] plus the non-fatal [`ParseWarn`]s — a construct that parsed fine but that
/// other tools read differently. They ride a SEPARATE channel from `ParseError` because
/// a warning must not gate the parse, and because a severity field on the error type
/// would put "did this stop the parse?" and "how bad is it?" in one place.
pub fn parse_with_warnings(
    tokens: &[Spanned],
    src: &str,
) -> (Option<SourceUnit>, Vec<ParseError>, Vec<ParseWarn>) {
    let mut p = Parser::new(tokens, src);
    let unit = p.parse_source_unit();
    let su = if unit.items.is_empty() && p.errors.is_empty() {
        None
    } else {
        Some(unit)
    };
    (su, p.errors, p.warnings)
}
