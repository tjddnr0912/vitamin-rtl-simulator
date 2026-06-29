The precedence oracle is confirmed. Critical mapping of the doc's 14 levels (1=highest) to my binding powers (higher bp = binds tighter):

| Doc level | Operators | Assoc | My (lbp,rbp) |
|---|---|---|---|
| 1 | `()` `[]` | — | postfix (handled structurally) |
| 2 | unary/reduction | R→L | bp=27 |
| 3 | `**` | R→L | (26,25) |
| 4 | `* / %` | L→R | (23,24) |
| 5 | `+ -` | L→R | (21,22) |
| 6 | `<< >> <<< >>>` | L→R | (19,20) |
| 7 | `< <= > >=` | L→R | (17,18) |
| 8 | `== != === !==` | L→R | (15,16) |
| 9 | `&` | L→R | (13,14) |
| 10 | `^ ~^ ^~` | L→R | (11,12) |
| 11 | `\|` | L→R | (9,10) |
| 12 | `&&` | L→R | (7,8) |
| 13 | `\|\|` | L→R | (5,6) |
| 14 | `?:` | R→L | (4,3) |

This matches my binding-power table exactly (higher level number in doc = lower bp = looser binding). Both gotchas (`a+b<<2`→`(a+b)<<2`, `a&b==c`→`a&(b==c)`) are correctly handled. The original design table is **verified correct**.

Now I have everything needed. Let me write the final implementer-ready spec. I'll fold in all the fixes: B1 (operator-leftover error), B3 (forward-progress guard), M2 (mintypmax delay), M4 (first-ANSI-port direction error), M5 (`Replicate.value: Vec<Expr>`), M6/M7 (contract docs + full Stmt set), and the HIGH/MEDIUM hardening (Span::to debug_assert, prev_span doc, Error-token handling, serde_json→workspace).

The deliverable is the spec itself (my final text response). No files to write per the task instructions.

---

# vitamin — `hdl-ast` + `hdl-parser` Implementer Spec (PR1), FINAL

**Grounding re-verified against the live tree** (not from memory): lexer token/`Kw` API (`hdl-lexer/src/lib.rs:143-538`), derive catch-all (`vita-artifact-derive/src/lib.rs:240-267`), sim-ir `UnOp`/`BinOp`/`SelKind` (`sim-ir/src/lib.rs:95-150`), precedence oracle (`hdl-reference/verilog/03-expressions-operators.md`), workspace deps (`Cargo.toml`: no `winnow`, no `serde_json`; `serde`/`postcard` present). Both targets are empty stubs.

**Confirmed architecture (unchanged):** hand-RD + Pratt, no `winnow`/`diag` deps, `Span{lo,hi:u32}`, `(Option<SourceUnit>, Vec<ParseError>)`, serde-only / **SchemaHash deferred**, `UnOp`/`BinOp` mirror sim-ir 1:1. The precedence/binding-power table is verified bug-free.

**Resolved this revision:** B1, B3 (BLOCKERs); M2, M4, M5, M6, M7 (MAJORs); H2, H3, M1, M3, M4 (HIGH/MEDIUM from the soundness verdict). Folded: m1–m7, L1–L5. Noted: NITs.

---

## 1. `crates/hdl-ast/src/lib.rs`

```rust
//! hdl-ast — the parsed AST for the vitamin front-end (preprocess → lex → PARSE → …).
//!
//! Produced by `hdl-parser`, consumed by `elaborate` (which lowers it to the
//! span-free `sim-ir`, dropping spans into a side-table). Unlike sim-ir, **every
//! node carries a source span** (doc-14 §1: the `.vu` body = "hdl-ast 단위 트리 …
//! + 소스 스팬"). Spans are `u32` byte offsets → deterministic, `.vu`-safe.
//!
//! ## Serialization decision (load-bearing — verified against the derive source)
//! These types derive `Serialize + Deserialize` NOW so elaborate can write the
//! `.vu` body. They do **NOT** yet derive `SchemaHash`. Verified at
//! `vita-artifact-derive/src/lib.rs:248`: the catch-all arm pushes
//! `render_full_path(tp)` into `children`, and `render_full_path` (lib.rs:260)
//! joins only segment *idents* — so `Box<Expr>` renders to the bare string
//! `"Box"`, which is later re-parsed as a `Type` and emitted as
//! `<Box as SchemaShape>::register(reg)` — a hard compile error (no impl, and the
//! derive rejects type-args here). `Range<usize>` additionally trips the
//! `usize`-forbidden guard (lib.rs:210). A `Box`-recursive AST therefore cannot
//! carry `#[derive(SchemaHash)]` today. UNBLOCK (Residual 1): add one transparent
//! arm `("Box",1) => render_type_expr(args[0], children)` next to the existing
//! `Option`/`Vec`/`BTreeSet` arms (lib.rs:240-247). Until then the `.vu`
//! schema_hash root stays unlocked (doc-14 §5; consistent with sim-ir deferring
//! its M3 root freeze). All shapes here already obey the determinism rules
//! (no usize/HashMap/float; `Span` = two `u32`), so enabling the hash later does
//! NOT change the byte layout.

use serde::{Deserialize, Serialize};

// ───────────────────────────── Span ─────────────────────────────
/// Half-open byte range `[lo, hi)` into the preprocessed source. `u32` (not the
/// lexer's `Range<usize>`) so the serialized shape is deterministic across OSes.
/// The parser narrows each `Spanned.span: Range<usize>` at node construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub lo: u32,
    pub hi: u32,
}
impl Span {
    #[inline] pub fn new(lo: u32, hi: u32) -> Self { Self { lo, hi } }
    /// Span union: `self.lo .. other.hi`. Caller must ensure `other` ends at or
    /// after `self` (the parser's cursor is strictly monotonic, so this always
    /// holds for `start.to(prev_span())`-style unions). The debug_assert catches
    /// any future recovery path that composes spans out of order (verdict M4).
    #[inline] pub fn to(self, other: Span) -> Span {
        debug_assert!(other.hi >= self.lo, "Span::to: inverted union {self:?}..{other:?}");
        Span { lo: self.lo, hi: other.hi }
    }
}

/// An identifier reference: raw lexeme (re-sliced from source by the parser) + span.
/// `EscapedIdent` keeps its raw `\…` form; stripping the leading `\` and the
/// trailing-whitespace rule is the consumer's job. Interning to a `u32 Symbol` is
/// a later optimization (Residual 9); `String` keeps PR1 simple, determinism-safe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// Hierarchical name `a` | `a.b.c`. One-segment is the common case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HierPath {
    pub segments: Vec<Ident>,
    pub span: Span,
}

// ──────────────────────────── SourceUnit ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceUnit {
    pub items: Vec<TopItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopItem {
    Module(ModuleDecl),
    /// Recovery placeholder for an unparseable top-level construct.
    Error(Span),
}

// ──────────────────────────── Module ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleDecl {
    pub is_macromodule: bool,        // `module` vs `macromodule`
    pub name: Ident,
    pub params: Vec<ParamDecl>,      // ANSI `#( … )` param port list (empty if none)
    pub ports: PortList,
    pub body: Vec<ModuleItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortList {
    /// ANSI: header carries dir + type + range inline.
    Ansi(Vec<AnsiPort>),
    /// non-ANSI: header = bare names; dir/type come from body `PortDecl`s.
    NonAnsi(Vec<Ident>),
    /// `module m;` — no port parenthesis at all.
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnsiPort {
    pub dir: PortDir,
    pub net_or_var: Option<NetVarKind>, // None ⇒ default wire
    pub signed: bool,
    pub range: Option<Range>,           // packed [msb:lsb]
    pub name: Ident,
    pub default: Option<Expr>,          // ANSI default value slot (rare)
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortDir { Input, Output, Inout }

// ──────────────────────────── ModuleItem ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleItem {
    NetVar(NetVarDecl),
    Param(ParamDecl),                 // body-level parameter/localparam
    PortDecl(PortDecl),               // non-ANSI body port-direction decl
    ContAssign(ContinuousAssign),
    Proc(ProceduralBlock),            // [A] type defined; body stub-parsed in PR1
    Instance(ModuleInstance),         // [A]
    Generate(GenerateConstruct),      // [A]
    Genvar { names: Vec<Ident>, span: Span }, // [A]
    Func(FunctionDef),                // [A]
    Task(TaskDef),                    // [A]
    Defparam(DefparamItem),           // [A] defparam path = expr;
    /// Recovery placeholder for an unparseable item.
    Error(Span),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefparamItem { pub assigns: Vec<(HierPath, Expr)>, pub span: Span }

// ──────────────────── PortDecl (non-ANSI body dir) ────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortDecl {
    pub dir: PortDir,
    pub net_or_var: Option<NetVarKind>, // e.g. `output reg`
    pub signed: bool,
    pub range: Option<Range>,
    pub names: Vec<Ident>,              // `input [3:0] a, b;`
    pub span: Span,
}

// ──────────────────────────── ParamDecl ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamDecl {
    pub kind: ParamKind,
    pub signed: bool,
    pub ty: ParamType,
    pub range: Option<Range>,
    pub name: Ident,
    pub value: Expr,                   // RHS const-expr (required)
    pub span: Span,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamKind { Parameter, Localparam }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamType { Implicit, Integer, Real, Realtime, Time }

// ──────────────────────────── NetVarDecl ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetVarDecl {
    pub kind: NetVarKind,
    pub signed: bool,
    pub range: Option<Range>,          // packed/vector [msb:lsb]
    pub names: Vec<DeclName>,          // one decl, possibly many names
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclName {
    pub name: Ident,
    pub unpacked: Vec<Dim>,            // memory dims: reg [7:0] mem [0:255] / mem [256]
    pub init: Option<Expr>,            // net/var initializer `= expr`
    pub span: Span,
}
/// An unpacked array dimension. `[msb:lsb]` (V2005) OR `[size]` (SV size-form).
/// (verdict M3: the AST must represent both; the parser accepts `[size]` too.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dim {
    Range(Range),       // [hi:lo]
    Size(Expr),         // [N]
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetVarKind {
    // nets
    Wire, Tri, Wand, Triand, Wor, Trior, Tri0, Tri1,
    Supply0, Supply1, Trireg, Uwire,
    // variables
    Reg, Logic, Integer, Real, Realtime, Time,
}

/// `[msb:lsb]`. Bounds are exprs (usually const), NOT pre-evaluated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub msb: Expr,
    pub lsb: Expr,
    pub span: Span,
}

