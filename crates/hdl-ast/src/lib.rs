//! hdl-ast — the parsed AST for the vitamin front-end (preprocess → lex → PARSE → …).
//!
//! Produced by `hdl-parser`, consumed by `elaborate` (which lowers it to the
//! span-free `sim-ir`, dropping spans into a side-table). Unlike sim-ir, **every
//! node carries a source span** (doc-14 §1: the `.vu` body = "hdl-ast 단위 트리 …
//! + 소스 스팬"). Spans are `u32` byte offsets → deterministic, `.vu`-safe.
//!
//! ## Serialization decision (load-bearing — verified against the derive source)
//! These types derive `Serialize + Deserialize` so elaborate can write the `.vu`
//! body, AND `SchemaHash` so the `.vu` container can gate staleness against the
//! `SourceUnit` shape. The `Box`-recursive AST is hashable because the derive
//! carries a transparent `("Box", 1)` arm (`vita-artifact-derive`) that renders a
//! `Box<T>` as its inner `T` — alongside the `Option`/`Vec`/`BTreeSet` arms —
//! instead of emitting a bare `<Box as SchemaShape>::register`. The shapes obey
//! the determinism rules (no usize/HashMap/float; `Span` = two `u32`), so the
//! `schema_hash::<SourceUnit>()` root is pinned by the golden gate in
//! `tests/schema_hash.rs`. Field add/remove/reorder flips that root, which
//! invalidates every `.vu` on disk (intentional — a `format_version` bump and a
//! golden update must accompany any deliberate change).

use serde::{Deserialize, Serialize};
use vita_artifact_derive::SchemaHash;

// ───────────────────────────── Span ─────────────────────────────
/// Half-open byte range `[lo, hi)` into the preprocessed source. `u32` (not the
/// lexer's `Range<usize>`) so the serialized shape is deterministic across OSes.
/// The parser narrows each `Spanned.span: Range<usize>` at node construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Span {
    pub lo: u32,
    pub hi: u32,
}
impl Span {
    #[inline]
    pub fn new(lo: u32, hi: u32) -> Self {
        Self { lo, hi }
    }
    /// Span union: `self.lo .. max(other.hi, self.lo)`. The normal case is a
    /// strictly-monotonic cursor (`start.to(prev_span())`), where `other.hi >=
    /// self.lo` already holds and the union is `self.lo .. other.hi` byte for byte.
    #[inline]
    pub fn to(self, other: Span) -> Span {
        // CLAMP (verdict M2): a recovery path that composes spans out of order —
        // a parser header whose tokens never advanced past `start` (e.g.
        // `generate for endgenerate`, or PR2's `initial for end`) — would otherwise
        // yield an inverted `[lo, hi)` with `hi < lo`. The old `debug_assert!`
        // PANICKED there (a debug/test-only DoS on truncated input); release
        // silently produced a wrong span. Flooring `hi` at `lo` makes the union
        // total and non-panicking on EVERY input while preserving the normal
        // monotonic-cursor case byte for byte (where `other.hi >= self.lo` already
        // holds), so the determinism / golden-hash contract is unchanged.
        Span {
            lo: self.lo,
            hi: other.hi.max(self.lo),
        }
    }
}

/// An identifier reference: raw lexeme (re-sliced from source by the parser) + span.
/// `EscapedIdent` keeps its raw `\…` form; stripping the leading `\` and the
/// trailing-whitespace rule is the consumer's job. Interning to a `u32 Symbol` is
/// a later optimization (Residual 9); `String` keeps PR1 simple, determinism-safe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// Hierarchical name `a` | `a.b.c`. One-segment is the common case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct HierPath {
    pub segments: Vec<Ident>,
    pub span: Span,
}

// ──────────────────────────── SourceUnit ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct SourceUnit {
    pub items: Vec<TopItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum TopItem {
    Module(ModuleDecl),
    /// `interface name; … endinterface` (v5 ⑥). The body SHAPE reuses
    /// `ModuleDecl` (params/signals/cont-assigns/procs come for free);
    /// elaborate keeps interfaces in their own map and flattens an instance
    /// into plain nets + symbol aliases (spike 2026-06-10: no new IR).
    Interface(ModuleDecl),
    /// `package name; … endpackage` (v7 P2-D). Body shape reuses
    /// `ModuleDecl` like interfaces (params/typedefs/funcs/tasks); elaborate
    /// flattens imported symbols by name — no IR.
    Package(ModuleDecl),
    /// Compilation-unit-scope `import pkg::*;` / `import pkg::sym;` (v7) —
    /// one item per comma-separated term.
    Import(ImportDecl),
    /// `class NAME [extends BASE]; … endclass` (N7). Top-level OOP class
    /// declaration. Elaborate lowers to an engine-side heap (`class_heap`) +
    /// sidecar layout/vtable tables — pure IR-0 (no sim-ir/format_version
    /// change), like interfaces/packages.
    Class(ClassDecl),
    /// Recovery placeholder for an unparseable top-level construct.
    Error(Span),
}

/// One `import` term (v7 P2-D): `pkg::*` (`item: None`) or `pkg::sym`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ImportDecl {
    pub pkg: Ident,
    pub item: Option<Ident>,
    pub span: Span,
}

/// `class NAME [extends BASE]; … endclass` (N7). Data members (`Property`) and
/// methods (`Func`/`Task`, possibly `virtual`). Lowered by elaborate to a
/// `class_heap` (engine side) + layout/vtable sidecars — pure IR-0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ClassDecl {
    pub name: Ident,
    /// `class C #(int W = 8, …)` value parameters (ⓑ-breadth, §8.25). Empty for a
    /// non-parameterized class. Elaborate MONOMORPHIZES: each distinct `C #(args)`
    /// specialization is a concrete class with the params substituted by value.
    pub params: Vec<ClassParam>,
    /// `extends BASE` single-inheritance base name (`None` = root class).
    pub extends: Option<Ident>,
    pub items: Vec<ClassItem>,
    pub span: Span,
}

/// A class value parameter `#(int NAME = DEFAULT)` (ⓑ-breadth, §8.25). v1 supports
/// value parameters (no `type` parameters); `default` is `None` for a parameter
/// with no default (every specialization must then override it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ClassParam {
    pub name: Ident,
    pub default: Option<Expr>,
}

/// Member access control (IEEE §8.18). `Public` is the default (no qualifier);
/// `Local`/`Protected` are the `local`/`protected` qualifiers. `static`/`const`/
/// `pure`/`extern` remain loud-rejected at the parser (separate slices).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash, Default)]
pub enum Visibility {
    /// No qualifier — accessible everywhere (the default).
    #[default]
    Public,
    /// `local` — accessible only from within the SAME class's own methods.
    Local,
    /// `protected` — accessible from the same class AND derived classes.
    Protected,
}

/// A member of a [`ClassDecl`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum ClassItem {
    /// A data member: `int x;`, `logic [7:0] b;`, a class-typed handle, etc.
    /// `vis` is the `local`/`protected`/public access control (IEEE §8.18).
    Property(Visibility, NetVarDecl),
    /// A `rand`/`randc` data member (N7-REST). `randc` = cyclic random. The
    /// declaration is an ordinary data member; the flag drives `randomize()`.
    RandProperty { randc: bool, decl: NetVarDecl },
    /// A `constraint NAME { expr; … }` block (N7-REST). Each expr is a boolean
    /// constraint over the class's rand members.
    Constraint(ConstraintDecl),
    /// A method `[virtual] function … endfunction` (the constructor is a
    /// function named `new`). `is_virtual` drives the vtable; `vis` is the
    /// `local`/`protected`/public access control (IEEE §8.18).
    Func {
        is_virtual: bool,
        vis: Visibility,
        def: FunctionDef,
    },
    /// A method `[virtual] task … endtask`.
    Task {
        is_virtual: bool,
        vis: Visibility,
        def: TaskDef,
    },
    /// Recovery placeholder for an unparseable class item.
    Error(Span),
}

/// `constraint NAME { constraint_expr ; … }` (N7-REST). The body is a list of
/// boolean constraint expressions over the enclosing class's `rand` members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ConstraintDecl {
    pub name: Ident,
    pub exprs: Vec<Expr>,
    /// Parallel to `exprs`: whether each constraint expression was declared `soft`
    /// (IEEE §18.5.14). A soft constraint is satisfied if feasible, else dropped.
    pub soft: Vec<bool>,
    pub span: Span,
}

