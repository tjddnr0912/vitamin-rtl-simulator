//! The refusal of a signing keyword that LEADS a parameter's type (§5.2
//! Q-PARAM2STATE S1v): a spelling outside IEEE 1800-2017's grammar that the
//! parameter prefix (`Parser::parse_param_prefix`) accepted and folded.
//!
//! IEEE 1800-2017 (ieee1800-2017.txt line numbers): `data_type` (A.2.2.1,
//! :52141-52143, :52170-52172) puts a signing only AFTER an integer type keyword —
//! `integer_vector_type [ signing ]`, `integer_atom_type [ signing ]` — and gives
//! `non_integer_type`, `string` and a type identifier none; an `implicit_data_type`
//! (:52160, `[ signing ] { packed_dimension }`) is the only type a signing leads.
//! `specparam_declaration` (A.2.1.1, :52085-52086) takes a packed dimension and no
//! signing. A parameter port with no `parameter` / `localparam` keyword
//! (`parameter_port_declaration`, A.1.3, :51741-51748) is a `data_type
//! list_of_param_assignments`, never an implicit type.
//!
//! Only the LEADING position is refused: on a `specparam`, before a type keyword or
//! type name the prefix parses, and in a header port without the keyword.  A trailing
//! signing (`int unsigned`) keeps PRE's reading, and so do the spellings the grammar
//! refuses that the prefix accepts without a leading signing (a `specparam` with a
//! data type, a signing after `logic [3:0]`; filed in §2).  The refusal is one parse
//! error at the signing token; the prefix then reads on as before and builds the
//! `ParamDecl` PRE builds.

use super::*;

/// The keyword a parameter prefix opened with — which rule a leading signing breaks.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeadKeyword {
    /// `parameter` or `localparam`.
    Param,
    /// `specparam` (parsed as a localparam).
    Specparam,
    /// None: a `#(…)` header port without the keyword.
    None,
}

impl Parser<'_, '_> {
    /// The E2002 text for the signing keyword (`signed` true for `signed`) just
    /// consumed at the head of a parameter prefix, read off the token after it;
    /// `None` where the grammar admits the signing (an implicit type, `parameter
    /// signed [3:0] P`) or where the next token starts no type the prefix parses
    /// (PRE's own path then reports it, or not).
    pub(crate) fn leading_signing_refusal(
        &self,
        lead: LeadKeyword,
        signed: bool,
    ) -> Option<String> {
        if lead == LeadKeyword::Specparam {
            return Some(
                "no signing on a `specparam`, which takes only a packed range \
                 (IEEE 1800-2017 A.2.1.1 `specparam_declaration`)"
                    .to_string(),
            );
        }
        match self.peek() {
            // `integer_vector_type` / `integer_atom_type`: the signing follows the keyword.
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Logic
                | Kw::Reg
                | Kw::Bit
                | Kw::Int
                | Kw::Byte
                | Kw::Shortint
                | Kw::Longint
                | Kw::Integer
                | Kw::Time,
            ))) => {
                // A signing written after the keyword too (`unsigned int signed`): the
                // one to drop is the leading one, so the text names the trailing one.
                let trail = match self.peek_at(1) {
                    Some(TokenKind::Word(WordKind::Keyword(Kw::Signed))) => Some("signed"),
                    Some(TokenKind::Word(WordKind::Keyword(Kw::Unsigned))) => Some("unsigned"),
                    _ => None,
                };
                let kw = self.cur_text();
                Some(match trail {
                    Some(t) => format!(
                        "one signing only, after the type keyword, where `{kw} {t}` has one \
                         (IEEE 1800-2017 A.2.2.1 `data_type`)"
                    ),
                    None => format!(
                        "the signing after the type keyword, `{kw} {}` \
                         (IEEE 1800-2017 A.2.2.1 `data_type`)",
                        if signed { "signed" } else { "unsigned" }
                    ),
                })
            }
            // `non_integer_type` / `string`: no signing in any position.
            Some(TokenKind::Word(WordKind::Keyword(Kw::Real | Kw::Realtime | Kw::String))) => {
                Some(format!(
                    "no signing before `{}`, which takes none \
                     (IEEE 1800-2017 A.2.2.1 `data_type`)",
                    self.cur_text()
                ))
            }
            _ if self.peek_block_typedef_decl().is_some() || self.peek_alias_param().is_some() => {
                Some(
                    "no signing before a type name, which takes none \
                     (IEEE 1800-2017 A.2.2.1 `data_type`)"
                        .to_string(),
                )
            }
            _ if lead == LeadKeyword::None => Some(
                "`parameter` or `localparam` before a signing in a parameter port \
                 (IEEE 1800-2017 A.1.3 `parameter_port_declaration`)"
                    .to_string(),
            ),
            _ => None,
        }
    }

    /// Record an error ON a token already consumed: `span` / `tok` are that token's,
    /// so the report is anchored at it and its `found` tail names it — where
    /// [`Self::error_at`] would name the token after it.
    pub(crate) fn error_on_token(&mut self, span: Span, tok: Option<TokenKind>, expected: String) {
        if self.errors.len() < self.error_limit {
            self.errors.push(ParseError {
                span,
                expected: expected.into(),
                found: tok,
                found_span: span,
            });
        }
    }
}
