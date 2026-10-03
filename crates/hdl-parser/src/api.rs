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
/// other tools read differently, or one whose IEEE check vita skips (the first `unique`
/// / `unique0` qualifier). They ride a SEPARATE channel from `ParseError` because
/// a warning must not gate the parse, and because a severity field on the error type
/// would put "did this stop the parse?" and "how bad is it?" in one place.
pub fn parse_with_warnings(
    tokens: &[Spanned],
    src: &str,
) -> (Option<SourceUnit>, Vec<ParseError>, Vec<ParseWarn>) {
    let mut p = Parser::new(tokens, src);
    let unit = p.parse_source_unit();
    let mut su = if unit.items.is_empty() && p.errors.is_empty() {
        None
    } else {
        Some(unit)
    };
    // Only into a unit that exists: the record is not a design unit, so a source
    // with none stays `None`, as before it existed.
    if let (Some(u), Some(span)) = (&mut su, first_inside_name_use(tokens, src, &p.inside_kw_at)) {
        u.items.push(TopItem::InsideNameUse(span));
    }
    (su, p.errors, p.warnings)
}

/// The first identifier token spelled `inside` that is a NAME: counted over the WHOLE
/// token vector, so it does not depend on which tokens the parse consumed, minus the
/// tokens the two contextual-keyword sites took (`kw_at`). An escaped identifier
/// `\inside` is the name `inside` (IEEE 1800-2017 §5.6.1) and is never the keyword.
fn first_inside_name_use(tokens: &[Spanned], src: &str, kw_at: &[usize]) -> Option<Span> {
    tokens.iter().enumerate().find_map(|(i, t)| {
        let text = src.get(t.span.clone())?;
        let name = match t.kind {
            TokenKind::Word(WordKind::Ident) => text == "inside" && !kw_at.contains(&i),
            TokenKind::EscapedIdent => text.strip_prefix('\\').map(str::trim_end) == Some("inside"),
            _ => false,
        };
        name.then(|| Span::new(t.span.start as u32, t.span.end as u32))
    })
}
