//! §5.2 row 20: an associative-array index written as a TYPEDEF name
//! (`bit [7:0] m [addr_t];`, `[p::addr_t]`, an enum). The keyword-led spellings
//! (`[int]`, `[bit [31:0]]`, …) are parsed in `parse_dim` itself.

use super::*;

impl Parser<'_, '_> {
    /// Is the cursor (just after a dimension's `[`) at a typedef name followed by
    /// `]` whose type is integral and not a class handle or an unpacked array? Only
    /// such a name is a key TYPE; any other identifier keeps the size reading
    /// (`[N]`), exactly as before row 20.
    pub(crate) fn assoc_typedef_key_ahead(&self) -> bool {
        let Some(info) = self.peek_typedef_name() else {
            return false;
        };
        let q = self.scope_qualifier_len();
        self.peek_at(q + 1) == Some(TokenKind::RBracket)
            && (q != 0 || self.name_is_only_a_type(self.cur_text()))
            && info.class_name.is_none()
            && info.unpacked.is_empty()
            && matches!(
                info.kind,
                NetVarKind::Bit
                    | NetVarKind::Logic
                    | NetVarKind::Reg
                    | NetVarKind::Int
                    | NetVarKind::Integer
                    | NetVarKind::Byte
                    | NetVarKind::Shortint
                    | NetVarKind::Longint
                    | NetVarKind::Time
            )
    }

    /// Parse the typedef key [`Self::assoc_typedef_key_ahead`] accepted, through its
    /// `]`. The key takes the typedef's resolved kind, sign and packed dimensions.
    ///
    /// Refused by name, before any engine sees it: a type parameter (its shape is
    /// per instance, and this record is the parse-time default), a packed struct or
    /// union (no oracle measured for a struct key), and a type whose bounds are not
    /// all integer literals in its own declaration (`TypeInfo::layout_exact` false:
    /// a bound written as a name is folded where it is USED, not where the type was
    /// declared — the §3.b cont-array-typedef-elem hazard — and an `enum bit [N]`
    /// base is recorded as 4-state `logic`).
    pub(crate) fn parse_assoc_typedef_key(&mut self) -> Dim {
        let start = self.cur_span();
        let info = self
            .peek_typedef_name()
            .expect("assoc_typedef_key_ahead saw a typedef name");
        let key = self.type_name_key();
        let refused =
            info.shape_param.is_some() || self.packed_layout(&key).is_some() || !info.layout_exact;
        if refused {
            self.error(
                "an associative-array key type whose layout is fixed where it is declared \
                 (a key typed by a type parameter, a packed struct or union, or a typedef \
                 with a bound that is not an integer literal is unsupported)",
            );
        }
        self.eat_scope_qualifier();
        self.bump(); // the type name
        let span = start.to(self.prev_span());
        self.expect(TokenKind::RBracket, "']'");
        if refused {
            return Dim::Dyn;
        }
        Dim::Assoc(AssocKey::Typed(Box::new(AssocKeyType {
            kind: info.kind,
            signed: info.signed,
            range: info.range,
            packed: info.packed,
            named: true,
            explicit_sign: false,
            span,
        })))
    }

    /// Round-2 differential B1: is every occurrence of the bare identifier `name` in
    /// this unit's tokens one a TYPE can stand in? The typedef table is keyed by bare
    /// name and does not see a value declaration that shadows it (a parameter,
    /// localparam, genvar, port, variable or enum label of the same name — local,
    /// later in the text, or in a generate block — wins over an imported, `$unit`
    /// or outer typedef, IEEE 1800-2017 §26.3), so `[N]` with `parameter int N` and a
    /// visible `typedef … N` read the typedef and lost the array's size. Fail closed:
    /// any other occurrence — `N = …`, `N,`, `.N(…)`, `N - 1`, `genvar N;` — keeps the
    /// pre-row-20 reading of `[N]`, a size. The admitted occurrences are `[N]`
    /// itself, `p::N`, `N x` (a declaration's type), `N'(…)` (a cast), and the name a
    /// `typedef … N;` / `typedef … N [` statement declares.
    fn name_is_only_a_type(&self, name: &str) -> bool {
        use TokenKind as T;
        let toks = self.toks;
        let is_name = |t: &Spanned| {
            matches!(t.kind, T::Word(WordKind::Ident) | T::EscapedIdent)
                && self.src.get(t.span.clone()) == Some(name)
        };
        for (i, t) in toks.iter().enumerate() {
            if !is_name(t) {
                continue;
            }
            let prev = i.checked_sub(1).map(|j| toks[j].kind);
            let next = toks.get(i + 1).map(|t| t.kind);
            let admitted = (prev == Some(T::LBracket) && next == Some(T::RBracket))
                || prev == Some(T::ColonColon)
                || matches!(next, Some(T::Word(WordKind::Ident) | T::EscapedIdent))
                || next == Some(T::Apostrophe)
                || (matches!(next, Some(T::Semi | T::LBracket))
                    && toks[..i]
                        .iter()
                        .rev()
                        .take_while(|t| t.kind != T::Semi)
                        .any(|t| t.kind == T::Word(WordKind::Keyword(Kw::Typedef))));
            if !admitted {
                return false;
            }
        }
        true
    }
}