// ──────────────────────────── Module ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ModuleDecl {
    pub is_macromodule: bool, // `module` vs `macromodule`
    pub name: Ident,
    pub params: Vec<ParamDecl>, // ANSI `#( … )` param port list (empty if none)
    pub ports: PortList,
    pub body: Vec<ModuleItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum PortList {
    /// ANSI: header carries dir + type + range inline.
    Ansi(Vec<AnsiPort>),
    /// non-ANSI: header = bare names; dir/type come from body `PortDecl`s.
    NonAnsi(Vec<Ident>),
    /// `module m;` — no port parenthesis at all.
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct AnsiPort {
    pub dir: PortDir,
    pub net_or_var: Option<NetVarKind>, // None ⇒ default wire
    pub signed: bool,
    pub range: Option<Range>, // packed [msb:lsb] (the FIRST/outer packed dim)
    pub packed: Vec<Range>,   // ADDITIONAL packed dims: `[3:0][7:0]` ⇒ range=[3:0], packed=[[7:0]]
    pub name: Ident,
    pub default: Option<Expr>, // ANSI default value slot (rare)
    /// Interface-typed port `intf p` / `intf.mp p` (v5 ⑥). When set, `dir`
    /// is a placeholder (interface ports carry no direction) and elaborate
    /// binds the port by SYMBOL ALIASING instead of cont-assign wiring.
    pub iface: Option<IfaceRef>,
    pub span: Span,
}

/// The interface type of an ANSI port: `intf` or `intf.modport`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct IfaceRef {
    pub iface: Ident,
    pub modport: Option<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum PortDir {
    Input,
    Output,
    Inout,
}

// ──────────────────────────── ModuleItem ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum ModuleItem {
    NetVar(NetVarDecl),
    Param(ParamDecl),   // body-level parameter/localparam
    PortDecl(PortDecl), // non-ANSI body port-direction decl
    ContAssign(ContinuousAssign),
    Proc(ProceduralBlock),       // [A] type defined; body stub-parsed in PR1
    Instance(ModuleInstance),    // [A]
    Generate(GenerateConstruct), // [A]
    Genvar {
        names: Vec<Ident>,
        span: Span,
    }, // [A]
    Func(FunctionDef),           // [A]
    Task(TaskDef),               // [A]
    Defparam(DefparamItem),      // [A] defparam path = expr;
    Typedef(TypedefDecl),        // SV `typedef enum/struct/<type> name;` (Phase-2)
    Import(ImportDecl),          // v7 P2-D module-scope `import pkg::…;`
    /// `modport mp (input a, output b);` (v5 ⑥ — parsed and ACCEPTED; the
    /// per-member direction checks are a follow-on increment).
    Modport(ModportDecl),
    /// Named SVA `sequence NAME; …; endsequence` (Phase-3 named-SVA slice). Inlined
    /// at use sites by elaborate; pure IR-0.
    SequenceDecl(SeqDecl),
    /// Named SVA `property NAME; …; endproperty` (Phase-3 named-SVA slice). Spliced
    /// at `assert property(NAME)` by elaborate; pure IR-0.
    PropertyDecl(PropDecl),
    /// `covergroup NAME; coverpoint EXPR; … endgroup` — functional coverage model
    /// (N5). Lowered to a CoverageModel side-table (out-of-band, golden-free).
    Covergroup(CovergroupDecl),
    /// `cg c = new;` — a covergroup instance; registers a coverage tracker.
    CoverInstance(CoverInstance),
    /// `class NAME …; … endclass` declared inside a module/package body (N7).
    Class(ClassDecl),
    /// `let NAME [(formals)] = expr;` (SVA-REST, IEEE 1800 §11.13) — a named
    /// expression macro. Substituted at each use site by elaborate (a use is a plain
    /// `Ident` / `Call` that resolves against the let table); pure IR-0.
    LetDecl(LetDecl),
    /// `clocking NAME @(event); input/output sig; endclocking` (N4, IEEE 1800 §14).
    /// Elaborate synthesizes preponed-sampled holding nets for the inputs + a marked
    /// clocking-commit handler; `cb.sig` resolves to the holding net, `@(cb)` to the
    /// clocking event. Out-of-band sidecars (golden-free for non-clocking designs).
    Clocking(ClockingDecl),
    /// Recovery placeholder for an unparseable item.
    Error(Span),
}

/// A `clocking [NAME] @(event); { [default] input/output [skew] sig [= expr]; }
/// endclocking` block (N4, IEEE 1800 §14). v1 supports default-skew INPUT sampling
/// + `@(cb)`; output drivers and explicit skews are honest-loud at elaborate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ClockingDecl {
    /// `None` for an anonymous `clocking @(clk); … endclocking` (rare).
    pub name: Option<Ident>,
    /// `true` for `default clocking …`.
    pub is_default: bool,
    /// The clocking event, e.g. `@(posedge clk)`.
    pub clock: Sensitivity,
    pub items: Vec<ClockingItem>,
    pub span: Span,
}

/// One `input/output [skew] sig [= expr];` member of a clocking block. `skew_raw`
/// holds any explicit skew text (`#1`, `#1step`, …) so elaborate can honest-loud it
/// (v1 = default skew only); `None` is the default skew.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ClockingItem {
    pub dir: ClockingDir,
    pub skew_raw: Option<String>,
    pub name: Ident,
    /// The bound expression for `input sig = dut.q;`; `None` ⇒ the signal `name`
    /// itself (`input q;` binds to the net `q` in scope).
    pub expr: Option<Expr>,
    pub span: Span,
}

/// Clocking-item direction (IEEE 1800 §14.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum ClockingDir {
    Input,
    Output,
    Inout,
}

/// A `let NAME [(formals)] = expr;` declaration (SVA-REST, IEEE 1800 §11.13). The
/// body is substituted (with positional formal→actual binding) at each use site by
/// elaborate — pure IR-0 (no sim-ir change). Mirrors [`SeqDecl`] but for a plain
/// expression rather than a sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct LetDecl {
    pub name: Ident,
    pub formals: Vec<Ident>,
    pub body: Expr,
    pub span: Span,
}

/// A functional-coverage model `covergroup NAME [@(event)]; (coverpoint EXPR;)*
/// endgroup` (N5). `clock` is the optional sampling event (slice F): each instance
/// auto-samples on it (`always @(clock) inst.sample();`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct CovergroupDecl {
    pub name: Ident,
    pub points: Vec<Coverpoint>,
    pub crosses: Vec<CrossSpec>,
    pub clock: Option<Sensitivity>,
    /// Covergroup-level `option.at_least = N` — the default `at_least` for every
    /// coverpoint that does not override it (slice D). `None` ⇒ 1.
    pub at_least: Option<Expr>,
    pub span: Span,
}

/// `[LABEL:] cross cp_a, cp_b [, …];` — a cross of named coverpoints (N5 slice C).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct CrossSpec {
    pub name: Option<Ident>,
    /// The crossed coverpoint names (labels, or the implicit single-ident name).
    pub points: Vec<Ident>,
    pub span: Span,
}

/// A `[LABEL:] coverpoint EXPR [iff (G)] [{ bin* }];` inside a covergroup.
/// `bins` EMPTY ⇒ auto-bins (the byte-identical legacy path). `iff` is reserved
/// for the guard slice (parsed now, elaborate loud-rejects until implemented).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Coverpoint {
    pub label: Option<Ident>,
    pub expr: Expr,
    /// Coverpoint-level `iff (G)` sampling guard (slice B).
    pub iff: Option<Expr>,
    /// Explicit bin list. EMPTY ⇒ auto-bins fallback (byte-identical).
    pub bins: Vec<BinSpec>,
    /// `option.at_least = N` — a bin is covered only after N hits (slice D). `None`
    /// ⇒ inherit the covergroup default (else 1).
    pub at_least: Option<Expr>,
    /// `option.weight = N` — this coverpoint's weight in the covergroup average
    /// (slice D). `None` ⇒ 1.
    pub weight: Option<Expr>,
    pub span: Span,
}

