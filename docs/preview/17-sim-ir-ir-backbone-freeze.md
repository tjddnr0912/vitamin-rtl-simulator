# 17 · sim-ir backbone freeze

`sim-ir` is the language-neutral simulation IR, and its root type `sim_ir::SimIr` is the golden
shape that gates `.velab` staleness. This document is the catalogue of that frozen backbone: which
types are in it, their exact shapes, the rules that keep those shapes stable, how the arena indices
resolve, what the front end lowers onto them, what deliberately rides outside them, and what a
re-freeze costs. The hash mechanism is [16-schema-hash-spec.md](16-schema-hash-spec.md); the
container that carries the hash is [14-staged-artifacts.md](14-staged-artifacts.md).

---

## 1. Freeze principles

| # | Principle | What it means in practice |
|---|---|---|
| P1 | The golden root is `schema_hash::<sim_ir::SimIr>()` | `SimIr` holds every arena by value, so its type-reachability closure covers the whole backbone. `Process` is a sub-pin only (§2) |
| P2 | Every inter-node edge is an arena index | `u32`, `u64`, `Option<u32>` or `Vec<u32>`. There is no `Box<Self>` or `Vec<Self>` self-recursion, so the type-reachability graph is a finite acyclic DAG and reloading needs zero pointer fixup |
| P3 | The shapes are byte-identical on every platform | No `HashMap`, no `HashSet`; `BTreeMap`, `BTreeSet` and `Vec` only. No `usize`, `isize`, `f32` or `f64`. Order-stable `Vec`. Every type monomorphic — no type parameter, lifetime or const generic. Zero serde attributes |
| P4 | The IR is span-free | No `Span` type and no `span` field exists anywhere in `sim-ir`. Source locations travel in an out-of-band table keyed by node index |
| P5 | Growth is deliberate, never incidental | Adding, removing or reordering a field or a variant flips the root hash. That is legitimate, and it is a re-freeze: `format_version` bump, every artifact regenerated, both goldens re-pinned (§10) |
| P6 | A frozen type stays at the crate root | The registry key embeds `module_path!()`, so relocating a type into a submodule flips the hash on its own ([16](16-schema-hash-spec.md) §4) |

Every frozen type carries the same derive list:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]   // + Copy where small
```

Only `SchemaHash` and the two serde derives are part of the contract. `Copy` is present on the
small value types and affects nothing on the wire.

---

## 2. Why the root is `SimIr` and not `Process`

`Process`'s own type-reachability closure stops at the process boundary: every edge out of it —
into expressions, statements, nets or constants — is a bare `u32` arena index, and a `u32` reaches
no type. A hash rooted at `Process` would therefore leave `Expr`, `Stmt`, `NetVar` and `ConstVal`
free to change outside the hash.

`SimIr` holds each arena as a `Vec<T>` by value, so its closure covers the whole backbone and,
through `Vec<Process>`, the runtime process cluster as well. It is the compatibility gate.

`Process` is kept as a **sub-pin**: a second, cheap golden that gives an immediate signal when the
runtime cluster moves. It is not a gate, and a bump that touches only the expression or statement
arenas leaves the sub-pin unchanged — which is the correct reading, not a missed regression.

---

## 3. The frozen catalogue

Thirty-seven types, all at the crate root of `crates/sim-ir/src/lib.rs`, all with empty attribute
slots.

| Type | Kind | Size |
|---|---|---|
| `FourState` | enum | 4 |
| `EdgeKind` | enum | 3 |
| `ProcFlags` | newtype | `u8` |
| `RegionTag` | enum | 4 |
| `WakeCond` | enum | 6 |
| `WakeKey` | struct | 3 |
| `Frame` | struct | 5 |
| `JoinState` | struct | 4 |
| `SuspendState` | struct | 6 |
| `UnOp` | enum | 10 |
| `BinOp` | enum | 26 |
| `SelKind` | enum | 4 |
| `SysFuncId` | enum | 82 |
| `Expr` | enum | 11 |
| `SysTaskId` | enum | 41 |
| `DisableKind` | enum | 2 |
| `Stmt` | enum | 6 |
| `Lvalue` | struct | 1 |
| `LvalChunk` | struct | 5 |
| `DelayRegion` | enum | 2 |
| `WaitCause` | enum | 5 |
| `Terminator` | enum | 7 |
| `SensKind` | enum | 5 |
| `EdgeTerm` | struct | 2 |
| `Sensitivity` | struct | 2 |
| `NetKind` | enum | 10 |
| `PortDir` | enum | 4 |
| `BitPacked` | struct | 2 |
| `NetVar` | struct | 8 |
| `ConstRepr` | enum | 3 |
| `ConstVal` | struct | 4 |
| `BasicBlock` | struct | 2 |
| `Process` | struct | 4 |
| `ContAssign` | struct | 3 |
| `Instance` | struct | 4 |
| `FuncDef` | struct | 4 |
| `SimIr` | struct | 9 |

The canonical string is one sentinel line plus one line per type, committed as
`crates/testdata/sim_ir_canonical.txt`.

**Outside the closure by design.** `sim_ir::COp` and `sim_ir::CBinOp`, the constraint-solver
postfix ops, derive `Serialize` and `Deserialize` but not `SchemaHash`. They ride the out-of-band
`class_constraints` and `randomize_with` sidecars, so their wire shape is gated by
`format_version` instead of by the root hash. That is a division of labour, not a coverage hole.
The `sim_ir` submodules `analysis`, `mw`, `names`, `realness` and `selfwidth` hold no serialized
type; non-serialized code may live in a submodule, a `SchemaHash` type may not (P6).

---

## 4. The shapes

### 4.1 Runtime process cluster

```rust
pub enum FourState { Zero, One, X, Z }