// ─────────────────────── ContinuousAssign ───────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuousAssign {
    pub delay: Option<Delay>,
    pub assigns: Vec<(Lvalue, Expr)>,  // assign a=b, c=d;
    pub span: Span,
}
/// `#d` | `#(d)` | `#(rise,fall)` | `#(rise,fall,turnoff)`. Each value is a
/// `MinTypMax`-or-plain expr (verdict M2 — `#(1:2:3)` is legal). The parser stores
/// each delay value via `Expr` (mintypmax surfaces as `ExprKind::MinTypMax`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delay { pub values: Vec<Expr>, pub span: Span }

// ───────────────── ProceduralBlock + Sensitivity [A] ─────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProceduralBlock {
    pub kind: ProcKind,
    pub sensitivity: Option<Sensitivity>, // only general `always @(…)`
    pub body: Box<Stmt>,                  // usually Block; stub-parsed in PR1
    pub span: Span,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcKind { Initial, Always, AlwaysFf, AlwaysComb, AlwaysLatch }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sensitivity {
    Star,                               // @(*) / @* (both map here; M5 note)
    List(Vec<EventExpr>),               // @(posedge clk or negedge rst or a)
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventExpr { pub edge: Edge, pub expr: Expr, pub span: Span }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edge { Posedge, Negedge, NoEdge }

// ──────────────────────────── Statement [A] ────────────────────────────
// FULL variant set frozen NOW (verdict M7): SchemaHash will eventually hash this
// enum, so adding `Fork`/`Assign`/`Deassign`/`Force`/`Release` later would flip the
// schema. The grammar §2.7 superset is adopted; parsing of all of these is deferred
// to PR2 (PR1 only constructs `Block`/`Error`/`Null` via the recovering stub).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stmt {
    Blocking    { lhs: Lvalue, delay: Option<Delay>, rhs: Expr, span: Span }, // =
    NonBlocking { lhs: Lvalue, delay: Option<Delay>, rhs: Expr, span: Span }, // <=
    If    { cond: Expr, then_s: Box<Stmt>, else_s: Option<Box<Stmt>>, span: Span },
    Case  { kind: CaseKind, scrutinee: Expr, items: Vec<CaseItem>, span: Span },
    For   { init: Box<Stmt>, cond: Expr, step: Box<Stmt>, body: Box<Stmt>, span: Span },
    While { cond: Expr, body: Box<Stmt>, span: Span },
    Repeat{ count: Expr, body: Box<Stmt>, span: Span },
    Forever { body: Box<Stmt>, span: Span },
    Block { label: Option<Ident>, decls: Vec<NetVarDecl>, stmts: Vec<Stmt>, span: Span },
    Fork  { label: Option<Ident>, decls: Vec<NetVarDecl>, stmts: Vec<Stmt>, join: JoinKind, span: Span },
    SysTaskCall  { name: Ident,    args: Vec<Expr>, span: Span }, // $display(...); name retains `$`
    UserTaskCall { name: HierPath, args: Vec<Expr>, span: Span },
    DelayCtrl { delay: Delay, body: Option<Box<Stmt>>, span: Span },      // #d stmt
    EventCtrl { ctrl: Sensitivity, body: Option<Box<Stmt>>, span: Span }, // @(…) stmt
    EventTrigger { name: HierPath, span: Span },                         // -> ev ;
    Wait  { cond: Expr, body: Option<Box<Stmt>>, span: Span },
    Disable { target: HierPath, span: Span },
    // procedural-continuous family (§2.7):
    Assign   { lhs: Lvalue, rhs: Expr, span: Span },   // procedural `assign lv = e;`
    Deassign { lhs: Lvalue, span: Span },
    Force    { lhs: Lvalue, rhs: Expr, span: Span },
    Release  { lhs: Lvalue, span: Span },
    Null(Span),                          // bare ;
    /// Recovery placeholder for an unparseable / not-yet-implemented statement.
    Error(Span),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinKind { Join, JoinAny, JoinNone }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseKind { Case, Casez, Casex }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseItem {
    Match { labels: Vec<Expr>, body: Box<Stmt>, span: Span },
    Default { body: Box<Stmt>, span: Span },
}

// ──────────────────────────── Expr [P] ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExprKind {
    // literals: raw lexeme + kind; value parse deferred to elaborate.
    IntLit  { kind: IntLitKind, raw: String },
    RealLit { kind: RealLitKind, raw: String },
    StrLit  { raw: String },             // includes quotes; unescape deferred
    // names
    Ident(HierPath),                     // a, a.b.c
    // operators (precedence table §4 → Pratt binding powers)
    Unary   { op: UnOp,  operand: Box<Expr> },
    Binary  { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Ternary { cond: Box<Expr>, then_e: Box<Expr>, else_e: Box<Expr> },
    // postfix / structural
    BitSelect   { base: Box<Expr>, index: Box<Expr> },                 // a[i]
    PartSelect  { base: Box<Expr>, msb: Box<Expr>, lsb: Box<Expr> },   // a[m:l]
    IndexedPart { base: Box<Expr>, offset: Box<Expr>, width: Box<Expr>, dir: PartDir }, // a[b+:w] (M6: `offset`)
    /// `{a,b,c}` concatenation.
    Concat    { parts: Vec<Expr> },
    /// `{n{x,y}}` replication. NOTE (verdict M5): `value` holds the repeated
    /// element list DIRECTLY (the concat parts), NOT a wrapper `Concat` node, so
    /// `{n{x}}` ⇒ `Replicate{count:n, value:[x]}`. No `ExprKind::Concat` wrapper.
    Replicate { count: Box<Expr>, value: Vec<Expr> },
    Call      { name: HierPath, args: Vec<Expr> },                     // func(args)
    /// `$time`, `$signed(x)`. NOTE (verdict M6): `name.name` retains the leading
    /// `$` (the lexer's `SystemTask` lexeme includes it), parallel to EscapedIdent.
    SysCall   { name: Ident, args: Vec<Expr> },
    Paren     { inner: Box<Expr> },                                    // (e) — span fidelity
    MinTypMax { min: Box<Expr>, typ: Box<Expr>, max: Box<Expr> },      // a:b:c
    /// Recovery placeholder so the Pratt loop can keep folding past an error.
    Error,
}

/// Unary / reduction operators — names mirror sim-ir `UnOp` 1:1 (verified
/// sim-ir/src/lib.rs:97) for a clean lowering name-map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnOp {
    Plus, Minus, LogNot, BitNot,
    RedAnd, RedNand, RedOr, RedNor, RedXor, RedXnor,
}
/// Binary operators — names mirror sim-ir `BinOp` 1:1 (verified sim-ir/src/lib.rs:112):
/// `AShl`/`AShr` = `<<<`/`>>>`; `Le`/`Ge`/`Ne`; `CaseEq`/`CaseNe` = `===`/`!==`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod, Pow,
    Shl, Shr, AShl, AShr,
    Lt, Le, Gt, Ge,
    Eq, Ne, CaseEq, CaseNe,
    BitAnd, BitXor, BitXnor, BitOr,
    LogAnd, LogOr,
}
/// Indexed part-select direction. Lowers to sim-ir `SelKind::PartIdxUp/Down`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartDir { PlusColon, MinusColon }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntLitKind { Decimal, Sized, UnsizedBased }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RealLitKind { Fixed, Exp }

// ──────────────────────────── Lvalue [P] ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lvalue {
    Ident(HierPath),
    BitSelect   { base: Box<Lvalue>, index: Box<Expr>, span: Span },
    PartSelect  { base: Box<Lvalue>, msb: Box<Expr>, lsb: Box<Expr>, span: Span },
    IndexedPart { base: Box<Lvalue>, offset: Box<Expr>, width: Box<Expr>, dir: PartDir, span: Span },
    Concat      { parts: Vec<Lvalue>, span: Span },  // {cout, sum} = …
    Error(Span),
}
impl Lvalue {
    pub fn span(&self) -> Span {
        match self {
            Lvalue::Ident(h) => h.span,
            Lvalue::BitSelect { span, .. }
            | Lvalue::PartSelect { span, .. }
            | Lvalue::IndexedPart { span, .. }
            | Lvalue::Concat { span, .. }
            | Lvalue::Error(span) => *span,
        }
    }
}