/// One explicit bin inside a coverpoint body: `KIND NAME[array] = RHS [iff (G)];`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct BinSpec {
    pub name: Ident,
    pub kind: BinKind,
    pub array: BinArray,
    /// Value set (range list). EMPTY iff `is_default` (a `default` catch-all bin).
    pub values: Vec<CoverRange>,
    pub is_default: bool,
    /// Per-bin `iff (G)` guard (slice B).
    pub iff: Option<Expr>,
    pub span: Span,
}

/// `bins` (regular, counts), `ignore_bins` (excluded), `illegal_bins` (runtime error).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum BinKind {
    Regular,
    Ignore,
    Illegal,
}

/// Bin array form: `b` (scalar), `b[]` (one bin per value), `b[N]` (fixed N bins).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum BinArray {
    Scalar,
    Unsized,
    Fixed(Expr),
}

/// One open_range_list entry: a single value (`lo==hi`) or inclusive `[lo:hi]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct CoverRange {
    pub lo: RangeEnd,
    pub hi: RangeEnd,
}

/// A range endpoint: a constant expression, or `$` (the coverpoint type extreme).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum RangeEnd {
    Val(Expr),
    TypeExtreme,
}

/// A covergroup instance declaration `CG_TYPE NAME = new;` (N5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct CoverInstance {
    pub cg_type: Ident,
    pub name: Ident,
    pub span: Span,
}

/// An interface modport: a named direction view over interface members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ModportDecl {
    pub name: Ident,
    /// `(dir, member)` pairs in source order; the direction is sticky across
    /// commas (`input a, b` ⇒ both inputs).
    pub ports: Vec<(PortDir, Ident)>,
    pub span: Span,
}

/// `typedef <kind> name;` (SV user-defined type, Phase-2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct TypedefDecl {
    pub name: Ident,
    pub kind: TypedefKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum TypedefKind {
    /// `typedef enum [base] { RED, GREEN=5, … } color_t;` — labels become module
    /// constants; the type is the (default int) base. `base` is the optional packed
    /// base range (`enum logic [1:0] {…}`).
    Enum {
        base: Option<Range>,
        labels: Vec<EnumLabel>,
    },
    /// `typedef logic [7:0] byte_t;` — a plain type alias. The parser resolves
    /// `byte_t x;` to this underlying net/var type directly; elaborate is a no-op.
    Alias {
        kind: NetVarKind,
        signed: bool,
        range: Option<Range>,
        packed: Vec<Range>,
    },
    /// `typedef struct packed { logic [7:0] a; logic [3:0] b; } pkt_t;` — a packed
    /// struct. The parser lays members MSB-first into one flat `logic` vector and
    /// desugars `s.a` to a constant part-select; elaborate is a no-op.
    Struct { members: Vec<StructMember> },
}

/// One enum label: `RED` or `GREEN = 5`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct EnumLabel {
    pub name: Ident,
    pub value: Option<Expr>,
}

/// One packed-struct member: `logic [7:0] a;`. v1 members are scalar/vector
/// net/var types with a constant-literal range (no nested struct / param width).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct StructMember {
    pub name: Ident,
    pub kind: NetVarKind,
    pub signed: bool,
    pub range: Option<Range>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct DefparamItem {
    pub assigns: Vec<(HierPath, Expr)>,
    pub span: Span,
}

// ──────────────────── PortDecl (non-ANSI body dir) ────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct PortDecl {
    pub dir: PortDir,
    pub net_or_var: Option<NetVarKind>, // e.g. `output reg`
    pub signed: bool,
    pub range: Option<Range>,
    pub names: Vec<Ident>, // `input [3:0] a, b;`
    pub span: Span,
}

// ──────────────────────────── ParamDecl ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ParamDecl {
    pub kind: ParamKind,
    pub signed: bool,
    pub ty: ParamType,
    pub range: Option<Range>,
    pub name: Ident,
    pub value: Expr, // RHS const-expr (required)
    pub span: Span,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum ParamKind {
    Parameter,
    Localparam,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum ParamType {
    Implicit,
    Integer,
    Real,
    Realtime,
    Time,
}

// ──────────────────────────── NetVarDecl ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct NetVarDecl {
    pub kind: NetVarKind,
    pub signed: bool,
    pub range: Option<Range>, // packed/vector [msb:lsb] (the FIRST/outer packed dim)
    pub packed: Vec<Range>,   // ADDITIONAL packed dims: `logic [3:0][7:0]` ⇒ packed=[[7:0]]
    /// IEEE §6.1.3 net declaration delay (`wire #3 w = a;` / `wire #(2,3) w = a;`).
    /// Applies to EVERY net-declaration-assignment in this decl, identical to a
    /// delay on the equivalent `assign #d net = expr;`. `None` = no delay (the
    /// common case → byte-identical). Parsed only for NET kinds (a `#` after a
    /// variable range stays a loud error). Var kinds always carry `None`.
    pub delay: Option<Delay>,
    pub names: Vec<DeclName>, // one decl, possibly many names
    /// B4: per-declaration lifetime OVERRIDE (`automatic int x;` / `static int y;`).
    /// `Some(true)` = automatic, `Some(false)` = static, `None` = follow the
    /// enclosing function/task default. Honored only on a frame function/task
    /// body_decl (IEEE §6.21; iverilog rejects the override outright).
    pub lifetime: Option<bool>,
    /// N7: when `kind == ClassHandle`, the declared class name (`Packet p;` ⇒
    /// `Some("Packet")`). `None` for every non-class declaration.
    pub class_type: Option<Ident>,
    /// ⓑ-breadth (§8.25): a parameterized class handle's specialization arguments
    /// (`C #(16) h;` ⇒ `[16]`). Empty = the default specialization (or a
    /// non-parameterized class). Elaborate folds these to the monomorphized class.
    pub class_args: Vec<Expr>,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct DeclName {
    pub name: Ident,
    pub unpacked: Vec<Dim>, // memory dims: reg [7:0] mem [0:255] / mem [256]
    pub init: Option<Expr>, // net/var initializer `= expr`
    pub span: Span,
}
/// An unpacked array dimension. `[msb:lsb]` (V2005) OR `[size]` (SV size-form).
/// (verdict M3: the AST must represent both; the parser accepts `[size]` too.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum Dim {
    Range(Range), // [hi:lo]
    Size(Expr),   // [N]
    /// `[]` — dynamic array (v5 ⑥).
    Dyn,
    /// `[$]` queue; `[$:N]` bounded queue (bound PARSED, elaborate loud-rejects
    /// it — bounded queues are outside the MVP).
    Queue(Option<Expr>),
    /// `[integer]` / `[time]` / `[string]` — associative array key type
    /// (integral keys live in the engine's signed-i64 domain; string keys in
    /// the byte-string domain, v6).
    Assoc(AssocKey),
}

/// Assoc-array key type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum AssocKey {
    /// Every integral key spelling (`integer`/`int`/`longint`/`shortint`/
    /// `byte`) — the engine key domain is uniformly signed i64 (the v5-⑥
    /// design pin: keys are never truncated to the declared width).
    Integer,
    Time, // `time` — same i64 engine domain; the variant is cosmetic today
    /// `[string]` (v6) — byte-string keys (contextual keyword: `string` stays
    /// a plain identifier everywhere else).
    Str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum NetVarKind {
    // nets
    Wire,
    Tri,
    Wand,
    Triand,
    Wor,
    Trior,
    Tri0,
    Tri1,
    Supply0,
    Supply1,
    Trireg,
    Uwire,
    // variables
    Reg,
    Logic,
    Integer,
    Real,
    Realtime,
    Time,
    /// `event e;` (v5 batch B — elaborate desugars it to a 64-bit counter reg).
    Event,
    /// SV `string` variable (v7 P2-C — heap-handle storage, dyn precedent).
    String,
    // SV 2-state integer types (SVPART): X-free — they default-initialise to 0,
    // never X. `bit` carries an optional packed range (default 1-bit, unsigned);
    // `byte`/`shortint`/`int`/`longint` are signed atom types of fixed width
    // (8/16/32/64). Stored like a `reg` in the IR; only the init differs.
    Bit,
    Byte,
    Shortint,
    Int,
    Longint,
    /// SV class-typed handle variable (N7). The declared class NAME rides the
    /// sibling `NetVarDecl.class_type` field (this enum stays `Copy`). Lowered by
    /// elaborate to a 32-bit `NetKind::Integer` net holding an object-id (0 =
    /// `null`) + a `class_handle_nets` sidecar entry; the object itself lives in
    /// the engine `class_heap`. Pure IR-0 (no sim-ir change).
    ClassHandle,
    /// SV `virtual INTERFACE vif;` handle (ⓑ-breadth, §25.9). The interface type
    /// name rides `NetVarDecl.class_type`. Elaborate resolves it as a STATIC ALIAS:
    /// when `vif` is bound once to a concrete interface instance (`vif = bif;`),
    /// every `vif.member` access is symbol-aliased to that instance's flattened net.
    /// Dynamic / conditional re-binding is a v1 loud-reject (never silent).
    VirtualIface,
}

