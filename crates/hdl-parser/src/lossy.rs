//! The parser's half of `hdl_ast::LossySpell` (§5.2 Q-LOSSYSPELL): which lossy constant
//! producers a unit's tokens spell. Structure only — nothing in production reads it yet.
//!
//! Two bits (`param_xz`, `unsized_xz`) are a function of the TOKEN VECTOR the parser was
//! given (`scan_lossy`, one pass, called once by `parse_lexed`), like
//! `first_inside_name_use`: they do not depend on which tokens the parse consumed, so a
//! parse path added later cannot bypass them. The third, `fill_xz`, is the token vector AND
//! the parser's marks (§5.2 Q-LOSSYSPELL U4, fix N4): a fill counts unless the token shape
//! `=` / `<=` FILL `;` holds and the parser marked the fill's token as the start of a right
//! side it took at one of its 8 assignment / initializer sites (`assign_rhs`); a path added
//! later that takes a right side without `assign_rhs` leaves its fill COUNTED (an
//! over-report), never exempt. The pass runs only on a unit whose lexer saw an x / z
//! literal (`hdl_lexer::LexFacts::xz_literal`, §5.2 Q-LOSSYSPELL U3): each of the three is
//! set only by an `IntSizedXz` / `IntUnsizedBasedXz` token, so on any other unit all three
//! are false without a pass. It reads no literal text the token kind answers, and returns
//! as soon as the three are set.
//! The other two are what the parser built: `sign_kw_param` the data type
//! (`Parser::param_dtype` calls `note_param_dtype`), because "a signing keyword with no
//! type and no range" is what the prefix parse decided, not a token shape; `let_any` that
//! the parser entered `parse_let_decl` (its first line calls `note_let_decl`), the one fn
//! that builds a `LetDecl`. Each has one parser field, written here only, by `|=`, in no
//! scope snapshot, so a backtrack or a scope exit can leave it set (over-report), never
//! clear it. The marks are the opposite polarity — a mark left by a parse the parser took
//! back would EXEMPT a fill — so every restore of the cursor is `Parser::rewind`, which pops
//! the marks at or past the restored position (home rule H6, `elaborate/tests/lossy_home.rs`).
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

/// The three token bits of `hdl_ast::LossySpell` over the whole token vector
/// (`sign_kw_param` and `let_any` stay false here: the caller ORs in the parser's notes).
/// `marks` is the parser's `lossy_whole_rhs` (ascending token indices, `assign_rhs`): only
/// `fill_xz` reads it. The caller runs it only when the lexer saw an x / z literal.
pub(crate) fn scan_lossy(toks: &[Spanned], src: &str, marks: &[usize]) -> LossySpell {
    debug_assert!(
        marks.windows(2).all(|w| w[0] < w[1]),
        "scan_lossy: the right-side marks are not ascending"
    );
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
            // The kind says the literal has an x / z / ? digit (the lexer's split, premise
            // PR18): no text is read for what the kind answers. A sized literal is never a
            // fill and never unsized, so the one bit it can set is `param_xz`, in a region.
            TokenKind::IntSizedXz => {
                if !regions.is_empty() {
                    ls.param_xz = true;
                    if all_three(&ls) {
                        return ls;
                    }
                }
            }
            // An unsized one is a FILL (`'x 'X 'z 'Z`) when the byte after its `'` is the
            // digit itself; `'sx` and `'bx` (a signing letter or a base after the `'`) are
            // unsized x / z literals, as U1's `based_digits` read them.
            TokenKind::IntUnsizedBasedXz => {
                ls.param_xz |= !regions.is_empty();
                let fill = matches!(
                    src.as_bytes().get(t.span.start + 1),
                    Some(b'x' | b'X' | b'z' | b'Z')
                );
                if !fill {
                    ls.unsized_xz = true;
                } else if !ls.fill_xz {
                    ls.fill_xz = fill_counts(toks, i, marks);
                }
                if all_three(&ls) {
                    return ls;
                }
            }
            _ => {}
        }
    }
    ls
}

/// Every bit `scan_lossy` can set is set: nothing later in the vector can change it.
#[inline]
fn all_three(ls: &LossySpell) -> bool {
    ls.param_xz && ls.unsized_xz && ls.fill_xz
}

/// The owner's question Q1 (RULINGS-plan R10, R14): which `'x` / `'z` FILL tokens count
/// as a lossy spelling. Built: option (c) — every one EXCEPT a fill the parser took as the
/// WHOLE right side of an assignment or of a net / variable declaration initializer, at one
/// of the 8 sites that call `assign_rhs` (premise PR16 measured the plain fill right or loud
/// on every left side those sites take). Fix N4 (§5.2 Q-LOSSYSPELL U4): the exemption is two
/// facts at once — the token shape `=` / `<=` FILL `;` (the `;` makes the fill the whole
/// right side, the `=` / `<=` puts no timing control before it) AND the fill's index in
/// `marks` (the parser took a right side starting there). The shape alone also held for a
/// relational tail (`b = a <= 'z;`, in every position), a parameter / localparam /
/// specparam / defparam value, a `let` body, a genvar initialization, a covergroup option, a
/// clocking signal's and a UDP's initializer: none is a mark site (main's K1), so their
/// fill counts. Option (b) — every fill counts — is this body replaced by `true`. Keep the
/// rule in this one fn and one condition.
fn fill_counts(toks: &[Spanned], i: usize, marks: &[usize]) -> bool {
    let kind = |j: Option<usize>| j.and_then(|j| toks.get(j)).map(|t| t.kind);
    !(matches!(
        kind(i.checked_sub(1)),
        Some(TokenKind::Eq | TokenKind::LtEq)
    ) && matches!(kind(Some(i + 1)), Some(TokenKind::Semi))
        && marks.binary_search(&i).is_ok())
}

/// The head of a `let` declaration: the word `let` (an identifier token, never the
/// escaped `\let `) followed by an identifier or an escaped identifier. The guard of the
/// module-item dispatch (`item_dispatch.rs`), the only way to `parse_let_decl`.
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

    /// `let_any`'s note: the first line of `parse_let_decl`, the one fn that builds a
    /// `LetDecl` (one caller, the dispatch under `is_let_head`). A head whose body the
    /// parse then refuses stays noted (over-report; the refused unit reaches no reader).
    pub(crate) fn note_let_decl(&mut self) {
        self.lossy_let_any |= true;
    }

    /// The right side of an assignment or of a net / variable declaration initializer, at
    /// one of the 8 sites that take one (fix N4, §5.2 Q-LOSSYSPELL U4): blocking `=` and
    /// nonblocking `<=` (`parse_assign_or_call`, after any intra-assignment timing),
    /// procedural `assign` (`parse_proc_assign`), `force` (`parse_force`), continuous
    /// `assign` (`parse_cont_assign`), a declarator's initializer (`parse_decl_name_list`,
    /// the funnel of every net / variable declaration), a typed for-init
    /// (`build_for_typed_init`) and a for-clause assignment (`parse_for_assign`). When the
    /// right side starts at an unsized x / z token its index is marked (`lossy_whole_rhs`):
    /// the mark says only "a right side starts here" ; `fill_counts`'s token shape makes it
    /// the whole right side. A site without this call leaves its fill counted. The sites
    /// are pinned by home rule H6 (b) ; a ninth comes with the cell measuring its plain fill.
    pub(crate) fn assign_rhs(&mut self) -> Expr {
        if self.peek() == Some(TokenKind::IntUnsizedBasedXz) {
            self.lossy_whole_rhs.push(self.pos);
        }
        self.expr(0)
    }
}
