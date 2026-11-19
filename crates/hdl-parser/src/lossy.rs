//! The parser's half of `hdl_ast::LossySpell` (§5.2 Q-LOSSYSPELL): which lossy constant
//! producers a unit's tokens spell. Structure only — nothing in production reads it yet.
//!
//! Four bits are a function of the TOKEN VECTOR the parser was given (`scan_lossy`, one
//! pass, called once by `parse_with_warnings`), like `first_inside_name_use`: they do not
//! depend on which tokens the parse consumed, so a parse path added later cannot bypass
//! them. The fifth, `sign_kw_param`, is the data type the parser built
//! (`Parser::param_dtype` calls `note_param_dtype`), because "a signing keyword with no
//! type and no range" is what the prefix parse decided, not a token shape. Its one
//! parser field is written here only, by `|=`, and is in no scope snapshot, so a
//! backtrack or a scope exit can leave it set (over-report), never clear it.
//!
//! Each bit may over-report and must not under-report what the parser accepts:
//! - a REGION is `#` directly followed by `(`, through the matching `)` (r1), or a
//!   `parameter` / `localparam` / `specparam` / `defparam` keyword through the next `;`
//!   at the keyword's bracket depth or the token closing the bracket the keyword sits in,
//!   whichever comes first (r2); the end of input closes every region. IEEE 1800-2017
//!   Annex A opens every parameter value, default, override and defparam right side with
//!   one of these five tokens, but for a config's `use .P(v)`, which vita refuses (E2002):
//!   a row that makes `config` parse adds its use clause here in the same commit. A
//!   delay `#( … )` and a type's range inside a parameter statement are regions too.
//! - bracket depth counts `(` `[` `[*` `[->` `[=` `{` up and `)` `]` `}` down (`[+]` is
//!   one self-closed token, `'{` is `'` + `{`, `##` is never an opener).
//! - an escaped identifier (`\parameter `, `\let `) is a name, never the keyword.

use super::*;

/// The region kinds of `scan_lossy`, each with the bracket depth at its opener.
#[derive(Clone, Copy)]
enum Region {
    /// `#(` … `)`: closes when a closer brings the depth back to the opener's.
    HashParen,
    /// `parameter` / `localparam` / `specparam` / `defparam` … `;`: closes at a `;` at
    /// the keyword's depth, or when a closer goes below it.
    Keyword,
}

fn opens(k: TokenKind) -> bool {
    matches!(
        k,
        TokenKind::LParen
            | TokenKind::LBracket
            | TokenKind::LBracketStar
            | TokenKind::LBracketArrow
            | TokenKind::LBracketEq
            | TokenKind::LBrace
    )
}

fn closes(k: TokenKind) -> bool {
    matches!(
        k,
        TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace
    )
}

/// The four token bits of `hdl_ast::LossySpell` over the whole token vector
/// (`sign_kw_param` stays false here: the caller ORs in the parser's note).
pub(crate) fn scan_lossy(toks: &[Spanned], src: &str) -> LossySpell {
    let mut ls = LossySpell::default();
    let mut depth: u32 = 0;
    let mut regions: Vec<(Region, u32)> = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        match t.kind {
            TokenKind::Hash
                if matches!(toks.get(i + 1).map(|n| n.kind), Some(TokenKind::LParen)) =>
            {
                regions.push((Region::HashParen, depth));
            }
            TokenKind::Word(WordKind::Keyword(
                Kw::Parameter | Kw::Localparam | Kw::Specparam | Kw::Defparam,
            )) => regions.push((Region::Keyword, depth)),
            k if opens(k) => depth += 1,
            k if closes(k) => {
                // Saturating: a stray closer leaves every region open (over-report).
                depth = depth.saturating_sub(1);
                while let Some(&(r, d)) = regions.last() {
                    let done = match r {
                        Region::HashParen => depth <= d,
                        Region::Keyword => depth < d,
                    };
                    if !done {
                        break;
                    }
                    regions.pop();
                }
            }
            TokenKind::Semi => {
                while matches!(regions.last(), Some(&(Region::Keyword, d)) if d == depth) {
                    regions.pop();
                }
            }
            TokenKind::IntSized | TokenKind::IntUnsizedBased => {
                let text = src.get(t.span.clone()).unwrap_or("");
                let (fill, xz) = based_digits(text);
                if xz {
                    ls.param_xz |= !regions.is_empty();
                    ls.unsized_xz |= t.kind == TokenKind::IntUnsizedBased && !fill;
                    ls.fill_xz |= fill && fill_counts(toks, i);
                }
            }
            TokenKind::Word(WordKind::Ident) => ls.let_any |= is_let_head(toks, src, i),
            _ => {}
        }
    }
    ls
}