impl NetVarKind {
    /// True for the wire-family NET kinds (as opposed to variable kinds). Only a
    /// net kind may carry an IEEE §6.1.3 net-declaration delay (`wire #3 w = a`).
    pub fn is_net(self) -> bool {
        use NetVarKind::*;
        matches!(
            self,
            Wire | Tri
                | Wand
                | Triand
                | Wor
                | Trior
                | Tri0
                | Tri1
                | Supply0
                | Supply1
                | Trireg
                | Uwire
        )
    }
}

/// `[msb:lsb]`. Bounds are exprs (usually const), NOT pre-evaluated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Range {
    pub msb: Expr,
    pub lsb: Expr,
    pub span: Span,
}

// ─────────────────────── ContinuousAssign ───────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ContinuousAssign {
    pub delay: Option<Delay>,
    pub assigns: Vec<(Lvalue, Expr)>, // assign a=b, c=d;
    pub span: Span,
}
/// `#d` | `#(d)` | `#(rise,fall)` | `#(rise,fall,turnoff)`. Each value is a
/// `MinTypMax`-or-plain expr (verdict M2 — `#(1:2:3)` is legal). The parser stores
/// each delay value via `Expr` (mintypmax surfaces as `ExprKind::MinTypMax`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Delay {
    pub values: Vec<Expr>,
    pub span: Span,
}