// ──────────────────── ModuleInstance / Generate / TF [A] ────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInstance {
    pub module_name: Ident,
    pub param_overrides: Vec<ParamConn>,   // #(.W(8)) | #(8)
    pub instances: Vec<InstanceItem>,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceItem { pub name: Ident, pub unpacked: Vec<Dim>, pub conns: PortConnList, pub span: Span }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamConn { Named { name: Ident, value: Option<Expr>, span: Span }, Positional(Expr) }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortConnList { Named(Vec<PortConn>), Positional(Vec<Option<Expr>>) }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortConn { pub name: Ident, pub value: Option<Expr>, pub span: Span } // .a(x) / .a()

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateConstruct { pub items: Vec<GenItem>, pub span: Span }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenItem {
    For  { init: GenAssign, cond: Expr, step: GenAssign, label: Option<Ident>, body: Vec<GenItem>, span: Span },
    If   { cond: Expr, then_b: Vec<GenItem>, else_b: Vec<GenItem>, label: Option<Ident>, span: Span },
    Case { scrutinee: Expr, items: Vec<GenCaseItem>, span: Span },
    Block{ label: Option<Ident>, items: Vec<GenItem>, span: Span },
    Item(Box<ModuleItem>),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenAssign { pub lvalue: Ident, pub value: Expr, pub span: Span }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenCaseItem {
    Match { labels: Vec<Expr>, body: Vec<GenItem>, span: Span },
    Default { body: Vec<GenItem>, span: Span },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionDef {
    pub automatic: bool, pub signed: bool, pub range: Option<Range>,
    pub ret_type: ParamType, pub name: Ident,
    pub ports: Vec<TfPort>, pub body_decls: Vec<NetVarDecl>, pub body: Box<Stmt>, pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDef {
    pub automatic: bool, pub name: Ident,
    pub ports: Vec<TfPort>, pub body_decls: Vec<NetVarDecl>, pub body: Box<Stmt>, pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TfPort {
    pub dir: PortDir, pub net_or_var: Option<NetVarKind>,
    pub signed: bool, pub range: Option<Range>, pub name: Ident, pub span: Span,
}
```

**Changes vs prior draft:** added `Dim` enum (M3 — `[N]` representable), `Replicate.value: Vec<Expr>` (M5 — no Concat wrapper), `IndexedPart.offset` rename (M6), full `Stmt` set with `Fork/Assign/Deassign/Force/Release`+`JoinKind` (M7), `Defparam` item, `SysCall`/`SysTaskCall` `$`-retained doc (M6), `Span::to` debug_assert (M4-soundness), `unpacked: Vec<Dim>` on `DeclName`/`InstanceItem`.

---

## 2. `crates/hdl-parser/src/lib.rs`

```rust
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

use hdl_lexer::{Kw, LexErrorKind, Spanned, TokenKind, WordKind};
use hdl_ast::*;

// ───────────────────────────── errors ─────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub span: Span,                  // offending token's span (u32)
    pub expected: &'static str,      // "expression", "';'", "identifier", …
    pub found: Option<TokenKind>,    // None ⇒ EOF
}

// ───────────────────────────── cursor ─────────────────────────────
pub struct Parser<'t, 's> {
    toks: &'t [Spanned],
    src: &'s str,
    pos: usize,
    src_end: u32,
    pub errors: Vec<ParseError>,
    error_limit: usize,
}

impl<'t, 's> Parser<'t, 's> {
    pub fn new(toks: &'t [Spanned], src: &'s str) -> Self {
        Self { toks, src, pos: 0, src_end: src.len() as u32, errors: Vec::new(), error_limit: 50 }
    }

    // -- span helpers --
    #[inline] fn sp(r: &std::ops::Range<usize>) -> Span { Span::new(r.start as u32, r.end as u32) }
    #[inline] fn cur_span(&self) -> Span {
        self.toks.get(self.pos).map(|t| Self::sp(&t.span))
            .unwrap_or(Span::new(self.src_end, self.src_end))
    }
    /// Span of the just-consumed token. VALID ONLY after ≥1 bump (verdict M3-soundness):
    /// at `pos==0` it falls back to `cur_span()` (a safe degenerate), never an
    /// inverted span. Every call site (`start.to(prev_span())`) has bumped first.
    #[inline] fn prev_span(&self) -> Span {
        if self.pos == 0 { return self.cur_span(); }
        self.toks.get(self.pos - 1).map(|t| Self::sp(&t.span))
            .unwrap_or(Span::new(self.src_end, self.src_end))
    }
    /// Raw lexeme of the token at `pos` (re-slice — tokens carry no value).
    fn cur_text(&self) -> &'s str {
        self.toks.get(self.pos).map(|t| &self.src[t.span.clone()]).unwrap_or("")
    }

    // -- cursor primitives --
    #[inline] fn peek(&self) -> Option<TokenKind> { self.toks.get(self.pos).map(|t| t.kind) }
    #[inline] fn peek2(&self) -> Option<TokenKind> { self.toks.get(self.pos + 1).map(|t| t.kind) }
    #[inline] fn at_eof(&self) -> bool { self.pos >= self.toks.len() }
    fn bump(&mut self) -> Option<&'t Spanned> {
        let t = self.toks.get(self.pos); if t.is_some() { self.pos += 1; } t
    }
    fn at_kw(&self, kw: Kw) -> bool {
        matches!(self.peek(), Some(TokenKind::Word(WordKind::Keyword(k))) if k == kw)
    }
    fn is_ident(&self) -> bool {
        matches!(self.peek(), Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent))
    }
    /// True if the next token is a lexer error sentinel (verdict: dedicated handling —
    /// the lexer already emitted the LexError, so we recover WITHOUT re-reporting).
    fn at_lex_error(&self) -> bool { matches!(self.peek(), Some(TokenKind::Error(_))) }
    fn eat(&mut self, k: TokenKind) -> bool {
        if self.peek() == Some(k) { self.pos += 1; true } else { false }
    }
    fn eat_kw(&mut self, kw: Kw) -> bool {
        if self.at_kw(kw) { self.pos += 1; true } else { false }
    }
    /// Consume `k` or record an error (does NOT advance — caller decides to sync).
    fn expect(&mut self, k: TokenKind, what: &'static str) -> bool {
        if self.peek() == Some(k) { self.pos += 1; true } else { self.error(what); false }
    }
    /// Record an error. Suppresses re-reporting on a lexer `Error` token (already
    /// diagnosed by the lexer) — we still record nothing for it, just let the caller
    /// recover. Capped at `error_limit`.
    fn error(&mut self, expected: &'static str) {
        if self.at_lex_error() { return; }  // lexer already emitted a LexError here
        if self.errors.len() < self.error_limit {
            self.errors.push(ParseError { span: self.cur_span(), expected, found: self.peek() });
        }
    }

    // -- ident extraction --
    fn ident(&mut self) -> Option<Ident> {
        if self.is_ident() {
            let t = self.bump().unwrap();
            Some(Ident { name: self.src[t.span.clone()].to_string(), span: Self::sp(&t.span) })
        } else { self.error("identifier"); None }
    }
    fn hier_path(&mut self) -> Option<HierPath> {
        let first = self.ident()?;
        let lo = first.span;
        let mut segs = vec![first];
        while self.peek() == Some(TokenKind::Dot) {
            self.bump();
            match self.ident() { Some(id) => segs.push(id), None => break }
        }
        let hi = segs.last().unwrap().span;
        Some(HierPath { segments: segs, span: lo.to(hi) })
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
                TokenKind::Semi => { self.bump(); return; }
                TokenKind::Word(WordKind::Keyword(
                    Kw::End | Kw::Endmodule | Kw::Endcase | Kw::Endfunction | Kw::Endtask
                    | Kw::Endgenerate | Kw::Join | Kw::Module | Kw::Macromodule | Kw::Assign
                    | Kw::Input | Kw::Output | Kw::Inout | Kw::Wire | Kw::Tri | Kw::Wand
                    | Kw::Triand | Kw::Wor | Kw::Trior | Kw::Tri0 | Kw::Tri1 | Kw::Supply0
                    | Kw::Supply1 | Kw::Trireg | Kw::Uwire | Kw::Reg | Kw::Logic | Kw::Integer
                    | Kw::Real | Kw::Realtime | Kw::Time | Kw::Parameter | Kw::Localparam
                    | Kw::Initial | Kw::Always | Kw::AlwaysFf | Kw::AlwaysComb | Kw::AlwaysLatch
                    | Kw::Generate | Kw::Genvar | Kw::Defparam
                )) => return,
                _ => { self.bump(); }
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
        T::PipePipe                                   => (5, 6),   // ||   lvl13
        T::AmpAmp                                     => (7, 8),   // &&   lvl12
        T::Pipe                                       => (9, 10),  // |    lvl11
        T::Caret | T::TildeCaret | T::CaretTilde      => (11, 12), // ^ ~^ ^~ lvl10
        T::Amp                                        => (13, 14), // &    lvl9
        T::EqEq | T::BangEq | T::EqEqEq | T::BangEqEq => (15, 16), // == != === !== lvl8
        T::Lt | T::LtEq | T::Gt | T::GtEq             => (17, 18), // < <= > >= lvl7
        T::Shl | T::Shr | T::ShlA | T::ShrA           => (19, 20), // << >> <<< >>> lvl6
        T::Plus | T::Minus                            => (21, 22), // + -  lvl5
        T::Star | T::Slash | T::Percent               => (23, 24), // * / % lvl4
        T::StarStar                                   => (26, 25), // **   lvl3 right-assoc
        _ => return None,
    })
}
const TERNARY_LBP: u8 = 4;   // lvl14, right-assoc; rbp = 3
const TERNARY_RBP: u8 = 3;
const UNARY_BP: u8 = 27;     // lvl2, prefix right-assoc — binds tighter than **

