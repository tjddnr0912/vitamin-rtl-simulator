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

// ---- split modules (mechanical refactor; see the module-size policy note in the crate docs) ----
mod api;
mod assertions;
mod assign_pattern;
mod blocks_timing;
mod casts;
mod classes;
mod cover;
mod decls;
mod enums;
mod expr;
mod expr_primary;
mod functask;
mod gates;
mod generate;
mod instances;
mod lvalue;
mod module_items;
mod monomorph;
mod params;
mod scope;
mod soa;
mod stmt;
mod stmt_ctl;
mod struct_sel;
mod structs;
mod sva_prop;
mod sva_seq;
mod typedefs;
mod udp;
mod udp_table;
pub use api::*;
pub(crate) use blocks_timing::*;
pub(crate) use decls::*;
pub(crate) use expr::*;
pub(crate) use monomorph::*;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub span: Span, // where to ANCHOR the report (u32)
    /// "expression", "';'", "identifier", … — or, for the few diagnostics that must
    /// name something from the source (an unknown package, a rejected spelling), an
    /// OWNED message. The vast majority stay `Borrowed` and allocate nothing.
    pub expected: std::borrow::Cow<'static, str>,
    pub found: Option<TokenKind>, // None ⇒ EOF
    /// Span of the token in [`Self::found`]. Usually equal to [`Self::span`],
    /// but `error_at` anchors the report at an EARLIER node while `found` stays
    /// the current token — so the two are separate fields and only this one may
    /// be sliced to recover the offending spelling.
    pub found_span: Span,
}

/// A NON-FATAL parse observation: the construct is accepted and its value is
/// unchanged here, but other tools read it differently — so the log has to say so.
///
/// Separate from [`ParseError`] on purpose: a warning must not abort the run, and a
/// severity field on the error type would put "did this stop the parse?" and "how bad
/// is it?" in one place, which is how a gate ends up suppressing something it should
/// not (Error/Fatal are unsuppressible; these are `-Wno-`-able).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseWarn {
    pub span: Span,
    pub kind: ParseWarnKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseWarnKind {
    /// A bit/part select whose BASE is not a variable reference (IEEE 1800 §11.5.1).
    NonStandardSelectBase,
}