// ───────────────── ProceduralBlock + Sensitivity [A] ─────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ProceduralBlock {
    pub kind: ProcKind,
    pub sensitivity: Option<Sensitivity>, // only general `always @(…)`
    pub body: Box<Stmt>,                  // usually Block; stub-parsed in PR1
    pub span: Span,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum ProcKind {
    Initial,
    Always,
    AlwaysFf,
    AlwaysComb,
    AlwaysLatch,
    /// `final` (P2-E): a zero-time one-shot at end of simulation (IEEE
    /// §9.2.3 — timing controls inside are illegal, loud at elaborate).
    Final,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum Sensitivity {
    Star,                 // @(*) / @* (both map here; M5 note)
    List(Vec<EventExpr>), // @(posedge clk or negedge rst or a)
}
/// Intra-assignment EVENT control after `=`/`<=` (IEEE 1800 §9.4.5): `@(event)` or
/// `repeat(n) @(event)`. The RHS (and any LHS index) is captured at the statement,
/// then the value is written after the event occurs (`repeat` times when present).
/// On `Stmt::Blocking` the process BLOCKS until the write; on `Stmt::NonBlocking`
/// (slice N1) it does NOT block — a detached helper performs the NBA write later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct IntraEvent {
    /// `repeat(n)` count; `None` is a plain `@(event)` (one occurrence).
    pub repeat: Option<Expr>,
    pub ctrl: Sensitivity,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct EventExpr {
    pub edge: Edge,
    pub expr: Expr,
    pub span: Span,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum Edge {
    Posedge,
    Negedge,
    NoEdge,
}

// ──────────────────────────── Statement [A] ────────────────────────────
// FULL variant set frozen NOW (verdict M7): SchemaHash will eventually hash this
// enum, so adding `Fork`/`Assign`/`Deassign`/`Force`/`Release` later would flip the
// schema. The grammar §2.7 superset is adopted; parsing of all of these is deferred
// to PR2 (PR1 only constructs `Block`/`Error`/`Null` via the recovering stub).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum Stmt {
    Blocking {
        lhs: Lvalue,
        delay: Option<Delay>,
        /// Intra-assignment event control `= @(ev) rhs` / `= repeat(n) @(ev) rhs`
        /// (mutually exclusive with `delay`). `None` for a plain `=`.
        event: Option<IntraEvent>,
        rhs: Expr,
        span: Span,
    }, // =
    NonBlocking {
        lhs: Lvalue,
        delay: Option<Delay>,
        /// Intra-assignment event control `<= @(ev) rhs` / `<= repeat(n) @(ev) rhs`
        /// (mutually exclusive with `delay`). `None` for a plain `<=`. (Slice N1.)
        event: Option<IntraEvent>,
        rhs: Expr,
        span: Span,
    }, // <=
    If {
        cond: Expr,
        then_s: Box<Stmt>,
        else_s: Option<Box<Stmt>>,
        span: Span,
    },
    /// SV `return [expr];` (N7 — used pervasively by class methods). In a value
    /// function/method, `expr` assigns the return var; in a void task/method,
    /// `expr` is absent. Lowers to a return-var assign + a jump to the body exit.
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Case {
        kind: CaseKind,
        scrutinee: Expr,
        items: Vec<CaseItem>,
        span: Span,
    },
    For {
        init: Box<Stmt>,
        cond: Expr,
        step: Box<Stmt>,
        body: Box<Stmt>,
        span: Span,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    Repeat {
        count: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    Forever {
        body: Box<Stmt>,
        span: Span,
    },
    Block {
        label: Option<Ident>,
        decls: Vec<NetVarDecl>,
        stmts: Vec<Stmt>,
        span: Span,
    },
    Fork {
        label: Option<Ident>,
        decls: Vec<NetVarDecl>,
        stmts: Vec<Stmt>,
        join: JoinKind,
        span: Span,
    },
    SysTaskCall {
        name: Ident,
        args: Vec<Expr>,
        span: Span,
    }, // $display(...); name retains `$`
    UserTaskCall {
        name: HierPath,
        args: Vec<Expr>,
        span: Span,
    },
    /// `obj.randomize() with { … };` as a void statement (result discarded). The
    /// inline constraints ride a per-call `randomize_with` sidecar (B-CRV final).
    RandomizeWith {
        name: HierPath,
        args: Vec<Expr>,
        constraints: Vec<Expr>,
        span: Span,
    },
    DelayCtrl {
        delay: Delay,
        body: Option<Box<Stmt>>,
        span: Span,
    }, // #d stmt
    EventCtrl {
        ctrl: Sensitivity,
        body: Option<Box<Stmt>>,
        span: Span,
    }, // @(…) stmt
    EventTrigger {
        name: HierPath,
        span: Span,
    }, // -> ev ;
    Wait {
        cond: Expr,
        body: Option<Box<Stmt>>,
        span: Span,
    },
    Disable {
        target: HierPath,
        span: Span,
    },
    /// `wait fork;` (IEEE §9.6.1) — block until all child processes forked by
    /// the current process complete. v8 AST flip; parser/elaborate wired in the
    /// wait-fork feature slice (the bump leaves it shape-only).
    WaitFork {
        span: Span,
    },
    /// Concurrent assertion subset (SVA, Phase-3): `assert property(@(clk) a
    /// |-> b)` / `|=>`. The antecedent is a `Sequence` (slice S4 added bounded
    /// `##n` cycle-delay + `[*n]` consecutive repetition; a flat boolean is
    /// `Sequence::Boolean`); the consequent stays a flat boolean `Expr`. The
    /// parser/elaborate desugar (shift-register pipeline + clocked `$error`)
    /// lands in the SVA feature slices.
    ConcurrentAssert {
        clock: Sensitivity,
        /// Optional `disable iff (expr)` reset (slice S12, IEEE 1800 §16.12.7):
        /// when the (clock-sampled) condition is true the attempt is aborted (no
        /// pass/fail) and in-flight pipeline/pending state is cleared.
        disable_iff: Option<Expr>,
        antecedent: Sequence,
        implication_kind: ImplicationKind,
        /// The consequent. Slice S14 generalized this from a flat boolean `Expr`
        /// to a `Sequence` (`a |-> b ##1 c`); a plain boolean consequent is
        /// `Sequence::Boolean(_)` and keeps the byte-identical lowering.
        consequent: Sequence,
        /// Optional CONSEQUENT clocking event (slice A3, multi-clock): the leading
        /// `@(c2)` of `@(c1) ante |=> @(c2) cons`. `None` = single-clock (the byte-
        /// identical lowering). `Some(c2)` selects the two-process handoff synthesis
        /// (a c1-clocked sampler + a c2-clocked consumer); only valid with `|=>`.
        consequent_clock: Option<Sensitivity>,
        /// Optional action block (slice S11, IEEE 1800 §16.14.1):
        /// `assert property(...) [pass] [else fail]`. `pass` runs on a
        /// non-vacuous success, `fail` (the `else` statement) replaces the
        /// default `$error` on a violation. A bare `assert property(...);` leaves
        /// both `None` (default $error, no pass action).
        pass: Option<Box<Stmt>>,
        fail: Option<Box<Stmt>>,
        /// Property-expression tree (slice N2d): the `and`/`or`/recursive-property
        /// layer above a flat implication. `None` (the common case) means the
        /// property is a single flat implication carried by the
        /// `antecedent/implication_kind/consequent` fields above — the byte-
        /// identical lowering. `Some(_)` means the body uses property-level
        /// `and`/`or` (or a legal tail-`|=>` recursion); the flat fields then hold
        /// placeholders and elaborate's `synth_prop_expr` reduces the tree to a
        /// per-clock boolean violation check. Pure IR-0 (no sim-ir change).
        prop_expr: Option<PropExpr>,
        /// Sequence/property LOCAL VARIABLE declarations (slice N2c, IEEE §16.10):
        /// the `int x;` at the body start. Empty (the common case) keeps the
        /// byte-identical lowering — the data-tracking machinery only activates when
        /// a declaration (and a `MatchItem` capture) is present. Out-of-band of the
        /// frozen sim-ir (elaborate lowers it to extra regs + NBA shifts, pure IR-0).
        local_vars: Vec<SvaLocalDecl>,
        span: Span,
    },
    // procedural-continuous family (§2.7):
    Assign {
        lhs: Lvalue,
        rhs: Expr,
        span: Span,
    }, // procedural `assign lv = e;`
    Deassign {
        lhs: Lvalue,
        span: Span,
    },
    Force {
        lhs: Lvalue,
        rhs: Expr,
        span: Span,
    },
    Release {
        lhs: Lvalue,
        span: Span,
    },
    /// Deferred immediate assertion (IEEE 1800-2017 §16.4): `assert #0 (c)` is
    /// the Observed-deferred form, `assert final (c)` the Reactive-deferred form.
    /// Unlike a plain immediate `assert` (which stays `Stmt::If`), the condition
    /// is sampled WHEN REACHED but the pass/fail action MATURES in a later
    /// scheduling region with flush-on-re-reach (§4.4). The desugar mirrors `If`:
    /// `then_s` is the pass action (`Null` when none), `else_s` the fail action
    /// (the synthesized `$error("Assertion failed")` when there is no `else`).
    DeferredAssert {
        region: AssertDefer,
        cond: Expr,
        then_s: Box<Stmt>,
        else_s: Box<Stmt>,
        span: Span,
    },
    /// Cover property (SVA-REST, IEEE 1800 §16.13): `cover property(@(clk) seq);`.
    /// In simulation this COUNTS sequence matches and reports the hit count at
    /// end-of-sim (a coverage statement, not a pass/fail assertion). The desugar is a
    /// synthesized clocked counter + a `final` `$display` of the hit count — pure
    /// IR-0. A bare boolean `seq` is `Sequence::Boolean`.
    CoverProperty {
        clock: Sensitivity,
        disable_iff: Option<Expr>,
        seq: Sequence,
        span: Span,
    },
    Null(Span), // bare ;
    /// Recovery placeholder for an unparseable / not-yet-implemented statement.
    Error(Span),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum JoinKind {
    Join,
    JoinAny,
    JoinNone,
}
/// Maturation region of a deferred immediate assertion (IEEE 1800 §16.4 / §4.4).
/// `assert #0` → Observed, `assert final` → Reactive. The plain immediate
/// `assert` carries no `AssertDefer` (it stays `Stmt::If`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum AssertDefer {
    /// `assert #0` — matures in the Observed region.
    Observed,
    /// `assert final` — matures in the Reactive region.
    Reactive,
}
/// SVA implication operator (Phase-3 subset). `|->` checks the consequent in the
/// SAME clock tick as the antecedent match (overlapping); `|=>` checks it on the
/// NEXT tick (non-overlapping).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum ImplicationKind {
    /// `|->` overlapping — consequent evaluated in the same tick.
    Overlap,
    /// `|=>` non-overlapping — consequent evaluated on the next clock tick.
    NonOverlap,
}
/// SVA sequence (Phase-3 subset, slice S4). A sequence describes a multi-clock
/// match pattern in a concurrent assertion's antecedent. Slice S4 supports only
/// bounded compile-time-constant forms (`min == max`); ranges (`##[m:n]`,
/// `[*m:n]`), unbounded (`[*0:$]`), goto/nonconsecutive (`[->n]`/`[=n]`),
/// `throughout`/`within` and multi-clock are deferred (loud parse errors). The
/// elaborate desugar lowers a `Sequence` to a synthesized shift-register pipeline
/// of 1-bit pending registers inside the clocked checker — pure IR-0, no sim-ir
/// change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum Sequence {
    /// A boolean leaf term. A flat property antecedent (`a |-> b`) is
    /// `Boolean(a)`.
    Boolean(Expr),
    /// `lhs ##min rhs` cycle-delay concatenation. Slice S4 emits only
    /// `min == max` (a constant delay; `max` is `Some(min)`); `##0` is
    /// same-cycle fusion, `##1` next cycle. The range form keeps `max` for
    /// forward compatibility but the parser rejects `min != max` (deferred).
    Delay {
        min: u32,
        max: Option<u32>,
        lhs: Box<Sequence>,
        rhs: Box<Sequence>,
    },
    /// Repetition. `Consec` (`[*n]`/`[*m:n]`, slices S4/S5) is consecutive;
    /// `Goto` (`[->n]`, slice S8) ends ON the n-th (gap-allowed) occurrence;
    /// `Nonconsec` (`[=n]`, slice S8) allows the match to extend past the n-th.
    /// Goto/Nonconsec require a boolean `seq` operand and a single count
    /// (`min == max`); their ranges are deferred (loud).
    Repeat {
        seq: Box<Sequence>,
        min: u32,
        max: Option<u32>,
        kind: RepeatKind,
    },
    /// `cond throughout seq` (slice S7) — the boolean `cond` must hold at every
    /// clock of `seq`'s match window. Lowered by ANDing `cond` into the seed and
    /// every shift stage of the synthesized pipeline (bounded inner only).
    Throughout { cond: Box<Expr>, seq: Box<Sequence> },
    /// `seq1 within seq2` (slice S9) — `seq1` must match entirely inside a match
    /// of `seq2`. Lowered (bounded both) to `match(seq2) & OR over the seq2
    /// window of registered `match(seq1)`. Top-level antecedent only.
    Within {
        seq1: Box<Sequence>,
        seq2: Box<Sequence>,
    },
    /// A multi-clock re-clocking boundary (slice N2a): `@(clock) seq`. Emitted by
    /// the parser at a `##`-boundary clocking event inside a sequence (e.g.
    /// `a ##1 @(c2) b`), where `clock` re-establishes the sampling clock for `seq`
    /// from that boundary onward (IEEE 1800 §16.13/§16.16 clock flow). Lowered by a
    /// dedicated cross-clock handoff synthesis (`synth_crossclock`); a `Clocked`
    /// node reaching the single-clock sequence pipeline is a routing bug → loud.
    Clocked {
        clock: Sensitivity,
        seq: Box<Sequence>,
    },
    /// A NAMED property/sequence instance: `assert property(NAME)` (a property
    /// instance) or a future parameterized reference. The parser emits this ONLY
    /// at the property-instance position (a bare `NAME` inside a sequence body
    /// still parses as `Boolean(Ident)` and is resolved against the sequence
    /// table at elaborate). `args` is reserved for the formal-arguments follow-on
    /// (the current subset rejects a non-empty list loud); carrying the field now
    /// means that slice adds NO further `.vu` AST-hash re-pin. Elaborate inlines
    /// the named declaration's body, so this is pure IR-0 (no sim-ir change).
    Instance {
        name: Ident,
        args: Vec<Expr>,
        span: Span,
    },
    /// A sequence MATCH-ITEM with local-variable assignments (slice N2c, IEEE
    /// 1800-2017 §16.10): `(b, x = e {, y = f})` — a boolean term `b` that, when it
    /// matches, CAPTURES one or more local variables. The capture is a
    /// data-tracking idiom (`(req, d=data) ##1 grant |-> (rdata == d)`): the value
    /// is read at a LATER term/consequent within the same match attempt. Elaborate
    /// lowers a FIXED-DELAY single-capture carrier to a parallel DATA shift register
    /// shifted in lockstep with the liveness pipeline (the shift register has at
    /// most one attempt per stage → no collision); ranged delays (which let two
    /// attempts CONVERGE on one stage, a data collision) are loud-rejected.
    MatchItem {
        seq: Box<Sequence>,
        /// `(name, expr)` captures — the assigned local variable(s) and the value
        /// expression captured when `seq` matches. v1 supports a single capture.
        assigns: Vec<(Ident, Expr)>,
    },
}

/// A sequence/property LOCAL VARIABLE declaration (slice N2c, IEEE 1800-2017
/// §16.10): a typed `int x;` / `bit [7:0] y;` at the body start of a
/// `property`/`assert property`. The declared width/sign governs the synthesized
/// DATA-tracking shift register storage and the read. `init` is the optional `= e`
/// (carried for completeness; the v1 subset loud-rejects a non-`None` initializer
/// since it would need per-attempt re-seeding state).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct SvaLocalDecl {
    pub name: Ident,
    /// Storage width in bits, resolved at parse time from the type keyword
    /// (`int`/`integer` = 32, `byte` = 8, `shortint` = 16, `longint` = 64,
    /// `bit`/`logic`/`reg` = 1 unless a packed range widens it).
    pub width: u32,
    /// Signedness: `int`/`byte`/`shortint`/`longint`/`integer` are signed;
    /// `bit`/`logic`/`reg` are unsigned (matching the 2-state/4-state defaults).
    pub signed: bool,
    /// `true` when the declared type is NOT a synthesizable fixed-width integral
    /// var (`real`/`realtime`/`string`/`event`/class/a net kind). Such a type has
    /// no fixed-width data-tracking shift register in this subset; the parser sets
    /// this flag (the `width`/`signed` fields are a 1-bit placeholder) and elaborate
    /// loud-rejects the capture (`synth_local_var_assert`) — NEVER a silent 1-bit
    /// truncation that would flip the assertion verdict.
    pub unsupported_type: bool,
    pub init: Option<Expr>,
    pub span: Span,
}