pub enum EdgeKind { Posedge, Negedge, AnyEdge }

pub struct ProcFlags(pub u8);

pub enum RegionTag { Active, Inactive, Nba, Monitor }

pub enum WakeCond {
    Edge       { net: u32, kind: sim_ir::EdgeKind },
    Level      { nets: Vec<u32> },
    WaitTrue   { expr: u32 },
    TimeAbs    { tick: u64 },
    NamedEvent { ev: u32 },
    Join       { join_ref: u32 },
}

pub struct WakeKey {
    pub cond:      sim_ir::WakeCond,
    pub region:    sim_ir::RegionTag,
    pub tie_break: u32,
}

pub struct Frame {
    pub return_pc:    u32,
    pub callee_entry: u32,
    pub locals_base:  u32,
    pub locals_len:   u32,
    pub is_automatic: bool,
}

pub struct JoinState {
    pub parent:   Option<u32>,
    pub children: Vec<u32>,
    pub detached: Vec<u32>,
    pub flags:    sim_ir::ProcFlags,
}

pub struct SuspendState {
    pub resume_pc:   u32,
    pub locals:      Vec<sim_ir::FourState>,
    pub join_state:  sim_ir::JoinState,
    pub wake_key:    sim_ir::WakeKey,
    pub call_stack:  Vec<sim_ir::Frame>,
    pub frame_arena: Vec<sim_ir::FourState>,
}
```

`ProcFlags` is a newtype rather than a bare `u8` so its structural shape is distinct: the postcard
bytes are identical, the registry entries are not.

`Frame` holds no `Box<Frame>` or `Vec<Frame>` — the call stack is owned by `SuspendState` — so the
cluster contributes no self-edge.

Status at HEAD: this whole cluster is **inert**. Elaborate emits one constant `SuspendState` per
process (`resume_pc` equal to the entry block, empty `locals`, `call_stack` and `frame_arena`, and
a `WakeKey` of `Level{nets:[]}` / `RegionTag::Active` / `tie_break: 0`) and no engine code reads or
writes any part of it. Live suspension state is engine-side. See §9.

### 4.2 Expressions

```rust
pub enum Expr {
    Const     { val: u32 },                                              // -> consts[val]
    Signal    { net: u32, word: Option<u32> },                           // net read; word = unpacked element index expr
    Select    { base: u32, offset: u32, width: u32, kind: sim_ir::SelKind },
    Concat    { parts: Vec<u32> },                                       // {a,b,...} MSB-first, order frozen
    Replicate { count: u32, value: u32 },                                // {N{x}}
    Unary     { op: sim_ir::UnOp,  operand: u32 },
    Binary    { op: sim_ir::BinOp, lhs: u32, rhs: u32 },
    Ternary   { cond: u32, then_e: u32, else_e: u32 },
    SysFunc   { which: sim_ir::SysFuncId, args: Vec<u32> },
    Call      { func: u32, args: Vec<u32> },                             // -> funcs[func]
    ArrayItem { index: bool, width: u32, signed: bool },                 // array-method `with` iterator
}

pub enum UnOp {
    Plus, Minus, LogNot, BitNot,                                         // + - ! ~
    RedAnd, RedNand, RedOr, RedNor, RedXor, RedXnor,                     // & ~& | ~| ^ ~^
}

pub enum BinOp {
    Add, Sub, Mul, Div, Mod, Pow,                                        // + - * / % **
    BitAnd, BitOr, BitXor, BitXnor,                                      // & | ^ ~^   (^~ normalizes to BitXnor)
    LogAnd, LogOr,                                                       // && ||
    Lt, Le, Gt, Ge,                                                      // < <= > >=
    Eq, Ne, CaseEq, CaseNe,                                              // == != === !==
    Shl, Shr, AShl, AShr,                                                // << >> <<< >>>
    CasezEq, CasexEq,                                                    // per-label casez / casex match
}