fn bin_op(k: TokenKind) -> BinOp {
    use TokenKind as T;
    match k {
        T::StarStar => BinOp::Pow, T::Star => BinOp::Mul, T::Slash => BinOp::Div,
        T::Percent => BinOp::Mod, T::Plus => BinOp::Add, T::Minus => BinOp::Sub,
        T::Shl => BinOp::Shl, T::Shr => BinOp::Shr, T::ShlA => BinOp::AShl, T::ShrA => BinOp::AShr,
        T::Lt => BinOp::Lt, T::LtEq => BinOp::Le, T::Gt => BinOp::Gt, T::GtEq => BinOp::Ge,
        T::EqEq => BinOp::Eq, T::BangEq => BinOp::Ne, T::EqEqEq => BinOp::CaseEq, T::BangEqEq => BinOp::CaseNe,
        T::Amp => BinOp::BitAnd, T::Caret => BinOp::BitXor,
        T::TildeCaret | T::CaretTilde => BinOp::BitXnor, T::Pipe => BinOp::BitOr,
        T::AmpAmp => BinOp::LogAnd, T::PipePipe => BinOp::LogOr,
        _ => unreachable!("bin_op called on non-binary token"),
    }
}
fn prefix_op(k: TokenKind) -> Option<UnOp> {
    use TokenKind as T;
    Some(match k {
        T::Plus => UnOp::Plus, T::Minus => UnOp::Minus, T::Bang => UnOp::LogNot, T::Tilde => UnOp::BitNot,
        T::Amp => UnOp::RedAnd, T::TildeAmp => UnOp::RedNand, T::Pipe => UnOp::RedOr,
        T::TildePipe => UnOp::RedNor, T::Caret => UnOp::RedXor,
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
        || matches!(k, T::Question | T::TildeAmp | T::TildePipe | T::Tilde | T::Bang | T::StarStar)
}

impl<'t, 's> Parser<'t, 's> {
    /// Pratt entry. `min_bp` = caller's right binding power. After the fold loop,
    /// if the next token is operator-class but matched no infix slot, emit one
    /// error (verdict B1: do not silently leave `~& b` unconsumed).
    pub fn expr(&mut self, min_bp: u8) -> Expr {
        let mut lhs = self.expr_prefix();
        loop {
            let Some(op) = self.peek() else { break };
            if op == TokenKind::Question {                         // ternary, right-assoc
                if TERNARY_LBP < min_bp { break; }
                self.bump();
                let then_e = self.expr(0);                         // reset inside branch
                self.expect(TokenKind::Colon, "':' in conditional");
                let else_e = self.expr(TERNARY_RBP);               // right-assoc
                let span = lhs.span.to(else_e.span);
                lhs = Expr { kind: ExprKind::Ternary {
                    cond: Box::new(lhs), then_e: Box::new(then_e), else_e: Box::new(else_e) }, span };
                continue;
            }
            let Some((l_bp, r_bp)) = infix_bp(op) else { break };
            if l_bp < min_bp { break; }
            self.bump();
            let rhs = self.expr(r_bp);
            let span = lhs.span.to(rhs.span);
            lhs = Expr { kind: ExprKind::Binary {
                op: bin_op(op), lhs: Box::new(lhs), rhs: Box::new(rhs) }, span };
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

    fn expr_prefix(&mut self) -> Expr {
        if let Some(op) = self.peek().and_then(prefix_op) {
            let start = self.cur_span();
            self.bump();
            let operand = self.expr(UNARY_BP);                     // lvl2 right-assoc, tighter than **
            let span = start.to(operand.span);
            return Expr { kind: ExprKind::Unary { op, operand: Box::new(operand) }, span };
        }
        self.expr_postfix()
    }

    /// primary, then postfix loop: [idx]/[m:l]/[b+:w]; call(args) handled in primary.
    fn expr_postfix(&mut self) -> Expr {
        let mut e = self.expr_primary();
        while self.peek() == Some(TokenKind::LBracket) {
            e = self.parse_select(e);
        }
        e
    }

    fn parse_select(&mut self, base: Expr) -> Expr {
        let start = base.span;
        self.bump(); // '['
        let first = self.expr(0);
        let kind = match self.peek() {
            Some(TokenKind::Colon)      => { self.bump(); let lsb = self.expr(0);
                ExprKind::PartSelect { base: Box::new(base), msb: Box::new(first), lsb: Box::new(lsb) } }
            Some(TokenKind::PlusColon)  => { self.bump(); let w = self.expr(0);
                ExprKind::IndexedPart { base: Box::new(base), offset: Box::new(first), width: Box::new(w), dir: PartDir::PlusColon } }
            Some(TokenKind::MinusColon) => { self.bump(); let w = self.expr(0);
                ExprKind::IndexedPart { base: Box::new(base), offset: Box::new(first), width: Box::new(w), dir: PartDir::MinusColon } }
            _ => ExprKind::BitSelect { base: Box::new(base), index: Box::new(first) },
        };
        self.expect(TokenKind::RBracket, "']'");
        Expr { kind, span: start.to(self.prev_span()) }
    }

    fn expr_primary(&mut self) -> Expr {
        use TokenKind as T;
        let start = self.cur_span();
        match self.peek() {
            // lexer error sentinel: skip it (already diagnosed), yield Error node
            Some(T::Error(_)) => { self.bump(); Expr { kind: ExprKind::Error, span: start } }
            // numeric / string literals
            Some(T::IntDecimal)        => self.lit_int(IntLitKind::Decimal),
            Some(T::IntSized)          => self.lit_int(IntLitKind::Sized),
            Some(T::IntUnsizedBased)   => self.lit_int(IntLitKind::UnsizedBased),
            Some(T::RealFixed)         => self.lit_real(RealLitKind::Fixed),
            Some(T::RealExp)           => self.lit_real(RealLitKind::Exp),
            Some(T::Str)               => { let raw = self.cur_text().to_string(); self.bump();
                Expr { kind: ExprKind::StrLit { raw }, span: start } }
            // system function call: $time, $signed(x). name retains the `$`.
            Some(T::SystemTask) => {
                let t = self.bump().unwrap();
                let name = Ident { name: self.src[t.span.clone()].to_string(), span: Self::sp(&t.span) };
                let args = if self.peek() == Some(T::LParen) { self.call_args() } else { Vec::new() };
                Expr { kind: ExprKind::SysCall { name, args }, span: start.to(self.prev_span()) }
            }
            // identifier / hierarchical name / function call
            _ if self.is_ident() => {
                let path = self.hier_path().unwrap();
                if self.peek() == Some(T::LParen) {
                    let args = self.call_args();
                    Expr { kind: ExprKind::Call { name: path, args }, span: start.to(self.prev_span()) }
                } else {
                    let sp = path.span;
                    Expr { kind: ExprKind::Ident(path), span: sp }
                }
            }
            // parenthesized / min:typ:max
            Some(T::LParen) => {
                self.bump();
                let inner = self.expr(0);
                if self.peek() == Some(T::Colon) {
                    self.bump(); let typ = self.expr(0);
                    self.expect(T::Colon, "':' in min:typ:max"); let max = self.expr(0);
                    self.expect(T::RParen, "')'");
                    Expr { kind: ExprKind::MinTypMax { min: Box::new(inner), typ: Box::new(typ), max: Box::new(max) },
                           span: start.to(self.prev_span()) }
                } else {
                    self.expect(T::RParen, "')'");
                    Expr { kind: ExprKind::Paren { inner: Box::new(inner) }, span: start.to(self.prev_span()) }
                }
            }
            // concat / replication
            Some(T::LBrace) => self.brace_expr(start),
            _ => { self.error("expression"); Expr { kind: ExprKind::Error, span: start } }
        }
    }

    fn lit_int(&mut self, kind: IntLitKind) -> Expr {
        let start = self.cur_span(); let raw = self.cur_text().to_string(); self.bump();
        Expr { kind: ExprKind::IntLit { kind, raw }, span: start }
    }
    fn lit_real(&mut self, kind: RealLitKind) -> Expr {
        let start = self.cur_span(); let raw = self.cur_text().to_string(); self.bump();
        Expr { kind: ExprKind::RealLit { kind, raw }, span: start }
    }
    fn call_args(&mut self) -> Vec<Expr> {
        self.bump(); // '('
        let mut args = Vec::new();
        if self.peek() != Some(TokenKind::RParen) {
            loop {
                args.push(self.expr(0));
                if !self.eat(TokenKind::Comma) { break; }
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
            while self.eat(TokenKind::Comma) { value.push(self.expr(0)); }
            self.expect(TokenKind::RBrace, "'}' closing replication value");
            self.expect(TokenKind::RBrace, "'}' closing replication");
            return Expr { kind: ExprKind::Replicate { count: Box::new(first), value },
                          span: start.to(self.prev_span()) };
        }
        let mut parts = vec![first];
        while self.eat(TokenKind::Comma) { parts.push(self.expr(0)); }
        self.expect(TokenKind::RBrace, "'}'");
        Expr { kind: ExprKind::Concat { parts }, span: start.to(self.prev_span()) }
    }
}

// ───────────── module / port / param / decl / contassign ─────────────
impl<'t, 's> Parser<'t, 's> {
    fn opt_signed(&mut self) -> bool { self.eat_kw(Kw::Signed) }
    /// `[msb:lsb]` packed range (requires `:`).
    fn opt_range(&mut self) -> Option<Range> {
        if self.peek() != Some(TokenKind::LBracket) { return None; }
        let start = self.cur_span(); self.bump();
        let msb = self.expr(0);
        self.expect(TokenKind::Colon, "':' in range");
        let lsb = self.expr(0);
        self.expect(TokenKind::RBracket, "']'");
        Some(Range { msb, lsb, span: start.to(self.prev_span()) })
    }
    /// Unpacked dimension `[hi:lo]` (Range) or `[N]` (Size) — verdict M3.
    fn parse_dim(&mut self) -> Option<Dim> {
        if self.peek() != Some(TokenKind::LBracket) { return None; }
        self.bump(); // '['
        let first = self.expr(0);
        let dim = if self.peek() == Some(TokenKind::Colon) {
            let r_start = first.span; self.bump();
            let lsb = self.expr(0);
            Dim::Range(Range { msb: first, lsb, span: r_start.to(self.prev_span()) })
        } else {
            Dim::Size(first)
        };
        self.expect(TokenKind::RBracket, "']'");
        Some(dim)
    }
    fn net_var_kind(&self) -> Option<NetVarKind> {
        use Kw::*;
        match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(k))) => Some(match k {
                Wire=>NetVarKind::Wire, Tri=>NetVarKind::Tri, Wand=>NetVarKind::Wand,
                Triand=>NetVarKind::Triand, Wor=>NetVarKind::Wor, Trior=>NetVarKind::Trior,
                Tri0=>NetVarKind::Tri0, Tri1=>NetVarKind::Tri1, Supply0=>NetVarKind::Supply0,
                Supply1=>NetVarKind::Supply1, Trireg=>NetVarKind::Trireg, Uwire=>NetVarKind::Uwire,
                Reg=>NetVarKind::Reg, Logic=>NetVarKind::Logic, Integer=>NetVarKind::Integer,
                Real=>NetVarKind::Real, Realtime=>NetVarKind::Realtime, Time=>NetVarKind::Time,
                _ => return None,
            }),
            _ => None,
        }
    }

    pub fn parse_source_unit(&mut self) -> SourceUnit {
        let start = self.cur_span();
        let mut items = Vec::new();
        while !self.at_eof() {
            let before = self.pos;
            if self.at_kw(Kw::Module) || self.at_kw(Kw::Macromodule) {
                match self.parse_module() {
                    Some(m) => items.push(TopItem::Module(m)),
                    None => { items.push(TopItem::Error(self.prev_span())); self.synchronize(); }
                }
            } else {
                self.error("'module'");
                let s = self.cur_span(); items.push(TopItem::Error(s)); self.synchronize();
            }
            // BLOCKER B3 (top level): guarantee forward progress.
            if self.pos == before { self.bump(); }
        }
        SourceUnit { items, span: start.to(self.prev_span()) }
    }

    fn parse_module(&mut self) -> Option<ModuleDecl> {
        let start = self.cur_span();
        let is_macromodule = self.at_kw(Kw::Macromodule);
        self.bump(); // module / macromodule
        let name = self.ident()?;

        // ANSI param port list: #( parameter … )
        let mut params = Vec::new();
        if self.peek() == Some(TokenKind::Hash) {
            self.bump();
            self.expect(TokenKind::LParen, "'(' after '#'");
            loop {
                if let Some(p) = self.parse_param_decl() { params.push(p); }
                if !self.eat(TokenKind::Comma) { break; }
            }
            self.expect(TokenKind::RParen, "')'");
        }

        // port list: ANSI ( dir type name, … ) | non-ANSI ( name, … ) | none
        let ports = self.parse_port_list();
        self.expect(TokenKind::Semi, "';' after module header");

        // body until endmodule — with forward-progress guard (BLOCKER B3)
        let mut body = Vec::new();
        while !self.at_eof() && !self.at_kw(Kw::Endmodule) {
            let before = self.pos;
            match self.parse_module_item() {
                Some(it) => body.push(it),
                None => { body.push(ModuleItem::Error(self.cur_span())); self.synchronize(); }
            }
            if self.pos == before { self.bump(); } // B3: never spin on a stuck token
        }
        self.expect(TokenKind::Word(WordKind::Keyword(Kw::Endmodule)), "'endmodule'");
        Some(ModuleDecl { is_macromodule, name, params, ports, body, span: start.to(self.prev_span()) })
    }

    /// Decide ANSI vs non-ANSI by the FIRST token inside `(`: a direction keyword
    /// ⇒ ANSI. A bare identifier ⇒ non-ANSI name list. (Documented PR1 limitation,
    /// verdict H2/M1: a malformed ANSI header beginning with a bare net/var kind —
    /// e.g. illegal `module m(reg x)` — is routed to non-ANSI and errors in the body.
    /// Strict V2005 non-ANSI headers are bare-name-only, so this is correct for scope.)
    fn parse_port_list(&mut self) -> PortList {
        if self.peek() != Some(TokenKind::LParen) { return PortList::None; }
        self.bump(); // '('
        if self.peek() == Some(TokenKind::RParen) { self.bump(); return PortList::Ansi(Vec::new()); }
        let ansi = matches!(self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(Kw::Input | Kw::Output | Kw::Inout))));
        if ansi {
            let mut ports = Vec::new();
            let mut prev_dir: Option<PortDir> = None;  // None ⇒ this is the FIRST port
            loop {
                let port = self.parse_ansi_port(prev_dir);
                prev_dir = Some(port.dir);
                ports.push(port);
                if !self.eat(TokenKind::Comma) { break; }
            }
            self.expect(TokenKind::RParen, "')'");
            PortList::Ansi(ports)
        } else {
            let mut names = Vec::new();
            loop {
                if let Some(id) = self.ident() { names.push(id); }
                if !self.eat(TokenKind::Comma) { break; }
            }
            self.expect(TokenKind::RParen, "')'");
            PortList::NonAnsi(names)
        }
    }

    /// `inherited = None` ⇒ first ANSI port; a missing direction is then an ERROR
    /// (verdict M4: don't silently default the first port to Input). Subsequent
    /// ports inherit the previous direction when omitted.
    fn parse_ansi_port(&mut self, inherited: Option<PortDir>) -> AnsiPort {
        let start = self.cur_span();
        let explicit_dir = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::Input)))  => { self.bump(); Some(PortDir::Input) }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Output))) => { self.bump(); Some(PortDir::Output) }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Inout)))  => { self.bump(); Some(PortDir::Inout) }
            _ => None,
        };
        let dir = match (explicit_dir, inherited) {
            (Some(d), _) => d,
            (None, Some(prev)) => prev,                     // inherit
            (None, None) => { self.error("port direction (first ANSI port must specify one)");
                              PortDir::Input }              // recover as Input
        };
        let net_or_var = self.net_var_kind();
        if net_or_var.is_some() { self.bump(); }
        let signed = self.opt_signed();
        let range = self.opt_range();
        let name = self.ident().unwrap_or(Ident { name: String::new(), span: self.cur_span() });
        let default = if self.eat(TokenKind::Eq) { Some(self.expr(0)) } else { None };
        AnsiPort { dir, net_or_var, signed, range, name, default, span: start.to(self.prev_span()) }
    }

    /// Parse one parameter/localparam decl (the keyword is optional on `#(…)`
    /// continuations, defaulting to `Parameter`, which matches IEEE-1364 §12.2).
    fn parse_param_decl(&mut self) -> Option<ParamDecl> {
        let start = self.cur_span();
        let kind = if self.eat_kw(Kw::Localparam) { ParamKind::Localparam }
                   else if self.eat_kw(Kw::Parameter) { ParamKind::Parameter }
                   else { ParamKind::Parameter };
        let signed = self.opt_signed();
        let ty = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::Integer)))  => { self.bump(); ParamType::Integer }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Real)))     => { self.bump(); ParamType::Real }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Realtime))) => { self.bump(); ParamType::Realtime }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Time)))     => { self.bump(); ParamType::Time }
            _ => ParamType::Implicit,
        };
        let range = self.opt_range();
        let name = self.ident()?;
        self.expect(TokenKind::Eq, "'=' in parameter");
        let value = self.expr(0);
        Some(ParamDecl { kind, signed, ty, range, name, value, span: start.to(self.prev_span()) })
    }

    fn parse_module_item(&mut self) -> Option<ModuleItem> {
        // skip a stray lexer error token without re-reporting (already diagnosed)
        if self.at_lex_error() { let s = self.cur_span(); self.bump(); return Some(ModuleItem::Error(s)); }
        // parameter / localparam
        if self.at_kw(Kw::Parameter) || self.at_kw(Kw::Localparam) {
            let p = self.parse_param_decl()?;
            self.expect(TokenKind::Semi, "';'");
            return Some(ModuleItem::Param(p));
        }
        // continuous assign
        if self.at_kw(Kw::Assign) { return self.parse_cont_assign().map(ModuleItem::ContAssign); }
        // non-ANSI body port direction decl
        if matches!(self.peek(), Some(TokenKind::Word(WordKind::Keyword(Kw::Input|Kw::Output|Kw::Inout)))) {
            return self.parse_port_decl().map(ModuleItem::PortDecl);
        }
        // net/var declaration
        if self.net_var_kind().is_some() { return self.parse_net_var().map(ModuleItem::NetVar); }
        // procedural blocks / generate / genvar → recovering STUB (types exist)
        if matches!(self.peek(), Some(TokenKind::Word(WordKind::Keyword(
            Kw::Initial|Kw::Always|Kw::AlwaysFf|Kw::AlwaysComb|Kw::AlwaysLatch|Kw::Generate|Kw::Genvar))))
        {
            let s = self.cur_span();
            self.error("(procedural/generate parsing not yet implemented)");
            self.skip_balanced_block();      // consume @(…) begin…end / generate…endgenerate
            return Some(ModuleItem::Error(s));
        }
        // bare ident ⇒ likely a module instance (deferred) → stub
        if self.is_ident() {
            let s = self.cur_span();
            self.error("(module instantiation parsing not yet implemented)");
            self.synchronize();
            return Some(ModuleItem::Error(s));
        }
        self.error("module item"); None
    }

    fn parse_port_decl(&mut self) -> Option<PortDecl> {
        let start = self.cur_span();
        let dir = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::Input)))  => { self.bump(); PortDir::Input }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Output))) => { self.bump(); PortDir::Output }
            _ => { self.bump(); PortDir::Inout }
        };
        let net_or_var = self.net_var_kind();
        if net_or_var.is_some() { self.bump(); }
        let signed = self.opt_signed();
        let range = self.opt_range();
        let mut names = Vec::new();
        loop { if let Some(id) = self.ident() { names.push(id); } if !self.eat(TokenKind::Comma) { break; } }
        self.expect(TokenKind::Semi, "';'");
        Some(PortDecl { dir, net_or_var, signed, range, names, span: start.to(self.prev_span()) })
    }

    fn parse_net_var(&mut self) -> Option<NetVarDecl> {
        let start = self.cur_span();
        let kind = self.net_var_kind().unwrap(); self.bump();
        let signed = self.opt_signed();
        let range = self.opt_range();
        let mut names = Vec::new();
        loop {
            let n_start = self.cur_span();
            let name = self.ident()?;
            let mut unpacked = Vec::new();
            while self.peek() == Some(TokenKind::LBracket) {
                match self.parse_dim() { Some(d) => unpacked.push(d), None => break }
            }
            let init = if self.eat(TokenKind::Eq) { Some(self.expr(0)) } else { None };
            names.push(DeclName { name, unpacked, init, span: n_start.to(self.prev_span()) });
            if !self.eat(TokenKind::Comma) { break; }
        }
        self.expect(TokenKind::Semi, "';'");
        Some(NetVarDecl { kind, signed, range, names, span: start.to(self.prev_span()) })
    }

    fn parse_cont_assign(&mut self) -> Option<ContinuousAssign> {
        let start = self.cur_span();
        self.bump(); // assign
        let delay = if self.peek() == Some(TokenKind::Hash) { self.parse_delay() } else { None };
        let mut assigns = Vec::new();
        loop {
            let lv = self.parse_lvalue();
            self.expect(TokenKind::Eq, "'=' in assign");
            let rhs = self.expr(0);
            assigns.push((lv, rhs));
            if !self.eat(TokenKind::Comma) { break; }
        }
        self.expect(TokenKind::Semi, "';'");
        Some(ContinuousAssign { delay, assigns, span: start.to(self.prev_span()) })
    }
    /// `#5` | `#(d)` | `#(r,f)` | `#(r,f,t)`. Each paren'd value may be mintypmax
    /// `1:2:3` (verdict M2). Uses `parse_delay_value` which accepts `a:b:c`.
    fn parse_delay(&mut self) -> Option<Delay> {
        let start = self.cur_span(); self.bump(); // '#'
        let mut values = Vec::new();
        if self.eat(TokenKind::LParen) {
            loop { values.push(self.parse_delay_value()); if !self.eat(TokenKind::Comma) { break; } }
            self.expect(TokenKind::RParen, "')'");
        } else {
            // bare `#delay_value`: a single number/ident (no parens) — high bp,
            // no mintypmax (a bare `#1:2:3` is not legal V2005 delay).
            values.push(self.expr(UNARY_BP));
        }
        Some(Delay { values, span: start.to(self.prev_span()) })
    }
    /// A delay value inside `#(…)`: `expr` or `min:typ:max` (verdict M2).
    fn parse_delay_value(&mut self) -> Expr {
        let start = self.cur_span();
        let first = self.expr(0);
        if self.peek() == Some(TokenKind::Colon) {
            self.bump(); let typ = self.expr(0);
            self.expect(TokenKind::Colon, "':' in min:typ:max delay"); let max = self.expr(0);
            Expr { kind: ExprKind::MinTypMax { min: Box::new(first), typ: Box::new(typ), max: Box::new(max) },
                   span: start.to(self.prev_span()) }
        } else { first }
    }

    /// LHS = concat of selects/idents only. Parse directly to `Lvalue`.
    fn parse_lvalue(&mut self) -> Lvalue {
        if self.peek() == Some(TokenKind::LBrace) {
            let start = self.cur_span(); self.bump();
            let mut parts = Vec::new();
            loop { parts.push(self.parse_lvalue()); if !self.eat(TokenKind::Comma) { break; } }
            self.expect(TokenKind::RBrace, "'}'");
            return Lvalue::Concat { parts, span: start.to(self.prev_span()) };
        }
        let Some(path) = self.hier_path() else {
            let s = self.cur_span(); return Lvalue::Error(s);
        };
        let mut lv = Lvalue::Ident(path);
        while self.peek() == Some(TokenKind::LBracket) {
            let start = lv.span(); self.bump();
            let first = self.expr(0);
            lv = match self.peek() {
                Some(TokenKind::Colon)      => { self.bump(); let lsb = self.expr(0); self.expect(TokenKind::RBracket,"']'");
                    Lvalue::PartSelect { base: Box::new(lv), msb: Box::new(first), lsb: Box::new(lsb), span: start.to(self.prev_span()) } }
                Some(TokenKind::PlusColon)  => { self.bump(); let w = self.expr(0); self.expect(TokenKind::RBracket,"']'");
                    Lvalue::IndexedPart { base: Box::new(lv), offset: Box::new(first), width: Box::new(w), dir: PartDir::PlusColon, span: start.to(self.prev_span()) } }
                Some(TokenKind::MinusColon) => { self.bump(); let w = self.expr(0); self.expect(TokenKind::RBracket,"']'");
                    Lvalue::IndexedPart { base: Box::new(lv), offset: Box::new(first), width: Box::new(w), dir: PartDir::MinusColon, span: start.to(self.prev_span()) } }
                _ => { self.expect(TokenKind::RBracket,"']'");
                    Lvalue::BitSelect { base: Box::new(lv), index: Box::new(first), span: start.to(self.prev_span()) } }
            };
        }
        lv
    }

    /// STUB (PR1): consume an `@(…) begin … end` / single stmt / `generate …
    /// endgenerate` body without parsing it, balancing depth so we land cleanly
    /// past it. Has its own forward-progress safety via `at_eof` checks.
    fn skip_balanced_block(&mut self) {
        // skip leading `@(...)` sensitivity if present
        if self.peek() == Some(TokenKind::At) {
            self.bump();
            if self.eat(TokenKind::LParen) {
                let mut depth = 1;
                while depth > 0 {
                    match self.bump().map(|t| t.kind) {
                        Some(TokenKind::LParen) => depth += 1,
                        Some(TokenKind::RParen) => depth -= 1,
                        None => return, _ => {}
                    }
                }
            } else if self.eat(TokenKind::Star) { /* @* */ }
        }
        // begin/end OR generate/endgenerate block, else a single procedural stmt
        let opener_closer = if self.at_kw(Kw::Begin) { Some((Kw::Begin, Kw::End)) }
            else if self.at_kw(Kw::Generate) { Some((Kw::Generate, Kw::Endgenerate)) }
            else { None };
        if let Some((opener, closer)) = opener_closer {
            self.bump();
            let mut depth = 1;
            while depth > 0 && !self.at_eof() {
                if self.at_kw(opener) { depth += 1; self.bump(); }
                else if self.at_kw(closer) { depth -= 1; self.bump(); }
                else { self.bump(); }
            }
        } else {
            self.synchronize(); // single procedural stmt → sync to ';'
        }
    }
}