/// SVA property expression (slice N2d) — the property-level `and`/`or` /
/// recursive-property layer above a flat implication. A property whose body is a
/// single flat implication (`a |-> b`) does NOT use this enum (it stays in the
/// flat `antecedent/implication_kind/consequent` fields with `prop_expr: None`,
/// byte-identical). A body using `and`/`or`, a parenthesized property, or a
/// (legal tail-`|=>`) self-reference parses to a `PropExpr` tree, and elaborate's
/// `synth_prop_expr` reduces it to a per-clock boolean violation check (pure IR-0).
///
/// A bare property/recursion reference is NOT a dedicated variant — it parses as
/// `Seq(Sequence::Boolean(Ident))` and elaborate resolves the identifier against
/// the property table / the recursive self-name. Precedence (loosest→tightest):
/// `or` < `and` < implication (`|->`/`|=>`, whose LHS is a sequence per IEEE 1800
/// §16.12) < primary (a sequence leaf or a parenthesized property).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum PropExpr {
    /// A bare sequence used as a property — it holds iff the sequence matches this
    /// clock. The subset restricts the operand to a boolean leaf (a multi-term
    /// sequence operand of `and`/`or` is loud-rejected at elaborate).
    Seq(Sequence),
    /// An implication leaf `ante |-> cons` / `ante |=> cons`. Per IEEE the
    /// antecedent is a sequence; the consequent is itself a property expression, so
    /// `1'b1 |=> p` can reference a property name `p` (the recursion site).
    Impl {
        ante: Sequence,
        kind: ImplicationKind,
        cons: Box<PropExpr>,
    },
    /// `lhs and rhs` — both must hold (violation = either side violates).
    And(Box<PropExpr>, Box<PropExpr>),
    /// `lhs or rhs` — at least one must hold (violation = both sides violate).
    Or(Box<PropExpr>, Box<PropExpr>),
    /// `not p` (SVA-REST) — holds iff `p` does NOT hold (violation = `p` holds).
    /// The operand must reduce to a same-clock (skew-0) verdict (a boolean / `|->`
    /// leaf); `not` of a multi-clock-skew (`|=>`) operand is loud-rejected.
    Not(Box<PropExpr>),
    /// `lhs until rhs` / `lhs s_until rhs` (SVA-REST, IEEE 1800 §16.12.9). The
    /// (skew-0) safety obligation is "`lhs` holds at every clock up to and
    /// including the clock before `rhs` first holds" → per-clock violation =
    /// `!held(lhs) && !held(rhs)`. `strong` (`s_until`) ADDS the liveness
    /// obligation that `rhs` eventually holds (checked by an end-of-sim `final`
    /// block); weak `until` (the default) does not. Both operands must be skew-0.
    Until {
        lhs: Box<PropExpr>,
        rhs: Box<PropExpr>,
        strong: bool,
    },
    /// `s_eventually p` (SVA-REST, IEEE 1800 §16.12.10) — the strong liveness
    /// obligation that `p` (a skew-0 verdict) eventually holds. There is no
    /// per-clock safety violation; an unfulfilled obligation is reported by an
    /// end-of-sim `final` block. Unbounded only (`strong` is always true; the
    /// bounded `s_eventually [m:n]` / weak unbounded `eventually` forms are loud).
    Eventually { strong: bool, prop: Box<PropExpr> },
    /// `always p` (SVA-REST, IEEE 1800 §16.12.10) — `p` holds at every clock from
    /// the attempt start. At the TOP level of a per-clock-re-attempted assertion
    /// this is exactly `p` (every clock re-checks `p`), so elaborate reduces a
    /// top-level `Always(p)` to `p`'s per-clock violation. A NESTED `always` (e.g.
    /// `a |-> always b`, which needs a "once armed, forever" latch) is loud-rejected.
    Always(Box<PropExpr>),
}

/// A named SVA sequence declaration: `sequence NAME [(formals)]; <seq>; endsequence`
/// (IEEE 1800 §16.8). Stored at elaborate and INLINED at each use site (reusing the
/// existing sequence desugar). `formals` is reserved for the parameterized follow-on
/// (the current subset rejects a non-empty list loud).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct SeqDecl {
    pub name: Ident,
    pub formals: Vec<Ident>,
    pub body: Sequence,
    pub span: Span,
}