pub enum SelKind { Bit, PartConst, PartIdxUp, PartIdxDown }              // x[i] / x[m:l] / x[b+:w] / x[b-:w]
```

Reduction and bitwise operators are distinguished by arity — reduction lives in `UnOp`, bitwise in
`BinOp` — not by a token.

`CasezEq` treats a position as don't-care when either side is `z`; `CasexEq` when either side is
`x` or `z`. Both always produce a known `1'b0`/`1'b1`, like `CaseEq`.

`ArrayItem` is the per-element iterator inside an array method's `with` clause. `index = false`
reads the current element's value at the element type; `index = true` reads its zero-based
position, 32-bit signed. The width and signedness ride in the node so the static width table can
size it.

`SysFuncId` has 82 variants, in declaration order — the order is itself contract, because it fixes
the postcard discriminant and enters the hash:

```
Time, Realtime, Signed, Unsigned, Clog2, Rtoi, Itor, RealToBits, BitsToReal,
DynSize, QPopBack, QPopFront, AssocExists, AssocNum, AssocFirst, AssocNext, AssocLast, AssocPrev,
Random, Urandom, UrandomRange, CountOnes, OneHot, OneHot0, IsUnknown, Stime,
Fopen, Sformatf, TestPlusargs, ValuePlusargs,
StrLen, StrGetC, StrSubstr, StrToUpper, StrToLower, StrCmp,
Fgets, Fscanf, Sscanf, Fread, Feof, Fgetc, Ungetc,
DistUniform, DistNormal, DistExponential, DistPoisson, DistChiSquare, Cast,
ArrSum, ArrProduct, ArrAnd, ArrOr, ArrXor,
StrAtoi, StrAtohex, StrAtooct, StrAtobin, StrAtoreal,
Ln, Log10, Exp, Sqrt, Pow, Floor, Ceil, Sin, Cos, Tan, Asin, Acos, Atan, Atan2, Hypot,
Sinh, Cosh, Tanh, Asinh, Acosh, Atanh,
DistT, DistErlang
```

The 21 real-math identifiers compute through the vendored pure-Rust libm, built with
`default-features = false` so no hardware intrinsic is used and the results are byte-identical on
every target ([03-build-and-portability.md](03-build-and-portability.md)). Several of these are
side-effecting on a reference argument — the queue pops, `Random`'s seeded form, `Fopen`, `Fgets`,
`Fscanf`, `ValuePlusargs`, the `$dist_*` family, the function form of `Cast`, and the assoc
iterators — and are legal only as the direct right-hand side of a blocking assignment, where the
lowering intercepts them at statement level.

Structural invariant: `Expr::Call.func` is a `FuncId` into `funcs`. It is the frame-call node in
expression position. A function that the front end inlines produces no `Expr::Call` at all — its
body is folded into the caller.

`Replicate.count` must be constant-foldable at elaboration; it is stored as an expression index for
uniformity with `Select`'s width, and a non-constant count is an elaboration error.

### 4.3 Statements

A statement is straight-line work inside a basic block. It carries no control flow.

```rust
pub enum Stmt {
    BlockingAssign    { lhs: sim_ir::Lvalue, rhs: u32 },                                  // =
    NonblockingAssign { lhs: sim_ir::Lvalue, rhs: u32, delay: Option<u32> },              // <=  (NBA region)
    SysTask           { which: sim_ir::SysTaskId, fmt: Option<u32>, args: Vec<u32> },
    Disable           { scope_kind: sim_ir::DisableKind, target: u32 },
    Force             { lhs: sim_ir::Lvalue, rhs: u32 },                                  // force lhs = rhs
    Release           { lhs: sim_ir::Lvalue },                                            // release lhs
}

pub enum DisableKind { Fork, Scope }                                     // detached-only vs enclosing-scope teardown

pub struct Lvalue { pub chunks: Vec<sim_ir::LvalChunk> }                 // concat-LHS = many chunks; simple = one

pub struct LvalChunk {
    pub net:    u32,                                                     // -> nets[net]
    pub word:   Option<u32>,                                             // unpacked word index expr (None = scalar/whole)
    pub offset: Option<u32>,                                             // part-select base expr (None = offset 0)
    pub width:  Option<u32>,                                             // None = whole net; Some(w) = explicit width
    pub kind:   sim_ir::SelKind,
}
```

`SysTaskId` has 41 variants, in declaration order:

```
Display, Write, Monitor, Strobe, Finish, Stop,
DumpFile, DumpVars, DumpOn, DumpOff, DumpAll, DumpFlush, DumpLimit,
DynNew, DynDelete, QPushBack, QPushFront, AssocDeleteKey, QInsert, QDeleteIdx,
Fclose, Fdisplay, Fwrite, Sformat, ReadmemB, ReadmemH, StrPutC,
WritememB, WritememH, Cast, MonitorOn, MonitorOff, ClassRandomize,
ArrSort, ArrRsort, ArrReverse,
StrItoa, StrHextoa, StrOcttoa, StrBintoa, ArrLocator
```

Rules that attach to these shapes:

| # | Rule |
|---|---|
| S1 | `NonblockingAssign.delay` is an `ExprId` evaluated at execution time. Each activation carries its own captured value to `t + d`, so overlapping activations stay independent. `None` is a plain same-tick NBA. The blocking form `a = #d b` needs no field: it lowers to a temporary capture plus a `Delay` terminator |
| S2 | `SysTask.fmt` points at a `consts` entry whose `repr` is `ConstRepr::StrUtf8` — the format-control string. `args` are post-format expression indices, and a task with no format string uses `fmt = None`. Keeping the format string in its own field, pointing at a self-identifying constant, is what makes `$display` output byte-identical across platforms |
| S3 | `$monitor` and `$strobe` persist by index: the engine keeps the `(fmt, args)` index tuple, so their persistence costs no shape |
| S4 | A continuous assignment is not a `Stmt`. It lives in the top-level `cont_assigns` arena |
| S5 | `Force`/`Release` are live. Procedural `assign`/`deassign` also lower onto them, at a weaker rank carried by the out-of-band `assign_ranks` sidecar. Only a bit- or part-select force target is refused |