impl ParseError {
    /// The offending token as the user SPELLED it, for the `…, found X` tail.
    /// `src` is the same expanded text the parse ran on.
    ///
    /// The `found` field is an internal token enum; rendering it with `Debug`
    /// puts vita's own type names in a user-facing message (`found
    /// Word(Keyword(End))` for `end`), which the reader has to translate back
    /// to source and a log consumer has to know vita's lexer to parse. The
    /// spelling is recoverable from the span, so quote that instead, and name
    /// the word class for the one token shape where the bare spelling is
    /// ambiguous about why it is wrong (a keyword where a name was expected).
    ///
    /// `None` ⇒ say nothing rather than guess: the caller drops the tail. That
    /// arm is unreachable for a real token (`found` is `Some` only when the
    /// cursor is on one, and every token has a non-empty span), but the slice
    /// is fallible and a wrong `found` is worse than no `found`.
    pub fn found_desc(&self, src: &str) -> Option<String> {
        let Some(kind) = self.found else {
            return Some("end of file".to_string());
        };
        let text = src.get(self.found_span.lo as usize..self.found_span.hi as usize)?;
        if text.is_empty() {
            return None;
        }
        // One line, bounded: a string literal may carry newlines and a token
        // has no length limit, and either would break the one-diagnostic-per-line
        // contract the whole stage renders under.
        let mut shown: String = text
            .chars()
            .take(32)
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        if text.chars().nth(32).is_some() {
            shown.push('…');
        }
        Some(match kind {
            TokenKind::Word(WordKind::Keyword(_)) => format!("keyword '{shown}'"),
            TokenKind::Word(WordKind::Ident) | TokenKind::EscapedIdent => {
                format!("identifier '{shown}'")
            }
            _ => format!("'{shown}'"),
        })
    }
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

/// A resolved tf-port type carried through the comma-sticky inheritance:
/// `(net_or_var kind, signed, range, struct-type-name, enum-type-name)`. The struct
/// name is `Some` only for a packed struct/union typedef port (EXT2-C); the enum name
/// (r18/E1) is `Some` for an `enum`-typedef port so `m.name()`/`m.next()` desugar in the
/// body (`var_enum`). Both thread onward so a bare continuation `input e_t a, b` binds
/// every name.
type TfPortType = (
    Option<NetVarKind>,
    bool,
    Option<Range>,
    Option<String>,
    Option<String>,
);

/// Flat bit layout of a packed struct: members are placed MSB-first into one
/// `logic [total-1:0]` vector. `fields` carries `(name, lsb_offset, width,
/// ascending, signed, two_state, dbase)` so a `s.field` access desugars to the
/// constant part-select `s[off+w-1 : off]`, and a trailing sub-select (`s.f[i]` /
/// `s.f[a:b]` / `s.f[base±:w]`) can be remapped onto the flat vector with the
/// member's declared direction (`ascending` = `logic [0:N]`, field index 0 = MSB).
/// `signed` is the member's EFFECTIVE signedness (atom types `int`/`byte`/… and
/// `signed`-qualified vectors are signed); the WHOLE-field read is wrapped in a
/// `$signed()` so a signed member reads back negative (a sub-select stays
/// unsigned per §5.4.1, matching iverilog). `two_state` (the member is `bit`/
/// `byte`/`int`/`shortint`/`longint`) drives the `'{…}` pattern desugar to coerce
/// X/Z→0 into that field (§6.11.3), which a 4-state member does not. `dbase` is the
/// member's DECLARED base index — `min(msb, lsb)` of the member's own range (0 for
/// a plain `[N:0]`/`[0:N]`/atom member) — subtracted from a sub-select's source
/// index so a NON-zero-LSB member (`logic [15:8] a; s.a[11:8]`) selects the right
/// field-relative bits instead of reading raw/out-of-range positions (silent X).
/// One packed struct/union member's flat layout: `(name, lsb_offset, width,
/// ascending, signed, two_state, dbase, elem_stride)`. `elem_stride` is the
/// first-level element width of a multi-dim packed member (`logic [1:0][3:0] m` →
/// 4), or 1 for an ordinary single-dim member (so a `s.m[i]` bit-select stays
/// byte-identical). `> 1` marks a multi-dim member whose `m[i]` selects an
/// `elem_stride`-bit element, not a bit.
type StructFieldLayout = (String, u32, u32, bool, bool, bool, i64, u32);
#[derive(Clone, PartialEq)]
struct StructLayout {
    fields: Vec<StructFieldLayout>,
}
impl StructLayout {
    fn field(&self, name: &str) -> Option<(u32, u32, bool, bool, i64, u32)> {
        self.fields
            .iter()
            .find(|(n, ..)| n == name)
            .map(|(_, o, w, asc, sgn, _ts, dbase, stride)| (*o, *w, *asc, *sgn, *dbase, *stride))
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
    unpacked_struct_layouts: std::collections::HashMap<String, Vec<StructMember>>,
    enum_defs: std::collections::HashMap<String, Vec<(String, i64)>>,
    union_type_names: std::collections::HashSet<String>,
    // VAR-name-keyed (a block-local struct/enum variable shadowing an outer one).
    var_struct: std::collections::HashMap<String, String>,
    var_unpacked_struct: std::collections::HashMap<String, String>,
    // N3: a dynamic array of a PACKABLE record (`rec_t arr[]`, all-integral members) →
    // the record type name. `arr` lowers to a single `DynArray` net of the packed-
    // struct total width; `arr[i].field` is a part-select on the element (offsets
    // computed on demand from `unpacked_struct_layouts` via `packable_record_layout`).
    record_array_vars: std::collections::HashMap<String, String>,
    // N3 heterogeneous heap (SoA): a dyn array of a NON-uniform record (mixed 2-/4-state,
    // or a string/real member) → the record type name. Each member becomes its own typed
    // dyn array `$unp$arr$field`, so `arr[i].field` = a native dyn element access (per-
    // field 2-state/string/real semantics come for free, unlike the packed single-net).
    record_soa_vars: std::collections::HashMap<String, String>,
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

/// The parsed TYPE PREFIX of a parameter/localparam decl —
/// `[parameter|localparam] [signing] [data_type] [packed_range]` — shared by
/// EVERY name in a comma-list (`localparam [T] A = 1, B = 2` /
/// `#(parameter [T] A = 1, B = 2)`). Split from the name+value tail
/// (`finish_param_assignment`) so the prefix parses ONCE and an unadorned
/// continuation (`, B = 2`) inherits the leading type/width/signedness
/// (IEEE §6.20.1) instead of silently re-defaulting to a value-sized implicit
/// param. `expl0`/`expl1` keep explicit-signing PRESENCE (leading/trailing) for
/// the A2a array desugar's `signed_eff`.
#[derive(Clone)]
struct ParamPrefix {
    start: Span,
    kind: ParamKind,
    signed: bool,
    ty: ParamType,
    var_kind: Option<NetVarKind>,
    forced_range: Option<Range>,
    explicit_range: Option<Range>,
    expl0: Option<bool>,
    expl1: Option<bool>,
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
    /// Non-fatal observations — see [`ParseWarn`]. Never gates the parse.
    pub warnings: Vec<ParseWarn>,
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
    /// Round-9: UNPACKED struct (record) type name → its members (each keeps its
    /// OWN type — a `string`/`int` member can't share a flat vector). A scalar
    /// variable of this type desugars to N independent member nets `k$field`
    /// (no aggregate storage in v1); accumulates across the source unit like
    /// `struct_layouts` (scoped `pkg::T` twins added at `endpackage`).
    unpacked_struct_layouts: std::collections::HashMap<String, Vec<StructMember>>,
    /// Round-9: variable name → its UNPACKED-struct type name (module-scoped;
    /// cleared per module). Drives the `k.field` → `k$field` member-net desugar.
    var_unpacked_struct: std::collections::HashMap<String, String>,
    record_array_vars: std::collections::HashMap<String, String>,
    /// N3 SoA record arrays (var → typename): a NON-uniform record dyn array whose
    /// members each became a `$unp$arr$field` typed dyn array (module-scoped).
    record_soa_vars: std::collections::HashMap<String, String>,
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
    /// §23.11 binds written INSIDE a module/interface body. A bind whose target is a
    /// module NAME means the same thing wherever it is written — elaborate keys the
    /// bind table by target module name alone and wires the checker in each target
    /// INSTANCE's scope (`elaborate/driver.rs`), never consulting the directive's
    /// enclosing scope — so a body bind is hoisted here and appended to the source
    /// unit as an ordinary `TopItem::Bind`. There is no `ModuleItem::Bind`: the AST
    /// is frozen and needs no new variant for a construct whose meaning is
    /// scope-independent. Drained (and asserted empty) in `parse_source_unit`.
    pending_binds: Vec<BindDecl>,
    /// A body parameter/localparam COMMA-LIST (`localparam A=1, B=2;`) yields one
    /// `ModuleItem` per name from a single `parse_module_item` call. The FIRST is
    /// returned inline; the REST queue here (FIFO) and drain at the top of
    /// `parse_module_item` and `parse_gen_item` (which wraps them), plus the
    /// single-item `parse_gen_branch` arm, so every name lands in the SAME scope.
    pending_module_items: Vec<ModuleItem>,
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
    /// SV §6.19.5 `x.name()`: a synthetic `function string $enum_name$<T>(x)` —
    /// a `case(x)` returning each label's string literal — generated on first use
    /// per enum type in the CURRENT container, then injected into its body at the
    /// end (`take_pending_enum_name_fns`). A string-returning function gives the
    /// EXACT label length in every context (assign AND `$display("%s", …)`),
    /// which a packed string-literal ternary cannot (it pads to the widest label).
    /// BTreeMap (not HashMap) so the module-end injection order is DETERMINISTic
    /// (3-OS byte-identical golden — never iterate a HashMap into the AST).
    pending_enum_name_fns: std::collections::BTreeMap<String, FunctionDef>,
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
            warnings: Vec::new(),
            error_limit: 50,
            expr_depth: 0,
            stmt_depth: 0,
            node_count: 0,
            node_budget_blown: false,
            typedefs: std::collections::HashMap::new(),
            struct_layouts: std::collections::HashMap::new(),
            unpacked_struct_layouts: std::collections::HashMap::new(),
            var_unpacked_struct: std::collections::HashMap::new(),
            record_array_vars: std::collections::HashMap::new(),
            record_soa_vars: std::collections::HashMap::new(),
            var_struct: std::collections::HashMap::new(),
            struct_scalar_vars: std::collections::HashSet::new(),
            struct_1d_array_vars: std::collections::HashSet::new(),
            union_type_names: std::collections::HashSet::new(),
            const_locals: std::collections::HashMap::new(),
            pending_mono_specs: Vec::new(),
            pending_binds: Vec::new(),
            pending_module_items: Vec::new(),
            loop_labels: Vec::new(),
            enum_defs: std::collections::HashMap::new(),
            var_enum: std::collections::HashMap::new(),
            pending_enum_name_fns: std::collections::BTreeMap::new(),
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
            let at = self.cur_span();
            self.errors.push(ParseError {
                span: at,
                expected: expected.into(),
                found: self.peek(),
                found_span: at,
            });
        }
    }