/// Public API — mirrors `hdl_lexer::lex`'s two-channel shape. Never panics; returns
/// a (partial) AST plus all recovered errors. The driver maps errors → diagnostics
/// (E-PARSE-UNEXPECTED-TOKEN / VITA-E2002) and enforces `--error-limit`.
/// Empty input ⇒ `(None, [])`.
pub fn parse(tokens: &[Spanned], src: &str) -> (Option<SourceUnit>, Vec<ParseError>) {
    let mut p = Parser::new(tokens, src);
    let unit = p.parse_source_unit();
    let su = if unit.items.is_empty() && p.errors.is_empty() { None } else { Some(unit) };
    (su, p.errors)
}
```

---

## 3. Both `Cargo.toml`

**`crates/hdl-ast/Cargo.toml`:**
```toml
[package]
name = "hdl-ast"
version = "0.0.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
serde = { workspace = true }
# vita-schema / vita-artifact-derive: DEFERRED. The derive's catch-all arm
# (vita-artifact-derive/src/lib.rs:248) renders Box<Expr> -> bare "Box" and emits
# <Box as SchemaShape>::register() (won't compile). Add both crates + #[derive(
# SchemaHash)] once the one-arm Box passthrough lands (Residual 1).

[dev-dependencies]
postcard = { workspace = true }   # round-trip AST in unit tests (workspace-pinned;
                                  # avoids a new serde_json tree — verdict L4)