/// A based literal's text (`IntSized` / `IntUnsizedBased`, the whole lexeme): whether it
/// is a FILL (`'0 '1 'x 'z`, no `s`, no base letter) and whether its digits after the
/// base letter — or its fill digit — hold an `x X z Z ?`. `'sx` / `'sz` (the lexer's fill
/// arm admits an `s`) are not fills: they count as unsized x / z literals.
fn based_digits(text: &str) -> (bool, bool) {
    let Some((_, after)) = text.split_once('\'') else {
        return (false, false);
    };
    let signed = after.strip_prefix(['s', 'S']);
    let rest = signed.unwrap_or(after);
    let base = rest.starts_with(['d', 'D', 'b', 'B', 'o', 'O', 'h', 'H']);
    let digits = if base { &rest[1..] } else { rest };
    let fill = signed.is_none() && !base;
    (fill, digits.contains(['x', 'X', 'z', 'Z', '?']))
}

/// The owner's question Q1 (RULINGS-plan R10, R14): which `'x` / `'z` FILL tokens count
/// as a lossy spelling. Built: option (c) — every one EXCEPT the plain shape `=` / `<=`
/// FILL `;` (the whole right side of an assignment or initializer; premise PR16 measured
/// it right or loud on every left side that takes it). Option (b) — every fill counts —
/// is this body replaced by `true`. Keep the rule in this one fn and one condition.
fn fill_counts(toks: &[Spanned], i: usize) -> bool {
    let kind = |j: Option<usize>| j.and_then(|j| toks.get(j)).map(|t| t.kind);
    !(matches!(
        kind(i.checked_sub(1)),
        Some(TokenKind::Eq | TokenKind::LtEq)
    ) && matches!(kind(Some(i + 1)), Some(TokenKind::Semi)))
}

/// The head of a `let` declaration: the word `let` (an identifier token, never the
/// escaped `\let `) followed by an identifier or an escaped identifier. The ONE
/// predicate both the module-item dispatch's guard (`item_dispatch.rs`, the only way to
/// `parse_let_decl`) and `scan_lossy` call, so the bit cannot miss a `let` the parser
/// accepts.
pub(crate) fn is_let_head(toks: &[Spanned], src: &str, i: usize) -> bool {
    matches!(
        toks.get(i + 1).map(|t| t.kind),
        Some(TokenKind::Word(WordKind::Ident) | TokenKind::EscapedIdent)
    ) && matches!(toks.get(i), Some(t) if t.kind == TokenKind::Word(WordKind::Ident)
        && src.get(t.span.clone()) == Some("let"))
}

impl Parser<'_, '_> {
    /// `sign_kw_param`'s note, from the data type `param_dtype` just built: a written
    /// `signed` / `unsigned`, no type keyword and no packed range (`parameter signed N`).
    /// The range form (`parameter signed [3:0] S`) is measured right (cells f7_sgnrange,
    /// b4_sgnrange) and stays out.
    pub(crate) fn note_param_dtype(&mut self, dt: &DataType) {
        self.lossy_sign_kw_param |= matches!(dt.kind(), TypeKind::Implicit)
            && matches!(dt.signing(), Some(Signing::Signed | Signing::Unsigned))
            && dt.range.is_none();
    }
}