### 4.4 Control flow

```rust
pub enum Terminator {
    Goto   { target: u32 },
    Branch { cond: u32, then_bb: u32, else_bb: u32 },
    Delay  { amount: u32, region: sim_ir::DelayRegion, resume: u32 },
    Wait   { cond: sim_ir::WaitCause, resume: u32 },
    Fork   { children: Vec<u32>, join: u32, resume_bb: u32 },
    Call   { target: u32, ret_bb: u32 },
    Return,
}

pub enum DelayRegion { Active, Inactive }                                // #d > 0 / #0

pub enum WaitCause {
    Edge  { net: u32, kind: sim_ir::EdgeKind },                          // @(posedge/negedge/edge net)
    Level { nets: Vec<u32> },                                            // @(a or b ...) / @(*)
    Expr  { expr: u32 },                                                 // wait(expr)
    Named { ev: u32 },                                                   // @(named_event)
    Fork,                                                                // wait fork
}
```

| # | Rule |
|---|---|
| T1 | `Fork` carries no join-kind field. `join` names the child rendezvous block and `resume_bb` the parent's landing block; the join mode itself rides the out-of-band `fork_modes` sidecar, keyed by `(template ProcId, join block)`. A `Fork` with no matching sidecar entry is a clean fatal naming the missing trailer, never a guess |
| T2 | `Call` names the callee by `FuncId` in `target` and the caller's landing block in `ret_bb`. It carries no argument list: the caller emits `Stmt::BlockingAssign` into the callee's frame window before the terminator, and the return value is a caller-visible local by the same ABI |
| T3 | `Delay.amount` is an `ExprId` for the delay value in the process's own module time units, raw and unscaled. Only the amount is evaluated at run time; `region` is baked at elaboration (`#0` is `Inactive`, `#d` with `d > 0` is `Active`) so routing stays data-independent and no scheduling logic re-derives it |
| T4 | `WaitCause` is a restricted enum, not the full `WakeCond`. Time is structurally excluded from a `Wait` — a delay is a `Delay` — so `Wait { TimeAbs }` cannot be constructed |
| T5 | Exactly one terminator ends each basic block |

`Return` is fieldless. A process-level sensitivity list is not a `Wait`; it belongs to `Sensitivity`
(§4.5) and re-arms at the process entry.

### 4.5 Sensitivity, nets, values and constants

```rust
pub struct Sensitivity {
    pub kind:  sim_ir::SensKind,
    pub edges: Vec<sim_ir::EdgeTerm>,
}

pub struct EdgeTerm { pub net: u32, pub kind: sim_ir::EdgeKind }

pub enum SensKind { Initial, Comb, Latch, Edge, Level }

pub struct NetVar {
    pub kind:      sim_ir::NetKind,
    pub width:     u32,                                                  // total bit width (1 = scalar)
    pub msb:       u32,                                                  // [msb:lsb] upper bound, post-elaboration
    pub lsb:       u32,                                                  // lower bound (msb < lsb = descending [0:N])
    pub signed:    bool,
    pub array_len: u32,                                                  // flat unpacked element count (1 = not an array)
    pub dir:       sim_ir::PortDir,
    pub init:      sim_ir::BitPacked,                                    // time-0 4-state value
}

pub enum NetKind { Wire, Reg, Logic, Integer, Real, DynArray, Queue, Assoc, AssocStr, String }

pub enum PortDir { Input, Output, Inout, Internal }

pub struct BitPacked {
    pub val: Vec<u64>,                                                   // value plane:   ceil(width/64) words, word0 bit0 = LSB
    pub unk: Vec<u64>,                                                   // unknown plane: (v,u) = 00→0, 10→1, 01→X, 11→Z
}

pub struct ConstVal {
    pub width:  u32,
    pub signed: bool,
    pub repr:   sim_ir::ConstRepr,
    pub bits:   sim_ir::BitPacked,
}

pub enum ConstRepr { Numeric, StrUtf8, Real }
```

| # | Rule |
|---|---|
| N1 | `Sensitivity` is a struct, not an enum: a uniform `edges` list with an orthogonal `kind`. Level, comb and latch sensitivity use `EdgeKind::AnyEdge` entries; `Initial` has empty `edges` |
| N2 | `EdgeKind` is one registry entry shared by `EdgeTerm`, `WaitCause::Edge` and `WakeCond::Edge`. It is never redeclared |
| V1 | The 4-state encoding is two planes, two bits per bit position: `00` is 0, `10` is 1, `01` is X, `11` is Z, with word 0 bit 0 as the LSB. The packing order is part of the format |
| V2 | `NetVar.init` is an inline `BitPacked`, a self-contained snapshot with no back edge |
| V3 | `BitPacked` carries no width. The width belongs to the parent — `NetVar.width` or `ConstVal.width` |
| V4 | A range is `(msb, lsb)`; a descending declaration `[0:N]` is `msb < lsb`. `array_len` is a flat element count, and `init` holds `width * array_len` bits. A multidimensional **unpacked** array is flattened row-major at elaboration onto that single `array_len` and a single word expression, so the IR shape does not grow; the per-dimension geometry rides the out-of-band `net_dims` sidecar, which exists for per-element waveform naming |
| V5 | `Real` and `realtime` share `NetKind::Real`. A real value is stored as `f64::to_bits()` in `init.val[0]` with `width = 64`, `signed = true` and `unk = [0]`, so no `f32`/`f64` field enters the IR and the determinism rule of P3 holds with reals as full participants in the golden shape rather than in a side lane. `ConstRepr::Real` mirrors it for literals |
| V6 | `DynArray`, `Queue`, `Assoc`, `AssocStr` and `String` are HANDLE kinds. The storage lives in the engine heap, never in the flat packed store; `width` is the element width (zero for `String`) and `array_len` is 0 |
| V7 | `ConstRepr` distinguishes a format or string literal from a numeric operand structurally, which is what makes `$display` formatting unambiguous |
| V8 | `genvar`, `parameter` and `localparam` are folded into `consts` at elaboration. They are never `NetVar` entries |