/// A named SVA property declaration: `property NAME [(formals)]; <property_spec>;
/// endproperty` (IEEE 1800 §16.12). The body mirrors a `Stmt::ConcurrentAssert`'s
/// property spec (clock + optional `disable iff` + `antecedent impl consequent`); a
/// `assert property(NAME)` instance splices these fields at elaborate. `formals`
/// reserved (see [`SeqDecl`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct PropDecl {
    pub name: Ident,
    pub formals: Vec<Ident>,
    pub clock: Sensitivity,
    pub disable_iff: Option<Expr>,
    pub antecedent: Sequence,
    pub implication_kind: ImplicationKind,
    pub consequent: Sequence,
    /// Optional consequent clocking event (slice A3, multi-clock) — see
    /// [`Stmt::ConcurrentAssert`]'s `consequent_clock`. `None` = single-clock.
    pub consequent_clock: Option<Sensitivity>,
    /// Property-expression tree (slice N2d) — see [`Stmt::ConcurrentAssert`]'s
    /// `prop_expr`. `None` = a flat implication (the byte-identical path).
    pub prop_expr: Option<PropExpr>,
    /// Sequence/property LOCAL VARIABLE declarations (slice N2c) — see
    /// [`Stmt::ConcurrentAssert`]'s `local_vars`. Empty keeps the byte-identical path.
    pub local_vars: Vec<SvaLocalDecl>,
    pub span: Span,
}
/// SVA repetition operator (slices S4/S5/S8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum RepeatKind {
    /// `[*n]` / `[*m:n]` — consecutive.
    Consec,
    /// `[->n]` — goto: the n-th (gap-allowed) occurrence, ending on it.
    Goto,
    /// `[=n]` — nonconsecutive: n occurrences, match may extend past the n-th.
    Nonconsec,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum CaseKind {
    Case,
    Casez,
    Casex,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum CaseItem {
    Match {
        labels: Vec<Expr>,
        body: Box<Stmt>,
        span: Span,
    },
    Default {
        body: Box<Stmt>,
        span: Span,
    },
}

// ──────────────────────────── Expr [P] ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

/// Boxed payload of `ExprKind::RandomizeWith` (`obj.randomize() with { … }`). Held
/// behind a `Box` so the variant stays pointer-sized (see the variant's note).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct RandomizeWithExpr {
    pub name: HierPath,
    pub args: Vec<Expr>,
    pub constraints: Vec<Expr>,
}

/// Boxed payload of `ExprKind::ArrayMethodWith` — an array reduction/locator
/// method call carrying a `with (expr)` iterator clause (IEEE 1800 §7.12).
/// BOXED for the same reason as `RandomizeWithExpr` (keep `ExprKind` pointer-
/// sized so the expr-recursion depth cap is unaffected). `recv` is the source
/// handle path, `method` the method name. `iter_var` is the optional named
/// iterator declared in the method's parens (`find(x) with (x>2)`); `None` means
/// the default `item`. `with_expr` is the per-element expression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ArrayMethodWithExpr {
    pub recv: HierPath,
    pub method: Ident,
    pub iter_var: Option<Ident>,
    pub with_expr: Expr,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum ExprKind {
    // literals: raw lexeme + kind; value parse deferred to elaborate.
    IntLit {
        kind: IntLitKind,
        raw: String,
    },
    RealLit {
        kind: RealLitKind,
        raw: String,
    },
    StrLit {
        raw: String,
    }, // includes quotes; unescape deferred
    /// `pkg::name` package-scoped value reference (v7 P2-D).
    PkgScoped {
        pkg: Ident,
        name: Ident,
    },
    // names
    Ident(HierPath), // a, a.b.c
    // operators (precedence table §4 → Pratt binding powers)
    Unary {
        op: UnOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Ternary {
        cond: Box<Expr>,
        then_e: Box<Expr>,
        else_e: Box<Expr>,
    },
    // postfix / structural
    BitSelect {
        base: Box<Expr>,
        index: Box<Expr>,
    }, // a[i]
    PartSelect {
        base: Box<Expr>,
        msb: Box<Expr>,
        lsb: Box<Expr>,
    }, // a[m:l]
    IndexedPart {
        base: Box<Expr>,
        offset: Box<Expr>,
        width: Box<Expr>,
        dir: PartDir,
    }, // a[b+:w] (M6: `offset`)
    /// `{a,b,c}` concatenation.
    Concat {
        parts: Vec<Expr>,
    },
    /// `{n{x,y}}` replication. NOTE (verdict M5): `value` holds the repeated
    /// element list DIRECTLY (the concat parts), NOT a wrapper `Concat` node, so
    /// `{n{x}}` ⇒ `Replicate{count:n, value:[x]}`. No `ExprKind::Concat` wrapper.
    Replicate {
        count: Box<Expr>,
        value: Vec<Expr>,
    },
    Call {
        name: HierPath,
        args: Vec<Expr>,
    }, // func(args)
    /// `obj.randomize() with { c1; c2; … }` (IEEE 1800 §18.7) — a per-call
    /// constrained-random draw. The payload is BOXED so this rare variant does not
    /// enlarge `ExprKind`: the expression parser recurses through `ExprKind`-sized
    /// frames and a fat inline variant shrinks the recursion-depth-cap margin (it
    /// tipped `depth_guard.rs`'s deep-nesting test into a stack overflow). B-CRV
    /// final slice. `constraints` are extra constraint expressions ADDED to the
    /// object's class constraints for this one call.
    RandomizeWith(Box<RandomizeWithExpr>),
    /// `q.sum() with (item*2)` / `q.find() with (item>2)` (IEEE §7.12) — an array
    /// reduction or locator method with a `with` iterator clause. BOXED (see the
    /// payload note). Reductions yield a scalar; locators yield a queue (only
    /// valid as the direct rhs of a blocking assign, intercepted at statement
    /// level like the queue pops).
    ArrayMethodWith(Box<ArrayMethodWithExpr>),
    /// `$time`, `$signed(x)`. NOTE (verdict M6): `name.name` retains the leading
    /// `$` (the lexer's `SystemTask` lexeme includes it), parallel to EscapedIdent.
    SysCall {
        name: Ident,
        args: Vec<Expr>,
    },
    Paren {
        inner: Box<Expr>,
    }, // (e) — span fidelity
    MinTypMax {
        min: Box<Expr>,
        typ: Box<Expr>,
        max: Box<Expr>,
    }, // a:b:c
    /// `new[n]` / `new[n](src)` — dynamic-array allocation (v5 ⑥). Parsed
    /// CONTEXTUALLY (the ident `new` immediately followed by `[`); elaborate
    /// falls back to an ordinary array read when a net named `new` is in
    /// scope, preserving V2005 programs that use `new` as an identifier.
    New {
        size: Box<Expr>,
        src: Option<Box<Expr>>,
    },
    /// `new` / `new(args)` — class object allocation (N7). Distinct from the
    /// dynamic-array `New{size,src}` (which is `new[n]`). The class is inferred
    /// from the assignment LHS handle's declared type at elaborate.
    ClassNew {
        args: Vec<Expr>,
    },
    /// `null` — the null class handle literal (N7). Lowers to a 0-valued
    /// 32-bit handle; dereferencing it yields X + a warn-once diagnostic.
    Null,
    /// Bare `$` — the queue last-index (`q[$]`, `q[$-1]`). Only meaningful
    /// inside a queue element select; elaborate substitutes `size()-1` there
    /// and loud-rejects it anywhere else.
    Dollar,
    /// `value dist { item, … }` — weighted-distribution constraint (IEEE §18.5.4),
    /// valid only inside a constraint. The `randomize()` solver samples `value`
    /// from the weighted distribution (a SAMPLER, not a boolean predicate).
    Dist {
        value: Box<Expr>,
        items: Vec<DistItem>,
    },
    /// SV static cast `casting_type'(expr)` (IEEE 1800 §6.24). `target` carries
    /// the casting type (a primitive type keyword, a `signed`/`unsigned` signing,
    /// a size expression `N'(e)`, or a typedef/class name); `expr` is the operand.
    /// BOXED to keep this rare variant from enlarging `ExprKind` and shrinking the
    /// expr-parser recursion-depth margin (same rationale as `RandomizeWith`).
    Cast {
        target: CastTarget,
        expr: Box<Expr>,
    },
    /// SV §10.9 POSITIONAL assignment pattern `'{e0, e1, …, eN}`. v1 supports only
    /// the positional form bound to a 1-D unpacked array (`int a[N] = '{…};` /
    /// `a = '{…};`), where each element is assigned to the corresponding array
    /// slot in order. A named / `default:` / replicated pattern, a packed-array or
    /// struct target, or any non-array context is loud-rejected at elaborate.
    AssignPattern(Vec<Expr>),
    /// Recovery placeholder so the Pratt loop can keep folding past an error.
    Error,
}

/// The casting type in `casting_type'(expr)` (IEEE 1800 §6.24).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum CastTarget {
    /// A primitive integral/real type keyword: `int'(e)`, `byte'(e)`, `real'(e)`…
    Prim(CastPrim),
    /// A signing cast: `signed'(e)` (`signed:true`) / `unsigned'(e)`. Width is
    /// PRESERVED; only the operand's sign interpretation flips.
    Signing { signed: bool },
    /// A size cast `N'(e)` / `(W+1)'(e)`: the result is `N` bits wide. Signedness
    /// is INHERITED from the operand (sign-extend iff operand signed). The width
    /// expression is constant-folded at elaborate.
    Size(Box<Expr>),
    /// A typedef/class-name cast `name'(e)`. Numeric typedefs are resolved at
    /// elaborate; class casts `Base'(d)` are loud-rejected (no oracle yet).
    Named(HierPath),
}