```

**`crates/hdl-parser/Cargo.toml`:**
```toml
[package]
name = "hdl-parser"
version = "0.0.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
hdl-lexer = { path = "../hdl-lexer" }
hdl-ast   = { path = "../hdl-ast" }
# NO winnow (hand-RD; doc-02 hot+recovery target; absent from workspace deps).
# NO diag (parser stays diag-light like the lexer; the driver maps ParseError).
```

---

## 4. Verified precedence → binding powers

Source of truth: `hdl-reference/verilog/03-expressions-operators.md` (14-level table, level 1 = highest). Mapping: **higher doc level number = looser binding = lower bp**. Verified the table's two worked gotchas reproduce.

| Doc lvl | Operators | Assoc | tokens | (lbp, rbp) |
|---|---|---|---|---|
| 1 | `()` `[]` | — | LParen/LBracket | postfix/primary (structural, not in `infix_bp`) |
| 2 | `! ~ +u -u & ~& \| ~\| ^ ~^` | R→L | Bang/Tilde/Plus/Minus/Amp/TildeAmp/Pipe/TildePipe/Caret/TildeCaret | prefix bp = **27** |
| 3 | `**` | R→L | StarStar | (26, 25) |
| 4 | `* / %` | L→R | Star/Slash/Percent | (23, 24) |
| 5 | `+ -` | L→R | Plus/Minus | (21, 22) |
| 6 | `<< >> <<< >>>` | L→R | Shl/Shr/ShlA/ShrA | (19, 20) |
| 7 | `< <= > >=` | L→R | Lt/LtEq/Gt/GtEq | (17, 18) |
| 8 | `== != === !==` | L→R | EqEq/BangEq/EqEqEq/BangEqEq | (15, 16) |
| 9 | `&` | L→R | Amp | (13, 14) |
| 10 | `^ ~^ ^~` | L→R | Caret/TildeCaret/CaretTilde | (11, 12) |
| 11 | `\|` | L→R | Pipe | (9, 10) |
| 12 | `&&` | L→R | AmpAmp | (7, 8) |
| 13 | `\|\|` | L→R | PipePipe | (5, 6) |
| 14 | `?:` | R→L | Question | TERNARY_LBP=4, RBP=3 (special-cased, NOT in `infix_bp`) |

Left-assoc ⇒ `rbp = lbp+1`; right-assoc (`**`, `?:`) ⇒ `rbp = lbp-1`. Unary `27 > 26` so `-a**b` = `(-a)**b`... — **note**: IEEE actually makes `**` bind tighter than unary on the *left operand* only via right-assoc; the doc places unary (lvl2) above `**` (lvl3), so `-a ** b` parses as `(-a) ** b`. The prefix-bp=27 > `**`-lbp=26 encodes exactly the doc's lvl2 > lvl3. Note the dead `Question` row was removed from `infix_bp` (verdict m1).

---

## 5. Parse test cases (→ `#[cfg(test)]`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use hdl_ast::*;

    fn p(src: &str) -> (Option<SourceUnit>, Vec<ParseError>) {
        let (toks, lex_errs) = hdl_lexer::lex(src);
        assert!(lex_errs.is_empty(), "lex errors: {lex_errs:?}");
        parse(&toks, src)
    }
    fn first_module(su: &SourceUnit) -> &ModuleDecl {
        match &su.items[0] { TopItem::Module(m) => m, _ => panic!("not a module") }
    }
    /// Parse a bare expression via `assign x = <expr>;` and return the RHS.
    fn expr_of(src: &str) -> Expr {
        let wrapped = format!("module m; assign x = {src};\nendmodule");
        let (su, errs) = p(&wrapped);
        assert!(errs.is_empty(), "parse errors for `{src}`: {errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        match &m.body[0] { ModuleItem::ContAssign(ca) => ca.assigns[0].1.clone(), _ => panic!() }
    }
    fn bin(e: &Expr) -> (BinOp, &Expr, &Expr) {
        match &e.kind { ExprKind::Binary { op, lhs, rhs } => (*op, lhs, rhs),
                        other => panic!("not binary: {other:?}") }
    }

    // 1. mul binds tighter than add:  a + b * c  =>  +(a, *(b,c))
    #[test] fn t1_mul_tighter_than_add() {
        let (op, _l, r) = { let e = expr_of("a + b * c"); let (o,l,r)=bin(&e); (o,l.clone(),r.clone()) };
        assert_eq!(op, BinOp::Add);
        assert_eq!(bin(&r).0, BinOp::Mul);
    }

    // 2. ternary right-assoc:  a ? b : c ? d : e  =>  a ? b : (c ? d : e)
    #[test] fn t2_ternary_right_assoc() {
        let e = expr_of("a ? b : c ? d : e");
        let ExprKind::Ternary { else_e, .. } = &e.kind else { panic!() };
        assert!(matches!(else_e.kind, ExprKind::Ternary { .. }));
    }

    // 3. concat LHS + left-assoc add:  assign {cout,sum} = a + b + cin;
    #[test] fn t3_concat_lhs_left_assoc() {
        let (su, errs) = p("module m; assign {cout, sum} = a + b + cin;\nendmodule");
        assert!(errs.is_empty(), "{errs:?}");
        let m = first_module(&su.unwrap());
        let ModuleItem::ContAssign(ca) = &m.body[0] else { panic!() };
        let Lvalue::Concat { parts, .. } = &ca.assigns[0].0 else { panic!("LHS not concat") };
        assert_eq!(parts.len(), 2);
        let (op, l, _r) = bin(&ca.assigns[0].1);
        assert_eq!(op, BinOp::Add);
        assert_eq!(bin(l).0, BinOp::Add); // left child is (a+b)  → left-assoc
    }

    // 4. ANSI #(param)(ports) + direction inheritance
    #[test] fn t4_ansi_header() {
        let (su, errs) = p("module adder #(parameter WIDTH = 8)\
            (input [WIDTH-1:0] a, b, output [WIDTH-1:0] sum);\nendmodule");
        assert!(errs.is_empty(), "{errs:?}");
        let m = first_module(&su.unwrap());
        assert_eq!(m.name.name, "adder");
        assert_eq!(m.params.len(), 1);
        assert_eq!(m.params[0].kind, ParamKind::Parameter);
        let PortList::Ansi(ports) = &m.ports else { panic!("not ANSI") };
        assert_eq!(ports.len(), 3);
        assert_eq!(ports[0].dir, PortDir::Input);
        assert_eq!(ports[1].dir, PortDir::Input);  // `b` inherits
        assert_eq!(ports[2].dir, PortDir::Output);
    }

    // 5. non-ANSI module: header names + body dir/type
    #[test] fn t5_non_ansi() {
        let (su, errs) = p("module m(a, b, y);\n  input a, b;\n  output y;\n  wire [3:0] tmp;\n\
            assign y = a & b;\nendmodule");
        assert!(errs.is_empty(), "{errs:?}");
        let m = first_module(&su.unwrap());
        let PortList::NonAnsi(names) = &m.ports else { panic!("not non-ANSI") };
        assert_eq!(names.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["a","b","y"]);
        assert!(matches!(m.body[0], ModuleItem::PortDecl(_)));
        assert!(m.body.iter().any(|i| matches!(i, ModuleItem::NetVar(_))));
        assert!(m.body.iter().any(|i| matches!(i, ModuleItem::ContAssign(_))));
    }

    // 6. vector range is an expr, not pre-evaluated:  wire [WIDTH-1:0] bus;
    #[test] fn t6_range_is_expr() {
        let (su, _e) = p("module m; wire [WIDTH-1:0] bus;\nendmodule");
        let m = first_module(&su.unwrap());
        let ModuleItem::NetVar(nv) = &m.body[0] else { panic!() };
        let r = nv.range.as_ref().unwrap();
        assert_eq!(bin(&r.msb).0, BinOp::Sub);
        assert!(matches!(r.lsb.kind, ExprKind::IntLit { .. }));
    }

    // 7. indexed part-select [b+:w]
    #[test] fn t7_indexed_part_select() {
        let e = expr_of("data[base +: 8]");
        let ExprKind::IndexedPart { dir, .. } = &e.kind else { panic!("{:?}", e.kind) };
        assert_eq!(*dir, PartDir::PlusColon);
    }

    // 8. & tighter than | :  a & b | c  =>  |(&(a,b), c)
    #[test] fn t8_and_tighter_than_or() {
        let e = expr_of("a & b | c");
        let (op, l, _r) = bin(&e);
        assert_eq!(op, BinOp::BitOr);
        assert_eq!(bin(l).0, BinOp::BitAnd);
    }

    // 9. unary tighter than equality:  !a == b  =>  ==(!a, b)
    #[test] fn t9_unary_tighter_than_eq() {
        let e = expr_of("!a == b");
        let (op, l, _r) = bin(&e);
        assert_eq!(op, BinOp::Eq);
        assert!(matches!(l.kind, ExprKind::Unary { op: UnOp::LogNot, .. }));
    }

    // 10. add tighter than shift (the doc's #1 gotcha):  a + b << 2  =>  (a+b) << 2
    #[test] fn t10_add_tighter_than_shift() {
        let e = expr_of("a + b << 2");
        let (op, l, _r) = bin(&e);
        assert_eq!(op, BinOp::Shl);
        assert_eq!(bin(l).0, BinOp::Add);
    }

    // 11. replication value is a Vec, NOT a Concat wrapper (verdict M5):  {3{a}}
    #[test] fn t11_replication_value_is_vec() {
        let e = expr_of("{3{a}}");
        let ExprKind::Replicate { count, value } = &e.kind else { panic!("{:?}", e.kind) };
        assert!(matches!(count.kind, ExprKind::IntLit { .. }));
        assert_eq!(value.len(), 1);
        assert!(matches!(value[0].kind, ExprKind::Ident(_))); // bare `a`, not Concat{[a]}
    }

    // 12. mintypmax delay (verdict M2):  assign #(1:2:3) y = a;
    #[test] fn t12_mintypmax_delay() {
        let (su, errs) = p("module m; assign #(1:2:3) y = a;\nendmodule");
        assert!(errs.is_empty(), "{errs:?}");
        let m = first_module(&su.unwrap());
        let ModuleItem::ContAssign(ca) = &m.body[0] else { panic!() };
        let d = ca.delay.as_ref().unwrap();
        assert_eq!(d.values.len(), 1);
        assert!(matches!(d.values[0].kind, ExprKind::MinTypMax { .. }));
    }

    // 13. recovery continues after a bad item (uses a lexer-error token `@`-stray
    //     plus garbage); the trailing valid assign still parses (verdict B3).
    #[test] fn t13_recovery_continues() {
        let (su, errs) = p("module m; wire @ ; assign y = a;\nendmodule");
        assert!(!errs.is_empty(), "expected a recovered error");
        let m = first_module(&su.unwrap());
        assert!(m.body.iter().any(|i| matches!(i, ModuleItem::ContAssign(_))),
                "parser must recover and parse the trailing assign");
    }

    // 14. termination edges (verdict H3-soundness): must not hang / must terminate.
    #[test] fn t14_termination_edges() {
        assert_eq!(p("").0, None);                 // empty input ⇒ (None, [])
        let _ = p("module");                       // truncated header
        let _ = p("module module;");               // sync-anchor == entry-token trap
        let _ = p("module m; endmodule extra ;");  // trailing junk
        // reaching here without hang is the assertion
    }

    // 15. ** right-assoc and unary precedence:  -a ** b  =>  (-a) ** b ; 2**3**4 right
    #[test] fn t15_pow_assoc_and_unary() {
        let e = expr_of("2 ** 3 ** 4");
        let (op, _l, r) = bin(&e);
        assert_eq!(op, BinOp::Pow);
        assert_eq!(bin(&r.clone()).0, BinOp::Pow);  // right child is 3**4 (right-assoc)
        let e2 = expr_of("- a ** b");
        let (op2, l2, _r2) = bin(&e2);
        assert_eq!(op2, BinOp::Pow);                 // top is **
        assert!(matches!(l2.kind, ExprKind::Unary { op: UnOp::Minus, .. })); // left is (-a)
    }
}
```

15 cases (≥ the 8–10 requested): precedence (t1, t8, t9, t10, t15), associativity (t2 ternary-R, t3 add-L, t15 pow-R), ANSI/non-ANSI (t4, t5), concat-LHS (t3), part-select (t7), range-as-expr (t6), replication contract (t11), mintypmax delay (t12), recovery (t13), termination (t14).

---

## 6. Coverage statement (MVP construct → AST type; parser does/defers)

| MVP construct (doc-01 Phase-1) | AST type | PR1 parser |
|---|---|---|
| `module`/`macromodule` + `endmodule` | `ModuleDecl` | **PARSE** |
| ANSI `#(params)(ports)` | `params: Vec<ParamDecl>` + `PortList::Ansi` | **PARSE** (incl. dir inheritance, M4 first-port error) |
| non-ANSI `(names); input…` | `PortList::NonAnsi` + body `PortDecl` | **PARSE** |
| `parameter`/`localparam` | `ParamDecl` / `ParamKind` | **PARSE** (header `#()` + body) |
| wire/reg/logic/integer + ranges + signed | `NetVarDecl`/`NetVarKind`/`Range`/`signed` | **PARSE** |
| packed/unpacked arrays `[m:l]`/`[N]` | `range: Option<Range>` + `unpacked: Vec<Dim>` | **PARSE** (`Dim::Range`/`Size`) |
| `assign` (+ delay, concat LHS) | `ContinuousAssign`/`Delay`/`Lvalue` | **PARSE** (incl. mintypmax delay M2) |
| full expression operator set | `Expr`/`ExprKind`/`UnOp`/`BinOp` | **PARSE** (precedence-correct, §4) |
| concat `{}` / replication `{{}}` | `Concat` / `Replicate{value:Vec}` | **PARSE** (M5 contract) |
| bit/part/indexed select | `BitSelect`/`PartSelect`/`IndexedPart` | **PARSE** (expr + lvalue) |
| function/system-function calls | `Call` / `SysCall` (`$`-retained, M6) | **PARSE** |
| hierarchical `a.b.c` | `HierPath` | **PARSE** |
| literals (int/real/string, raw) | `IntLit`/`RealLit`/`StrLit` | **PARSE** (raw lexeme) |
| initial/always/always_ff/comb/latch + `@(…)` | `ProceduralBlock`/`ProcKind`/`Sensitivity`/`Edge` | **TYPES ONLY** — body `skip_balanced_block` → `ModuleItem::Error` |
| statements (=, <=, if/case/for/while/repeat/forever, begin-end, systask, #delay, @event, wait, fork, force…) | `Stmt` (full set, M7) / `CaseItem` / `JoinKind` | **TYPES ONLY** (deferred parse) |
| module instantiation | `ModuleInstance`/`PortConn`/`ParamConn` | **TYPES ONLY** → stub `Error` |
| generate / genvar | `GenerateConstruct`/`GenItem`/`Genvar` | **TYPES ONLY** → stub `Error` |
| function/task defs | `FunctionDef`/`TaskDef`/`TfPort` | **TYPES ONLY** (deferred) |
| `defparam` | `DefparamItem` | **TYPES ONLY** (deferred) |

Every MVP construct is **representable** in hdl-ast. The parser first-slice covers the structural + full-expression subset end-to-end; procedural/instance/generate/TF **parsing** is deferred but their AST types are frozen now (so SchemaHash, when enabled, hashes the final shape).

---

## 7. Residual risks + deferred follow-ups

1. **`SchemaHash` on hdl-ast (top follow-up).** Blocked by `vita-artifact-derive/src/lib.rs:248`. Cheapest fix: add transparent arm `("Box",1) => render_type_expr(args[0], children)` beside the `Option`/`Vec` arms (lib.rs:240); then add `vita-schema`+derive deps, `#[derive(SchemaHash)]`, and a determinism guard test. Alt: move `Expr`/`Stmt` to a `u32` index-arena like sim-ir (larger churn). Until then the `.vu` schema_hash root stays unlocked (doc-14 §5), consistent with sim-ir deferring its M3 freeze.
2. **Statement / procedural-block parsing** (types complete, incl. the M7 full set): `always`/`initial` bodies, `if`/`case(z/x)`/`for`/`while`/`repeat`/`forever`, `begin…end` (named, E2009 end-label match), `fork…join(_any/_none)`, `= `/`<=`, `#delay`/`@(event)`/`wait`, procedural `assign`/`deassign`/`force`/`release`. Currently `skip_balanced_block`→`Error`.
3. **Module instantiation parsing** (`ModuleInstance` complete): `ident #(...) inst(.p(e), …);`, gate-primitive vs instance disambiguation. Currently bare-ident-in-body → stub.
4. **Generate/genvar parsing** (`GenerateConstruct` complete). Currently skipped.
5. **Function/task parsing** (`FunctionDef`/`TaskDef` complete). Deferred.
6. **Driver-level diagnostics** (not in these crates): `ParseError → diag::Diagnostic` (E-PARSE-UNEXPECTED-TOKEN/E2002), `E-DUP-UNIT`/E2001 (work-lib dedup), `F-LIMIT-ERRORS`/F0002 on `--error-limit`, `W-PARSE-IMPLICIT-NET`/W2003 (needs elaborate symbol context).
7. **Reserved future parse codes** to split out of the catch-all E2002 (parser keeps `expected: &'static str`; widen to `&'static [&'static str]` when these land — verdict n2): E2004 unsized-concat, E2005 zero-replication, E2006 reserved-kw-as-ident, E2007 illegal-number, E2009 end-label-mismatch, E2011 null-portlist, E2012 decl-after-stmt.
8. **Known PR1 parser divergences** (documented, bounded by V2005 scope):
   - **ANSI/non-ANSI heuristic** (H2/M1): first-token = direction-kw ⇒ ANSI, else bare-name list. An illegal ANSI header starting with a bare net/var kind routes to non-ANSI and errors in the body. Correct for strict V2005.
   - **Brace replication** (M1-soundness): `{ a {b} }` without a comma is read as replication. Semantic-check (constness, illegal-comma) is elaborate's job; deferred.
   - `@*` and `@(*)` both lower to `Sensitivity::Star` (syntactic distinction lost; semantically identical — m5).
   - `Paren` is kept for span fidelity but `(a:b:c)` → `MinTypMax` strips the parens (m7); spans still present on the node.
9. **`Ident` interning** (`String` → `u32 Symbol`) once a symbol table exists — perf only, determinism-neutral (L3).
10. **Lexer `Error` token** is now handled without re-reporting (`at_lex_error` guard in `error()`, `expr_primary`, `parse_module_item`); the lexer already emitted the `LexError`. A further refinement (carrying the original `LexErrorKind` into a typed parse-recovery code) is deferred.

**Key correction preserved from review:** all three research outputs recommended deriving `SchemaHash` on hdl-ast now; verified against `vita-artifact-derive` source, that does **not compile** for a `Box`-recursive tree. PR1 ships **serde-only**; SchemaHash is a one-arm-derive-fix follow-up. Everything else — hand-RD + Pratt, no `winnow`/`diag`, `Span{lo,hi:u32}`, the binding-power table, `(Option<SourceUnit>, Vec<ParseError>)`, sim-ir `UnOp`/`BinOp` 1:1 mirror — is confirmed against the live tree. **BLOCKERs B1 (operator-leftover error), B3 (forward-progress guards at both loop levels) and MAJORs M2/M4/M5/M6/M7 are resolved in the code above.**

Files (greenfield, to be created): `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl/crates/hdl-ast/src/lib.rs`, `/crates/hdl-ast/Cargo.toml`, `/crates/hdl-parser/src/lib.rs`, `/crates/hdl-parser/Cargo.toml`.