### 4.6 Blocks, processes, top-level arenas and the root

```rust
pub struct BasicBlock {
    pub stmts: Vec<u32>,                                                 // -> stmts[..], executed in order
    pub term:  sim_ir::Terminator,                                       // exactly one
}

pub struct Process {
    pub sensitivity: sim_ir::Sensitivity,
    pub body:        Vec<sim_ir::BasicBlock>,                            // PRIVATE block arena
    pub entry:       u32,
    pub suspend:     sim_ir::SuspendState,
}

pub struct ContAssign {
    pub lhs:   sim_ir::Lvalue,
    pub rhs:   u32,
    pub delay: Option<u32>,
}

pub struct Instance {
    pub parent:    Option<u32>,
    pub module:    u32,
    pub first_net: u32,
    pub net_count: u32,
}

pub struct FuncDef {
    pub entry:      u32,
    pub n_params:   u32,
    pub locals_len: u32,
    pub is_task:    bool,
}

pub struct SimIr {
    pub instances:    Vec<sim_ir::Instance>,
    pub nets:         Vec<sim_ir::NetVar>,
    pub processes:    Vec<sim_ir::Process>,
    pub cont_assigns: Vec<sim_ir::ContAssign>,
    pub funcs:        Vec<sim_ir::FuncDef>,
    pub exprs:        Vec<sim_ir::Expr>,
    pub stmts:        Vec<sim_ir::Stmt>,
    pub blocks:       Vec<sim_ir::BasicBlock>,
    pub consts:       Vec<sim_ir::ConstVal>,
}
```

| # | Rule |
|---|---|
| R1 | The compatibility gate is `schema_hash::<sim_ir::SimIr>()`. `schema_hash::<sim_ir::Process>()` is a sub-pin (§2) |
| R2 | **A process body is a private block arena.** `Process.body` holds its own blocks and `Process.entry` and every `Terminator` block target inside that body index it. The top-level `SimIr.blocks` arena holds function and task body CFGs only, rebased into one global space; `FuncDef.entry` and every terminator target inside a function body index that one. The two index spaces are distinct |
| R3 | Waveform-dump intent is a `Stmt::SysTask`, never a root field |
| R4 | `Instance` is a flat hierarchy record — a parent link, a module name reference and the half-open net slice `[first_net, first_net + net_count)`. It carries no behaviour |
| R5 | `FuncDef` is the minimum call ABI. The window into which arguments are bound, the return slot and the suspendability classification ride the out-of-band `func_table` sidecar, index-aligned to `funcs` |

---

## 5. Arena layout

`SimIr` is a flat arena of arenas. Every edge is an index into exactly one `Vec`, resolved at load
as `vec[idx as usize]`. That cast is transient in the resolver and never a stored field, so the
no-`usize` rule holds.

| `SimIr` field | Element | Index | Referenced by |
|---|---|---|---|
| `exprs` | `Expr` | ExprId = `u32` | every `rhs`, `cond`, `operand`, `args`, `Select.*`, `Replicate.*`, `ContAssign.rhs`, `Delay.amount`, `WaitCause::Expr.expr`, `LvalChunk.word`/`offset`/`width` |
| `stmts` | `Stmt` | StmtId = `u32` | `BasicBlock.stmts` |
| `blocks` | `BasicBlock` | BlockId = `u32` | `FuncDef.entry` and every terminator target inside a function or task body |
| `nets` | `NetVar` | NetId = `u32` | `Signal.net`, `LvalChunk.net`, `EdgeTerm.net`, `WaitCause::Edge`/`Level`, `Instance` slice |
| `consts` | `ConstVal` | ConstId = `u32` | `Expr::Const.val`, `Stmt::SysTask.fmt` |
| `processes` | `Process` | ProcId = `u32` | top level; `Terminator::Fork.children` |
| `cont_assigns` | `ContAssign` | CaId = `u32` | top level |
| `instances` | `Instance` | InstId = `u32` | `Instance.parent` |
| `funcs` | `FuncDef` | FuncId = `u32` | `Expr::Call.func`, `Terminator::Call.target` |

One append-only `Vec` per node kind means reloading a `.velab` needs no pointer fixup at all.

Block indices inside a `Process` are the exception: they index `Process.body`, that process's own
arena, not `SimIr.blocks` (R2).

---

## 6. Lowering — control constructs onto terminators