/// Primitive casting-type keywords for `CastTarget::Prim`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum CastPrim {
    Int,      // 32-bit signed 2-state
    Integer,  // 32-bit signed 4-state
    Byte,     // 8-bit signed 2-state
    Shortint, // 16-bit signed 2-state
    Longint,  // 64-bit signed 2-state
    Bit,      // 1-bit unsigned 2-state
    Logic,    // 1-bit unsigned 4-state
    Reg,      // 1-bit unsigned 4-state (alias of logic)
    Time,     // 64-bit unsigned 4-state
    Real,     // double-precision real (also `realtime'`)
}

/// One weighted item of a `dist { … }`: a single value (`hi == None`) or a
/// `[lo:hi]` range, with a weight applied PER-VALUE (`:=`, `per_range == false`)
/// or SPREAD across the range (`:/`, `per_range == true`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct DistItem {
    pub lo: Box<Expr>,
    pub hi: Option<Box<Expr>>,
    pub weight: Box<Expr>,
    pub per_range: bool,
}

/// Unary / reduction operators — names mirror sim-ir `UnOp` 1:1 (verified
/// sim-ir/src/lib.rs:97) for a clean lowering name-map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum UnOp {
    Plus,
    Minus,
    LogNot,
    BitNot,
    RedAnd,
    RedNand,
    RedOr,
    RedNor,
    RedXor,
    RedXnor,
}
/// Binary operators — names mirror sim-ir `BinOp` 1:1 (verified sim-ir/src/lib.rs:112):
/// `AShl`/`AShr` = `<<<`/`>>>`;  `Le`/`Ge`/`Ne`; `CaseEq`/`CaseNe` = `===`/`!==`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Shl,
    Shr,
    AShl,
    AShr,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    CaseEq,
    CaseNe,
    /// `==?` wildcard equality (§11.4.6): x/z bits of the RHS pattern are
    /// don't-care; an LHS x/z in a compared position propagates x (unlike
    /// `CasexEq`, which wildcards EITHER side). Lowered by elaborate's
    /// `lower_wildcard_eq` (const-RHS mask & compare), never via `map_binop`.
    WildEq,
    /// `!=?` wildcard inequality — the §11.4.6 negation of `WildEq`.
    WildNe,
    BitAnd,
    BitXor,
    BitXnor,
    BitOr,
    LogAnd,
    LogOr,
}
/// Indexed part-select direction. Lowers to sim-ir `SelKind::PartIdxUp/Down`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum PartDir {
    PlusColon,
    MinusColon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum IntLitKind {
    Decimal,
    Sized,
    UnsizedBased,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum RealLitKind {
    Fixed,
    Exp,
}

// ──────────────────────────── Lvalue [P] ────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum Lvalue {
    Ident(HierPath),
    BitSelect {
        base: Box<Lvalue>,
        index: Box<Expr>,
        span: Span,
    },
    PartSelect {
        base: Box<Lvalue>,
        msb: Box<Expr>,
        lsb: Box<Expr>,
        span: Span,
    },
    IndexedPart {
        base: Box<Lvalue>,
        offset: Box<Expr>,
        width: Box<Expr>,
        dir: PartDir,
        span: Span,
    },
    Concat {
        parts: Vec<Lvalue>,
        span: Span,
    }, // {cout, sum} = …
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ModuleInstance {
    pub module_name: Ident,
    pub param_overrides: Vec<ParamConn>, // #(.W(8)) | #(8)
    pub instances: Vec<InstanceItem>,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct InstanceItem {
    pub name: Ident,
    pub unpacked: Vec<Dim>,
    pub conns: PortConnList,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum ParamConn {
    Named {
        name: Ident,
        value: Option<Expr>,
        span: Span,
    },
    Positional(Expr),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum PortConnList {
    /// `.p(e)` / `.p` connections. The trailing `bool` is the `.*` wildcard
    /// flag (IEEE §23.3.2.5): when set, every port the explicit list does not
    /// name is connected to a same-named signal in the instantiating scope.
    Named(Vec<PortConn>, bool),
    Positional(Vec<Option<Expr>>),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct PortConn {
    pub name: Ident,
    pub value: Option<Expr>,
    pub span: Span,
} // .a(x) / .a()

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct GenerateConstruct {
    pub items: Vec<GenItem>,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum GenItem {
    For {
        init: GenAssign,
        cond: Expr,
        step: GenAssign,
        label: Option<Ident>,
        body: Vec<GenItem>,
        span: Span,
    },
    If {
        cond: Expr,
        then_b: Vec<GenItem>,
        else_b: Vec<GenItem>,
        label: Option<Ident>,
        span: Span,
    },
    Case {
        scrutinee: Expr,
        items: Vec<GenCaseItem>,
        span: Span,
    },
    Block {
        label: Option<Ident>,
        items: Vec<GenItem>,
        span: Span,
    },
    Item(Box<ModuleItem>),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct GenAssign {
    pub lvalue: Ident,
    pub value: Expr,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum GenCaseItem {
    Match {
        labels: Vec<Expr>,
        body: Vec<GenItem>,
        span: Span,
    },
    Default {
        body: Vec<GenItem>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct FunctionDef {
    pub automatic: bool,
    pub signed: bool,
    pub range: Option<Range>,
    pub ret_type: ParamType,
    /// The return type is a 2-state integral (`int`/`byte`/`shortint`/`longint`/
    /// `bit`) — it can never hold X/Z (§6.11.3), so the return assignment coerces
    /// any unknown to 0. `ParamType` cannot carry this (`int` shares
    /// `ParamType::Integer` with 4-state `integer`; byte/shortint/longint reach
    /// elaborate as a bare range like `reg [N]`), so the parser records it here.
    pub ret_two_state: bool,
    pub name: Ident,
    pub ports: Vec<TfPort>,
    pub body_decls: Vec<NetVarDecl>,
    pub body: Box<Stmt>,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct TaskDef {
    pub automatic: bool,
    pub name: Ident,
    pub ports: Vec<TfPort>,
    pub body_decls: Vec<NetVarDecl>,
    pub body: Box<Stmt>,
    pub span: Span,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct TfPort {
    pub dir: PortDir,
    pub net_or_var: Option<NetVarKind>,
    pub signed: bool,
    pub range: Option<Range>,
    pub name: Ident,
    /// IEEE §13.5.3 default argument value (`function f(int a, int b = 10)`). Only
    /// an ANSI tf-port carries one; a call omitting trailing args fills them in.
    pub default: Option<Expr>,
    pub span: Span,
}

// ──────────────────────────── Tests ────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip a Box-recursive `Expr` tree through postcard (proves serde works
    /// on the Box-recursive AST — the primary goal of the dev-dep).
    #[test]
    fn postcard_roundtrip_expr() {
        let span = Span::new(0, 4);
        // Build:  -(a + b)
        let a = Expr {
            kind: ExprKind::Ident(HierPath {
                segments: vec![Ident {
                    name: "a".to_string(),
                    span,
                }],
                span,
            }),
            span,
        };
        let b = Expr {
            kind: ExprKind::Ident(HierPath {
                segments: vec![Ident {
                    name: "b".to_string(),
                    span,
                }],
                span,
            }),
            span,
        };
        let add = Expr {
            kind: ExprKind::Binary {
                op: BinOp::Add,
                lhs: Box::new(a),
                rhs: Box::new(b),
            },
            span,
        };
        let neg = Expr {
            kind: ExprKind::Unary {
                op: UnOp::Minus,
                operand: Box::new(add),
            },
            span,
        };

        let bytes = postcard::to_stdvec(&neg).expect("serialize");
        let decoded: Expr = postcard::from_bytes(&bytes).expect("deserialize");
        assert_eq!(neg, decoded);
    }
}
