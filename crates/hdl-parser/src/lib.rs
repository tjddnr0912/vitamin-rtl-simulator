//! hdl-parser — token-stream → hdl-ast (PARSE stage).
//!
//! Hand-written recursive descent + Pratt expression parser over `&[Spanned]`.
//! Never panics: errors are recorded in `Vec<ParseError>` and recovered via
//! panic-mode sync (to `;` / `end` / `endmodule` / top-level keywords). The driver
//! maps each `ParseError` → `diag::Diagnostic` (E-PARSE-UNEXPECTED-TOKEN/VITA-E2002)
//! and owns the `--error-limit` hard stop (doc-13). PR1 fully parses: module header
//! (ANSI + non-ANSI), parameter/localparam, net/var decls, continuous `assign` —
//! each with the full precedence-correct expression grammar. Procedural blocks /
//! instances / generate recover to a stub `Error` item (their hdl-ast types exist).
//!
//! Technique (decisive): pure hand-RD + Pratt, NO winnow dep — verified absent from
//! `[workspace.dependencies]`. Per doc-02 this slice IS the hand-RD target set
//! (hot + recovery-critical + precedence-heavy); winnow's `TokenSlice` needs a
//! `Location` newtype to surface spans and its recovery is `unstable-recover`-gated.

use hdl_ast::*;
use hdl_lexer::{Kw, Spanned, TokenKind, WordKind};

// ── YELLOW #1/#9: User-Defined Primitive table symbols (module-scope so the
//    `parse_udp_decl` row scanner helpers can name them in their signatures). All
//    purely parser-local — they never reach the AST/IR (the UDP desugars to an
//    ordinary `ModuleDecl`). ──
/// A combinational LEVEL input symbol (single input column).
#[derive(Clone, Copy, PartialEq)]
enum UdpLevSym {
    Zero,
    One,
    X,
    Q, // `?` wildcard
    B, // `b` = 0-or-1
}
/// A sequential-UDP edge ENDPOINT (`(from to)`). `Q` = `?` wildcard endpoint.
#[derive(Clone, Copy, PartialEq)]
enum UdpEnd {
    Zero,
    One,
    X,
    Q,
}
/// One input column: a level symbol, or ONE edge spec (a set of (from,to) pairs).
enum UdpInCol {
    Lev(UdpLevSym),
    Edge(Vec<(UdpEnd, UdpEnd)>),
}
/// A combinational output symbol (`0 1 x`).
#[derive(Clone, Copy)]
enum UdpOutSym {
    Zero,
    One,
    X,
}
/// A sequential current-STATE column symbol (`?` = wildcard).
#[derive(Clone, Copy)]
enum UdpStateSym {
    Zero,
    One,
    X,
    Q,
}
/// A sequential NEXT-state symbol (`-` = no-change / hold).
#[derive(Clone, Copy)]
enum UdpNextSym {
    Zero,
    One,
    X,
    Hold,
}

/// GATE: the 12 gate-level primitive kinds (desugared to continuous assigns).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateKind {
    And,
    Or,
    Nand,
    Nor,
    Xor,
    Xnor,
    Buf,
    Not,
    Bufif0,
    Bufif1,
    Notif0,
    Notif1,
}

// ───────────────────────────── errors ─────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub span: Span,               // offending token's span (u32)
    pub expected: &'static str,   // "expression", "';'", "identifier", …
    pub found: Option<TokenKind>, // None ⇒ EOF
}

// ───────────────────────────── cursor ─────────────────────────────
/// Resolved underlying type of a `typedef` name, used to lower `T x;` declarations
/// (Phase-2). For `typedef enum {…} color_t;` the underlying storage is `int`
/// (32-bit signed); a future `typedef logic [7:0] byte_t;` would carry its range.
#[derive(Clone)]
struct TypeInfo {
    kind: NetVarKind,
    signed: bool,
    range: Option<Range>,
    packed: Vec<Range>,
    /// N7: for a `NetVarKind::ClassHandle` alias, the class name; else `None`.
    class_name: Option<String>,
}

/// Flat bit layout of a packed struct: members are placed MSB-first into one
/// `logic [total-1:0]` vector. `fields` carries `(name, lsb_offset, width,
/// ascending, signed, two_state)` so a `s.field` access desugars to the constant
/// part-select `s[off+w-1 : off]`, and a trailing sub-select (`s.f[i]` /
/// `s.f[a:b]` / `s.f[base±:w]`) can be remapped onto the flat vector with the
/// member's declared direction (`ascending` = `logic [0:N]`, field index 0 = MSB).
/// `signed` is the member's EFFECTIVE signedness (atom types `int`/`byte`/… and
/// `signed`-qualified vectors are signed); the WHOLE-field read is wrapped in a
/// `$signed()` so a signed member reads back negative (a sub-select stays
/// unsigned per §5.4.1, matching iverilog). `two_state` (the member is `bit`/
/// `byte`/`int`/`shortint`/`longint`) drives the `'{…}` pattern desugar to coerce
/// X/Z→0 into that field (§6.11.3), which a 4-state member does not.
#[derive(Clone)]
struct StructLayout {
    fields: Vec<(String, u32, u32, bool, bool, bool)>,
}
impl StructLayout {
    fn field(&self, name: &str) -> Option<(u32, u32, bool, bool)> {
        self.fields
            .iter()
            .find(|(n, _, _, _, _, _)| n == name)
            .map(|(_, o, w, asc, sgn, _ts)| (*o, *w, *asc, *sgn))
    }
}

/// A snapshot of the parser's lexically-scoped registries, used to give a
/// procedural block its own scope: snapshotted at the block's first body-local
/// typedef DEFINITION or struct/enum-typed VAR declaration and restored when the
/// block ends, so a local name is visible inside the block but does NOT leak out
/// or clobber a same-named outer name (which would be a silent-wrong — iverilog
/// scopes both). The TYPE-name-keyed maps scope a body-local `typedef`
/// (§4.5.51); the VAR-name-keyed maps scope a block-local struct/enum variable
/// whose name shadows an outer struct/enum variable (else a later outer
/// `x.field` would desugar against the inner layout — `b` instead of `bb`).
struct ScopeSnapshot {
    // TYPE-name-keyed (a body-local typedef definition).
    typedefs: std::collections::HashMap<String, TypeInfo>,
    struct_layouts: std::collections::HashMap<String, StructLayout>,
    enum_defs: std::collections::HashMap<String, Vec<(String, i64)>>,
    union_type_names: std::collections::HashSet<String>,
    // VAR-name-keyed (a block-local struct/enum variable shadowing an outer one).
    var_struct: std::collections::HashMap<String, String>,
    var_enum: std::collections::HashMap<String, String>,
    struct_scalar_vars: std::collections::HashSet<String>,
    struct_1d_array_vars: std::collections::HashSet<String>,
}

/// A trailing READ sub-select on a packed-struct member, normalized to an
/// indexed part-select *relative to the field part-select* `pv = s[off+w-1:off]`
/// (so elaborate's `IndexedPart`-on-`PartSelect` fold keeps it FIELD-bounded —
/// out-of-field bits read X, never leak into an adjacent member). Every form
/// (bit / regular `[a:b]` / indexed `[base±:w]`) collapses to one indexed shape:
/// the offset/width address `pv[w-1:0]`, where `pv[k]` = flat bit `off+k`.
enum FieldSel {
    Whole, // `s.f` — read the whole field
    Indexed {
        offset: Expr,
        width: Expr,
        dir: PartDir,
    },
}

/// A parsed `parameter`/`localparam` declaration. `Scalar` = the ordinary
/// `ParamDecl` (elaborate folds the value). `ConstArrayVar` = an A2a body
/// ARRAY parameter (`localparam int RHO [0:4] = '{…}`) desugared to the
/// equivalent variable-array `NetVarDecl` with `const_param: true`.
enum ParamItem {
    Scalar(ParamDecl),
    ConstArrayVar(NetVarDecl),
}

/// The components of a parsed `property_spec` (the body shared by an inline
/// `assert property(<spec>)` and a named `property NAME; <spec>; endproperty`):
/// `(clock, disable_iff, antecedent, implication_kind, consequent,
/// consequent_clock, prop_expr, local_vars)`. A flat implication leaves `prop_expr`
/// `None`; a property-operator tree fills it (the flat fields then hold
/// placeholders). `local_vars` (slice N2c) is the body-start `int x;` declarations.
type PropertySpecParts = (
    Sensitivity,
    Option<Expr>,
    Sequence,
    ImplicationKind,
    Sequence,
    Option<Sensitivity>,
    Option<PropExpr>,
    Vec<SvaLocalDecl>,
);

pub struct Parser<'t, 's> {
    toks: &'t [Spanned],
    src: &'s str,
    pos: usize,
    src_end: u32,
    pub errors: Vec<ParseError>,
    error_limit: usize,
    /// P2-5: live expression-recursion depth; capped so a pathological
    /// `((((…))))` yields a parse error instead of a stack overflow.
    expr_depth: u32,
    /// STMT-DEPTH: live statement-recursion depth; capped so pathological
    /// `begin begin … end end` nesting yields a parse error, not a SIGABRT.
    stmt_depth: u32,
    /// PARSE-CONCAT-CAP: cumulative count of parsed expression nodes (every
    /// `expr()` call). A flat `{a,a,…}` concat / arg list builds one `Expr` (80 B)
    /// per element with no depth, so the expr comma-loops are bounded by this
    /// GLOBAL budget (`MAX_AST_NODES`) rather than per-list — robust against the
    /// many-lists bypass too. Monotonic (never decremented).
    node_count: usize,
    /// Latched once `node_count` passes `MAX_AST_NODES`; the expr comma-loops stop
    /// pushing so the AST cannot exceed the budget, and the diagnostic fires once.
    node_budget_blown: bool,
    /// SV user-defined type names (`typedef … name;`) → resolved underlying type.
    /// Accumulates across the source unit; lets `name var;` parse as a typed decl.
    typedefs: std::collections::HashMap<String, TypeInfo>,
    /// Packed-struct type name → flat bit layout (for `s.field` desugaring).
    struct_layouts: std::collections::HashMap<String, StructLayout>,
    /// Variable name → its struct type name (module-scoped; cleared per module).
    var_struct: std::collections::HashMap<String, String>,
    /// Scalar (no unpacked dims) packed-struct variable names — the subset of
    /// `var_struct` keys eligible for the `'{e0,…}` assignment-pattern desugar
    /// (§10.9.1). An array-of-struct (`st_t a[4]`) is in `var_struct` but NOT
    /// here, so `a = '{…}` is left on the unpacked-array path, never mistaken
    /// for a packed-struct concat. Module-scoped; cleared per module.
    struct_scalar_vars: std::collections::HashSet<String>,
    /// 1-D-array-of-packed-struct variable names (`st_t arr[N]`) — eligible for
    /// the element pattern `arr[i] = '{…}` (the element is a scalar struct). A
    /// scalar struct (0 dims, in `struct_scalar_vars`), a multi-dim array (≥2
    /// dims), and a union array are all excluded. Module-scoped.
    struct_1d_array_vars: std::collections::HashSet<String>,
    /// Packed-union type names. Unions share `struct_layouts` (for `u.field`
    /// reads) but their overlay layout is NOT a packed concat, so a union var is
    /// kept OUT of `struct_scalar_vars` and its `'{…}` pattern stays loud.
    /// Accumulates across the source unit (type names are not module-scoped).
    union_type_names: std::collections::HashSet<String>,
    /// Module-scope `localparam` name → its constant value, but ONLY when the value
    /// is a pure literal constant (no `parameter` dependency). Used to fold a
    /// constant generate-array hier index (`g[P].x`, P a localparam). Safe because a
    /// `localparam` is non-overridable, so a literal value is fixed at parse time; a
    /// `parameter` (overridable) is never recorded → its index stays loud, never
    /// silently folding to a value an instance override could disagree with.
    /// Module-scoped; cleared per module.
    const_locals: std::collections::HashMap<String, i64>,
    /// ⓑ-breadth (§8.25): override specializations of parameterized classes,
    /// produced by `monomorphize_param_classes` and appended at top level.
    pending_mono_specs: Vec<ClassDecl>,
    /// SV §11.5 loop-control context stack (one entry per enclosing for/while/
    /// repeat/forever/foreach being parsed). `break`/`continue` desugar to
    /// `disable <synthetic-label>` of the innermost loop; the loop is wrapped in
    /// a synthetic named block ONLY when the corresponding control was used (so a
    /// loop with no break/continue is byte-identical). The top entry is the
    /// innermost loop.
    loop_labels: Vec<LoopLabels>,
    /// SV §6.19.5 enum methods. `typedef enum` name → its ordered `(label, value)`
    /// list, BUT only when every label value is literal-foldable (`const_lit`);
    /// an enum with a non-foldable label value (e.g. `B = SOME_PARAM`) is omitted,
    /// so `x.method()` on it stays a loud error (correct-or-loud). Accumulates
    /// across the source unit (typedef enums are file-scoped like `typedefs`).
    enum_defs: std::collections::HashMap<String, Vec<(String, i64)>>,
    /// Variable name → its enum type name (module-scoped; cleared per module like
    /// `var_struct`). Lets `x.first/last/next/prev/name/num` desugar to literals /
    /// ternary chains over the enum's labels.
    var_enum: std::collections::HashMap<String, String>,
}

/// One enclosing-loop entry for `break`/`continue` desugar. The labels name the
/// synthetic blocks the loop is wrapped in (`$break$<lo>` around the whole loop,
/// `$continue$<lo>` around its body); `*_used` records whether that wrap is needed.
struct LoopLabels {
    break_label: String,
    continue_label: String,
    break_used: bool,
    continue_used: bool,
}

impl<'t, 's> Parser<'t, 's> {
    pub fn new(toks: &'t [Spanned], src: &'s str) -> Self {
        Self {
            toks,
            src,
            pos: 0,
            src_end: src.len() as u32,
            errors: Vec::new(),
            error_limit: 50,
            expr_depth: 0,
            stmt_depth: 0,
            node_count: 0,
            node_budget_blown: false,
            typedefs: std::collections::HashMap::new(),
            struct_layouts: std::collections::HashMap::new(),
            var_struct: std::collections::HashMap::new(),
            struct_scalar_vars: std::collections::HashSet::new(),
            struct_1d_array_vars: std::collections::HashSet::new(),
            union_type_names: std::collections::HashSet::new(),
            const_locals: std::collections::HashMap::new(),
            pending_mono_specs: Vec::new(),
            loop_labels: Vec::new(),
            enum_defs: std::collections::HashMap::new(),
            var_enum: std::collections::HashMap::new(),
        }
    }

    // -- span helpers --
    #[inline]
    fn sp(r: &std::ops::Range<usize>) -> Span {
        Span::new(r.start as u32, r.end as u32)
    }
    #[inline]
    fn cur_span(&self) -> Span {
        self.toks
            .get(self.pos)
            .map(|t| Self::sp(&t.span))
            .unwrap_or(Span::new(self.src_end, self.src_end))
    }
    /// Span of the just-consumed token. VALID ONLY after ≥1 bump (verdict M3-soundness):
    /// at `pos==0` it falls back to `cur_span()` (a safe degenerate), never an
    /// inverted span. Every call site (`start.to(prev_span())`) has bumped first.
    #[inline]
    fn prev_span(&self) -> Span {
        if self.pos == 0 {
            return self.cur_span();
        }
        self.toks
            .get(self.pos - 1)
            .map(|t| Self::sp(&t.span))
            .unwrap_or(Span::new(self.src_end, self.src_end))
    }
    /// Raw lexeme of the token at `pos` (re-slice — tokens carry no value).
    fn cur_text(&self) -> &'s str {
        self.toks
            .get(self.pos)
            .map(|t| &self.src[t.span.clone()])
            .unwrap_or("")
    }
    /// Source text of the token `n` past the cursor (0 = current). Empty past EOF.
    fn text_at(&self, n: usize) -> &'s str {
        self.toks
            .get(self.pos + n)
            .map(|t| &self.src[t.span.clone()])
            .unwrap_or("")
    }

    // -- cursor primitives --
    #[inline]
    fn peek(&self) -> Option<TokenKind> {
        self.toks.get(self.pos).map(|t| t.kind)
    }
    /// Lookahead `n` tokens past the cursor (0 = `peek`).
    #[inline]
    fn peek_at(&self, n: usize) -> Option<TokenKind> {
        self.toks.get(self.pos + n).map(|t| t.kind)
    }
    #[inline]
    fn at_eof(&self) -> bool {
        self.pos >= self.toks.len()
    }
    fn bump(&mut self) -> Option<&'t Spanned> {
        let t = self.toks.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
    fn at_kw(&self, kw: Kw) -> bool {
        matches!(self.peek(), Some(TokenKind::Word(WordKind::Keyword(k))) if k == kw)
    }
    fn is_ident(&self) -> bool {
        matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
        )
    }
    /// True if the next token is a plain identifier spelled exactly `name` — used
    /// for SVA contextual keywords (`throughout`) that are not reserved globally.
    fn at_ident_kw(&self, name: &str) -> bool {
        matches!(self.peek(), Some(TokenKind::Word(WordKind::Ident))) && self.cur_text() == name
    }
    /// True if the next token is a lexer error sentinel (verdict: dedicated handling —
    /// the lexer already emitted the LexError, so we recover WITHOUT re-reporting).
    fn at_lex_error(&self) -> bool {
        matches!(self.peek(), Some(TokenKind::Error(_)))
    }
    fn eat(&mut self, k: TokenKind) -> bool {
        if self.peek() == Some(k) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn eat_kw(&mut self, kw: Kw) -> bool {
        if self.at_kw(kw) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    /// Consume a CONTEXTUAL keyword (an `Ident` token whose text is `name`, e.g. the
    /// SVA `until`/`implies`/`s_eventually` operators), returning whether it matched.
    fn eat_ident_kw(&mut self, name: &str) -> bool {
        if self.at_ident_kw(name) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    /// Consume `k` or record an error (does NOT advance — caller decides to sync).
    fn expect(&mut self, k: TokenKind, what: &'static str) -> bool {
        if self.peek() == Some(k) {
            self.pos += 1;
            true
        } else {
            self.error(what);
            false
        }
    }
    /// Record an error. Suppresses re-reporting on a lexer `Error` token (already
    /// diagnosed by the lexer) — we still record nothing for it, just let the caller
    /// recover. Capped at `error_limit`.
    fn error(&mut self, expected: &'static str) {
        if self.at_lex_error() {
            return;
        } // lexer already emitted a LexError here
        if self.errors.len() < self.error_limit {
            self.errors.push(ParseError {
                span: self.cur_span(),
                expected,
                found: self.peek(),
            });
        }
    }

    /// Like [`error`] but reports at an explicit `span` (e.g. a node parsed earlier).
    fn error_at(&mut self, span: Span, expected: &'static str) {
        if self.errors.len() < self.error_limit {
            self.errors.push(ParseError {
                span,
                expected,
                found: self.peek(),
            });
        }
    }

    // -- ident extraction --
    fn ident(&mut self) -> Option<Ident> {
        if self.is_ident() {
            let t = self.bump().unwrap();
            Some(Ident {
                name: self.src[t.span.clone()].to_string(),
                span: Self::sp(&t.span),
            })
        } else {
            self.error("identifier");
            None
        }
    }
    /// Member name after a `.`: a normal identifier, OR one of the array-method
    /// names the lexer classifies as a keyword because it reuses an operator/
    /// qualifier spelling (`and`/`or`/`xor` reductions §7.12.3, `unique` locator
    /// §7.12.1), OR a qualifier keyword accepted defensively so a legacy member
    /// literally named `unique0`/`priority0` keeps parsing in dot position now
    /// that those are keywords. Reading the source span keeps the segment name
    /// intact regardless of token kind.
    fn member_ident(&mut self) -> Option<Ident> {
        if self.is_ident()
            || self.at_kw(Kw::And)
            || self.at_kw(Kw::Or)
            || self.at_kw(Kw::Xor)
            || self.at_kw(Kw::Unique)
            || self.at_kw(Kw::Unique0)
            || self.at_kw(Kw::Priority0)
        {
            let t = self.bump().unwrap();
            Some(Ident {
                name: self.src[t.span.clone()].to_string(),
                span: Self::sp(&t.span),
            })
        } else {
            self.error("member name");
            None
        }
    }
    fn hier_path(&mut self) -> Option<HierPath> {
        let first = self.ident()?;
        let lo = first.span;
        let mut segs = vec![first];
        while self.peek() == Some(TokenKind::Dot) {
            self.bump();
            match self.member_ident() {
                Some(id) => segs.push(id),
                None => break,
            }
        }
        let hi = segs.last().unwrap().span;
        Some(HierPath {
            segments: segs,
            span: lo.to(hi),
        })
    }

    // ───────────────────────── recovery ─────────────────────────
    /// Panic-mode: skip to a sync anchor. Consumes a `;`; stops AT a top-level
    /// keyword. Note: block-terminator keywords (`end`/`endcase`/`endfunction`/…)
    /// are stop-anchors so PR2 statement recovery lands on the right boundary
    /// (verdict m4 pre-emptive). Always makes ≥0 progress; the body loop's
    /// forward-progress guard (parse_module) handles the no-progress case.
    fn synchronize(&mut self) {
        while let Some(k) = self.peek() {
            match k {
                TokenKind::Semi => {
                    self.bump();
                    return;
                }
                TokenKind::Word(WordKind::Keyword(
                    Kw::End
                    | Kw::Endmodule
                    | Kw::Endcase
                    | Kw::Endfunction
                    | Kw::Endtask
                    | Kw::Endgenerate
                    | Kw::Join
                    | Kw::Module
                    | Kw::Macromodule
                    | Kw::Assign
                    | Kw::Input
                    | Kw::Output
                    | Kw::Inout
                    | Kw::Wire
                    | Kw::Tri
                    | Kw::Wand
                    | Kw::Triand
                    | Kw::Wor
                    | Kw::Trior
                    | Kw::Tri0
                    | Kw::Tri1
                    | Kw::Supply0
                    | Kw::Supply1
                    | Kw::Trireg
                    | Kw::Uwire
                    | Kw::Reg
                    | Kw::Logic
                    | Kw::Integer
                    | Kw::Real
                    | Kw::Realtime
                    | Kw::Time
                    | Kw::Bit
                    | Kw::Byte
                    | Kw::Shortint
                    | Kw::Int
                    | Kw::Longint
                    | Kw::Parameter
                    | Kw::Localparam
                    | Kw::Initial
                    | Kw::Always
                    | Kw::AlwaysFf
                    | Kw::AlwaysComb
                    | Kw::AlwaysLatch
                    | Kw::Generate
                    | Kw::Genvar
                    | Kw::Defparam,
                )) => return,
                _ => {
                    self.bump();
                }
            }
        }
    }
}

// ─────────────────────── Pratt binding powers ───────────────────────
// Verified against hdl-reference/verilog/03-expressions-operators.md (14-level
// table, 1=highest). Higher bp = binds tighter. Left-assoc ⇒ rbp=lbp+1;
// right-assoc ⇒ rbp=lbp-1. Ternary handled specially in `expr` (NOT in infix_bp).
fn infix_bp(k: TokenKind) -> Option<(u8, u8)> {
    use TokenKind as T;
    Some(match k {
        T::PipePipe => (5, 6),                                // ||   lvl13
        T::AmpAmp => (7, 8),                                  // &&   lvl12
        T::Pipe => (9, 10),                                   // |    lvl11
        T::Caret | T::TildeCaret | T::CaretTilde => (11, 12), // ^ ~^ ^~ lvl10
        T::Amp => (13, 14),                                   // &    lvl9
        T::EqEq | T::BangEq | T::EqEqEq | T::BangEqEq | T::EqEqQ | T::BangEqQ => (15, 16), // == != === !== ==? !=? lvl8
        T::Lt | T::LtEq | T::Gt | T::GtEq => (17, 18), // < <= > >= lvl7
        T::Shl | T::Shr | T::ShlA | T::ShrA => (19, 20), // << >> <<< >>> lvl6
        T::Plus | T::Minus => (21, 22),                // + -  lvl5
        T::Star | T::Slash | T::Percent => (23, 24),   // * / % lvl4
        T::StarStar => (26, 27), // **   lvl3 LEFT-assoc (IEEE Table 11-2 / iverilog: `2**2**3` = (2**2)**3)
        _ => return None,
    })
}
const TERNARY_LBP: u8 = 4; // lvl14, right-assoc; rbp = 3
const TERNARY_RBP: u8 = 3;
const UNARY_BP: u8 = 27; // lvl2, prefix right-assoc — binds tighter than **

/// Flat bit index used to encode a DROPPED struct-member bit-select WRITE (OOB
/// source index): any net is at most `MAX_NET_WIDTH` (1<<20) bits, so a write to
/// bit `1<<21` always falls off the end and the engine drops it (no-op) — exactly
/// iverilog's behaviour for `s.f[i]` with `i` past the member, with no risk of
/// leaking into a neighbouring member however wide the struct.
const OOB_DROP_BIT: u32 = 1 << 21;

fn bin_op(k: TokenKind) -> BinOp {
    use TokenKind as T;
    match k {
        T::StarStar => BinOp::Pow,
        T::Star => BinOp::Mul,
        T::Slash => BinOp::Div,
        T::Percent => BinOp::Mod,
        T::Plus => BinOp::Add,
        T::Minus => BinOp::Sub,
        T::Shl => BinOp::Shl,
        T::Shr => BinOp::Shr,
        T::ShlA => BinOp::AShl,
        T::ShrA => BinOp::AShr,
        T::Lt => BinOp::Lt,
        T::LtEq => BinOp::Le,
        T::Gt => BinOp::Gt,
        T::GtEq => BinOp::Ge,
        T::EqEq => BinOp::Eq,
        T::BangEq => BinOp::Ne,
        T::EqEqEq => BinOp::CaseEq,
        T::BangEqEq => BinOp::CaseNe,
        T::EqEqQ => BinOp::WildEq,
        T::BangEqQ => BinOp::WildNe,
        T::Amp => BinOp::BitAnd,
        T::Caret => BinOp::BitXor,
        T::TildeCaret | T::CaretTilde => BinOp::BitXnor,
        T::Pipe => BinOp::BitOr,
        T::AmpAmp => BinOp::LogAnd,
        T::PipePipe => BinOp::LogOr,
        _ => unreachable!("bin_op called on non-binary token"),
    }
}
/// The default signedness of a declared kind when no `signed`/`unsigned` is given:
/// the 2-state integer atom types (and `integer`) default SIGNED (IEEE §6.11.1);
/// nets / `reg` / `logic` / `bit` default unsigned.
fn atom_default_signed(kind: Option<NetVarKind>) -> bool {
    matches!(
        kind,
        Some(
            NetVarKind::Byte
                | NetVarKind::Shortint
                | NetVarKind::Int
                | NetVarKind::Longint
                | NetVarKind::Integer
        )
    )
}

/// Build a binary `Expr` spanning its operands (used by the `inside` desugar).
fn mk_bin(op: BinOp, l: Expr, r: Expr) -> Expr {
    let span = l.span.to(r.span);
    Expr {
        kind: ExprKind::Binary {
            op,
            lhs: Box::new(l),
            rhs: Box::new(r),
        },
        span,
    }
}

fn prefix_op(k: TokenKind) -> Option<UnOp> {
    use TokenKind as T;
    Some(match k {
        T::Plus => UnOp::Plus,
        T::Minus => UnOp::Minus,
        T::Bang => UnOp::LogNot,
        T::Tilde => UnOp::BitNot,
        T::Amp => UnOp::RedAnd,
        T::TildeAmp => UnOp::RedNand,
        T::Pipe => UnOp::RedOr,
        T::TildePipe => UnOp::RedNor,
        T::Caret => UnOp::RedXor,
        T::TildeCaret | T::CaretTilde => UnOp::RedXnor,
        _ => return None,
    })
}
/// True for any operator-class token that can legally appear in INFIX position.
/// Used after the Pratt loop to detect a leftover operator (verdict B1): e.g.
/// `~&`/`~|`/`~` are pure-unary, so `a ~& b` would otherwise silently truncate.
fn is_operatorish(k: TokenKind) -> bool {
    use TokenKind as T;
    infix_bp(k).is_some()
        || matches!(
            k,
            T::Question | T::TildeAmp | T::TildePipe | T::Tilde | T::Bang | T::StarStar
        )
}

impl<'t, 's> Parser<'t, 's> {
    /// Pratt entry. `min_bp` = caller's right binding power. After the fold loop,
    /// if the next token is operator-class but matched no infix slot, emit one
    /// error (verdict B1: do not silently leave `~& b` unconsumed).
    /// P2-5 guard: cap the expression recursion so deep nesting is a clean
    /// parse error, never a SIGSEGV. 256 is ≫ any real RTL expression (deepest
    /// practical cones are <50) and fits a default 2 MiB test-thread stack
    /// even in debug builds (~5 fat frames per paren level).
    const MAX_EXPR_DEPTH: u32 = 256;

    /// PARSE-CONCAT-CAP global budget on parsed expression nodes (user decision,
    /// 2026-06-22). 2^21 ≈ 2.1 M nodes × 80 B ≈ 168 MiB of `Expr` — ~80,000× the
    /// largest concat in the test corpus (26 elements), so any realistic v1
    /// single-file design is far below it, while a `{a,a,…,4M}` flood is a loud,
    /// bounded parse error instead of an OOM.
    const MAX_AST_NODES: usize = 1 << 21;

    pub fn expr(&mut self, min_bp: u8) -> Expr {
        self.expr_depth += 1;
        if self.expr_depth > Self::MAX_EXPR_DEPTH {
            self.expr_depth -= 1;
            self.error("expression nesting too deep (cap 256)");
            return Expr {
                kind: ExprKind::Error,
                span: self.cur_span(),
            };
        }
        // PARSE-CONCAT-CAP: count every expression node; past the budget, latch
        // `node_budget_blown` (the expr comma-loops check it and stop pushing) and
        // report once. Returns an Error leaf so no further nodes are built here.
        self.node_count += 1;
        if self.node_count > Self::MAX_AST_NODES {
            if !self.node_budget_blown {
                self.node_budget_blown = true;
                self.error("expression too large (AST node budget 2097152 exceeded)");
            }
            self.expr_depth -= 1;
            return Expr {
                kind: ExprKind::Error,
                span: self.cur_span(),
            };
        }
        let r = self.expr_capped(min_bp);
        self.expr_depth -= 1;
        r
    }

    fn expr_capped(&mut self, min_bp: u8) -> Expr {
        let mut lhs = self.expr_prefix();
        loop {
            let Some(op) = self.peek() else { break };
            if op == TokenKind::Question {
                // ternary, right-assoc
                if TERNARY_LBP < min_bp {
                    break;
                }
                self.bump();
                let then_e = self.expr(0); // reset inside branch
                self.expect(TokenKind::Colon, "':' in conditional");
                let else_e = self.expr(TERNARY_RBP); // right-assoc
                let span = lhs.span.to(else_e.span);
                lhs = Expr {
                    kind: ExprKind::Ternary {
                        cond: Box::new(lhs),
                        then_e: Box::new(then_e),
                        else_e: Box::new(else_e),
                    },
                    span,
                };
                continue;
            }
            // `lhs inside { items }` (IEEE §11.4.13): a SET-membership test that
            // desugars at parse time to an OR of equality / range tests (relational
            // binding power, lvl7) — so there is no new AST node, and it works in
            // constraints AND ordinary `if (x inside {…})` for free.
            if self.at_ident_kw("inside") {
                if 17 < min_bp {
                    break;
                }
                self.bump(); // `inside`
                lhs = self.parse_inside(lhs);
                continue;
            }
            // `value dist { item, … }` (IEEE §18.5.4) — weighted distribution.
            // Relational binding power (lvl7), like `inside`.
            if self.at_ident_kw("dist") {
                if 17 < min_bp {
                    break;
                }
                self.bump(); // `dist`
                lhs = self.parse_dist(lhs);
                continue;
            }
            // A `with` postfix on a method CALL: `obj.randomize() with { c; … }`
            // (IEEE §18.7) OR the array-method `arr.sum()/find() with (expr)`
            // iterator clause (IEEE §7.12). Both dispatch into ONE `#[inline(never)]`
            // helper (`parse_with_postfix`, brace vs paren) so this hot recursive
            // frame stays small — the expr-depth cap relies on `expr_capped` not
            // growing (`depth_guard.rs` deep-nesting test; a second inline branch
            // here tipped it into a stack overflow).
            if self.at_ident_kw("with")
                && matches!(self.peek_at(1), Some(TokenKind::LBrace | TokenKind::LParen))
                && matches!(lhs.kind, ExprKind::Call { .. })
            {
                lhs = self.parse_with_postfix(lhs);
                continue;
            }
            // `a -> b` constraint/property implication ≡ `!a || b`. Lowest binding
            // (below ternary), right-assoc; desugared at parse time (no new node).
            // A LEADING `->` is an event-trigger STATEMENT (handled at stmt level),
            // so reaching here means infix position.
            if op == TokenKind::Arrow {
                const IMP_LBP: u8 = 2;
                const IMP_RBP: u8 = 1;
                if IMP_LBP < min_bp {
                    break;
                }
                self.bump(); // ->
                let rhs = self.expr(IMP_RBP);
                let lspan = lhs.span;
                let not_lhs = Expr {
                    kind: ExprKind::Unary {
                        op: UnOp::LogNot,
                        operand: Box::new(lhs),
                    },
                    span: lspan,
                };
                lhs = mk_bin(BinOp::LogOr, not_lhs, rhs);
                continue;
            }
            let Some((l_bp, r_bp)) = infix_bp(op) else {
                break;
            };
            if l_bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.expr(r_bp);
            let span = lhs.span.to(rhs.span);
            lhs = Expr {
                kind: ExprKind::Binary {
                    op: bin_op(op),
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            };
        }
        // B1: leftover operator that is not a valid infix continuation
        if min_bp == 0 {
            if let Some(op) = self.peek() {
                if is_operatorish(op) && infix_bp(op).is_none() && op != TokenKind::Question {
                    self.error("operator (got a unary-only operator in infix position)");
                }
            }
        }
        lhs
    }

    /// Desugar `lhs inside { item, … }` to an OR of equality (`lhs == v`) and range
    /// (`lhs >= lo && lhs <= hi`) tests. `lhs` is cloned per item (constraint / `if`
    /// operands are side-effect-free). An empty set never matches (`1'b0`).
    fn parse_inside(&mut self, lhs: Expr) -> Expr {
        let start = lhs.span;
        self.expect(TokenKind::LBrace, "'{' to open an `inside` set");
        let mut terms: Vec<Expr> = Vec::new();
        while self.peek() != Some(TokenKind::RBrace) && self.peek().is_some() {
            let before = self.pos;
            let term = if self.peek() == Some(TokenKind::LBracket) {
                self.bump(); // [
                let lo = self.expr(0);
                self.expect(TokenKind::Colon, "':' in an `inside` range");
                let hi = self.expr(0);
                self.expect(TokenKind::RBracket, "']' to close an `inside` range");
                let ge = mk_bin(BinOp::Ge, lhs.clone(), lo);
                let le = mk_bin(BinOp::Le, lhs.clone(), hi);
                mk_bin(BinOp::LogAnd, ge, le)
            } else {
                let v = self.expr(0);
                mk_bin(BinOp::Eq, lhs.clone(), v)
            };
            terms.push(term);
            if self.peek() == Some(TokenKind::Comma) {
                self.bump();
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "'}' to close an `inside` set");
        let span = start.to(self.prev_span());
        let mut it = terms.into_iter();
        let mut acc = it.next().unwrap_or(Expr {
            kind: ExprKind::IntLit {
                kind: IntLitKind::Sized,
                raw: "1'b0".to_string(),
            },
            span,
        });
        for t in it {
            acc = mk_bin(BinOp::LogOr, acc, t);
        }
        Expr {
            kind: acc.kind,
            span,
        }
    }

    /// Parse `value dist { item, … }` into `ExprKind::Dist`. Each item is a single
    /// value or a `[lo:hi]` range, optionally followed by `:= weight` (per-value) or
    /// `:/ weight` (weight spread over the range); the default weight is `:= 1`.
    fn parse_dist(&mut self, lhs: Expr) -> Expr {
        let start = lhs.span;
        self.expect(TokenKind::LBrace, "'{' to open a `dist` set");
        let mut items: Vec<DistItem> = Vec::new();
        while self.peek() != Some(TokenKind::RBrace) && self.peek().is_some() {
            let before = self.pos;
            let (lo, hi) = if self.peek() == Some(TokenKind::LBracket) {
                self.bump(); // [
                let lo = self.expr(0);
                self.expect(TokenKind::Colon, "':' in a `dist` range");
                let hi = self.expr(0);
                self.expect(TokenKind::RBracket, "']' to close a `dist` range");
                (lo, Some(Box::new(hi)))
            } else {
                (self.expr(0), None)
            };
            // weight: `:= w` (per-value) or `:/ w` (spread); default `:= 1`.
            let (weight, per_range) = if self.peek() == Some(TokenKind::Colon) {
                self.bump(); // :
                let per_range = if self.eat(TokenKind::Slash) {
                    true
                } else {
                    self.expect(TokenKind::Eq, "'=' or '/' after ':' in a `dist` weight");
                    false
                };
                (Box::new(self.expr(0)), per_range)
            } else {
                (
                    Box::new(Expr {
                        kind: ExprKind::IntLit {
                            kind: IntLitKind::Decimal,
                            raw: "1".to_string(),
                        },
                        span: self.cur_span(),
                    }),
                    false,
                )
            };
            items.push(DistItem {
                lo: Box::new(lo),
                hi,
                weight,
                per_range,
            });
            if self.peek() == Some(TokenKind::Comma) {
                self.bump();
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "'}' to close a `dist` set");
        let span = start.to(self.prev_span());
        Expr {
            kind: ExprKind::Dist {
                value: Box::new(lhs),
                items,
            },
            span,
        }
    }

    fn expr_prefix(&mut self) -> Expr {
        if let Some(op) = self.peek().and_then(prefix_op) {
            let start = self.cur_span();
            self.bump();
            let operand = self.expr(UNARY_BP); // lvl2 right-assoc, tighter than **
            let span = start.to(operand.span);
            return Expr {
                kind: ExprKind::Unary {
                    op,
                    operand: Box::new(operand),
                },
                span,
            };
        }
        self.expr_postfix()
    }

    /// primary, then postfix loop: [idx]/[m:l]/[b+:w]; call(args) handled in primary.
    fn expr_postfix(&mut self) -> Expr {
        let mut e = self.expr_primary();
        loop {
            match self.peek() {
                Some(TokenKind::LBracket) => e = self.parse_select(e),
                // HIER-REST②: a `.` after a `name[idx]` select is a hierarchical
                // reference into a generate / instance-array element (`g[0].x`,
                // `bank[3].c.r`). Fold the CONSTANT index into the scope-segment name
                // (`g`+`[0]` ⇒ `g[0]`) so the normal hierarchical resolver handles it
                // — no new IR. (A deeper `g[0].sub[2].y` re-enters via this loop.)
                Some(TokenKind::Dot) if Self::is_indexed_hier_base(&e) => {
                    e = self.parse_indexed_hier(e);
                }
                // SV size/typedef cast `N'(e)` / `(W+1)'(e)` / `name'(e)` (§6.24):
                // a `'(` immediately after a primary. The already-parsed primary
                // `e` IS the casting type — do NOT re-parse it. REQUIRES `(` after
                // the apostrophe; a bare `'` or `'{` (assignment pattern, still
                // unsupported) is left for the diagnostic, never silently eaten.
                Some(TokenKind::Apostrophe) if self.peek_at(1) == Some(TokenKind::LParen) => {
                    e = self.parse_size_or_named_cast(e);
                }
                _ => break,
            }
        }
        e
    }

    /// Finish a size/typedef cast whose casting type is the already-parsed primary
    /// `ty` and whose cursor is at `'(`. A plain `Ident` casting type → `Named`
    /// (resolved at elaborate; class casts loud-reject there); any other primary
    /// (number, `(W+1)`, arithmetic) → `Size`. (SV §6.24.)
    fn parse_size_or_named_cast(&mut self, ty: Expr) -> Expr {
        let start = ty.span;
        self.bump(); // '
        self.expect(TokenKind::LParen, "'(' to open a cast");
        let operand = self.expr(0);
        self.expect(TokenKind::RParen, "')' closing a cast");
        let end = self.prev_span();
        // B2 (§6.24.1): `T'(e)` where T is a simple vector/enum typedef → a numeric
        // cast to T's width and sign. Desugar to the composition of the existing
        // built-in casts `(signed'|unsigned')(W'(e))` — the size cast sets the
        // width (extending with the OPERAND's sign), then the signing cast stamps
        // T's signedness. A struct/union/class/multi-dim/atom/2-state typedef has
        // no simple (width, signed) form and stays loud (the `Named` arm).
        if let ExprKind::Ident(path) = &ty.kind {
            if path.segments.len() == 1 {
                if let Some((w, signed)) = self.simple_typedef_cast(&path.segments[0].name) {
                    let width_lit = Expr {
                        kind: ExprKind::IntLit {
                            kind: IntLitKind::Decimal,
                            raw: w.to_string(),
                        },
                        span: start,
                    };
                    let inner = Expr {
                        kind: ExprKind::Cast {
                            target: CastTarget::Size(Box::new(width_lit)),
                            expr: Box::new(operand),
                        },
                        span: start,
                    };
                    return Expr {
                        kind: ExprKind::Cast {
                            target: CastTarget::Signing { signed },
                            expr: Box::new(inner),
                        },
                        span: start.to(end),
                    };
                }
            }
        }
        let target = match ty.kind {
            ExprKind::Ident(path) => CastTarget::Named(path),
            _ => CastTarget::Size(Box::new(ty)),
        };
        Expr {
            kind: ExprKind::Cast {
                target,
                expr: Box::new(operand),
            },
            span: start.to(end),
        }
    }

    /// A user typedef name usable as a NUMERIC cast `T'(e)` (B2) → its
    /// `(width, signed)`. `Some` only for a simple 4-state vector or
    /// enum-with-logic-base typedef whose range folds to a literal; `None`
    /// (→ honest-loud at the `Named` cast arm) for a struct / union / class /
    /// multi-dim-packed / atom (no range, e.g. base-less enum / `int`) / 2-state
    /// (`bit`) typedef — those need per-field or 2-state-coercion semantics the
    /// size+sign desugar cannot reproduce.
    fn simple_typedef_cast(&self, name: &str) -> Option<(i64, bool)> {
        let info = self.typedefs.get(name)?;
        if self.struct_layouts.contains_key(name)
            || self.union_type_names.contains(name)
            || info.class_name.is_some()
            || !info.packed.is_empty()
            || !matches!(info.kind, NetVarKind::Logic | NetVarKind::Reg)
        {
            return None;
        }
        let range = info.range.as_ref()?;
        let msb = Self::const_lit(&range.msb)?;
        let lsb = Self::const_lit(&range.lsb)?;
        // Direction-agnostic width (overflow-safe `abs_diff`, matching
        // `member_width`); the range direction does not affect the cast VALUE.
        Some((msb.abs_diff(lsb) as i64 + 1, info.signed))
    }

    /// Map a casting-type keyword (`int`/`byte`/…/`signed`/`unsigned`) to its
    /// `CastTarget`, or `None` if the keyword is not a casting type. `realtime'`
    /// folds to `real`. (SV §6.24.)
    fn cast_type_kw(kw: Kw) -> Option<CastTarget> {
        use CastPrim as P;
        use CastTarget as C;
        Some(match kw {
            Kw::Int => C::Prim(P::Int),
            Kw::Integer => C::Prim(P::Integer),
            Kw::Byte => C::Prim(P::Byte),
            Kw::Shortint => C::Prim(P::Shortint),
            Kw::Longint => C::Prim(P::Longint),
            Kw::Bit => C::Prim(P::Bit),
            Kw::Logic => C::Prim(P::Logic),
            Kw::Reg => C::Prim(P::Reg),
            Kw::Time => C::Prim(P::Time),
            Kw::Real | Kw::Realtime => C::Prim(P::Real),
            Kw::Signed => C::Signing { signed: true },
            Kw::Unsigned => C::Signing { signed: false },
            _ => return None,
        })
    }

    /// Finish a keyword cast `int'(e)` / `signed'(e)` whose cursor is at the type
    /// keyword and where `'(` is already confirmed to follow. (SV §6.24.)
    fn parse_keyword_cast(&mut self, target: CastTarget) -> Expr {
        let start = self.cur_span();
        self.bump(); // type keyword
        self.bump(); // '
        self.expect(TokenKind::LParen, "'(' to open a cast");
        let operand = self.expr(0);
        self.expect(TokenKind::RParen, "')' closing a cast");
        Expr {
            kind: ExprKind::Cast {
                target,
                expr: Box::new(operand),
            },
            span: start.to(self.prev_span()),
        }
    }

    /// True when `e` is `name[idx]` / `path[idx]` — a bit-select rooted at a plain
    /// Ident, the shape a following `.` turns into a generate/instance-array
    /// hierarchical reference. (HIER-REST②.)
    fn is_indexed_hier_base(e: &Expr) -> bool {
        matches!(&e.kind, ExprKind::BitSelect { base, .. }
            if matches!(base.kind, ExprKind::Ident(_)))
    }

    /// Parse `path[idx].member(.member)*` into a hierarchical `Ident` whose indexed
    /// segment folds the CONSTANT index into the scope-segment name. Reuses the normal
    /// hierarchical resolver (no new AST/IR). A non-plain-decimal index is a loud parse
    /// error (documented sub-limitation). (HIER-REST②.)
    fn parse_indexed_hier(&mut self, base: Expr) -> Expr {
        let start = base.span;
        let mut segs: Vec<Ident> = Vec::new();
        if let ExprKind::BitSelect { base: b, index } = base.kind {
            if let ExprKind::Ident(p) = b.kind {
                let n = p.segments.len();
                for (i, seg) in p.segments.into_iter().enumerate() {
                    if i + 1 == n {
                        let idx_str = self.const_index_string(&index);
                        segs.push(Ident {
                            name: format!("{}[{idx_str}]", seg.name),
                            span: seg.span,
                        });
                    } else {
                        segs.push(seg);
                    }
                }
            }
        }
        // Consume `.member` segments (plain names; a following `[k].` re-enters the
        // outer postfix loop, a leaf `[k]` is a normal bit-select on the whole path).
        while self.eat(TokenKind::Dot) {
            match self.ident() {
                Some(id) => segs.push(id),
                None => break,
            }
        }
        let hi = segs.last().map(|s| s.span).unwrap_or(start);
        Expr {
            kind: ExprKind::Ident(HierPath {
                segments: segs,
                span: start.to(hi),
            }),
            span: start.to(hi),
        }
    }

    /// Const-fold a generate-array hier index to its non-negative value: a decimal
    /// literal, literal arithmetic (`P-1`, `1+2`), or a module-scope literal-valued
    /// `localparam` (recorded in `const_locals`). A `parameter` / param-derived value
    /// is NOT in `const_locals`, so it does not fold (its index stays loud — folding
    /// it could disagree with an instance override). Returns `None` if not foldable.
    fn try_const_index(&self, e: &Expr) -> Option<i64> {
        match &e.kind {
            ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw,
            } => raw
                .chars()
                .filter(|c| *c != '_')
                .collect::<String>()
                .parse::<i64>()
                .ok(),
            ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.const_locals.get(&p.segments[0].name).copied()
            }
            ExprKind::Paren { inner } => self.try_const_index(inner),
            ExprKind::Unary {
                op: UnOp::Minus,
                operand,
            } => Some(-self.try_const_index(operand)?),
            ExprKind::Binary { op, lhs, rhs } => {
                let a = self.try_const_index(lhs)?;
                let b = self.try_const_index(rhs)?;
                match op {
                    BinOp::Add => Some(a + b),
                    BinOp::Sub => Some(a - b),
                    BinOp::Mul => Some(a * b),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Format a CONSTANT generate-array index as a decimal scope-segment string. A
    /// decimal literal (`g[0]`), literal arithmetic, or a literal-valued
    /// `localparam` (`g[P]`) folds; anything else (a `parameter`, a runtime value)
    /// is a loud parse error.
    fn const_index_string(&mut self, idx: &Expr) -> String {
        if let Some(v) = self.try_const_index(idx) {
            if v >= 0 {
                // Normalize the value (strip leading zeros: `g[00]` ⇒ scope `g[0]`).
                return v.to_string();
            }
        }
        self.error_at(
            idx.span,
            "a constant generate-array index (decimal literal or literal-valued \
             localparam) in a hierarchical reference",
        );
        "0".to_string()
    }

    fn parse_select(&mut self, base: Expr) -> Expr {
        let start = base.span;
        self.bump(); // '['
        let first = self.expr(0);
        let kind = match self.peek() {
            Some(TokenKind::Colon) => {
                self.bump();
                let lsb = self.expr(0);
                ExprKind::PartSelect {
                    base: Box::new(base),
                    msb: Box::new(first),
                    lsb: Box::new(lsb),
                }
            }
            Some(TokenKind::PlusColon) => {
                self.bump();
                let w = self.expr(0);
                ExprKind::IndexedPart {
                    base: Box::new(base),
                    offset: Box::new(first),
                    width: Box::new(w),
                    dir: PartDir::PlusColon,
                }
            }
            Some(TokenKind::MinusColon) => {
                self.bump();
                let w = self.expr(0);
                ExprKind::IndexedPart {
                    base: Box::new(base),
                    offset: Box::new(first),
                    width: Box::new(w),
                    dir: PartDir::MinusColon,
                }
            }
            _ => ExprKind::BitSelect {
                base: Box::new(base),
                index: Box::new(first),
            },
        };
        self.expect(TokenKind::RBracket, "']'");
        Expr {
            kind,
            span: start.to(self.prev_span()),
        }
    }

    /// Parse a POSITIONAL assignment pattern `'{e0, e1, …, eN}` (cursor at `'`).
    /// A `default:` or `key:value` (named) element is loud-rejected — only the
    /// positional form is supported (v1). A replicated `'{N{e}}` parses `N`, then
    /// the trailing `{` fails the `,`/`}` expectation → a loud parse error (which
    /// matches iverilog, that also rejects it).
    fn parse_assign_pattern(&mut self) -> Expr {
        let start = self.cur_span();
        self.bump(); // '
        self.expect(TokenKind::LBrace, "'{' to open an assignment pattern");
        let mut elems = Vec::new();
        if self.peek() != Some(TokenKind::RBrace) {
            loop {
                if self.at_kw(Kw::Default) {
                    self.error(
                        "a `default:` assignment pattern (only positional `'{e0,…}` is supported)",
                    );
                    while !self.at_eof() && self.peek() != Some(TokenKind::RBrace) {
                        self.bump();
                    }
                    break;
                }
                let e = self.expr(0);
                if self.peek() == Some(TokenKind::Colon) {
                    self.error(
                        "a named `key:value` assignment pattern (only positional `'{e0,…}` is supported)",
                    );
                    while !self.at_eof() && self.peek() != Some(TokenKind::RBrace) {
                        self.bump();
                    }
                    break;
                }
                elems.push(e);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RBrace, "'}' closing an assignment pattern");
        Expr {
            kind: ExprKind::AssignPattern(elems),
            span: start.to(self.prev_span()),
        }
    }

    fn expr_primary(&mut self) -> Expr {
        use TokenKind as T;
        let start = self.cur_span();
        match self.peek() {
            // lexer error sentinel: skip it (already diagnosed), yield Error node
            Some(T::Error(_)) => {
                self.bump();
                Expr {
                    kind: ExprKind::Error,
                    span: start,
                }
            }
            // SV §10.9 positional assignment pattern `'{e0, e1, …}`. The lexer
            // keeps `'{` as Apostrophe + LBrace (a cast is `'(`), so a `'` followed
            // by `{` opens an assignment pattern. Named/`default:`/replicated forms
            // are loud (only positional is supported — elaborate binds it to a 1-D
            // unpacked array).
            Some(T::Apostrophe) if self.peek_at(1) == Some(T::LBrace) => {
                self.parse_assign_pattern()
            }
            // SV type/signing cast `int'(e)` / `signed'(e)` (§6.24): a casting-type
            // keyword followed by `'(`. The guard requires `'(` so a bare type kw
            // (not a primary today) still falls to the `_ => error` arm unchanged.
            Some(T::Word(WordKind::Keyword(kw)))
                if self.peek_at(1) == Some(T::Apostrophe)
                    && self.peek_at(2) == Some(T::LParen)
                    && Self::cast_type_kw(kw).is_some() =>
            {
                self.parse_keyword_cast(Self::cast_type_kw(kw).unwrap())
            }
            // numeric / string literals
            Some(T::IntDecimal) => self.lit_int(IntLitKind::Decimal),
            Some(T::IntSized) => self.lit_int(IntLitKind::Sized),
            Some(T::IntUnsizedBased) => self.lit_int(IntLitKind::UnsizedBased),
            Some(T::RealFixed) => self.lit_real(RealLitKind::Fixed),
            Some(T::RealExp) => self.lit_real(RealLitKind::Exp),
            Some(T::Str) => {
                let raw = self.cur_text().to_string();
                self.bump();
                Expr {
                    kind: ExprKind::StrLit { raw },
                    span: start,
                }
            }
            // system function call: $time, $signed(x). name retains the `$`.
            Some(T::SystemTask) => {
                let t = self.bump().unwrap();
                let name = Ident {
                    name: self.src[t.span.clone()].to_string(),
                    span: Self::sp(&t.span),
                };
                let args = if self.peek() == Some(T::LParen) {
                    self.call_args()
                } else {
                    Vec::new()
                };
                Expr {
                    kind: ExprKind::SysCall { name, args },
                    span: start.to(self.prev_span()),
                }
            }
            // v5 ⑥: bare `$` — queue last-index (`q[$]`, `q[$-1]`). A primary
            // so Pratt arithmetic folds over it; elaborate substitutes
            // `size()-1` inside a queue select and loud-rejects it elsewhere.
            Some(T::Dollar) => {
                self.bump();
                Expr {
                    kind: ExprKind::Dollar,
                    span: start,
                }
            }
            // N7: `null` — the null class-handle literal.
            Some(T::Word(WordKind::Keyword(Kw::Null))) => {
                self.bump();
                Expr {
                    kind: ExprKind::Null,
                    span: start,
                }
            }
            // identifier / hierarchical name / function call
            _ if self.is_ident() => {
                let path = self.hier_path().unwrap();
                // v7 P2-D: `pkg::name` package-scoped value reference.
                if path.segments.len() == 1 && self.peek() == Some(T::ColonColon) {
                    self.bump(); // '::'
                    if let Some(name) = self.ident() {
                        return Expr {
                            kind: ExprKind::PkgScoped {
                                pkg: path.segments.into_iter().next().unwrap(),
                                name,
                            },
                            span: start.to(self.prev_span()),
                        };
                    }
                    return Expr {
                        kind: ExprKind::Error,
                        span: start.to(self.prev_span()),
                    };
                }
                // v5 ⑥: contextual `new[n]` / `new[n](src)` — the ident `new`
                // immediately followed by `[`. Elaborate falls back to an
                // array read when a net named `new` is actually in scope
                // (V2005 keeps `new` as an ordinary identifier).
                if path.segments.len() == 1
                    && path.segments[0].name == "new"
                    && self.peek() == Some(T::LBracket)
                {
                    self.bump(); // '['
                    let size = self.expr(0);
                    self.expect(T::RBracket, "']'");
                    let src = if self.peek() == Some(T::LParen) {
                        self.bump();
                        let s = self.expr(0);
                        self.expect(T::RParen, "')'");
                        Some(Box::new(s))
                    } else {
                        None
                    };
                    return Expr {
                        kind: ExprKind::New {
                            size: Box::new(size),
                            src,
                        },
                        span: start.to(self.prev_span()),
                    };
                }
                // N7: contextual class `new` / `new(args)` — the ident `new` NOT
                // followed by `[` (the dyn-array form is handled just above). The
                // class is inferred from the assignment LHS handle at elaborate;
                // a V2005 program using `new` as a plain net is unaffected because
                // elaborate falls back when no class-handle LHS is in play.
                if path.segments.len() == 1 && path.segments[0].name == "new" {
                    let args = if self.peek() == Some(T::LParen) {
                        self.call_args()
                    } else {
                        Vec::new()
                    };
                    return Expr {
                        kind: ExprKind::ClassNew { args },
                        span: start.to(self.prev_span()),
                    };
                }
                // packed-struct member access `s.field` → constant part-select.
                // Extracted to a non-inlined helper so the (rare) struct-field
                // locals never inflate `expr_primary`'s frame on the hot paren-
                // recursion path (the MAX_EXPR_DEPTH stack budget is frame-sized).
                if let Some((base, off, w, asc, sgn)) = self.struct_field_select(&path) {
                    return self.struct_member_expr(base, off, w, asc, sgn, path.span);
                }
                // SV §6.19.5 enum method `x.first/last/num/next/prev/name [()]` —
                // the no-arg form only (a `x.next(2)` step arg falls through to a
                // Call → loud). Desugars to literals / ternary chains over the
                // enum's labels; non-enum `x.foo` returns None → normal path.
                {
                    let empty_call =
                        self.peek() == Some(T::LParen) && self.peek_at(1) == Some(T::RParen);
                    if (self.peek() != Some(T::LParen) || empty_call) && !path.segments.is_empty() {
                        if let Some(e) = self.enum_method_expr(&path) {
                            if empty_call {
                                self.bump(); // (
                                self.bump(); // )
                            }
                            return e;
                        }
                    }
                }
                if self.peek() == Some(T::LParen) {
                    let args = self.call_args();
                    Expr {
                        kind: ExprKind::Call { name: path, args },
                        span: start.to(self.prev_span()),
                    }
                } else {
                    let sp = path.span;
                    Expr {
                        kind: ExprKind::Ident(path),
                        span: sp,
                    }
                }
            }
            // parenthesized / min:typ:max
            Some(T::LParen) => {
                self.bump();
                let inner = self.expr(0);
                if self.peek() == Some(T::Colon) {
                    self.bump();
                    let typ = self.expr(0);
                    self.expect(T::Colon, "':' in min:typ:max");
                    let max = self.expr(0);
                    self.expect(T::RParen, "')'");
                    Expr {
                        kind: ExprKind::MinTypMax {
                            min: Box::new(inner),
                            typ: Box::new(typ),
                            max: Box::new(max),
                        },
                        span: start.to(self.prev_span()),
                    }
                } else {
                    self.expect(T::RParen, "')'");
                    Expr {
                        kind: ExprKind::Paren {
                            inner: Box::new(inner),
                        },
                        span: start.to(self.prev_span()),
                    }
                }
            }
            // concat / replication
            Some(T::LBrace) => self.brace_expr(start),
            _ => {
                self.error("expression");
                Expr {
                    kind: ExprKind::Error,
                    span: start,
                }
            }
        }
    }

    fn lit_int(&mut self, kind: IntLitKind) -> Expr {
        let start = self.cur_span();
        let raw = self.cur_text().to_string();
        self.bump();
        Expr {
            kind: ExprKind::IntLit { kind, raw },
            span: start,
        }
    }
    fn lit_real(&mut self, kind: RealLitKind) -> Expr {
        let start = self.cur_span();
        let raw = self.cur_text().to_string();
        self.bump();
        Expr {
            kind: ExprKind::RealLit { kind, raw },
            span: start,
        }
    }
    fn call_args(&mut self) -> Vec<Expr> {
        self.bump(); // '('
        let mut args = Vec::new();
        if self.peek() != Some(TokenKind::RParen) {
            loop {
                args.push(self.expr(0));
                // PARSE-CONCAT-CAP: stop consuming once the node budget is blown.
                if self.node_budget_blown || !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "')'");
        args
    }
    /// `{a,b}` concat OR `{n{a,b}}` replication. After parsing `first`, a following
    /// `{` ⇒ replication (first=count); the inner braced list becomes `value:
    /// Vec<Expr>` DIRECTLY (verdict M5 — no Concat wrapper). `{ {a},{b} }` is a
    /// concat-of-concats: `first={a}` then next is `,`, so concat path is taken.
    fn brace_expr(&mut self, start: Span) -> Expr {
        self.bump(); // outer '{'
        let first = self.expr(0);
        if self.peek() == Some(TokenKind::LBrace) {
            // replication: first = count, inner {…} = the repeated element list.
            self.bump(); // inner '{'
            let mut value = vec![self.expr(0)];
            while !self.node_budget_blown && self.eat(TokenKind::Comma) {
                value.push(self.expr(0));
            }
            self.expect(TokenKind::RBrace, "'}' closing replication value");
            self.expect(TokenKind::RBrace, "'}' closing replication");
            return Expr {
                kind: ExprKind::Replicate {
                    count: Box::new(first),
                    value,
                },
                span: start.to(self.prev_span()),
            };
        }
        let mut parts = vec![first];
        while !self.node_budget_blown && self.eat(TokenKind::Comma) {
            parts.push(self.expr(0));
        }
        self.expect(TokenKind::RBrace, "'}'");
        Expr {
            kind: ExprKind::Concat { parts },
            span: start.to(self.prev_span()),
        }
    }
}

// ───────────── module / port / param / decl / contassign ─────────────
impl<'t, 's> Parser<'t, 's> {
    /// Parse an optional signing qualifier. Returns `Some(true)` for `signed`,
    /// `Some(false)` for `unsigned`, `None` for neither — so the caller can apply
    /// the TYPE's default (2-state atom types default signed, reg/wire/logic/bit
    /// default unsigned). Conflating `unsigned` with "absent" lost the distinction
    /// for atom types (`int unsigned` was treated as signed).
    fn opt_signed(&mut self) -> Option<bool> {
        if self.eat_kw(Kw::Unsigned) {
            return Some(false);
        }
        if self.eat_kw(Kw::Signed) {
            return Some(true);
        }
        None
    }

    /// Resolve an optional signing qualifier to the EFFECTIVE signedness using the
    /// declared kind's default (atom types `byte`/`shortint`/`int`/`longint`/
    /// `integer` default SIGNED; everything else defaults unsigned).
    fn signed_eff(&mut self, kind: Option<NetVarKind>) -> bool {
        self.opt_signed()
            .unwrap_or_else(|| atom_default_signed(kind))
    }
    /// `[msb:lsb]` packed range (requires `:`).
    fn opt_range(&mut self) -> Option<Range> {
        if self.peek() != Some(TokenKind::LBracket) {
            return None;
        }
        let start = self.cur_span();
        self.bump();
        let msb = self.expr(0);
        self.expect(TokenKind::Colon, "':' in range");
        let lsb = self.expr(0);
        self.expect(TokenKind::RBracket, "']'");
        Some(Range {
            msb,
            lsb,
            span: start.to(self.prev_span()),
        })
    }

    /// Additional packed dims after the first `[msb:lsb]` — `logic [3:0][7:0]` ⇒
    /// `[[7:0]]`. Each is a `[msb:lsb]` range; collected greedily before the name.
    fn opt_packed_dims(&mut self) -> Vec<Range> {
        let mut dims = Vec::new();
        while let Some(r) = self.opt_range() {
            dims.push(r);
        }
        dims
    }

    /// Unpacked dimension `[hi:lo]` (Range) or `[N]` (Size) — verdict M3.
    /// v5 ⑥ adds the dynamic-storage forms: `[]` (dyn array), `[$]`/`[$:N]`
    /// (queue / bounded queue — the bound parses, elaborate loud-rejects it),
    /// `[integer]`/`[time]` (assoc, integer key types only). `[*]` (wildcard
    /// assoc) is a parse error — outside the MVP.
    /// A dimension can start with `[` or — since slice S4 fused `[*` into one
    /// token — `[*` (the wildcard assoc `[*]` spelling). `parse_dim` handles
    /// both; the array-dim loops gate on this so the no-space `[*]` still reaches
    /// the precise wildcard diagnostic instead of a generic token cascade.
    fn at_dim_start(&self) -> bool {
        matches!(
            self.peek(),
            Some(TokenKind::LBracket | TokenKind::LBracketStar)
        )
    }
    fn parse_dim(&mut self) -> Option<Dim> {
        // `[*]` wildcard assoc index. Since slice S4 the lexer fuses `[*` into a
        // single `LBracketStar` token (for SVA `[*n]`), so the canonical no-space
        // spelling never reaches the `Star` arm below — handle it here. Outside
        // the MVP: reject loudly with the precise message, recover as a dyn dim.
        // (The spaced `[ *]` spelling still lexes as `[`+`*` and hits the `Star`
        // arm.)
        if self.peek() == Some(TokenKind::LBracketStar) {
            self.bump(); // `[*`
            self.error(
                "a concrete assoc key type (`[integer]`/`[int]`/`[longint]`/`[time]`/`[string]`/…) — wildcard `[*]` is unsupported",
            );
            self.expect(TokenKind::RBracket, "']'");
            return Some(Dim::Dyn);
        }
        if self.peek() != Some(TokenKind::LBracket) {
            return None;
        }
        self.bump(); // '['
        match self.peek() {
            // `[]` — dynamic array.
            Some(TokenKind::RBracket) => {
                self.bump();
                return Some(Dim::Dyn);
            }
            // `[$]` / `[$:N]` — queue.
            Some(TokenKind::Dollar) => {
                self.bump();
                let bound = if self.peek() == Some(TokenKind::Colon) {
                    self.bump();
                    Some(self.expr(0))
                } else {
                    None
                };
                self.expect(TokenKind::RBracket, "']'");
                return Some(Dim::Queue(bound));
            }
            // `[integer]` / `[time]` / `[int]` / `[longint]` / `[shortint]` /
            // `[byte]` — assoc key type (keyword-led, so it can never shadow a
            // same-named size parameter). Every integral spelling shares the
            // documented signed-i64 key domain (the ⑥ design pin: keys are NOT
            // truncated to the declared width — `[integer]`/`[time]` already
            // behave this way), so the 2-state atoms map onto the same
            // `AssocKey::Integer` lowering with zero AST/schema change.
            Some(TokenKind::Word(WordKind::Keyword(
                k @ (Kw::Integer | Kw::Time | Kw::Int | Kw::Longint | Kw::Shortint | Kw::Byte),
            ))) => {
                self.bump();
                self.expect(TokenKind::RBracket, "']'");
                return Some(Dim::Assoc(if k == Kw::Time {
                    AssocKey::Time
                } else {
                    AssocKey::Integer
                }));
            }
            // `[string]` (v6) — since the v7 AST flip `string` is a real
            // KEYWORD (the P2-C type), so the assoc key form is keyword-led
            // like `[integer]`/`[time]`.
            Some(TokenKind::Word(WordKind::Keyword(Kw::String))) => {
                self.bump();
                self.expect(TokenKind::RBracket, "']'");
                return Some(Dim::Assoc(AssocKey::Str));
            }
            // `[*]` — wildcard assoc index: outside the MVP, reject loudly at
            // parse (recover as a plain dyn dim so the decl still resolves).
            Some(TokenKind::Star) => {
                self.bump();
                self.error(
                    "a concrete assoc key type (`[integer]`/`[int]`/`[longint]`/`[time]`/`[string]`/…) — wildcard `[*]` is unsupported",
                );
                self.expect(TokenKind::RBracket, "']'");
                return Some(Dim::Dyn);
            }
            _ => {}
        }
        let first = self.expr(0);
        let dim = if self.peek() == Some(TokenKind::Colon) {
            let r_start = first.span;
            self.bump();
            let lsb = self.expr(0);
            Dim::Range(Range {
                msb: first,
                lsb,
                span: r_start.to(self.prev_span()),
            })
        } else {
            Dim::Size(first)
        };
        self.expect(TokenKind::RBracket, "']'");
        Some(dim)
    }
    /// v7 P2-D: `import pkg::*;` / `import pkg::sym;` — ONE term per
    /// statement (a comma list is a loud parse error; rare in practice).
    fn parse_import_decl(&mut self) -> Option<ImportDecl> {
        let start = self.cur_span();
        self.bump(); // import
        let pkg = self.ident()?;
        if !self.expect(TokenKind::ColonColon, "'::'") {
            return None;
        }
        let item = if self.peek() == Some(TokenKind::Star) {
            self.bump();
            None
        } else {
            Some(self.ident()?)
        };
        if self.peek() == Some(TokenKind::Comma) {
            self.error("';' (one import term per statement in v7)");
            return None;
        }
        if !self.expect(TokenKind::Semi, "';'") {
            return None;
        }
        Some(ImportDecl {
            pkg,
            item,
            span: start.to(self.prev_span()),
        })
    }

    fn net_var_kind(&self) -> Option<NetVarKind> {
        use Kw::*;
        match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(k))) => Some(match k {
                Wire => NetVarKind::Wire,
                Tri => NetVarKind::Tri,
                Wand => NetVarKind::Wand,
                Triand => NetVarKind::Triand,
                Wor => NetVarKind::Wor,
                Trior => NetVarKind::Trior,
                Tri0 => NetVarKind::Tri0,
                Tri1 => NetVarKind::Tri1,
                Supply0 => NetVarKind::Supply0,
                Supply1 => NetVarKind::Supply1,
                Trireg => NetVarKind::Trireg,
                Uwire => NetVarKind::Uwire,
                Reg => NetVarKind::Reg,
                Logic => NetVarKind::Logic,
                Integer => NetVarKind::Integer,
                Real => NetVarKind::Real,
                Realtime => NetVarKind::Realtime,
                Time => NetVarKind::Time,
                Event => NetVarKind::Event,
                String => NetVarKind::String,
                Bit => NetVarKind::Bit,
                Byte => NetVarKind::Byte,
                Shortint => NetVarKind::Shortint,
                Int => NetVarKind::Int,
                Longint => NetVarKind::Longint,
                _ => return None,
            }),
            _ => None,
        }
    }

    pub fn parse_source_unit(&mut self) -> SourceUnit {
        // N7: pre-scan the token stream for every `class NAME` (any nesting) so a
        // class-typed declaration `Packet p;` parses through the ordinary
        // typed-decl path even when the variable precedes the class decl
        // (forward reference) — registered as a `NetVarKind::Class` type alias.
        self.prescan_class_names();
        let start = self.cur_span();
        let mut items = Vec::new();
        while !self.at_eof() {
            let before = self.pos;
            if self.at_kw(Kw::Module) || self.at_kw(Kw::Macromodule) {
                match self.parse_module() {
                    Some(m) => items.push(TopItem::Module(m)),
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
            } else if self.at_kw(Kw::Interface) {
                // v5 ⑥: `interface … endinterface` — same shape as a module.
                match self.parse_module_like(Kw::Interface, Kw::Endinterface) {
                    Some(m) => items.push(TopItem::Interface(m)),
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
            } else if self.at_kw(Kw::Program) {
                // ⓑ-breadth (§24): `program … endprogram` parses into the module
                // AST and elaborates as a top-level module container. The §24
                // Reactive-region scheduling of program processes is approximated
                // as Active (documented limitation). Pure parser addition (IR-0).
                match self.parse_module_like(Kw::Program, Kw::Endprogram) {
                    Some(m) => items.push(TopItem::Module(m)),
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
            } else if self.at_kw(Kw::Package) {
                // v7 P2-D: `package … endpackage` — body shape reuses modules.
                match self.parse_module_like(Kw::Package, Kw::Endpackage) {
                    Some(m) => items.push(TopItem::Package(m)),
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
            } else if self.at_kw(Kw::Import) {
                // v7 P2-D: compilation-unit-scope import.
                match self.parse_import_decl() {
                    Some(i) => items.push(TopItem::Import(i)),
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
            } else if self.at_kw(Kw::Class) {
                // N7: top-level `class … endclass`.
                match self.parse_class_decl() {
                    Some(c) => items.push(TopItem::Class(c)),
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
            } else if self.at_kw(Kw::Primitive) {
                // YELLOW #1: combinational User-Defined Primitive (IEEE 1364 §29).
                // DESUGARED in the parser (mirroring `parse_gate_primitive`) into a
                // synthetic ordinary `ModuleDecl` — so it auto-registers in the module
                // map, root-picks, and instantiates with ZERO downstream change. No new
                // AST node / no `.vu` schema-hash flip / IR-0.
                match self.parse_udp_decl() {
                    Some(m) => items.push(TopItem::Module(m)),
                    None => {
                        items.push(TopItem::Error(self.prev_span()));
                        self.synchronize();
                    }
                }
            } else {
                self.error("'module'");
                let s = self.cur_span();
                items.push(TopItem::Error(s));
                self.synchronize();
            }
            // BLOCKER B3 (top level): guarantee forward progress.
            if self.pos == before {
                self.bump();
            }
        }
        // ⓑ-breadth (§8.25): expand parameterized classes into concrete
        // specializations (no-op when no class is parameterized).
        self.monomorphize_param_classes(&mut items);
        items.extend(
            std::mem::take(&mut self.pending_mono_specs)
                .into_iter()
                .map(TopItem::Class),
        );
        SourceUnit {
            items,
            span: start.to(self.prev_span()),
        }
    }

    /// ⓑ-breadth (§8.25): MONOMORPHIZE parameterized classes. For each distinct
    /// `C #(args)` used at a handle declaration (plus the all-defaults instance),
    /// generate a fully-concrete class with the parameter values substituted, and
    /// rewrite each handle's type to the specialization. The default specialization
    /// keeps the bare class name (so `C h;` resolves); overrides get a mangled name
    /// (`C__16`). v1: positional INTEGER-LITERAL spec args only (a non-literal arg
    /// is a loud reject — its value is not foldable into a stable specialization).
    fn monomorphize_param_classes(&mut self, items: &mut [TopItem]) {
        use std::collections::BTreeMap;
        // Pass 1: collect parameterized templates by name.
        let mut templates: BTreeMap<String, ClassDecl> = BTreeMap::new();
        let mut collect = |c: &ClassDecl| {
            if !c.params.is_empty() {
                templates.insert(c.name.name.clone(), c.clone());
            }
        };
        for it in items.iter() {
            match it {
                TopItem::Class(c) => collect(c),
                TopItem::Module(m) | TopItem::Interface(m) | TopItem::Package(m) => {
                    for bi in &m.body {
                        if let ModuleItem::Class(c) = bi {
                            collect(c);
                        }
                    }
                }
                _ => {}
            }
        }
        if templates.is_empty() {
            return;
        }
        // Build the param map (param → value-expr) for a given spec's args.
        // Missing trailing args fall back to the parameter default; an unfilled
        // parameter with no default is a loud reject.
        let mut new_specs: Vec<ClassDecl> = Vec::new();
        let mut spec_seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut errors: Vec<(Span, &'static str)> = Vec::new();
        // The map of every (orig handle) → its rewritten class name is applied by a
        // second walk; here we just register the needed specs and remember the name
        // each (template,args) maps to via `mangle`.
        let mangle = |tmpl: &str, args: &[Expr]| -> Option<String> {
            if args.is_empty() {
                return Some(tmpl.to_string());
            }
            let mut parts = Vec::new();
            for a in args {
                parts.push(arg_render(a)?);
            }
            Some(format!("{tmpl}__{}", parts.join("_")))
        };
        // Register a spec (idempotent) given its template + args.
        let mut register = |tmpl_name: &str, args: &[Expr], at: Span| -> String {
            let tmpl = &templates[tmpl_name];
            let Some(name) = mangle(tmpl_name, args) else {
                errors.push((
                    at,
                    "a parameterized class specialization argument (an integer literal, `C #(16)`)",
                ));
                return tmpl_name.to_string();
            };
            if spec_seen.insert(name.clone()) {
                // build param map
                let mut map: BTreeMap<String, Expr> = BTreeMap::new();
                let mut ok = true;
                for (i, p) in tmpl.params.iter().enumerate() {
                    let val = args.get(i).cloned().or_else(|| p.default.clone());
                    match val {
                        Some(v) => {
                            map.insert(p.name.name.clone(), v);
                        }
                        None => {
                            errors.push((
                                at,
                                "a specialization argument for a parameterized class parameter \
                                 that has no default",
                            ));
                            ok = false;
                        }
                    }
                }
                if ok {
                    new_specs.push(monomorphize_class(tmpl, &name, &map));
                }
            }
            name
        };
        // Pass 2: walk every handle decl; rewrite its type and register the spec.
        // Module-level handles (`ModuleItem::NetVar`) and class-field handles
        // (`ClassItem::Property`) are covered.
        fn rewrite_netvar(
            d: &mut NetVarDecl,
            templates: &BTreeMap<String, ClassDecl>,
            register: &mut dyn FnMut(&str, &[Expr], Span) -> String,
        ) {
            let Some(ct) = &d.class_type else { return };
            if !templates.contains_key(&ct.name) {
                return;
            }
            let new_name = register(&ct.name.clone(), &d.class_args, d.span);
            if let Some(ci) = &mut d.class_type {
                ci.name = new_name;
            }
            d.class_args = Vec::new();
        }
        for it in items.iter_mut() {
            match it {
                TopItem::Class(c) => {
                    for item in &mut c.items {
                        if let ClassItem::Property(_, d) = item {
                            rewrite_netvar(d, &templates, &mut register);
                        }
                    }
                }
                TopItem::Module(m) | TopItem::Interface(m) | TopItem::Package(m) => {
                    for bi in &mut m.body {
                        match bi {
                            ModuleItem::NetVar(d) => {
                                rewrite_netvar(d, &templates, &mut register);
                            }
                            ModuleItem::Class(c) => {
                                for item in &mut c.items {
                                    if let ClassItem::Property(_, d) = item {
                                        rewrite_netvar(d, &templates, &mut register);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        // Always register the all-defaults instance for each template (so `C h;`
        // resolves even if every concrete handle overrode the parameters).
        let tmpl_names: Vec<String> = templates.keys().cloned().collect();
        for t in &tmpl_names {
            let _ = register(t, &[], templates[t].name.span);
        }
        for (sp, msg) in errors {
            self.error_at(sp, msg);
        }
        // Pass 3: replace each template ClassDecl with its DEFAULT specialization
        // (bare name) and append the override specializations at top level.
        for it in items.iter_mut() {
            if let TopItem::Class(c) = it {
                if let Some(def) = new_specs.iter().find(|s| s.name.name == c.name.name) {
                    *c = def.clone();
                }
            }
            if let TopItem::Module(m) | TopItem::Interface(m) | TopItem::Package(m) = it {
                for bi in &mut m.body {
                    if let ModuleItem::Class(c) = bi {
                        if let Some(def) = new_specs.iter().find(|s| s.name.name == c.name.name) {
                            *c = def.clone();
                        }
                    }
                }
            }
        }
        // The OVERRIDE specs (mangled names, not matching any template) are added as
        // fresh top-level classes via the returned list (handled by the caller).
        self.pending_mono_specs = new_specs
            .into_iter()
            .filter(|s| !templates.contains_key(&s.name.name))
            .collect();
    }

    fn parse_module(&mut self) -> Option<ModuleDecl> {
        self.parse_module_like(Kw::Module, Kw::Endmodule)
    }

    /// One body shared by `module…endmodule` and `interface…endinterface`
    /// (v5 ⑥): the header/body grammar is identical for the MVP subset.
    fn parse_module_like(&mut self, _start_kw: Kw, end_kw: Kw) -> Option<ModuleDecl> {
        let start = self.cur_span();
        // Variable→struct bindings are module-scoped (type *names* are not).
        self.var_struct.clear();
        self.struct_scalar_vars.clear();
        self.struct_1d_array_vars.clear();
        self.var_enum.clear();
        self.const_locals.clear();
        let is_macromodule = self.at_kw(Kw::Macromodule);
        self.bump(); // module / macromodule / interface
        let name = self.ident()?;

        // ANSI module-header package imports (IEEE §A.1.2 / §26.4): zero or more
        // `import pkg::item;` between the module name and the parameter/port list.
        // They exist so the imported symbols are visible to the port list that
        // follows (`module m import p::*; (input logic [W-1:0] a)`). Collected as
        // `ModuleItem::Import` at the FRONT of the body — elaborate's import pass
        // already scans body imports (and applies them before resolving port
        // widths), so a header import and a body import register identically.
        let mut header_imports = Vec::new();
        while self.at_kw(Kw::Import) {
            match self.parse_import_decl() {
                Some(i) => header_imports.push(ModuleItem::Import(i)),
                None => break, // diagnostic already emitted; stop the header loop
            }
        }

        // ANSI param port list: #( parameter … )
        let mut params = Vec::new();
        if self.peek() == Some(TokenKind::Hash) {
            self.bump();
            self.expect(TokenKind::LParen, "'(' after '#'");
            loop {
                // body=false: an array parameter in the header is a loud error
                // inside `parse_param_decl` (never a `ConstArrayVar` here).
                if let Some(ParamItem::Scalar(p)) = self.parse_param_decl(false) {
                    params.push(p);
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')'");
        }

        // port list: ANSI ( dir type name, … ) | non-ANSI ( name, … ) | none
        let ports = self.parse_port_list();
        self.expect(TokenKind::Semi, "';' after module header");

        // body until the end keyword — with forward-progress guard (BLOCKER B3).
        // Header imports lead the body so elaborate registers them first.
        let mut body = header_imports;
        while !self.at_eof() && !self.at_kw(end_kw) {
            let before = self.pos;
            match self.parse_module_item() {
                Some(it) => body.push(it),
                None => {
                    body.push(ModuleItem::Error(self.cur_span()));
                    self.synchronize();
                }
            }
            if self.pos == before {
                self.bump();
            } // B3: never spin on a stuck token
        }
        self.expect(
            TokenKind::Word(WordKind::Keyword(end_kw)),
            if end_kw == Kw::Endinterface {
                "'endinterface'"
            } else {
                "'endmodule'"
            },
        );
        Some(ModuleDecl {
            is_macromodule,
            name,
            params,
            ports,
            body,
            span: start.to(self.prev_span()),
        })
    }

    /// Decide ANSI vs non-ANSI by the FIRST token inside `(`: a direction keyword
    /// ⇒ ANSI. A bare identifier ⇒ non-ANSI name list. (Documented PR1 limitation,
    /// verdict H2/M1: a malformed ANSI header beginning with a bare net/var kind —
    /// e.g. illegal `module m(reg x)` — is routed to non-ANSI and errors in the body.
    /// Strict V2005 non-ANSI headers are bare-name-only, so this is correct for scope.)
    fn parse_port_list(&mut self) -> PortList {
        if self.peek() != Some(TokenKind::LParen) {
            return PortList::None;
        }
        self.bump(); // '('
        if self.peek() == Some(TokenKind::RParen) {
            self.bump();
            return PortList::Ansi(Vec::new());
        }
        let ansi = matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Input | Kw::Output | Kw::Inout
            )))
        ) ||
            // v5 ⑥: `module m(intf bus, …)` — an interface-typed first port:
            // Ident followed by Ident (`intf bus`) or Dot (`intf.mp bus`).
            (matches!(self.peek(), Some(TokenKind::Word(WordKind::Ident)))
                && matches!(
                    self.peek_at(1),
                    Some(TokenKind::Word(WordKind::Ident) | TokenKind::Dot)
                ));
        if ansi {
            let mut ports: Vec<AnsiPort> = Vec::new();
            loop {
                let prev = ports.last().cloned(); // for dir + type/range inheritance
                let port = self.parse_ansi_port(prev.as_ref());
                ports.push(port);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')'");
            PortList::Ansi(ports)
        } else {
            let mut names = Vec::new();
            loop {
                if let Some(id) = self.ident() {
                    names.push(id);
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')'");
            PortList::NonAnsi(names)
        }
    }

    /// v5 ⑥: `modport name (input a, b, output c);` — the direction is sticky
    /// across commas. Parsed + ACCEPTED (per-member direction checks are a
    /// follow-on); task/function modport members are outside the MVP.
    fn parse_modport(&mut self) -> Option<ModportDecl> {
        let start = self.cur_span();
        self.bump(); // modport
        let name = self.ident()?;
        self.expect(TokenKind::LParen, "'('");
        let mut ports = Vec::new();
        let mut dir: Option<PortDir> = None;
        loop {
            match self.peek() {
                Some(TokenKind::Word(WordKind::Keyword(Kw::Input))) => {
                    self.bump();
                    dir = Some(PortDir::Input);
                }
                Some(TokenKind::Word(WordKind::Keyword(Kw::Output))) => {
                    self.bump();
                    dir = Some(PortDir::Output);
                }
                Some(TokenKind::Word(WordKind::Keyword(Kw::Inout))) => {
                    self.bump();
                    dir = Some(PortDir::Inout);
                }
                _ => {}
            }
            let Some(d) = dir else {
                self.error("a direction (input/output/inout) before the first modport member");
                break;
            };
            let Some(member) = self.ident() else {
                self.error("modport member name");
                break;
            };
            ports.push((d, member));
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "')'");
        self.expect(TokenKind::Semi, "';'");
        Some(ModportDecl {
            name,
            ports,
            span: start.to(self.prev_span()),
        })
    }

    /// `prev = None` ⇒ first ANSI port; a missing direction is then an ERROR
    /// (verdict M4: don't silently default the first port to Input). A comma-continued
    /// port with no direction inherits the previous port's direction; a PURE
    /// continuation (`input [7:0] a, b`) that also omits its own type/range/signed
    /// inherits those too, so both `a` and `b` are `[7:0]` (IEEE 1800 §23.2.2.1).
    fn parse_ansi_port(&mut self, prev: Option<&AnsiPort>) -> AnsiPort {
        let start = self.cur_span();
        // v5 ⑥: interface-typed port `intf p` / `intf.mp p` — an Ident in the
        // type position followed by Ident/Dot. No direction, no range.
        //
        // Guard: if the current Ident is a registered typedef name and the next
        // token is an Ident (port name), this is `typedef_t portname` — a typedef
        // port in a comma-continuation context (e.g. `input byte_t a, word_t c`).
        // Skip the interface heuristic so try_port_typedef() handles it below.
        // (typedef_t followed by `.` cannot be a valid typedef port, so the `.`
        // case is intentionally left to the interface heuristic.)
        let is_typedef_portname_shape = matches!(
            self.peek_at(1),
            Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
        ) && self.peek_typedef_name().is_some();
        if !is_typedef_portname_shape
            && matches!(self.peek(), Some(TokenKind::Word(WordKind::Ident)))
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Ident) | TokenKind::Dot)
            )
        {
            let iface = self.ident().unwrap();
            let modport = if self.eat(TokenKind::Dot) {
                self.ident()
            } else {
                None
            };
            let name = self.ident().unwrap_or(Ident {
                name: String::new(),
                span: self.cur_span(),
            });
            let ispan = iface.span;
            return AnsiPort {
                dir: PortDir::Input, // placeholder — iface ports carry no dir
                net_or_var: None,
                signed: false,
                range: None,
                packed: Vec::new(),
                name,
                default: None,
                iface: Some(IfaceRef {
                    iface,
                    modport,
                    span: ispan,
                }),
                span: start.to(self.prev_span()),
            };
        }
        let explicit_dir = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::Input))) => {
                self.bump();
                Some(PortDir::Input)
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Output))) => {
                self.bump();
                Some(PortDir::Output)
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Inout))) => {
                self.bump();
                Some(PortDir::Inout)
            }
            _ => None,
        };
        let dir = match (explicit_dir, prev.map(|p| p.dir)) {
            (Some(d), _) => d,
            (None, Some(prev_dir)) => prev_dir, // inherit
            (None, None) => {
                self.error("port direction (first ANSI port must specify one)");
                PortDir::Input
            } // recover as Input
        };
        let mut net_or_var = self.net_var_kind();
        if net_or_var.is_some() {
            self.bump();
        }
        // `input mode_e m` — a typedef name as the port type (typedef-recognition
        // family). When no built-in kind keyword matched, a simple vector/enum
        // typedef resolves to its (kind, signed, range); the typedef name carries
        // the range, so the normal signed/range/packed reads are SKIPPED for it
        // (they would otherwise consume the port NAME). Built-in path unchanged.
        let typedef_ty = if net_or_var.is_none() {
            self.try_port_typedef()
        } else {
            None
        };
        let (mut signed, mut range, mut packed) = if let Some((k, s, r)) = typedef_ty {
            net_or_var = Some(k);
            (s, r, Vec::new())
        } else {
            (
                self.signed_eff(net_or_var),
                self.opt_range(),
                self.opt_packed_dims(), // additional packed dims `[3:0][7:0]`
            )
        };
        // A pure continuation (no own direction/type/range/signed) inherits the
        // previous port's type — `input [7:0] a, b` ⇒ b is also `[7:0]`.
        if explicit_dir.is_none()
            && net_or_var.is_none()
            && range.is_none()
            && packed.is_empty()
            && !signed
        {
            if let Some(p) = prev {
                net_or_var = p.net_or_var;
                signed = p.signed;
                range = p.range.clone();
                packed = p.packed.clone();
            }
        }
        let name = self.ident().unwrap_or(Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        let default = if self.eat(TokenKind::Eq) {
            Some(self.expr(0))
        } else {
            None
        };
        AnsiPort {
            dir,
            net_or_var,
            signed,
            range,
            packed,
            name,
            default,
            iface: None,
            span: start.to(self.prev_span()),
        }
    }

    /// Parse one parameter/localparam decl (the keyword is optional on `#(…)`
    /// continuations, defaulting to `Parameter`, which matches IEEE-1364 §12.2).
    /// `parse_param_decl` result: almost always a scalar `ParamDecl`; a body
    /// ARRAY parameter (A2a) desugars to the equivalent const variable-array
    /// decl instead (see `parse_array_param`).
    fn parse_param_decl(&mut self, body: bool) -> Option<ParamItem> {
        let start = self.cur_span();
        let kind = if self.eat_kw(Kw::Localparam) {
            ParamKind::Localparam
        } else {
            self.eat_kw(Kw::Parameter);
            ParamKind::Parameter
        };
        // Track explicit-`signed` PRESENCE alongside the folded bool: the A2a
        // array desugar must mirror `signed_eff` (explicit wins, else the atom
        // default) — the folded bool alone conflates "absent" with `false`.
        let expl0 = self.opt_signed();
        let mut signed = expl0.unwrap_or(false);
        // SV typed parameter: a data-type KIND keyword may lead — `parameter int W`,
        // `parameter logic [3:0] X`, `byte`/`shortint`/`longint`. 2-state atoms imply
        // a fixed signed range; `int` maps to the 32-bit signed Integer path. The
        // V2005 `integer`/`real`/`realtime`/`time` types stay in the else branch.
        let mut ty = ParamType::Implicit;
        let mut forced_range = None;
        // The exact declared kind, kept for the A2a array-parameter desugar (the
        // scalar path collapses `int` into `ParamType::Integer` etc. — the desugar
        // must construct the SAME `NetVarDecl` the equivalent var decl would).
        let mut var_kind: Option<NetVarKind> = None;
        let mut expl1: Option<bool> = None;
        let kw_kind = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(
                k @ (Kw::Logic
                | Kw::Reg
                | Kw::Bit
                | Kw::Int
                | Kw::Byte
                | Kw::Shortint
                | Kw::Longint),
            ))) => Some(k),
            _ => None,
        };
        if let Some(k) = kw_kind {
            self.bump(); // the kind keyword
            match k {
                Kw::Int => {
                    ty = ParamType::Integer; // 32-bit signed 2-state
                    var_kind = Some(NetVarKind::Int);
                }
                Kw::Byte => {
                    forced_range = Some(Self::dec_range(7));
                    signed = true;
                    var_kind = Some(NetVarKind::Byte);
                }
                Kw::Shortint => {
                    forced_range = Some(Self::dec_range(15));
                    signed = true;
                    var_kind = Some(NetVarKind::Shortint);
                }
                Kw::Longint => {
                    forced_range = Some(Self::dec_range(63));
                    signed = true;
                    var_kind = Some(NetVarKind::Longint);
                }
                // logic/reg/bit: width from an explicit range below
                Kw::Logic => var_kind = Some(NetVarKind::Logic),
                Kw::Reg => var_kind = Some(NetVarKind::Reg),
                Kw::Bit => var_kind = Some(NetVarKind::Bit),
                _ => {}
            }
            expl1 = self.opt_signed();
            signed = signed || expl1.unwrap_or(false);
        } else {
            ty = match self.peek() {
                Some(TokenKind::Word(WordKind::Keyword(Kw::Integer))) => {
                    self.bump();
                    var_kind = Some(NetVarKind::Integer);
                    ParamType::Integer
                }
                Some(TokenKind::Word(WordKind::Keyword(Kw::Real))) => {
                    self.bump();
                    var_kind = Some(NetVarKind::Real);
                    ParamType::Real
                }
                Some(TokenKind::Word(WordKind::Keyword(Kw::Realtime))) => {
                    self.bump();
                    var_kind = Some(NetVarKind::Realtime);
                    ParamType::Realtime
                }
                Some(TokenKind::Word(WordKind::Keyword(Kw::Time))) => {
                    self.bump();
                    var_kind = Some(NetVarKind::Time);
                    ParamType::Time
                }
                _ => ParamType::Implicit,
            };
        }
        let explicit_range = if forced_range.is_some() {
            None
        } else {
            self.opt_range()
        };
        let range = forced_range.or_else(|| explicit_range.clone());
        let name = self.ident()?;
        // A2a: `[` after the parameter name ⇒ an ARRAY parameter (IEEE §6.20.2).
        if self.peek() == Some(TokenKind::LBracket) {
            // Explicit-signing presence (either position): the desugar mirrors
            // `signed_eff` — an explicit `signed`/`unsigned` wins over the
            // atom default (`int unsigned` must come out unsigned, like the
            // equivalent var decl — the folded bool can't express that).
            let expl_signed = match (expl0, expl1) {
                (None, None) => None,
                (a, b) => Some(a.unwrap_or(false) || b.unwrap_or(false)),
            };
            return self.parse_array_param(
                body,
                kind,
                var_kind,
                expl_signed,
                explicit_range,
                name,
                start,
            );
        }
        self.expect(TokenKind::Eq, "'=' in parameter");
        let value = self.expr(0);
        Some(ParamItem::Scalar(ParamDecl {
            kind,
            signed,
            ty,
            range,
            name,
            value,
            span: start.to(self.prev_span()),
        }))
    }

    /// A2a (IEEE §6.20.2): a body `localparam <type> NAME [dims] = '{…};` —
    /// an ARRAY parameter. DESUGARED here into the EXACT `NetVarDecl` the
    /// equivalent variable-array decl (`int NAME [dims] = '{…};`) parses to,
    /// so storage, `'{…}` init, indexing, foreach and `$size`/`$bits` reuse
    /// the verified unpacked-array path verbatim, plus `const_param: true` so
    /// elaborate registers the net as an elaboration constant (any later
    /// write is a loud error). Kept loud (v1): the ANSI `#(…)` header form
    /// and an overridable body `parameter` (no override machinery for
    /// aggregates), an implicit-typed element, and non-fixed dims (`[$]`/`[]`).
    #[allow(clippy::too_many_arguments)]
    fn parse_array_param(
        &mut self,
        body: bool,
        kind: ParamKind,
        var_kind: Option<NetVarKind>,
        expl_signed: Option<bool>,
        explicit_range: Option<Range>,
        name: Ident,
        start: Span,
    ) -> Option<ParamItem> {
        if !body {
            self.error(
                "a scalar parameter in the ANSI `#(…)` header (an array parameter is supported only as a body `localparam` in v1)",
            );
            return None;
        }
        if kind == ParamKind::Parameter {
            self.error(
                "`localparam` for an array parameter (an overridable array `parameter` is unsupported in v1)",
            );
            return None;
        }
        let Some(vk) = var_kind else {
            self.error(
                "an explicit data type on an array parameter (`localparam int …` — an implicit-typed array parameter is unsupported in v1)",
            );
            return None;
        };
        let n_start = name.span;
        let mut unpacked = Vec::new();
        while self.at_dim_start() {
            match self.parse_dim() {
                Some(d) => unpacked.push(d),
                None => break,
            }
        }
        if !unpacked
            .iter()
            .all(|d| matches!(d, Dim::Range(_) | Dim::Size(_)))
        {
            self.error("a fixed array-parameter dimension (`[msb:lsb]`/`[size]`)");
            return None;
        }
        self.expect(TokenKind::Eq, "'=' in parameter");
        let value = self.expr(0);
        // Mirror `signed_eff`: an explicit `signed`/`unsigned` wins; otherwise
        // the atom default (byte/shortint/int/longint/integer are signed),
        // exactly like the equivalent var decl.
        let signed = expl_signed.unwrap_or_else(|| atom_default_signed(Some(vk)));
        Some(ParamItem::ConstArrayVar(NetVarDecl {
            kind: vk,
            signed,
            range: explicit_range,
            packed: Vec::new(),
            delay: None,
            names: vec![DeclName {
                name,
                unpacked,
                init: Some(value),
                span: n_start.to(self.prev_span()),
            }],
            lifetime: None,
            class_type: None,
            class_args: Vec::new(),
            const_param: true,
            span: start.to(self.prev_span()),
        }))
    }

    /// `defparam hier.path = expr [, hier.path = expr]* ;` (IEEE §23.10.1) — a
    /// hierarchical parameter override. Each LHS is a hierarchical path whose last
    /// segment names the parameter and whose prefix names the target instance.
    fn parse_defparam(&mut self) -> Option<DefparamItem> {
        let start = self.cur_span();
        self.bump(); // 'defparam'
        let mut assigns = Vec::new();
        loop {
            let Some(path) = self.hier_path() else {
                self.error("a hierarchical parameter path after `defparam`");
                break;
            };
            self.expect(TokenKind::Eq, "'=' in defparam");
            let value = self.expr(0);
            assigns.push((path, value));
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semi, "';' after defparam");
        Some(DefparamItem {
            assigns,
            span: start.to(self.prev_span()),
        })
    }

    fn parse_module_item(&mut self) -> Option<ModuleItem> {
        // skip a stray lexer error token without re-reporting (already diagnosed)
        if self.at_lex_error() {
            let s = self.cur_span();
            self.bump();
            return Some(ModuleItem::Error(s));
        }
        // parameter / localparam
        if self.at_kw(Kw::Parameter) || self.at_kw(Kw::Localparam) {
            let p = self.parse_param_decl(true)?;
            self.expect(TokenKind::Semi, "';'");
            return Some(match p {
                ParamItem::Scalar(p) => {
                    // Record a module-scope `localparam` whose value is a pure literal
                    // constant, so a constant generate hier index `g[P].x` can fold it.
                    // A `parameter` is overridable → never recorded (its index stays loud).
                    if p.kind == ParamKind::Localparam {
                        if let Some(v) = self.try_const_index(&p.value) {
                            self.const_locals.insert(p.name.name.clone(), v);
                        }
                    }
                    ModuleItem::Param(p)
                }
                // A2a: an array parameter arrives as the desugared const
                // variable-array decl — flows through every NetVar pass verbatim.
                ParamItem::ConstArrayVar(d) => ModuleItem::NetVar(d),
            });
        }
        // defparam path = expr [, path = expr]* ;  (IEEE §23.10.1)
        if self.at_kw(Kw::Defparam) {
            return self.parse_defparam().map(ModuleItem::Defparam);
        }
        // continuous assign
        if self.at_kw(Kw::Assign) {
            return self.parse_cont_assign().map(ModuleItem::ContAssign);
        }
        // GATE: gate-level primitive instantiation (and/or/nand/nor/xor/xnor/
        // buf/not/bufif0/bufif1/notif0/notif1) — desugared to continuous assigns.
        if let Some(g) = self.gate_kind() {
            return self.parse_gate_primitive(g).map(ModuleItem::ContAssign);
        }
        // non-ANSI body port direction decl
        if matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Input | Kw::Output | Kw::Inout
            )))
        ) {
            return self.parse_port_decl().map(ModuleItem::PortDecl);
        }
        // SV `typedef enum/…/<type> name;` (Phase-2 user-defined types).
        if self.at_kw(Kw::Typedef) {
            return self.parse_typedef();
        }
        // ⓑ-breadth (§25.9): `virtual INTERFACE vif [, vif2];` handle declaration.
        // Distinguished from a `virtual function/task` method by the keyword that
        // follows: an interface/type NAME (an ident) vs `function`/`task`.
        if self.at_kw(Kw::Virtual)
            && matches!(self.peek_at(1), Some(TokenKind::Word(WordKind::Ident)))
        {
            return self.parse_virtual_iface_decl().map(ModuleItem::NetVar);
        }
        // N7: `class NAME …; … endclass` declared inside a module/package body.
        if self.at_kw(Kw::Class) {
            return self.parse_class_decl().map(ModuleItem::Class);
        }
        // v5 ⑥: `modport mp (input a, output b);` — interface body item.
        if self.at_kw(Kw::Modport) {
            return self.parse_modport().map(ModuleItem::Modport);
        }
        // v7 P2-D: module/package-scope `import pkg::…;`.
        if self.at_kw(Kw::Import) {
            return self.parse_import_decl().map(ModuleItem::Import);
        }
        // net/var declaration
        if self.net_var_kind().is_some() {
            // Module-item scope (also reached for generate items via
            // parse_gen_item → parse_module_item): a net-decl delay IS allowed.
            return self.parse_net_var(true).map(ModuleItem::NetVar);
        }
        // typedef-name declaration: `color_t c, d;` where `color_t` was typedef'd.
        if let Some(info) = self.peek_typedef_name() {
            return self.parse_typed_decl(info).map(ModuleItem::NetVar);
        }
        // procedural blocks → REAL parsing (PR2).
        if matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Initial
                    | Kw::Always
                    | Kw::AlwaysFf
                    | Kw::AlwaysComb
                    | Kw::AlwaysLatch
                    | Kw::Final
            )))
        ) {
            return Some(ModuleItem::Proc(self.parse_procedural_block()));
        }
        // function/endfunction and task/endtask definitions.
        if self.at_kw(Kw::Function) {
            let (fd, is_void) = self.parse_function_def();
            if is_void {
                // `function void` in module/package scope ⇒ task-equivalent: reuse the
                // full task machinery (statement call, output formals, control flow).
                return Some(ModuleItem::Task(TaskDef {
                    automatic: fd.automatic,
                    name: fd.name,
                    ports: fd.ports,
                    body_decls: fd.body_decls,
                    body: fd.body,
                    span: fd.span,
                }));
            }
            return Some(ModuleItem::Func(fd));
        }
        if self.at_kw(Kw::Task) {
            return Some(ModuleItem::Task(self.parse_task_def()));
        }
        // genvar declaration:  genvar i, j;
        if self.at_kw(Kw::Genvar) {
            return Some(self.parse_genvar_decl());
        }
        // generate construct:  generate … endgenerate  (PR3 — real parsing).
        if self.at_kw(Kw::Generate) {
            return Some(ModuleItem::Generate(self.parse_generate_construct()));
        }
        // module-level concurrent assertion: `assert property(@(clk) …);`
        // (slice S10). Only `assert property` is a module item — an immediate
        // `assert (expr)` is procedural-only and is a loud error here. The
        // concurrent form is wrapped in a synthetic `initial` so it flows
        // through the same procedural ConcurrentAssert collection
        // (`pending_sva`); the checker is materialized at module level
        // regardless, so this is a pure parser-placement change (no AST shape
        // change, no sim-ir change).
        if self.at_kw(Kw::Assert) || self.at_kw(Kw::Assume) {
            let start = self.cur_span();
            self.bump(); // `assert` / `assume`
            if !self.at_kw(Kw::Property) {
                self.error(
                    "`property` after `assert`/`assume` at module level (immediate \
                     assertions are procedural-only)",
                );
                return Some(ModuleItem::Error(start.to(self.prev_span())));
            }
            // SVA-REST: `assume property` is checked exactly like `assert property`
            // in simulation (IEEE §16.12 — the assumption is verified); the same
            // synthesized checker is materialized.
            let stmt = self.parse_concurrent_assert(start);
            let span = start.to(self.prev_span());
            return Some(ModuleItem::Proc(ProceduralBlock {
                kind: ProcKind::Initial,
                sensitivity: None,
                body: Box::new(stmt),
                span,
            }));
        }
        // SVA-REST: module-level `cover property(@(clk) seq);` — wrapped in a synthetic
        // `initial` (like module-level `assert property`) so it flows through the same
        // procedural collection; the counter/report is materialized at module level.
        if self.at_ident_kw("cover")
            && self.peek_at(1) == Some(TokenKind::Word(WordKind::Keyword(Kw::Property)))
        {
            let start = self.cur_span();
            let stmt = self.parse_cover_property();
            let span = start.to(self.prev_span());
            return Some(ModuleItem::Proc(ProceduralBlock {
                kind: ProcKind::Initial,
                sensitivity: None,
                body: Box::new(stmt),
                span,
            }));
        }
        // SVA-REST: `let NAME [(formals)] = expr;` (IEEE 1800 §11.13) — a named
        // expression macro. `let` is contextual (an SV reserved word, never a legal
        // net name), recognized only when followed by an identifier.
        if self.at_ident_kw("let")
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
            )
        {
            return self.parse_let_decl();
        }
        // Named SVA declarations (Phase-3 named-SVA slice). `sequence` /
        // `endsequence` / `endproperty` are CONTEXTUAL keywords (`at_ident_kw`,
        // like `throughout`/`within`/`iff`); `property` is `Kw::Property`. Placed
        // before the bare-ident instantiation arm so `sequence s; …` is not
        // mis-parsed as a module instantiation. A net named `sequence`
        // (`wire sequence;`) is unaffected — `net_var_kind` matches first.
        //
        // `sequence` is NOT a Verilog-2005 reserved word, so a V2005 module TYPE
        // literally named `sequence` and its instantiation (`sequence u(.o(o))`) must
        // STILL parse. A no-formals decl `sequence NAME ;` routes here on the cheap
        // 2-token guard. A PARAMETERIZED decl `sequence NAME ( … ) ;` (slice A1)
        // collides with a positional/named module instantiation of the same shape;
        // disambiguate by a content-independent forward scan for the terminating
        // `endsequence` (a decl always has one; an instantiation never does). The
        // scan is what lets `sequence u(.o(o));` (no `endsequence`) stay an
        // instantiation while `sequence s(x,y); … endsequence` is a decl.
        // `property` IS a hard keyword (`Kw::Property`) — it cannot name a module, so
        // there is no masking there.
        if self.at_ident_kw("sequence")
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
            )
            && (self.peek_at(2) == Some(TokenKind::Semi)
                || (self.peek_at(2) == Some(TokenKind::LParen) && self.is_sequence_decl_ahead()))
        {
            return self.parse_sequence_decl();
        }
        if self.at_kw(Kw::Property) {
            return self.parse_property_decl();
        }
        // N5: functional-coverage model `covergroup NAME; … endgroup`.
        if self.at_kw(Kw::Covergroup) {
            return self.parse_covergroup();
        }
        // N4: `clocking …` / `default clocking …` block (IEEE 1800 §14).
        if self.at_kw(Kw::Clocking)
            || (self.at_kw(Kw::Default)
                && matches!(
                    self.peek_at(1),
                    Some(TokenKind::Word(WordKind::Keyword(Kw::Clocking)))
                ))
        {
            return self.parse_clocking();
        }
        // N5: a covergroup INSTANCE `CG_TYPE NAME = new;` — distinguished from a module
        // instantiation (`CG_TYPE NAME ( … )`) by the `=` at lookahead 2. Placed before
        // the bare-ident instantiation arm.
        if self.is_ident()
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
            )
            && self.peek_at(2) == Some(TokenKind::Eq)
        {
            return self.parse_cover_instance();
        }
        // bare ident at module-item position ⇒ module instantiation.
        // (No keyword-led item matched above; in V2005 module scope a leading
        //  bare identifier can ONLY begin an instantiation — there is no
        //  bare-ident contassign/decl. The dispatch position itself is the
        //  disambiguation, so no multi-token lookahead is needed to decide.
        //  Gate PRIMITIVES (`and`/`or`/`buf`/…) are keyword-led, never reach
        //  this arm, and are not parsed in v1 — they fall through to the loud
        //  "expected module item" E2002 below.)
        if self.is_ident() {
            let module_name = self.ident().unwrap();
            return Some(ModuleItem::Instance(
                self.parse_module_instance(module_name),
            ));
        }
        self.error("module item");
        None
    }

    // ─────────────────────── module instantiation ───────────────────────
    /// Parse a module instantiation, given the already-consumed `module_name`.
    /// Grammar:  module_name [ #(param_overrides) ] inst_body {, inst_body} ;
    /// where     inst_body = inst_name [unpacked_dims] ( port_connections )
    ///
    /// Disambiguation: the caller reaches a bare ident at module-item position
    /// only after every keyword-led item is ruled out; in V2005 module scope a
    /// leading bare identifier can ONLY start an instantiation, so no lookahead
    /// is needed to decide. Gate primitives (and/or/not …) are NOT special-cased
    /// here — they lex as plain idents and so flow through this path; a true
    /// gate-primitive instance has no module body for elaborate to find and is a
    /// DEFERRED limitation (it still recovers as an ordinary instance shape).
    /// Always returns a `ModuleInstance` (recovery is internal: sync via the
    /// terminal `expect(Semi)` + per-list forward-progress guards).
    fn parse_module_instance(&mut self, module_name: Ident) -> ModuleInstance {
        let start = module_name.span;

        // optional parameter override list  #( … )
        let param_overrides = if self.peek() == Some(TokenKind::Hash) {
            self.bump(); // '#'
            self.parse_param_overrides()
        } else {
            Vec::new()
        };

        // one-or-more instance bodies, comma-separated
        let mut instances = Vec::new();
        loop {
            let before = self.pos;
            instances.push(self.parse_instance_item());
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        self.expect(TokenKind::Semi, "';' after instantiation");
        ModuleInstance {
            module_name,
            param_overrides,
            instances,
            span: start.to(self.prev_span()),
        }
    }

    // ───────────────────────── GATE: gate-level primitives ─────────────────
    /// Classify the current token as a gate-primitive keyword, if any.
    fn gate_kind(&self) -> Option<GateKind> {
        use GateKind::*;
        match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(k))) => Some(match k {
                Kw::And => And,
                Kw::Or => Or,
                Kw::Nand => Nand,
                Kw::Nor => Nor,
                Kw::Xor => Xor,
                Kw::Xnor => Xnor,
                Kw::Buf => Buf,
                Kw::Not => Not,
                Kw::Bufif0 => Bufif0,
                Kw::Bufif1 => Bufif1,
                Kw::Notif0 => Notif0,
                Kw::Notif1 => Notif1,
                _ => return None,
            }),
            _ => None,
        }
    }

    /// `gate_type [#delay] inst {, inst} ;` where `inst = [name] ( terminals )`.
    /// Desugared to continuous assigns (IEEE 1364 §7): multi-input gates reduce
    /// inputs with the gate op (first terminal = output); buf/not pass/invert the
    /// LAST terminal to every preceding output; bufif/notif drive `out` with the
    /// (inverted) data when the control matches, else `1'bz`. Drive strength is
    /// not modelled (a strength spec parses as terminals ⇒ a natural loud error).
    fn parse_gate_primitive(&mut self, gate: GateKind) -> Option<ContinuousAssign> {
        let start = self.cur_span();
        self.bump(); // gate keyword
        let delay = if self.peek() == Some(TokenKind::Hash) {
            self.parse_delay()
        } else {
            None
        };
        let mut assigns: Vec<(Lvalue, Expr)> = Vec::new();
        loop {
            // optional instance name precedes the terminal list `(`.
            if self.is_ident() {
                let _name = self.ident();
            }
            self.expect(TokenKind::LParen, "'(' before gate terminals");
            let mut terms: Vec<Expr> = Vec::new();
            loop {
                let before = self.pos;
                terms.push(self.expr(0));
                if self.pos == before {
                    self.bump(); // forward-progress guard
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')' after gate terminals");
            self.gate_desugar(gate, &terms, &mut assigns);
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semi, "';' after gate instantiation");
        Some(ContinuousAssign {
            delay,
            assigns,
            span: start.to(self.prev_span()),
        })
    }

    /// Lower one gate instance's terminals into `(output_lvalue, rhs_expr)` pairs.
    fn gate_desugar(&mut self, gate: GateKind, terms: &[Expr], out: &mut Vec<(Lvalue, Expr)>) {
        use GateKind::*;
        let sp = terms
            .first()
            .map(|e| e.span)
            .unwrap_or_else(|| self.cur_span());
        let bin = |op: BinOp, l: Expr, r: Expr| Expr {
            span: l.span.to(r.span),
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(l),
                rhs: Box::new(r),
            },
        };
        let inv = |e: Expr| Expr {
            span: e.span,
            kind: ExprKind::Unary {
                op: UnOp::BitNot,
                operand: Box::new(e),
            },
        };
        // z→x coercion via double bitwise-not: `~~v` is identity for 0/1/x and maps
        // z→x (a gate input's z becomes x, IEEE 1364 §7.3/§7.4). Needed only on the
        // NON-inverting pass-through paths (buf, bufif data); the inverting paths
        // (not/notif via `~`) and the multi-input operator folds already coerce z→x.
        let zx = |e: Expr| inv(inv(e));
        let hi_z = || Expr {
            span: sp,
            kind: ExprKind::IntLit {
                kind: IntLitKind::Sized,
                raw: "1'bz".to_string(),
            },
        };
        match gate {
            And | Or | Nand | Nor | Xor | Xnor => {
                // first terminal = output; the rest fold with the gate operator.
                if terms.len() < 2 {
                    self.error("gate needs an output and at least one input");
                    return;
                }
                let op = match gate {
                    And | Nand => BinOp::BitAnd,
                    Or | Nor => BinOp::BitOr,
                    _ => BinOp::BitXor, // Xor | Xnor
                };
                // 2+ inputs fold through the operator (z→x naturally); a single
                // input has no operator, so coerce its z→x explicitly.
                let mut acc = if terms.len() == 2 {
                    zx(terms[1].clone())
                } else {
                    terms[1].clone()
                };
                for t in &terms[2..] {
                    acc = bin(op, acc, t.clone());
                }
                let rhs = if matches!(gate, Nand | Nor | Xnor) {
                    inv(acc)
                } else {
                    acc
                };
                out.push((self.expr_to_lvalue(terms[0].clone()), rhs));
            }
            Buf | Not => {
                // LAST terminal = input; every preceding terminal is an output.
                if terms.len() < 2 {
                    self.error("buf/not needs at least one output and one input");
                    return;
                }
                let input = terms.last().unwrap().clone();
                for o in &terms[..terms.len() - 1] {
                    let rhs = if matches!(gate, Not) {
                        inv(input.clone()) // ~ already maps z→x
                    } else {
                        zx(input.clone()) // buf pass-through: coerce z→x
                    };
                    out.push((self.expr_to_lvalue(o.clone()), rhs));
                }
            }
            Bufif0 | Bufif1 | Notif0 | Notif1 => {
                // ( out, data, control ): drive (inverted) data when control
                // matches the gate's active level, else high-Z.
                if terms.len() != 3 {
                    self.error("bufif/notif needs exactly (output, data, control)");
                    return;
                }
                let data = if matches!(gate, Notif0 | Notif1) {
                    inv(terms[1].clone()) // ~ already maps z→x
                } else {
                    zx(terms[1].clone()) // bufif data pass-through: coerce z→x
                };
                let ctrl = terms[2].clone();
                // bufif1/notif1 drive on control==1 (then=data, else=Z);
                // bufif0/notif0 drive on control==0 (then=Z, else=data).
                let (then_e, else_e) = if matches!(gate, Bufif1 | Notif1) {
                    (data, hi_z())
                } else {
                    (hi_z(), data)
                };
                let rhs = Expr {
                    span: sp,
                    kind: ExprKind::Ternary {
                        cond: Box::new(ctrl),
                        then_e: Box::new(then_e),
                        else_e: Box::new(else_e),
                    },
                };
                out.push((self.expr_to_lvalue(terms[0].clone()), rhs));
            }
        }
    }

    /// Combinational User-Defined Primitive (IEEE 1364 §29), DESUGARED in the parser
    /// (mirroring [`parse_gate_primitive`]) into a synthetic ordinary [`ModuleDecl`].
    /// The module body is one `always @(*)` whose if/else-if cascade realizes the
    /// truth table with iverilog-faithful semantics. (1) Each input column is matched
    /// 4-state-EXACT (`===`), NOT `casez` — a `0`/`1`/`x` column matches only that
    /// value; `?` matches anything (incl. z); `b` matches 0 or 1 only (never x/z).
    /// casez would wildcard the SCRUTINEE's x/z and silently mis-match (the cardinal
    /// trap). (2) Conflicting rows resolve order-INDEPENDENTLY by priority 0 > 1 > x:
    /// `if (any-0-row) o=0; else if (any-1-row) o=1; else o=x;`. (3) Any combination
    /// matched by no row (and every `x`-output row) → x (the trailing `else`).
    ///
    /// Honest-loud rejects (combinational only — sequential UDPs are slice #9):
    /// `reg` output / a second `:` (sequential current-state form) / edge & `z`
    /// symbols / `z`/`-` outputs / multi-output / wrong column count.
    fn parse_udp_decl(&mut self) -> Option<ModuleDecl> {
        // The UDP table symbol kinds live at module scope (so the row-scanner helper
        // methods can name them); alias to short local names for the body.
        use UdpEnd as EndSym;
        use UdpInCol as InCol;
        use UdpLevSym as LevSym;
        use UdpNextSym as NextSym;
        use UdpOutSym as OutSym;
        use UdpStateSym as StateSym;
        let start = self.cur_span();
        self.bump(); // `primitive`
        let name = self.ident()?;
        // ── header port list: ( out, in0, in1, … ) ;  (positional names only) ──
        self.expect(TokenKind::LParen, "'(' after primitive name");
        let mut port_names: Vec<Ident> = Vec::new();
        loop {
            let before = self.pos;
            if let Some(id) = self.ident() {
                port_names.push(id);
            }
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "')' after primitive ports");
        self.expect(TokenKind::Semi, "';' after primitive header");
        if port_names.len() < 2 {
            self.error_at(
                name.span,
                "a UDP with one output and at least one input port",
            );
            return None;
        }
        // ── direction declarations: `output [reg] OUT;`, `input a, b;`, plus the
        //    sequential-only `initial OUT = 1'bN;` — in any order before `table`. ──
        let mut out_name: Option<Ident> = None;
        let mut in_names: Vec<Ident> = Vec::new();
        // `output reg` ⇒ sequential UDP.
        let mut seq = false;
        // Whether the output was actually declared `reg` (via `output reg` or a
        // standalone `reg OUT;`). A sequential UDP table REQUIRES a reg output
        // (IEEE §29.7); a plain `wire` output with sequential rows is internally
        // inconsistent and must be loud-rejected (iverilog asserts/aborts).
        let mut out_is_reg = false;
        // sequential power-on value from `initial OUT = …;` (None ⇒ x).
        let mut initial_val: Option<char> = None;
        let mut saw_initial = false;
        while !self.at_kw(Kw::Table) && !self.at_eof() {
            if self.at_kw(Kw::Output) {
                self.bump();
                if self.at_kw(Kw::Reg) {
                    // `output reg` is the canonical sequential-UDP marker.
                    self.bump();
                    seq = true;
                    out_is_reg = true;
                }
                // Honest-loud: a VECTOR output (`output reg [N:0] o;`) is out of
                // scope — UDP outputs are scalar 1-bit per IEEE §29.
                if matches!(self.peek(), Some(TokenKind::LBracket)) {
                    self.error("a scalar (1-bit) UDP output (vector output is unsupported)");
                    return None;
                }
                loop {
                    let before = self.pos;
                    if let Some(id) = self.ident() {
                        if out_name.is_some() {
                            self.error_at(id.span, "exactly one UDP output port");
                            return None;
                        }
                        out_name = Some(id);
                    }
                    if self.pos == before {
                        self.bump();
                    }
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::Semi, "';' after UDP output declaration");
            } else if self.at_kw(Kw::Input) {
                self.bump();
                loop {
                    let before = self.pos;
                    if let Some(id) = self.ident() {
                        in_names.push(id);
                    }
                    if self.pos == before {
                        self.bump();
                    }
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::Semi, "';' after UDP input declaration");
            } else if self.at_kw(Kw::Reg) {
                // `reg OUT;` separate from the `output` decl (V2005 UDP form) ⇒
                // sequential. The named reg must be the output; we just note `seq`.
                self.bump();
                seq = true;
                out_is_reg = true;
                loop {
                    let before = self.pos;
                    let _ = self.ident();
                    if self.pos == before {
                        self.bump();
                    }
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::Semi, "';' after UDP reg declaration");
            } else if self.at_kw(Kw::Initial) {
                // `initial OUT = 1'bN;` power-on state (sequential-only ⇒ seq).
                self.bump();
                seq = true;
                saw_initial = true;
                let _ = self.ident(); // OUT (positional; value goes to the state reg)
                self.expect(TokenKind::Eq, "'=' in a UDP initial statement");
                // Collect the full literal text up to ';' (handles multi-token forms
                // like `1'b0`/`1 'b z`), then the bit VALUE is its LAST non-space char.
                // A 1-bit power-on must be exactly 0/1/x — `z` (or anything else) is an
                // illegal power-on value → loud-reject (NOT silently coerced to 1).
                let mut lit = String::new();
                while !matches!(self.peek(), Some(TokenKind::Semi)) && !self.at_eof() {
                    lit.push_str(self.cur_text());
                    self.bump();
                }
                self.expect(TokenKind::Semi, "';' after UDP initial statement");
                let v = lit
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .next_back()
                    .map(|c| c.to_ascii_lowercase());
                match v {
                    Some(c @ ('0' | '1' | 'x')) => initial_val = Some(c),
                    _ => {
                        self.error_at(
                            name.span,
                            "a UDP initial value of 0, 1, or x (1'b0 / 1'b1 / 1'bx)",
                        );
                        return None;
                    }
                }
            } else {
                self.error("'output', 'input', 'reg', 'initial', or 'table' in a UDP body");
                return None;
            }
        }
        let out_name = match out_name {
            Some(o) => o,
            None => {
                self.error_at(name.span, "a UDP 'output' declaration");
                return None;
            }
        };
        // output must be the FIRST port; the rest (in port-list order) are inputs.
        if out_name.name != port_names[0].name {
            self.error_at(out_name.span, "the UDP output to be the first port");
            return None;
        }
        let ordered_inputs: Vec<Ident> = port_names[1..].to_vec();
        {
            // every non-first port must be declared `input`, and vice versa.
            let decl_in: std::collections::BTreeSet<&str> =
                in_names.iter().map(|i| i.name.as_str()).collect();
            let port_in: std::collections::BTreeSet<&str> =
                ordered_inputs.iter().map(|i| i.name.as_str()).collect();
            if decl_in != port_in {
                self.error_at(
                    name.span,
                    "every UDP input port to be declared `input` (and vice versa)",
                );
                return None;
            }
        }
        let n_in = ordered_inputs.len();
        // ── table … endtable ──
        if !self.at_kw(Kw::Table) {
            self.error("'table' in a UDP body");
            return None;
        }
        self.bump(); // `table`
                     // A combinational row: (level cols, out). A sequential row carries the
                     // current-state column and a next-state symbol, and each input column may be
                     // a level or an edge.
        struct SeqRow {
            cols: Vec<InCol>,
            state: StateSym,
            next: NextSym,
            is_edge: bool, // exactly one column is an edge
        }
        let mut comb_rows: Vec<(Vec<LevSym>, OutSym)> = Vec::new();
        let mut seq_rows: Vec<SeqRow> = Vec::new();
        // A two-colon row (`inputs : state : next`) is itself a sequential signal even
        // when the state column is `?` and there is no edge/`-`.
        let mut has_two_colon = false;
        // `?` endpoint inside an edge is legal; the level-table `b` is NOT a legal
        // edge endpoint and must loud-reject.
        while !self.at_kw(Kw::Endtable) && !self.at_eof() {
            let row_start = self.cur_span();
            // Collect the three fields as raw text. Each table symbol is a single
            // char (`0 1 x ? b - r f p n *`) or a self-delimiting paren group
            // `(vw)`; joining each token's source text and char-scanning is therefore
            // unambiguous (whitespace is irrelevant). We split on the literal `:`
            // tokens so a parenthesised pair is never mistaken for a colon.
            let mut fields: Vec<String> = vec![String::new()];
            loop {
                if self.at_eof() || self.at_kw(Kw::Endtable) {
                    self.error("';' to end a UDP table row");
                    return None;
                }
                match self.peek() {
                    Some(TokenKind::Semi) => {
                        self.bump();
                        break;
                    }
                    Some(TokenKind::Colon) => {
                        self.bump();
                        if fields.len() >= 3 {
                            self.error_at(row_start, "at most two colons in a UDP table row");
                            return None;
                        }
                        fields.push(String::new());
                    }
                    _ => {
                        let last = fields.last_mut().unwrap();
                        last.push_str(self.cur_text());
                        self.bump();
                    }
                }
            }
            if fields.len() == 2 {
                // ── single-colon row: combinational OR sequential level/edge row
                //    (output is the next-state column; no separate state column). ──
                let cols = match Self::scan_udp_input_cols(&fields[0], n_in) {
                    Ok(c) => c,
                    Err(msg) => {
                        self.error_at(row_start, msg);
                        return None;
                    }
                };
                let has_edge = cols.iter().any(|c| matches!(c, InCol::Edge(_)));
                if has_edge {
                    // an edge column makes this a sequential row (single-colon form:
                    // state column omitted ⇒ `?`).
                    seq = true;
                    let n_edge = cols.iter().filter(|c| matches!(c, InCol::Edge(_))).count();
                    if n_edge > 1 {
                        // Honest-loud (correct-or-loud-stricter): IEEE §29 forbids >1
                        // edge per row; iverilog accepts-but-never-fires it.
                        self.error_at(row_start, "at most one edge column per UDP table row");
                        return None;
                    }
                    let next = match Self::scan_udp_next(&fields[1]) {
                        Ok(n) => n,
                        Err(msg) => {
                            self.error_at(row_start, msg);
                            return None;
                        }
                    };
                    seq_rows.push(SeqRow {
                        cols,
                        state: StateSym::Q,
                        next,
                        is_edge: true,
                    });
                } else {
                    // No edge → could be a combinational row OR a sequential LEVEL row
                    // (single-colon level form). Stash as a level SeqRow; also keep a
                    // comb projection for the combinational fallback (used only if the
                    // whole table turns out to be combinational). A `-` next is
                    // sequential-only ⇒ it forces `seq` and has NO comb projection.
                    let next = match Self::scan_udp_next(&fields[1]) {
                        Ok(n) => n,
                        Err(msg) => {
                            self.error_at(row_start, msg);
                            return None;
                        }
                    };
                    let levs: Vec<LevSym> = cols
                        .into_iter()
                        .map(|c| match c {
                            InCol::Lev(l) => l,
                            InCol::Edge(_) => unreachable!(),
                        })
                        .collect();
                    match next {
                        NextSym::Hold => {
                            // `-` ⇒ sequential; no comb projection.
                            seq = true;
                        }
                        NextSym::Zero => comb_rows.push((levs.clone(), OutSym::Zero)),
                        NextSym::One => comb_rows.push((levs.clone(), OutSym::One)),
                        NextSym::X => comb_rows.push((levs.clone(), OutSym::X)),
                    }
                    seq_rows.push(SeqRow {
                        cols: levs.into_iter().map(InCol::Lev).collect(),
                        state: StateSym::Q,
                        next,
                        is_edge: false,
                    });
                }
            } else {
                // ── two-colon row: SEQUENTIAL  inputs : state : next ──
                seq = true;
                has_two_colon = true;
                let cols = match Self::scan_udp_input_cols(&fields[0], n_in) {
                    Ok(c) => c,
                    Err(msg) => {
                        self.error_at(row_start, msg);
                        return None;
                    }
                };
                let n_edge = cols.iter().filter(|c| matches!(c, InCol::Edge(_))).count();
                if n_edge > 1 {
                    self.error_at(row_start, "at most one edge column per UDP table row");
                    return None;
                }
                let state = match fields[1]
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect::<Vec<_>>()
                    .as_slice()
                {
                    ['0'] => StateSym::Zero,
                    ['1'] => StateSym::One,
                    ['x'] | ['X'] => StateSym::X,
                    ['?'] => StateSym::Q,
                    _ => {
                        self.error_at(row_start, "a UDP state symbol (0 1 x ?)");
                        return None;
                    }
                };
                let next = match Self::scan_udp_next(&fields[2]) {
                    Ok(n) => n,
                    Err(msg) => {
                        self.error_at(row_start, msg);
                        return None;
                    }
                };
                seq_rows.push(SeqRow {
                    cols,
                    state,
                    next,
                    is_edge: n_edge == 1,
                });
            }
        }
        if !self.at_kw(Kw::Endtable) {
            self.error("'endtable'");
            return None;
        }
        self.bump(); // `endtable`
        if !self.at_kw(Kw::Endprimitive) {
            self.error("'endprimitive'");
            return None;
        }
        self.bump(); // `endprimitive`
        if seq_rows.is_empty() {
            // An empty `table … endtable` is an illegal UDP form (iverilog: "Empty
            // UDP table") — loud-reject rather than silently synthesize an always-x
            // primitive.
            self.error_at(name.span, "a non-empty UDP table");
            return None;
        }
        // ── internal-consistency: a sequential marker (`output reg` / two-colon /
        //    edge / `initial`) must be matched by a sequential table, and a purely
        //    combinational table must NOT carry sequential-only markers. ──
        if !seq {
            // Combinational UDP: every row must be single-colon, no edge, next ∈ 0/1/x.
            let span = start.to(self.prev_span());
            return self.build_comb_udp(span, name, port_names, &ordered_inputs, &comb_rows);
        }
        // Sequential consistency: every state column drawn from a single-colon row is
        // `?`; that is fine. But a `wire` (non-reg) output mixed with a two-colon row,
        // or a `reg`/`initial` marker with an all-combinational single-colon table
        // (no edges, no `-`, no state column), is internally inconsistent.
        let has_edge = seq_rows.iter().any(|r| r.is_edge);
        let has_hold = seq_rows.iter().any(|r| matches!(r.next, NextSym::Hold));
        let has_state = seq_rows.iter().any(|r| !matches!(r.state, StateSym::Q));
        if !out_is_reg {
            // A `wire` (non-reg) output with a SEQUENTIAL table (two-colon rows /
            // edge column / `-` next-state / state column) is internally
            // inconsistent: a sequential UDP requires a `reg` output (IEEE §29.7).
            // iverilog aborts on this; vita loud-rejects (correct-or-loud). NB
            // `initial OUT=…;` alone is NOT a `reg` declaration — it also requires
            // the output be declared `reg`, so this guard covers it too.
            self.error_at(
                name.span,
                "a `reg` output (`output reg …;`) for a sequential UDP table",
            );
            return None;
        }
        if !has_edge && !has_hold && !has_state && !has_two_colon {
            // A `reg`/`initial` marker but a table with no sequential content at all.
            self.error_at(
                name.span,
                "a sequential UDP table (with an edge column, a '-' next-state, or a state column) to match the 'reg'/'initial' marker",
            );
            return None;
        }
        let _ = saw_initial; // (only sets the t0 value; presence already set `seq`)

        // ── desugar: build the synthetic SEQUENTIAL module (literal §29 state-table
        //    evaluator: level rows first, then edge rows, no-match→x, '-'=hold). ──
        let span = start.to(self.prev_span());
        let mk_lit = |raw: &str| Expr {
            span,
            kind: ExprKind::IntLit {
                kind: IntLitKind::Sized,
                raw: raw.to_string(),
            },
        };
        let mk_ident = |id: &Ident| Expr {
            span,
            kind: ExprKind::Ident(HierPath {
                segments: vec![id.clone()],
                span,
            }),
        };
        let mk_bin = |op: BinOp, a: Expr, b: Expr| Expr {
            span,
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(a),
                rhs: Box::new(b),
            },
        };
        // case-equality of an expr against a 4-state literal raw (`1'b0/1/x/z`).
        let case_eq = |raw: &str, lhs: Expr| Expr {
            span,
            kind: ExprKind::Binary {
                op: BinOp::CaseEq,
                lhs: Box::new(lhs),
                rhs: Box::new(Expr {
                    span,
                    kind: ExprKind::IntLit {
                        kind: IntLitKind::Sized,
                        raw: raw.to_string(),
                    },
                }),
            },
        };
        // 4-state-exact level match of expr `e` against a LevSym/EndSym kind, with
        // z→x folding for the `x` symbol (matches BOTH x and z — IEEE §29.3.4).
        let match_zero = |e: Expr| case_eq("1'b0", e);
        let match_one = |e: Expr| case_eq("1'b1", e);
        let match_x = |e: Expr| {
            // x OR z
            mk_bin(BinOp::LogOr, case_eq("1'bx", e.clone()), case_eq("1'bz", e))
        };
        let match_b =
            |e: Expr| mk_bin(BinOp::LogOr, case_eq("1'b0", e.clone()), case_eq("1'b1", e));
        // state name + shadow regs.
        let out_id = &port_names[0];
        let shadow_name = |k: usize| Ident {
            name: format!("__p_udp_{}_{}", out_id.name, k),
            span,
        };
        // condition that input `k` LEVEL-matches symbol `s` (against CURRENT value).
        let lev_cond = |k: usize, s: LevSym| -> Option<Expr> {
            let e = mk_ident(&ordered_inputs[k]);
            match s {
                LevSym::Zero => Some(match_zero(e)),
                LevSym::One => Some(match_one(e)),
                LevSym::X => Some(match_x(e)),
                LevSym::Q => None, // `?` ⇒ no constraint
                LevSym::B => Some(match_b(e)),
            }
        };
        // condition for the state column matching stored `out`.
        let state_cond = |s: StateSym| -> Option<Expr> {
            let e = mk_ident(out_id);
            match s {
                StateSym::Zero => Some(match_zero(e)),
                StateSym::One => Some(match_one(e)),
                StateSym::X => Some(match_x(e)),
                StateSym::Q => None,
            }
        };
        // endpoint case-eq against current value `e` (z→x for `x`; `?` ⇒ None).
        let end_cond = |e: Expr, ep: EndSym| -> Option<Expr> {
            match ep {
                EndSym::Zero => Some(match_zero(e)),
                EndSym::One => Some(match_one(e)),
                EndSym::X => Some(match_x(e)),
                EndSym::Q => None,
            }
        };
        // FOLDED-change guard for input `k`: an edge exists iff prev/cur differ AFTER
        // z→x folding (IEEE §29.3.4). A pure z↔x swap (both fold to x) is NOT a change,
        // so it must neither fire an edge NOR re-evaluate the table (the output holds).
        // Encoded without a NOT operator:
        //   (prev !== cur) && ( is012(prev) || is012(cur) )
        // where is012(e) = (e===1'b0) || (e===1'b1).
        let folded_changed = |k: usize| -> Expr {
            let cur = mk_ident(&ordered_inputs[k]);
            let prev = mk_ident(&shadow_name(k));
            let is012 =
                |e: Expr| mk_bin(BinOp::LogOr, case_eq("1'b0", e.clone()), case_eq("1'b1", e));
            mk_bin(
                BinOp::LogAnd,
                mk_bin(BinOp::CaseNe, prev.clone(), cur.clone()),
                mk_bin(BinOp::LogOr, is012(prev), is012(cur)),
            )
        };
        // Build the full match condition for one row. `current` reads inputs;
        // edges also read the shadow regs.
        let and_fold = |conds: Vec<Expr>| -> Option<Expr> {
            conds.into_iter().reduce(|a, b| mk_bin(BinOp::LogAnd, a, b))
        };
        let or_fold = |conds: Vec<Expr>| -> Option<Expr> {
            conds.into_iter().reduce(|a, b| mk_bin(BinOp::LogOr, a, b))
        };
        let row_match = |row: &SeqRow| -> Expr {
            let mut conds: Vec<Expr> = Vec::new();
            for (k, col) in row.cols.iter().enumerate() {
                match col {
                    InCol::Lev(l) => {
                        if let Some(c) = lev_cond(k, *l) {
                            conds.push(c);
                        }
                    }
                    InCol::Edge(pairs) => {
                        // edge column k: folded_changed(k) AND OR_over_pairs[
                        //   CaseEq(__p_ik,from) && CaseEq(ik,to) ]  (?-endpoint drops).
                        let cur = mk_ident(&ordered_inputs[k]);
                        let prev = mk_ident(&shadow_name(k));
                        let changed = folded_changed(k);
                        let mut pair_terms: Vec<Expr> = Vec::new();
                        for (from, to) in pairs {
                            let mut t: Vec<Expr> = Vec::new();
                            if let Some(c) = end_cond(prev.clone(), *from) {
                                t.push(c);
                            }
                            if let Some(c) = end_cond(cur.clone(), *to) {
                                t.push(c);
                            }
                            // both endpoints `?` ⇒ any-change (just the guard).
                            match and_fold(t) {
                                Some(e) => pair_terms.push(e),
                                None => pair_terms.push(mk_lit("1'b1")),
                            }
                        }
                        let any_pair = or_fold(pair_terms).unwrap_or(mk_lit("1'b1"));
                        conds.push(mk_bin(BinOp::LogAnd, changed, any_pair));
                    }
                }
            }
            if let Some(c) = state_cond(row.state) {
                conds.push(c);
            }
            and_fold(conds).unwrap_or(mk_lit("1'b1"))
        };
        // Partition rows into LEVEL group and EDGE group, then bucket by next-value
        // (0/1/x/hold) — within a group, all rows producing the same next are
        // OR-folded; cross-group priority is fixed (level beats edge).
        let out_lval = Lvalue::Ident(HierPath {
            segments: vec![out_id.clone()],
            span,
        });
        let assign_lit = |raw: &str| Stmt::Blocking {
            lhs: out_lval.clone(),
            delay: None,
            event: None,
            rhs: mk_lit(raw),
            span,
        };
        let hold_stmt = || Stmt::Block {
            label: None,
            decls: Vec::new(),
            stmts: Vec::new(), // EMPTY then-branch that OWNS its else-slot (holds out)
            span,
        };
        // Build one priority cascade for a row group. Returns the innermost-else
        // chain WITHOUT the final no-match else (caller appends it).
        let build_group = |rows: &[&SeqRow], inner: Stmt| -> Stmt {
            let mut zero: Vec<Expr> = Vec::new();
            let mut one: Vec<Expr> = Vec::new();
            let mut xv: Vec<Expr> = Vec::new();
            let mut hold: Vec<Expr> = Vec::new();
            for r in rows {
                let m = row_match(r);
                match r.next {
                    NextSym::Zero => zero.push(m),
                    NextSym::One => one.push(m),
                    NextSym::X => xv.push(m),
                    NextSym::Hold => hold.push(m),
                }
            }
            let mut node = inner;
            // Build from lowest to highest priority so the final wrapping gives
            // 0 > 1 > x > hold.
            if let Some(c) = or_fold(hold) {
                node = Stmt::If {
                    cond: c,
                    then_s: Box::new(hold_stmt()),
                    else_s: Some(Box::new(node)),
                    span,
                };
            }
            if let Some(c) = or_fold(xv) {
                node = Stmt::If {
                    cond: c,
                    then_s: Box::new(assign_lit("1'bx")),
                    else_s: Some(Box::new(node)),
                    span,
                };
            }
            if let Some(c) = or_fold(one) {
                node = Stmt::If {
                    cond: c,
                    then_s: Box::new(assign_lit("1'b1")),
                    else_s: Some(Box::new(node)),
                    span,
                };
            }
            if let Some(c) = or_fold(zero) {
                node = Stmt::If {
                    cond: c,
                    then_s: Box::new(assign_lit("1'b0")),
                    else_s: Some(Box::new(node)),
                    span,
                };
            }
            node
        };
        let level_rows: Vec<&SeqRow> = seq_rows.iter().filter(|r| !r.is_edge).collect();
        let edge_rows: Vec<&SeqRow> = seq_rows.iter().filter(|r| r.is_edge).collect();
        // No-match → x (NEVER hold). Then edge group reached only when no level row
        // matched, then level group on top.
        let nomatch = assign_lit("1'bx");
        let edge_cascade = build_group(&edge_rows, nomatch);
        let full_cascade = build_group(&level_rows, edge_cascade);
        // GLOBAL folded-change guard: the table is RE-EVALUATED only when at least one
        // input's FOLDED (z→x) value changed. A wake caused solely by a z↔x swap on
        // some input (no folded change) holds the output — matching iverilog, which
        // does not re-evaluate a UDP on a z↔x-only transition (a no-match else would
        // otherwise clobber a previously-matched output to x → silent-wrong). With ≥1
        // input, fold each per-input guard with OR; the always already only wakes on
        // an input change, so `any_folded_change` is the only gate needed.
        let any_folded_change = (0..n_in)
            .map(folded_changed)
            .reduce(|a, b| mk_bin(BinOp::LogOr, a, b))
            .unwrap_or(mk_lit("1'b1"));
        let guarded_cascade = Stmt::If {
            cond: any_folded_change,
            then_s: Box::new(full_cascade),
            else_s: None, // no folded change ⇒ hold the output (empty else)
            span,
        };
        // Re-snapshot shadows LAST (blocking), after all reads — UNCONDITIONAL so the
        // shadow always tracks the latest raw input (folded matching is z/x-agnostic).
        let mut body_stmts: Vec<Stmt> = vec![guarded_cascade];
        for (k, _inp) in ordered_inputs.iter().enumerate() {
            body_stmts.push(Stmt::Blocking {
                lhs: Lvalue::Ident(HierPath {
                    segments: vec![shadow_name(k)],
                    span,
                }),
                delay: None,
                event: None,
                rhs: mk_ident(&ordered_inputs[k]),
                span,
            });
        }
        // Sensitivity: ALL inputs as plain NoEdge terms (→ AnyEdge wake = finest).
        // Shadow regs and `out` are NOT in the list.
        let sens: Vec<EventExpr> = ordered_inputs
            .iter()
            .map(|inp| EventExpr {
                edge: Edge::NoEdge,
                expr: mk_ident(inp),
                span,
            })
            .collect();
        let always = ProceduralBlock {
            kind: ProcKind::Always,
            sensitivity: Some(Sensitivity::List(sens)),
            body: Box::new(Stmt::Block {
                label: None,
                decls: Vec::new(),
                stmts: body_stmts,
                span,
            }),
            span,
        };
        // ── module body: the output port is a `reg` (single declaration, like the
        //    comb path); shadow regs are body regs; t0 power-on state is seeded by a
        //    synthetic `initial out = <init>;`. Shadow regs are NOT initialised — they
        //    power on as x (4-state reg default), so the first real input transition
        //    classifies as `x → value` through the table (exact t0-settling replay). ──
        let init_raw = match initial_val {
            Some('0') => "1'b0",
            Some('1') => "1'b1",
            _ => "1'bx",
        };
        let init_block = ProceduralBlock {
            kind: ProcKind::Initial,
            sensitivity: None,
            body: Box::new(Stmt::Blocking {
                lhs: out_lval.clone(),
                delay: None,
                event: None,
                rhs: mk_lit(init_raw),
                span,
            }),
            span,
        };
        // shadow regs (no init — power on x; seeded on the first input change).
        let mut body: Vec<ModuleItem> = Vec::new();
        for (k, _inp) in ordered_inputs.iter().enumerate() {
            body.push(ModuleItem::NetVar(NetVarDecl {
                kind: NetVarKind::Reg,
                signed: false,
                range: None,
                packed: Vec::new(),
                delay: None,
                names: vec![DeclName {
                    name: shadow_name(k),
                    unpacked: Vec::new(),
                    init: None,
                    span,
                }],
                lifetime: None,
                class_type: None,
                class_args: Vec::new(),
                const_param: false,
                span,
            }));
        }
        body.push(ModuleItem::Proc(init_block));
        body.push(ModuleItem::Proc(always));
        // ── ports: output (reg, procedurally driven) first, then input wires ──
        let mut ports: Vec<AnsiPort> = Vec::with_capacity(port_names.len());
        ports.push(AnsiPort {
            dir: PortDir::Output,
            net_or_var: Some(NetVarKind::Reg),
            signed: false,
            range: None,
            packed: Vec::new(),
            name: out_id.clone(),
            default: None,
            iface: None,
            span,
        });
        for inp in &ordered_inputs {
            ports.push(AnsiPort {
                dir: PortDir::Input,
                net_or_var: None, // default wire
                signed: false,
                range: None,
                packed: Vec::new(),
                name: inp.clone(),
                default: None,
                iface: None,
                span,
            });
        }
        Some(ModuleDecl {
            is_macromodule: false,
            name,
            params: Vec::new(),
            ports: PortList::Ansi(ports),
            body,
            span,
        })
    }

    /// Combinational-UDP desugar (the original YELLOW #1 path), factored out so the
    /// sequential path can early-return to it after the shared header parse.
    fn build_comb_udp(
        &mut self,
        span: Span,
        name: Ident,
        port_names: Vec<Ident>,
        ordered_inputs: &[Ident],
        rows: &[(Vec<UdpLevSym>, UdpOutSym)],
    ) -> Option<ModuleDecl> {
        let mk_eq = |raw: &str, lhs: Expr| Expr {
            span,
            kind: ExprKind::Binary {
                op: BinOp::CaseEq,
                lhs: Box::new(lhs),
                rhs: Box::new(Expr {
                    span,
                    kind: ExprKind::IntLit {
                        kind: IntLitKind::Sized,
                        raw: raw.to_string(),
                    },
                }),
            },
        };
        let row_cond = |in_syms: &[UdpLevSym]| -> Expr {
            let mut conds: Vec<Expr> = Vec::new();
            for (i, s) in in_syms.iter().enumerate() {
                let in_e = Expr {
                    span,
                    kind: ExprKind::Ident(HierPath {
                        segments: vec![ordered_inputs[i].clone()],
                        span,
                    }),
                };
                match s {
                    UdpLevSym::Zero => conds.push(mk_eq("1'b0", in_e)),
                    UdpLevSym::One => conds.push(mk_eq("1'b1", in_e)),
                    UdpLevSym::X => conds.push(Expr {
                        span,
                        kind: ExprKind::Binary {
                            op: BinOp::LogOr,
                            lhs: Box::new(mk_eq("1'bx", in_e.clone())),
                            rhs: Box::new(mk_eq("1'bz", in_e)),
                        },
                    }),
                    UdpLevSym::Q => {}
                    UdpLevSym::B => conds.push(Expr {
                        span,
                        kind: ExprKind::Binary {
                            op: BinOp::LogOr,
                            lhs: Box::new(mk_eq("1'b0", in_e.clone())),
                            rhs: Box::new(mk_eq("1'b1", in_e)),
                        },
                    }),
                }
            }
            conds
                .into_iter()
                .reduce(|a, b| Expr {
                    span,
                    kind: ExprKind::Binary {
                        op: BinOp::LogAnd,
                        lhs: Box::new(a),
                        rhs: Box::new(b),
                    },
                })
                .unwrap_or(Expr {
                    span,
                    kind: ExprKind::IntLit {
                        kind: IntLitKind::Sized,
                        raw: "1'b1".to_string(),
                    },
                })
        };
        let or_fold = |conds: Vec<Expr>| -> Option<Expr> {
            conds.into_iter().reduce(|a, b| Expr {
                span,
                kind: ExprKind::Binary {
                    op: BinOp::LogOr,
                    lhs: Box::new(a),
                    rhs: Box::new(b),
                },
            })
        };
        let mut zero_conds: Vec<Expr> = Vec::new();
        let mut one_conds: Vec<Expr> = Vec::new();
        for (in_syms, out) in rows {
            match out {
                UdpOutSym::Zero => zero_conds.push(row_cond(in_syms)),
                UdpOutSym::One => one_conds.push(row_cond(in_syms)),
                UdpOutSym::X => {}
            }
        }
        let out_lval = Lvalue::Ident(HierPath {
            segments: vec![port_names[0].clone()],
            span,
        });
        let assign = |raw: &str| Stmt::Blocking {
            lhs: out_lval.clone(),
            delay: None,
            event: None,
            rhs: Expr {
                span,
                kind: ExprKind::IntLit {
                    kind: IntLitKind::Sized,
                    raw: raw.to_string(),
                },
            },
            span,
        };
        let mut inner = assign("1'bx");
        if let Some(any1) = or_fold(one_conds) {
            inner = Stmt::If {
                cond: any1,
                then_s: Box::new(assign("1'b1")),
                else_s: Some(Box::new(inner)),
                span,
            };
        }
        if let Some(any0) = or_fold(zero_conds) {
            inner = Stmt::If {
                cond: any0,
                then_s: Box::new(assign("1'b0")),
                else_s: Some(Box::new(inner)),
                span,
            };
        }
        let always = ProceduralBlock {
            kind: ProcKind::Always,
            sensitivity: Some(Sensitivity::Star),
            body: Box::new(Stmt::Block {
                label: None,
                decls: Vec::new(),
                stmts: vec![inner],
                span,
            }),
            span,
        };
        let mut ports: Vec<AnsiPort> = Vec::with_capacity(port_names.len());
        ports.push(AnsiPort {
            dir: PortDir::Output,
            net_or_var: Some(NetVarKind::Reg),
            signed: false,
            range: None,
            packed: Vec::new(),
            name: port_names[0].clone(),
            default: None,
            iface: None,
            span,
        });
        for inp in ordered_inputs {
            ports.push(AnsiPort {
                dir: PortDir::Input,
                net_or_var: None,
                signed: false,
                range: None,
                packed: Vec::new(),
                name: inp.clone(),
                default: None,
                iface: None,
                span,
            });
        }
        Some(ModuleDecl {
            is_macromodule: false,
            name,
            params: Vec::new(),
            ports: PortList::Ansi(ports),
            body: vec![ModuleItem::Proc(always)],
            span,
        })
    }

    /// Scan a UDP input FIELD (raw concatenated token text) into `n_in` columns.
    /// Each column is one level symbol (`0 1 x ? b`) or one edge spec (`r f p n *`
    /// or a parenthesised `(vw)` pair). Whitespace is ignored.
    fn scan_udp_input_cols(field: &str, n_in: usize) -> Result<Vec<UdpInCol>, &'static str> {
        let chars: Vec<char> = field.chars().filter(|c| !c.is_whitespace()).collect();
        let mut cols: Vec<UdpInCol> = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            match c {
                '0' => {
                    cols.push(UdpInCol::Lev(UdpLevSym::Zero));
                    i += 1;
                }
                '1' => {
                    cols.push(UdpInCol::Lev(UdpLevSym::One));
                    i += 1;
                }
                'x' | 'X' => {
                    cols.push(UdpInCol::Lev(UdpLevSym::X));
                    i += 1;
                }
                '?' => {
                    cols.push(UdpInCol::Lev(UdpLevSym::Q));
                    i += 1;
                }
                'b' | 'B' => {
                    cols.push(UdpInCol::Lev(UdpLevSym::B));
                    i += 1;
                }
                'r' | 'R' => {
                    cols.push(UdpInCol::Edge(vec![(UdpEnd::Zero, UdpEnd::One)]));
                    i += 1;
                }
                'f' | 'F' => {
                    cols.push(UdpInCol::Edge(vec![(UdpEnd::One, UdpEnd::Zero)]));
                    i += 1;
                }
                'p' | 'P' => {
                    cols.push(UdpInCol::Edge(vec![
                        (UdpEnd::Zero, UdpEnd::One),
                        (UdpEnd::Zero, UdpEnd::X),
                        (UdpEnd::X, UdpEnd::One),
                    ]));
                    i += 1;
                }
                'n' | 'N' => {
                    cols.push(UdpInCol::Edge(vec![
                        (UdpEnd::One, UdpEnd::Zero),
                        (UdpEnd::One, UdpEnd::X),
                        (UdpEnd::X, UdpEnd::Zero),
                    ]));
                    i += 1;
                }
                '*' => {
                    // `*` = (??) = any change.
                    cols.push(UdpInCol::Edge(vec![(UdpEnd::Q, UdpEnd::Q)]));
                    i += 1;
                }
                '(' => {
                    // parse exactly TWO endpoint chars then ')'.
                    let from = match chars.get(i + 1) {
                        Some(&fc) => Self::udp_endpoint(fc)?,
                        None => return Err("a complete edge pair (vw) in a UDP row"),
                    };
                    let to = match chars.get(i + 2) {
                        Some(&tc) => Self::udp_endpoint(tc)?,
                        None => return Err("a complete edge pair (vw) in a UDP row"),
                    };
                    if chars.get(i + 3) != Some(&')') {
                        return Err("a two-symbol edge pair (vw) closed by ')'");
                    }
                    cols.push(UdpInCol::Edge(vec![(from, to)]));
                    i += 4;
                }
                _ => return Err("a UDP input symbol (0 1 x ? b r f p n * or (vw))"),
            }
        }
        if cols.len() != n_in {
            return Err("a UDP table row with one symbol per input port");
        }
        Ok(cols)
    }

    fn udp_endpoint(c: char) -> Result<UdpEnd, &'static str> {
        match c {
            '0' => Ok(UdpEnd::Zero),
            '1' => Ok(UdpEnd::One),
            'x' | 'X' => Ok(UdpEnd::X),
            '?' => Ok(UdpEnd::Q),
            // `b`, `z`, `*` etc. are NOT legal edge endpoints — loud.
            _ => Err("a UDP edge endpoint (0 1 x ?)"),
        }
    }

    /// Scan the NEXT-state field into a symbol (`0 1 x` or `-`=hold).
    fn scan_udp_next(field: &str) -> Result<UdpNextSym, &'static str> {
        let chars: Vec<char> = field.chars().filter(|c| !c.is_whitespace()).collect();
        match chars.as_slice() {
            ['0'] => Ok(UdpNextSym::Zero),
            ['1'] => Ok(UdpNextSym::One),
            ['x'] | ['X'] => Ok(UdpNextSym::X),
            ['-'] => Ok(UdpNextSym::Hold),
            _ => Err("a UDP next-state symbol (0 1 x -)"),
        }
    }

    /// Convert a gate OUTPUT terminal expression into an `Lvalue` (an output is a
    /// net reference / select / concat). Non-lvalue shapes recover as `Error`.
    fn expr_to_lvalue(&mut self, e: Expr) -> Lvalue {
        match e.kind {
            ExprKind::Paren { inner } => self.expr_to_lvalue(*inner),
            ExprKind::Ident(p) => Lvalue::Ident(p),
            ExprKind::BitSelect { base, index } => Lvalue::BitSelect {
                base: Box::new(self.expr_to_lvalue(*base)),
                index,
                span: e.span,
            },
            ExprKind::PartSelect { base, msb, lsb } => Lvalue::PartSelect {
                base: Box::new(self.expr_to_lvalue(*base)),
                msb,
                lsb,
                span: e.span,
            },
            ExprKind::IndexedPart {
                base,
                offset,
                width,
                dir,
            } => Lvalue::IndexedPart {
                base: Box::new(self.expr_to_lvalue(*base)),
                offset,
                width,
                dir,
                span: e.span,
            },
            ExprKind::Concat { parts } => Lvalue::Concat {
                parts: parts.into_iter().map(|p| self.expr_to_lvalue(p)).collect(),
                span: e.span,
            },
            _ => {
                self.error("gate output must be a net or net select");
                Lvalue::Error(e.span)
            }
        }
    }

    /// Inverse of `expr_to_lvalue`: rebuild the read-side `Expr` for an already-
    /// parsed lvalue. Used to desugar a compound assignment / increment
    /// (`lvalue += e` → `lvalue = lvalue + e`; `lvalue++` → `lvalue = lvalue + 1`):
    /// the lvalue appears on BOTH sides, so the rhs needs its expression form.
    /// The lvalue↔expr select shapes are 1:1, so this is a structural clone.
    /// A free associated fn (no `self`): it reads nothing from the parser state.
    fn lvalue_to_expr(lv: &Lvalue) -> Expr {
        match lv {
            Lvalue::Ident(p) => Expr {
                span: p.span,
                kind: ExprKind::Ident(p.clone()),
            },
            Lvalue::BitSelect { base, index, span } => Expr {
                span: *span,
                kind: ExprKind::BitSelect {
                    base: Box::new(Self::lvalue_to_expr(base)),
                    index: index.clone(),
                },
            },
            Lvalue::PartSelect {
                base,
                msb,
                lsb,
                span,
            } => Expr {
                span: *span,
                kind: ExprKind::PartSelect {
                    base: Box::new(Self::lvalue_to_expr(base)),
                    msb: msb.clone(),
                    lsb: lsb.clone(),
                },
            },
            Lvalue::IndexedPart {
                base,
                offset,
                width,
                dir,
                span,
            } => Expr {
                span: *span,
                kind: ExprKind::IndexedPart {
                    base: Box::new(Self::lvalue_to_expr(base)),
                    offset: offset.clone(),
                    width: width.clone(),
                    dir: *dir,
                },
            },
            Lvalue::Concat { parts, span } => Expr {
                span: *span,
                kind: ExprKind::Concat {
                    parts: parts.iter().map(Self::lvalue_to_expr).collect(),
                },
            },
            Lvalue::Error(span) => Expr {
                span: *span,
                kind: ExprKind::Error,
            },
        }
    }

    /// Map a compound-assignment / increment operator token to its binary op
    /// (SV §11.4.1/§11.4.2). `None` for any non-compound token. `++`/`--` reuse
    /// Add/Sub (with a synthesized `1`); `<<<=`/`>>>=` are the arithmetic shifts.
    fn compound_assign_binop(t: TokenKind) -> Option<BinOp> {
        use TokenKind as T;
        Some(match t {
            T::PlusEq | T::PlusPlus => BinOp::Add,
            T::MinusEq | T::MinusMinus => BinOp::Sub,
            T::StarEq => BinOp::Mul,
            T::SlashEq => BinOp::Div,
            T::PercentEq => BinOp::Mod,
            T::AmpEq => BinOp::BitAnd,
            T::PipeEq => BinOp::BitOr,
            T::CaretEq => BinOp::BitXor,
            T::ShlEq => BinOp::Shl,
            T::ShrEq => BinOp::Shr,
            T::ShlAEq => BinOp::AShl,
            T::ShrAEq => BinOp::AShr,
            _ => return None,
        })
    }

    /// If the cursor is on a compound-assign (`+=`…) or postfix `++`/`--`, consume
    /// it (plus the rhs for a compound op) and return the desugared blocking
    /// assignment `lvalue = lvalue <op> operand` (`++`/`--` use a literal `1`).
    /// Does NOT consume a trailing `;` (the caller owns that — statement vs
    /// for-clause). `None` ⇒ not a compound/inc-dec operator (caller handles
    /// `=`/`<=`/task-call). Side-effect-bearing EXPRESSION forms (`a = i++`) are
    /// NOT parsed here — they stay a loud error (correct-or-loud).
    ///
    /// The lvalue appears on both sides, so any index/select sub-expression is
    /// re-read on the rhs — but this is BYTE-IDENTICAL to the explicit
    /// `lvalue = lvalue <op> e` it desugars to (verified by differential), so the
    /// transform itself is exact. (A pure index reads the same value twice; a
    /// side-effecting index like `arr[f()]` follows the SAME path as the explicit
    /// form, where vita's pre-existing index-eval semantics already apply — that
    /// quirk is out of scope here, not introduced by this desugar.)
    fn try_compound_assign(&mut self, lhs: &Lvalue, start: Span) -> Option<Stmt> {
        let t = self.peek()?;
        let op = Self::compound_assign_binop(t)?;
        let is_incdec = matches!(t, TokenKind::PlusPlus | TokenKind::MinusMinus);
        self.bump(); // the operator
        let operand = if is_incdec {
            Expr {
                span: self.prev_span(),
                kind: ExprKind::IntLit {
                    kind: IntLitKind::Decimal,
                    raw: "1".to_string(),
                },
            }
        } else {
            self.expr(0)
        };
        let span = start.to(self.prev_span());
        let lhs_expr = Self::lvalue_to_expr(lhs);
        let rhs = Expr {
            span,
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(lhs_expr),
                rhs: Box::new(operand),
            },
        };
        Some(Stmt::Blocking {
            lhs: lhs.clone(),
            delay: None,
            event: None,
            rhs,
            span,
        })
    }

    /// `++lvalue` / `--lvalue` (prefix). As a STATEMENT the pre/post distinction is
    /// invisible (the value is discarded), so this desugars identically to
    /// `lvalue = lvalue ± 1`. Cursor is on `++`/`--`. Does NOT consume `;`.
    fn parse_pre_incdec(&mut self, start: Span) -> Stmt {
        let t = self.peek().expect("caller checked ++/--");
        let op = Self::compound_assign_binop(t).expect("caller checked ++/--");
        self.bump(); // ++ / --
        let lhs = self.parse_lvalue();
        let span = start.to(self.prev_span());
        let lhs_expr = Self::lvalue_to_expr(&lhs);
        let one = Expr {
            span,
            kind: ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw: "1".to_string(),
            },
        };
        let rhs = Expr {
            span,
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(lhs_expr),
                rhs: Box::new(one),
            },
        };
        Stmt::Blocking {
            lhs,
            delay: None,
            event: None,
            rhs,
            span,
        }
    }

    // ───────────────────────── N5: functional coverage ─────────────────────
    /// `covergroup NAME [(args)] [@(event)]; ([LABEL:] coverpoint EXPR [{..}|iff..];)*
    /// endgroup` — a functional-coverage model. The header tail (args / sampling event)
    /// and any per-coverpoint bins/iff are SKIPPED to `;` in this slice (auto-bins,
    /// explicit `sample()`); only the coverpoint EXPR is captured.
    /// N4: `[default] clocking [NAME] @(event); { [default] input/output [skew]
    /// sig [= expr] {, …}; } endclocking [: NAME]` (IEEE 1800 §14). v1 scope =
    /// default-skew INPUT/OUTPUT + `@(cb)`; an explicit skew (`#…`) is captured in
    /// `skew_raw` so elaborate can honest-loud it. A clocking-wide `default
    /// input/output …;` skew setter is loud here (out of v1 scope).
    fn parse_clocking(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        let is_default = self.eat_kw(Kw::Default);
        self.eat_kw(Kw::Clocking); // guaranteed by the dispatch in parse_module_item
                                   // Optional clocking-block name (`clocking cb @(...)` vs anonymous `clocking
                                   // @(...)`). Present iff the next token is an identifier (the `@` starts the
                                   // event otherwise).
        let name = if self.is_ident() { self.ident() } else { None };
        let clock = if self.peek() == Some(TokenKind::At) {
            self.parse_sensitivity()
        } else {
            self.error("'@(event)' clocking event");
            Sensitivity::Star
        };
        self.expect(TokenKind::Semi, "';' after clocking header");
        let mut items = Vec::new();
        loop {
            if self.at_kw(Kw::Endclocking) || self.peek().is_none() {
                break;
            }
            // Clocking-wide skew setter `default input/output [skew];` — out of v1
            // scope (skews unsupported). Loud, then skip to its `;`.
            if self.at_kw(Kw::Default) {
                self.error(
                    "a clocking-wide `default input/output` skew is unsupported in \
                     this subset (default skew only)",
                );
                while !matches!(self.peek(), Some(TokenKind::Semi) | None) {
                    self.bump();
                }
                self.eat(TokenKind::Semi);
                continue;
            }
            let dir = if self.eat_kw(Kw::Input) {
                ClockingDir::Input
            } else if self.eat_kw(Kw::Inout) {
                ClockingDir::Inout
            } else if self.eat_kw(Kw::Output) {
                ClockingDir::Output
            } else {
                self.error("'input'/'output' in a clocking block");
                while !matches!(self.peek(), Some(TokenKind::Semi) | None) {
                    self.bump();
                }
                self.eat(TokenKind::Semi);
                continue;
            };
            // Optional skew `#delay` / `#1step` — captured raw so elaborate can
            // accept `#1step` (the explicit default) or honest-loud others.
            // `#1step` is TWO tokens after `#`: IntDecimal("1") + Word(Ident("step")).
            let skew_raw = if self.peek() == Some(TokenKind::Hash) {
                self.bump(); // consume `#`
                             // Special-case `#1step`: IntDecimal "1" followed immediately by
                             // Word(Ident "step"). Maximal-munch does NOT merge them.
                let is_1step = matches!(self.peek(), Some(TokenKind::IntDecimal))
                    && self.cur_text() == "1"
                    && matches!(self.peek_at(1), Some(TokenKind::Word(WordKind::Ident)))
                    && self.text_at(1) == "step";
                if is_1step {
                    self.bump(); // consume `1`
                    self.bump(); // consume `step`
                    Some("#1step".to_string())
                } else {
                    let txt = self.cur_text().to_string();
                    self.bump();
                    Some(txt)
                }
            } else {
                None
            };
            // Signal list: `sig [= expr] {, sig [= expr]}` ;
            loop {
                let isp = self.cur_span();
                let Some(sig) = self.ident() else {
                    self.error("a clocking signal name");
                    break;
                };
                let expr = if self.eat(TokenKind::Eq) {
                    Some(self.expr(0))
                } else {
                    None
                };
                items.push(ClockingItem {
                    dir,
                    skew_raw: skew_raw.clone(),
                    name: sig,
                    expr,
                    span: isp,
                });
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Semi, "';' after a clocking item");
        }
        if !self.eat_kw(Kw::Endclocking) {
            self.error("'endclocking'");
        }
        // Optional `: NAME` label.
        if self.eat(TokenKind::Colon) {
            self.ident();
        }
        Some(ModuleItem::Clocking(ClockingDecl {
            name,
            is_default,
            clock,
            items,
            span: start,
        }))
    }

    fn parse_covergroup(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        self.bump(); // `covergroup`
        let name = self.ident()?;
        // optional `( ports )` — skip balanced (covergroup args, slice-future).
        if self.peek() == Some(TokenKind::LParen) {
            let mut depth = 0i32;
            loop {
                match self.peek() {
                    Some(TokenKind::LParen) => depth += 1,
                    Some(TokenKind::RParen) => {
                        depth -= 1;
                        if depth == 0 {
                            self.bump();
                            break;
                        }
                    }
                    None => break,
                    _ => {}
                }
                self.bump();
            }
        }
        // optional `@(event)` sampling clock (slice F): auto-sample on this event.
        let clock = if self.peek() == Some(TokenKind::At) {
            Some(self.parse_sensitivity())
        } else {
            None
        };
        // skip any remaining header tail (`with function sample(...)`, etc.) to `;`.
        while !matches!(self.peek(), Some(TokenKind::Semi) | None) {
            self.bump();
        }
        self.expect(TokenKind::Semi, "';' after covergroup header");
        let mut points = Vec::new();
        let mut crosses = Vec::new();
        let mut cg_at_least: Option<Expr> = None;
        loop {
            if self.at_kw(Kw::Endgroup) || self.peek().is_none() {
                break;
            }
            // optional `LABEL :`
            let label = if self.is_ident() && self.peek_at(1) == Some(TokenKind::Colon) {
                let l = self.ident().unwrap();
                self.bump(); // ':'
                Some(l)
            } else {
                None
            };
            if self.at_ident_kw("cross") {
                if let Some(cr) = self.parse_cross(label) {
                    crosses.push(cr);
                }
                continue;
            }
            // covergroup-level `option.NAME = expr;` (slice D): only `at_least` affects
            // the measured %; other options (goal/comment/per_instance/…) are accepted
            // and ignored (they do not change the coverage value in this model).
            if self.at_ident_kw("option") || self.at_ident_kw("type_option") {
                if let Some((name, val)) = self.parse_cover_option() {
                    if name == "at_least" {
                        cg_at_least = Some(val);
                    }
                }
                continue;
            }
            if self.at_kw(Kw::Coverpoint) {
                let cp_start = self.cur_span();
                self.bump(); // `coverpoint`
                let expr = self.expr(0);
                // optional coverpoint-level `iff (G)` guard (slice B).
                let iff = self.parse_cover_iff();
                // optional `{ bin* | option* }` body (else a bare `;`).
                let (bins, at_least, weight) = if self.peek() == Some(TokenKind::LBrace) {
                    let b = self.parse_coverpoint_body();
                    self.eat(TokenKind::Semi); // `;` after `}` is optional
                    b
                } else {
                    self.expect(TokenKind::Semi, "';' after coverpoint");
                    (Vec::new(), None, None)
                };
                points.push(Coverpoint {
                    label,
                    expr,
                    iff,
                    bins,
                    at_least,
                    weight,
                    span: cp_start.to(self.prev_span()),
                });
            } else {
                // an unsupported covergroup item (cross / option / …) — loud, skip to `;`.
                self.error("`coverpoint` in covergroup (cross/option are a follow-on)");
                while !matches!(self.peek(), Some(TokenKind::Semi) | None) {
                    self.bump();
                }
                self.eat(TokenKind::Semi);
            }
        }
        if self.at_kw(Kw::Endgroup) {
            self.bump();
        } else {
            self.error("`endgroup`");
        }
        // optional `: NAME` after endgroup
        if self.peek() == Some(TokenKind::Colon) {
            self.bump();
            let _ = self.ident();
        }
        Some(ModuleItem::Covergroup(CovergroupDecl {
            name,
            points,
            crosses,
            clock,
            at_least: cg_at_least,
            span: start.to(self.prev_span()),
        }))
    }

    /// `CG_TYPE NAME = new [(args)] ;` — a covergroup instance.
    fn parse_cover_instance(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        let cg_type = self.ident()?;
        let name = self.ident()?;
        self.expect(TokenKind::Eq, "'=' in covergroup instance");
        if self.at_ident_kw("new") {
            self.bump();
        } else {
            self.error("`new` in covergroup instance");
        }
        // optional `( args )` — skip balanced.
        if self.peek() == Some(TokenKind::LParen) {
            let mut depth = 0i32;
            loop {
                match self.peek() {
                    Some(TokenKind::LParen) => depth += 1,
                    Some(TokenKind::RParen) => {
                        depth -= 1;
                        if depth == 0 {
                            self.bump();
                            break;
                        }
                    }
                    None => break,
                    _ => {}
                }
                self.bump();
            }
        }
        self.expect(TokenKind::Semi, "';' after covergroup instance");
        Some(ModuleItem::CoverInstance(CoverInstance {
            cg_type,
            name,
            span: start.to(self.prev_span()),
        }))
    }

    /// `[LABEL:] cross cp_a, cp_b [, …] [{ … }] ;` — a cross of named coverpoints
    /// (slice C; the `cross` ident is at the cursor, LABEL already consumed). A cross
    /// SELECT body `{ binsof/intersect }` is loud-rejected and balanced-skipped.
    fn parse_cross(&mut self, label: Option<Ident>) -> Option<CrossSpec> {
        let start = self.cur_span();
        self.bump(); // `cross`
        let mut points = Vec::new();
        loop {
            let before = self.pos;
            if let Some(id) = self.ident() {
                points.push(id);
            }
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        // optional cross SELECT body `{ binsof … }` — follow-on; loud + balanced skip.
        if self.peek() == Some(TokenKind::LBrace) {
            self.error("cross select body (binsof/intersect) (follow-on)");
            let mut depth = 0i32;
            loop {
                match self.peek() {
                    Some(TokenKind::LBrace) => {
                        depth += 1;
                        self.bump();
                    }
                    Some(TokenKind::RBrace) => {
                        depth -= 1;
                        self.bump();
                        if depth == 0 {
                            break;
                        }
                    }
                    None => break,
                    _ => {
                        self.bump();
                    }
                }
            }
        }
        self.expect(TokenKind::Semi, "';' after cross");
        Some(CrossSpec {
            name: label,
            points,
            span: start.to(self.prev_span()),
        })
    }

    /// Optional `iff ( expr )` guard after a coverpoint expr or a bin RHS (slice B).
    /// `iff` is a contextual ident here (not a reserved keyword globally).
    fn parse_cover_iff(&mut self) -> Option<Expr> {
        if !self.at_ident_kw("iff") {
            return None;
        }
        self.bump(); // `iff`
        self.expect(TokenKind::LParen, "'(' after iff");
        let g = self.expr(0);
        self.expect(TokenKind::RParen, "')' after iff guard");
        Some(g)
    }

    /// Parse a coverpoint body `{ (bin | option)* }` (the opening `{` is at the
    /// cursor). Returns `(bins, at_least, weight)`. Each bin is `KIND NAME[array] =
    /// ( {range_list} | default ) [iff(G)] ;`. Unsupported bin forms
    /// (wildcard/transition/`binsof`/`intersect`/junk) are LOUD-rejected and
    /// balanced-skipped — never silently dropped. `option.at_least`/`option.weight`
    /// are captured; other `option.*` are accepted and ignored.
    #[allow(clippy::type_complexity)]
    fn parse_coverpoint_body(&mut self) -> (Vec<BinSpec>, Option<Expr>, Option<Expr>) {
        self.bump(); // `{`
        let mut bins = Vec::new();
        let mut at_least = None;
        let mut weight = None;
        loop {
            if matches!(self.peek(), Some(TokenKind::RBrace) | None) {
                break;
            }
            let before = self.pos;
            if self.at_ident_kw("option") || self.at_ident_kw("type_option") {
                if let Some((name, val)) = self.parse_cover_option() {
                    match name.as_str() {
                        "at_least" => at_least = Some(val),
                        "weight" => weight = Some(val),
                        _ => {} // accepted-ignored (does not change the measured %)
                    }
                }
            } else if let Some(b) = self.parse_bin_spec() {
                bins.push(b);
            }
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
        }
        self.eat(TokenKind::RBrace);
        (bins, at_least, weight)
    }

    /// `option.NAME = expr ;` / `type_option.NAME = expr ;` (the `option` ident is at
    /// the cursor). Returns `(NAME, value-expr)`. Slice D.
    fn parse_cover_option(&mut self) -> Option<(String, Expr)> {
        self.bump(); // `option` / `type_option`
        self.expect(TokenKind::Dot, "'.' after option");
        let name = self.ident()?;
        self.expect(TokenKind::Eq, "'=' in option");
        let val = self.expr(0);
        self.expect(TokenKind::Semi, "';' after option");
        Some((name.name, val))
    }

    /// One `KIND NAME[array] = RHS [iff(G)] ;` bin. Returns `None` (after a loud
    /// diagnostic + balanced skip to the bin's `;`) for unsupported forms.
    fn parse_bin_spec(&mut self) -> Option<BinSpec> {
        let start = self.cur_span();
        // `wildcard bins …` — follow-on; loud-reject.
        if self.at_ident_kw("wildcard") {
            self.error("wildcard coverage bins (follow-on)");
            self.skip_bin_to_semi();
            return None;
        }
        let kind = if self.at_ident_kw("bins") {
            BinKind::Regular
        } else if self.at_ident_kw("ignore_bins") {
            BinKind::Ignore
        } else if self.at_ident_kw("illegal_bins") {
            BinKind::Illegal
        } else {
            // `cross`/`option`/junk inside a coverpoint body — loud-reject.
            self.error("`bins`/`ignore_bins`/`illegal_bins` in coverpoint body");
            self.skip_bin_to_semi();
            return None;
        };
        self.bump(); // the bins-kind ident
        let name = self.ident()?;
        // optional array suffix: `[]` (unsized) or `[N]` (fixed).
        let array = if self.peek() == Some(TokenKind::LBracket) {
            self.bump(); // `[`
            if self.eat(TokenKind::RBracket) {
                BinArray::Unsized
            } else {
                let n = self.expr(0);
                self.expect(TokenKind::RBracket, "']' in bin array size");
                BinArray::Fixed(n)
            }
        } else {
            BinArray::Scalar
        };
        self.expect(TokenKind::Eq, "'=' in bin definition");
        // RHS: `default` | `{ open_range_list }` | `( trans_list )`(loud).
        let (values, is_default) = if self.at_kw(Kw::Default) {
            self.bump(); // `default`
            if self.at_ident_kw("sequence") {
                self.error("default sequence (transition) bins (follow-on)");
                self.skip_bin_to_semi();
                return None;
            }
            (Vec::new(), true)
        } else if self.peek() == Some(TokenKind::LParen) {
            self.error("transition coverage bins (follow-on)");
            self.skip_bin_to_semi();
            return None;
        } else if self.peek() == Some(TokenKind::LBrace) {
            (self.parse_open_range_list(), false)
        } else {
            self.error("bin value set `{...}` or `default`");
            self.skip_bin_to_semi();
            return None;
        };
        let iff = self.parse_cover_iff();
        self.expect(TokenKind::Semi, "';' after bin");
        Some(BinSpec {
            name,
            kind,
            array,
            values,
            is_default,
            iff,
            span: start.to(self.prev_span()),
        })
    }

    /// Parse `{ range (, range)* }` (the opening `{` is at the cursor).
    fn parse_open_range_list(&mut self) -> Vec<CoverRange> {
        self.bump(); // `{`
        let mut out = Vec::new();
        loop {
            if matches!(self.peek(), Some(TokenKind::RBrace) | None) {
                break;
            }
            let before = self.pos;
            if let Some(r) = self.parse_cover_range() {
                out.push(r);
            }
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.eat(TokenKind::RBrace);
        out
    }

    /// One open_range_list element: `[ end : end ]` (inclusive range) or a single
    /// value `expr` (`lo==hi`). A transition arrow `=>` after a value is loud-rejected.
    fn parse_cover_range(&mut self) -> Option<CoverRange> {
        if self.peek() == Some(TokenKind::LBracket) {
            self.bump(); // `[`
            let lo = self.parse_range_end();
            self.expect(TokenKind::Colon, "':' in range");
            let hi = self.parse_range_end();
            self.expect(TokenKind::RBracket, "']' in range");
            Some(CoverRange { lo, hi })
        } else {
            let v = self.expr(0);
            // transition `=>` (lexes as `=` then `>`) — follow-on.
            if self.peek() == Some(TokenKind::Eq) && self.peek_at(1) == Some(TokenKind::Gt) {
                self.error("transition coverage bins (follow-on)");
                return None;
            }
            let end = RangeEnd::Val(v);
            Some(CoverRange {
                lo: end.clone(),
                hi: end,
            })
        }
    }

    /// A range endpoint: `$` (type extreme) or a constant expression.
    fn parse_range_end(&mut self) -> RangeEnd {
        if self.peek() == Some(TokenKind::Dollar) {
            self.bump();
            RangeEnd::TypeExtreme
        } else {
            RangeEnd::Val(self.expr(0))
        }
    }

    /// Balanced skip to the terminating `;` of a malformed bin (recovery). Stops at
    /// a depth-0 `}` (the body terminator) without consuming it.
    fn skip_bin_to_semi(&mut self) {
        let mut depth = 0i32;
        loop {
            match self.peek() {
                None => break,
                Some(TokenKind::RBrace) if depth == 0 => break,
                Some(TokenKind::Semi) if depth == 0 => {
                    self.bump();
                    break;
                }
                Some(TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace) => {
                    depth += 1;
                    self.bump();
                }
                Some(TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace) => {
                    depth -= 1;
                    self.bump();
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    /// Parse `( param_overrides )` after a consumed `#`.
    /// `.NAME(expr)` ⇒ ParamConn::Named ; bare `expr` ⇒ ParamConn::Positional.
    /// The first token being `Dot` selects the named form for the whole list.
    /// An empty `#()` is legal (yields an empty Vec).
    fn parse_param_overrides(&mut self) -> Vec<ParamConn> {
        let mut out = Vec::new();
        if !self.expect(TokenKind::LParen, "'(' after '#'") {
            return out;
        }
        if self.peek() == Some(TokenKind::RParen) {
            self.bump(); // empty `#()`
            return out;
        }
        let named = self.peek() == Some(TokenKind::Dot);
        loop {
            let before = self.pos;
            if named {
                out.push(self.parse_named_param_conn());
            } else {
                // positional override: a single const-expr (never empty)
                out.push(ParamConn::Positional(self.expr(0)));
            }
            if self.pos == before {
                self.bump(); // progress guard
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "')' closing parameter overrides");
        out
    }

    /// `.NAME(expr)` | `.NAME()`  → ParamConn::Named { name, value, span }.
    fn parse_named_param_conn(&mut self) -> ParamConn {
        let start = self.cur_span();
        self.expect(TokenKind::Dot, "'.' in named parameter override");
        let name = self.ident().unwrap_or(Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        self.expect(TokenKind::LParen, "'(' after parameter name");
        let value = if self.peek() == Some(TokenKind::RParen) {
            None // `.W()` — explicitly-empty override
        } else {
            Some(self.expr(0))
        };
        self.expect(TokenKind::RParen, "')' after parameter value");
        ParamConn::Named {
            name,
            value,
            span: start.to(self.prev_span()),
        }
    }

    /// One instance: inst_name [unpacked_dims] ( port_connections )
    fn parse_instance_item(&mut self) -> InstanceItem {
        let start = self.cur_span();
        let name = self.ident().unwrap_or(Ident {
            name: String::new(),
            span: self.cur_span(),
        });

        // optional instance-array dims: `u_x [3:0] (...)` / `u_x [4] (...)`
        let mut unpacked = Vec::new();
        while self.at_dim_start() {
            match self.parse_dim() {
                Some(d) => unpacked.push(d),
                None => break,
            }
        }

        let conns = self.parse_port_conns();
        InstanceItem {
            name,
            unpacked,
            conns,
            span: start.to(self.prev_span()),
        }
    }

    /// `( … )` port-connection list.
    ///   first element `.NAME(...)`      ⇒ Named
    ///   first element `.*`               ⇒ implicit (DEFERRED: stub → empty Named)
    ///   first element bare expr / empty  ⇒ Positional (empty `()` ⇒ Positional([]))
    fn parse_port_conns(&mut self) -> PortConnList {
        if !self.expect(TokenKind::LParen, "'(' before port connections") {
            // recovered with no '(' — synthesize an empty positional list
            return PortConnList::Positional(Vec::new());
        }
        // empty `()` ⇒ zero-arity positional
        if self.peek() == Some(TokenKind::RParen) {
            self.bump();
            return PortConnList::Positional(Vec::new());
        }
        // named iff the first connection starts with a dot (covers `.p(e)`, the
        // `.p` shorthand, and the `.*` wildcard — all begin with `.`).
        let named = self.peek() == Some(TokenKind::Dot);
        if named {
            let mut conns = Vec::new();
            let mut wildcard = false;
            loop {
                // `.*` wildcard item (Dot then Star — there is no DotStar token).
                if self.peek() == Some(TokenKind::Dot)
                    && self.toks.get(self.pos + 1).map(|t| t.kind) == Some(TokenKind::Star)
                {
                    if wildcard {
                        self.error("a single `.*` per port connection list");
                    }
                    wildcard = true;
                    self.bump(); // '.'
                    self.bump(); // '*'
                } else {
                    let before = self.pos;
                    conns.push(self.parse_named_port_conn());
                    if self.pos == before {
                        self.bump();
                    }
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')' closing port connections");
            PortConnList::Named(conns, wildcard)
        } else {
            // positional: each element is `expr` OR empty (a skipped port → None).
            let mut conns: Vec<Option<Expr>> = Vec::new();
            loop {
                match self.peek() {
                    // an empty slot: `,` or `)` where an expr would start
                    Some(TokenKind::Comma) | Some(TokenKind::RParen) => conns.push(None),
                    _ => conns.push(Some(self.expr(0))),
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')' closing port connections");
            PortConnList::Positional(conns)
        }
    }

    /// `.PORT(expr)` | `.PORT()` | `.PORT`  → PortConn { name, value, span }.
    /// `.PORT()` (explicitly-unconnected) ⇒ value = None.
    /// `.PORT` (no parens, IEEE §23.3.2.3 implicit-named shorthand) ⇒
    /// `.PORT(PORT)` — the port binds to a same-named signal in this scope.
    fn parse_named_port_conn(&mut self) -> PortConn {
        let start = self.cur_span();
        self.expect(TokenKind::Dot, "'.' in named port connection");
        let name = self.ident().unwrap_or(Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        // `.name` shorthand: no `(` ⇒ desugar to `.name(name)`. The synthesized
        // identifier flows through the ordinary named-connection path, so a
        // missing same-named signal becomes a normal (loud) bind error — exactly
        // as iverilog reports it. A bare `.name(...)` is unchanged.
        if self.peek() != Some(TokenKind::LParen) {
            let value = (!name.name.is_empty()).then(|| Expr {
                span: name.span,
                kind: ExprKind::Ident(HierPath {
                    segments: vec![name.clone()],
                    span: name.span,
                }),
            });
            return PortConn {
                name,
                value,
                span: start.to(self.prev_span()),
            };
        }
        self.expect(TokenKind::LParen, "'(' after port name");
        let value = if self.peek() == Some(TokenKind::RParen) {
            None // `.clk()` — explicitly unconnected
        } else {
            Some(self.expr(0))
        };
        self.expect(TokenKind::RParen, "')' after port expression");
        PortConn {
            name,
            value,
            span: start.to(self.prev_span()),
        }
    }

    fn parse_port_decl(&mut self) -> Option<PortDecl> {
        let start = self.cur_span();
        let dir = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::Input))) => {
                self.bump();
                PortDir::Input
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Output))) => {
                self.bump();
                PortDir::Output
            }
            _ => {
                self.bump();
                PortDir::Inout
            }
        };
        let mut net_or_var = self.net_var_kind();
        if net_or_var.is_some() {
            self.bump();
        }
        // `input byte_t a;` — a typedef name as a non-ANSI port type (mirrors the
        // ANSI path; typedef-recognition family §4.5.36 ALL-variants).
        let typedef_ty = if net_or_var.is_none() {
            self.try_port_typedef()
        } else {
            None
        };
        let (signed, range) = if let Some((k, s, r)) = typedef_ty {
            net_or_var = Some(k);
            (s, r)
        } else {
            (self.signed_eff(net_or_var), self.opt_range())
        };
        let mut names = Vec::new();
        loop {
            if let Some(id) = self.ident() {
                names.push(id);
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semi, "';'");
        Some(PortDecl {
            dir,
            net_or_var,
            signed,
            range,
            names,
            span: start.to(self.prev_span()),
        })
    }

    /// `allow_net_delay`: only a MODULE-ITEM / GENERATE-scope net declaration may
    /// carry an IEEE §6.1.3 net-declaration delay (it desugars to a continuous
    /// assign, which only those scopes elaborate). In a procedural block / function
    /// / task / class body a `wire #3 w = a;` is illegal — the caller passes `false`
    /// so the `#` is left to error as before (correct-or-loud: never parse a delay
    /// we would then silently drop).
    fn parse_net_var(&mut self, allow_net_delay: bool) -> Option<NetVarDecl> {
        let start = self.cur_span();
        let kind = self.net_var_kind().unwrap();
        self.bump();
        let signed = self.signed_eff(Some(kind));
        let range = self.opt_range();
        let packed = self.opt_packed_dims(); // additional packed dims `logic [3:0][7:0]`
                                             // IEEE §6.1.3 net-declaration delay (`wire #3 w = a;` / `wire #(2,3) w = a;`):
                                             // after the range, before the name list. Only a NET kind in a delay-permitting
                                             // scope takes one — a `#` after a variable range, or any `#` in a procedural /
                                             // class scope, stays a loud error (never silently accept `reg #3 r`).
        let delay = if allow_net_delay && kind.is_net() && self.peek() == Some(TokenKind::Hash) {
            self.parse_delay()
        } else {
            None
        };
        let names = self.parse_decl_name_list()?;
        self.expect(TokenKind::Semi, "';'");
        Some(NetVarDecl {
            kind,
            signed,
            range,
            packed,
            delay,
            names,
            lifetime: None,
            class_type: None,
            class_args: Vec::new(),
            const_param: false,
            span: start.to(self.prev_span()),
        })
    }

    /// Comma-separated declarator list: `a, b[3:0], c = init`. Shared by
    /// `parse_net_var` and the typedef-name decl path. Does NOT consume the `;`.
    fn parse_decl_name_list(&mut self) -> Option<Vec<DeclName>> {
        let mut names = Vec::new();
        loop {
            let n_start = self.cur_span();
            let name = self.ident()?;
            let mut unpacked = Vec::new();
            while self.at_dim_start() {
                match self.parse_dim() {
                    Some(d) => unpacked.push(d),
                    None => break,
                }
            }
            let init = if self.eat(TokenKind::Eq) {
                Some(self.expr(0))
            } else {
                None
            };
            names.push(DeclName {
                name,
                unpacked,
                init,
                span: n_start.to(self.prev_span()),
            });
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        Some(names)
    }

    /// If the current token names a known typedef, return its resolved underlying
    /// type (peek only — the caller commits the decl). `None` ⇒ not a type name.
    fn peek_typedef_name(&self) -> Option<TypeInfo> {
        if self.is_ident() {
            return self.typedefs.get(self.cur_text()).cloned();
        }
        None
    }

    /// Inside a procedural block's decl region, a `<typedef_name> <ident>` opener is
    /// unambiguously a local declaration — no statement begins that way (`e = x` is
    /// invalid since `e` is a type, `e::A`/`e'(…)` have a non-ident second token).
    /// Returns the type only for that shape, so a typedef name used otherwise falls
    /// through to the statement region. (The module-item parser needs no such guard:
    /// no statements appear there.)
    fn peek_block_typedef_decl(&self) -> Option<TypeInfo> {
        let info = self.peek_typedef_name()?;
        if matches!(
            self.peek_at(1),
            Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
        ) {
            Some(info)
        } else {
            None
        }
    }

    /// `T name1, name2 = init, …;` where the leading type-name resolved to `info`.
    fn parse_typed_decl(&mut self, info: TypeInfo) -> Option<NetVarDecl> {
        let start = self.cur_span();
        let tyname = self.cur_text().to_string();
        self.bump(); // the type-name identifier
                     // ⓑ-breadth (§8.25): a parameterized class handle override `C #(16) h;`.
                     // Only a class-typed name takes specialization args; for any other type a
                     // `#` here is not ours (left for the caller / a loud error downstream).
        let class_args = if info.class_name.is_some() && self.peek() == Some(TokenKind::Hash) {
            self.parse_param_override_args()
        } else {
            Vec::new()
        };
        let mut names = self.parse_decl_name_list()?;
        self.expect(TokenKind::Semi, "';'");
        // If this is a struct type, bind each declared name → type so `var.field`
        // member accesses can be desugared to part-selects. A scalar (no unpacked
        // dims) struct var is additionally eligible for the `'{…}` pattern
        // desugar, applied here to its decl-init `= '{…}` (§10.9.1).
        if self.struct_layouts.contains_key(&tyname) {
            let is_union = self.union_type_names.contains(&tyname);
            for n in &mut names {
                self.var_struct.insert(n.name.name.clone(), tyname.clone());
                // A scalar STRUCT (not union, no unpacked dims) is eligible for the
                // `'{…}` pattern desugar; a union overlay is not (kept loud).
                if n.unpacked.is_empty() && !is_union {
                    self.struct_scalar_vars.insert(n.name.name.clone());
                    if let Some(init) = n.init.take() {
                        let nm = n.name.name.clone();
                        n.init = Some(self.desugar_struct_assign_pattern(&nm, init));
                    }
                } else if n.unpacked.len() == 1 && !is_union {
                    // A 1-D array of struct (`st_t arr[N]`): `arr[i] = '{…}` is a
                    // packed-struct pattern on the element. Multi-dim arrays stay
                    // loud (the indexed lvalue would be a nested BitSelect).
                    self.struct_1d_array_vars.insert(n.name.name.clone());
                }
            }
        }
        // If this is a (literal-foldable) enum type, bind each name → enum type so
        // `var.first/last/next/prev/name/num` can desugar to its labels (§6.19.5).
        if self.enum_defs.contains_key(&tyname) {
            for n in &names {
                self.var_enum.insert(n.name.name.clone(), tyname.clone());
            }
        }
        // N7: a class-typed alias carries the class name through to elaborate.
        let class_type = info.class_name.as_ref().map(|c| Ident {
            name: c.clone(),
            span: start,
        });
        Some(NetVarDecl {
            kind: info.kind,
            signed: info.signed,
            range: info.range,
            packed: info.packed,
            delay: None,
            names,
            lifetime: None,
            class_type,
            class_args,
            const_param: false,
            span: start.to(self.prev_span()),
        })
    }

    /// Parse a parameterized class handle's `#( expr, expr, … )` specialization
    /// arguments (ⓑ-breadth, §8.25). Positional value args only (named `.W(16)`
    /// is a v1 loud-reject left to elaborate). The leading `#` is at the cursor.
    fn parse_param_override_args(&mut self) -> Vec<Expr> {
        self.bump(); // `#`
        let mut args = Vec::new();
        if self.expect(TokenKind::LParen, "'(' after `#` in a class specialization") {
            if self.peek() != Some(TokenKind::RParen) {
                loop {
                    args.push(self.expr(0));
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RParen, "')' to close class specialization args");
        }
        args
    }

    /// Snapshot the lexically-scoped registries before a block's first body-local
    /// typedef / struct-or-enum-typed var, so they can be restored to give that
    /// block its own scope.
    fn snapshot_scope(&self) -> ScopeSnapshot {
        ScopeSnapshot {
            typedefs: self.typedefs.clone(),
            struct_layouts: self.struct_layouts.clone(),
            enum_defs: self.enum_defs.clone(),
            union_type_names: self.union_type_names.clone(),
            var_struct: self.var_struct.clone(),
            var_enum: self.var_enum.clone(),
            struct_scalar_vars: self.struct_scalar_vars.clone(),
            struct_1d_array_vars: self.struct_1d_array_vars.clone(),
        }
    }

    /// Restore the registries to a prior snapshot, dropping any block-local
    /// typedefs / struct-var bindings added since (so they do not leak out of or
    /// clobber an outer scope).
    fn restore_scope(&mut self, s: ScopeSnapshot) {
        self.typedefs = s.typedefs;
        self.struct_layouts = s.struct_layouts;
        self.enum_defs = s.enum_defs;
        self.union_type_names = s.union_type_names;
        self.var_struct = s.var_struct;
        self.var_enum = s.var_enum;
        self.struct_scalar_vars = s.struct_scalar_vars;
        self.struct_1d_array_vars = s.struct_1d_array_vars;
    }

    /// A procedural body-local typedef DEFINITION (`typedef logic [3:0] t;` inside
    /// a begin/function/task body — IEEE §6.18). The cursor is at the `typedef`
    /// keyword. Registers the name in `self.typedefs` (and `struct_layouts` for a
    /// struct/union) so subsequent local decls resolve it; the typedef itself emits
    /// no runtime decl, so the returned AST node is discarded. An ENUM typedef
    /// additionally needs elaborate-side label-constant registration (the discarded
    /// AST node carries the labels) that the body parser cannot perform — so a
    /// body-local enum typedef is honest-loud (define it at module scope). The
    /// caller is responsible for snapshotting/restoring the registries for scope.
    fn parse_body_typedef_def(&mut self) {
        if matches!(
            self.peek_at(1),
            Some(TokenKind::Word(WordKind::Keyword(Kw::Enum)))
        ) {
            self.error(
                "an alias / struct / union typedef in a begin/function/task body (a body-local enum typedef is unsupported in v1 — define it at module scope)",
            );
            self.synchronize();
            return;
        }
        // Registration is the functional effect; there is no body slot for the
        // ModuleItem, and elaborate needs nothing from an alias/struct/union node.
        let _ = self.parse_typedef();
    }

    /// `typedef enum [base] { L0, L1 = expr, … } name;` (Phase-2). Registers
    /// `name` in `self.typedefs` (so a later `name var;` parses) and returns the
    /// AST node so elaborate can register the labels as integer constants.
    fn parse_typedef(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        self.bump(); // `typedef`
        if self.at_kw(Kw::Struct) {
            return self.parse_typedef_struct(start);
        }
        if self.at_kw(Kw::Union) {
            return self.parse_typedef_union(start);
        }
        if !self.at_kw(Kw::Enum) {
            // `typedef logic [7:0] byte_t;` — plain alias to a net/var type.
            if self.net_var_kind().is_some() {
                return self.parse_typedef_alias(start);
            }
            // `typedef base_t alias_t;` — a chained alias of an EXISTING typedef.
            if let Some(info) = self.peek_typedef_name() {
                return self.parse_typedef_chained_alias(start, info);
            }
            // unpacked struct / union forms are out of v1 scope.
            self.error("`enum`, `struct packed`, or a type after `typedef`");
            self.synchronize();
            return Some(ModuleItem::Error(start.to(self.prev_span())));
        }
        self.bump(); // `enum`
                     // Optional packed base: `enum logic [1:0] {…}` or `enum [1:0] {…}`.
        let base = if self.net_var_kind().is_some() {
            self.bump(); // base kind keyword (logic/reg/integer/…)
            let _ = self.opt_signed();
            self.opt_range()
        } else if let Some(info) = self.peek_typedef_name() {
            // `enum b_t {…}` — the base type is an existing typedef name. Support a
            // SIMPLE UNSIGNED vector typedef (`logic`/`bit`/`reg` `[N]`); the enum
            // then stores as that vector (its range). A SIGNED base (the built-in
            // `enum logic signed[N]` path also drops signedness — a separate
            // pre-existing limit), an atom (`int`/`byte` — signed, no explicit
            // range), or a struct / class / multi-dim-packed typedef cannot be
            // represented by the enum's `Option<Range>` base model — honest-loud.
            let nm = self.cur_text().to_string();
            if self.struct_layouts.contains_key(&nm)
                || info.class_name.is_some()
                || info.signed
                || !info.packed.is_empty()
                || info.range.is_none()
            {
                self.error(
                    "a simple unsigned vector typedef (logic/bit/reg [N]) as an enum base (a signed / atom / struct / class / multi-dim typedef base is unsupported in v1)",
                );
                self.synchronize();
                return Some(ModuleItem::Error(start.to(self.prev_span())));
            }
            self.bump(); // the typedef-name token
            info.range
        } else {
            self.opt_range()
        };
        self.expect(TokenKind::LBrace, "'{' for enum body");
        let mut labels = Vec::new();
        if self.peek() != Some(TokenKind::RBrace) {
            loop {
                let name = self.ident()?;
                let value = if self.eat(TokenKind::Eq) {
                    Some(self.expr(0))
                } else {
                    None
                };
                labels.push(EnumLabel { name, value });
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RBrace, "'}' to close enum body");
        let tname = self.ident()?;
        self.expect(TokenKind::Semi, "';'");
        // Enum storage is `int` (32-bit signed) unless a packed base range was
        // given, in which case a `logic` vector of that range.
        let info = match &base {
            Some(r) => TypeInfo {
                kind: NetVarKind::Logic,
                signed: false,
                range: Some(r.clone()),
                packed: Vec::new(),
                class_name: None,
            },
            None => TypeInfo {
                kind: NetVarKind::Integer,
                signed: true,
                range: None,
                packed: Vec::new(),
                class_name: None,
            },
        };
        self.typedefs.insert(tname.name.clone(), info);
        // SV §6.19.5 enum-method support: fold each label's value (running counter,
        // reset by an explicit literal-foldable `= expr`). Record the ordered
        // (label, value) list ONLY if EVERY value folds (`const_lit`) — an enum with
        // a non-literal label value is omitted, so `x.method()` on it stays loud.
        {
            let mut folded: Vec<(String, i64)> = Vec::with_capacity(labels.len());
            let mut counter: i64 = 0;
            let mut foldable = true;
            for lab in &labels {
                let v = match &lab.value {
                    None => counter,
                    Some(e) => match Self::const_lit(e) {
                        Some(v) => v,
                        None => {
                            foldable = false;
                            break;
                        }
                    },
                };
                folded.push((lab.name.name.clone(), v));
                counter = v.wrapping_add(1);
            }
            if foldable && !folded.is_empty() {
                self.enum_defs.insert(tname.name.clone(), folded);
            }
        }
        Some(ModuleItem::Typedef(TypedefDecl {
            name: tname,
            kind: TypedefKind::Enum { base, labels },
            span: start.to(self.prev_span()),
        }))
    }

    /// `typedef <kind> [signed] [range] [packed] name;` — a plain type alias.
    /// `start` is the span of the leading `typedef` keyword (already consumed).
    fn parse_typedef_alias(&mut self, start: Span) -> Option<ModuleItem> {
        let kind = self.net_var_kind().unwrap();
        self.bump(); // kind keyword
        let signed = self.signed_eff(Some(kind));
        let range = self.opt_range();
        let packed = self.opt_packed_dims();
        let tname = self.ident()?;
        self.expect(TokenKind::Semi, "';'");
        self.typedefs.insert(
            tname.name.clone(),
            TypeInfo {
                kind,
                signed,
                range: range.clone(),
                packed: packed.clone(),
                class_name: None,
            },
        );
        Some(ModuleItem::Typedef(TypedefDecl {
            name: tname,
            kind: TypedefKind::Alias {
                kind,
                signed,
                range,
                packed,
            },
            span: start.to(self.prev_span()),
        }))
    }

    /// `typedef base_t alias_t;` — a chained alias of an EXISTING typedef
    /// (IEEE §6.18). The new name inherits the base's full registration — type
    /// info plus any struct/union layout or enum-method binding — so a later
    /// `alias_t v;` / `v.field` / `v.next` resolves exactly as `base_t v;` would.
    /// `start` is the span of the leading `typedef` (already consumed); the cursor
    /// is on the base typedef-name token, whose resolved `info` is passed in.
    /// Adding packed or unpacked dimensions to an aliased type
    /// (`typedef base_t [3:0] a_t;` / `typedef base_t a_t [4];`) needs type
    /// composition not in v1 — honest-loud.
    fn parse_typedef_chained_alias(&mut self, start: Span, info: TypeInfo) -> Option<ModuleItem> {
        let base_name = self.cur_text().to_string();
        self.bump(); // base typedef name
                     // honest-loud: PACKED dims before the new name (`typedef base_t [3:0] a_t;`).
        if self.peek() == Some(TokenKind::LBracket) {
            self.error(
                "a simple chained typedef alias (adding packed dimensions to an aliased type is unsupported in v1)",
            );
            self.synchronize();
            return Some(ModuleItem::Error(start.to(self.prev_span())));
        }
        let tname = self.ident()?;
        // honest-loud: UNPACKED dims after the new name (`typedef base_t a_t [4];`).
        if self.peek() == Some(TokenKind::LBracket) {
            self.error(
                "a simple chained typedef alias (an unpacked-array typedef of an aliased type is unsupported in v1)",
            );
            self.synchronize();
            return Some(ModuleItem::Error(start.to(self.prev_span())));
        }
        self.expect(TokenKind::Semi, "';'");
        let alias = tname.name.clone();
        // Mirror the base's registration under the new name. Each side-map is SET
        // when the base has that property and CLEARED otherwise, so a cross-module
        // same-name stale entry (the union-desync hazard) cannot leak through.
        self.typedefs.insert(alias.clone(), info.clone());
        match self.struct_layouts.get(&base_name).cloned() {
            Some(layout) => {
                self.struct_layouts.insert(alias.clone(), layout);
            }
            None => {
                self.struct_layouts.remove(&alias);
            }
        }
        match self.enum_defs.get(&base_name).cloned() {
            Some(defs) => {
                self.enum_defs.insert(alias.clone(), defs);
            }
            None => {
                self.enum_defs.remove(&alias);
            }
        }
        if self.union_type_names.contains(&base_name) {
            self.union_type_names.insert(alias.clone());
        } else {
            self.union_type_names.remove(&alias);
        }
        Some(ModuleItem::Typedef(TypedefDecl {
            name: tname,
            kind: TypedefKind::Alias {
                kind: info.kind,
                signed: info.signed,
                range: info.range,
                packed: info.packed,
            },
            span: start.to(self.prev_span()),
        }))
    }

    /// Parse a packed struct/union MEMBER's type into `(kind, signed, range)`. A
    /// built-in keyword resolves directly (§7.2.1); a SIMPLE user-defined type name
    /// (a vector / enum / atom typedef) resolves to its `TypeInfo`. A nested
    /// struct/union, a class handle, or a multi-dim packed typedef member needs
    /// nested-layout machinery not in v1 — honest-loud. Returns `None` (with the
    /// error already emitted) on a non-type token or an unsupported member type; the
    /// caller breaks out of the member loop.
    fn parse_struct_member_type(&mut self) -> Option<(NetVarKind, bool, Option<Range>)> {
        if let Some(kind) = self.net_var_kind() {
            self.bump(); // kind keyword
            let signed = self.signed_eff(Some(kind));
            let range = self.opt_range();
            return Some((kind, signed, range));
        }
        if let Some(info) = self.peek_typedef_name() {
            let nm = self.cur_text().to_string();
            if self.struct_layouts.contains_key(&nm)
                || info.class_name.is_some()
                || !info.packed.is_empty()
            {
                self.error(
                    "a simple (non-struct) type for a struct/union member (a nested struct / class / multi-dim packed member is unsupported in v1)",
                );
                return None;
            }
            self.bump(); // the typedef-name token
            return Some((info.kind, info.signed, info.range));
        }
        self.error("a net/var type in a struct/union member");
        None
    }

    /// `typedef struct packed { <type> f1, f2; … } name;` (Phase-2). Members are
    /// laid out MSB-first into one flat `logic [W-1:0]` vector; the layout is
    /// recorded so `name var;` resolves and `var.field` desugars to a part-select.
    /// `start` is the span of the leading `typedef` keyword (already consumed).
    fn parse_typedef_struct(&mut self, start: Span) -> Option<ModuleItem> {
        self.bump(); // `struct`
        if !self.eat_kw(Kw::Packed) {
            // unpacked struct has no flat layout in v1 — reject loudly.
            self.error("`packed` after `struct` (unpacked struct unsupported in v1)");
            self.synchronize();
            return Some(ModuleItem::Error(start.to(self.prev_span())));
        }
        let _ = self.opt_signed(); // `struct packed signed` — sign ignored for layout
        self.expect(TokenKind::LBrace, "'{' for struct body");
        let mut members = Vec::new();
        while self.peek() != Some(TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            let m_start = self.cur_span();
            let Some((kind, signed, range)) = self.parse_struct_member_type() else {
                break;
            };
            loop {
                let Some(name) = self.ident() else { break };
                members.push(StructMember {
                    name,
                    kind,
                    signed,
                    range: range.clone(),
                    span: m_start.to(self.prev_span()),
                });
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Semi, "';'");
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
        }
        self.expect(TokenKind::RBrace, "'}' to close struct body");
        let tname = self.ident()?;
        self.expect(TokenKind::Semi, "';'");
        // Compute each member width. A named integer-atom kind (`int`/`byte`/…)
        // carries a fixed width from its TYPE; a vector kind (`bit`/`logic`) sizes
        // from a constant-literal range (`None` ⇒ 1).
        let mut widths = Vec::with_capacity(members.len());
        for m in &members {
            match self.member_width_kind(m.kind, &m.range) {
                Some(w) if w > 0 => widths.push(w),
                _ => {
                    self.error_at(
                        m.span,
                        "struct member width must be a named integer type or a \
                         constant-literal range in v1",
                    );
                    widths.push(1);
                }
            }
        }
        let total: u32 = widths.iter().sum();
        // Lay out MSB-first: first member occupies the high bits.
        let mut off = total;
        let mut fields = Vec::with_capacity(members.len());
        for (m, w) in members.iter().zip(&widths) {
            off -= w;
            fields.push((
                m.name.name.clone(),
                off,
                *w,
                Self::member_ascending(&m.range),
                m.signed,
                Self::member_kind_two_state(m.kind),
            ));
        }
        self.struct_layouts
            .insert(tname.name.clone(), StructLayout { fields });
        // If a union with the same name was defined in an earlier module, retract it
        // from union_type_names so this struct definition wins (consistent with
        // struct_layouts last-writer-wins semantics; otherwise a later same-named
        // struct var would be wrongly excluded from '{…} pattern desugar).
        self.union_type_names.remove(&tname.name);
        // §7.2.1: an all-2-state struct is itself 2-state — back it with a 2-state
        // `bit` vector so it defaults to 0, not X (matches iverilog). Any 4-state
        // member (`logic`/`integer`/`time`/net) makes the whole struct 4-state.
        let struct_kind =
            if !members.is_empty() && members.iter().all(|m| Self::member_kind_two_state(m.kind)) {
                NetVarKind::Bit
            } else {
                NetVarKind::Logic
            };
        self.typedefs.insert(
            tname.name.clone(),
            TypeInfo {
                kind: struct_kind,
                signed: false,
                range: Some(Self::dec_range(total.saturating_sub(1))),
                packed: Vec::new(),
                class_name: None,
            },
        );
        Some(ModuleItem::Typedef(TypedefDecl {
            name: tname,
            kind: TypedefKind::Struct { members },
            span: start.to(self.prev_span()),
        }))
    }

    /// `typedef union packed { <type> f1; … } name;` (ⓑ-breadth, IEEE §7.3.1).
    /// A packed union OVERLAYS its members — every member shares bit 0, and the
    /// union width is the MAX member width (vs the struct's SUM). Recorded in the
    /// same `struct_layouts` map, so `u.field` desugars to a part-select exactly
    /// like a struct; a write to one member is visible through every other
    /// (different-width members read/write their own low bits). Pure parser
    /// addition (IR-0) — reuses `TypedefKind::Struct` for the AST node.
    fn parse_typedef_union(&mut self, start: Span) -> Option<ModuleItem> {
        self.bump(); // `union`
        if !self.eat_kw(Kw::Packed) {
            self.error("`packed` after `union` (unpacked union unsupported in v1)");
            self.synchronize();
            return Some(ModuleItem::Error(start.to(self.prev_span())));
        }
        let _ = self.opt_signed();
        self.expect(TokenKind::LBrace, "'{' for union body");
        let mut members = Vec::new();
        while self.peek() != Some(TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            let m_start = self.cur_span();
            let Some((kind, signed, range)) = self.parse_struct_member_type() else {
                break;
            };
            loop {
                let Some(name) = self.ident() else { break };
                members.push(StructMember {
                    name,
                    kind,
                    signed,
                    range: range.clone(),
                    span: m_start.to(self.prev_span()),
                });
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Semi, "';'");
            if self.pos == before {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "'}' to close union body");
        let tname = self.ident()?;
        self.expect(TokenKind::Semi, "';'");
        let mut widths = Vec::with_capacity(members.len());
        for m in &members {
            match self.member_width_kind(m.kind, &m.range) {
                Some(w) if w > 0 => widths.push(w),
                _ => {
                    self.error_at(
                        m.span,
                        "union member width must be a named integer type or a \
                         constant-literal range in v1",
                    );
                    widths.push(1);
                }
            }
        }
        // OVERLAY: union width = MAX member width; every member starts at bit 0.
        let total: u32 = widths.iter().copied().max().unwrap_or(1);
        let fields = members
            .iter()
            .zip(&widths)
            .map(|(m, w)| {
                (
                    m.name.name.clone(),
                    0u32,
                    *w,
                    Self::member_ascending(&m.range),
                    m.signed,
                    Self::member_kind_two_state(m.kind),
                )
            })
            .collect();
        self.struct_layouts
            .insert(tname.name.clone(), StructLayout { fields });
        // A union shares the `struct_layouts` map (for `u.field` member reads) but
        // its overlay layout (all fields at offset 0, width = MAX) is NOT a packed
        // concat — so it is recorded here to EXCLUDE it from the `'{…}` pattern
        // desugar (which would wrongly concatenate the fields). Pattern on a union
        // stays loud.
        self.union_type_names.insert(tname.name.clone());
        // §7.2.1: an all-2-state union is itself 2-state (defaults to 0); any
        // 4-state member makes the whole union 4-state (defaults to X).
        let union_kind =
            if !members.is_empty() && members.iter().all(|m| Self::member_kind_two_state(m.kind)) {
                NetVarKind::Bit
            } else {
                NetVarKind::Logic
            };
        self.typedefs.insert(
            tname.name.clone(),
            TypeInfo {
                kind: union_kind,
                signed: false,
                range: Some(Self::dec_range(total.saturating_sub(1))),
                packed: Vec::new(),
                class_name: None,
            },
        );
        Some(ModuleItem::Typedef(TypedefDecl {
            name: tname,
            kind: TypedefKind::Struct { members },
            span: start.to(self.prev_span()),
        }))
    }

    /// Width of a struct member from its range. `None` ⇒ scalar (1). Only
    /// constant-literal bounds fold (`[7:0]`, `[8-1:0]`); param widths return `None`.
    fn member_width(&self, range: &Option<Range>) -> Option<u32> {
        match range {
            None => Some(1),
            Some(r) => {
                let msb = Self::const_lit(&r.msb)?;
                let lsb = Self::const_lit(&r.lsb)?;
                Some(msb.abs_diff(lsb) as u32 + 1)
            }
        }
    }

    /// Is this member kind a 2-state type (`bit`/`byte`/`shortint`/`int`/`longint`)?
    /// IEEE §7.2.1: a packed struct is 2-state iff EVERY member is 2-state — then
    /// it defaults to 0, not X. `integer`/`time`/`logic`/`reg`/nets are 4-state.
    /// Mirrors `elaborate::net_kind_is_two_state` exactly (the engine's default-fill
    /// reads the same predicate via the `two_state_nets` sidecar).
    fn member_kind_two_state(kind: NetVarKind) -> bool {
        matches!(
            kind,
            NetVarKind::Bit
                | NetVarKind::Byte
                | NetVarKind::Shortint
                | NetVarKind::Int
                | NetVarKind::Longint
        )
    }

    /// Fixed bit-width of a named integer-atom type used as a struct/union member,
    /// or `None` for a vector-capable kind (`bit`/`logic`/`reg`/nets) whose width
    /// is given by the range instead (`member_width`). The atom types carry an
    /// implicit width that no `[msb:lsb]` range follows, so a member declared with
    /// a bare named type (`int a;`) must size from the type — NOT default to 1.
    /// Mirrors the SVA-local-var atom table (`parse_sva_local_decl`) and §6.11.
    fn atom_member_width(kind: NetVarKind) -> Option<u32> {
        match kind {
            NetVarKind::Byte => Some(8),
            NetVarKind::Shortint => Some(16),
            NetVarKind::Int | NetVarKind::Integer => Some(32),
            NetVarKind::Longint | NetVarKind::Time => Some(64),
            _ => None,
        }
    }

    /// Width of a struct/union member from its declared kind AND range. A named
    /// integer-atom kind (`int`/`byte`/…) carries a fixed width (the range, if any,
    /// is not a packed dimension on an atom in this subset); a vector-capable kind
    /// (`bit`/`logic`/`reg`) sizes from the range (`None` ⇒ 1). Returns the width
    /// or `None` when a vector range is present but non-constant.
    fn member_width_kind(&self, kind: NetVarKind, range: &Option<Range>) -> Option<u32> {
        if let Some(w) = Self::atom_member_width(kind) {
            return Some(w);
        }
        self.member_width(range)
    }

    /// Fold a constant-literal expression to `i64` at parse time (decimal literals
    /// and +/-/* of them). Returns `None` for anything non-constant.
    fn const_lit(e: &Expr) -> Option<i64> {
        match &e.kind {
            ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw,
            } => raw
                .chars()
                .filter(|c| *c != '_')
                .collect::<String>()
                .parse::<i64>()
                .ok(),
            ExprKind::Unary {
                op: UnOp::Minus,
                operand,
            } => Self::const_lit(operand)?.checked_neg(),
            ExprKind::Binary { op, lhs, rhs } => {
                let a = Self::const_lit(lhs)?;
                let b = Self::const_lit(rhs)?;
                // Checked arithmetic: an overflowing constant fold returns None
                // (→ caller treats the value as non-foldable / loud) rather than
                // panicking in debug or silently wrapping in release.
                match op {
                    BinOp::Add => a.checked_add(b),
                    BinOp::Sub => a.checked_sub(b),
                    BinOp::Mul => a.checked_mul(b),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// A `[hi:0]` range made of decimal literals, for the synthesized struct vector.
    fn dec_range(hi: u32) -> Range {
        Range {
            msb: Self::dec_lit(hi, Span::new(0, 0)),
            lsb: Self::dec_lit(0, Span::new(0, 0)),
            span: Span::new(0, 0),
        }
    }

    /// A decimal integer-literal expression with the given value.
    fn dec_lit(v: u32, span: Span) -> Expr {
        Expr {
            kind: ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw: v.to_string(),
            },
            span,
        }
    }

    // ───────────────────────── SV §6.19.5 enum methods ─────────────────────
    /// A decimal integer literal for a possibly-negative `i64` (negatives become
    /// `-<magnitude>`). Used to build the enum-method desugar's constants.
    fn i64_lit(v: i64, span: Span) -> Expr {
        let mag = Expr {
            kind: ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw: v.unsigned_abs().to_string(),
            },
            span,
        };
        if v < 0 {
            Expr {
                kind: ExprKind::Unary {
                    op: UnOp::Minus,
                    operand: Box::new(mag),
                },
                span,
            }
        } else {
            mag
        }
    }

    /// `var == val` (logical equality), the condition of an enum-method ternary.
    fn enum_eq(var: &Expr, val: i64, span: Span) -> Expr {
        Expr {
            kind: ExprKind::Binary {
                op: BinOp::Eq,
                lhs: Box::new(var.clone()),
                rhs: Box::new(Self::i64_lit(val, span)),
            },
            span,
        }
    }

    /// `.next`/`.prev` → a ternary chain over the ordered label values. `next`
    /// maps vᵢ→vᵢ₊₁ and wraps the last→first; `prev` maps vᵢ→vᵢ₋₁ and wraps the
    /// first→last (§6.19.5). The default (an out-of-range value) takes the wrap
    /// target, matching the boundary case.
    fn enum_step_chain(var: &Expr, labels: &[(String, i64)], span: Span, is_next: bool) -> Expr {
        let vals: Vec<i64> = labels.iter().map(|(_, v)| *v).collect();
        let n = vals.len();
        // (match_value, result_value) pairs + a default result for any other value.
        let (pairs, default): (Vec<(i64, i64)>, i64) = if is_next {
            (
                (0..n.saturating_sub(1))
                    .map(|i| (vals[i], vals[i + 1]))
                    .collect(),
                vals[0],
            )
        } else {
            let mut p = vec![(vals[0], vals[n - 1])];
            p.extend((1..n).map(|i| (vals[i], vals[i - 1])));
            (p, vals[n - 1])
        };
        pairs
            .iter()
            .rev()
            .fold(Self::i64_lit(default, span), |else_e, (m, r)| Expr {
                kind: ExprKind::Ternary {
                    cond: Box::new(Self::enum_eq(var, *m, span)),
                    then_e: Box::new(Self::i64_lit(*r, span)),
                    else_e: Box::new(else_e),
                },
                span,
            })
    }

    /// If `path` is `var.method` where `var` is a (literal-foldable) enum variable
    /// and `method` ∈ {first,last,num,next,prev,name}, build the §6.19.5 desugar
    /// (constants for first/last/num; ternary chains for next/prev/name). `None`
    /// when it is not such an access — the caller falls through to its normal
    /// path (so `var.bar` on an enum stays a loud undeclared-name error).
    fn enum_method_expr(&self, path: &HierPath) -> Option<Expr> {
        if path.segments.len() != 2 {
            return None;
        }
        let ename = self.var_enum.get(&path.segments[0].name)?;
        let labels = self.enum_defs.get(ename)?;
        if labels.is_empty() {
            return None;
        }
        let span = path.span;
        let var = Expr {
            kind: ExprKind::Ident(HierPath {
                segments: vec![path.segments[0].clone()],
                span,
            }),
            span,
        };
        Some(match path.segments[1].name.as_str() {
            "first" => Self::i64_lit(labels[0].1, span),
            "last" => Self::i64_lit(labels[labels.len() - 1].1, span),
            "num" => Self::i64_lit(labels.len() as i64, span),
            "next" => Self::enum_step_chain(&var, labels, span, true),
            "prev" => Self::enum_step_chain(&var, labels, span, false),
            // `.name`/`.name()` is honest-loud (NOT desugared): a packed
            // string-literal ternary chain pads shorter labels to the longest
            // label's width (a packed-string property iverilog shares for a raw
            // ternary), whereas iverilog's `.name()` returns a dynamic `string`
            // of the EXACT length. Producing a dynamic string needs a statement
            // (string-var assign), not an expression — left to a follow-up slice.
            _ => return None,
        })
    }

    /// If `path` is `var.field` where `var` is a packed-struct variable and `field`
    /// is one of its members, return `(base_path_to_var, lsb_offset, width,
    /// ascending, signed)`.
    fn struct_field_select(&self, path: &HierPath) -> Option<(HierPath, u32, u32, bool, bool)> {
        if path.segments.len() != 2 {
            return None;
        }
        let tyname = self.var_struct.get(&path.segments[0].name)?;
        let (off, w, asc, sgn) = self
            .struct_layouts
            .get(tyname)?
            .field(&path.segments[1].name)?;
        let base = HierPath {
            segments: vec![path.segments[0].clone()],
            span: path.segments[0].span,
        };
        Some((base, off, w, asc, sgn))
    }

    /// IEEE §10.9.1 packed-struct positional assignment pattern. When `rhs` is
    /// `'{e0,…,eN}` and `var_name` is a *scalar* packed-struct variable, desugar
    /// it to the field-width-cast concat `{w0'(e0), …, wN'(eN)}` — field 0 is the
    /// MSB (leftmost). Each element is sized to its FIELD width (NOT
    /// self-determined), so an unsized or fill (`'1`/`'x`/`'z`) element grows to
    /// the field: `'{5,6}` ≠ `{5,6}`. The size cast reuses the existing
    /// `CastTarget::Size` lowering (which sizes a fill operand in the cast width,
    /// §11.6), so no elaborate/IR change is needed — struct layout is parser-only.
    ///
    /// `rhs` is returned untouched when it is not a pattern or `var_name` is not a
    /// scalar struct var (an array-of-struct stays on the 1-D unpacked-array path,
    /// a non-struct var is unaffected) — so every non-struct assignment is
    /// byte-identical. A struct pattern with the wrong element count is a loud
    /// parse error (matching iverilog, which rejects a field-count mismatch).
    fn desugar_struct_assign_pattern(&mut self, var_name: &str, rhs: Expr) -> Expr {
        if !matches!(rhs.kind, ExprKind::AssignPattern(_))
            || !self.struct_scalar_vars.contains(var_name)
        {
            return rhs;
        }
        match self.var_struct.get(var_name).cloned() {
            Some(tyname) => self.build_struct_pattern_concat(&tyname, rhs),
            None => rhs,
        }
    }

    /// Build the field-width-cast concat for a packed-struct `'{…}` pattern whose
    /// target resolves to struct type `tyname` (shared by the scalar-variable path
    /// `s = '{…}` and the 1-D-array-element path `arr[i] = '{…}`). `rhs` must be an
    /// `AssignPattern`. A count mismatch or a 2-state field wider than 64 bits is a
    /// loud parse error (returning the pattern unchanged).
    fn build_struct_pattern_concat(&mut self, tyname: &str, rhs: Expr) -> Expr {
        // Each field's (width, is_two_state) in declaration order (field 0 = MSB =
        // leftmost concat part); cloned out so `self` is free for `error` below.
        let fields: Vec<(u32, bool)> = match self.struct_layouts.get(tyname) {
            Some(l) => l
                .fields
                .iter()
                .map(|(_, _, w, _, _, ts)| (*w, *ts))
                .collect(),
            None => return rhs,
        };
        let span = rhs.span;
        let ExprKind::AssignPattern(elems) = rhs.kind else {
            unreachable!("caller guarantees an AssignPattern rhs")
        };
        if elems.len() != fields.len() {
            self.error("exactly one `'{…}` element for each packed-struct field");
            return Expr {
                kind: ExprKind::AssignPattern(elems),
                span,
            };
        }
        // A 2-state field is X/Z-coerced by squashing the element through
        // `longint'(e)` (the widest 2-state prim) before sizing; one wider than 64
        // bits cannot be squashed this way, so honest-loud rather than silent-wrong.
        if fields.iter().any(|&(w, ts)| ts && w > 64) {
            self.error("a 2-state packed-struct field no wider than 64 bits in `'{…}`");
            return Expr {
                kind: ExprKind::AssignPattern(elems),
                span,
            };
        }
        let parts = elems
            .into_iter()
            .zip(fields)
            .map(|(e, (w, two_state))| {
                // 4-state field: keep the value (plain size cast). 2-state field:
                // coerce X/Z→0 (§6.11.3) via `w'(longint'(e))` — `longint'` squashes
                // unknowns to 0; the size cast then takes the field's low `w` bits.
                let inner = if two_state {
                    Expr {
                        kind: ExprKind::Cast {
                            target: CastTarget::Prim(CastPrim::Longint),
                            expr: Box::new(e),
                        },
                        span,
                    }
                } else {
                    e
                };
                Expr {
                    kind: ExprKind::Cast {
                        target: CastTarget::Size(Box::new(Self::dec_lit(w, span))),
                        expr: Box::new(inner),
                    },
                    span,
                }
            })
            .collect();
        Expr {
            kind: ExprKind::Concat { parts },
            span,
        }
    }

    /// Shared assignment hook for the packed-struct `'{…}` pattern. Desugars two
    /// target shapes: a whole scalar struct variable (`s = '{…}`, a single-segment
    /// `Lvalue::Ident`), and a 1-D struct-array element (`arr[i] = '{…}`, a
    /// `BitSelect` of a plain Ident in `struct_1d_array_vars`). Every other lvalue
    /// (field-select, multi-dim/nested index, concat, scalar bit-select) is left
    /// untouched (loud downstream). Called at every statement/expression
    /// `lvalue = rhs` site — blocking/nonblocking, continuous /
    /// procedural-continuous / `force` assigns, and for-init/step. (A scalar
    /// decl-init `st_t s = '{…}` is desugared by a direct `desugar_struct_assign_pattern`
    /// call in `parse_typed_decl`, not through this hook.)
    fn maybe_struct_pattern_rhs(&mut self, lhs: &Lvalue, rhs: Expr) -> Expr {
        // Fast path: only `'{…}` to an eligible target can desugar; every other
        // assignment returns `rhs` untouched (byte-identical) with no work.
        if !matches!(rhs.kind, ExprKind::AssignPattern(_)) {
            return rhs;
        }
        match lhs {
            // Whole scalar struct variable `s = '{…}`.
            Lvalue::Ident(p) if p.segments.len() == 1 => {
                let nm = p.segments[0].name.clone();
                self.desugar_struct_assign_pattern(&nm, rhs)
            }
            // 1-D struct-array element `arr[i] = '{…}`: the base must be a plain
            // single-name Ident registered as a 1-D struct array (this excludes a
            // scalar struct's bit-select `s[i]`, a multi-dim element `arr[i][j]`
            // whose base is itself a BitSelect, and a union array). The element's
            // struct type is the array variable's struct type.
            Lvalue::BitSelect { base, .. } => {
                if let Lvalue::Ident(p) = base.as_ref() {
                    if p.segments.len() == 1
                        && self.struct_1d_array_vars.contains(&p.segments[0].name)
                    {
                        if let Some(tyname) = self.var_struct.get(&p.segments[0].name).cloned() {
                            return self.build_struct_pattern_concat(&tyname, rhs);
                        }
                    }
                }
                rhs
            }
            _ => rhs,
        }
    }

    /// Is a packed-struct member declared with an ascending range (`logic [0:N]`,
    /// so field index 0 is the MSB)? Scalar members (no range) are not ascending.
    fn member_ascending(range: &Option<Range>) -> bool {
        match range {
            Some(r) => match (Self::const_lit(&r.msb), Self::const_lit(&r.lsb)) {
                (Some(m), Some(l)) => m < l,
                _ => false,
            },
            None => false,
        }
    }

    /// Build the read-side `Expr` for a packed-struct member access. The base is
    /// always the field part-select `pv = s[off+w-1 : off]`; a trailing sub-select
    /// becomes an `IndexedPart` on `pv` (FIELD-bounded, direction-aware).
    /// `#[inline(never)]` keeps these locals out of the recursive `expr_primary`
    /// frame (see MAX_EXPR_DEPTH).
    ///
    /// `sgn` is the member's effective signedness. A WHOLE-field read of a signed
    /// member (`int`/`byte`/… or a `signed`-qualified vector) is wrapped in a
    /// `signed'(pv)` cast so it reads back negative — a packed-struct member ref is
    /// TYPED, not a raw part-select, so iverilog preserves member signedness here.
    /// A sub-select (`s.f[a:b]`) stays unsigned (§5.4.1), matching iverilog.
    #[inline(never)]
    fn struct_member_expr(
        &mut self,
        base: HierPath,
        off: u32,
        w: u32,
        asc: bool,
        sgn: bool,
        span: Span,
    ) -> Expr {
        let pv = Expr {
            kind: ExprKind::PartSelect {
                base: Box::new(Expr {
                    kind: ExprKind::Ident(base),
                    span,
                }),
                msb: Box::new(Self::dec_lit(off + w - 1, span)),
                lsb: Box::new(Self::dec_lit(off, span)),
            },
            span,
        };
        match self.parse_struct_field_sel(w, asc) {
            FieldSel::Whole if sgn => Expr {
                kind: ExprKind::Cast {
                    target: CastTarget::Signing { signed: true },
                    expr: Box::new(pv),
                },
                span,
            },
            FieldSel::Whole => pv,
            FieldSel::Indexed { offset, width, dir } => Expr {
                kind: ExprKind::IndexedPart {
                    base: Box::new(pv),
                    offset: Box::new(offset),
                    width: Box::new(width),
                    dir,
                },
                span: span.to(self.prev_span()),
            },
        }
    }

    /// Map a member-relative bit index `e` onto the field part-select `pv[w-1:0]`.
    /// Descending member: `pv[e]` (identity). Ascending member: `pv[w-1-e]`
    /// (field index 0 is the field MSB, which is `pv`'s high bit). `e` may be
    /// runtime; constant `w` folds in elaborate.
    fn remap_pv_idx(w: u32, ascending: bool, e: Expr) -> Expr {
        if ascending {
            let sp = e.span;
            mk_bin(BinOp::Sub, Self::dec_lit(w - 1, sp), e)
        } else {
            e
        }
    }

    /// Parse the trailing `[...]` of a packed-struct member READ sub-select and
    /// normalize it to one indexed part-select on the field part-select `pv`. No
    /// `[` ⇒ `Whole`. Every form is FIELD-bounded by `pv` (OOB reads X). For an
    /// ascending member the `+:`/`-:` direction flips and the offset mirrors,
    /// matching an ascending NET part-select; a reversed regular range (`s.f[3:0]`
    /// on `logic [0:N]`, or `s.f[0:3]` on `logic [N:0]`) is a loud parse error.
    fn parse_struct_field_sel(&mut self, w: u32, ascending: bool) -> FieldSel {
        if self.peek() != Some(TokenKind::LBracket) {
            return FieldSel::Whole;
        }
        self.bump(); // '['
        let first = self.expr(0);
        let sel = match self.peek() {
            // regular `[a:b]` — bounds must be constant and run in the member's
            // declared direction (a≥b descending, a≤b ascending). Normalize to the
            // equivalent indexed part `[min(a,b) +: |a-b|+1]` and reuse the indexed
            // remap below, so an out-of-field range X-extends on the correct end
            // (the indexed path is differentially validated against the NET oracle).
            Some(TokenKind::Colon) => {
                self.bump();
                let last = self.expr(0);
                match (Self::const_lit(&first), Self::const_lit(&last)) {
                    (Some(a), Some(b)) if (ascending && a <= b) || (!ascending && a >= b) => {
                        let lo = a.min(b).max(0) as u32;
                        let width = (a - b).unsigned_abs() as u32 + 1;
                        let dir = if ascending {
                            PartDir::MinusColon
                        } else {
                            PartDir::PlusColon
                        };
                        FieldSel::Indexed {
                            offset: Self::remap_pv_idx(w, ascending, Self::dec_lit(lo, first.span)),
                            width: Self::dec_lit(width, first.span),
                            dir,
                        }
                    }
                    _ => {
                        self.error_at(
                            first.span,
                            "packed-struct member part-select must be a constant range in the \
                             member's declared direction",
                        );
                        FieldSel::Indexed {
                            offset: Self::dec_lit(0, first.span),
                            width: Self::dec_lit(1, first.span),
                            dir: PartDir::PlusColon,
                        }
                    }
                }
            }
            Some(TokenKind::PlusColon) => {
                self.bump();
                let width = self.expr(0);
                let dir = if ascending {
                    PartDir::MinusColon
                } else {
                    PartDir::PlusColon
                };
                FieldSel::Indexed {
                    offset: Self::remap_pv_idx(w, ascending, first),
                    width,
                    dir,
                }
            }
            Some(TokenKind::MinusColon) => {
                self.bump();
                let width = self.expr(0);
                let dir = if ascending {
                    PartDir::PlusColon
                } else {
                    PartDir::MinusColon
                };
                FieldSel::Indexed {
                    offset: Self::remap_pv_idx(w, ascending, first),
                    width,
                    dir,
                }
            }
            // bit-select `[i]` — a width-1 indexed part-select on `pv`.
            _ => {
                let span = first.span;
                FieldSel::Indexed {
                    offset: Self::remap_pv_idx(w, ascending, first),
                    width: Self::dec_lit(1, span),
                    dir: PartDir::PlusColon,
                }
            }
        };
        self.expect(TokenKind::RBracket, "']'");
        sel
    }

    fn parse_cont_assign(&mut self) -> Option<ContinuousAssign> {
        let start = self.cur_span();
        self.bump(); // assign
        let delay = if self.peek() == Some(TokenKind::Hash) {
            self.parse_delay()
        } else {
            None
        };
        let mut assigns = Vec::new();
        loop {
            let lv = self.parse_lvalue();
            self.expect(TokenKind::Eq, "'=' in assign");
            let rhs = self.expr(0);
            // §10.9.1 packed-struct `'{…}` pattern on a continuous assign to a
            // whole struct net (same desugar as a procedural assign).
            let rhs = self.maybe_struct_pattern_rhs(&lv, rhs);
            assigns.push((lv, rhs));
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semi, "';'");
        Some(ContinuousAssign {
            delay,
            assigns,
            span: start.to(self.prev_span()),
        })
    }
    /// `#5` | `#(d)` | `#(r,f)` | `#(r,f,t)`. Each paren'd value may be mintypmax
    /// `1:2:3` (verdict M2). Uses `parse_delay_value` which accepts `a:b:c`.
    fn parse_delay(&mut self) -> Option<Delay> {
        let start = self.cur_span();
        self.bump(); // '#'
        let mut values = Vec::new();
        if self.eat(TokenKind::LParen) {
            loop {
                values.push(self.parse_delay_value());
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')'");
        } else {
            // bare `#delay_value`: a single number/ident (no parens) — high bp,
            // no mintypmax (a bare `#1:2:3` is not legal V2005 delay).
            values.push(self.expr(UNARY_BP));
        }
        Some(Delay {
            values,
            span: start.to(self.prev_span()),
        })
    }
    /// A delay value inside `#(…)`: `expr` or `min:typ:max` (verdict M2).
    fn parse_delay_value(&mut self) -> Expr {
        let start = self.cur_span();
        let first = self.expr(0);
        if self.peek() == Some(TokenKind::Colon) {
            self.bump();
            let typ = self.expr(0);
            self.expect(TokenKind::Colon, "':' in min:typ:max delay");
            let max = self.expr(0);
            Expr {
                kind: ExprKind::MinTypMax {
                    min: Box::new(first),
                    typ: Box::new(typ),
                    max: Box::new(max),
                },
                span: start.to(self.prev_span()),
            }
        } else {
            first
        }
    }

    /// LHS = concat of selects/idents only. Parse directly to `Lvalue`.
    fn parse_lvalue(&mut self) -> Lvalue {
        if self.peek() == Some(TokenKind::LBrace) {
            let start = self.cur_span();
            self.bump();
            let mut parts = Vec::new();
            loop {
                parts.push(self.parse_lvalue());
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBrace, "'}'");
            return Lvalue::Concat {
                parts,
                span: start.to(self.prev_span()),
            };
        }
        let Some(path) = self.hier_path() else {
            let s = self.cur_span();
            return Lvalue::Error(s);
        };
        // packed-struct member target `s.field = …` → constant part-select lvalue.
        // A trailing WRITE sub-select (`s.f[a:b] = …` / `s.f[i] = …`) folds to a
        // FLAT field-bounded part-select on the struct net, mirroring the READ-side
        // normalization (`parse_struct_field_sel`): the member's declared direction
        // and offset map the source index onto the flat vector, so the write never
        // leaks past the member region. An indexed `[i±:w]` / runtime / reverse
        // sub-select stays loud (iverilog 13.0 itself refuses those struct-member
        // writes — "sorry: not yet supported" — so there is no oracle to match).
        let mut lv = if let Some((base, off, w, asc, _sgn)) = self.struct_field_select(&path) {
            let span = path.span;
            if self.peek() == Some(TokenKind::LBracket) {
                self.parse_struct_field_lval(base, off, w, asc, span)
            } else {
                Lvalue::PartSelect {
                    base: Box::new(Lvalue::Ident(base)),
                    msb: Box::new(Self::dec_lit(off + w - 1, span)),
                    lsb: Box::new(Self::dec_lit(off, span)),
                    span,
                }
            }
        } else {
            Lvalue::Ident(path)
        };
        loop {
            if self.peek() == Some(TokenKind::LBracket) {
                let start = lv.span();
                self.bump();
                let first = self.expr(0);
                lv = match self.peek() {
                    Some(TokenKind::Colon) => {
                        self.bump();
                        let lsb = self.expr(0);
                        self.expect(TokenKind::RBracket, "']'");
                        Lvalue::PartSelect {
                            base: Box::new(lv),
                            msb: Box::new(first),
                            lsb: Box::new(lsb),
                            span: start.to(self.prev_span()),
                        }
                    }
                    Some(TokenKind::PlusColon) => {
                        self.bump();
                        let w = self.expr(0);
                        self.expect(TokenKind::RBracket, "']'");
                        Lvalue::IndexedPart {
                            base: Box::new(lv),
                            offset: Box::new(first),
                            width: Box::new(w),
                            dir: PartDir::PlusColon,
                            span: start.to(self.prev_span()),
                        }
                    }
                    Some(TokenKind::MinusColon) => {
                        self.bump();
                        let w = self.expr(0);
                        self.expect(TokenKind::RBracket, "']'");
                        Lvalue::IndexedPart {
                            base: Box::new(lv),
                            offset: Box::new(first),
                            width: Box::new(w),
                            dir: PartDir::MinusColon,
                            span: start.to(self.prev_span()),
                        }
                    }
                    _ => {
                        self.expect(TokenKind::RBracket, "']'");
                        Lvalue::BitSelect {
                            base: Box::new(lv),
                            index: Box::new(first),
                            span: start.to(self.prev_span()),
                        }
                    }
                };
            } else if self.peek() == Some(TokenKind::Dot) && Self::is_indexed_hier_lval(&lv) {
                // HIER-REST②: `g[0].x = …` — fold the constant index into the
                // scope-segment name, mirroring the expression side.
                lv = self.parse_indexed_hier_lval(lv);
            } else {
                break;
            }
        }
        lv
    }

    /// Parse the trailing `[...]` of a packed-struct member WRITE sub-select and
    /// fold it to a FLAT, field-bounded lvalue on the struct net `base[total-1:0]`.
    /// The cursor is on the `[`. `off`/`w`/`asc` are the member's flat offset, width
    /// and declared direction (ascending = `logic [0:N]`, source index 0 = field
    /// MSB). This is the WRITE twin of [`Self::parse_struct_field_sel`]: every form
    /// maps a SOURCE index `k` onto flat bit `off + k` (descending) or
    /// `off + (w-1-k)` (ascending), so the write stays inside the member region —
    /// never leaking into an adjacent member.
    ///
    /// SCOPE (correct-or-loud): only a CONSTANT range `[a:b]` running in the
    /// member's declared direction and a CONSTANT bit-select `[i]` are folded —
    /// these are exactly the forms iverilog 13.0 supports for a struct-member
    /// write. An indexed `[i±:w]`, a runtime/non-constant index, or a reversed
    /// range is a loud parse error (iverilog refuses the indexed/runtime forms
    /// outright; the reversed range stays loud to match the READ side). An OOB
    /// bit-select drops (no-op), matching iverilog; an OOB range is loud (iverilog
    /// itself asserts on it — no oracle).
    fn parse_struct_field_lval(
        &mut self,
        base: HierPath,
        off: u32,
        w: u32,
        asc: bool,
        span: Span,
    ) -> Lvalue {
        self.bump(); // '['
        let first = self.expr(0);
        match self.peek() {
            // Regular `[a:b]` — must be constant and run in the member's direction.
            Some(TokenKind::Colon) => {
                self.bump();
                let last = self.expr(0);
                let end = self.cur_span();
                self.expect(TokenKind::RBracket, "']'");
                match (Self::const_lit(&first), Self::const_lit(&last)) {
                    // In-direction range, both bounds inside the field [0, w).
                    (Some(a), Some(b))
                        if ((asc && a <= b) || (!asc && a >= b))
                            && a >= 0
                            && b >= 0
                            && (a as u32) < w
                            && (b as u32) < w =>
                    {
                        let (ka, kb) = (a as u32, b as u32);
                        // Map source MSB/LSB index onto the flat vector. Ascending:
                        // source index k → flat `off + (w-1-k)`; descending: `off + k`.
                        let (fmsb, flsb) = if asc {
                            (off + (w - 1 - ka), off + (w - 1 - kb))
                        } else {
                            (off + ka, off + kb)
                        };
                        Lvalue::PartSelect {
                            base: Box::new(Lvalue::Ident(base)),
                            msb: Box::new(Self::dec_lit(fmsb, span)),
                            lsb: Box::new(Self::dec_lit(flsb, span)),
                            span: span.to(self.prev_span()),
                        }
                    }
                    _ => {
                        self.error_at(
                            span.to(end),
                            "a constant in-bounds packed-struct member range WRITE in the \
                             member's declared direction",
                        );
                        Lvalue::Error(span.to(self.prev_span()))
                    }
                }
            }
            // Bit-select `[i]` — constant index; OOB drops (no-op), matching iverilog.
            Some(TokenKind::RBracket) => {
                self.bump(); // ']'
                match Self::const_lit(&first) {
                    Some(i) if i >= 0 && (i as u32) < w => {
                        let fbit = if asc {
                            off + (w - 1 - i as u32)
                        } else {
                            off + i as u32
                        };
                        Lvalue::BitSelect {
                            base: Box::new(Lvalue::Ident(base)),
                            index: Box::new(Self::dec_lit(fbit, span)),
                            span: span.to(self.prev_span()),
                        }
                    }
                    Some(_) => {
                        // OOB bit-select: iverilog drops it (no-op). Address a flat
                        // bit guaranteed past the struct net so the engine drops the
                        // write too — never a leak into a neighbour member.
                        Lvalue::BitSelect {
                            base: Box::new(Lvalue::Ident(base)),
                            index: Box::new(Self::dec_lit(OOB_DROP_BIT, span)),
                            span: span.to(self.prev_span()),
                        }
                    }
                    None => {
                        self.error_at(
                            span.to(self.prev_span()),
                            "a constant packed-struct member bit-select WRITE index",
                        );
                        Lvalue::Error(span.to(self.prev_span()))
                    }
                }
            }
            // Indexed `[i+:w]` / `[i-:w]`: iverilog refuses these for a struct-member
            // write ("sorry: not yet supported"), so there is no oracle — loud.
            Some(TokenKind::PlusColon) | Some(TokenKind::MinusColon) => {
                self.bump();
                let _ = self.expr(0);
                let _ = self.expect(TokenKind::RBracket, "']'");
                self.error_at(
                    span.to(self.prev_span()),
                    "a constant `[a:b]` range or bit-select on a packed-struct member \
                     WRITE (an indexed `[i+:w]`/`[i-:w]` is unsupported — iverilog \
                     refuses it too)",
                );
                Lvalue::Error(span.to(self.prev_span()))
            }
            _ => {
                self.error_at(span, "']' or ':' after a struct-member sub-select index");
                Lvalue::Error(span.to(self.prev_span()))
            }
        }
    }

    /// True when `lv` is `name[idx]` — a bit-select lvalue rooted at a plain Ident.
    fn is_indexed_hier_lval(lv: &Lvalue) -> bool {
        matches!(lv, Lvalue::BitSelect { base, .. } if matches!(**base, Lvalue::Ident(_)))
    }

    /// LVALUE twin of [`Self::parse_indexed_hier`]: `g[0].x = …`.
    fn parse_indexed_hier_lval(&mut self, base: Lvalue) -> Lvalue {
        let start = base.span();
        let mut segs: Vec<Ident> = Vec::new();
        if let Lvalue::BitSelect { base: b, index, .. } = base {
            if let Lvalue::Ident(p) = *b {
                let n = p.segments.len();
                for (i, seg) in p.segments.into_iter().enumerate() {
                    if i + 1 == n {
                        let idx_str = self.const_index_string(&index);
                        segs.push(Ident {
                            name: format!("{}[{idx_str}]", seg.name),
                            span: seg.span,
                        });
                    } else {
                        segs.push(seg);
                    }
                }
            }
        }
        while self.eat(TokenKind::Dot) {
            match self.ident() {
                Some(id) => segs.push(id),
                None => break,
            }
        }
        let hi = segs.last().map(|s| s.span).unwrap_or(start);
        Lvalue::Ident(HierPath {
            segments: segs,
            span: start.to(hi),
        })
    }
}

// ════════════════════════ PR3: generate / genvar ════════════════════════
//
// Parse-only: build the hdl-ast `GenerateConstruct`/`GenItem` tree; elaborate
// unrolls it. Mirrors the procedural for/if/case shapes (PR2) but produces
// `GenItem`s, not `Stmt`s. Every loop over a sub-item list carries a
// forward-progress guard (`pos == before → bump`) so malformed input can never
// spin, matching the rest of the parser's recovery discipline.
impl<'t, 's> Parser<'t, 's> {
    /// `genvar i, j;` → `ModuleItem::Genvar{names, span}`. The `genvar` keyword is
    /// already at `peek()`. An empty/garbled name list still terminates at `;`.
    fn parse_genvar_decl(&mut self) -> ModuleItem {
        let start = self.cur_span();
        self.bump(); // `genvar`
        let mut names = Vec::new();
        if let Some(id) = self.ident() {
            names.push(id);
            while self.eat(TokenKind::Comma) {
                match self.ident() {
                    Some(id) => names.push(id),
                    None => break, // diagnosed by ident(); stop the list
                }
            }
        }
        self.expect(TokenKind::Semi, "';' after genvar declaration");
        ModuleItem::Genvar {
            names,
            span: start.to(self.prev_span()),
        }
    }

    /// `generate <gen_items> endgenerate`. Dispatch only calls this on the
    /// `generate` keyword; the SV bare-`if`/`for`/`case`-at-module-scope form is a
    /// DEFERRED variant.
    fn parse_generate_construct(&mut self) -> GenerateConstruct {
        let start = self.cur_span();
        self.bump(); // `generate`
        let items = self.parse_gen_items_until(&|p| p.at_kw(Kw::Endgenerate) || p.at_eof());
        self.expect(
            TokenKind::Word(WordKind::Keyword(Kw::Endgenerate)),
            "'endgenerate'",
        );
        GenerateConstruct {
            items,
            span: start.to(self.prev_span()),
        }
    }

    /// Parse `GenItem`s until `stop` is true (or EOF). Shared by the construct
    /// body, gen-blocks (`begin … end`), and case-item bodies. Forward-progress
    /// guarded.
    fn parse_gen_items_until(&mut self, stop: &dyn Fn(&Self) -> bool) -> Vec<GenItem> {
        let mut items = Vec::new();
        while !self.at_eof() && !stop(self) {
            let before = self.pos;
            if let Some(it) = self.parse_gen_item() {
                items.push(it);
            }
            if self.pos == before {
                self.bump(); // never spin on a stuck gen-item
            }
        }
        items
    }

    /// One generate item: `for` / `if` / `case` / `begin…end` block / genvar decl
    /// / a plain module-item (instance, cont-assign, net, procedural block). A
    /// stray `;` (empty item) is consumed and yields nothing.
    fn parse_gen_item(&mut self) -> Option<GenItem> {
        if self.eat(TokenKind::Semi) {
            return None; // empty generate item
        }
        if self.at_kw(Kw::For) {
            return Some(self.parse_gen_for());
        }
        if self.at_kw(Kw::If) {
            return Some(self.parse_gen_if());
        }
        if self.at_kw(Kw::Case) {
            return Some(self.parse_gen_case());
        }
        if self.at_kw(Kw::Begin) {
            return Some(self.parse_gen_block());
        }
        // genvar decls inside generate are legal — keep them wrapped so elaborate's
        // no-op handler ignores them (they never become nets).
        if self.at_kw(Kw::Genvar) {
            return Some(GenItem::Item(Box::new(self.parse_genvar_decl())));
        }
        // anything else → a plain module-item (instance / assign / net / proc / …).
        // `parse_module_item` returns None only after recording an error; wrap a
        // real item, else propagate None (the caller's progress guard syncs).
        self.parse_module_item()
            .map(|mi| GenItem::Item(Box::new(mi)))
    }

    /// `for ( genvar_id = e ; cond ; genvar_id = e ) gen_block`. A `begin : label`
    /// hoists its label onto the For node (see `parse_gen_branch`).
    fn parse_gen_for(&mut self) -> GenItem {
        let start = self.cur_span();
        self.bump(); // `for`
        self.expect(TokenKind::LParen, "'(' after generate 'for'");
        let init = self.parse_gen_assign();
        self.expect(TokenKind::Semi, "';' after generate-for init");
        let cond = self.expr(0);
        self.expect(TokenKind::Semi, "';' after generate-for cond");
        let step = self.parse_gen_assign();
        self.expect(TokenKind::RParen, "')' after generate-for header");
        let (label, body) = self.parse_gen_branch();
        GenItem::For {
            init,
            cond,
            step,
            label,
            body,
            span: start.to(self.prev_span()),
        }
    }

    /// `genvar_id = expr` (no trailing `;`) for a generate-for init/step. LHS is a
    /// single genvar identifier (the LRM restricts it — not a general lvalue).
    fn parse_gen_assign(&mut self) -> GenAssign {
        let start = self.cur_span();
        let lvalue = self.ident().unwrap_or(Ident {
            name: String::new(),
            span: start,
        });
        self.expect(TokenKind::Eq, "'=' in generate-for assignment");
        let value = self.expr(0);
        GenAssign {
            lvalue,
            value,
            span: start.to(self.prev_span()),
        }
    }

    /// `if ( cond ) gen_item [ else gen_item ]`. Dangling-else binds EAGERLY to the
    /// nearest `if` (same rule as the procedural parser).
    fn parse_gen_if(&mut self) -> GenItem {
        let start = self.cur_span();
        self.bump(); // `if`
        self.expect(TokenKind::LParen, "'(' after generate 'if'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')' after generate-if condition");
        let (label, then_b) = self.parse_gen_branch();
        let else_b = if self.eat_kw(Kw::Else) {
            self.parse_gen_branch().1
        } else {
            Vec::new()
        };
        GenItem::If {
            cond,
            then_b,
            else_b,
            label,
            span: start.to(self.prev_span()),
        }
    }

    /// `case ( e ) { label{,label}: gen_item | default[:] gen_item } endcase`.
    fn parse_gen_case(&mut self) -> GenItem {
        let start = self.cur_span();
        self.bump(); // `case`
        self.expect(TokenKind::LParen, "'(' after generate 'case'");
        let scrutinee = self.expr(0);
        self.expect(TokenKind::RParen, "')' after generate-case scrutinee");
        let mut items = Vec::new();
        while !self.at_eof() && !self.at_kw(Kw::Endcase) {
            let before = self.pos;
            items.push(self.parse_gen_case_item());
            if self.pos == before {
                self.bump(); // never spin on a stuck case item
            }
        }
        self.expect(
            TokenKind::Word(WordKind::Keyword(Kw::Endcase)),
            "'endcase' for generate-case",
        );
        GenItem::Case {
            scrutinee,
            items,
            span: start.to(self.prev_span()),
        }
    }

    /// One generate-case item: `default [:] gen_item` | `label {, label} : gen_item`.
    fn parse_gen_case_item(&mut self) -> GenCaseItem {
        let start = self.cur_span();
        if self.eat_kw(Kw::Default) {
            self.eat(TokenKind::Colon); // ':' OPTIONAL after default
            let body = self.parse_gen_branch().1;
            return GenCaseItem::Default {
                body,
                span: start.to(self.prev_span()),
            };
        }
        let mut labels = vec![self.expr(0)];
        while !self.node_budget_blown && self.eat(TokenKind::Comma) {
            labels.push(self.expr(0));
        }
        self.expect(TokenKind::Colon, "':' in generate-case item");
        let body = self.parse_gen_branch().1;
        GenCaseItem::Match {
            labels,
            body,
            span: start.to(self.prev_span()),
        }
    }

    /// `begin [: label] gen_items end [: label]` → a `GenItem::Block`.
    fn parse_gen_block(&mut self) -> GenItem {
        let start = self.cur_span();
        self.bump(); // `begin`
        let label = self.opt_block_label(); // reuse PR2 helper (`: name` or None)
        let items = self.parse_gen_items_until(&|p| p.at_kw(Kw::End) || p.at_eof());
        self.expect(TokenKind::Word(WordKind::Keyword(Kw::End)), "'end'");
        self.opt_block_label(); // optional `: end_label` (no AST slot → discard)
        GenItem::Block {
            label,
            items,
            span: start.to(self.prev_span()),
        }
    }

    /// Parse a control-structure BRANCH body and HOIST a `begin:label` label out of
    /// it. Returns `(label, items)`:
    /// - `begin [: lbl] … end` → `(lbl, inner_items)` (the begin/end is unwrapped so
    ///   the For/If node carries the label directly — elaborate's `label[idx]`
    ///   prefixing expects the loop/if to OWN the label).
    /// - any other single gen-item → `(None, vec![item])`.
    fn parse_gen_branch(&mut self) -> (Option<Ident>, Vec<GenItem>) {
        if self.at_kw(Kw::Begin) {
            match self.parse_gen_block() {
                GenItem::Block { label, items, .. } => (label, items),
                other => (None, vec![other]), // unreachable; defensive
            }
        } else {
            match self.parse_gen_item() {
                Some(it) => (None, vec![it]),
                None => (None, Vec::new()),
            }
        }
    }
}

// ════════════════════════ PR2: statements + procedural blocks ════════════════════════
impl<'t, 's> Parser<'t, 's> {
    // ─────────────────────── 1. procedural blocks ───────────────────────
    /// `initial S` | `always [@(…)] S` | `always_ff @(…) S` | `always_comb S`
    /// | `always_latch S`. For `always`/`always_ff` a leading `@(…)` folds onto
    /// `ProceduralBlock.sensitivity`.
    fn parse_procedural_block(&mut self) -> ProceduralBlock {
        let start = self.cur_span();
        let kind = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(k))) => match k {
                Kw::Initial => ProcKind::Initial,
                Kw::Always => ProcKind::Always,
                Kw::AlwaysFf => ProcKind::AlwaysFf,
                Kw::AlwaysComb => ProcKind::AlwaysComb,
                Kw::AlwaysLatch => ProcKind::AlwaysLatch,
                Kw::Final => ProcKind::Final,
                _ => unreachable!("parse_procedural_block: caller pre-screens proc kw"),
            },
            _ => unreachable!("parse_procedural_block: caller pre-screens proc kw"),
        };
        self.bump(); // initial / always*

        let sensitivity = match kind {
            ProcKind::Always | ProcKind::AlwaysFf if self.peek() == Some(TokenKind::At) => {
                Some(self.parse_sensitivity())
            }
            _ => None,
        };

        let body = Box::new(self.parse_statement());
        ProceduralBlock {
            kind,
            sensitivity,
            body,
            span: start.to(self.prev_span()),
        }
    }

    // ─────────────────────── 1b. function / task definitions ───────────────────────
    /// `function [automatic] [signed] [range] [ret_type] name [(tf_ports)] ;
    ///    {body_decl} body_stmt endfunction`
    /// V2005: return width = `signed` + `range`; `ret_type` is one of
    /// ParamType::{Implicit,Integer,Real,Realtime,Time} (a `reg [N]` return maps to
    /// Implicit + range — ParamType has no Reg/Logic). Ports may be ANSI (in the
    /// paren list) or non-ANSI (input/output decls in the body prefix, hoisted).
    /// N7: register every `class NAME` in the token stream as a class-typed
    /// alias so `NAME var;` parses (forward-reference safe; any nesting).
    fn prescan_class_names(&mut self) {
        let mut names: Vec<String> = Vec::new();
        for i in 0..self.toks.len() {
            if matches!(
                self.toks[i].kind,
                TokenKind::Word(WordKind::Keyword(Kw::Class))
            ) {
                if let Some(t) = self.toks.get(i + 1) {
                    if matches!(t.kind, TokenKind::Word(WordKind::Ident)) {
                        names.push(self.src[t.span.clone()].to_string());
                    }
                }
            }
        }
        for n in names {
            self.typedefs.entry(n.clone()).or_insert(TypeInfo {
                kind: NetVarKind::ClassHandle,
                signed: false,
                range: None,
                packed: Vec::new(),
                class_name: Some(n.clone()),
            });
        }
    }

    /// `class NAME [extends BASE] ; { class_item } endclass [: NAME]` (N7).
    /// Parameterized classes (`class C #(…)`) and `virtual class` (abstract) are
    /// loud-deferred at elaborate; here we parse the plain single-inheritance
    /// form. Returns `None` only on a missing class name.
    /// `virtual [interface] IFACE name [, name2];` (ⓑ-breadth, §25.9) — a virtual
    /// interface handle. The interface type name rides `class_type`; elaborate
    /// resolves the static binding alias.
    fn parse_virtual_iface_decl(&mut self) -> Option<NetVarDecl> {
        let start = self.cur_span();
        self.bump(); // `virtual`
        let _ = self.eat_kw(Kw::Interface); // optional `interface` keyword
        let iface = self.ident()?;
        let mut names = Vec::new();
        loop {
            let Some(name) = self.ident() else { break };
            names.push(DeclName {
                span: name.span,
                name,
                unpacked: Vec::new(),
                init: None,
            });
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semi, "';' after a virtual interface declaration");
        Some(NetVarDecl {
            kind: NetVarKind::VirtualIface,
            signed: false,
            range: None,
            packed: Vec::new(),
            delay: None,
            names,
            lifetime: None,
            class_type: Some(iface),
            class_args: Vec::new(),
            const_param: false,
            span: start.to(self.prev_span()),
        })
    }

    fn parse_class_decl(&mut self) -> Option<ClassDecl> {
        let start = self.cur_span();
        self.bump(); // 'class'
        let name = self.ident()?;
        // ⓑ-breadth (§8.25): `class C #(int W = 8, …)` value parameter list.
        let params = if self.peek() == Some(TokenKind::Hash) {
            self.parse_class_param_list()
        } else {
            Vec::new()
        };
        let extends = if self.eat_kw(Kw::Extends) {
            self.ident()
        } else {
            None
        };
        self.expect(TokenKind::Semi, "';' after class header");
        let mut items = Vec::new();
        while !self.at_eof() && !self.at_kw(Kw::Endclass) {
            let before = self.pos;
            if let Some(it) = self.parse_class_item() {
                items.push(it);
            }
            if self.pos == before {
                self.bump(); // guarantee forward progress
            }
        }
        self.expect(
            TokenKind::Word(WordKind::Keyword(Kw::Endclass)),
            "'endclass'",
        );
        self.opt_block_label(); // optional `: name`
        Some(ClassDecl {
            name,
            params,
            extends,
            items,
            span: start.to(self.prev_span()),
        })
    }

    /// Parse `#( [parameter] [type] NAME [= DEFAULT], … )` class value parameters
    /// (ⓑ-breadth, §8.25). The optional `parameter` keyword and a leading type
    /// (`int`/`logic [W]`/…) are accepted and ignored for layout (the value is what
    /// matters); a missing default is allowed (the spec must then override it).
    fn parse_class_param_list(&mut self) -> Vec<ClassParam> {
        self.bump(); // `#`
        let mut params = Vec::new();
        if !self.expect(TokenKind::LParen, "'(' after `#` in a class parameter list") {
            return params;
        }
        if self.peek() != Some(TokenKind::RParen) {
            loop {
                let _ = self.eat_kw(Kw::Parameter); // optional `parameter`
                                                    // optional leading type: a net/var kind keyword + optional range.
                if self.net_var_kind().is_some() {
                    self.bump();
                    let _ = self.opt_signed();
                    let _ = self.opt_range();
                }
                let Some(name) = self.ident() else { break };
                let default = if self.eat(TokenKind::Eq) {
                    Some(self.expr(0))
                } else {
                    None
                };
                params.push(ClassParam { name, default });
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "')' to close the class parameter list");
        params
    }

    /// One class member: `[virtual] function/task …`, a data member `T name;`,
    /// or a loud-rejected deferred qualifier (`rand`/`static`/…). An optional
    /// leading `local`/`protected` access qualifier (IEEE §8.18) rides as the
    /// member `Visibility` (the default is public).
    fn parse_class_item(&mut self) -> Option<ClassItem> {
        if self.at_lex_error() {
            let s = self.cur_span();
            self.bump();
            return Some(ClassItem::Error(s));
        }
        // IEEE §8.18 access control: an optional leading `local`/`protected`
        // qualifier (contextual idents, not lexer keywords). Consume it once; it
        // may precede a `[virtual] function/task` or a plain data member. A
        // duplicate (`local protected x`) is loud — never silently take the last.
        let vis = if self.eat_ident_kw("local") {
            Visibility::Local
        } else if self.eat_ident_kw("protected") {
            Visibility::Protected
        } else {
            Visibility::Public
        };
        if vis != Visibility::Public && (self.at_ident_kw("local") || self.at_ident_kw("protected"))
        {
            self.error("a single `local`/`protected` access qualifier on a class member");
            let s = self.cur_span();
            self.skip_class_item_recover();
            return Some(ClassItem::Error(s));
        }
        let is_virtual = self.eat_kw(Kw::Virtual);
        if self.at_kw(Kw::Function) {
            return Some(ClassItem::Func {
                is_virtual,
                vis,
                def: self.parse_function_def().0,
            });
        }
        if self.at_kw(Kw::Task) {
            return Some(ClassItem::Task {
                is_virtual,
                vis,
                def: self.parse_task_def(),
            });
        }
        if is_virtual {
            self.error("`function` or `task` after `virtual` in a class body");
            let s = self.cur_span();
            self.skip_class_item_recover();
            return Some(ClassItem::Error(s));
        }
        // A `local`/`protected` qualifier combined with `rand`/`randc`/`constraint`
        // is outside this slice (access-controlled randomization) — loud, never a
        // silent drop of the visibility OR the rand-ness.
        if vis != Visibility::Public
            && (self.at_ident_kw("rand")
                || self.at_ident_kw("randc")
                || self.at_ident_kw("constraint"))
        {
            self.error(
                "a plain data member or method after `local`/`protected` \
                 (access-controlled rand/constraint members are outside this slice)",
            );
            let s = self.cur_span();
            self.skip_class_item_recover();
            return Some(ClassItem::Error(s));
        }
        // N7-REST: `rand`/`randc` data member — consume the qualifier, parse the
        // member declaration, and tag it for `randomize()`.
        let randc = self.at_ident_kw("randc");
        if randc || self.at_ident_kw("rand") {
            self.bump(); // the rand/randc qualifier (an Ident, not a lexer keyword)
            let decl = if self.net_var_kind().is_some() {
                self.parse_net_var(false) // class data member: no net delay
            } else if let Some(info) = self.peek_typedef_name() {
                self.parse_typed_decl(info)
            } else {
                self.error("a data member declaration after `rand`/`randc`");
                let s = self.cur_span();
                self.skip_class_item_recover();
                return Some(ClassItem::Error(s));
            };
            return Some(match decl {
                Some(d) => ClassItem::RandProperty { randc, decl: d },
                None => ClassItem::Error(self.prev_span()),
            });
        }
        // N7-REST: `constraint NAME { expr; … }` block.
        if self.at_ident_kw("constraint") {
            return self.parse_constraint();
        }
        // Loud-reject the remaining deferred member qualifiers so they never
        // silently parse as a net type name (N7 MVP: plain data members + methods;
        // `local`/`protected` are handled above, the rest stay deferred — and a
        // `local static x` combo is still loud here since `static` is caught).
        for kw in ["static", "const", "pure", "extern"] {
            if self.at_ident_kw(kw) {
                self.error(
                    "a plain data member or method (N7 MVP does not support \
                     rand/randc/static/const/constraint/pure/extern class members)",
                );
                let s = self.cur_span();
                self.skip_class_item_recover();
                return Some(ClassItem::Error(s));
            }
        }
        // Data member: a net/var declaration, a typedef-name, or a class-typed
        // handle (registered as a `NetVarKind::Class` alias in the prescan). The
        // leading `local`/`protected` access qualifier rides as `vis`.
        if self.net_var_kind().is_some() {
            return self
                .parse_net_var(false) // class property: no net delay
                .map(|d| ClassItem::Property(vis, d));
        }
        if let Some(info) = self.peek_typedef_name() {
            return self
                .parse_typed_decl(info)
                .map(|d| ClassItem::Property(vis, d));
        }
        self.error("class member (data member or `function`/`task` method)");
        let s = self.cur_span();
        self.skip_class_item_recover();
        Some(ClassItem::Error(s))
    }

    /// `constraint NAME { constraint_expr ; … }` (N7-REST). The `constraint`
    /// qualifier is the current token. Each body item is a boolean expression
    /// terminated by `;`; unsupported forms (`inside`/`dist`/`->`) parse-fail loud,
    /// and elaborate loud-rejects any expr it cannot fold to a per-field bound.
    fn parse_constraint(&mut self) -> Option<ClassItem> {
        let start = self.cur_span();
        self.bump(); // `constraint` (an Ident)
        let Some(name) = self.ident() else {
            self.error("a constraint name after `constraint`");
            let s = self.cur_span();
            self.skip_class_item_recover();
            return Some(ClassItem::Error(s));
        };
        self.expect(TokenKind::LBrace, "'{' to open a constraint block");
        let mut exprs = Vec::new();
        let mut soft = Vec::new();
        while self.peek() != Some(TokenKind::RBrace) && self.peek().is_some() {
            let before = self.pos;
            // optional `soft` qualifier (IEEE §18.5.14) before a constraint expr.
            let is_soft = self.eat_ident_kw("soft");
            let e = self.expr(0);
            exprs.push(e);
            soft.push(is_soft);
            self.expect(TokenKind::Semi, "';' after a constraint expression");
            // Guard against a non-advancing error loop.
            if self.pos == before {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "'}' to close a constraint block");
        Some(ClassItem::Constraint(ConstraintDecl {
            name,
            exprs,
            soft,
            span: start.to(self.prev_span()),
        }))
    }

    /// `obj.randomize() with { … }` postfix: consume `with`, parse the inline
    /// constraint block, and wrap the Call `lhs` into `ExprKind::RandomizeWith`.
    /// `#[inline(never)]` so its locals never inflate the recursive `expr_capped`
    /// frame (the expr-depth cap depends on a small hot frame).
    #[inline(never)]
    fn parse_randomize_with_postfix(&mut self, lhs: Expr) -> Expr {
        self.bump(); // `with`
        let constraints = self.parse_with_constraints();
        let span = lhs.span.to(self.prev_span());
        let (name, args) = match lhs.kind {
            ExprKind::Call { name, args } => (name, args),
            _ => unreachable!("caller gates on ExprKind::Call"),
        };
        Expr {
            kind: ExprKind::RandomizeWith(Box::new(RandomizeWithExpr {
                name,
                args,
                constraints,
            })),
            span,
        }
    }

    /// Dispatch a `with` postfix on a method call: brace ⇒ inline `randomize()
    /// with {…}` (§18.7), paren ⇒ array-method `with (expr)` iterator (§7.12).
    /// `#[inline(never)]` so the hot recursive `expr` frame stays small (the
    /// caller is guarded to brace/paren lookahead).
    #[inline(never)]
    fn parse_with_postfix(&mut self, lhs: Expr) -> Expr {
        if self.peek_at(1) == Some(TokenKind::LBrace) {
            self.parse_randomize_with_postfix(lhs)
        } else {
            self.parse_array_with_postfix(lhs)
        }
    }

    /// Parse `arr.method(opt_iter_var) with (expr)` (IEEE §7.12). `#[inline(never)]`
    /// so it does not enlarge the hot recursive `expr` frame (same depth-cap
    /// discipline as `parse_randomize_with_postfix`). The receiver method call has
    /// already been parsed into `lhs` (an `ExprKind::Call`); we split its path into
    /// receiver + method and capture the optional single bare-ident iterator var.
    #[inline(never)]
    fn parse_array_with_postfix(&mut self, lhs: Expr) -> Expr {
        self.bump(); // `with`
        self.expect(TokenKind::LParen, "'(' to open a `with` iterator clause");
        let with_expr = self.expr(0);
        self.expect(TokenKind::RParen, "')' to close a `with` iterator clause");
        let span = lhs.span.to(self.prev_span());
        let (mut path, args) = match lhs.kind {
            ExprKind::Call { name, args } => (name, args),
            _ => unreachable!("caller gates on ExprKind::Call"),
        };
        // method = last path segment; receiver = the rest.
        let method = path.segments.pop().unwrap_or(Ident {
            name: String::new(),
            span,
        });
        let recv = HierPath {
            span,
            segments: path.segments,
        };
        // A single bare-identifier method arg is the named iterator variable
        // (`find(x) with (x>2)`); anything else means the default `item`.
        let iter_var = match args.as_slice() {
            [Expr {
                kind: ExprKind::Ident(p),
                ..
            }] if p.segments.len() == 1 => Some(p.segments[0].clone()),
            _ => None,
        };
        Expr {
            kind: ExprKind::ArrayMethodWith(Box::new(ArrayMethodWithExpr {
                recv,
                method,
                iter_var,
                with_expr,
            })),
            span,
        }
    }

    /// Parse `{ (constraint_expr ;)* }` after `with` for inline `randomize() with`
    /// (IEEE §18.7). Mirrors the constraint-block body. `soft` inside an inline
    /// `with` is a v1 loud reject (the per-call sidecar carries hard predicates
    /// only — class-level `soft` is unaffected).
    fn parse_with_constraints(&mut self) -> Vec<Expr> {
        self.expect(
            TokenKind::LBrace,
            "'{' to open an inline `with` constraint block",
        );
        let mut exprs = Vec::new();
        while self.peek() != Some(TokenKind::RBrace) && self.peek().is_some() {
            let before = self.pos;
            if self.at_ident_kw("soft") {
                self.error(
                    "`soft` inside inline `randomize() with` is unsupported \
                     (v1: hard constraints only)",
                );
                self.bump();
            }
            let e = self.expr(0);
            exprs.push(e);
            self.expect(TokenKind::Semi, "';' after an inline constraint expression");
            if self.pos == before {
                self.bump();
            }
        }
        self.expect(
            TokenKind::RBrace,
            "'}' to close an inline `with` constraint block",
        );
        exprs
    }

    /// Recover from a malformed class item by skipping to the next `;` or
    /// `endclass` (consuming the `;`), without re-reporting.
    fn skip_class_item_recover(&mut self) {
        while !self.at_eof() && !self.at_kw(Kw::Endclass) {
            let semi = self.peek() == Some(TokenKind::Semi);
            self.bump();
            if semi {
                break;
            }
        }
    }

    /// Returns the parsed `FunctionDef` plus a `is_void` flag. A `function void`
    /// in module/package scope is task-equivalent (statement-called, output
    /// formals, control flow) — the module-item caller converts it to a `TaskDef`
    /// to reuse the full task machinery. Class methods ignore the flag (a void
    /// method is just a frame-function whose result is discarded at the call).
    fn parse_function_def(&mut self) -> (FunctionDef, bool) {
        let start = self.cur_span();
        self.bump(); // 'function'
        let automatic = self.eat_kw(Kw::Automatic);
        // N7/SV: a return-type KIND keyword (`logic`/`reg`/`bit`/`int`/`byte`/
        // `shortint`/`longint`) — `function int f` / `function logic [7:0] g`.
        // `integer`/`real`/`realtime`/`time` stay in `opt_param_type` below.
        // 2-state atoms imply a fixed signed range; `int` maps to the 32-bit
        // signed `Integer` return path (exact width/sign).
        let mut signed = false;
        let mut range = None;
        let mut ret_type = ParamType::Implicit;
        let mut ret_two_state = false;
        let is_void = self.eat_kw(Kw::Void);
        if is_void {
            // `function void f(...)`: no return value. In module/package scope the
            // caller converts to a TaskDef (task-equivalent); inside a class it is a
            // frame-function whose result is discarded. ret_type stays Implicit with
            // no range (the slot is never read). No AST shape change (IR-0).
        } else {
            let kw_kind = match self.peek() {
                Some(TokenKind::Word(WordKind::Keyword(
                    k @ (Kw::Logic
                    | Kw::Reg
                    | Kw::Bit
                    | Kw::Int
                    | Kw::Byte
                    | Kw::Shortint
                    | Kw::Longint),
                ))) => Some(k),
                _ => None,
            };
            if let Some(k) = kw_kind {
                self.bump(); // the kind keyword
                             // int/byte/shortint/longint/bit are 2-state integral return types.
                ret_two_state =
                    matches!(k, Kw::Int | Kw::Byte | Kw::Shortint | Kw::Longint | Kw::Bit);
                match k {
                    // `int` is 32-bit SIGNED 2-state (defaults signed).
                    Kw::Int => {
                        ret_type = ParamType::Integer;
                        signed = true;
                    }
                    Kw::Byte => {
                        range = Some(Self::dec_range(7));
                        signed = true;
                    }
                    Kw::Shortint => {
                        range = Some(Self::dec_range(15));
                        signed = true;
                    }
                    Kw::Longint => {
                        range = Some(Self::dec_range(63));
                        signed = true;
                    }
                    _ => {} // logic/reg/bit: width from an explicit range below
                }
                // An explicit trailing `unsigned` must override the atom default.
                if let Some(s) = self.opt_signed() {
                    signed = s;
                }
                if range.is_none() {
                    range = self.opt_range();
                }
            } else if let Some(info) = self.peek_block_typedef_decl() {
                // A user-defined type name as the return type: `function b_t f;`
                // (the `<typedef_name> <function_name>` shape — same disambiguation
                // as a block-local decl). Map the typedef's resolved type onto the
                // return fields, mirroring the built-in-keyword arm above.
                self.bump(); // the typedef name
                             // The function return-type fields carry one packed dimension
                             // (`range`); a multi-dim packed typedef (`typedef logic [3:0][7:0]
                             // m_t`) cannot be represented, so loud-reject rather than silently
                             // return only the first dimension's width (correct-or-loud).
                if !info.packed.is_empty() {
                    self.error("a multi-dimension packed type as a function return type");
                }
                signed = info.signed;
                range = info.range.clone();
                ret_two_state = matches!(
                    info.kind,
                    NetVarKind::Bit
                        | NetVarKind::Byte
                        | NetVarKind::Int
                        | NetVarKind::Shortint
                        | NetVarKind::Longint
                );
                // `int`/`integer` carry their 32-bit width via ParamType::Integer;
                // every other kind sizes from `range` (a named atom with no explicit
                // range gets its fixed atom width here).
                if matches!(info.kind, NetVarKind::Int | NetVarKind::Integer) {
                    ret_type = ParamType::Integer;
                } else if range.is_none() {
                    if let Some(w) = Self::atom_member_width(info.kind) {
                        range = Some(Self::dec_range(w - 1));
                    }
                }
            } else {
                // return-type signedness/range/type, V2005 order: [signed] [range] [type]
                let sign_kw = self.opt_signed();
                range = self.opt_range();
                ret_type = self.opt_param_type();
                // `integer` defaults SIGNED; an explicit qualifier wins.
                signed = sign_kw.unwrap_or(matches!(ret_type, ParamType::Integer));
            }
            // a second `signed` after an integer-ish return is tolerated.
            signed = signed || self.opt_signed().unwrap_or(false);
        }
        let name = self.ident().unwrap_or_else(|| Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        let mut ports = self.opt_tf_port_paren_list();
        self.expect(TokenKind::Semi, "';' after function header");
        let (body_decls, body) = self.tf_body(BlockEnd2::Endfunction, &mut ports);
        self.expect(
            TokenKind::Word(WordKind::Keyword(Kw::Endfunction)),
            "'endfunction'",
        );
        self.opt_block_label(); // optional `: name` after endfunction → discard
        (
            FunctionDef {
                automatic,
                signed,
                range,
                ret_type,
                ret_two_state,
                name,
                ports,
                body_decls,
                body: Box::new(body),
                span: start.to(self.prev_span()),
            },
            is_void,
        )
    }

    /// `task [automatic] name [(tf_ports)] ; {body_decl} body_stmt endtask`
    fn parse_task_def(&mut self) -> TaskDef {
        let start = self.cur_span();
        self.bump(); // 'task'
        let automatic = self.eat_kw(Kw::Automatic);
        let name = self.ident().unwrap_or_else(|| Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        let mut ports = self.opt_tf_port_paren_list();
        self.expect(TokenKind::Semi, "';' after task header");
        let (body_decls, body) = self.tf_body(BlockEnd2::Endtask, &mut ports);
        self.expect(TokenKind::Word(WordKind::Keyword(Kw::Endtask)), "'endtask'");
        self.opt_block_label();
        TaskDef {
            automatic,
            name,
            ports,
            body_decls,
            body: Box::new(body),
            span: start.to(self.prev_span()),
        }
    }

    /// Map an optional return/var type keyword to ParamType (V2005 set only).
    /// `reg`/`logic`/bit-vector returns are NOT a ParamType — they surface via
    /// signed+range with ret_type = Implicit, so those keywords are NOT consumed.
    fn opt_param_type(&mut self) -> ParamType {
        match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::Integer))) => {
                self.bump();
                ParamType::Integer
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Real))) => {
                self.bump();
                ParamType::Real
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Realtime))) => {
                self.bump();
                ParamType::Realtime
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Time))) => {
                self.bump();
                ParamType::Time
            }
            _ => ParamType::Implicit,
        }
    }

    /// Optional ANSI tf-port list `( tf_port {, tf_port} )`. Returns `[]` if there
    /// is no `(` (non-ANSI form — ports come from body input/output decls instead).
    /// Empty `()` ⇒ `[]`. Direction AND type are sticky across comma-grouped
    /// names (a bare `, name` inherits both — see `parse_tf_port`).
    fn opt_tf_port_paren_list(&mut self) -> Vec<TfPort> {
        let mut ports = Vec::new();
        if self.peek() != Some(TokenKind::LParen) {
            return ports;
        }
        self.bump(); // '('
        if self.peek() == Some(TokenKind::RParen) {
            self.bump();
            return ports;
        }
        let mut inherited = PortDir::Input;
        let mut inherited_type: (Option<NetVarKind>, bool, Option<Range>) = (None, false, None);
        loop {
            let before = self.pos;
            let (port, dir, ty) = self.parse_tf_port(inherited, &inherited_type);
            inherited = dir;
            inherited_type = ty;
            ports.push(port);
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "')' closing tf-port list");
        ports
    }

    /// A typedef name as a MODULE port type (`input mode_e m`, `input byte_t a`) →
    /// its `(kind, signed, range)`, mirroring `try_tf_port_typedef` for tf-ports
    /// (typedef-recognition family). REQUIRES the next token to be the port NAME
    /// (an Ident) so a bare continuation (`input byte_t a, b` — `b` is a name, not a
    /// type) is NOT misresolved as a type. A struct/union/class/multi-dim-packed
    /// typedef port type is honest-loud (v1 — the AnsiPort/PortDecl shape carries
    /// only kind/signed/range, like a tf-port). Used by both the ANSI
    /// (`parse_ansi_port`) and non-ANSI (`parse_port_decl`) port parsers.
    fn try_port_typedef(&mut self) -> Option<(NetVarKind, bool, Option<Range>)> {
        let info = self.peek_typedef_name()?;
        if !matches!(
            self.peek_at(1),
            Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
        ) {
            return None; // not `<type> <name>` — leave for the continuation/name path
        }
        let nm = self.cur_text().to_string();
        if self.struct_layouts.contains_key(&nm)
            || info.class_name.is_some()
            || !info.packed.is_empty()
        {
            self.error(
                "a struct/union/class/multi-dim-packed typedef as a module port type is unsupported in v1 (a simple vector/enum typedef port is supported)",
            );
        }
        self.bump(); // the typedef-name token
        Some((info.kind, info.signed, info.range))
    }

    /// One ANSI tf-port: `[input|output|inout] [net_or_var] [signed] [range] name`.
    /// Returns the port, the (possibly-inherited) direction, and the
    /// (possibly-inherited) type, so a following bare `, name` keeps both.
    ///
    /// §13.3/§23.2.2.3 type stickiness: a bare `, name` (no direction keyword AND
    /// no type spec) inherits the previous port's full type. A direction keyword
    /// resets the type (`input y` after `input logic [7:0] x` makes `y` the default
    /// 1-bit), and any explicit type (net/var keyword, `signed`/`unsigned`, or a
    /// range) starts a fresh type that itself propagates onward.
    /// If the cursor is on a user-defined type name usable as a tf-port type
    /// (`task t(byte_t a)` / `input byte_t a;`), consume it and return its
    /// `(kind, signed, range)`. A struct / union / class / multi-dim-packed
    /// typedef port needs per-port layout or method binding not in v1 —
    /// honest-loud (the diagnostic is emitted, but the name is still consumed so
    /// it does not cascade as the port name). Returns `None` when the cursor is
    /// not on a typedef name, so the caller keeps its built-in / inherited-type
    /// handling. Shared by the ANSI (`parse_tf_port`) and non-ANSI
    /// (`parse_tf_port_decl_into`) port parsers.
    fn try_tf_port_typedef(&mut self) -> Option<(NetVarKind, bool, Option<Range>)> {
        let info = self.peek_typedef_name()?;
        let nm = self.cur_text().to_string();
        if self.struct_layouts.contains_key(&nm)
            || info.class_name.is_some()
            || !info.packed.is_empty()
        {
            self.error(
                "a simple (non-struct) typedef type for a tf-port (a struct/union/class/multi-dim packed port type is unsupported in v1)",
            );
        }
        self.bump(); // the typedef-name token
        Some((info.kind, info.signed, info.range))
    }

    fn parse_tf_port(
        &mut self,
        inherited: PortDir,
        inherited_type: &(Option<NetVarKind>, bool, Option<Range>),
    ) -> (TfPort, PortDir, (Option<NetVarKind>, bool, Option<Range>)) {
        let start = self.cur_span();
        let (dir, dir_present) = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::Input))) => {
                self.bump();
                (PortDir::Input, true)
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Output))) => {
                self.bump();
                (PortDir::Output, true)
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Inout))) => {
                self.bump();
                (PortDir::Inout, true)
            }
            _ => (inherited, false), // bare `, b` continues the previous direction
        };
        let mut net_or_var = self.net_var_kind();
        if net_or_var.is_some() {
            self.bump();
        }
        let explicit_signed = self.opt_signed();
        let mut range = self.opt_range();
        // A tf-port type given as a user-defined type name (`task t(byte_t a)`).
        // Resolve a SIMPLE typedef (vector / enum / atom) to its kind/sign/range,
        // exactly as a built-in keyword type would (struct/union/class/multi-dim =
        // honest-loud, handled inside the helper).
        let mut typedef_signed: Option<bool> = None;
        if net_or_var.is_none() && range.is_none() {
            if let Some((k, s, r)) = self.try_tf_port_typedef() {
                net_or_var = Some(k);
                range = r;
                typedef_signed = Some(s);
            }
        }
        // A port carries its own type when a direction keyword OR any explicit type
        // token is present; otherwise (a bare `, name`) it inherits the previous
        // type. The resolved type then propagates to the next bare port.
        let type_present = net_or_var.is_some() || range.is_some() || explicit_signed.is_some();
        let (net_or_var, signed, range) = if dir_present || type_present {
            (
                net_or_var,
                explicit_signed
                    .or(typedef_signed)
                    .unwrap_or_else(|| atom_default_signed(net_or_var)),
                range,
            )
        } else {
            inherited_type.clone()
        };
        let name = self.ident().unwrap_or_else(|| Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        // IEEE §13.5.3: an ANSI tf-port may carry a default argument value
        // (`int b = 10`), used when a call omits the trailing actual.
        let default = if self.eat(TokenKind::Eq) {
            Some(self.expr(0))
        } else {
            None
        };
        let port = TfPort {
            dir,
            net_or_var,
            signed,
            range,
            name,
            default,
            span: start.to(self.prev_span()),
        };
        let next_type = (port.net_or_var, port.signed, port.range.clone());
        (port, dir, next_type)
    }

    /// Body of a function/task: a decl prefix (net/var decls AND — for the non-ANSI
    /// form — input/output/inout formal decls, hoisted into `ports`), then exactly
    /// ONE body statement (usually a `begin … end`), up to the endfunction/endtask
    /// closer. `ports` is appended to for non-ANSI formals.
    fn tf_body(&mut self, end: BlockEnd2, ports: &mut Vec<TfPort>) -> (Vec<NetVarDecl>, Stmt) {
        let mut body_decls = Vec::new();
        // Body-local typedef DEFINITIONs are lexically scoped (see `block_body`):
        // snapshot at the first one, restore when the body ends.
        let mut typedef_scope: Option<ScopeSnapshot> = None;
        while !self.at_eof() && !self.at_tf_end(end) {
            if matches!(
                self.peek(),
                Some(TokenKind::Word(WordKind::Keyword(
                    Kw::Input | Kw::Output | Kw::Inout
                )))
            ) {
                // non-ANSI formal: `input [7:0] a, b;` → one TfPort per name.
                let before = self.pos;
                self.parse_tf_port_decl_into(ports);
                if self.pos == before {
                    self.bump();
                }
                continue;
            }
            // A body-local typedef DEFINITION (`typedef logic [3:0] t;`).
            if self.at_kw(Kw::Typedef) {
                if typedef_scope.is_none() {
                    typedef_scope = Some(self.snapshot_scope());
                }
                let before = self.pos;
                self.parse_body_typedef_def();
                if self.pos == before {
                    self.bump();
                }
                continue;
            }
            // B4: a per-decl lifetime override `automatic <kind> <name>;` (only
            // `automatic` — `static` is not a reserved word). The keyword precedes
            // a normal var decl; consume it and stamp the lifetime on the decl.
            if self.at_kw(Kw::Automatic)
                && matches!(
                    self.peek_at(1),
                    Some(TokenKind::Word(WordKind::Keyword(
                        Kw::Reg | Kw::Logic | Kw::Integer | Kw::Real | Kw::Realtime | Kw::Time
                    )))
                )
            {
                self.bump(); // 'automatic'
                let before = self.pos;
                if let Some(mut d) = self.parse_net_var(false) {
                    // function/task body decl: no net delay
                    d.lifetime = Some(true);
                    body_decls.push(d);
                }
                if self.pos == before {
                    self.bump();
                }
                continue;
            }
            if self.net_var_kind().is_some() {
                let before = self.pos;
                if let Some(d) = self.parse_net_var(false) {
                    // function/task body decl: no net delay
                    body_decls.push(d);
                }
                if self.pos == before {
                    self.bump();
                }
                continue;
            }
            // A function/task body decl using a user-defined type name
            // (`my_enum_t s; byte_t b;`), same `<typedef_name> <ident>` shape as a
            // block-local decl (see `block_body`). This writes the VAR-name-keyed
            // maps, so — exactly as in `block_body` — it triggers the scope
            // snapshot: a function-local struct var (`inner_s x;`) whose name
            // shadows an outer one must not leak its layout binding past the body
            // (a function need NOT do a struct field assign to clobber it, so the
            // frame-call-subset E3009 does not cover this).
            if let Some(info) = self.peek_block_typedef_decl() {
                if typedef_scope.is_none() {
                    typedef_scope = Some(self.snapshot_scope());
                }
                let before = self.pos;
                if let Some(d) = self.parse_typed_decl(info) {
                    body_decls.push(d);
                }
                if self.pos == before {
                    self.bump();
                }
                continue;
            }
            break; // first non-decl token starts the body statement
        }
        let body = if self.at_tf_end(end) {
            Stmt::Null(self.cur_span()) // empty body: `function f; endfunction`
        } else {
            // SV: a function/task body may hold MULTIPLE statements with no
            // explicit `begin`/`end` (`function f; a=1; b=2; endfunction`). Collect
            // them all (until the closer) and wrap in an implicit sequential block.
            // A SINGLE statement is returned bare — byte-identical to the V2005
            // one-statement form, so every existing design is unaffected.
            let start = self.cur_span();
            let mut stmts = Vec::new();
            while !self.at_eof() && !self.at_tf_end(end) {
                let before = self.pos;
                stmts.push(self.parse_statement());
                if self.pos == before {
                    self.bump(); // guarantee forward progress
                }
            }
            if stmts.len() == 1 {
                stmts.pop().unwrap()
            } else {
                Stmt::Block {
                    label: None,
                    decls: Vec::new(),
                    stmts,
                    span: start.to(self.prev_span()),
                }
            }
        };
        // Drop body-local typedefs (restore outer scope) after the body is parsed.
        if let Some(scope) = typedef_scope {
            self.restore_scope(scope);
        }
        (body_decls, body)
    }

    /// True at the `endfunction`/`endtask` closer.
    fn at_tf_end(&self, end: BlockEnd2) -> bool {
        match end {
            BlockEnd2::Endfunction => self.at_kw(Kw::Endfunction),
            BlockEnd2::Endtask => self.at_kw(Kw::Endtask),
        }
    }

    /// Non-ANSI formal decl `input [r] a, b;` → one TfPort per name, appended.
    fn parse_tf_port_decl_into(&mut self, ports: &mut Vec<TfPort>) {
        let dir = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::Output))) => {
                self.bump();
                PortDir::Output
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Inout))) => {
                self.bump();
                PortDir::Inout
            }
            _ => {
                self.bump();
                PortDir::Input
            }
        };
        let mut net_or_var = self.net_var_kind();
        if net_or_var.is_some() {
            self.bump();
        }
        let mut signed = self.signed_eff(net_or_var);
        let mut range = self.opt_range();
        // A non-ANSI tf-port type given as a user-defined type name
        // (`input byte_t a;`) — resolve a SIMPLE typedef exactly as the ANSI path.
        if net_or_var.is_none() && range.is_none() {
            if let Some((k, s, r)) = self.try_tf_port_typedef() {
                net_or_var = Some(k);
                signed = s;
                range = r;
            }
        }
        loop {
            let n_start = self.cur_span();
            let Some(name) = self.ident() else { break };
            ports.push(TfPort {
                dir,
                net_or_var,
                signed,
                range: range.clone(),
                name,
                default: None, // non-ANSI formals have no default (ANSI-only, §13.5.3)
                span: n_start.to(self.prev_span()),
            });
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semi, "';' after tf-port declaration");
    }

    /// `@*` | `@(*)` → Star ;  `@(ev or ev , …)` → List ;  bare `@e` / `@clk`
    /// → single NO-EDGE event (IEEE 1364 `@ hierarchical_identifier`).  Consumes
    /// the leading `@`. The bare reference is parsed identically to the paren form,
    /// so whatever `@(X)` does, `@X` does too: a whole signal/event simulates, while
    /// a form whose feature is unsupported (single-bit level `@a[2]`, hierarchical
    /// `@u.s`, a call `@f(x)`) routes to the SAME loud reject as `@(…)` at elaborate.
    fn parse_sensitivity(&mut self) -> Sensitivity {
        self.bump(); // '@'
        if self.eat(TokenKind::Star) {
            return Sensitivity::Star; // `@*`
        }
        // Bare, paren-free event control `@ hierarchical_event_identifier` — a
        // single NO-EDGE reference (`@e`, `@clk`, `@u.s`, `@a[2]`), equivalent to
        // `@(e)`. Parse a primary+postfix REFERENCE (`expr_postfix`, not a full
        // `expr(0)`): a bare binary form `@a+b` stops after `a`, and the trailing
        // `+b` is a loud statement error — matching iverilog, which rejects
        // `@a+b`/`@a && b`. A bare edge `@posedge clk` is also illegal (parens
        // required); `posedge`/`negedge` are keywords (not idents), so they fall
        // through to the `'(' or '*'` error below.
        if self.is_ident() {
            let start = self.cur_span();
            let expr = self.expr_postfix();
            let span = start.to(self.prev_span());
            return Sensitivity::List(vec![EventExpr {
                edge: Edge::NoEdge,
                expr,
                span,
            }]);
        }
        if !self.expect(TokenKind::LParen, "'(' or '*' after '@'") {
            return Sensitivity::List(Vec::new()); // recover; only `@` consumed
        }
        if self.peek() == Some(TokenKind::Star) {
            self.bump(); // `@(*)`
            self.expect(TokenKind::RParen, "')'");
            return Sensitivity::Star;
        }
        let mut events = Vec::new();
        if self.peek() == Some(TokenKind::RParen) {
            self.error("event expression"); // m2: `@()` is illegal — diagnose
        } else {
            loop {
                let before = self.pos;
                events.push(self.parse_event_expr());
                let sep = self.eat_kw(Kw::Or) || self.eat(TokenKind::Comma);
                // forward-progress guard MUST stay AFTER the separator-eat
                if self.pos == before {
                    self.bump();
                }
                if !sep || self.peek() == Some(TokenKind::RParen) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "')'");
        Sensitivity::List(events)
    }

    /// `[posedge|negedge] expr` → EventExpr.
    fn parse_event_expr(&mut self) -> EventExpr {
        let start = self.cur_span();
        let edge = if self.eat_kw(Kw::Posedge) {
            Edge::Posedge
        } else if self.eat_kw(Kw::Negedge) {
            Edge::Negedge
        } else {
            Edge::NoEdge
        };
        let expr = self.expr(0);
        let span = start.to(expr.span);
        EventExpr { edge, expr, span }
    }

    // ─────────────────────── 2. statement dispatcher ───────────────────────
    /// STMT-DEPTH guard: cap statement-recursion so pathological `begin begin …`
    /// nesting is a clean parse error, never a SIGABRT. 256 is ≫ any real RTL
    /// (deepest practical nesting is <30) and the deepest frame reached at the
    /// cap (≈3 frames/level: parse_statement → parse_seq_block → block_body)
    /// fits a 2 MiB test-thread stack even in debug — 1024 overflowed it. The
    /// cap path consumes no token, but the `block_body` loop's `pos == before`
    /// guard bumps one, so recovery always makes progress (no spin).
    const MAX_STMT_DEPTH: u32 = 256;

    fn parse_statement(&mut self) -> Stmt {
        self.stmt_depth += 1;
        if self.stmt_depth > Self::MAX_STMT_DEPTH {
            self.stmt_depth -= 1;
            let s = self.cur_span();
            self.error("statement nesting too deep (cap 256)");
            return Stmt::Error(s);
        }
        let r = self.parse_statement_inner();
        self.stmt_depth -= 1;
        r
    }

    fn parse_statement_inner(&mut self) -> Stmt {
        use TokenKind as T;
        if self.at_lex_error() {
            let s = self.cur_span();
            self.bump(); // skip the lexer-error sentinel without re-reporting
            return Stmt::Error(s);
        }
        match self.peek() {
            Some(T::Semi) => {
                let s = self.cur_span();
                self.bump();
                Stmt::Null(s)
            }
            Some(T::Hash) => self.parse_delay_stmt(),
            Some(T::At) => self.parse_event_stmt(),
            Some(T::Arrow) => self.parse_trigger_stmt(),
            Some(T::LBrace) => self.parse_assign_or_call(), // {a,b} = … concat lvalue
            // SV §11.4.2 prefix `++i;` / `--i;` (statement form). As a statement the
            // pre/post distinction is invisible → `i = i ± 1`.
            Some(T::PlusPlus) | Some(T::MinusMinus) => {
                let start = self.cur_span();
                let s = self.parse_pre_incdec(start);
                self.expect(TokenKind::Semi, "';'");
                s
            }
            Some(T::Word(WordKind::Keyword(kw))) => match kw {
                Kw::Begin => self.parse_seq_block(),
                Kw::Fork => self.parse_par_block(),
                Kw::If => self.parse_if(),
                Kw::Case => self.parse_case(CaseKind::Case),
                Kw::Casez => self.parse_case(CaseKind::Casez),
                Kw::Casex => self.parse_case(CaseKind::Casex),
                Kw::For => self.parse_for(),
                Kw::While => self.parse_while(),
                // P2-E: `do body while (cond);` — parse-time desugar (no new
                // AST node): { body; while (cond) body }.
                Kw::Do => self.parse_do_while(),
                // P2-E: unique/priority QUALIFIERS on if/case — the violation
                // check desugars to a synthesized `$warning` arm (IEEE
                // §12.4/12.5: a no-match is a runtime violation warning).
                Kw::Unique | Kw::Priority | Kw::Unique0 | Kw::Priority0 => {
                    self.parse_unique_priority()
                }
                Kw::Foreach => self.parse_foreach(),
                Kw::Repeat => self.parse_repeat(),
                Kw::Forever => self.parse_forever(),
                Kw::Wait => self.parse_wait(),
                Kw::Disable => self.parse_disable(),
                Kw::Assign => self.parse_proc_assign(),
                Kw::Deassign => self.parse_deassign(),
                Kw::Force => self.parse_force(),
                Kw::Release => self.parse_release(),
                // SVA-REST: `assume` parses like `assert` (sim-checked the same).
                Kw::Assert | Kw::Assume => self.parse_assert(),
                _ => self.stmt_error(),
            },
            Some(T::SystemTask) => self.parse_systask_call(),
            // N7: SV `return [expr];` — contextual (not a V2005 reserved word), so
            // a net literally named `return` in legacy code still parses as an
            // assign/call (the `return EXPR;` / `return;` shape is unambiguous in
            // statement position: a V2005 program has no `return` statement).
            _ if self.at_ident_kw("return") => self.parse_return(),
            // SVA-REST: `cover property(@(clk) seq);` — `cover` is contextual (an SV
            // reserved word, never a legal net name) and recognized only when
            // immediately followed by `property`.
            _ if self.at_ident_kw("cover")
                && self.peek_at(1) == Some(TokenKind::Word(WordKind::Keyword(Kw::Property))) =>
            {
                self.parse_cover_property()
            }
            // SV §11.5 `break;` / `continue;` — contextual (not V2005 reserved), so
            // a net literally named `break`/`continue` used as `break = x;` still
            // parses as an assign. Recognized ONLY in the `break;`/`continue;`
            // statement shape (immediately followed by `;`).
            _ if self.at_ident_kw("break") && self.peek_at(1) == Some(TokenKind::Semi) => {
                self.parse_break_continue(true)
            }
            _ if self.at_ident_kw("continue") && self.peek_at(1) == Some(TokenKind::Semi) => {
                self.parse_break_continue(false)
            }
            _ if self.is_ident() => self.parse_assign_or_call(),
            _ => self.stmt_error(),
        }
    }

    /// `return [expr] ;` (N7).
    fn parse_return(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // 'return'
        let value = if self.peek() == Some(TokenKind::Semi) {
            None
        } else {
            Some(self.expr(0))
        };
        self.expect(TokenKind::Semi, "';' after return");
        Stmt::Return {
            value,
            span: start.to(self.prev_span()),
        }
    }

    /// Unparseable statement: record one error, build Error, sync, GUARANTEE ≥1
    /// token consumed.
    fn stmt_error(&mut self) -> Stmt {
        let s = self.cur_span();
        let before = self.pos;
        self.error("statement");
        self.synchronize();
        if self.pos == before {
            self.bump(); // forced progress when sync stopped immediately
        }
        Stmt::Error(s)
    }

    /// On a recovery path where `synchronize` may stop immediately: sync then
    /// force ≥1 token. Returns an `Error` spanning from `start`.
    fn stmt_error_at(&mut self, start: Span) -> Stmt {
        let before = self.pos;
        self.synchronize();
        if self.pos == before {
            self.bump();
        }
        Stmt::Error(start.to(self.prev_span()))
    }

    // ─────────────────────── 3. assignments / task calls ───────────────────────
    /// Leading ident or `{`: blocking `=`, nonblocking `<=`, or a user-task call.
    fn parse_assign_or_call(&mut self) -> Stmt {
        let start = self.cur_span();
        let lhs = self.parse_lvalue();
        // SV §11.4.1/§11.4.2 statement form: `lvalue += e;` / `lvalue++;` desugar to
        // a blocking `lvalue = lvalue <op> …`. An expression-embedded `a = i++` is
        // NOT handled (the expr parser has no `++`) → stays a loud parse error.
        if let Some(stmt) = self.try_compound_assign(&lhs, start) {
            self.expect(TokenKind::Semi, "';'");
            return stmt;
        }
        match self.peek() {
            Some(TokenKind::Eq) => {
                self.bump();
                let (delay, event) = self.parse_intra_assign_timing(true);
                let rhs = self.expr(0);
                let rhs = self.maybe_struct_pattern_rhs(&lhs, rhs);
                self.expect(TokenKind::Semi, "';'");
                Stmt::Blocking {
                    lhs,
                    delay,
                    event,
                    rhs,
                    span: start.to(self.prev_span()),
                }
            }
            Some(TokenKind::LtEq) => {
                self.bump();
                let (delay, event) = self.parse_intra_assign_timing(false);
                let rhs = self.expr(0);
                let rhs = self.maybe_struct_pattern_rhs(&lhs, rhs);
                self.expect(TokenKind::Semi, "';'");
                Stmt::NonBlocking {
                    lhs,
                    delay,
                    event,
                    rhs,
                    span: start.to(self.prev_span()),
                }
            }
            // user-task call: bare HierPath followed by `(` or `;`
            Some(TokenKind::LParen) | Some(TokenKind::Semi) => {
                if let Lvalue::Ident(path) = lhs {
                    let args = if self.peek() == Some(TokenKind::LParen) {
                        self.call_args()
                    } else {
                        Vec::new()
                    };
                    // `obj.randomize() with { … };` as a void statement (§18.7).
                    if self.at_ident_kw("with") && self.peek_at(1) == Some(TokenKind::LBrace) {
                        self.bump(); // `with`
                        let constraints = self.parse_with_constraints();
                        self.expect(TokenKind::Semi, "';'");
                        return Stmt::RandomizeWith {
                            name: path,
                            args,
                            constraints,
                            span: start.to(self.prev_span()),
                        };
                    }
                    self.expect(TokenKind::Semi, "';'");
                    Stmt::UserTaskCall {
                        name: path,
                        args,
                        span: start.to(self.prev_span()),
                    }
                } else {
                    // e.g. `a[i](…)` — an indexed lvalue cannot be a call.
                    self.error("'=' or '<=' after lvalue");
                    self.stmt_error_at(start)
                }
            }
            _ => {
                self.error("'=' or '<=' after lvalue");
                self.stmt_error_at(start)
            }
        }
    }

    /// Intra-assignment timing control after `=`/`<=` (IEEE 1800 §9.4.5): a `#d`
    /// delay (CAPTURED into `delay`), an `@(ev)` event control, or `repeat(n) @(ev)`
    /// (both CAPTURED into `event`). The elaborator lowers the event form as
    /// capture-now/wait/write for a blocking `=` (process blocks), and as a
    /// capture-now/`fork … join_none` desugar for a non-blocking `<=` (slice N1 —
    /// the process does not block). The `blocking` flag is retained for symmetry and
    /// future per-form diagnostics; both forms capture identically here.
    fn parse_intra_assign_timing(
        &mut self,
        _blocking: bool,
    ) -> (Option<Delay>, Option<IntraEvent>) {
        match self.peek() {
            Some(TokenKind::Hash) => (self.parse_delay(), None),
            Some(TokenKind::At) => {
                let ctrl = self.parse_sensitivity(); // consumes `@(…)`
                (None, Some(IntraEvent { repeat: None, ctrl }))
            }
            _ if self.at_kw(Kw::Repeat) => {
                self.bump(); // repeat
                self.expect(TokenKind::LParen, "'(' after 'repeat'");
                let count = self.expr(0);
                self.expect(TokenKind::RParen, "')'");
                if self.peek() == Some(TokenKind::At) {
                    let ctrl = self.parse_sensitivity();
                    (
                        None,
                        Some(IntraEvent {
                            repeat: Some(count),
                            ctrl,
                        }),
                    )
                } else {
                    self.error("`@(event)` after `repeat(n)` in an intra-assignment control");
                    (None, None)
                }
            }
            _ => (None, None),
        }
    }

    fn parse_systask_call(&mut self) -> Stmt {
        let start = self.cur_span();
        let t = self.bump().unwrap(); // SystemTask; lexeme retains `$`
        let name = Ident {
            name: self.src[t.span.clone()].to_string(),
            span: Self::sp(&t.span),
        };
        let args = if self.peek() == Some(TokenKind::LParen) {
            self.call_args()
        } else {
            Vec::new()
        };
        self.expect(TokenKind::Semi, "';'");
        Stmt::SysTaskCall {
            name,
            args,
            span: start.to(self.prev_span()),
        }
    }

    // procedural-continuous family — all reuse parse_lvalue
    fn parse_proc_assign(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // assign
        let lhs = self.parse_lvalue();
        self.expect(TokenKind::Eq, "'=' in procedural assign");
        let rhs = self.expr(0);
        let rhs = self.maybe_struct_pattern_rhs(&lhs, rhs);
        self.expect(TokenKind::Semi, "';'");
        Stmt::Assign {
            lhs,
            rhs,
            span: start.to(self.prev_span()),
        }
    }
    fn parse_force(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // force
        let lhs = self.parse_lvalue();
        self.expect(TokenKind::Eq, "'=' in force");
        let rhs = self.expr(0);
        let rhs = self.maybe_struct_pattern_rhs(&lhs, rhs);
        self.expect(TokenKind::Semi, "';'");
        Stmt::Force {
            lhs,
            rhs,
            span: start.to(self.prev_span()),
        }
    }
    fn parse_deassign(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // deassign
        let lhs = self.parse_lvalue();
        self.expect(TokenKind::Semi, "';'");
        Stmt::Deassign {
            lhs,
            span: start.to(self.prev_span()),
        }
    }
    fn parse_release(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // release
        let lhs = self.parse_lvalue();
        self.expect(TokenKind::Semi, "';'");
        Stmt::Release {
            lhs,
            span: start.to(self.prev_span()),
        }
    }

    /// SV immediate assertion (IEEE 1800 §16.3):
    ///   `assert [final] (expr) [pass_stmt] [else fail_stmt]`
    /// Desugared AT PARSE TIME to `Stmt::If` — the AST `Stmt` variant set is
    /// frozen (verdict M7), and `if` already has the exact assert condition
    /// semantics (0/X/Z cond → else branch = assertion failure). A missing
    /// else clause synthesizes the IEEE default failure action
    /// `$error("Assertion failed")`, which lowers through the severity table
    /// (stderr diagnostic + nonzero exit; run continues).
    ///
    /// DEFERRED immediate assertions (§16.4): `assert #0 (expr)` (Observed
    /// deferred) and `assert final (expr)` (Reactive deferred) are evaluated WHEN
    /// REACHED but their action MATURES in a later scheduling region with
    /// flush-on-re-reach. These parse to `Stmt::DeferredAssert` (carrying the
    /// region); elaborate emits a per-assertion flush marker + records the action
    /// StmtIds in the deferred sidecars, and the engine adds genuine Observed/
    /// Reactive maturation queues. iverilog rejects deferred assertions, so there
    /// is no oracle (hand-IEEE). A non-zero `#<n>` delay on an assert is NOT a
    /// deferred assertion → loud. Concurrent (`assert property`) is handled
    /// separately. Dangling-else: in `assert (c) if (x) a; else b;` the else binds
    /// to the inner if and the assert gets the synthesized default.
    fn parse_assert(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // `assert`
                     // v8 SVA subset: `assert property(@(clk) a |-> b);`
        if self.at_kw(Kw::Property) {
            return self.parse_concurrent_assert(start);
        }
        // Deferred immediate assertion (IEEE 1800-2017 §16.4): `assert #0` is the
        // Observed-deferred form, `assert final` the Reactive-deferred form. Both
        // sample the condition WHEN REACHED but MATURE the pass/fail action in a
        // later scheduling region with flush-on-re-reach (see Stmt::DeferredAssert
        // + the engine's Observed/Reactive maturation queues). A plain `assert`
        // (no `#0`/`final`) stays the immediate `Stmt::If` desugar below.
        let defer: Option<AssertDefer> = if self.peek() == Some(TokenKind::Hash) {
            self.bump(); // `#`
                         // Only `#0` is the Observed deferred form (§16.4). A non-zero delay on
                         // an assert is not a deferred assertion → loud.
            if matches!(self.peek(), Some(TokenKind::IntDecimal)) && self.cur_text() == "0" {
                self.bump(); // `0`
                Some(AssertDefer::Observed)
            } else {
                self.error(
                    "a deferred-assertion delay must be `#0` (the Observed deferred form); \
                     a non-zero `#` delay on an assertion is unsupported",
                );
                return self.stmt_error_at(start);
            }
        } else if self.eat_kw(Kw::Final) {
            Some(AssertDefer::Reactive)
        } else {
            None
        };
        if self.peek() != Some(TokenKind::LParen) {
            self.error("'(' after 'assert'");
            return self.stmt_error_at(start);
        }
        self.bump(); // `(`
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        // action_block ::= statement_or_null | [statement] `else` statement
        let then_s = if self.at_kw(Kw::Else) {
            Box::new(Stmt::Null(start)) // else-only form: no pass action
        } else {
            Box::new(self.parse_statement())
        };
        let else_s = if self.eat_kw(Kw::Else) {
            Box::new(self.parse_statement())
        } else {
            let sp = start.to(self.prev_span());
            Box::new(Stmt::SysTaskCall {
                name: Ident {
                    name: "$error".to_string(),
                    span: sp,
                },
                args: vec![Expr {
                    kind: ExprKind::StrLit {
                        raw: "\"Assertion failed\"".to_string(),
                    },
                    span: sp,
                }],
                span: sp,
            })
        };
        let span = start.to(self.prev_span());
        match defer {
            // Deferred (#0 / final): preserve the region so elaborate emits the
            // flush marker + records the action StmtIds in the deferred sidecars.
            Some(region) => Stmt::DeferredAssert {
                region,
                cond,
                then_s,
                else_s,
                span,
            },
            // Plain immediate assert: the byte-identical `Stmt::If` desugar.
            None => Stmt::If {
                cond,
                then_s,
                else_s: Some(else_s),
                span,
            },
        }
    }

    /// SVA subset (Phase-3): `assert property(@(posedge clk) seq |-> consequent);`
    /// (overlapping `|->` / non-overlapping `|=>`). Single clock. The antecedent
    /// is a `Sequence` — slice S4 added bounded `##n` cycle-delay and `[*n]`
    /// consecutive repetition (ranges/unbounded/goto/throughout/within stay a
    /// loud parse error). The consequent stays a flat boolean. The failure
    /// action is the implicit `$error` synthesized at elaborate time.
    fn parse_concurrent_assert(&mut self, start: Span) -> Stmt {
        self.bump(); // `property`
        self.expect(TokenKind::LParen, "'(' after 'property'");
        // Named-property INSTANCE: `assert property(NAME);` — NAME is a property
        // declared elsewhere, resolved + inlined at elaborate. Detect by a single
        // identifier immediately followed by `)`. A `NAME(args)` form is the
        // parameterized instance, reserved + loud in this subset.
        if self.is_ident() && self.peek_at(1) == Some(TokenKind::RParen) {
            let name = self.ident().unwrap();
            self.expect(TokenKind::RParen, "')'");
            let (pass, fail) = self.parse_assert_action_block();
            return Stmt::ConcurrentAssert {
                // empty clock = "named-property reference"; elaborate splices the
                // declared property's real clock/spec in at collect time.
                clock: Sensitivity::List(Vec::new()),
                disable_iff: None,
                antecedent: Sequence::Instance {
                    name,
                    args: Vec::new(),
                    span: start,
                },
                implication_kind: ImplicationKind::Overlap,
                consequent: Sequence::Boolean(Self::sva_true_lit(start)),
                consequent_clock: None,
                pass,
                fail,
                prop_expr: None,
                local_vars: Vec::new(),
                span: start.to(self.prev_span()),
            };
        }
        if self.is_ident() && self.peek_at(1) == Some(TokenKind::LParen) {
            // `assert property(NAME(args))` — parameterized property instance
            // (slice A1). Parse the positional actual arguments; elaborate binds them
            // to the declared property's formals and substitutes before splicing.
            let name = self.ident().unwrap();
            self.expect(TokenKind::LParen, "'(' before property arguments");
            let mut args = Vec::new();
            if self.peek() != Some(TokenKind::RParen) {
                loop {
                    args.push(self.expr(0));
                    if self.eat(TokenKind::Comma) {
                        continue;
                    }
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')' after property arguments");
            self.expect(TokenKind::RParen, "')'");
            let (pass, fail) = self.parse_assert_action_block();
            return Stmt::ConcurrentAssert {
                clock: Sensitivity::List(Vec::new()),
                disable_iff: None,
                antecedent: Sequence::Instance {
                    name,
                    args,
                    span: start,
                },
                implication_kind: ImplicationKind::Overlap,
                consequent: Sequence::Boolean(Self::sva_true_lit(start)),
                consequent_clock: None,
                pass,
                fail,
                prop_expr: None,
                local_vars: Vec::new(),
                span: start.to(self.prev_span()),
            };
        }
        let (
            clock,
            disable_iff,
            antecedent,
            implication_kind,
            consequent,
            consequent_clock,
            prop_expr,
            local_vars,
        ) = self.parse_property_spec(start);
        self.expect(TokenKind::RParen, "')'");
        // action_block ::= statement_or_null | [statement] `else` statement_or_null
        // (slice S11). A bare `;` leaves both None (default $error, no pass).
        let (pass, fail) = self.parse_assert_action_block();
        Stmt::ConcurrentAssert {
            clock,
            disable_iff,
            antecedent,
            implication_kind,
            consequent,
            consequent_clock,
            pass,
            fail,
            prop_expr,
            local_vars,
            span: start.to(self.prev_span()),
        }
    }

    /// SVA-REST: `cover property(@(clk) [disable iff(e)] seq);` — a coverage
    /// statement (counts sequence matches, reports the hit count at end-of-sim).
    /// Shares the clock + `disable iff` + sequence grammar with a property spec; an
    /// optional cover action block is loud-rejected (unsupported — never silently
    /// dropped). Cursor on `cover`.
    fn parse_cover_property(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // `cover`
        self.bump(); // `property` (Kw::Property)
        self.expect(TokenKind::LParen, "'(' after 'property'");
        let clock = if self.peek() == Some(TokenKind::At) {
            self.parse_sensitivity()
        } else {
            self.error("'@(...)' clocking event in cover property");
            Sensitivity::List(Vec::new())
        };
        let disable_iff = if self.at_kw(Kw::Disable) {
            self.bump(); // `disable`
            if self.at_ident_kw("iff") {
                self.bump();
            } else {
                self.error("`iff` after `disable` in a cover property");
            }
            self.expect(TokenKind::LParen, "'(' after `disable iff`");
            let e = self.expr(0);
            self.expect(TokenKind::RParen, "')' after `disable iff` condition");
            Some(e)
        } else {
            None
        };
        let seq = self.parse_sequence();
        self.expect(TokenKind::RParen, "')'");
        if !self.eat(TokenKind::Semi) {
            // A `cover property(...) <stmt>` success-action block is unsupported in
            // this subset — loud (do not silently drop the action), then skip the
            // statement for recovery.
            self.error(
                "';' after `cover property(...)` (a cover action block is unsupported \
                 in this subset)",
            );
            let _ = self.parse_statement();
        }
        Stmt::CoverProperty {
            clock,
            disable_iff,
            seq,
            span: start.to(self.prev_span()),
        }
    }

    /// SVA-REST: `let NAME [(formals)] = expr ;` (IEEE 1800 §11.13). Cursor on `let`.
    fn parse_let_decl(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        self.bump(); // `let`
        let name = self.ident()?;
        let formals = self.parse_sva_formals();
        self.expect(TokenKind::Eq, "'=' in a let declaration");
        let body = self.expr(0);
        self.expect(TokenKind::Semi, "';' after a let declaration");
        Some(ModuleItem::LetDecl(LetDecl {
            name,
            formals,
            body,
            span: start.to(self.prev_span()),
        }))
    }

    /// A synthetic `1` literal expr (the bare-property `1'b1 |-> e` sentinel and
    /// the named-instance placeholder consequent).
    fn sva_true_lit(span: Span) -> Expr {
        Expr {
            kind: ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw: "1".to_string(),
            },
            span,
        }
    }

    /// Parse a property spec `@(clk) [disable iff(e)] seq [ |-> | |=> ] seq` — the
    /// body shared by an inline `assert property( <spec> )` and a named
    /// `property NAME; <spec>; endproperty`. Does NOT consume the surrounding
    /// parens / terminators; the caller does.
    fn parse_property_spec(&mut self, start: Span) -> PropertySpecParts {
        // Sequence/property LOCAL VARIABLES (slice N2c, IEEE 1800-2017 §16.10): typed
        // declarations at the body start (`property p; int x; @(clk) …`). Captured at
        // a `(b, x = e)` match-item and read at a later term/consequent within the
        // SAME match attempt. Elaborate lowers a FIXED-DELAY single-capture to a
        // parallel DATA shift register (no per-attempt collision); ranges / multi-write
        // / cross-clock are loud-rejected there (the convergence cases). Parse the
        // declarations here into AST; each is `<type> <name> [= e] ;` and ends at its
        // own `;` (which precedes the clock).
        let mut local_vars: Vec<SvaLocalDecl> = Vec::new();
        while self.at_sva_local_var_decl() {
            if let Some(decl) = self.parse_sva_local_decl() {
                local_vars.push(decl);
            } else {
                // Malformed decl — recover by skipping to the next `;` / `@` so the
                // rest of the property still parses (never silently swallow it).
                while !matches!(
                    self.peek(),
                    Some(TokenKind::Semi) | Some(TokenKind::At) | None
                ) {
                    self.bump();
                }
                if !self.eat(TokenKind::Semi) {
                    break;
                }
            }
        }
        // Clocking event `@(...)`. `parse_sensitivity` consumes the leading `@`.
        let clock = if self.peek() == Some(TokenKind::At) {
            self.parse_sensitivity()
        } else {
            self.error("'@(...)' clocking event in concurrent assertion");
            Sensitivity::List(Vec::new())
        };
        // Optional `disable iff (expr)` reset (slice S12), between the clocking
        // event and the property expression. `disable` is a keyword; `iff` is a
        // contextual keyword (a plain identifier elsewhere).
        let disable_iff = if self.at_kw(Kw::Disable) {
            self.bump(); // `disable`
            if self.at_ident_kw("iff") {
                self.bump(); // `iff`
            } else {
                self.error("`iff` after `disable` in a concurrent assertion");
            }
            self.expect(TokenKind::LParen, "'(' after `disable iff`");
            let e = self.expr(0);
            self.expect(TokenKind::RParen, "')' after `disable iff` condition");
            Some(e)
        } else {
            None
        };
        // Sequence/property LOCAL VARIABLES can ALSO follow the clocking event (IEEE
        // §16.10 `property_spec`: the assertion_variable_declaration comes after the
        // clocking_event) — `@(clk) int x; (a, x=d) ##1 b |-> …`. Parse them here too
        // (the before-clock loop above covers the named-property `property p; int x;`
        // form). A type keyword at the property-expression start is unambiguously a
        // local-var decl (a sequence term never begins with a bare type keyword).
        while self.at_sva_local_var_decl() {
            if let Some(decl) = self.parse_sva_local_decl() {
                local_vars.push(decl);
            } else {
                while !matches!(self.peek(), Some(TokenKind::Semi) | None) {
                    self.bump();
                }
                if !self.eat(TokenKind::Semi) {
                    break;
                }
            }
        }
        // Property-level operators (slice N2d + SVA-REST): when the body uses a
        // top-level (paren-depth-0) property operator — `and`/`or` (N2d), or
        // `not`/`always`/`until`/`s_until`/`implies`/`iff`/`s_eventually`/`nexttime`
        // (SVA-REST) — parse a `PropExpr` TREE instead of the flat `seq impl seq`.
        // The flat fields then hold inert placeholders; elaborate dispatches on
        // `Some(prop_expr)`. This detection keeps every operator-free property (the
        // whole existing flat corpus) on the byte-identical flat path below —
        // including slice A3 multi-clock, whose `@(c2)` consequent clock the tree
        // grammar does NOT carry (combining a tree with multi-clock is out of subset
        // → loud at elaborate). An operator inside the clocking event or a
        // parenthesized sub-expression is at depth > 0 and ignored.
        if self.prop_has_toplevel_op() {
            let pe = self.parse_prop_expr();
            let true_lit = Self::sva_true_lit(start);
            return (
                clock,
                disable_iff,
                Sequence::Boolean(true_lit.clone()),
                ImplicationKind::Overlap,
                Sequence::Boolean(true_lit),
                None,
                Some(pe),
                local_vars,
            );
        }
        // `seq [ |-> | |=> ] expr` — a bare `property(@(clk) expr)` (no
        // implication) desugars to `1'b1 |-> expr`; `seq [ |-> | |=> ] seq` — the
        // consequent is also a Sequence (slice S14). A leading `@(c2)` on the
        // consequent of a `|=>` is a multi-clock property (slice A3).
        let ante_seq = self.parse_sequence();
        if self.eat(TokenKind::PipeArrow) {
            let cons_clock = self.parse_optional_consequent_clock(true);
            (
                clock,
                disable_iff,
                ante_seq,
                ImplicationKind::Overlap,
                self.parse_sequence(),
                cons_clock,
                None,
                local_vars,
            )
        } else if self.eat(TokenKind::PipeEqArrow) {
            let cons_clock = self.parse_optional_consequent_clock(false);
            (
                clock,
                disable_iff,
                ante_seq,
                ImplicationKind::NonOverlap,
                self.parse_sequence(),
                cons_clock,
                None,
                local_vars,
            )
        } else {
            let true_lit = Self::sva_true_lit(start);
            match ante_seq {
                // bare `property(@(clk) expr)` desugars to `1'b1 |-> expr`.
                Sequence::Boolean(e) => (
                    clock,
                    disable_iff,
                    Sequence::Boolean(true_lit),
                    ImplicationKind::Overlap,
                    Sequence::Boolean(e),
                    None,
                    None,
                    local_vars,
                ),
                other => {
                    self.error("an implication `|->`/`|=>` (a bare sequence property is unsupported in this subset)");
                    (
                        clock,
                        disable_iff,
                        other,
                        ImplicationKind::Overlap,
                        Sequence::Boolean(true_lit),
                        None,
                        None,
                        local_vars,
                    )
                }
            }
        }
    }

    /// Bounded paren/bracket-balanced lookahead from the cursor (which sits at the
    /// start of a property expression, after the clock + `disable iff`): true iff a
    /// property-level `and`/`or` keyword appears at depth 0 before the property's
    /// closing `)` (inline `assert property( … )`) or its `;` (a `property NAME; …;
    /// endproperty` declaration). Decisive and cannot be poisoned by a later
    /// construct — it stops at the first depth-underflow `)` / depth-0 `;` /
    /// `endproperty` / module boundary / EOF. `and`/`or` nested in the clocking
    /// event or a parenthesized sub-expression is at depth > 0 and ignored.
    fn prop_has_toplevel_op(&self) -> bool {
        const BUDGET: usize = 65536;
        let mut i = 0usize;
        let mut depth: i32 = 0;
        loop {
            match self.peek_at(i) {
                None => return false,
                // SVA repeat-open tokens (`[*` / `[->` / `[=`) open a bracket that
                // closes with a plain `]` (RBracket), so they must count for depth
                // or the `]` underflows and a trailing top-level operator is missed
                // (review N2d — the same new-token-vs-bracket-scan hazard as N2a-1).
                Some(
                    TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::LBracketStar
                    | TokenKind::LBracketArrow
                    | TokenKind::LBracketEq,
                ) => depth += 1,
                Some(TokenKind::RParen | TokenKind::RBracket) => {
                    if depth == 0 {
                        return false; // the property's closing `)` (inline form)
                    }
                    depth -= 1;
                }
                Some(TokenKind::Semi) if depth == 0 => return false, // decl body terminator
                // N2d keyword property operators (`and`/`or`) + SVA-REST prefix
                // `not`/`always`.
                Some(TokenKind::Word(WordKind::Keyword(
                    Kw::And | Kw::Or | Kw::Not | Kw::Always,
                ))) if depth == 0 => return true,
                Some(TokenKind::Word(WordKind::Keyword(Kw::Module | Kw::Endmodule)))
                    if depth == 0 =>
                {
                    return false
                }
                // SVA-REST contextual property operators (`until`/`implies`/
                // `s_eventually`/`nexttime`/…) — reserved SV words, so a property body
                // identifier never legitimately collides with them.
                Some(TokenKind::Word(WordKind::Ident)) if depth == 0 => {
                    if Self::is_prop_op_text(self.text_at(i)) {
                        return true;
                    }
                }
                _ => {}
            }
            if self.peek_at(i).is_some() && self.text_at(i) == "endproperty" {
                return false;
            }
            i += 1;
            if i > BUDGET {
                return false;
            }
        }
    }

    /// True for a CONTEXTUAL (non-keyword in our lexer, but reserved in SV) property
    /// operator word — the infix `until`/`s_until`/`implies`/`iff` and prefix
    /// `eventually`/`s_eventually`/`nexttime`/`s_nexttime`/`s_always`. These are
    /// IEEE 1800 reserved words, so a property-body identifier never legitimately
    /// shadows them (unlike a hand-rolled keyword guess).
    fn is_prop_op_text(s: &str) -> bool {
        matches!(
            s,
            "until"
                | "s_until"
                | "implies"
                | "iff"
                | "eventually"
                | "s_eventually"
                | "nexttime"
                | "s_nexttime"
                | "s_always"
        )
    }

    /// Parse a property expression (slice N2d + SVA-REST). Precedence loosest→
    /// tightest: `implies`/`iff` < `until`/`s_until` < `or` < `and` < unary prefix
    /// (`not`/`always`/`s_eventually`/`nexttime`) < sequence-implication < primary.
    /// Reached only when `prop_has_toplevel_op` detected a property-level operator.
    fn parse_prop_expr(&mut self) -> PropExpr {
        self.parse_prop_implies()
    }

    /// `lhs implies rhs` / `lhs iff rhs` (SVA-REST) — desugared to the `and`/`or`/`not`
    /// core: `p implies q` ≡ `(not p) or q`; `p iff q` ≡ `(p implies q) and (q implies
    /// p)`. Right-associative (`a implies b implies c` = `a implies (b implies c)`).
    fn parse_prop_implies(&mut self) -> PropExpr {
        let lhs = self.parse_prop_until();
        if self.eat_ident_kw("implies") {
            let rhs = self.parse_prop_implies();
            // p implies q ≡ (not p) or q
            return PropExpr::Or(Box::new(PropExpr::Not(Box::new(lhs))), Box::new(rhs));
        }
        if self.eat_ident_kw("iff") {
            let rhs = self.parse_prop_implies();
            // p iff q ≡ (not p or q) and (not q or p)
            let pq = PropExpr::Or(
                Box::new(PropExpr::Not(Box::new(lhs.clone()))),
                Box::new(rhs.clone()),
            );
            let qp = PropExpr::Or(Box::new(PropExpr::Not(Box::new(rhs))), Box::new(lhs));
            return PropExpr::And(Box::new(pq), Box::new(qp));
        }
        lhs
    }

    /// `lhs until rhs` / `lhs s_until rhs` (SVA-REST, non-associative single use).
    fn parse_prop_until(&mut self) -> PropExpr {
        let lhs = self.parse_prop_or();
        let strong = if self.at_ident_kw("s_until") {
            self.bump();
            true
        } else if self.eat_ident_kw("until") {
            false
        } else {
            return lhs;
        };
        let rhs = self.parse_prop_or();
        PropExpr::Until {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            strong,
        }
    }

    fn parse_prop_or(&mut self) -> PropExpr {
        let mut lhs = self.parse_prop_and();
        while self.at_kw(Kw::Or) {
            self.bump(); // `or`
            let rhs = self.parse_prop_and();
            lhs = PropExpr::Or(Box::new(lhs), Box::new(rhs));
        }
        lhs
    }

    fn parse_prop_and(&mut self) -> PropExpr {
        let mut lhs = self.parse_prop_unary();
        while self.at_kw(Kw::And) {
            self.bump(); // `and`
            let rhs = self.parse_prop_unary();
            lhs = PropExpr::And(Box::new(lhs), Box::new(rhs));
        }
        lhs
    }

    /// Unary prefix property operators (SVA-REST): `not p`, `always p`,
    /// `s_eventually p`, `nexttime p` (right-recursive: `not always p` =
    /// `Not(Always(p))`). `nexttime`/`s_nexttime` desugar to `1'b1 |=> p`. The bounded
    /// forms (`s_eventually [m:n]`, `nexttime [n]`, `s_always`) and weak unbounded
    /// `eventually` are loud-rejected (recovery: parse the operand so the rest syncs).
    fn parse_prop_unary(&mut self) -> PropExpr {
        if self.eat_kw(Kw::Not) {
            return PropExpr::Not(Box::new(self.parse_prop_unary()));
        }
        if self.eat_kw(Kw::Always) {
            return PropExpr::Always(Box::new(self.parse_prop_unary()));
        }
        if self.at_ident_kw("s_eventually") || self.at_ident_kw("eventually") {
            let strong = self.cur_text() == "s_eventually";
            self.bump();
            if self.peek() == Some(TokenKind::LBracket) {
                self.error(
                    "an unbounded `s_eventually` (a bounded `s_eventually [m:n]` range \
                     is unsupported in this subset)",
                );
                // consume the `[ … ]` for recovery.
                let mut d = 0i32;
                while let Some(t) = self.peek() {
                    match t {
                        TokenKind::LBracket => d += 1,
                        TokenKind::RBracket => {
                            d -= 1;
                            if d == 0 {
                                self.bump();
                                break;
                            }
                        }
                        _ => {}
                    }
                    self.bump();
                }
            }
            if !strong {
                self.error(
                    "`s_eventually` (a weak unbounded `eventually` has no bounded-sim \
                     verdict; use `s_eventually`)",
                );
            }
            return PropExpr::Eventually {
                strong: true,
                prop: Box::new(self.parse_prop_unary()),
            };
        }
        if self.at_ident_kw("nexttime") || self.at_ident_kw("s_nexttime") {
            self.bump();
            if self.peek() == Some(TokenKind::LBracket) {
                self.error(
                    "an unbounded `nexttime` (a bounded `nexttime [n]` is unsupported \
                     in this subset)",
                );
            }
            // `nexttime p` ≡ `1'b1 |=> p`.
            let sp = self.prev_span();
            return PropExpr::Impl {
                ante: Sequence::Boolean(Self::sva_true_lit(sp)),
                kind: ImplicationKind::NonOverlap,
                cons: Box::new(self.parse_prop_unary()),
            };
        }
        if self.at_ident_kw("s_always") {
            self.error(
                "`always` (a bounded `s_always` strong-always is unsupported in this \
                 subset)",
            );
            self.bump();
            return PropExpr::Always(Box::new(self.parse_prop_unary()));
        }
        self.parse_prop_impl()
    }

    /// A property primary, optionally the antecedent of a single implication. A
    /// parenthesized PROPERTY `( … |-> … )` / `( … and … )` recurses; a
    /// parenthesized boolean expression `(a && b)` is left to `parse_sequence`
    /// (the implication antecedent). The consequent of `|->`/`|=>` is a full
    /// property expression, so `1'b1 |=> p` (the recursion site) parses with `p`
    /// as a bare `Seq(Boolean(Ident))` leaf resolved at elaborate.
    fn parse_prop_impl(&mut self) -> PropExpr {
        if self.peek() == Some(TokenKind::LParen) && self.paren_group_is_property() {
            self.bump(); // `(`
            let inner = self.parse_prop_expr();
            self.expect(TokenKind::RParen, "')' to close a parenthesized property");
            return inner;
        }
        let ante = self.parse_sequence();
        if self.eat(TokenKind::PipeArrow) {
            PropExpr::Impl {
                ante,
                kind: ImplicationKind::Overlap,
                cons: Box::new(self.parse_prop_expr()),
            }
        } else if self.eat(TokenKind::PipeEqArrow) {
            PropExpr::Impl {
                ante,
                kind: ImplicationKind::NonOverlap,
                cons: Box::new(self.parse_prop_expr()),
            }
        } else {
            PropExpr::Seq(ante)
        }
    }

    /// Cursor on `(`: true iff the balanced paren group contains, at the depth just
    /// inside this paren, a property operator (`|->`/`|=>`/`and`/`or`) — i.e. it is
    /// a parenthesized PROPERTY rather than a parenthesized boolean expression
    /// (which `parse_sequence` handles as an implication antecedent / leaf).
    fn paren_group_is_property(&self) -> bool {
        const BUDGET: usize = 65536;
        let mut i = 0usize;
        let mut depth: i32 = 0;
        loop {
            match self.peek_at(i) {
                None => return false,
                // SVA repeat-open tokens count for depth (see `prop_has_toplevel_andor`).
                Some(
                    TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::LBracketStar
                    | TokenKind::LBracketArrow
                    | TokenKind::LBracketEq,
                ) => depth += 1,
                Some(TokenKind::RParen | TokenKind::RBracket) => {
                    depth -= 1;
                    if depth == 0 {
                        return false; // closed without a property operator
                    }
                }
                Some(TokenKind::PipeArrow | TokenKind::PipeEqArrow) if depth == 1 => return true,
                Some(TokenKind::Word(WordKind::Keyword(
                    Kw::And | Kw::Or | Kw::Not | Kw::Always,
                ))) if depth == 1 => return true,
                Some(TokenKind::Word(WordKind::Ident)) if depth == 1 => {
                    if Self::is_prop_op_text(self.text_at(i)) {
                        return true;
                    }
                }
                _ => {}
            }
            i += 1;
            if i > BUDGET {
                return false;
            }
        }
    }

    /// Parse an optional leading `@(c2)` consequent clocking event (slice A3, after
    /// the implication operator). `|=>` accepts it (multi-clock handoff); `|->` does
    /// NOT (no coherent same-tick cross-clock check) → loud, consume for recovery.
    fn parse_optional_consequent_clock(&mut self, is_overlap: bool) -> Option<Sensitivity> {
        if self.peek() != Some(TokenKind::At) {
            return None;
        }
        if is_overlap {
            // `self.error` frames its argument as "expected <X>, found <Y>", so the
            // message must be a noun phrase (review 2026-06-16).
            self.error(
                "a `|=>` for a multi-clock property (an overlapping `|->` cannot take \
                 a consequent clocking event)",
            );
            let _ = self.parse_sensitivity(); // consume `@(c2)` so the rest recovers
            return None;
        }
        Some(self.parse_sensitivity())
    }

    /// Disambiguate `sequence IDENT ( … )` (cursor on `sequence`) between an SVA
    /// sequence DECLARATION and a module instantiation of a V2005 module literally
    /// named `sequence`. The decl shape is exactly `sequence NAME ( formals ) ; BODY
    /// ; endsequence`, so AFTER the formals-terminator `;` the body is a single
    /// sequence expression with NO top-level `;`, and its terminating `;` is
    /// immediately followed by `endsequence`. An instantiation `sequence u(…) ;` has
    /// no such body+`endsequence`. We therefore (1) skip the balanced `( … )`, (2)
    /// require the formals `;`, then (3) scan the body to its next depth-0 `;` and
    /// accept ONLY if `endsequence` follows it. DECISIVE and bounded to the candidate
    /// construct — it cannot be poisoned by an unrelated later `sequence … endsequence`
    /// (review 2026-06-16: a content-independent scan-until-`endsequence` mis-routed a
    /// positional `sequence u(o);` merely followed by a real decl, and a fixed token
    /// budget flipped long decls). Lets `sequence u(.o(o));` stay an instantiation.
    fn is_sequence_decl_ahead(&self) -> bool {
        const BUDGET: usize = 65536;
        // (1) Skip the balanced `( … )` — peek_at(2) is the opening `(`.
        let mut i = 2usize;
        let mut depth = 0usize;
        loop {
            match self.peek_at(i) {
                None => return false,
                Some(TokenKind::LParen) => {
                    depth += 1;
                    i += 1;
                }
                Some(TokenKind::RParen) => {
                    i += 1;
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => i += 1,
            }
            if i > BUDGET {
                return false;
            }
        }
        // (2) The formals list must be terminated by `;`.
        if self.peek_at(i) != Some(TokenKind::Semi) {
            return false;
        }
        i += 1;
        // (3) Scan the body to its next depth-0 `;`; a decl has `endsequence` after it.
        let mut bdepth = 0usize;
        loop {
            match self.peek_at(i) {
                None => return false,
                Some(TokenKind::LParen | TokenKind::LBracket) => {
                    bdepth += 1;
                    i += 1;
                }
                Some(TokenKind::RParen | TokenKind::RBracket) => {
                    bdepth = bdepth.saturating_sub(1);
                    i += 1;
                }
                Some(TokenKind::Semi) if bdepth == 0 => {
                    return self.text_at(i + 1) == "endsequence";
                }
                // A hard module boundary before the body terminator ⇒ not a decl.
                Some(TokenKind::Word(WordKind::Keyword(Kw::Module | Kw::Endmodule)))
                    if bdepth == 0 =>
                {
                    return false
                }
                _ => i += 1,
            }
            if i > BUDGET {
                return false;
            }
        }
    }

    /// Named SVA `sequence NAME [(formals)]; <seq>; endsequence` (IEEE §16.8).
    /// Formal arguments (slice A1) bind by position at the use site; the body reuses
    /// the existing `parse_sequence`, so every sequence operator is available by name.
    fn parse_sequence_decl(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        self.bump(); // `sequence` (contextual keyword)
        let name = self.ident()?;
        let formals = self.parse_sva_formals();
        self.expect(TokenKind::Semi, "';' after sequence name");
        let body = self.parse_sequence();
        self.expect(TokenKind::Semi, "';' after sequence body");
        if self.at_ident_kw("endsequence") {
            self.bump();
        } else {
            self.error("`endsequence`");
        }
        self.eat_end_label();
        Some(ModuleItem::SequenceDecl(SeqDecl {
            name,
            formals,
            body,
            span: start.to(self.prev_span()),
        }))
    }

    /// Named SVA `property NAME [(formals)]; <property_spec>; endproperty`
    /// (IEEE §16.12). Reuses `parse_property_spec` for the body; spliced at an
    /// `assert property(NAME)` instance by elaborate.
    fn parse_property_decl(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        self.bump(); // `property` (Kw::Property)
        let name = self.ident()?;
        let formals = self.parse_sva_formals();
        self.expect(TokenKind::Semi, "';' after property name");
        let (
            clock,
            disable_iff,
            antecedent,
            implication_kind,
            consequent,
            consequent_clock,
            prop_expr,
            local_vars,
        ) = self.parse_property_spec(start);
        self.expect(TokenKind::Semi, "';' after property body");
        if self.at_ident_kw("endproperty") {
            self.bump();
        } else {
            self.error("`endproperty`");
        }
        self.eat_end_label();
        Some(ModuleItem::PropertyDecl(PropDecl {
            name,
            formals,
            clock,
            disable_iff,
            antecedent,
            implication_kind,
            consequent,
            consequent_clock,
            prop_expr,
            local_vars,
            span: start.to(self.prev_span()),
        }))
    }

    /// Parse an SVA formal-argument list `( formal {, formal} )` after a named
    /// sequence/property name (slice A1, IEEE 1800 §16.8/§16.12). A formal is
    /// `[data_type] name [= default]`; the formal NAME is what elaborate substitutes,
    /// so we capture the LAST identifier before a top-level `,` / `)` / `=` and skip
    /// the optional type prefix and any default value (defaults are unsupported — all
    /// actuals must be passed; an arity mismatch is loud at the use site). No `(` →
    /// an empty list (a non-parameterized decl, byte-identical to before this slice).
    fn parse_sva_formals(&mut self) -> Vec<Ident> {
        let mut out = Vec::new();
        if self.peek() != Some(TokenKind::LParen) {
            return out;
        }
        self.bump(); // `(`
        if self.eat(TokenKind::RParen) {
            return out; // empty `()`
        }
        loop {
            match self.parse_one_sva_formal() {
                Some(id) => out.push(id),
                // An empty entry (`(,)`, `(,x)`, `(x,)`) is malformed — loud rather
                // than silently normalized (review 2026-06-16). Arity is still
                // enforced at the use site; this is recovery, not fatal.
                None => self.error("a formal name in the sequence/property formal list"),
            }
            if self.eat(TokenKind::Comma) {
                continue;
            }
            if !self.eat(TokenKind::RParen) {
                self.error("',' or ')' in a sequence/property formal list");
            }
            break;
        }
        out
    }

    /// One SVA formal: scan to the next top-level `,` / `)` / `=`, returning the last
    /// identifier seen (the formal name), regardless of any leading type / range
    /// tokens. A `= default` value is parse-and-dropped (unsupported).
    fn parse_one_sva_formal(&mut self) -> Option<Ident> {
        let mut last: Option<Ident> = None;
        loop {
            match self.peek() {
                Some(TokenKind::Comma) | Some(TokenKind::RParen) | None => break,
                Some(TokenKind::Eq) => {
                    self.bump(); // `=`
                    let _ = self.expr(0); // default value — consumed and ignored
                    break;
                }
                _ if self.is_ident() => {
                    last = self.ident();
                }
                _ => {
                    self.bump(); // a type keyword / `[m:l]` range / etc.
                }
            }
        }
        last
    }

    /// True at a sequence/property body LOCAL-VARIABLE declaration (slice A2): a
    /// data-type keyword (`logic`/`reg`/`integer`/`bit`-via-`logic`/…) or an SV
    /// integral type name lexed as an identifier (`int`/`bit`/`byte`/`shortint`/
    /// `longint`). A property/sequence body must otherwise begin with `@(clk)` (a
    /// property) or a sequence expression, so a type at the body start is a local var.
    fn at_sva_local_var_decl(&self) -> bool {
        if self.net_var_kind().is_some() {
            return true;
        }
        self.is_ident()
            && matches!(
                self.cur_text(),
                "int" | "bit" | "byte" | "shortint" | "longint"
            )
    }

    /// Parse ONE sequence/property local-variable declaration (slice N2c, IEEE
    /// §16.10): `<type> [packed_range] <name> [= init] ;`. The cursor is on the type
    /// keyword (`at_sva_local_var_decl` is true). Returns the resolved width/sign so
    /// elaborate can size the data-tracking register. `None` on a malformed decl (the
    /// caller recovers by skipping to the next `;`). Only the integral atom types and
    /// a literal packed range are supported; the type keyword itself is consumed.
    fn parse_sva_local_decl(&mut self) -> Option<SvaLocalDecl> {
        let start = self.cur_span();
        let kind = self.net_var_kind()?;
        self.bump(); // the type keyword
                     // Atom storage width by kind; a packed range (below) overrides for the
                     // vector-capable kinds (bit/logic/reg).
                     // `unsupported`: the declared type is NOT a synthesizable fixed-width
                     // integral var, so a capture into it has no data-tracking register in this
                     // subset. The width/sign below are a 1-bit placeholder; elaborate's
                     // `synth_local_var_assert` reads this flag and LOUD-rejects the capture —
                     // never a silent 1-bit truncation that flips the assertion verdict.
        let (atom_width, ranged_ok, unsupported): (u32, bool, bool) = match kind {
            NetVarKind::Byte => (8, false, false),
            NetVarKind::Shortint => (16, false, false),
            NetVarKind::Int | NetVarKind::Integer => (32, false, false),
            NetVarKind::Longint | NetVarKind::Time => (64, false, false),
            NetVarKind::Bit | NetVarKind::Logic | NetVarKind::Reg => (1, true, false),
            // Real / realtime / string / event / nets / class are not a
            // synthesizable data-tracking var — flag them so elaborate loud-rejects
            // (carry a 1-bit placeholder width; the type is validated at the capture).
            _ => (1, false, true),
        };
        // Optional packed range `[msb:lsb]` (vector kinds only). Literal bounds only;
        // a non-literal bound recovers via `parse_small_const` (loud) → width fallback.
        let width = if ranged_ok && self.peek() == Some(TokenKind::LBracket) {
            self.bump(); // `[`
            let msb = self.parse_small_const("a packed-range MSB in an SVA local var");
            self.expect(TokenKind::Colon, "':' in an SVA local-var packed range");
            let lsb = self.parse_small_const("a packed-range LSB in an SVA local var");
            self.expect(
                TokenKind::RBracket,
                "']' to close an SVA local-var packed range",
            );
            msb.abs_diff(lsb) + 1
        } else {
            atom_width
        };
        let signed = atom_default_signed(Some(kind));
        let name = self.ident()?;
        let init = if self.eat(TokenKind::Eq) {
            Some(self.expr(0))
        } else {
            None
        };
        self.expect(
            TokenKind::Semi,
            "';' after an SVA local-variable declaration",
        );
        Some(SvaLocalDecl {
            name,
            width,
            signed,
            unsupported_type: unsupported,
            init,
            span: start.to(self.prev_span()),
        })
    }

    /// True if the upcoming `( … )` (cursor on `(`) contains a comma at paren-depth
    /// one — a sequence MATCH-ITEM local-variable list `(bool, x = e, …)` (slice A2).
    /// A parenthesized sequence has no top-level comma; concat/select commas nest
    /// deeper (counted via all bracket kinds), so they are not mistaken for one.
    fn at_sva_match_item_paren(&self) -> bool {
        if self.peek() != Some(TokenKind::LParen) {
            return false;
        }
        const BUDGET: usize = 8192;
        let mut depth = 0usize;
        let mut i = 0usize;
        while i < BUDGET {
            match self.peek_at(i) {
                None => return false,
                Some(
                    TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::LBrace
                    | TokenKind::LBracketStar
                    | TokenKind::LBracketArrow
                    | TokenKind::LBracketEq,
                ) => {
                    depth += 1;
                    i += 1;
                }
                Some(TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace) => {
                    depth = depth.saturating_sub(1);
                    i += 1;
                    if depth == 0 {
                        return false; // outer paren closed with no top-level comma
                    }
                }
                Some(TokenKind::Comma) if depth == 1 => return true,
                _ => i += 1,
            }
        }
        false
    }

    /// True if the upcoming `( … )` (cursor on `(`) is a parenthesized SUB-SEQUENCE
    /// — it contains a `##` cycle-delay concatenation at paren-depth one (slice A.2
    /// cross-clock multi-term segment, e.g. `@(c1)(a ##1 b)`). A parenthesized boolean
    /// expression `(a && b)` has no top-level `##`, so it is left to `self.expr(0)`
    /// (byte-identical). A match-item paren is detected separately and takes priority.
    fn at_paren_subsequence(&self) -> bool {
        if self.peek() != Some(TokenKind::LParen) {
            return false;
        }
        const BUDGET: usize = 8192;
        let mut depth = 0usize;
        let mut i = 0usize;
        while i < BUDGET {
            match self.peek_at(i) {
                None => return false,
                Some(
                    TokenKind::LParen
                    | TokenKind::LBracket
                    | TokenKind::LBrace
                    | TokenKind::LBracketStar
                    | TokenKind::LBracketArrow
                    | TokenKind::LBracketEq,
                ) => {
                    depth += 1;
                    i += 1;
                }
                Some(TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace) => {
                    depth = depth.saturating_sub(1);
                    i += 1;
                    if depth == 0 {
                        return false; // outer paren closed with no top-level `##`
                    }
                }
                Some(TokenKind::HashHash) if depth == 1 => return true,
                _ => i += 1,
            }
        }
        false
    }

    /// Consume an optional `: label` after `endsequence`/`endproperty`
    /// (accept-and-ignore — the minimal-surface choice).
    fn eat_end_label(&mut self) {
        if self.peek() == Some(TokenKind::Colon) {
            self.bump();
            let _ = self.ident();
        }
    }

    /// Parse an assertion action block after the `property(...)` close paren
    /// (slice S11): `[pass_stmt] [else fail_stmt]`. A bare `;` yields
    /// `(None, None)` — the default $error fail and no pass action, kept distinct
    /// from `(Some(Null), None)` so the no-action checker is byte-identical to
    /// before this slice. Each statement consumes its own terminating `;`.
    fn parse_assert_action_block(&mut self) -> (Option<Box<Stmt>>, Option<Box<Stmt>>) {
        // `eat(Semi)` consumes a bare `;` (empty action block); `at_kw(Else)`
        // (else-only form) leaves the `else` for the `fail` arm below — both yield
        // no pass action. Short-circuit `||` keeps the `;`-consuming side effect.
        let pass = if self.eat(TokenKind::Semi) || self.at_kw(Kw::Else) {
            None
        } else {
            Some(Box::new(self.parse_statement()))
        };
        let fail = if self.eat_kw(Kw::Else) {
            Some(Box::new(self.parse_statement()))
        } else {
            None
        };
        (pass, fail)
    }

    /// Parse an SVA sequence (Phase-3 slices S4/S5): `##n` / bounded-range
    /// `##[m:n]` cycle-delay concatenation (left-associative, looser) over
    /// primaries that may carry a `[*n]` / `[*m:n]` consecutive-repetition
    /// postfix (tighter). Unbounded (`[*m:$]`/`##[m:$]`) / goto / nonconsecutive
    /// / `throughout` / `within` forms are deferred — loud at the enclosing
    /// `expect`. (min,max) carry the bound; max==Some(min) is the single-count
    /// form.
    fn parse_sequence(&mut self) -> Sequence {
        // `##` concatenation (tightest of the binary sequence ops).
        let lhs = self.parse_seq_concat();
        // `seq1 within seq2` (slice S9) — contextual keyword, binary over `##`
        // chains, RHS a full sequence.
        if self.at_ident_kw("within") {
            self.bump(); // `within`
            let rhs = self.parse_sequence();
            return Sequence::Within {
                seq1: Box::new(lhs),
                seq2: Box::new(rhs),
            };
        }
        // `cond throughout seq` (slice S7) — `throughout` is a contextual keyword;
        // its left operand must be a boolean leaf, its right operand a full
        // sequence (looser than `##`, so `g throughout a ##2 c` is
        // `g throughout (a ##2 c)`).
        if self.at_ident_kw("throughout") {
            self.bump(); // `throughout`
            let seq = self.parse_sequence();
            return match lhs {
                Sequence::Boolean(cond) => Sequence::Throughout {
                    cond: Box::new(cond),
                    seq: Box::new(seq),
                },
                _ => {
                    self.error("`throughout` requires a boolean left operand");
                    seq
                }
            };
        }
        lhs
    }

    /// `##`-concatenation chain over sequence primaries (left-associative).
    fn parse_seq_concat(&mut self) -> Sequence {
        // A leading `##N` with no left operand — e.g. the consequent of
        // `a |-> ##1 b`. Per IEEE 1800 §16.9, `##N b` ≡ `1 ##N b` (a true leaf
        // delayed by N). Synthesize the implicit `1` so the delay chain has a left
        // operand; this produces the SAME pipeline `a |=> b` / `1 ##1 b` already do
        // (golden-neutral). Without it the primary parser hits `##` as an expression
        // and reports a spurious E2002.
        let mut lhs = if self.peek() == Some(TokenKind::HashHash) {
            Sequence::Boolean(Self::sva_true_lit(self.cur_span()))
        } else {
            self.parse_seq_primary()
        };
        while self.peek() == Some(TokenKind::HashHash) {
            self.bump(); // `##`
            let (min, max) = self.parse_seq_delay();
            let rhs = self.parse_seq_primary();
            lhs = Sequence::Delay {
                min,
                max,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        lhs
    }

    /// A sequence primary: a boolean leaf expression, optionally followed by one
    /// or more repetition postfixes — `[*n]`/`[*m:n]` consecutive, `[->n]` goto,
    /// or `[=n]` nonconsecutive.
    fn parse_seq_primary(&mut self) -> Sequence {
        // A `@(...)` clocking event at a sequence primary is a multi-clock RE-CLOCKING
        // boundary (slice N2a): `a ##1 @(c2) b`. The leading property clock was already
        // consumed by `parse_concurrent_assert`, so a `@` here re-establishes the
        // sampling clock for the following primary from this `##`-boundary onward
        // (IEEE 1800 §16.13/§16.16 clock flow). Wrap the following primary in
        // `Sequence::Clocked`; elaborate's `synth_crossclock` handles the supported
        // `a ##1 @(c2) b` shape and loud-rejects the rest.
        if self.peek() == Some(TokenKind::At) {
            let clock = self.parse_sensitivity();
            let seq = self.parse_seq_primary();
            return Sequence::Clocked {
                clock,
                seq: Box::new(seq),
            };
        }
        // A `( boolean , local_var = expr {, …} )` match-item paren (slice N2c, IEEE
        // §16.10) is a sequence LOCAL-VARIABLE capture — a top-level comma just inside
        // the paren distinguishes it from a parenthesized sequence (which has none).
        // Parse the boolean term + the `name = expr` assignment list into a
        // `Sequence::MatchItem`; elaborate lowers a fixed-delay single capture to a
        // parallel data shift register (and loud-rejects the convergence cases).
        if self.at_sva_match_item_paren() {
            return self.parse_seq_match_item();
        }
        // A parenthesized SUB-SEQUENCE `( … ## … )` (slice A.2 cross-clock multi-term
        // segment) — `self.expr(0)` cannot parse a `##` cycle-delay, so when the group
        // carries a top-level `##` recurse into `parse_sequence`. A parenthesized
        // boolean `(a && b)` has no top-level `##` → falls through to `expr(0)` (byte-
        // identical). The result still flows through the postfix-repetition loop below.
        if self.at_paren_subsequence() {
            self.bump(); // `(`
            let inner = self.parse_sequence();
            self.expect(TokenKind::RParen, "')' to close a parenthesized sequence");
            return self.parse_seq_postfix(inner);
        }
        let e = self.expr(0);
        let seq = Sequence::Boolean(e);
        self.parse_seq_postfix(seq)
    }

    /// Parse a sequence MATCH-ITEM `( <bool> , <name> = <expr> {, <name> = <expr>} )`
    /// (slice N2c, IEEE §16.10): a boolean term that CAPTURES local variable(s) when
    /// it matches. The cursor is on `(` and `at_sva_match_item_paren` is true (a
    /// top-level comma was seen). Emits `Sequence::MatchItem`; the postfix loop still
    /// applies (a `(...)[*n]` repeated capture is parsed but elaborate loud-rejects it
    /// — repetition would converge multiple captures, a data collision).
    fn parse_seq_match_item(&mut self) -> Sequence {
        self.bump(); // `(`
        let term = self.expr(0); // the boolean term
        let mut assigns: Vec<(Ident, Expr)> = Vec::new();
        // At least one `, name = expr` (the detector guaranteed a top-level comma).
        while self.eat(TokenKind::Comma) {
            let name = match self.ident() {
                Some(id) => id,
                None => {
                    self.error("a local-variable name in a sequence match-item `(b, x = e)`");
                    break;
                }
            };
            if !self.eat(TokenKind::Eq) {
                self.error("`=` in a sequence match-item local-variable assignment");
                break;
            }
            let val = self.expr(0);
            assigns.push((name, val));
        }
        self.expect(TokenKind::RParen, "')' to close a sequence match-item");
        let mi = Sequence::MatchItem {
            seq: Box::new(Sequence::Boolean(term)),
            assigns,
        };
        self.parse_seq_postfix(mi)
    }

    /// Apply the trailing repetition postfixes (`[*n]`/`[*m:n]` consecutive, `[+]`,
    /// `[->n]` goto, `[=n]` nonconsecutive) to an already-parsed sequence primary.
    /// Shared by the boolean-leaf and parenthesized-subsequence primary forms.
    fn parse_seq_postfix(&mut self, mut seq: Sequence) -> Sequence {
        loop {
            match self.peek() {
                Some(TokenKind::LBracketStar) => {
                    self.bump(); // `[*`
                    let (min, max) = self.parse_seq_repeat_bounds();
                    self.expect(TokenKind::RBracket, "']' to close `[*n]`");
                    seq = Sequence::Repeat {
                        seq: Box::new(seq),
                        min,
                        max,
                        kind: RepeatKind::Consec,
                    };
                }
                // SVA-REST `seq[+]` consecutive-repetition sugar ≡ `seq[*1:$]`
                // (one-or-more, unbounded — the S13 run-latch). `seq[*]` (≡ `[*0:$]`,
                // a zero-or-more EMPTY match) is `parse_seq_repeat_bounds` → (0, None).
                Some(TokenKind::BracketPlus) => {
                    self.bump(); // `[+]`
                    seq = Sequence::Repeat {
                        seq: Box::new(seq),
                        min: 1,
                        max: None,
                        kind: RepeatKind::Consec,
                    };
                }
                Some(tok @ (TokenKind::LBracketArrow | TokenKind::LBracketEq)) => {
                    self.bump(); // `[->` / `[=`
                    let (which, kind) = if tok == TokenKind::LBracketArrow {
                        ("[->n]", RepeatKind::Goto)
                    } else {
                        ("[=n]", RepeatKind::Nonconsec)
                    };
                    let n = self.parse_seq_count_single(which);
                    self.expect(TokenKind::RBracket, "']' to close goto/nonconsec count");
                    seq = Sequence::Repeat {
                        seq: Box::new(seq),
                        min: n,
                        max: Some(n),
                        kind,
                    };
                }
                _ => break,
            }
        }
        seq
    }

    /// Single positive count for `[->n]` / `[=n]`. Ranges (`[->m:n]`) and `0`
    /// are deferred (loud, recovered to 1).
    fn parse_seq_count_single(&mut self, which: &'static str) -> u32 {
        let n = self.parse_small_const(which);
        if self.peek() == Some(TokenKind::Colon) {
            self.error("a single goto/nonconsec count (ranges are unsupported in this subset)");
        }
        if n == 0 {
            self.error("a positive goto/nonconsec count");
            return 1;
        }
        n
    }

    /// Cycle delay after `##`: `##n` → (n, Some(n)), bounded range `##[m:n]`
    /// → (m, Some(n)), or unbounded `##[m:$]` → (m, None) (slice S6).
    fn parse_seq_delay(&mut self) -> (u32, Option<u32>) {
        if self.peek() == Some(TokenKind::LBracket) {
            self.bump(); // `[`
            let lo = self.parse_small_const("a lower bound in `##[m:n]`");
            self.expect(TokenKind::Colon, "':' in `##[m:n]`");
            if self.peek() == Some(TokenKind::Dollar) {
                self.bump(); // `$` — unbounded upper bound
                self.expect(TokenKind::RBracket, "']'");
                return (lo, None);
            }
            let hi = self.parse_small_const("an upper bound in `##[m:n]`");
            self.expect(TokenKind::RBracket, "']'");
            let (lo, hi) = (lo.min(hi), lo.max(hi));
            return (lo, Some(hi));
        }
        let n = self.parse_small_const("a constant cycle delay after `##`");
        (n, Some(n))
    }

    /// `[*n]` repetition bounds: `[*n]` → (n, Some(n)), bounded range `[*m:n]`
    /// → (m, Some(n)), unbounded `[*m:$]` → (m, None) (slice S13), or a zero
    /// lower bound — `[*0]`/`[*0:0]` → (0, Some(0)) (exactly empty), `[*0:n]` →
    /// (0, Some(n)), bare `[*]`/`[*0:$]` → (0, None) (empty-or-more). The empty
    /// (zero-repetition) match is synthesized for SUFFIX/MIDDLE positions
    /// (`a ##1 b[*0:n]`); a leading/standalone empty is honest-loud at elaborate
    /// (the empty SEED's -1 offset is not expressible). See `sva_empty_match.rs`.
    /// Caller consumed `[*`; this stops before `]`.
    fn parse_seq_repeat_bounds(&mut self) -> (u32, Option<u32>) {
        // Bare `[*]` ≡ `[*0:$]` — zero-or-more (empty-or-more).
        if self.peek() == Some(TokenKind::RBracket) {
            return (0, None);
        }
        let lo = self.parse_small_const("a repetition count in `[*n]`");
        if self.peek() == Some(TokenKind::Colon) {
            self.bump(); // ':'
            if self.peek() == Some(TokenKind::Dollar) {
                self.bump(); // `$` — unbounded upper bound: `[*m:$]` (>= m)
                return (lo, None);
            }
            let hi = self.parse_small_const("an upper bound in `[*m:n]`");
            let (lo, hi) = (lo.min(hi), lo.max(hi));
            return (lo, Some(hi));
        }
        (lo, Some(lo))
    }

    /// Read a small unsigned decimal constant from the current `IntDecimal`
    /// token (digit separators stripped). Non-literal / oversized → loud, 1.
    fn parse_small_const(&mut self, what: &'static str) -> u32 {
        if self.peek() == Some(TokenKind::IntDecimal) {
            let v = self.cur_text().replace('_', "").parse::<u32>().ok();
            self.bump();
            if let Some(v) = v {
                return v;
            }
        }
        self.error(what);
        1
    }

    // ─────────────────────── 4. control flow ───────────────────────
    fn parse_if(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // if
        self.expect(TokenKind::LParen, "'(' after 'if'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let then_s = Box::new(self.parse_statement());
        // dangling-else binds EAGERLY to this (nearest) if
        let else_s = if self.eat_kw(Kw::Else) {
            Some(Box::new(self.parse_statement()))
        } else {
            None
        };
        Stmt::If {
            cond,
            then_s,
            else_s,
            span: start.to(self.prev_span()),
        }
    }

    fn parse_case(&mut self, kind: CaseKind) -> Stmt {
        let start = self.cur_span();
        self.bump(); // case/casez/casex
        self.expect(TokenKind::LParen, "'(' after case");
        let scrutinee = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let mut items = Vec::new();
        while !self.at_eof() && !self.at_kw(Kw::Endcase) {
            let before = self.pos;
            items.push(self.parse_case_item());
            if self.pos == before {
                self.bump(); // never spin on a stuck case item
            }
        }
        self.expect(TokenKind::Word(WordKind::Keyword(Kw::Endcase)), "'endcase'");
        Stmt::Case {
            kind,
            scrutinee,
            items,
            span: start.to(self.prev_span()),
        }
    }

    /// `default [:] stmt` | `label {, label} : stmt`.
    fn parse_case_item(&mut self) -> CaseItem {
        let start = self.cur_span();
        if self.eat_kw(Kw::Default) {
            self.eat(TokenKind::Colon); // ':' OPTIONAL after default
            let body = Box::new(self.parse_statement());
            return CaseItem::Default {
                body,
                span: start.to(self.prev_span()),
            };
        }
        let mut labels = vec![self.expr(0)];
        while !self.node_budget_blown && self.eat(TokenKind::Comma) {
            labels.push(self.expr(0));
        }
        self.expect(TokenKind::Colon, "':' in case item");
        let body = Box::new(self.parse_statement());
        CaseItem::Match {
            labels,
            body,
            span: start.to(self.prev_span()),
        }
    }

    // ───────────────────────── SV §11.5 break/continue ─────────────────────
    /// Parse a loop body while tracking `break`/`continue` that target THIS loop.
    /// Pushes a `LoopLabels` (unique by the loop's start offset), parses the body,
    /// then — IF `continue` was used — wraps the body in a synthetic named block
    /// `begin : $continue$<lo> body end` (its exit is the loop's continue point:
    /// before the for-step / at the while back-edge). Returns the (maybe-wrapped)
    /// body and whether `break` was used (the caller wraps the whole loop). A loop
    /// with no break/continue is returned UNWRAPPED ⇒ byte-identical.
    fn parse_loop_body(&mut self, start: Span) -> (Stmt, bool) {
        let lo = start.lo;
        self.loop_labels.push(LoopLabels {
            break_label: format!("$break${lo}"),
            continue_label: format!("$continue${lo}"),
            break_used: false,
            continue_used: false,
        });
        let body = self.parse_statement();
        let lbl = self.loop_labels.pop().expect("pushed above");
        let body = if lbl.continue_used {
            Stmt::Block {
                label: Some(Ident {
                    name: lbl.continue_label,
                    span: start,
                }),
                decls: Vec::new(),
                stmts: vec![body],
                span: start,
            }
        } else {
            body
        };
        (body, lbl.break_used)
    }

    /// If `break` was used in this loop, wrap the whole loop in a synthetic named
    /// block `begin : $break$<lo> loop end` (its exit is past the loop). No-op
    /// (byte-identical) when `break` was not used.
    fn wrap_break(&self, loop_stmt: Stmt, break_used: bool, start: Span) -> Stmt {
        if break_used {
            Stmt::Block {
                label: Some(Ident {
                    name: format!("$break${}", start.lo),
                    span: start,
                }),
                decls: Vec::new(),
                stmts: vec![loop_stmt],
                span: start,
            }
        } else {
            loop_stmt
        }
    }

    /// `break;` / `continue;` (SV §11.5). Desugars to `disable <synthetic-label>`
    /// of the innermost enclosing loop — `break` jumps past the loop, `continue`
    /// to the loop's continue point. Records that the wrap is needed. Outside any
    /// loop it is a loud error (correct-or-loud). Reuses the proven `disable`→Goto
    /// lowering, so fork-crossing break/continue is loud-rejected at elaborate.
    fn parse_break_continue(&mut self, is_break: bool) -> Stmt {
        let start = self.cur_span();
        self.bump(); // `break` / `continue`
        self.expect(TokenKind::Semi, "';'");
        let span = start.to(self.prev_span());
        match self.loop_labels.last_mut() {
            Some(ctx) => {
                let label = if is_break {
                    ctx.break_used = true;
                    ctx.break_label.clone()
                } else {
                    ctx.continue_used = true;
                    ctx.continue_label.clone()
                };
                Stmt::Disable {
                    target: HierPath {
                        segments: vec![Ident { name: label, span }],
                        span,
                    },
                    span,
                }
            }
            None => {
                self.error(if is_break {
                    "an enclosing loop for this `break`"
                } else {
                    "an enclosing loop for this `continue`"
                });
                Stmt::Error(span)
            }
        }
    }

    fn parse_for(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // for
        self.expect(TokenKind::LParen, "'(' after 'for'");
        // SV §12.7.1: a TYPED loop-variable declaration in the for-init
        // (`for (int i = 0; …)`). When a type keyword leads the init we parse a
        // local `NetVarDecl` and WRAP the whole For in an (unlabeled) block whose
        // `decls` carries the loop-var decl, so `hoist_block_local_nets` flattens
        // it to a module net just like any other block-local. v1 elaborate has NO
        // per-block scoping (block-locals share the module namespace), so a decl
        // named like an outer variable would be SKIPPED and the loop would alias
        // / clobber that outer var (silent-wrong vs iverilog, where the for-init
        // variable is implicitly local). So — exactly as `parse_foreach` does for
        // its index — we rename the loop variable to a synthetic UNIQUE name and
        // rewrite its references inside the for's init/cond/step/body. A nested
        // for / foreach reusing the same name renames ITS subtree first, so this
        // pass only ever sees its own occurrences (the rename helper's block arm
        // also stops at any inner redeclaration). An unlabeled wrapping block
        // lowers byte-identically to lowering the For alone.
        let typed_init = if self.net_var_kind().is_some() {
            self.parse_for_typed_init()
        } else if let Some(info) = self.peek_block_typedef_decl() {
            // SV §12.7.1 typed for-init using a user-defined type name
            // (`for (my_t i=0; …)`). The `<typedef> <ident>` shape is
            // unambiguously a decl (a plain `i = 0` re-uses an existing var and
            // `peek_block_typedef_decl` returns None for it).
            self.parse_for_typed_init_typedef(info)
        } else {
            None
        };
        let init = Box::new(match &typed_init {
            // The synthesized `i = init` assign (typed init always has one).
            Some((_, init_assign, _)) => init_assign.clone(),
            None => self.parse_for_assign(), // `i = 0`, no trailing ';'
        });
        self.expect(TokenKind::Semi, "';' after for-init");
        let cond = self.expr(0);
        self.expect(TokenKind::Semi, "';' after for-cond");
        let step = Box::new(self.parse_for_assign()); // `i = i+1`, no trailing ';'
        self.expect(TokenKind::RParen, "')'");
        let (body, break_used) = self.parse_loop_body(start);
        let span = start.to(self.prev_span());

        let mut for_stmt = Stmt::For {
            init,
            cond,
            step,
            body: Box::new(body),
            span,
        };

        let built = if let Some((decl, _, orig_name)) = typed_init {
            // Rewrite every reference to the original loop-var name across the
            // whole For → the synthetic name. The For's `init` was synthesized to
            // already carry the synthetic name (no `orig_name` occurrences), so the
            // rename only rebinds cond/step/body. `rename_ident_in_stmt`'s block
            // arm stops at any inner redeclaration, so a nested block/loop that
            // shadows the name keeps its own binding. (The synthetic `$continue$`
            // block has no decls, so the rename descends through it.)
            let synth = decl.names[0].name.name.clone();
            rename_ident_in_stmt(&mut for_stmt, &orig_name.name, &synth);
            Stmt::Block {
                label: None,
                decls: vec![decl],
                stmts: vec![for_stmt],
                span,
            }
        } else {
            for_stmt
        };
        self.wrap_break(built, break_used, start)
    }

    /// SV §12.7.1 typed for-init: `int i = 0` (or `integer` / `byte` /
    /// `logic [3:0]` / …). Parses the loop-variable declaration WITHOUT consuming
    /// the trailing `;` (the for-clause owns that). The declared variable is given
    /// a synthetic UNIQUE name (`__forvar_<name>_<span>`) so it never aliases a
    /// same-named outer var under v1's flat block-local namespace; the original
    /// name is returned so `parse_for` can rewrite the cond/step/body references.
    /// Returns the renamed `NetVarDecl`, the synthesized `i = init` blocking
    /// assign (already pointing at the synthetic name), and the ORIGINAL `Ident`.
    /// `None` when there is no `=` initializer to seed the loop (loud).
    fn parse_for_typed_init(&mut self) -> Option<(NetVarDecl, Stmt, Ident)> {
        let start = self.cur_span();
        let kind = self.net_var_kind().unwrap();
        self.bump(); // the type keyword
        let signed = self.signed_eff(Some(kind));
        let range = self.opt_range();
        let packed = self.opt_packed_dims();
        self.build_for_typed_init(start, kind, signed, range, packed)
    }

    /// SV §12.7.1 typed for-init with a USER-DEFINED type name (`for (my_t i=0; …)`
    /// where `my_t` is a typedef/enum/struct). The resolved `TypeInfo` supplies
    /// kind/sign/range/packed — the type NAME is a single token (no inline range),
    /// so unlike the built-in path there is nothing to `opt_range`/`opt_packed`.
    /// Mirrors the built-in arm; reuses the SAME `<T> <name>` disambiguation as
    /// block-local typedef decls and function-return typedefs (§4.5.40/§4.5.41).
    fn parse_for_typed_init_typedef(
        &mut self,
        info: TypeInfo,
    ) -> Option<(NetVarDecl, Stmt, Ident)> {
        let start = self.cur_span(); // type-name token span (matches the built-in path)
        self.bump(); // the type-name identifier
                     // A class-handle alias cannot be a loop counter (no arithmetic on a
                     // handle, §8.4). Emit a clear loud error rather than letting a
                     // ClassHandle decl with no class type flow to elaborate (which would
                     // report the misleading "class handle without a class type" + a
                     // cascade). The type name is already consumed, so returning None
                     // lets the plain-assign fallback parse the `name = init`; elaborate's
                     // single follow-on is then the (correct) undeclared-loop-var error.
        if info.class_name.is_some() {
            // Phrased to fit the parser's "expected <X>, found <token>" template
            // (the `found` token names the offending class).
            self.error("a non-class type for the for-loop variable (a class handle has no loop arithmetic)");
            return None;
        }
        self.build_for_typed_init(start, info.kind, info.signed, info.range, info.packed)
    }

    /// Shared tail of typed for-init parsing: read the single loop-variable
    /// declarator + `= init`, rename it to a synthetic unique name (so it never
    /// aliases a same-named outer var under v1's flat block-local namespace), and
    /// synthesize `(renamed decl, `i = init` assign, ORIGINAL ident)`.
    fn build_for_typed_init(
        &mut self,
        start: Span,
        kind: NetVarKind,
        signed: bool,
        range: Option<Range>,
        packed: Vec<Range>,
    ) -> Option<(NetVarDecl, Stmt, Ident)> {
        // SV §12.7.1 allows ONE loop variable; parse a single declarator (no
        // comma-list) so a stray comma stays a loud error rather than being
        // silently swallowed as a second for-init variable.
        let n_start = self.cur_span();
        let orig = self.ident()?;
        let init_expr = if self.eat(TokenKind::Eq) {
            self.expr(0)
        } else {
            // No initializer — the For has no defined start value. Emit loud and
            // bail rather than fabricate a `0`; the caller's `None` arm then falls
            // back to the plain-assign path (which errors on the leftover token).
            self.error("'=' initializer in a for-loop variable declaration");
            return None;
        };
        let synth = Ident {
            name: format!("__forvar_{}_{}", orig.name, start.lo),
            span: orig.span,
        };
        let decl_span = start.to(self.prev_span());
        let decl = NetVarDecl {
            kind,
            signed,
            range,
            packed,
            delay: None,
            names: vec![DeclName {
                name: synth.clone(),
                unpacked: Vec::new(),
                init: None, // seeded via the synthesized init assign below, not a static var-init
                span: n_start.to(self.prev_span()),
            }],
            lifetime: None,
            class_type: None,
            class_args: Vec::new(),
            const_param: false,
            span: decl_span,
        };
        let init_assign = Stmt::Blocking {
            lhs: Lvalue::Ident(HierPath {
                segments: vec![synth.clone()],
                span: synth.span,
            }),
            delay: None,
            event: None,
            rhs: init_expr,
            span: n_start.to(self.prev_span()),
        };
        Some((decl, init_assign, orig))
    }

    /// A single blocking assignment WITHOUT a trailing `;` (for-init / for-step).
    fn parse_for_assign(&mut self) -> Stmt {
        let start = self.cur_span();
        // for-init / for-step may use the SV §11.4 shorthands: `++i`, `i++`,
        // `i += n`. Prefix form first (leads with the operator), then the lvalue
        // forms (postfix / compound), else a plain `i = e`.
        if matches!(
            self.peek(),
            Some(TokenKind::PlusPlus) | Some(TokenKind::MinusMinus)
        ) {
            return self.parse_pre_incdec(start);
        }
        let lhs = self.parse_lvalue();
        if let Some(stmt) = self.try_compound_assign(&lhs, start) {
            return stmt;
        }
        self.expect(TokenKind::Eq, "'=' in for-clause assignment");
        let rhs = self.expr(0);
        let rhs = self.maybe_struct_pattern_rhs(&lhs, rhs);
        Stmt::Blocking {
            lhs,
            delay: None,
            event: None,
            rhs,
            span: start.to(self.prev_span()),
        }
    }

    fn parse_while(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // while
        self.expect(TokenKind::LParen, "'(' after 'while'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let (body, break_used) = self.parse_loop_body(start);
        let loop_stmt = Stmt::While {
            cond,
            body: Box::new(body),
            span: start.to(self.prev_span()),
        };
        self.wrap_break(loop_stmt, break_used, start)
    }

    /// P2-E: `do body while (cond);` desugars at parse to
    /// `begin body; while (cond) body end` — the body runs once before the
    /// first test (body CLONE; loops with side-effecting macro-expanded
    /// bodies are identical either way since both copies are the same AST).
    fn parse_do_while(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // do
                     // break/continue target THIS do-while (not an enclosing loop). The body
                     // is `$continue`-wrapped if needed; BOTH desugar copies (the once-run body
                     // and the while body) carry the same wrap — each is lowered separately so
                     // the disable stack resolves to the right copy's exit. `continue` in the
                     // first body falls through to the `while` (re-tests cond); in the while
                     // body it hits the back-edge. `break` wraps the whole desugar.
        let (body, break_used) = self.parse_loop_body(start);
        if !self.at_kw(Kw::While) {
            self.error("'while' after a do-body");
            return Stmt::Error(start.to(self.prev_span()));
        }
        self.bump(); // while
        self.expect(TokenKind::LParen, "'(' after 'while'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        self.expect(TokenKind::Semi, "';' after do-while");
        let span = start.to(self.prev_span());
        let again = Stmt::While {
            cond,
            body: Box::new(body.clone()),
            span,
        };
        let block = Stmt::Block {
            label: None,
            decls: Vec::new(),
            stmts: vec![body, again],
            span,
        };
        self.wrap_break(block, break_used, start)
    }

    /// P2-E: `unique`/`priority` qualified if/case. The qualified statement
    /// parses normally; the VIOLATION surface (IEEE §12.4.2/§12.5.3 — no
    /// branch/arm taken) desugars to a synthesized `$warning` else/default
    /// arm (iverilog-pinned text class: "value is unhandled..."). A statement
    /// that already HAS an else/default cannot miss — left untouched. The
    /// multi-match uniqueness check is a documented cut (the lowered cascade
    /// is first-match-wins, so overlap is unobservable).
    fn parse_unique_priority(&mut self) -> Stmt {
        let qspan = self.cur_span();
        // §12.4.2: the `0` variants keep the multi-match intent but SUPPRESS
        // the no-match violation — so they parse as the PLAIN if/case with no
        // synthetic warn injection (hand-IEEE: Icarus rejects `unique0 if`
        // outright and ignores the unique/unique0 distinction on case).
        let suppress_no_match = matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Unique0 | Kw::Priority0
            )))
        );
        self.bump(); // unique / priority / unique0 / priority0
        let warn_stmt = |span: Span| Stmt::SysTaskCall {
            name: Ident {
                name: "$warning".to_string(),
                span,
            },
            args: vec![Expr {
                kind: ExprKind::StrLit {
                    raw: "\"value is unhandled for priority or unique case statement\"".to_string(),
                },
                span,
            }],
            span,
        };
        match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::If))) => {
                let mut s = self.parse_if();
                if let Stmt::If { else_s, span, .. } = &mut s {
                    if else_s.is_none() && !suppress_no_match {
                        *else_s = Some(Box::new(warn_stmt(*span)));
                    }
                }
                s
            }
            Some(TokenKind::Word(WordKind::Keyword(k @ (Kw::Case | Kw::Casez | Kw::Casex)))) => {
                let kind = match k {
                    Kw::Casez => CaseKind::Casez,
                    Kw::Casex => CaseKind::Casex,
                    _ => CaseKind::Case,
                };
                let mut s = self.parse_case(kind);
                if let Stmt::Case { items, span, .. } = &mut s {
                    let has_default = items.iter().any(|i| matches!(i, CaseItem::Default { .. }));
                    if !has_default && !suppress_no_match {
                        items.push(CaseItem::Default {
                            body: Box::new(warn_stmt(*span)),
                            span: *span,
                        });
                    }
                }
                s
            }
            _ => {
                self.error("'if' or 'case' after a unique/priority qualifier");
                Stmt::Error(qspan.to(self.prev_span()))
            }
        }
    }

    /// v5 ⑥ follow-on, reworked at v6: `foreach (arr[i]) stmt` — PARSE-TIME
    /// desugar to the uniform first/next walk (no new AST/IR node):
    ///   begin : (anon)  integer i; integer __st;
    ///     __st = arr.first(i);
    ///     while (__st == 1) begin stmt  __st = arr.next(i); end
    ///   end
    /// ONE shape serves every dyn kind: elaborate lowers first/next on
    /// dyn/queue handles to the DENSE 0..size-1 walk (synthetic-index gated —
    /// the user surface keeps them assoc-only) and on assoc handles to the
    /// key-order walk (§7.9.4). A status of −1 (key wider than the integer
    /// index — possible on i64/string-keyed assoc) stops the loop with the
    /// engine's W4020 truncation warn. Anything that is not a dyn handle gets
    /// the method-call loud error at elaborate.
    /// Multi-index foreach (`a[i,j]`) is outside the MVP — loud at parse.
    fn parse_foreach(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // foreach
        self.expect(TokenKind::LParen, "'(' after 'foreach'");
        let Some(arr) = self.ident() else {
            self.error("an array name in 'foreach (name[index])'");
            return Stmt::Error(start);
        };
        self.expect(TokenKind::LBracket, "'['");
        let Some(ivar) = self.ident() else {
            self.error("a loop-index name in 'foreach (name[index])'");
            return Stmt::Error(start);
        };
        if self.peek() == Some(TokenKind::Comma) {
            // Multi-dimension foreach (IEEE §12.7.3): `foreach (a[i,j,…])`. `ivar`
            // is the already-parsed first index (dimension 1); collect the rest and
            // build the dimension-tagged nested desugar. (An empty LEADING slot
            // `foreach(a[,j])` is not handled here — `self.ident()` above already
            // loud-rejected the leading comma; that rare form stays honest-loud.)
            return self.parse_multidim_foreach(start, arr, ivar);
        }
        self.expect(TokenKind::RBracket, "']'");
        self.expect(TokenKind::RParen, "')'");
        // break/continue target THIS foreach. `continue` wraps the user body in
        // `$continue`, whose exit falls through to the `__st = next` advance (so
        // the iterator still advances after a continue); `break` wraps the
        // synthesized while below.
        let (mut body, break_used) = self.parse_loop_body(start);
        let span = start.to(self.prev_span());
        // v1 elaborate FLATTENS block-locals into the module namespace (no
        // per-block scoping), so a decl named like an outer variable would be
        // skipped and the loop would CLOBBER the outer one (silent-wrong vs
        // IEEE/iverilog, where the foreach index is implicitly local). Make
        // the index a synthetic unique name and rename its references inside
        // the body instead — correct shadowing with zero scoping support.
        // (A nested foreach reusing the same index name renames ITS body
        // first, so the outer pass only sees its own occurrences.)
        let synth = Ident {
            name: format!("__foreach_{}_{}", ivar.name, start.lo),
            span: ivar.span,
        };
        rename_ident_in_stmt(&mut body, &ivar.name, &synth.name);
        let ivar = synth;
        let one_seg = |id: &Ident| HierPath {
            segments: vec![id.clone()],
            span: id.span,
        };
        let ivar_expr = |id: &Ident| Expr {
            kind: ExprKind::Ident(one_seg(id)),
            span: id.span,
        };
        // synthetic status var (unique like the index — same collision rules).
        let stvar = Ident {
            name: format!("__foreach_st_{}", start.lo),
            span: ivar.span,
        };
        // __st = arr.first(i) / arr.next(i)
        let iter_call = |method: &str| Expr {
            kind: ExprKind::Call {
                name: HierPath {
                    segments: vec![
                        arr.clone(),
                        Ident {
                            name: method.to_string(),
                            span: arr.span,
                        },
                    ],
                    span: arr.span,
                },
                args: vec![ivar_expr(&ivar)],
            },
            span: arr.span,
        };
        let st_assign = |method: &str| Stmt::Blocking {
            lhs: Lvalue::Ident(one_seg(&stvar)),
            delay: None,
            event: None,
            rhs: iter_call(method),
            span,
        };
        // while (__st == 1) — a −1 truncation status stops the walk (W4020
        // already warned at the engine seam).
        let cond = Expr {
            kind: ExprKind::Binary {
                op: BinOp::Eq,
                lhs: Box::new(ivar_expr(&stvar)),
                rhs: Box::new(Self::dec_lit(1, span)),
            },
            span,
        };
        let loop_body = Stmt::Block {
            label: None,
            decls: Vec::new(),
            stmts: vec![body, st_assign("next")],
            span,
        };
        // block-local `integer i; integer __st;` so neither leaks/collides.
        let decl_of = |id: &Ident| NetVarDecl {
            kind: NetVarKind::Integer,
            signed: true,
            range: None,
            packed: Vec::new(),
            delay: None,
            names: vec![DeclName {
                name: id.clone(),
                unpacked: Vec::new(),
                init: None,
                span: id.span,
            }],
            lifetime: None,
            class_type: None,
            class_args: Vec::new(),
            const_param: false,
            span: id.span,
        };
        Stmt::Block {
            label: None, // the synthetic names need no block scope
            decls: vec![decl_of(&ivar), decl_of(&stvar)],
            stmts: vec![
                st_assign("first"),
                self.wrap_break(
                    Stmt::While {
                        cond,
                        body: Box::new(loop_body),
                        span,
                    },
                    break_used,
                    start,
                ),
            ],
            span,
        }
    }

    /// Multi-dimension `foreach (a[i,j,…])` (IEEE §12.7.3). `ivar0` is the already-
    /// parsed first index (dimension 1). Collects the remaining comma-separated
    /// slots (a named index iterates its 1-indexed dimension; an EMPTY slot leaves
    /// that dimension un-iterated), parses the loop body, and builds the nested
    /// per-dimension walk. Row-major: the leftmost named index is the OUTERMOST
    /// loop; `break`/`continue` target the innermost dimension's loop (matching the
    /// iverilog nested-for desugar).
    fn parse_multidim_foreach(&mut self, start: Span, arr: Ident, ivar0: Ident) -> Stmt {
        let mut named: Vec<(Ident, u32)> = vec![(ivar0, 1)];
        let mut dim: u32 = 1;
        while self.peek() == Some(TokenKind::Comma) {
            self.bump();
            dim += 1;
            // An empty slot (next is `,` or `]`) leaves this dimension un-iterated.
            if matches!(
                self.peek(),
                Some(TokenKind::Comma) | Some(TokenKind::RBracket)
            ) {
                continue;
            }
            if let Some(id) = self.ident() {
                // Each foreach index must be a distinct name (it declares an
                // implicit loop variable). A repeat (`foreach(a[i,i])`) is illegal —
                // iverilog rejects it, and silently aliasing both levels to one net
                // would mis-iterate. Loud here.
                if named.iter().any(|(e, _)| e.name == id.name) {
                    self.error_at(
                        id.span,
                        "a distinct name for each foreach index (this one repeats an earlier index)",
                    );
                    return Stmt::Error(start);
                }
                named.push((id, dim));
            } else {
                self.error("a loop-index name in 'foreach (name[index])'");
                return Stmt::Error(start);
            }
        }
        self.expect(TokenKind::RBracket, "']'");
        self.expect(TokenKind::RParen, "')'");
        let (body, break_used) = self.parse_loop_body(start);
        let span = start.to(self.prev_span());
        self.build_multi_foreach(&arr, &named, body, break_used, start, span)
    }

    /// Build the nested per-dimension walk for a multi-dimension `foreach`. Each
    /// named index `(idx, dim)` becomes one `while` level whose status var is driven
    /// by `arr.first/next(idx, dim)` — the 2nd (dimension) argument routes elaborate
    /// to the right unpacked dimension's bounds. Loops nest leftmost-outermost; the
    /// innermost carries `wrap_break` so `break`/`continue` affect only it.
    fn build_multi_foreach(
        &mut self,
        arr: &Ident,
        named: &[(Ident, u32)],
        mut body: Stmt,
        break_used: bool,
        start: Span,
        span: Span,
    ) -> Stmt {
        // Each user index → a unique synthetic name (implicit-local, shadow-correct
        // under v1's flat namespace; a nested foreach reusing a name renamed ITS
        // body first, so this pass only sees its own occurrences).
        let synths: Vec<(Ident, u32)> = named
            .iter()
            .map(|(id, d)| {
                let s = Ident {
                    name: format!("__foreach_{}_{}", id.name, start.lo),
                    span: id.span,
                };
                rename_ident_in_stmt(&mut body, &id.name, &s.name);
                (s, *d)
            })
            .collect();
        let one_seg = |id: &Ident| HierPath {
            segments: vec![id.clone()],
            span: id.span,
        };
        let decl_of = |id: &Ident| NetVarDecl {
            kind: NetVarKind::Integer,
            signed: true,
            range: None,
            packed: Vec::new(),
            delay: None,
            names: vec![DeclName {
                name: id.clone(),
                unpacked: Vec::new(),
                init: None,
                span: id.span,
            }],
            lifetime: None,
            class_type: None,
            class_args: Vec::new(),
            const_param: false,
            span: id.span,
        };
        let mut decls: Vec<NetVarDecl> = Vec::new();
        let mut cur: Stmt = body;
        let n = synths.len();
        for (lvl, (idx, d)) in synths.iter().enumerate().rev() {
            let innermost = lvl + 1 == n;
            let stvar = Ident {
                name: format!("__foreach_st{}_{}", lvl, start.lo),
                span: idx.span,
            };
            // `arr.first/next(idx, dim)` — args[1] tags the dimension for elaborate.
            let iter_call = |method: &str| Expr {
                kind: ExprKind::Call {
                    name: HierPath {
                        segments: vec![
                            arr.clone(),
                            Ident {
                                name: method.to_string(),
                                span: arr.span,
                            },
                        ],
                        span: arr.span,
                    },
                    args: vec![
                        Expr {
                            kind: ExprKind::Ident(one_seg(idx)),
                            span: idx.span,
                        },
                        Self::dec_lit(*d, span),
                    ],
                },
                span: arr.span,
            };
            let st_assign = |method: &str| Stmt::Blocking {
                lhs: Lvalue::Ident(one_seg(&stvar)),
                delay: None,
                event: None,
                rhs: iter_call(method),
                span,
            };
            let cond = Expr {
                kind: ExprKind::Binary {
                    op: BinOp::Eq,
                    lhs: Box::new(Expr {
                        kind: ExprKind::Ident(one_seg(&stvar)),
                        span: stvar.span,
                    }),
                    rhs: Box::new(Self::dec_lit(1, span)),
                },
                span,
            };
            let loop_body = Stmt::Block {
                label: None,
                decls: Vec::new(),
                stmts: vec![cur, st_assign("next")],
                span,
            };
            let while_stmt = Stmt::While {
                cond,
                body: Box::new(loop_body),
                span,
            };
            let wrapped = if innermost {
                self.wrap_break(while_stmt, break_used, start)
            } else {
                while_stmt
            };
            cur = Stmt::Block {
                label: None,
                decls: Vec::new(),
                stmts: vec![st_assign("first"), wrapped],
                span,
            };
            decls.push(decl_of(idx));
            decls.push(decl_of(&stvar));
        }
        Stmt::Block {
            label: None,
            decls,
            stmts: vec![cur],
            span,
        }
    }

    fn parse_repeat(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // repeat
        self.expect(TokenKind::LParen, "'(' after 'repeat'");
        let count = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let (body, break_used) = self.parse_loop_body(start);
        let loop_stmt = Stmt::Repeat {
            count,
            body: Box::new(body),
            span: start.to(self.prev_span()),
        };
        self.wrap_break(loop_stmt, break_used, start)
    }

    fn parse_forever(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // forever — NO parens, NO count
        let (body, break_used) = self.parse_loop_body(start);
        let loop_stmt = Stmt::Forever {
            body: Box::new(body),
            span: start.to(self.prev_span()),
        };
        self.wrap_break(loop_stmt, break_used, start)
    }

    // ─────────────────────── 5. blocks ───────────────────────
    fn parse_seq_block(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // begin
        let label = self.opt_block_label();
        let (decls, stmts) = self.block_body(BlockEnd::End);
        self.expect(TokenKind::Word(WordKind::Keyword(Kw::End)), "'end'");
        self.opt_block_label(); // optional `: end_label` (no AST slot → discard)
        Stmt::Block {
            label,
            decls,
            stmts,
            span: start.to(self.prev_span()),
        }
    }

    fn parse_par_block(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // fork
        let label = self.opt_block_label();
        let (decls, stmts) = self.block_body(BlockEnd::Join);
        let join = self.eat_join(); // Join | JoinAny | JoinNone (latter two are Idents)
        self.opt_block_label(); // optional `: join_label`
        Stmt::Fork {
            label,
            decls,
            stmts,
            join,
            span: start.to(self.prev_span()),
        }
    }

    /// `: name` after begin/fork (or end/join) → Some(ident), else None.
    fn opt_block_label(&mut self) -> Option<Ident> {
        if self.eat(TokenKind::Colon) {
            self.ident()
        } else {
            None
        }
    }

    /// Shared block body: decls-prefix THEN statements, until the closer.
    fn block_body(&mut self, end: BlockEnd) -> (Vec<NetVarDecl>, Vec<Stmt>) {
        let mut decls = Vec::new();
        // A block is lexically scoped: snapshot the scope registries at the block's
        // first body-local typedef DEFINITION or struct/enum-typed VAR decl, and
        // restore them when the block ends, so a local name does not leak out of /
        // clobber an outer one. `None` until such a decl appears → zero overhead
        // for the common (plain-var) block.
        let mut scope: Option<ScopeSnapshot> = None;
        while !self.at_eof() && !self.at_block_end(end) {
            let before = self.pos;
            if self.at_kw(Kw::Typedef) {
                if scope.is_none() {
                    scope = Some(self.snapshot_scope());
                }
                self.parse_body_typedef_def();
            } else if self.net_var_kind().is_some() {
                if let Some(d) = self.parse_net_var(false) {
                    // procedural block-local decl: no net delay
                    decls.push(d);
                }
            } else if let Some(info) = self.peek_block_typedef_decl() {
                // A procedural block-local declaration using a user-defined type
                // name (`my_enum_t state = IDLE;` / `byte_t b;` / `s_t s;`),
                // mirroring the module-item typedef-decl path. This writes the
                // VAR-name-keyed maps, so it too triggers the scope snapshot — a
                // block-local `s_t x` that shadows an outer `s_t x` must not leak
                // its layout binding out of the block.
                if scope.is_none() {
                    scope = Some(self.snapshot_scope());
                }
                if let Some(d) = self.parse_typed_decl(info) {
                    decls.push(d);
                }
            } else {
                break; // not a declaration → the statement region begins
            }
            if self.pos == before {
                self.bump(); // guard: malformed decl that consumed nothing
            }
        }
        let mut stmts = Vec::new();
        while !self.at_eof() && !self.at_block_end(end) {
            let before = self.pos;
            stmts.push(self.parse_statement());
            if self.pos == before {
                self.bump(); // guard: never spin on a stuck statement
            }
        }
        // Drop block-local typedefs / struct-var bindings (restore the outer scope)
        // AFTER statements are parsed — a statement may reference a local typedef
        // (e.g. a cast `t'(x)`) or a local struct var's `x.field`.
        if let Some(scope) = scope {
            self.restore_scope(scope);
        }
        (decls, stmts)
    }

    /// True at this block's closer. `End` for begin; any join form for fork.
    fn at_block_end(&self, end: BlockEnd) -> bool {
        match end {
            BlockEnd::End => self.at_kw(Kw::End),
            BlockEnd::Join => {
                self.at_kw(Kw::Join)
                    || (self.is_ident() && matches!(self.cur_text(), "join_any" | "join_none"))
            }
        }
    }

    /// Consume the fork terminator → JoinKind.
    fn eat_join(&mut self) -> JoinKind {
        if self.eat_kw(Kw::Join) {
            JoinKind::Join
        } else if self.is_ident() && self.cur_text() == "join_any" {
            self.bump();
            JoinKind::JoinAny
        } else if self.is_ident() && self.cur_text() == "join_none" {
            self.bump();
            JoinKind::JoinNone
        } else {
            self.error("'join' / 'join_any' / 'join_none'");
            JoinKind::Join
        }
    }

    // ─────────────────────── 6. timing / event / misc ───────────────────────
    fn parse_delay_stmt(&mut self) -> Stmt {
        let start = self.cur_span();
        let delay = self.parse_delay().unwrap_or(Delay {
            values: Vec::new(),
            span: start,
        });
        let body = if self.eat(TokenKind::Semi) {
            None
        } else {
            Some(Box::new(self.parse_statement()))
        };
        Stmt::DelayCtrl {
            delay,
            body,
            span: start.to(self.prev_span()),
        }
    }

    fn parse_event_stmt(&mut self) -> Stmt {
        let start = self.cur_span();
        let ctrl = self.parse_sensitivity(); // consumes the `@`
        let body = if self.eat(TokenKind::Semi) {
            None
        } else {
            Some(Box::new(self.parse_statement()))
        };
        Stmt::EventCtrl {
            ctrl,
            body,
            span: start.to(self.prev_span()),
        }
    }

    fn parse_trigger_stmt(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // '->'
                     // H1: on a missing name, emit Stmt::Error rather than an empty path.
        let Some(name) = self.hier_path() else {
            return self.stmt_error_at(start);
        };
        self.expect(TokenKind::Semi, "';'");
        Stmt::EventTrigger {
            name,
            span: start.to(self.prev_span()),
        }
    }

    fn parse_wait(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // wait
                     // `wait fork;` — `fork` is `Kw::Fork`, not an ident, so special-case it
                     // before the `wait(expr)` path (mirrors `parse_disable`).
        if self.at_kw(Kw::Fork) {
            self.bump(); // fork
            self.expect(TokenKind::Semi, "';'");
            return Stmt::WaitFork {
                span: start.to(self.prev_span()),
            };
        }
        self.expect(TokenKind::LParen, "'(' after 'wait'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let body = if self.eat(TokenKind::Semi) {
            None
        } else {
            Some(Box::new(self.parse_statement()))
        };
        Stmt::Wait {
            cond,
            body,
            span: start.to(self.prev_span()),
        }
    }

    fn parse_disable(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // disable
                     // M3: `disable fork;` — `fork` is `Kw::Fork`, not an ident, so special-case it.
        if self.at_kw(Kw::Fork) {
            let fspan = self.cur_span();
            self.bump(); // fork
            let seg = Ident {
                name: "fork".to_string(),
                span: fspan,
            };
            let target = HierPath {
                segments: vec![seg],
                span: fspan,
            };
            self.expect(TokenKind::Semi, "';'");
            return Stmt::Disable {
                target,
                span: start.to(self.prev_span()),
            };
        }
        // H1: on a missing/illegal name, emit Stmt::Error rather than an empty path.
        let Some(target) = self.hier_path() else {
            return self.stmt_error_at(start);
        };
        self.expect(TokenKind::Semi, "';'");
        Stmt::Disable {
            target,
            span: start.to(self.prev_span()),
        }
    }
}

/// Which keyword terminates a block body (begin→end, fork→join family).
#[derive(Clone, Copy)]
enum BlockEnd {
    End,
    Join,
}

/// Closer selector for function/task bodies (mirrors `BlockEnd` for begin/fork).
#[derive(Clone, Copy)]
enum BlockEnd2 {
    Endfunction,
    Endtask,
}

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

/// v5 ⑥ foreach desugar: rename every SINGLE-SEGMENT `Ident` reference to
/// `from` into `to`, across a statement tree — exprs, lvalues, nested stmts,
/// block-local decl initializers/dims AND event-control sensitivity exprs
/// (the last two were review finding 2026-06-11: a missed arm silently binds
/// the reference to the OUTER variable). Multi-segment paths are left alone
/// (`x.y` never names the loop index).
// ──────────── ⓑ-breadth (§8.25): parameterized-class monomorphization ────────
// Substitute class value parameters (`name → value-expr`) throughout a class's
// declarations so each specialization is a fully-concrete class. Coverage is the
// procedural/declarative subset a class body uses; any un-covered position simply
// keeps the parameter name, which then fails LOUD at elaborate (undeclared) — never
// a silent miscompile.

/// Replace every single-segment `Ident` matching a key in `map` with that key's
/// value-expression (cloned). Recurses through the full `ExprKind` closure.
fn subst_expr(e: &mut Expr, map: &std::collections::BTreeMap<String, Expr>) {
    match &mut e.kind {
        ExprKind::Ident(p) => {
            if p.segments.len() == 1 {
                if let Some(v) = map.get(&p.segments[0].name) {
                    let span = e.span;
                    *e = v.clone();
                    e.span = span;
                }
            }
        }
        ExprKind::Unary { operand, .. } => subst_expr(operand, map),
        ExprKind::Binary { lhs, rhs, .. } => {
            subst_expr(lhs, map);
            subst_expr(rhs, map);
        }
        ExprKind::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            subst_expr(cond, map);
            subst_expr(then_e, map);
            subst_expr(else_e, map);
        }
        ExprKind::BitSelect { base, index } => {
            subst_expr(base, map);
            subst_expr(index, map);
        }
        ExprKind::PartSelect { base, msb, lsb } => {
            subst_expr(base, map);
            subst_expr(msb, map);
            subst_expr(lsb, map);
        }
        ExprKind::IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            subst_expr(base, map);
            subst_expr(offset, map);
            subst_expr(width, map);
        }
        ExprKind::Concat { parts } => {
            for p in parts {
                subst_expr(p, map);
            }
        }
        ExprKind::AssignPattern(elems) => {
            for el in elems {
                subst_expr(el, map);
            }
        }
        ExprKind::Replicate { count, value } => {
            subst_expr(count, map);
            for v in value {
                subst_expr(v, map);
            }
        }
        ExprKind::Call { args, .. } | ExprKind::SysCall { args, .. } => {
            for a in args {
                subst_expr(a, map);
            }
        }
        ExprKind::RandomizeWith(b) => {
            for a in b.args.iter_mut().chain(b.constraints.iter_mut()) {
                subst_expr(a, map);
            }
        }
        ExprKind::ArrayMethodWith(b) => subst_expr(&mut b.with_expr, map),
        ExprKind::Paren { inner } => subst_expr(inner, map),
        ExprKind::MinTypMax { min, typ, max } => {
            subst_expr(min, map);
            subst_expr(typ, map);
            subst_expr(max, map);
        }
        ExprKind::New { size, src } => {
            subst_expr(size, map);
            if let Some(s) = src {
                subst_expr(s, map);
            }
        }
        ExprKind::ClassNew { args } => {
            for a in args {
                subst_expr(a, map);
            }
        }
        ExprKind::Dist { value, items } => {
            subst_expr(value, map);
            for it in items {
                subst_expr(&mut it.lo, map);
                if let Some(h) = &mut it.hi {
                    subst_expr(h, map);
                }
                subst_expr(&mut it.weight, map);
            }
        }
        ExprKind::Cast { target, expr } => {
            // Substitute the operand AND (for a size cast) the width expression —
            // a param-width cast `WIDTH'(x)` must monomorphize the WIDTH too.
            if let CastTarget::Size(w) = target {
                subst_expr(w, map);
            }
            subst_expr(expr, map);
        }
        ExprKind::IntLit { .. }
        | ExprKind::RealLit { .. }
        | ExprKind::StrLit { .. }
        | ExprKind::PkgScoped { .. }
        | ExprKind::Null
        | ExprKind::Dollar
        | ExprKind::Error => {}
    }
}

fn subst_opt_expr(e: &mut Option<Expr>, map: &std::collections::BTreeMap<String, Expr>) {
    if let Some(x) = e {
        subst_expr(x, map);
    }
}

fn subst_range(r: &mut Option<Range>, map: &std::collections::BTreeMap<String, Expr>) {
    if let Some(rng) = r {
        subst_expr(&mut rng.msb, map);
        subst_expr(&mut rng.lsb, map);
    }
}

fn subst_netvar(d: &mut NetVarDecl, map: &std::collections::BTreeMap<String, Expr>) {
    subst_range(&mut d.range, map);
    for p in &mut d.packed {
        subst_expr(&mut p.msb, map);
        subst_expr(&mut p.lsb, map);
    }
    for n in &mut d.names {
        for dim in &mut n.unpacked {
            match dim {
                Dim::Range(rg) => {
                    subst_expr(&mut rg.msb, map);
                    subst_expr(&mut rg.lsb, map);
                }
                Dim::Size(e) => subst_expr(e, map),
                Dim::Dyn | Dim::Queue(_) | Dim::Assoc(_) => {}
            }
        }
        subst_opt_expr(&mut n.init, map);
    }
}

/// Substitute params through the common procedural statement forms a class method
/// body uses. Un-covered forms keep the parameter (→ loud at elaborate).
fn subst_stmt(s: &mut Stmt, map: &std::collections::BTreeMap<String, Expr>) {
    match s {
        Stmt::Blocking { rhs, .. } | Stmt::NonBlocking { rhs, .. } => subst_expr(rhs, map),
        Stmt::If {
            cond,
            then_s,
            else_s,
            ..
        } => {
            subst_expr(cond, map);
            subst_stmt(then_s, map);
            if let Some(e) = else_s {
                subst_stmt(e, map);
            }
        }
        Stmt::Return { value, .. } => subst_opt_expr(value, map),
        Stmt::Case {
            scrutinee, items, ..
        } => {
            subst_expr(scrutinee, map);
            for it in items {
                match it {
                    CaseItem::Match { labels, body, .. } => {
                        for l in labels {
                            subst_expr(l, map);
                        }
                        subst_stmt(body, map);
                    }
                    CaseItem::Default { body, .. } => subst_stmt(body, map),
                }
            }
        }
        Stmt::For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            subst_stmt(init, map);
            subst_expr(cond, map);
            subst_stmt(step, map);
            subst_stmt(body, map);
        }
        Stmt::While { cond, body, .. }
        | Stmt::Repeat {
            count: cond, body, ..
        } => {
            subst_expr(cond, map);
            subst_stmt(body, map);
        }
        Stmt::Forever { body, .. } => subst_stmt(body, map),
        Stmt::Block { decls, stmts, .. } | Stmt::Fork { decls, stmts, .. } => {
            // A block-local declared with a parameter's name SHADOWS it: its decl
            // ranges/inits still use the outer params, but inside the block that
            // name must NOT be substituted (else a local read silently becomes the
            // parameter value — a silent-wrong).
            for d in decls.iter_mut() {
                subst_netvar(d, map);
            }
            let inner = map_without(
                map,
                decls
                    .iter()
                    .flat_map(|d| d.names.iter().map(|n| &n.name.name)),
            );
            let m = inner.as_ref().unwrap_or(map);
            for st in stmts.iter_mut() {
                subst_stmt(st, m);
            }
        }
        Stmt::SysTaskCall { args, .. } => {
            for a in args {
                subst_expr(a, map);
            }
        }
        Stmt::UserTaskCall { args, .. } => {
            for a in args {
                subst_expr(a, map);
            }
        }
        Stmt::RandomizeWith {
            args, constraints, ..
        } => {
            for a in args.iter_mut().chain(constraints.iter_mut()) {
                subst_expr(a, map);
            }
        }
        _ => {}
    }
}

/// A copy of `map` with `names` removed, or `None` when nothing was removed (so
/// the caller can keep using the original by reference — no allocation in the
/// common no-shadow case).
fn map_without<'a>(
    map: &std::collections::BTreeMap<String, Expr>,
    names: impl Iterator<Item = &'a String>,
) -> Option<std::collections::BTreeMap<String, Expr>> {
    let drop: Vec<&String> = names.filter(|n| map.contains_key(*n)).collect();
    if drop.is_empty() {
        return None;
    }
    let mut m = map.clone();
    for n in drop {
        m.remove(n);
    }
    Some(m)
}

fn subst_class_item(it: &mut ClassItem, map: &std::collections::BTreeMap<String, Expr>) {
    match it {
        ClassItem::Property(_, d) => subst_netvar(d, map),
        ClassItem::RandProperty { decl, .. } => subst_netvar(decl, map),
        ClassItem::Constraint(cd) => {
            for e in &mut cd.exprs {
                subst_expr(e, map);
            }
        }
        ClassItem::Func { def, .. } => {
            subst_range(&mut def.range, map);
            for p in &mut def.ports {
                subst_range(&mut p.range, map);
            }
            for d in &mut def.body_decls {
                subst_netvar(d, map);
            }
            // method args / locals shadow class params inside the body.
            let shadow = def.ports.iter().map(|p| &p.name.name).chain(
                def.body_decls
                    .iter()
                    .flat_map(|d| d.names.iter().map(|n| &n.name.name)),
            );
            let inner = map_without(map, shadow);
            subst_stmt(&mut def.body, inner.as_ref().unwrap_or(map));
        }
        ClassItem::Task { def, .. } => {
            for p in &mut def.ports {
                subst_range(&mut p.range, map);
            }
            for d in &mut def.body_decls {
                subst_netvar(d, map);
            }
            let shadow = def.ports.iter().map(|p| &p.name.name).chain(
                def.body_decls
                    .iter()
                    .flat_map(|d| d.names.iter().map(|n| &n.name.name)),
            );
            let inner = map_without(map, shadow);
            subst_stmt(&mut def.body, inner.as_ref().unwrap_or(map));
        }
        ClassItem::Error(_) => {}
    }
}

/// Build a fully-concrete specialization of a parameterized class: clone the
/// template, substitute the parameter values, clear the param list, and rename.
/// Render a parameterized-class specialization argument to an identifier-safe
/// string for the mangled class name. v1 accepts integer literals (and a leading
/// `-`/parens); anything else returns `None` (→ a loud reject upstream).
fn arg_render(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::IntLit { raw, .. } => {
            let s: String = raw.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        ExprKind::Unary {
            op: UnOp::Minus,
            operand,
        } => arg_render(operand).map(|s| format!("n{s}")),
        ExprKind::Paren { inner } => arg_render(inner),
        _ => None,
    }
}

fn monomorphize_class(
    template: &ClassDecl,
    new_name: &str,
    map: &std::collections::BTreeMap<String, Expr>,
) -> ClassDecl {
    let mut c = template.clone();
    c.name.name = new_name.to_string();
    c.params = Vec::new();
    // A class MEMBER (field/method) named like a parameter shadows it (a degenerate
    // §13.3 collision); the member wins. Excluding such names keeps the field/method
    // working — a width that then references the (shadowed) param is a non-constant
    // ref → loud at elaborate, never a silent miscompile.
    let members = c.items.iter().flat_map(|it| -> Vec<&String> {
        match it {
            ClassItem::Property(_, d) => d.names.iter().map(|n| &n.name.name).collect(),
            ClassItem::RandProperty { decl, .. } => {
                decl.names.iter().map(|n| &n.name.name).collect()
            }
            ClassItem::Func { def, .. } => vec![&def.name.name],
            ClassItem::Task { def, .. } => vec![&def.name.name],
            _ => Vec::new(),
        }
    });
    let eff = map_without(map, members);
    let m = eff.as_ref().unwrap_or(map);
    for it in &mut c.items {
        subst_class_item(it, m);
    }
    c
}

fn rename_ident_in_stmt(s: &mut Stmt, from: &str, to: &str) {
    let fix_path = |p: &mut HierPath| {
        if p.segments.len() == 1 && p.segments[0].name == from {
            p.segments[0].name = to.to_string();
        }
    };
    // SVA sequence antecedent: recurse into every boolean leaf so a loop-index
    // rename reaches sequence terms too (same outer-capture lesson as the
    // EventCtrl/foreach rename arms).
    fn fix_sequence(seq: &mut Sequence, from: &str, to: &str) {
        match seq {
            Sequence::Boolean(e) => fix_expr(e, from, to),
            Sequence::Delay { lhs, rhs, .. } => {
                fix_sequence(lhs, from, to);
                fix_sequence(rhs, from, to);
            }
            Sequence::Repeat { seq, .. } => fix_sequence(seq, from, to),
            Sequence::Throughout { cond, seq } => {
                fix_expr(cond, from, to);
                fix_sequence(seq, from, to);
            }
            Sequence::Within { seq1, seq2 } => {
                fix_sequence(seq1, from, to);
                fix_sequence(seq2, from, to);
            }
            // A re-clocking boundary: recurse into the inner sequence. The clock is a
            // module-level signal (never a loop index — you cannot clock on a genvar),
            // so its sensitivity is not renamed.
            Sequence::Clocked { seq, .. } => fix_sequence(seq, from, to),
            // A named instance: the `name` is a sequence/property identifier (not a
            // loop index), so it is never renamed; only the (reserved) actual-arg
            // expressions are.
            Sequence::Instance { args, .. } => {
                for a in args.iter_mut() {
                    fix_expr(a, from, to);
                }
            }
            // A match-item capture `(b, x = e)`: recurse into the boolean term and the
            // captured value expressions (a generate-loop index may appear in `e`); the
            // local-variable NAMES are not loop indices, so they are not renamed.
            Sequence::MatchItem { seq, assigns } => {
                fix_sequence(seq, from, to);
                for (_, val) in assigns.iter_mut() {
                    fix_expr(val, from, to);
                }
            }
        }
    }
    /// Rename a loop index inside an N2d property-expression tree (foreach desugar
    /// completeness — the antecedent sequences and nested consequents must all be
    /// renamed, mirroring `fix_sequence`). Property/recursion names are
    /// identifiers, not loop indices, so they are not renamed (they parse as bare
    /// `Seq(Boolean(Ident))` leaves and resolve at elaborate).
    fn fix_prop_expr(pe: &mut PropExpr, from: &str, to: &str) {
        match pe {
            PropExpr::Seq(s) => fix_sequence(s, from, to),
            PropExpr::Impl { ante, cons, .. } => {
                fix_sequence(ante, from, to);
                fix_prop_expr(cons, from, to);
            }
            PropExpr::And(l, r) | PropExpr::Or(l, r) => {
                fix_prop_expr(l, from, to);
                fix_prop_expr(r, from, to);
            }
            PropExpr::Not(p) => fix_prop_expr(p, from, to),
            PropExpr::Until { lhs, rhs, .. } => {
                fix_prop_expr(lhs, from, to);
                fix_prop_expr(rhs, from, to);
            }
            PropExpr::Eventually { prop, .. } => fix_prop_expr(prop, from, to),
            PropExpr::Always(p) => fix_prop_expr(p, from, to),
        }
    }
    fn fix_expr(e: &mut Expr, from: &str, to: &str) {
        match &mut e.kind {
            ExprKind::Ident(p) => {
                if p.segments.len() == 1 && p.segments[0].name == from {
                    p.segments[0].name = to.to_string();
                }
            }
            // v7: a package-scoped name can never be the loop index.
            ExprKind::PkgScoped { .. } => {}
            ExprKind::Unary { operand, .. } => fix_expr(operand, from, to),
            ExprKind::Binary { lhs, rhs, .. } => {
                fix_expr(lhs, from, to);
                fix_expr(rhs, from, to);
            }
            ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                fix_expr(cond, from, to);
                fix_expr(then_e, from, to);
                fix_expr(else_e, from, to);
            }
            ExprKind::BitSelect { base, index } => {
                fix_expr(base, from, to);
                fix_expr(index, from, to);
            }
            ExprKind::PartSelect { base, msb, lsb } => {
                fix_expr(base, from, to);
                fix_expr(msb, from, to);
                fix_expr(lsb, from, to);
            }
            ExprKind::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                fix_expr(base, from, to);
                fix_expr(offset, from, to);
                fix_expr(width, from, to);
            }
            ExprKind::Concat { parts } | ExprKind::Replicate { value: parts, .. } => {
                for p in parts {
                    fix_expr(p, from, to);
                }
            }
            ExprKind::AssignPattern(elems) => {
                for el in elems {
                    fix_expr(el, from, to);
                }
            }
            ExprKind::Call { args, .. } | ExprKind::SysCall { args, .. } => {
                for a in args {
                    fix_expr(a, from, to);
                }
            }
            ExprKind::RandomizeWith(b) => {
                for a in b.args.iter_mut().chain(b.constraints.iter_mut()) {
                    fix_expr(a, from, to);
                }
            }
            ExprKind::ArrayMethodWith(b) => fix_expr(&mut b.with_expr, from, to),
            ExprKind::Paren { inner } => fix_expr(inner, from, to),
            ExprKind::MinTypMax { min, typ, max } => {
                fix_expr(min, from, to);
                fix_expr(typ, from, to);
                fix_expr(max, from, to);
            }
            ExprKind::New { size, src } => {
                fix_expr(size, from, to);
                if let Some(s) = src {
                    fix_expr(s, from, to);
                }
            }
            ExprKind::ClassNew { args } => {
                for a in args {
                    fix_expr(a, from, to);
                }
            }
            ExprKind::Dist { value, items } => {
                fix_expr(value, from, to);
                for it in items {
                    fix_expr(&mut it.lo, from, to);
                    if let Some(h) = &mut it.hi {
                        fix_expr(h, from, to);
                    }
                    fix_expr(&mut it.weight, from, to);
                }
            }
            ExprKind::Cast { target, expr } => {
                if let CastTarget::Size(w) = target {
                    fix_expr(w, from, to);
                }
                fix_expr(expr, from, to);
            }
            ExprKind::IntLit { .. }
            | ExprKind::RealLit { .. }
            | ExprKind::StrLit { .. }
            | ExprKind::Null
            | ExprKind::Dollar
            | ExprKind::Error => {}
        }
        // Replicate.count rides outside the parts vec.
        if let ExprKind::Replicate { count, .. } = &mut e.kind {
            fix_expr(count, from, to);
        }
    }
    fn fix_lv(lv: &mut Lvalue, from: &str, to: &str) {
        match lv {
            Lvalue::Ident(p) => {
                if p.segments.len() == 1 && p.segments[0].name == from {
                    p.segments[0].name = to.to_string();
                }
            }
            Lvalue::BitSelect { base, index, .. } => {
                fix_lv(base, from, to);
                fix_expr(index, from, to);
            }
            Lvalue::PartSelect { base, msb, lsb, .. } => {
                fix_lv(base, from, to);
                fix_expr(msb, from, to);
                fix_expr(lsb, from, to);
            }
            Lvalue::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                fix_lv(base, from, to);
                fix_expr(offset, from, to);
                fix_expr(width, from, to);
            }
            Lvalue::Concat { parts, .. } => {
                for p in parts {
                    fix_lv(p, from, to);
                }
            }
            Lvalue::Error(_) => {}
        }
    }
    let fix_delay = |d: &mut Delay, from: &str, to: &str| {
        for e in &mut d.values {
            fix_expr(e, from, to);
        }
    };
    match s {
        Stmt::Blocking {
            lhs, delay, rhs, ..
        }
        | Stmt::NonBlocking {
            lhs, delay, rhs, ..
        } => {
            fix_lv(lhs, from, to);
            if let Some(d) = delay {
                fix_delay(d, from, to);
            }
            fix_expr(rhs, from, to);
        }
        Stmt::If {
            cond,
            then_s,
            else_s,
            ..
        } => {
            fix_expr(cond, from, to);
            rename_ident_in_stmt(then_s, from, to);
            if let Some(e) = else_s {
                rename_ident_in_stmt(e, from, to);
            }
        }
        Stmt::Case {
            scrutinee, items, ..
        } => {
            fix_expr(scrutinee, from, to);
            for it in items {
                match it {
                    CaseItem::Match { labels, body, .. } => {
                        for l in labels {
                            fix_expr(l, from, to);
                        }
                        rename_ident_in_stmt(body, from, to);
                    }
                    CaseItem::Default { body, .. } => rename_ident_in_stmt(body, from, to),
                }
            }
        }
        Stmt::For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            rename_ident_in_stmt(init, from, to);
            fix_expr(cond, from, to);
            rename_ident_in_stmt(step, from, to);
            rename_ident_in_stmt(body, from, to);
        }
        Stmt::While { cond, body, .. } => {
            fix_expr(cond, from, to);
            rename_ident_in_stmt(body, from, to);
        }
        Stmt::Repeat { count, body, .. } => {
            fix_expr(count, from, to);
            rename_ident_in_stmt(body, from, to);
        }
        Stmt::Forever { body, .. } => rename_ident_in_stmt(body, from, to),
        Stmt::Block { decls, stmts, .. } | Stmt::Fork { decls, stmts, .. } => {
            // a nested redeclaration of the SAME name shadows — stop renaming
            // inside (its own occurrences already bind to the inner decl).
            if decls
                .iter()
                .any(|d| d.names.iter().any(|n| n.name.name == from))
            {
                return;
            }
            // decl INITIALIZERS and dimension exprs reference outer names too
            // (review finding 2026-06-11 — they live outside `stmts`).
            for d in decls.iter_mut() {
                if let Some(r) = &mut d.range {
                    fix_expr(&mut r.msb, from, to);
                    fix_expr(&mut r.lsb, from, to);
                }
                for r in &mut d.packed {
                    fix_expr(&mut r.msb, from, to);
                    fix_expr(&mut r.lsb, from, to);
                }
                for n in d.names.iter_mut() {
                    if let Some(e) = &mut n.init {
                        fix_expr(e, from, to);
                    }
                    for dim in &mut n.unpacked {
                        match dim {
                            Dim::Size(e) => fix_expr(e, from, to),
                            Dim::Range(r) => {
                                fix_expr(&mut r.msb, from, to);
                                fix_expr(&mut r.lsb, from, to);
                            }
                            Dim::Queue(Some(b)) => fix_expr(b, from, to),
                            Dim::Queue(None) | Dim::Dyn | Dim::Assoc(_) => {}
                        }
                    }
                }
            }
            for st in stmts {
                rename_ident_in_stmt(st, from, to);
            }
        }
        Stmt::SysTaskCall { args, .. } | Stmt::UserTaskCall { args, .. } => {
            for a in args {
                fix_expr(a, from, to);
            }
        }
        Stmt::RandomizeWith {
            args, constraints, ..
        } => {
            for a in args.iter_mut().chain(constraints.iter_mut()) {
                fix_expr(a, from, to);
            }
        }
        Stmt::DelayCtrl { delay, body, .. } => {
            fix_delay(delay, from, to);
            if let Some(b) = body {
                rename_ident_in_stmt(b, from, to);
            }
        }
        Stmt::EventCtrl { ctrl, body, .. } => {
            // the sensitivity exprs reference names too (review finding
            // 2026-06-11 — `@(arr[i])` inside a foreach body).
            if let Sensitivity::List(evs) = ctrl {
                for ev in evs {
                    fix_expr(&mut ev.expr, from, to);
                }
            }
            if let Some(b) = body {
                rename_ident_in_stmt(b, from, to);
            }
        }
        Stmt::Wait { cond, body, .. } => {
            fix_expr(cond, from, to);
            if let Some(b) = body {
                rename_ident_in_stmt(b, from, to);
            }
        }
        Stmt::Assign { lhs, rhs, .. } | Stmt::Force { lhs, rhs, .. } => {
            fix_lv(lhs, from, to);
            fix_expr(rhs, from, to);
        }
        Stmt::Deassign { lhs, .. } | Stmt::Release { lhs, .. } => fix_lv(lhs, from, to),
        Stmt::EventTrigger { name, .. } => fix_path(name),
        Stmt::ConcurrentAssert {
            clock,
            disable_iff,
            antecedent,
            consequent,
            pass,
            fail,
            prop_expr,
            ..
        } => {
            // Rename every operand (clock sensitivity exprs + disable iff +
            // antecedent + consequent + action-block statements + the N2d
            // property-expression tree) — same completeness lesson as EventCtrl
            // above (an unrenamed operand would silently capture the outer signal).
            if let Sensitivity::List(evs) = clock {
                for ev in evs {
                    fix_expr(&mut ev.expr, from, to);
                }
            }
            if let Some(e) = disable_iff {
                fix_expr(e, from, to);
            }
            fix_sequence(antecedent, from, to);
            fix_sequence(consequent, from, to);
            if let Some(pe) = prop_expr {
                fix_prop_expr(pe, from, to);
            }
            if let Some(s) = pass {
                rename_ident_in_stmt(s, from, to);
            }
            if let Some(s) = fail {
                rename_ident_in_stmt(s, from, to);
            }
        }
        Stmt::DeferredAssert {
            cond,
            then_s,
            else_s,
            ..
        } => {
            fix_expr(cond, from, to);
            rename_ident_in_stmt(then_s, from, to);
            rename_ident_in_stmt(else_s, from, to);
        }
        Stmt::Return { value, .. } => {
            if let Some(e) = value {
                fix_expr(e, from, to);
            }
        }
        Stmt::CoverProperty {
            clock,
            disable_iff,
            seq,
            ..
        } => {
            if let Sensitivity::List(evs) = clock {
                for ev in evs {
                    fix_expr(&mut ev.expr, from, to);
                }
            }
            if let Some(e) = disable_iff {
                fix_expr(e, from, to);
            }
            fix_sequence(seq, from, to);
        }
        Stmt::WaitFork { .. } | Stmt::Disable { .. } | Stmt::Null(_) | Stmt::Error(_) => {}
    }
}

#[cfg(test)]
mod tests {
    /// SV cast `casting_type'(expr)` (§6.24) parses to `ExprKind::Cast` with the
    /// right `CastTarget`; malformed casts are loud parse errors (never silent).
    #[test]
    fn cast_parse_forms_and_malformed() {
        fn rhs_of(src_body: &str) -> Option<ExprKind> {
            let src = format!("module t; initial x = {src_body}; endmodule");
            let (toks, le) = hdl_lexer::lex(&src);
            assert!(le.is_empty(), "lex errors in {src_body:?}: {le:?}");
            let (unit, errs) = parse(&toks, &src);
            if !errs.is_empty() {
                return None; // signals a (loud) parse error to the caller
            }
            let TopItem::Module(ref m) = unit.as_ref()?.items[0] else {
                return None;
            };
            let ModuleItem::Proc(ref pb) = m.body[0] else {
                return None;
            };
            // initial <stmt>; — dig out the blocking-assign rhs.
            fn find_rhs(s: &Stmt) -> Option<ExprKind> {
                match s {
                    Stmt::Blocking { rhs, .. } => Some(rhs.kind.clone()),
                    Stmt::Block { stmts, .. } => stmts.iter().find_map(find_rhs),
                    _ => None,
                }
            }
            find_rhs(&pb.body)
        }
        use CastPrim as P;
        // type cast → Prim
        assert!(matches!(
            rhs_of("int'(8'hFF)"),
            Some(ExprKind::Cast {
                target: CastTarget::Prim(P::Int),
                ..
            })
        ));
        assert!(matches!(
            rhs_of("byte'(a)"),
            Some(ExprKind::Cast {
                target: CastTarget::Prim(P::Byte),
                ..
            })
        ));
        assert!(matches!(
            rhs_of("logic'(a)"),
            Some(ExprKind::Cast {
                target: CastTarget::Prim(P::Logic),
                ..
            })
        ));
        // signing cast → Signing
        assert!(matches!(
            rhs_of("signed'(a)"),
            Some(ExprKind::Cast {
                target: CastTarget::Signing { signed: true },
                ..
            })
        ));
        assert!(matches!(
            rhs_of("unsigned'(a)"),
            Some(ExprKind::Cast {
                target: CastTarget::Signing { signed: false },
                ..
            })
        ));
        // size cast → Size; typedef/class name → Named
        assert!(matches!(
            rhs_of("8'(a)"),
            Some(ExprKind::Cast {
                target: CastTarget::Size(_),
                ..
            })
        ));
        assert!(matches!(
            rhs_of("(W+1)'(a)"),
            Some(ExprKind::Cast {
                target: CastTarget::Size(_),
                ..
            })
        ));
        assert!(matches!(
            rhs_of("my_t'(a)"),
            Some(ExprKind::Cast {
                target: CastTarget::Named(_),
                ..
            })
        ));
        // precedence: cast binds tighter than `+` → `(8'(a)) + b`
        assert!(matches!(rhs_of("8'(a) + b"), Some(ExprKind::Binary { .. })));
        // replication wrapping a cast still parses
        assert!(matches!(
            rhs_of("{2{8'(a)}}"),
            Some(ExprKind::Replicate { .. })
        ));
        // malformed casts → loud parse error (None)
        for bad in ["int'", "int'(", "8'(", "8'(a", "signed'5"] {
            assert!(
                rhs_of(bad).is_none(),
                "expected loud parse error for {bad:?}"
            );
        }
    }

    /// v7 AST flip: package/import/string/pkg:: parse to their dedicated
    /// shapes (semantics land in the follow-on slices — parse-only here).
    #[test]
    fn v7_package_import_string_pkgscoped_parse() {
        let src = r#"
package p;
  parameter W = 8;
endpackage
import p::*;
module t;
  import p::W;
  string s;
  integer x;
  initial x = p::W;
endmodule
"#;
        let (toks, lex_errs) = hdl_lexer::lex(src);
        assert!(lex_errs.is_empty());
        let (unit, errs) = parse(&toks, src);
        assert!(errs.is_empty(), "parse errors: {errs:?}");
        let unit = unit.unwrap();
        assert!(matches!(unit.items[0], TopItem::Package(ref m) if m.name.name == "p"));
        assert!(
            matches!(unit.items[1], TopItem::Import(ref i) if i.pkg.name == "p" && i.item.is_none())
        );
        let TopItem::Module(ref m) = unit.items[2] else {
            panic!("expected module, got {:?}", unit.items[2]);
        };
        assert!(matches!(
            m.body[0],
            ModuleItem::Import(ref i) if i.pkg.name == "p"
                && i.item.as_ref().map(|x| x.name.as_str()) == Some("W")
        ));
        assert!(matches!(
            m.body[1],
            ModuleItem::NetVar(ref d) if matches!(d.kind, NetVarKind::String)
        ));
        // the initial body holds `x = p::W` — walk to the PkgScoped expr.
        let ModuleItem::Proc(ref pb) = m.body[3] else {
            panic!("expected proc, got {:?}", m.body[3]);
        };
        let mut found = false;
        fn walk(s: &Stmt, found: &mut bool) {
            if let Stmt::Blocking { rhs, .. } = s {
                if matches!(
                    rhs.kind,
                    ExprKind::PkgScoped { ref pkg, ref name }
                        if pkg.name == "p" && name.name == "W"
                ) {
                    *found = true;
                }
            }
            if let Stmt::Block { stmts, .. } = s {
                for st in stmts {
                    walk(st, found);
                }
            }
        }
        walk(&pb.body, &mut found);
        assert!(found, "p::W must parse as PkgScoped");
    }

    /// Review-finding regressions (2026-06-11): the foreach rename walker
    /// must leave NO single-segment reference to the source-level index name
    /// anywhere in the desugared tree — including block-local decl
    /// initializers/dims and event-control sensitivity exprs (the two arms a
    /// review caught as missed → silent outer-variable capture).
    #[test]
    fn foreach_rename_covers_decl_inits_and_event_ctrl() {
        let src = r#"
module t;
  integer q [$];
  integer r;
  initial begin
    foreach (q[i]) begin
      integer k = q[i];
      @(q[i]) r = q[i];
    end
  end
endmodule
"#;
        let (toks, lex_errs) = hdl_lexer::lex(src);
        assert!(lex_errs.is_empty());
        let (unit, errs) = parse(&toks, src);
        assert!(errs.is_empty(), "parse errors: {errs:?}");
        let unit = unit.unwrap();
        // walk the whole AST; collect every single-segment ident name.
        fn idents_in_expr(e: &Expr, out: &mut Vec<String>) {
            match &e.kind {
                ExprKind::Ident(p) => {
                    if p.segments.len() == 1 {
                        out.push(p.segments[0].name.clone());
                    }
                }
                ExprKind::Unary { operand, .. } => idents_in_expr(operand, out),
                ExprKind::Binary { lhs, rhs, .. } => {
                    idents_in_expr(lhs, out);
                    idents_in_expr(rhs, out);
                }
                ExprKind::Ternary {
                    cond,
                    then_e,
                    else_e,
                } => {
                    idents_in_expr(cond, out);
                    idents_in_expr(then_e, out);
                    idents_in_expr(else_e, out);
                }
                ExprKind::BitSelect { base, index } => {
                    idents_in_expr(base, out);
                    idents_in_expr(index, out);
                }
                ExprKind::PartSelect { base, msb, lsb } => {
                    idents_in_expr(base, out);
                    idents_in_expr(msb, out);
                    idents_in_expr(lsb, out);
                }
                ExprKind::Call { args, .. } | ExprKind::SysCall { args, .. } => {
                    for a in args {
                        idents_in_expr(a, out);
                    }
                }
                ExprKind::Paren { inner } => idents_in_expr(inner, out),
                _ => {}
            }
        }
        fn idents_in_stmt(s: &Stmt, out: &mut Vec<String>) {
            match s {
                Stmt::Blocking { lhs, rhs, .. } | Stmt::NonBlocking { lhs, rhs, .. } => {
                    if let Lvalue::Ident(p) = lhs {
                        if p.segments.len() == 1 {
                            out.push(p.segments[0].name.clone());
                        }
                    }
                    if let Lvalue::BitSelect { index, .. } = lhs {
                        idents_in_expr(index, out);
                    }
                    idents_in_expr(rhs, out);
                }
                Stmt::For {
                    init,
                    cond,
                    step,
                    body,
                    ..
                } => {
                    idents_in_stmt(init, out);
                    idents_in_expr(cond, out);
                    idents_in_stmt(step, out);
                    idents_in_stmt(body, out);
                }
                Stmt::Block { decls, stmts, .. } => {
                    for d in decls {
                        for n in &d.names {
                            if let Some(e) = &n.init {
                                idents_in_expr(e, out);
                            }
                        }
                    }
                    for st in stmts {
                        idents_in_stmt(st, out);
                    }
                }
                Stmt::EventCtrl { ctrl, body, .. } => {
                    if let Sensitivity::List(evs) = ctrl {
                        for ev in evs {
                            idents_in_expr(&ev.expr, out);
                        }
                    }
                    if let Some(b) = body {
                        idents_in_stmt(b, out);
                    }
                }
                _ => {}
            }
        }
        let mut names = Vec::new();
        for it in &unit.items {
            if let TopItem::Module(m) = it {
                for item in &m.body {
                    if let ModuleItem::Proc(pb) = item {
                        idents_in_stmt(&pb.body, &mut names);
                    }
                }
            }
        }
        assert!(
            !names.iter().any(|n| n == "i"),
            "the source index name must be fully renamed; leftover refs: {names:?}"
        );
        assert!(
            names.iter().any(|n| n.starts_with("__foreach_i_")),
            "the synthetic index must appear: {names:?}"
        );
    }

    use super::*;

    fn p(src: &str) -> (Option<SourceUnit>, Vec<ParseError>) {
        let (toks, lex_errs) = hdl_lexer::lex(src);
        assert!(lex_errs.is_empty(), "lex errors: {lex_errs:?}");
        parse(&toks, src)
    }
    fn first_module(su: &SourceUnit) -> &ModuleDecl {
        match &su.items[0] {
            TopItem::Module(m) => m,
            _ => panic!("not a module"),
        }
    }
    /// Parse a bare expression via `assign x = <expr>;` and return the RHS.
    fn expr_of(src: &str) -> Expr {
        let wrapped = format!("module m; assign x = {src};\nendmodule");
        let (su, errs) = p(&wrapped);
        assert!(errs.is_empty(), "parse errors for `{src}`: {errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        match &m.body[0] {
            ModuleItem::ContAssign(ca) => ca.assigns[0].1.clone(),
            _ => panic!(),
        }
    }
    fn bin(e: &Expr) -> (BinOp, &Expr, &Expr) {
        match &e.kind {
            ExprKind::Binary { op, lhs, rhs } => (*op, lhs, rhs),
            other => panic!("not binary: {other:?}"),
        }
    }
    /// Parse a module body; return the first ProceduralBlock.
    fn proc_of(body: &str) -> ProceduralBlock {
        let src = format!("module m;\n{body}\nendmodule");
        let (su, errs) = p(&src);
        assert!(errs.is_empty(), "parse errors: {errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        match m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_))) {
            Some(ModuleItem::Proc(pb)) => pb.clone(),
            _ => panic!("no procedural block in body"),
        }
    }
    fn as_block(s: &Stmt) -> (&Option<Ident>, &Vec<NetVarDecl>, &Vec<Stmt>) {
        match s {
            Stmt::Block {
                label,
                decls,
                stmts,
                ..
            } => (label, decls, stmts),
            other => panic!("not a Block: {other:?}"),
        }
    }

    // 1. mul binds tighter than add:  a + b * c  =>  +(a, *(b,c))
    #[test]
    fn t1_mul_tighter_than_add() {
        let (op, _l, r) = {
            let e = expr_of("a + b * c");
            let (o, l, r) = bin(&e);
            (o, l.clone(), r.clone())
        };
        assert_eq!(op, BinOp::Add);
        assert_eq!(bin(&r).0, BinOp::Mul);
    }

    // 2. ternary right-assoc:  a ? b : c ? d : e  =>  a ? b : (c ? d : e)
    #[test]
    fn t2_ternary_right_assoc() {
        let e = expr_of("a ? b : c ? d : e");
        let ExprKind::Ternary { else_e, .. } = &e.kind else {
            panic!()
        };
        assert!(matches!(else_e.kind, ExprKind::Ternary { .. }));
    }

    // 3. concat LHS + left-assoc add:  assign {cout,sum} = a + b + cin;
    #[test]
    fn t3_concat_lhs_left_assoc() {
        let (su, errs) = p("module m; assign {cout, sum} = a + b + cin;\nendmodule");
        assert!(errs.is_empty(), "{errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        let ModuleItem::ContAssign(ca) = &m.body[0] else {
            panic!()
        };
        let Lvalue::Concat { parts, .. } = &ca.assigns[0].0 else {
            panic!("LHS not concat")
        };
        assert_eq!(parts.len(), 2);
        let (op, l, _r) = bin(&ca.assigns[0].1);
        assert_eq!(op, BinOp::Add);
        assert_eq!(bin(l).0, BinOp::Add); // left child is (a+b)  → left-assoc
    }

    // 4. ANSI #(param)(ports) + direction inheritance
    #[test]
    fn t4_ansi_header() {
        let (su, errs) = p("module adder #(parameter WIDTH = 8)\
            (input [WIDTH-1:0] a, b, output [WIDTH-1:0] sum);\nendmodule");
        assert!(errs.is_empty(), "{errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        assert_eq!(m.name.name, "adder");
        assert_eq!(m.params.len(), 1);
        assert_eq!(m.params[0].kind, ParamKind::Parameter);
        let PortList::Ansi(ports) = &m.ports else {
            panic!("not ANSI")
        };
        assert_eq!(ports.len(), 3);
        assert_eq!(ports[0].dir, PortDir::Input);
        assert_eq!(ports[1].dir, PortDir::Input); // `b` inherits
        assert_eq!(ports[2].dir, PortDir::Output);
    }

    // 5. non-ANSI module: header names + body dir/type
    #[test]
    fn t5_non_ansi() {
        let (su, errs) = p(
            "module m(a, b, y);\n  input a, b;\n  output y;\n  wire [3:0] tmp;\n\
            assign y = a & b;\nendmodule",
        );
        assert!(errs.is_empty(), "{errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        let PortList::NonAnsi(names) = &m.ports else {
            panic!("not non-ANSI")
        };
        assert_eq!(
            names.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            ["a", "b", "y"]
        );
        assert!(matches!(m.body[0], ModuleItem::PortDecl(_)));
        assert!(m.body.iter().any(|i| matches!(i, ModuleItem::NetVar(_))));
        assert!(m
            .body
            .iter()
            .any(|i| matches!(i, ModuleItem::ContAssign(_))));
    }

    // 6. vector range is an expr, not pre-evaluated:  wire [WIDTH-1:0] bus;
    #[test]
    fn t6_range_is_expr() {
        let (su, _e) = p("module m; wire [WIDTH-1:0] bus;\nendmodule");
        let su = su.unwrap();
        let m = first_module(&su);
        let ModuleItem::NetVar(nv) = &m.body[0] else {
            panic!()
        };
        let r = nv.range.as_ref().unwrap();
        assert_eq!(bin(&r.msb).0, BinOp::Sub);
        assert!(matches!(r.lsb.kind, ExprKind::IntLit { .. }));
    }

    // 7. indexed part-select [b+:w]
    #[test]
    fn t7_indexed_part_select() {
        let e = expr_of("data[base +: 8]");
        let ExprKind::IndexedPart { dir, .. } = &e.kind else {
            panic!("{:?}", e.kind)
        };
        assert_eq!(*dir, PartDir::PlusColon);
    }

    // 8. & tighter than | :  a & b | c  =>  |(&(a,b), c)
    #[test]
    fn t8_and_tighter_than_or() {
        let e = expr_of("a & b | c");
        let (op, l, _r) = bin(&e);
        assert_eq!(op, BinOp::BitOr);
        assert_eq!(bin(l).0, BinOp::BitAnd);
    }

    // 9. unary tighter than equality:  !a == b  =>  ==(!a, b)
    #[test]
    fn t9_unary_tighter_than_eq() {
        let e = expr_of("!a == b");
        let (op, l, _r) = bin(&e);
        assert_eq!(op, BinOp::Eq);
        assert!(matches!(
            l.kind,
            ExprKind::Unary {
                op: UnOp::LogNot,
                ..
            }
        ));
    }

    // 10. add tighter than shift (the doc's #1 gotcha):  a + b << 2  =>  (a+b) << 2
    #[test]
    fn t10_add_tighter_than_shift() {
        let e = expr_of("a + b << 2");
        let (op, l, _r) = bin(&e);
        assert_eq!(op, BinOp::Shl);
        assert_eq!(bin(l).0, BinOp::Add);
    }

    // 11. replication value is a Vec, NOT a Concat wrapper (verdict M5):  {3{a}}
    #[test]
    fn t11_replication_value_is_vec() {
        let e = expr_of("{3{a}}");
        let ExprKind::Replicate { count, value } = &e.kind else {
            panic!("{:?}", e.kind)
        };
        assert!(matches!(count.kind, ExprKind::IntLit { .. }));
        assert_eq!(value.len(), 1);
        assert!(matches!(value[0].kind, ExprKind::Ident(_))); // bare `a`, not Concat{[a]}
    }

    // 12. mintypmax delay (verdict M2):  assign #(1:2:3) y = a;
    #[test]
    fn t12_mintypmax_delay() {
        let (su, errs) = p("module m; assign #(1:2:3) y = a;\nendmodule");
        assert!(errs.is_empty(), "{errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        let ModuleItem::ContAssign(ca) = &m.body[0] else {
            panic!()
        };
        let d = ca.delay.as_ref().unwrap();
        assert_eq!(d.values.len(), 1);
        assert!(matches!(d.values[0].kind, ExprKind::MinTypMax { .. }));
    }

    // 13. recovery continues after a bad item (uses a lexer-error token `@`-stray
    //     plus garbage); the trailing valid assign still parses (verdict B3).
    #[test]
    fn t13_recovery_continues() {
        let (su, errs) = p("module m; wire @ ; assign y = a;\nendmodule");
        assert!(!errs.is_empty(), "expected a recovered error");
        let su = su.unwrap();
        let m = first_module(&su);
        assert!(
            m.body
                .iter()
                .any(|i| matches!(i, ModuleItem::ContAssign(_))),
            "parser must recover and parse the trailing assign"
        );
    }

    // 14. termination edges (verdict H3-soundness): must not hang / must terminate.
    #[test]
    fn t14_termination_edges() {
        assert_eq!(p("").0, None); // empty input ⇒ (None, [])
        let _ = p("module"); // truncated header
        let _ = p("module module;"); // sync-anchor == entry-token trap
        let _ = p("module m; endmodule extra ;"); // trailing junk
                                                  // reaching here without hang is the assertion
    }

    // 15. ** LEFT-assoc and unary precedence:  -a ** b  =>  (-a) ** b ; 2**3**4
    //     => (2**3)**4 (IEEE Table 11-2 / iverilog).
    #[test]
    fn t15_pow_assoc_and_unary() {
        let e = expr_of("2 ** 3 ** 4");
        let (op, l, _r) = bin(&e);
        assert_eq!(op, BinOp::Pow);
        assert_eq!(bin(&l.clone()).0, BinOp::Pow); // LEFT child is 2**3 (left-assoc)
        let e2 = expr_of("- a ** b");
        let (op2, l2, _r2) = bin(&e2);
        assert_eq!(op2, BinOp::Pow); // top is **
        assert!(matches!(
            l2.kind,
            ExprKind::Unary {
                op: UnOp::Minus,
                ..
            }
        )); // left is (-a)
    }

    // S1. initial begin: blocking + nonblocking mix
    #[test]
    fn s1_initial_blocking_nonblocking() {
        let pb = proc_of("initial begin a = 1; q <= d; end");
        assert_eq!(pb.kind, ProcKind::Initial);
        assert!(pb.sensitivity.is_none());
        let (_l, _d, stmts) = as_block(&pb.body);
        assert!(matches!(stmts[0], Stmt::Blocking { .. }));
        assert!(matches!(stmts[1], Stmt::NonBlocking { .. }));
    }

    // S2. always @(posedge clk) if/else (no begin) — sensitivity on the BLOCK
    #[test]
    fn s2_always_posedge_if_else() {
        let pb = proc_of("always @(posedge clk) if (rst) q <= 0; else q <= d;");
        assert_eq!(pb.kind, ProcKind::Always);
        let Some(Sensitivity::List(evs)) = &pb.sensitivity else {
            panic!()
        };
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].edge, Edge::Posedge);
        let Stmt::If { else_s, .. } = &*pb.body else {
            panic!("body not If")
        };
        assert!(else_s.is_some());
    }

    // S3. posedge/negedge `or`-separated sensitivity list
    #[test]
    fn s3_sensitivity_or_list() {
        let pb = proc_of("always @(posedge clk or negedge rst_n) q <= d;");
        let Some(Sensitivity::List(evs)) = &pb.sensitivity else {
            panic!()
        };
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].edge, Edge::Posedge);
        assert_eq!(evs[1].edge, Edge::Negedge);
    }

    // S4. always_comb + case: sensitivity MUST be None (@ never consumed); multi-label
    #[test]
    fn s4_always_comb_case() {
        let pb = proc_of("always_comb case (sel) 2'b00, 2'b01: y = a; default: y = b; endcase");
        assert_eq!(pb.kind, ProcKind::AlwaysComb);
        assert!(pb.sensitivity.is_none());
        let Stmt::Case { kind, items, .. } = &*pb.body else {
            panic!()
        };
        assert_eq!(*kind, CaseKind::Case);
        let CaseItem::Match { labels, .. } = &items[0] else {
            panic!()
        };
        assert_eq!(labels.len(), 2); // two labels share one body
        assert!(matches!(items[1], CaseItem::Default { .. }));
    }

    // S5. casez kind + `default` WITHOUT a colon
    #[test]
    fn s5_casez_default_no_colon() {
        let pb = proc_of("always_comb casez (req) 4'b1???: g = 1; default g = 0; endcase");
        let Stmt::Case { kind, items, .. } = &*pb.body else {
            panic!()
        };
        assert_eq!(*kind, CaseKind::Casez);
        assert!(matches!(items[1], CaseItem::Default { .. }));
    }

    // S6. for-loop — init/step are Blocking built WITHOUT consuming the ';'
    #[test]
    fn s6_for_loop() {
        let pb = proc_of("initial for (i = 0; i < 8; i = i + 1) sum = sum + i;");
        let Stmt::For {
            init, step, body, ..
        } = &*pb.body
        else {
            panic!()
        };
        assert!(matches!(**init, Stmt::Blocking { .. }));
        assert!(matches!(**step, Stmt::Blocking { .. }));
        assert!(matches!(**body, Stmt::Blocking { .. }));
    }

    // S7. while + $display systask call (name retains `$`, 2 args)
    #[test]
    fn s7_while_and_display() {
        let pb =
            proc_of("initial while (cnt < 8) begin $display(\"c=%d\", cnt); cnt = cnt + 1; end");
        let Stmt::While { body, .. } = &*pb.body else {
            panic!()
        };
        let (_l, _d, stmts) = as_block(body);
        let Stmt::SysTaskCall { name, args, .. } = &stmts[0] else {
            panic!()
        };
        assert_eq!(name.name, "$display");
        assert_eq!(args.len(), 2);
    }

    // S8. #delay statement with body, then $finish with NO parens (empty args)
    #[test]
    fn s8_delay_and_finish() {
        let pb = proc_of("initial begin #20 rst = 0; #200 $finish; end");
        let (_l, _d, stmts) = as_block(&pb.body);
        let Stmt::DelayCtrl { body: b0, .. } = &stmts[0] else {
            panic!()
        };
        assert!(matches!(b0.as_deref(), Some(Stmt::Blocking { .. })));
        let Stmt::DelayCtrl { body: b1, .. } = &stmts[1] else {
            panic!()
        };
        let Some(Stmt::SysTaskCall { name, args, .. }) = b1.as_deref() else {
            panic!()
        };
        assert_eq!(name.name, "$finish");
        assert!(args.is_empty());
    }

    // S9. dangling-else binds to the INNER if
    #[test]
    fn s9_dangling_else_inner() {
        let pb = proc_of("initial if (a) if (b) x = 1; else x = 2;");
        let Stmt::If { then_s, else_s, .. } = &*pb.body else {
            panic!()
        };
        assert!(else_s.is_none(), "outer if must NOT own the else");
        let Stmt::If {
            else_s: inner_else, ..
        } = &**then_s
        else {
            panic!("then not If")
        };
        assert!(inner_else.is_some(), "inner if owns the else");
    }

    // S10. named begin-end with a local decl + end-label (label consumed, no hang)
    #[test]
    fn s10_named_block_local_decl() {
        let pb = proc_of("initial begin : blk reg [7:0] tmp; tmp = a; end");
        let (label, decls, stmts) = as_block(&pb.body);
        assert_eq!(label.as_ref().unwrap().name, "blk");
        assert_eq!(decls.len(), 1);
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0], Stmt::Blocking { .. }));
    }

    // S11. always @(*) Star + nested begin
    #[test]
    fn s11_nested_block_and_star() {
        let pb = proc_of("always @(*) begin a = b; begin c = d; end end");
        assert!(matches!(pb.sensitivity, Some(Sensitivity::Star)));
        let (_l, _d, stmts) = as_block(&pb.body);
        assert!(matches!(stmts[0], Stmt::Blocking { .. }));
        assert!(matches!(stmts[1], Stmt::Block { .. }));
    }

    // S12. recovery: garbage statement → Error, no infinite loop, following stmt parses
    #[test]
    fn s12_recovery_garbage_stmt() {
        let (su, errs) = p("module m;\ninitial begin & ; x = 1; end\nendmodule");
        assert!(!errs.is_empty(), "expected a recovered error");
        let su = su.unwrap();
        let m = first_module(&su);
        let Some(ModuleItem::Proc(pb)) = m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
        else {
            panic!("no proc block")
        };
        let (_l, _d, stmts) = as_block(&pb.body);
        assert!(
            stmts.iter().any(|s| matches!(s, Stmt::Error(_))),
            "garbage → Error"
        );
        assert!(
            stmts.iter().any(|s| matches!(s, Stmt::Blocking { .. })),
            "must recover and parse `x = 1;`"
        );
    }

    // S13. fork / join_none — JoinKind from an Ident token (not a keyword)
    #[test]
    fn s13_fork_join_none() {
        let pb = proc_of("initial fork #10 a = 1; #20 b = 1; join_none");
        let Stmt::Fork { stmts, join, .. } = &*pb.body else {
            panic!()
        };
        assert_eq!(*join, JoinKind::JoinNone);
        assert_eq!(stmts.len(), 2);
    }

    // S14. repeat body is a bare EventCtrl (body None); wait body None;
    //      and intra-assign delay `q <= #1 d;` parses CLEAN into the delay field.
    #[test]
    fn s14_event_body_none_and_intra_delay() {
        let src = "module m;\ninitial begin repeat (8) @(posedge clk); wait (ready); q <= #1 d; end\nendmodule";
        let (su, errs) = p(src);
        assert!(errs.is_empty(), "intra-assign delay parses clean: {errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        let Some(ModuleItem::Proc(pb)) = m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
        else {
            panic!("no proc block")
        };
        let (_l, _d, stmts) = as_block(&pb.body);
        let Stmt::Repeat { body, .. } = &stmts[0] else {
            panic!()
        };
        let Stmt::EventCtrl { body: eb, .. } = &**body else {
            panic!("repeat body not EventCtrl")
        };
        assert!(eb.is_none()); // `@(posedge clk);` → body None
        let Stmt::Wait { body: wb, .. } = &stmts[1] else {
            panic!()
        };
        assert!(wb.is_none());
        // intra-assign delay is CAPTURED into the AST delay field (the
        // elaborator decides semantics: blocking = real, NBA = loud defer).
        let Stmt::NonBlocking { delay, .. } = &stmts[2] else {
            panic!("not NonBlocking")
        };
        assert!(delay.is_some(), "intra-assign delay must be captured");
    }

    // S14b. blocking intra-assign delay `a = #3 b;` captures into `delay`; blocking
    //       intra-assign EVENT control `a = @(ev) b` / `a = repeat(n) @(ev) b`
    //       captures into `event` (slice: repeat-event intra-assignment).
    #[test]
    fn s14b_blocking_intra_delay_and_event_control_captured() {
        let (su, errs) = p("module m;\ninitial a = #3 b;\nendmodule");
        assert!(
            errs.is_empty(),
            "blocking intra-delay parses clean: {errs:?}"
        );
        let su = su.unwrap();
        let m = first_module(&su);
        let Some(ModuleItem::Proc(pb)) = m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
        else {
            panic!("no proc block")
        };
        let Stmt::Blocking { delay, event, .. } = &*pb.body else {
            panic!("not Blocking: {:?}", pb.body)
        };
        assert!(
            delay.is_some() && event.is_none(),
            "blocking intra-assign delay must be captured (no event)"
        );

        // Plain `@(ev)` intra-assign now parses clean and captures `event` (repeat=None).
        let (su, errs) = p("module m;\ninitial a = @(posedge clk) b;\nendmodule");
        assert!(
            errs.is_empty(),
            "intra-assign event control parses clean: {errs:?}"
        );
        let su = su.unwrap();
        let m = first_module(&su);
        let ModuleItem::Proc(pb) = m
            .body
            .iter()
            .find(|i| matches!(i, ModuleItem::Proc(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let Stmt::Blocking { event, delay, .. } = &*pb.body else {
            panic!("not Blocking")
        };
        let ev = event.as_ref().expect("event control must be captured");
        assert!(
            delay.is_none() && ev.repeat.is_none(),
            "plain @(ev): repeat None"
        );

        // `repeat(n) @(ev)` captures the count.
        let (su, errs) = p("module m;\ninitial a = repeat(3) @(posedge clk) b;\nendmodule");
        assert!(
            errs.is_empty(),
            "repeat-event intra-assign parses clean: {errs:?}"
        );
        let su = su.unwrap();
        let m = first_module(&su);
        let ModuleItem::Proc(pb) = m
            .body
            .iter()
            .find(|i| matches!(i, ModuleItem::Proc(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let Stmt::Blocking { event, .. } = &*pb.body else {
            panic!("not Blocking")
        };
        assert!(
            event.as_ref().and_then(|e| e.repeat.as_ref()).is_some(),
            "repeat(n) @(ev): repeat count must be captured"
        );
    }

    // S15. SV immediate assert (IEEE 1800 §16.3) desugars AT PARSE TIME to
    //      `Stmt::If` — the frozen AST Stmt set (M7) gains no variant, and `if`
    //      already has the exact assert condition semantics (X/Z → else). No
    //      else clause ⇒ the IEEE default failure action is synthesized as
    //      `$error("Assertion failed")`.
    #[test]
    fn s15_assert_desugars_to_if_with_default_error() {
        let (su, errs) = p("module m;\ninitial assert (a == 1);\nendmodule");
        assert!(errs.is_empty(), "immediate assert parses clean: {errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        let Some(ModuleItem::Proc(pb)) = m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
        else {
            panic!("no proc block")
        };
        let Stmt::If { then_s, else_s, .. } = &*pb.body else {
            panic!("assert must desugar to If: {:?}", pb.body)
        };
        assert!(
            matches!(**then_s, Stmt::Null(_)),
            "no pass action → Null then-branch"
        );
        let Some(e) = else_s else {
            panic!("missing else clause must synthesize the default action")
        };
        let Stmt::SysTaskCall { name, args, .. } = &**e else {
            panic!("default else must be a $error call: {e:?}")
        };
        assert_eq!(name.name, "$error");
        assert_eq!(args.len(), 1);
        assert!(
            matches!(&args[0].kind, ExprKind::StrLit { raw } if raw.contains("Assertion failed"))
        );
    }

    // S15b. explicit pass/else actions map onto the If branches verbatim; the
    //       else-only form gets a Null then-branch.
    #[test]
    fn s15b_assert_actions_map_to_if_branches() {
        let (su, errs) =
            p("module m;\ninitial assert (a) $display(\"ok\"); else $display(\"no\");\nendmodule");
        assert!(errs.is_empty(), "{errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        let Some(ModuleItem::Proc(pb)) = m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
        else {
            panic!("no proc block")
        };
        let Stmt::If { then_s, else_s, .. } = &*pb.body else {
            panic!("not If: {:?}", pb.body)
        };
        let Stmt::SysTaskCall { name, .. } = &**then_s else {
            panic!("pass action must be the then-branch")
        };
        assert_eq!(name.name, "$display");
        let Some(e) = else_s else { panic!("no else") };
        let Stmt::SysTaskCall { name, .. } = &**e else {
            panic!("user else action must be kept verbatim")
        };
        assert_eq!(name.name, "$display");

        // else-only: `assert (a) else x = 1;`
        let (su2, errs2) = p("module m;\ninitial assert (a) else x = 1;\nendmodule");
        assert!(errs2.is_empty(), "{errs2:?}");
        let su2 = su2.unwrap();
        let m2 = first_module(&su2);
        let Some(ModuleItem::Proc(pb2)) = m2.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
        else {
            panic!("no proc block")
        };
        let Stmt::If { then_s, else_s, .. } = &*pb2.body else {
            panic!("not If")
        };
        assert!(matches!(**then_s, Stmt::Null(_)));
        assert!(matches!(else_s.as_deref(), Some(Stmt::Blocking { .. })));
    }

    // S15c. the DEFERRED forms now PARSE to `Stmt::DeferredAssert` (faithful
    //       deferred-assert slice): `#0` = Observed, `final` = Reactive. A
    //       non-zero `#<n>` delay on an assert stays a LOUD parse error.
    #[test]
    fn s15c_deferred_assert_parses_observed_and_reactive() {
        for (src, want) in [
            (
                "module m;\ninitial assert #0 (a);\nendmodule",
                AssertDefer::Observed,
            ),
            (
                "module m;\ninitial assert final (a);\nendmodule",
                AssertDefer::Reactive,
            ),
        ] {
            let (su, errs) = p(src);
            assert!(errs.is_empty(), "{src}: {errs:?}");
            let su = su.unwrap();
            let m = first_module(&su);
            let Some(ModuleItem::Proc(pb)) =
                m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
            else {
                panic!("no proc block")
            };
            let Stmt::DeferredAssert { region, .. } = &*pb.body else {
                panic!("not DeferredAssert: {:?}", pb.body)
            };
            assert_eq!(*region, want, "{src}");
        }
        // a non-zero `#` delay on an assert is NOT a deferred assert → loud.
        let (_, errs) = p("module m;\ninitial assert #1 (a);\nendmodule");
        assert!(!errs.is_empty(), "`assert #1` must be a loud parse error");
    }

    // S15d (v8 SVA subset). `assert property(@(clk) a |-> b)` parses to a
    // `Stmt::ConcurrentAssert`; `|->` is Overlap, `|=>` is NonOverlap.
    #[test]
    fn concurrent_assert_property_parses_overlap_and_nonoverlap() {
        let (su, errs) =
            p("module m;\ninitial assert property (@(posedge clk) a |-> b);\nendmodule");
        assert!(errs.is_empty(), "concurrent assertion must parse: {errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        let ModuleItem::Proc(pb) = m
            .body
            .iter()
            .find(|i| matches!(i, ModuleItem::Proc(_)))
            .unwrap()
        else {
            unreachable!()
        };
        assert!(
            matches!(
                &*pb.body,
                Stmt::ConcurrentAssert {
                    implication_kind: ImplicationKind::Overlap,
                    ..
                }
            ),
            "expected ConcurrentAssert{{Overlap}}, got {:?}",
            pb.body
        );

        let (su2, errs2) =
            p("module m;\ninitial assert property (@(posedge clk) a |=> b);\nendmodule");
        assert!(errs2.is_empty(), "non-overlap must parse: {errs2:?}");
        let su2 = su2.unwrap();
        let m2 = first_module(&su2);
        let ModuleItem::Proc(pb2) = m2
            .body
            .iter()
            .find(|i| matches!(i, ModuleItem::Proc(_)))
            .unwrap()
        else {
            unreachable!()
        };
        assert!(
            matches!(
                &*pb2.body,
                Stmt::ConcurrentAssert {
                    implication_kind: ImplicationKind::NonOverlap,
                    ..
                }
            ),
            "expected ConcurrentAssert{{NonOverlap}}, got {:?}",
            pb2.body
        );
    }

    // S15e (SVA slice S4). Sequence antecedents: `##n` cycle-delay parses to
    // `Sequence::Delay`, `[*n]` consecutive repetition to `Sequence::Repeat`.
    #[test]
    fn concurrent_assert_seq_delay_parses() {
        let (su, errs) =
            p("module m;\ninitial assert property (@(posedge clk) a ##1 b |-> c);\nendmodule");
        assert!(
            errs.is_empty(),
            "sequence-delay antecedent must parse: {errs:?}"
        );
        let su = su.unwrap();
        let m = first_module(&su);
        let ModuleItem::Proc(pb) = m
            .body
            .iter()
            .find(|i| matches!(i, ModuleItem::Proc(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let Stmt::ConcurrentAssert {
            antecedent,
            implication_kind: ImplicationKind::Overlap,
            ..
        } = &*pb.body
        else {
            panic!("expected ConcurrentAssert(Overlap), got {:?}", pb.body)
        };
        assert!(
            matches!(
                antecedent,
                Sequence::Delay {
                    min: 1,
                    max: Some(1),
                    ..
                }
            ),
            "expected Sequence::Delay{{1}}, got {antecedent:?}"
        );
    }

    #[test]
    fn concurrent_assert_seq_repeat_parses() {
        let (su, errs) =
            p("module m;\ninitial assert property (@(posedge clk) a[*3] |-> b);\nendmodule");
        assert!(
            errs.is_empty(),
            "repetition antecedent must parse: {errs:?}"
        );
        let su = su.unwrap();
        let m = first_module(&su);
        let ModuleItem::Proc(pb) = m
            .body
            .iter()
            .find(|i| matches!(i, ModuleItem::Proc(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
            panic!("expected ConcurrentAssert, got {:?}", pb.body)
        };
        assert!(
            matches!(
                antecedent,
                Sequence::Repeat {
                    min: 3,
                    max: Some(3),
                    ..
                }
            ),
            "expected Sequence::Repeat{{3}}, got {antecedent:?}"
        );
    }

    // S15f (S5). Bounded ranges `##[m:n]` / `[*m:n]` now PARSE to Delay/Repeat
    // with min != max; unbounded (`$`), `throughout`, `within` stay LOUD.
    #[test]
    fn concurrent_assert_seq_ranges_parse() {
        let (su, errs) =
            p("module m;\ninitial assert property (@(posedge clk) a ##[1:2] b |-> c);\nendmodule");
        assert!(errs.is_empty(), "bounded delay range must parse: {errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        let ModuleItem::Proc(pb) = m
            .body
            .iter()
            .find(|i| matches!(i, ModuleItem::Proc(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
            panic!("expected ConcurrentAssert, got {:?}", pb.body)
        };
        assert!(
            matches!(
                antecedent,
                Sequence::Delay {
                    min: 1,
                    max: Some(2),
                    ..
                }
            ),
            "expected Sequence::Delay{{1,2}}, got {antecedent:?}"
        );

        let (su2, errs2) =
            p("module m;\ninitial assert property (@(posedge clk) a[*2:3] |-> b);\nendmodule");
        assert!(
            errs2.is_empty(),
            "bounded repeat range must parse: {errs2:?}"
        );
        let m2 = first_module(su2.as_ref().unwrap());
        let ModuleItem::Proc(pb2) = m2
            .body
            .iter()
            .find(|i| matches!(i, ModuleItem::Proc(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let Stmt::ConcurrentAssert { antecedent, .. } = &*pb2.body else {
            panic!("expected ConcurrentAssert, got {:?}", pb2.body)
        };
        assert!(
            matches!(
                antecedent,
                Sequence::Repeat {
                    min: 2,
                    max: Some(3),
                    ..
                }
            ),
            "expected Sequence::Repeat{{2,3}}, got {antecedent:?}"
        );
    }

    // S15g (S6). Unbounded cycle delay `##[m:$]` parses to Delay{min, max:None}.
    #[test]
    fn concurrent_assert_seq_unbounded_delay_parses() {
        let (su, errs) =
            p("module m;\ninitial assert property (@(posedge clk) a ##[1:$] b |-> c);\nendmodule");
        assert!(errs.is_empty(), "unbounded delay must parse: {errs:?}");
        let m = first_module(su.as_ref().unwrap());
        let ModuleItem::Proc(pb) = m
            .body
            .iter()
            .find(|i| matches!(i, ModuleItem::Proc(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
            panic!("expected ConcurrentAssert, got {:?}", pb.body)
        };
        assert!(
            matches!(
                antecedent,
                Sequence::Delay {
                    min: 1,
                    max: None,
                    ..
                }
            ),
            "expected Sequence::Delay{{1, $}}, got {antecedent:?}"
        );
    }

    // S15h (S7). `cond throughout seq` parses to Sequence::Throughout.
    #[test]
    fn concurrent_assert_throughout_parses() {
        let (su, errs) = p(
            "module m;\ninitial assert property (@(posedge clk) g throughout a ##2 c |-> d);\nendmodule",
        );
        assert!(errs.is_empty(), "throughout must parse: {errs:?}");
        let m = first_module(su.as_ref().unwrap());
        let ModuleItem::Proc(pb) = m
            .body
            .iter()
            .find(|i| matches!(i, ModuleItem::Proc(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
            panic!("expected ConcurrentAssert, got {:?}", pb.body)
        };
        // `g throughout (a ##2 c)` — throughout is looser than `##`.
        let Sequence::Throughout { seq, .. } = antecedent else {
            panic!("expected Sequence::Throughout, got {antecedent:?}")
        };
        assert!(
            matches!(&**seq, Sequence::Delay { min: 2, .. }),
            "throughout RHS must be the `a ##2 c` sequence, got {seq:?}"
        );
    }

    // S15i (S8). `b[->n]` goto / `b[=n]` nonconsec parse to Repeat with the right
    // RepeatKind.
    #[test]
    fn concurrent_assert_goto_nonconsec_parse() {
        for (src, want_goto) in [
            (
                "module m;\ninitial assert property (@(posedge clk) a ##1 b[->2] |-> c);\nendmodule",
                true,
            ),
            (
                "module m;\ninitial assert property (@(posedge clk) a ##1 b[=2] |-> c);\nendmodule",
                false,
            ),
        ] {
            let (su, errs) = p(src);
            assert!(errs.is_empty(), "goto/nonconsec must parse: {errs:?} ({src})");
            let m = first_module(su.as_ref().unwrap());
            let ModuleItem::Proc(pb) = m
                .body
                .iter()
                .find(|i| matches!(i, ModuleItem::Proc(_)))
                .unwrap()
            else {
                unreachable!()
            };
            let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
                panic!("expected ConcurrentAssert")
            };
            // antecedent is `a ##1 b[->2]` = Delay{.., rhs: Repeat{kind}}.
            let Sequence::Delay { rhs, .. } = antecedent else {
                panic!("expected Delay, got {antecedent:?}")
            };
            let Sequence::Repeat { kind, min: 2, .. } = &**rhs else {
                panic!("expected Repeat with count 2, got {rhs:?}")
            };
            let is_goto = matches!(kind, RepeatKind::Goto);
            assert_eq!(is_goto, want_goto, "wrong repeat kind for {src}");
        }
    }

    // S15j (S9). `seq1 within seq2` parses to Sequence::Within (binary over `##`
    // chains: `a within b ##2 c` = `a within (b ##2 c)`).
    #[test]
    fn concurrent_assert_within_parses() {
        let (su, errs) = p(
            "module m;\ninitial assert property (@(posedge clk) a within b ##2 c |-> d);\nendmodule",
        );
        assert!(errs.is_empty(), "within must parse: {errs:?}");
        let m = first_module(su.as_ref().unwrap());
        let ModuleItem::Proc(pb) = m
            .body
            .iter()
            .find(|i| matches!(i, ModuleItem::Proc(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
            panic!("expected ConcurrentAssert")
        };
        let Sequence::Within { seq2, .. } = antecedent else {
            panic!("expected Sequence::Within, got {antecedent:?}")
        };
        assert!(
            matches!(&**seq2, Sequence::Delay { min: 2, .. }),
            "within RHS must be `b ##2 c`, got {seq2:?}"
        );
    }

    // S13. Unbounded consecutive repeat `a[*m:$]` (m>=1) parses to
    // `Sequence::Repeat { min: m, max: None, kind: Consec }`.
    #[test]
    fn concurrent_assert_consec_unbounded_parses() {
        let (su, errs) =
            p("module m;\ninitial assert property (@(posedge clk) a[*2:$] |-> b);\nendmodule");
        assert!(errs.is_empty(), "`a[*2:$]` must parse: {errs:?}");
        let m = first_module(su.as_ref().unwrap());
        let ModuleItem::Proc(pb) = m
            .body
            .iter()
            .find(|i| matches!(i, ModuleItem::Proc(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
            panic!("expected ConcurrentAssert")
        };
        assert!(
            matches!(
                antecedent,
                Sequence::Repeat {
                    min: 2,
                    max: None,
                    kind: RepeatKind::Consec,
                    ..
                }
            ),
            "expected Repeat{{2, None, Consec}}, got {antecedent:?}"
        );
    }

    // goto/nonconsec RANGES stay parser-LOUD (single counts only). Empty-match
    // repetition `[*0:..]` now PARSES (2026-06-25) — a leading/standalone empty
    // is honest-loud at ELABORATE instead (see cli `sva_empty_match.rs`), so it
    // no longer belongs in this parser-level rejection net.
    #[test]
    fn concurrent_assert_deferred_seq_forms_are_loud() {
        for src in [
            "module m;\ninitial assert property (@(posedge clk) a ##1 b[->1:2] |-> c);\nendmodule",
            "module m;\ninitial assert property (@(posedge clk) a ##1 b[=1:2] |-> c);\nendmodule",
        ] {
            let (_, errs) = p(src);
            assert!(
                !errs.is_empty(),
                "deferred sequence form must be loud: {src}"
            );
        }
    }

    // Empty-match repetition now parses cleanly (the loud-ness moved to elaborate).
    #[test]
    fn empty_match_repetition_parses() {
        for src in [
            "module m;\ninitial assert property (@(posedge clk) a ##1 b[*0:$] |-> c);\nendmodule",
            "module m;\ninitial assert property (@(posedge clk) a ##1 b[*0:2] |-> c);\nendmodule",
            "module m;\ninitial assert property (@(posedge clk) a ##1 b[*] |-> c);\nendmodule",
            "module m;\ninitial assert property (@(posedge clk) a ##1 b[*0] |-> c);\nendmodule",
        ] {
            let (_, errs) = p(src);
            assert!(errs.is_empty(), "empty-match must parse: {src} -> {errs:?}");
        }
    }

    // ════════════════════ module instantiation (PR3) ════════════════════
    /// Return the first ModuleInstance in a module body.
    fn inst_of(body: &str) -> ModuleInstance {
        let src = format!("module m;\n{body}\nendmodule");
        let (su, errs) = p(&src);
        assert!(errs.is_empty(), "parse errors: {errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        match m.body.iter().find(|i| matches!(i, ModuleItem::Instance(_))) {
            Some(ModuleItem::Instance(mi)) => mi.clone(),
            _ => panic!("no module instance in body"),
        }
    }
    fn id_name(e: &Expr) -> &str {
        match &e.kind {
            ExprKind::Ident(p) => p.segments[0].name.as_str(),
            other => panic!("not a bare ident: {other:?}"),
        }
    }

    // I1. named connections: dff u1(.clk(clk), .d(d), .q(q));
    #[test]
    fn i1_named_connections() {
        let mi = inst_of("dff u1(.clk(clk), .d(d), .q(q));");
        assert_eq!(mi.module_name.name, "dff");
        assert!(mi.param_overrides.is_empty());
        assert_eq!(mi.instances.len(), 1);
        let it = &mi.instances[0];
        assert_eq!(it.name.name, "u1");
        let PortConnList::Named(conns, _) = &it.conns else {
            panic!("not named")
        };
        assert_eq!(conns.len(), 3);
        assert_eq!(conns[0].name.name, "clk");
        assert_eq!(id_name(conns[0].value.as_ref().unwrap()), "clk");
        assert_eq!(conns[2].name.name, "q");
        assert_eq!(id_name(conns[2].value.as_ref().unwrap()), "q");
    }

    // I2. positional connections: dff u1(clk, d, q);
    #[test]
    fn i2_positional_connections() {
        let mi = inst_of("dff u1(clk, d, q);");
        assert_eq!(mi.module_name.name, "dff");
        let PortConnList::Positional(conns) = &mi.instances[0].conns else {
            panic!("not positional")
        };
        assert_eq!(conns.len(), 3);
        assert_eq!(id_name(conns[0].as_ref().unwrap()), "clk");
        assert_eq!(id_name(conns[1].as_ref().unwrap()), "d");
        assert_eq!(id_name(conns[2].as_ref().unwrap()), "q");
    }

    // I3. named param override: reg8 #(.W(8)) r(.d(d), .q(q));
    #[test]
    fn i3_named_param_override() {
        let mi = inst_of("reg8 #(.W(8)) r(.d(d), .q(q));");
        assert_eq!(mi.module_name.name, "reg8");
        assert_eq!(mi.param_overrides.len(), 1);
        let ParamConn::Named { name, value, .. } = &mi.param_overrides[0] else {
            panic!("not a named override")
        };
        assert_eq!(name.name, "W");
        assert!(matches!(
            value.as_ref().unwrap().kind,
            ExprKind::IntLit { .. }
        ));
        assert_eq!(mi.instances[0].name.name, "r");
    }

    // I4. positional param override + multiple params: mem #(8, 256) u(.clk(clk));
    #[test]
    fn i4_positional_param_override() {
        let mi = inst_of("mem #(8, 256) u(.clk(clk));");
        assert_eq!(mi.param_overrides.len(), 2);
        assert!(matches!(mi.param_overrides[0], ParamConn::Positional(_)));
        assert!(matches!(mi.param_overrides[1], ParamConn::Positional(_)));
    }

    // I5. multiple instances per statement: dff u0(clk,q0), u1(q0,q1);
    #[test]
    fn i5_multiple_instances_per_statement() {
        let mi = inst_of("dff u0(clk, q0), u1(q0, q1);");
        assert_eq!(mi.module_name.name, "dff");
        assert_eq!(mi.instances.len(), 2);
        assert_eq!(mi.instances[0].name.name, "u0");
        assert_eq!(mi.instances[1].name.name, "u1");
    }

    // I6. unconnected positional slot: alu u(a, , c);  → None in the middle.
    #[test]
    fn i6_positional_unconnected_slot() {
        let mi = inst_of("alu u(a, , c);");
        let PortConnList::Positional(conns) = &mi.instances[0].conns else {
            panic!("not positional")
        };
        assert_eq!(conns.len(), 3);
        assert!(conns[0].is_some());
        assert!(conns[1].is_none()); // skipped port
        assert!(conns[2].is_some());
    }

    // I7. explicitly-unconnected named port `.q()` ⇒ value None; empty `()` list.
    #[test]
    fn i7_named_empty_and_empty_list() {
        let mi = inst_of("dff u1(.clk(clk), .q());");
        let PortConnList::Named(conns, _) = &mi.instances[0].conns else {
            panic!("not named")
        };
        assert_eq!(conns.len(), 2);
        assert!(conns[1].value.is_none(), "`.q()` ⇒ None");
        // empty `()` list ⇒ zero-arity Positional
        let mi2 = inst_of("noports u2();");
        let PortConnList::Positional(c2) = &mi2.instances[0].conns else {
            panic!("empty () should be Positional")
        };
        assert!(c2.is_empty());
    }

    // I8. instance-array dim + a connection expr: rep u_x [3:0] (.in(bus));
    #[test]
    fn i8_instance_array_dim() {
        let mi = inst_of("rep u_x [3:0] (.in(bus));");
        let it = &mi.instances[0];
        assert_eq!(it.name.name, "u_x");
        assert_eq!(it.unpacked.len(), 1);
        assert!(matches!(it.unpacked[0], Dim::Range(_)));
    }

    // I9. expression-valued named connection: dff u(.d(a & b), .q(q));
    #[test]
    fn i9_expression_connection() {
        let mi = inst_of("dff u(.d(a & b), .q(q));");
        let PortConnList::Named(conns, _) = &mi.instances[0].conns else {
            panic!("not named")
        };
        let (op, _l, _r) = bin(conns[0].value.as_ref().unwrap());
        assert_eq!(op, BinOp::BitAnd);
    }

    // I10. `.*` implicit wildcard connection now parses cleanly (it used to be a
    //      stub-with-advisory; the wildcard is now supported, IEEE §23.3.2.5), and
    //      the trailing item still parses.
    #[test]
    fn i10_dotstar_parses_as_wildcard() {
        let (su, errs) = p("module m; sub u1(.*); assign y = a;\nendmodule");
        assert!(errs.is_empty(), "`.*` should no longer emit an advisory");
        let su = su.unwrap();
        let m = first_module(&su);
        // the instance is present with an empty explicit list + wildcard = true.
        let inst = m
            .body
            .iter()
            .find_map(|i| match i {
                ModuleItem::Instance(it) => Some(it),
                _ => None,
            })
            .expect("instance present");
        let PortConnList::Named(conns, wildcard) = &inst.instances[0].conns else {
            panic!("expected a Named conn list for `.*`");
        };
        assert!(conns.is_empty(), "`.*` alone has no explicit conns");
        assert!(*wildcard, "`.*` sets the wildcard flag");
        // …and the trailing assign still parses.
        assert!(m
            .body
            .iter()
            .any(|i| matches!(i, ModuleItem::ContAssign(_))));
    }

    // ───────────────────────── PR3: generate / genvar ─────────────────────────

    /// Parse a single generate construct wrapped in a module; return its items.
    fn gen_of(body: &str) -> Vec<GenItem> {
        let src = format!("module m;\n{body}\nendmodule");
        let (su, errs) = p(&src);
        assert!(errs.is_empty(), "parse errors: {errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        match m.body.iter().find_map(|i| match i {
            ModuleItem::Generate(g) => Some(g),
            _ => None,
        }) {
            Some(g) => g.items.clone(),
            None => panic!("no generate construct in: {src}"),
        }
    }

    // g1. genvar multi-declaration → Genvar{names==["i","j"]}.
    #[test]
    fn g1_genvar_decl() {
        let (su, errs) = p("module m; genvar i, j;\nendmodule");
        assert!(errs.is_empty(), "{errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        let ModuleItem::Genvar { names, .. } = &m.body[0] else {
            panic!("not a genvar decl: {:?}", m.body[0]);
        };
        assert_eq!(
            names.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            ["i", "j"]
        );
    }

    // g2. labeled generate-for with an instance body → For{label hoisted to "g"},
    //     init/step lvalue "i", body one Item(Instance).
    #[test]
    fn g2_gen_for_labeled_instance() {
        let items = gen_of(
            "generate for (i = 0; i < 3; i = i + 1) begin : g\n  leaf u (.a(x[i]));\nend\nendgenerate",
        );
        assert_eq!(items.len(), 1);
        let GenItem::For {
            init,
            step,
            label,
            body,
            ..
        } = &items[0]
        else {
            panic!("not a For: {:?}", items[0]);
        };
        assert_eq!(init.lvalue.name, "i");
        assert_eq!(step.lvalue.name, "i");
        assert_eq!(label.as_ref().map(|l| l.name.as_str()), Some("g"));
        assert_eq!(body.len(), 1);
        assert!(matches!(
            &body[0],
            GenItem::Item(mi) if matches!(**mi, ModuleItem::Instance(_))
        ));
    }

    // g3. bare-body generate-for (no begin/end) → For{label none}, body one
    //     Item(ContAssign).
    #[test]
    fn g3_gen_for_bare_body() {
        let items =
            gen_of("generate for (i = 0; i < 2; i = i + 1) assign y[i] = a[i];\nendgenerate");
        assert_eq!(items.len(), 1);
        let GenItem::For { label, body, .. } = &items[0] else {
            panic!("not a For: {:?}", items[0]);
        };
        assert!(label.is_none());
        assert_eq!(body.len(), 1);
        assert!(matches!(
            &body[0],
            GenItem::Item(mi) if matches!(**mi, ModuleItem::ContAssign(_))
        ));
    }

    // g4. generate-if with and without else.
    #[test]
    fn g4_gen_if_else() {
        let items = gen_of("generate if (W) assign y = a; else assign y = b;\nendgenerate");
        let GenItem::If { then_b, else_b, .. } = &items[0] else {
            panic!("not an If: {:?}", items[0]);
        };
        assert_eq!(then_b.len(), 1);
        assert_eq!(else_b.len(), 1);

        let items = gen_of("generate if (W) assign y = a;\nendgenerate");
        let GenItem::If { then_b, else_b, .. } = &items[0] else {
            panic!("not an If: {:?}", items[0]);
        };
        assert_eq!(then_b.len(), 1);
        assert!(else_b.is_empty());
    }

    // g5. generate-case: 0:…  1,2:…  default:… → Match{1}, Match{2}, Default.
    #[test]
    fn g5_gen_case() {
        let items = gen_of(
            "generate case (W)\n  0: assign y = a;\n  1, 2: assign y = b;\n  default: assign y = c;\nendcase\nendgenerate",
        );
        let GenItem::Case { items: cis, .. } = &items[0] else {
            panic!("not a Case: {:?}", items[0]);
        };
        assert_eq!(cis.len(), 3);
        assert!(matches!(&cis[0], GenCaseItem::Match { labels, .. } if labels.len() == 1));
        assert!(matches!(&cis[1], GenCaseItem::Match { labels, .. } if labels.len() == 2));
        assert!(matches!(&cis[2], GenCaseItem::Default { .. }));
    }

    // g6. (M2 clamp) truncated generate headers recover with errors and DO NOT
    //     panic on the inverted Span::to union.
    #[test]
    fn g6_truncated_headers_no_panic() {
        for src in [
            "module m; generate for endgenerate\nendmodule",
            "module m; generate if (\nendmodule",
            "module m; generate case (\nendmodule",
            "module m; generate for (\nendmodule",
        ] {
            let (toks, _lex) = hdl_lexer::lex(src);
            let (_su, errs) = parse(&toks, src);
            assert!(!errs.is_empty(), "expected parse errors for `{src}`");
        }
    }

    // ─────────────────────── function / task definitions ───────────────────────
    fn item_of(body: &str) -> ModuleItem {
        let src = format!("module m;\n{body}\nendmodule");
        let (su, errs) = p(&src);
        assert!(errs.is_empty(), "parse errors: {errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        m.body[0].clone()
    }

    // ft1. ANSI combinational function: width range, one input formal, single
    //      `f = <expr>` body reachable as a Block with one Blocking to the func name.
    #[test]
    fn ft1_parse_ansi_function_def() {
        let it = item_of("function [7:0] add1(input [7:0] x); add1 = x + 1; endfunction");
        let ModuleItem::Func(f) = it else {
            panic!("not a function: {it:?}");
        };
        assert_eq!(f.name.name, "add1");
        assert!(!f.automatic);
        assert!(f.range.is_some(), "expected [7:0] return range");
        assert_eq!(f.ret_type, ParamType::Implicit);
        assert_eq!(f.ports.len(), 1);
        assert_eq!(f.ports[0].dir, PortDir::Input);
        assert_eq!(f.ports[0].name.name, "x");
        assert!(f.ports[0].range.is_some());
        // body: a single blocking assign `add1 = x + 1`
        let Stmt::Blocking { lhs, rhs, .. } = &*f.body else {
            panic!("expected single Blocking body, got {:?}", f.body);
        };
        assert!(matches!(lhs, Lvalue::Ident(p) if p.segments[0].name == "add1"));
        assert!(matches!(&rhs.kind, ExprKind::Binary { op: BinOp::Add, .. }));
    }

    // ft2. Non-ANSI function: formal declared in the body prefix, hoisted into ports.
    #[test]
    fn ft2_parse_non_ansi_function_def() {
        let it = item_of(
            "function [3:0] f; input [3:0] a; reg [3:0] t; begin t = a; f = t; end endfunction",
        );
        let ModuleItem::Func(f) = it else {
            panic!("not a function: {it:?}");
        };
        assert_eq!(f.name.name, "f");
        // non-ANSI input `a` hoisted into ports
        assert_eq!(f.ports.len(), 1);
        assert_eq!(f.ports[0].dir, PortDir::Input);
        assert_eq!(f.ports[0].name.name, "a");
        // local `reg t` lands in body_decls
        assert_eq!(f.body_decls.len(), 1);
        assert_eq!(f.body_decls[0].names[0].name.name, "t");
        // body is a begin..end with two blocking assigns
        let Stmt::Block { stmts, .. } = &*f.body else {
            panic!("expected begin-end body, got {:?}", f.body);
        };
        assert_eq!(stmts.len(), 2);
    }

    // ft3. Task with input + output formals (ANSI), begin-end body.
    #[test]
    fn ft3_parse_task_def() {
        let it = item_of("task drive(input [7:0] d, output [7:0] q); begin q = d; end endtask");
        let ModuleItem::Task(t) = it else {
            panic!("not a task: {it:?}");
        };
        assert_eq!(t.name.name, "drive");
        assert!(!t.automatic);
        assert_eq!(t.ports.len(), 2);
        assert_eq!(t.ports[0].dir, PortDir::Input);
        assert_eq!(t.ports[0].name.name, "d");
        assert_eq!(t.ports[1].dir, PortDir::Output);
        assert_eq!(t.ports[1].name.name, "q");
        let Stmt::Block { stmts, .. } = &*t.body else {
            panic!("expected begin-end body, got {:?}", t.body);
        };
        assert_eq!(stmts.len(), 1);
    }

    // ft4. Sticky direction across comma-grouped formals: `input a, b` → both Input.
    #[test]
    fn ft4_sticky_direction_and_empty_task() {
        let it = item_of("function f(input a, b); f = a & b; endfunction");
        let ModuleItem::Func(f) = it else {
            panic!("not a function");
        };
        assert_eq!(f.ports.len(), 2);
        assert_eq!(f.ports[0].dir, PortDir::Input);
        assert_eq!(f.ports[1].dir, PortDir::Input);
        assert_eq!(f.ports[1].name.name, "b");

        // empty-bodied task with no port list: `task t; endtask`
        let it2 = item_of("task t; endtask");
        let ModuleItem::Task(t) = it2 else {
            panic!("not a task");
        };
        assert!(t.ports.is_empty());
        assert!(matches!(&*t.body, Stmt::Null(_)));
    }
}