| Source construct | Lowering |
|---|---|
| `if (c) T else E` | the block ends with `Branch { cond: c, then_bb: T, else_bb: E }` |
| `case` / `casez` / `casex` | a `Branch` cascade. The scrutinee is lowered once — captured into a temporary net where it must not be re-evaluated — and each label becomes a compare against it: `CaseEq` for `case`, `CasezEq`/`CasexEq` for the wildcard forms. `default` is the final `else_bb`. Wildcard positions are realized in the compare, never stored in `nets` |
| `for (i; c; s) B` | init block `Goto` → test block `Branch { c, body, exit }`; body ends `Goto` → step block `Goto` → test |
| `while (c) B` | test block `Branch { c, body, exit }`; body ends `Goto` → test |
| `repeat (n) B` | a small constant count unrolls straight, with no runtime counter. A runtime or large count desugars to a signed down-counter loop: the count is evaluated once, and a zero or negative count runs the body zero times |
| `forever B` | body ends `Goto` back to the body entry; no exit edge. Verilog requires an in-body `@` or `#`, so a same-tick infinite loop is not reachable |
| `begin … end` | serial blocks joined by `Goto`. A named-block label becomes out-of-band metadata for `%m` and for diagnostics |
| `disable <enclosing named block>` | `Stmt::Disable { scope_kind: Scope, .. }` followed by a `Goto` to the block's exit, which is what actually carries the control flow |
| `disable fork` | `Stmt::Disable { scope_kind: Fork, .. }` — straight-line, no control flow |
| `#d` | `Delay { amount: d, region: (d == 0 ? Inactive : Active), resume: next_bb }` |
| `@(...)` / `wait(c)` | `Wait { cond: WaitCause::…, resume: next_bb }` |
| `wait fork` | `Wait { cond: WaitCause::Fork, resume: next_bb }` |
| `fork … join` / `join_any` / `join_none` | `Fork { children, join, resume_bb }`; the join mode rides the `fork_modes` sidecar (T1) |
| task call (statement) | `Call { target, ret_bb }`, with arguments pre-bound into the callee's frame window by `BlockingAssign` |
| function call (expression) | `Expr::Call { func, args }` for a frame function; an inlined function produces no node at all |
| end of a body or block | `Return` |

**No new terminator names.** `foreach`, `break` and `continue` desugar in the parser or onto the
existing `Goto`/`Branch`/`Disable` shapes, and `unique`/`priority` qualifiers become an additional
violation check rather than a new control shape.

---

## 7. The rules that keep the shapes stable

| Rule | Enforced by |
|---|---|
| No `usize`, `isize`, `f32` or `f64` in a schema type | The derive rejects the four type paths with a `compile_error!`; `crates/sim-ir/tests/no_float_usize.rs` additionally asserts that none of the four tokens appears anywhere in the canonical string, which catches an aliased reintroduction |
| No `HashMap` or `HashSet` | The derive rejects the literal type-path head |
| Every type monomorphic | The derive rejects any type, lifetime or const-generic parameter |
| Zero serde attributes on a frozen type | `crates/sim-ir/tests/no_serde_attrs.rs` root-walks the closure and asserts every attribute slot is exactly `#[]`, so a new frozen type is covered without editing the test |
| Every cross-type field spelled `sim_ir::Foo` | `crates/sim-ir/tests/body_refs.rs`, two tests: every `sim_ir::X` token in a body must be a registry key, and no bare user-type identifier may appear in type position. `sim-ir` declares `extern crate self as sim_ir;` to make that spelling legal. A bare, `crate::`-qualified or `use`-aliased spelling would emit a body reference that is not a key — a dangling reference the hash would not notice |
| Frozen types stay at the crate root | The registry key embeds `module_path!()`; moving a type flips the root hash by itself ([16](16-schema-hash-spec.md) §4) |
| Exact shapes | `crates/sim-ir/tests/frozen_shapes.rs` pins nine verbatim shapes and asserts every name is crate-root fully qualified; `crates/sim-ir/tests/m3_shapes.rs` pins `BinOp`, `Expr`, `Terminator` and `SimIr` verbatim and checks the closure builds without a collision |
| The whole canonical string | `crates/sim-ir/tests/schema_hash.rs::canonical_string_golden` against `crates/testdata/sim_ir_canonical.txt` |
| serde wire form | `crates/sim-ir/tests/reflection.rs::serde_reflection_ron_golden` against `crates/testdata/sim_ir_registry.ron`, traced from the real derives ([16](16-schema-hash-spec.md) §10) |

Two details that hold P2 and P3 without any test:

- No frozen enum uses an explicit discriminant, and no frozen type uses a fixed-size array `[T; N]`.
  Reorder safety comes from variant names plus source position, not from a discriminant token.
- postcard varint-encodes `Vec` and `String` lengths, so a sequence length is width-independent and
  only the element type enters the hash.

---

## 8. What rides outside the shape

The backbone stays small because everything the engine needs but the *shape* does not have to carry
travels out of band — either in a `SimOpts` field synthesized from run options, or in a `.velab`
trailer segment. None of it can flip the root hash; a change to any of it is gated by
`format_version` instead ([14-staged-artifacts.md](14-staged-artifacts.md) §6, §7).