    /// [`Self::error`] with a message built from the SOURCE — a package name, a
    /// rejected spelling. Kept separate so the common path stays allocation-free.
    pub(crate) fn error_owned(&mut self, expected: String) {
        if self.at_lex_error() {
            return;
        }
        if self.errors.len() < self.error_limit {
            let at = self.cur_span();
            self.errors.push(ParseError {
                span: at,
                expected: expected.into(),
                found: self.peek(),
                found_span: at,
            });
        }
    }

    /// Record a non-fatal [`ParseWarn`]. Deduplicated by span so one source site is
    /// reported once no matter how many times the parse revisits it.
    ///
    /// ⚠️ The dedup is UNREACHABLE today and measured so (`panic!` probe, 0 hits across
    /// the suite). It is kept because this parser DOES backtrack — `expr_primary`
    /// restores `self.pos` after a speculative `$bits(<type>)` read, and `functask`
    /// does the same for `const ref` — and today neither speculative path reaches
    /// `expr_postfix` before it gives up. The day one does, this latch is the only
    /// thing between a re-parse and the same source line reported twice.
    fn warn_select_base(&mut self, span: Span) {
        if self
            .warnings
            .iter()
            .any(|w| w.span == span && w.kind == ParseWarnKind::NonStandardSelectBase)
        {
            return;
        }
        self.warnings.push(ParseWarn {
            span,
            kind: ParseWarnKind::NonStandardSelectBase,
        });
    }

    /// Like [`error`] but reports at an explicit `span` (e.g. a node parsed earlier).
    fn error_at(&mut self, span: Span, expected: &'static str) {
        if self.errors.len() < self.error_limit {
            self.errors.push(ParseError {
                span,
                expected: expected.into(),
                found: self.peek(),
                found_span: self.cur_span(),
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

// ───────────── module / port / param / decl / contassign ─────────────

// ════════════════════════ PR3: generate / genvar ════════════════════════
//
// Parse-only: build the hdl-ast `GenerateConstruct`/`GenItem` tree; elaborate
// unrolls it. Mirrors the procedural for/if/case shapes (PR2) but produces
// `GenItem`s, not `Stmt`s. Every loop over a sub-item list carries a
// forward-progress guard (`pos == before → bump`) so malformed input can never
// spin, matching the rest of the parser's recovery discipline.

// ════════════════════════ PR2: statements + procedural blocks ════════════════════════

#[cfg(test)]
mod tests;