| Construct | Carrier |
|---|---|
| fork join mode | `fork_modes`, keyed by `(template ProcId, join block)` |
| wired-AND / wired-OR net resolution | `wired_and_nets`, `wired_or_nets` |
| continuous-assign rise / fall / turn-off delays | `ca_delays` |
| class layout, field initializers, vtable, call sites, field widths | `class_layouts`, `class_field_inits`, `class_vtable`, `class_calls`, `class_field_widths` |
| constrained random: bounds, predicates, distributions, cyclic fields, inline `with` | `class_rand`, `class_constraints`, `class_dist`, `class_randc`, `randomize_with` |
| deferred assertions | `defer_marks`, `defer_acts` |
| clocking blocks | `clocking_inputs`, `clocking_commit`, `clocking_outputs` |
| frame-call metadata: window, return slot, suspendability | `func_table`, index-aligned to `funcs` |
| runtime diagnostic locations | `stmt_locs` — the IR is span-free (P4), so this table is the only way a runtime report can print `file:line:col [in instance]`. It is resolved once, at elaboration, so staged and one-shot output are identical by construction |
| `%m` scope names | `proc_scopes`, `proc_inst_scopes`, `stmt_scopes`, `expr_scopes`, `func_names` |
| waveform names, per-dimension geometry, declared ranges | `net_names`, `net_dims`, `net_decl_ranges` |
| severity kind, output radix, queue bounds, assign ranks, two-state nets, real / string dynamic elements, declaration-initializer order, `final` blocks | the corresponding trailer segments |
| SVA concurrent assertions, sequences and properties | synthesized clocked checker processes built from the existing shapes; no IR node of their own |
| named events | a 64-bit counter register; `-> e` lowers to `e = e + 1`, so every waiter sees a guaranteed change and a double trigger in one slot is not lost |
| packed multidimensional parameters, packed struct and union members | rewritten in the parser to the flat part-select those bits occupy |
| interfaces | flattened at elaboration into ordinary nets and instances |

---

## 9. Shape the IR carries that nothing reads

Stated plainly, because a frozen field that no consumer reads is still part of the on-disk contract
and still costs a re-freeze to remove.

| Shape | State at HEAD |
|---|---|
| `Process.suspend` and the whole `SuspendState` closure — `WakeKey`, `WakeCond`, `RegionTag`, `Frame`, `JoinState`, `ProcFlags`, `FourState` | Elaborate emits one constant value per process; no engine code reads or writes any part of it. Live suspension state is engine-side, in the scheduler's own activity, waiter and wheel structures |
| `RegionTag::Nba`, `RegionTag::Monitor` | Constructed nowhere in the workspace. The four IEEE regions are implemented, but the engine's own Active/Inactive tags are in-memory wheel entries and the NBA and Postponed regions have no `RegionTag` representation. The only scheduling-region field the engine reads from the IR is `Terminator::Delay.region`, which is `DelayRegion` — a different enum |
| `WaitCause::Named`, `WakeCond::NamedEvent` | Reserved and never emitted: a named event lowers to a counter register (§8), so a `Wait` on `Named` cannot arise from source. The engine's classifiers treat it as the never-waking case |
| `SimIr.instances` | Populated by elaborate and read by no consumer. The hierarchy dumps `--hier-tree` and `--inst-paths` are served from an out-of-band elaborate table |
| `NetVar.dir` | Written by elaborate; no reader. Port direction is provenance for diagnostics, not a runtime input |
| `Stmt::Disable.target` | Written; no reader. The engine matches on `scope_kind` alone, and `DisableKind::Scope` executes as a no-op because the elaborator has already emitted the `Goto` that carries the control flow |

---

## 10. Re-freeze protocol

A change to any frozen shape is legitimate; it is not free, and it is never accidental.

1. Make the shape change. Appending a variant **last** to a frozen enum still flips the root hash —
   that is the opposite of the trailer rule in [14](14-staged-artifacts.md) §7, where postcard's
   positional discriminants let an appended variant leave old values decoding unchanged.
2. Bump `CURRENT_FORMAT_VERSION` in `crates/vita-artifact/src/header.rs` and record what moved in
   its doc comment, which is the canonical version-by-version record.
3. Regenerate the goldens through the sanctioned switch, never by hand:

```bash
REGEN_GOLDEN=1 cargo test -p sim-ir --test schema_hash -- --nocapture   # canonical string + both hashes
REGEN_GOLDEN=1 cargo test -p sim-ir --test reflection                   # the serde-reflection RON golden
```

4. Paste the printed root hash into `EXPECTED_SIMIR_HASH`. Update `EXPECTED_PROCESS_HASH` only if
   the runtime cluster itself moved; an unchanged sub-pin alongside a changed root is the expected
   reading for an expression- or statement-arena change, and is a sanity signal rather than a
   failure.
5. Re-pin whatever else asserts the number: the `run.json` `format_version` field in
   `crates/cli/tests/obs.rs`, and the cli-local wire-shape fixtures if a trailer moved as well.
6. Run the full gate: `cargo nextest run --workspace --locked`.

**What the bump costs.** Every `.vu` and `.velab` on disk is stale. Each one refuses to load with
`E-ART-FORMAT-MISMATCH` at the header gate, before a single body byte is deserialized, and exits 2
so CI rebuilds upstream instead of debugging RTL. There is no migration path and no silent reuse:
the refusal is the feature.

The container version and the schema hash are independent numbers and must not be read as one. A
trailer-only or tail-only change bumps `format_version` and leaves the golden hash untouched; a
frozen shape change moves both.

---

## 11. Coverage

Every Phase-1 construct group maps onto a frozen type.

| Construct group | Expressed by |
|---|---|
| design units: module, port, parameter, localparam, generate, genvar | `Instance` plus `NetVar.dir` for ports; parameters, localparams and genvars fold to `ConstVal` at elaboration |
| data types: wire, reg, logic, integer, vectors, packed arrays | `NetVar.kind` plus `width`/`msb`/`lsb` for vectors and `array_len` for a flat unpacked array; a packed array is simply a wider `BitPacked` |
| real and realtime | `NetKind::Real` and `ConstRepr::Real`, stored as raw IEEE-754 bits (V5) |
| dynamic arrays, queues, associative arrays, strings | the handle kinds of `NetKind` (V6), with heap storage engine-side |
| procedural blocks: initial, always, always_ff, always_comb, always_latch, final | `Process` plus `Sensitivity { kind, edges }`; `final` blocks are identified by an out-of-band set |
| statements: `=`, `<=`, if, case, casez, casex, for, while, repeat, forever, begin/end | `Stmt::{BlockingAssign, NonblockingAssign}` plus the `Branch`/`Goto` cascades of §6 |
| force / release / assign / deassign | `Stmt::{Force, Release}` (S5) |
| timing: `#delay`, `@(event)`, `wait`, `wait fork` | `Terminator::Delay` and `Terminator::Wait { cond: WaitCause }` |
| fork / join family | `Terminator::Fork` plus the join-mode sidecar (T1) |
| functions and tasks | `FuncDef`, `Terminator::Call` for statement position, `Expr::Call` for expression position; an inlined function folds into the caller |
| continuous assignment, with optional delay | `ContAssign { lhs, rhs, delay }` |
| system tasks and functions | `Stmt::SysTask { which, fmt, args }` and `Expr::SysFunc { which, args }`, over the 41 and 82 identifier sets of §4 |
| expressions | `Expr`'s eleven variants plus `UnOp`, `BinOp`, `Ternary`, `Concat`, `Replicate`, `Select` with `SelKind`, `Signal.word` and per-net signedness |

Concatenated assignment targets become multiple `Lvalue.chunks`; wildcard positions in a `casez`
or `casex` label live in the compare rather than in `nets`; multi-edge sensitivity becomes several
`Sensitivity.edges` entries or an in-body `WaitCause`; comb and latch sensitivity is `SensKind`
plus `EdgeKind::AnyEdge`; a string literal is a `ConstVal` with `repr: StrUtf8`.

---

## 12. What the frozen shape cannot express

| Construct | State at HEAD |
|---|---|
| drive strengths | Refused. A strength specification does not parse, so it surfaces as a parse error rather than an approximation |
| `trireg`, `supply0`, `supply1`, `tri0`, `tri1`, `triand`, `trior` | Parsed, then refused at elaboration with `E-ELAB-UNSUPPORTED` (`VITA-E3009`). `tri` and `uwire` collapse to `NetKind::Wire` with no strength or pull behaviour; only `wand` and `wor` get real wired resolution, through a sidecar (§8) |
| a negative declared packed low bound | The bits are correct but the frozen bounds are not expressive enough: `NetVar.msb`/`lsb` are `u32`, so such a net is stored normalized to `[w-1:0]` and the declared bounds ride the `net_decl_ranges` sidecar for waveform naming |
| per-dimension geometry of a multidimensional unpacked array | Not in the shape by design: the array is flattened row-major onto one `array_len` (V4) and the dimension list rides `net_dims` |
| source locations | Excluded by rule (P4). They ride `stmt_locs` |

### Accepted residuals

- `Select` and `LvalChunk` are not self-contained: the direction and declared width of a select
  resolve by joining against `nets[net].msb`/`lsb`. That is inherent to an arena IR and is accepted.
- `SensKind` carries five provenance variants where the runtime distinguishes fewer. The extra
  discrimination is kept for diagnostics.
- `Expr::Const` indirects into `consts` even for a small literal. A uniform pool that deduplicates
  is worth the indirection.
- A `#[serde(with = "…")]` module's internals are a Layer-1 blind spot
  ([16](16-schema-hash-spec.md) §1.2). Exposure is nil here: no frozen type carries a serde
  attribute of any kind, and a test asserts it.

---

## Related documents

- [16-schema-hash-spec.md](16-schema-hash-spec.md) — the hash that gates these shapes
- [14-staged-artifacts.md](14-staged-artifacts.md) — the container, the trailers and the staleness gate
- [06-simulation-engine.md](06-simulation-engine.md) — the process model these shapes serve
- [01-goals-and-scope.md](01-goals-and-scope.md) — the supported construct set
- [08-timescale-and-timing.md](08-timescale-and-timing.md) — how `Delay.amount` converts to ticks
- [09-testing-and-verification.md](09-testing-and-verification.md) — the golden gates listed in §7
