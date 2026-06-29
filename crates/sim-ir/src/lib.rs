//! sim-ir — language-neutral simulation IR.
//!
//! PR1-B defines ONLY the frozen `SuspendState` runtime-state closure
//! (06 process model · shapes FROZEN 2026-06-02). The unspecified types
//! `Process`/`BasicBlock`/`Stmt`/`Expr`/`Sensitivity`/`Terminator` are deferred
//! to M3 — they require the net/expr arena freeze before the *root* hash can lock.
//! `FourState` and `EdgeKind` are scalar leaf enums newly frozen here (the only
//! members of the unspecified set the SuspendState closure transitively touches).
extern crate self as sim_ir;

use serde::{Deserialize, Serialize};
use vita_artifact_derive::SchemaHash;

/// Scalar 4-state logic value (IEEE 1364 §6). NEWLY FROZEN in PR1-B.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum FourState {
    Zero,
    One,
    X,
    Z,
}

/// Edge kind for an edge-sensitive wake condition. NEWLY FROZEN in PR1-B.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum EdgeKind {
    Posedge,
    Negedge,
    AnyEdge,
}

/// [SD1] vvp 1-bit flag bitset; newtype so the schema shape is distinct from a bare u8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ProcFlags(pub u8);

/// [SD4] IEEE 1364 4 regions; 17-region split is an intentional Phase-2 flip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum RegionTag {
    Active,
    Inactive,
    Nba,
    Monitor,
}

/// [SD5] closed 6-variant set of process-suspend conditions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum WakeCond {
    Edge { net: u32, kind: sim_ir::EdgeKind },
    Level { nets: Vec<u32> },
    WaitTrue { expr: u32 },
    TimeAbs { tick: u64 },
    NamedEvent { ev: u32 },
    Join { join_ref: u32 },
}

/// [SD4] region stored explicitly (never re-derived → keeps logic out of the hash).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct WakeKey {
    pub cond: sim_ir::WakeCond,
    pub region: sim_ir::RegionTag,
    pub tie_break: u32,
}

/// [SD2] integer-indexed call frame (not a native call stack).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Frame {
    pub return_pc: u32,
    pub callee_entry: u32,
    pub locals_base: u32,
    pub locals_len: u32,
    pub is_automatic: bool,
}

/// [SD1] vvp two-set fork/join port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct JoinState {
    pub parent: Option<u32>,
    pub children: Vec<u32>,
    pub detached: Vec<u32>,
    pub flags: sim_ir::ProcFlags,
}

/// [resume/reserved] RULE D2 atomic-freeze unit (16 §1) — PR1-B golden root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct SuspendState {
    pub resume_pc: u32,
    pub locals: Vec<sim_ir::FourState>,
    pub join_state: sim_ir::JoinState,
    pub wake_key: sim_ir::WakeKey,
    pub call_stack: Vec<sim_ir::Frame>,
    pub frame_arena: Vec<sim_ir::FourState>,
}

// ── M3 backbone ──────────────────────────────────────────────────────────────

/// Unary operator (§1).
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

/// Binary operator (§1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    BitAnd,
    BitOr,
    BitXor,
    BitXnor,
    LogAnd,
    LogOr,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    CaseEq,
    CaseNe,
    Shl,
    Shr,
    AShl,
    AShr,
    /// `casez` per-label match (v7): a bit is don't-care iff EITHER side is
    /// `z` there; remaining positions compare 4-state exact (`===`, so x
    /// matches only x). Result is always known 1'b0/1'b1, like `CaseEq`.
    CasezEq,
    /// `casex` per-label match (v7): don't-care iff EITHER side is x OR z;
    /// remaining (both-known) positions compare by value. Known 0/1 result.
    CasexEq,
}

/// Bit/part-select kind (§1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum SelKind {
    Bit,
    PartConst,
    PartIdxUp,
    PartIdxDown,
}

/// System-function id (§1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum SysFuncId {
    Time,
    Realtime,
    Signed,
    Unsigned,
    Clog2,
    Rtoi,       // $rtoi  — real → int, TRUNCATE toward zero
    Itor,       // $itor  — int  → real, exact convert
    RealToBits, // $realtobits — real → 64-bit vector (raw IEEE bits)
    BitsToReal, // $bitstoreal — 64-bit vector → real (raw IEEE bits)
    /// dyn/queue/assoc `.size()`/`.num()` länge (v5; args = [handle Signal]).
    DynSize,
    /// queue `.pop_back()` (v5; side-effecting — excluded from the P9 VM allow-list).
    QPopBack,
    /// queue `.pop_front()` (v5; side-effecting — see `QPopBack`).
    QPopFront,
    /// assoc `.exists(key)` (v5; args = [handle, key]).
    AssocExists,
    /// assoc `.num()` (v5; args = [handle]).
    AssocNum,
    /// assoc `.first(k)` (v6; args = [handle, key-VAR Signal]). Side-effecting
    /// (writes the ref key argument) — legal only as the DIRECT rhs of a
    /// blocking assign, intercepted statement-level like the queue pops.
    /// On dyn/queue handles the engine serves the DENSE 0..size-1 order
    /// (internal `foreach` desugar target; user surface stays assoc-only).
    AssocFirst,
    /// assoc `.next(k)` (v6) — strictly-greater successor; see `AssocFirst`.
    AssocNext,
    /// assoc `.last(k)` (v6) — greatest key; see `AssocFirst`.
    AssocLast,
    /// assoc `.prev(k)` (v6) — strictly-less predecessor; see `AssocFirst`.
    AssocPrev,
    /// `$random[(seed)]` (v7) — IEEE 1364 Annex N algorithm, signed 32-bit.
    /// The seeded form writes the ref seed VAR — statement-level intercept
    /// (direct rhs of a blocking assign), like the queue pops.
    Random,
    /// `$urandom[(seed)]` (v7) — unsigned 32-bit. Implementation-defined by
    /// IEEE; vitamin pins its own generator (documented, 3-OS deterministic).
    Urandom,
    /// `$urandom_range(max[, min])` (v7) — inclusive range, arg order
    /// auto-swapped per IEEE §18.13.3.
    UrandomRange,
    /// `$countones(x)` (v7) — 1-bits; x/z positions do not count.
    CountOnes,
    /// `$onehot(x)` (v7) — exactly one 1-bit (x/z don't count).
    OneHot,
    /// `$onehot0(x)` (v7) — at most one 1-bit.
    OneHot0,
    /// `$isunknown(x)` (v7) — any x/z bit.
    IsUnknown,
    /// `$stime` (v7) — current time scaled to the caller's module, truncated
    /// to unsigned 32 bit.
    Stime,
    /// `$fopen(name[, mode])` (v7) — side-effecting (file table) — statement-
    /// level intercept as the direct rhs of a blocking assign.
    Fopen,
    /// `$sformatf(fmt, args...)` (v7) — formatted string VALUE, materialized
    /// as a packed-ASCII vector (8×len bits, string-net write strips NULs).
    Sformatf,
    /// `$test$plusargs(str)` (v7) — plusarg prefix probe, 1'b1/1'b0.
    TestPlusargs,
    /// `$value$plusargs(fmt, var)` (v7) — writes the ref VAR on match —
    /// statement-level intercept, like `Random`'s seeded form.
    ValuePlusargs,
    /// string `.len()` (v7; args = [handle Signal]).
    StrLen,
    /// string `.getc(i)` (v7; args = [handle, i]) — byte at i, 0 if OOB.
    StrGetC,
    /// string `.substr(i, j)` (v7; args = [handle, i, j]) — inclusive byte
    /// range; empty string on invalid range (IEEE §6.16.8).
    StrSubstr,
    /// string `.toupper()` (v7; args = [handle]) — ASCII-mapped copy.
    StrToUpper,
    /// string `.tolower()` (v7; args = [handle]).
    StrToLower,
    /// String lexicographic compare (v7; args = [a, b]) — signed <0/0/>0 like
    /// C `strcmp`. Backs both the `.compare()` method and string relational
    /// operators (packed compare zero-extends MSB-side, which is NOT
    /// lexicographic for unequal lengths).
    StrCmp,
    // ── v9 (2026-06-18): file-read family + $dist_* + $cast (Medium bundle
    //    rank 4 shape bump; semantics landed in ranks 5-6 as post-bump IR-0
    //    slices). WIRED & emitted by elaborate: Fgets, Fscanf, Sscanf, Fread,
    //    Feof, Fgetc, Ungetc, DistUniform, Cast (FUNCTION form). Still inert
    //    (not yet emitted): DistNormal/DistExponential/DistPoisson/DistChiSquare
    //    (non-uniform `$dist_*` remain loud-reject). ──
    /// `$fgets(str, fd)` (v9) — read a line into the str VAR; returns the byte
    /// count. Side-effecting (writes str + advances fd) — statement-level
    /// intercept as the direct rhs of a blocking assign, like `Fopen`.
    Fgets,
    /// `$fscanf(fd, fmt, args...)` (v9) — formatted read from fd; returns the
    /// match count and writes the ref VAR args. Statement-level intercept.
    Fscanf,
    /// `$sscanf(str, fmt, args...)` (v9) — like `Fscanf` but reads from a string
    /// VALUE instead of a file descriptor.
    Sscanf,
    /// `$fread(target, fd)` (v9) — binary read into a reg/memory; returns the
    /// byte count. Statement-level intercept.
    Fread,
    /// `$feof(fd)` (v9) — nonzero once fd is at end-of-file (lazy/post-read).
    Feof,
    /// `$fgetc(fd)` (v9) — next byte from fd, or -1 (0xFFFF_FFFF) at EOF.
    Fgetc,
    /// `$ungetc(c, fd)` (v9) — push a byte back onto fd; returns the byte
    /// (own contract: 0 on failure).
    Ungetc,
    /// `$dist_uniform(seed, start, end)` (v9) — seeded; writes the ref seed VAR
    /// (statement-level intercept). IEEE Annex K integer algorithm, ported
    /// faithfully for 3-OS byte-identical streams.
    DistUniform,
    /// `$dist_normal(seed, mean, std_dev)` (v9) — see `DistUniform`.
    DistNormal,
    /// `$dist_exponential(seed, mean)` (v9) — see `DistUniform`.
    DistExponential,
    /// `$dist_poisson(seed, mean)` (v9) — see `DistUniform`.
    DistPoisson,
    /// `$dist_chi_square(seed, degree_of_freedom)` (v9) — see `DistUniform`.
    DistChiSquare,
    /// `$cast(dest, source)` FUNCTION form (v9) — returns 1 on a legal cast
    /// (and writes dest), 0 otherwise. Statement-level intercept (writes the
    /// dest VAR). The TASK form is `SysTaskId::Cast`.
    Cast,
    // ── v15 (2026-06-24): array reduction methods (IEEE §7.12.3). args =
    //    [handle Signal]. The result takes the ELEMENT type (width/sign of the
    //    handle net); the fold runs left-to-right starting from element 0 and
    //    wraps at the element width. An empty array yields the element-type 0
    //    (documented pin — IEEE leaves the empty case to the accumulator
    //    default). x/z elements propagate through the engine's normal 4-state
    //    arithmetic/bitwise. PURE (a read query — no heap mutation). ──
    /// array `.sum()` — left-fold `+` over the elements.
    ArrSum,
    /// array `.product()` — left-fold `*`.
    ArrProduct,
    /// array `.and()` — left-fold bitwise `&`.
    ArrAnd,
    /// array `.or()` — left-fold bitwise `|`.
    ArrOr,
    /// array `.xor()` — left-fold bitwise `^`.
    ArrXor,
    // ── v18 (2026-06-24): ⓑ-breadth string→number conversion methods
    //    (IEEE §6.16.9-12). args = [str handle]. Each returns a 32-bit int (or
    //    real for `atoreal`) by parsing the leading numeric prefix in the given
    //    base; a leading sign is honored (atoi); parsing stops at the first
    //    non-matching character; empty/garbage → 0. ──
    /// string `.atoi()` — decimal ASCII → int (leading sign honored, §6.16.9).
    StrAtoi,
    /// string `.atohex()` — hex ASCII → int.
    StrAtohex,
    /// string `.atooct()` — octal ASCII → int.
    StrAtooct,
    /// string `.atobin()` — binary ASCII → int.
    StrAtobin,
    /// string `.atoreal()` — ASCII → real (64-bit, §6.16.13).
    StrAtoreal,
    // ── v19 (2026-06-25): N6 real-math system functions (IEEE 1800 §20.8.2).
    //    Each takes real arg(s) — an integral arg coerces to real (signed) — and
    //    returns `real` (64-bit, `is_real` established at eval). Computed via the
    //    vendored pure-Rust libm (third_party/libm, default-features=false → no
    //    hardware intrinsics → 3-OS byte-identical). PURE (no side effect). ──
    /// `$ln(x)` — natural log.
    Ln,
    /// `$log10(x)` — base-10 log.
    Log10,
    /// `$exp(x)` — e^x.
    Exp,
    /// `$sqrt(x)` — square root (IEEE-754 correctly rounded).
    Sqrt,
    /// `$pow(x, y)` — x^y.
    Pow,
    /// `$floor(x)` — round toward −∞ (real result).
    Floor,
    /// `$ceil(x)` — round toward +∞ (real result).
    Ceil,
    /// `$sin(x)`.
    Sin,
    /// `$cos(x)`.
    Cos,
    /// `$tan(x)`.
    Tan,
    /// `$asin(x)`.
    Asin,
    /// `$acos(x)`.
    Acos,
    /// `$atan(x)`.
    Atan,
    /// `$atan2(y, x)`.
    Atan2,
    /// `$hypot(x, y)` — sqrt(x²+y²) without intermediate overflow.
    Hypot,
    /// `$sinh(x)`.
    Sinh,
    /// `$cosh(x)`.
    Cosh,
    /// `$tanh(x)`.
    Tanh,
    /// `$asinh(x)`.
    Asinh,
    /// `$acosh(x)`.
    Acosh,
    /// `$atanh(x)`.
    Atanh,
    // ── v19 cont.: non-uniform `$dist_*` transcendentals that need NEW ids
    //    (DistNormal/DistExponential/DistPoisson/DistChiSquare already exist from
    //    v9 but were inert). Like DistUniform these advance the ref seed VAR
    //    (statement-level intercept) and return `int`; their Annex algorithms run
    //    on the vendored libm so the streams are 3-OS byte-identical. ──
    /// `$dist_t(seed, degree_of_freedom)` — Student's t (rounded to int).
    DistT,
    /// `$dist_erlang(seed, k_stage, mean)` — Erlang-k.
    DistErlang,
}

/// Expression arena node (§1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum Expr {
    Const {
        val: u32,
    },
    Signal {
        net: u32,
        word: Option<u32>,
    },
    Select {
        base: u32,
        offset: u32,
        width: u32,
        kind: sim_ir::SelKind,
    },
    Concat {
        parts: Vec<u32>,
    },
    Replicate {
        count: u32,
        value: u32,
    },
    Unary {
        op: sim_ir::UnOp,
        operand: u32,
    },
    Binary {
        op: sim_ir::BinOp,
        lhs: u32,
        rhs: u32,
    },
    Ternary {
        cond: u32,
        then_e: u32,
        else_e: u32,
    },
    SysFunc {
        which: sim_ir::SysFuncId,
        args: Vec<u32>,
    },
    Call {
        func: u32,
        args: Vec<u32>,
    },
    /// ⓑ-breadth (v17): the per-element iterator value inside an array method's
    /// `with` clause (IEEE §7.12). `index=false` reads the current element's
    /// VALUE (sized to `width`/`signed` = the element type); `index=true` reads
    /// its 0-based position (`width`/`signed` = 32-bit signed). The engine drives a
    /// scratch register during the reduction/locator fold; outside a fold it reads
    /// X (defensive — elaborate only emits this inside a with-expr). The `width`/
    /// `signed` ride in the node so the static width table can size it.
    ArrayItem {
        index: bool,
        width: u32,
        signed: bool,
    },
}

/// System-task id (§2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum SysTaskId {
    Display,
    Write,
    Monitor,
    Strobe,
    Finish,
    Stop,
    DumpFile,
    DumpVars,
    DumpOn,
    DumpOff,
    DumpAll,
    /// `$dumpflush` — flush the VCD sink now (format_version 4).
    DumpFlush,
    /// `$dumplimit(bytes)` — byte budget; on exceeding it the writer emits a
    /// one-time `$comment Dump limit reached $end` and drops further records
    /// (format_version 4).
    DumpLimit,
    /// dyn array `= new[n]` / `new[n](src)` (v5; args = [handle, n (, src)]).
    DynNew,
    /// dyn/queue/assoc `.delete()` — whole-object clear (v5; args = [handle]).
    DynDelete,
    /// queue `.push_back(v)` (v5; args = [handle, v]).
    QPushBack,
    /// queue `.push_front(v)` (v5; args = [handle, v]).
    QPushFront,
    /// assoc `.delete(key)` (v5; args = [handle, key]).
    AssocDeleteKey,
    /// queue `.insert(i, v)` (v6; args = [handle, i, v]) — IEEE §7.10.2.2,
    /// legal i ∈ [0, size] (i == size appends); OOB/X = warn + no-op.
    QInsert,
    /// queue `.delete(i)` — single-index erase (v6; args = [handle, i]) —
    /// IEEE §7.10.2.3, legal i ∈ [0, size-1]; OOB/X = warn + no-op.
    QDeleteIdx,
    /// `$fclose(fd)` (v7).
    Fclose,
    /// `$fdisplay(fd, fmt, args...)` (v7) — fd is args[0]; fmt/args split
    /// follows the print family (fmt = first STRING-LITERAL arg after fd).
    Fdisplay,
    /// `$fwrite(fd, fmt, args...)` (v7) — `Fdisplay` without the newline.
    Fwrite,
    /// `$sformat(dest, fmt, args...)` (v7) — renders into the dest VAR
    /// (string net or packed); dest is args[0].
    Sformat,
    /// `$readmemb(file, mem[, start[, finish]])` (v7).
    ReadmemB,
    /// `$readmemh(file, mem[, start[, finish]])` (v7).
    ReadmemH,
    /// string `.putc(i, c)` (v7; args = [handle, i, c]) — in-place byte
    /// write; OOB index or NUL byte = no-op (IEEE §6.16.3).
    StrPutC,
    // ── v9 (2026-06-18): $writememb/h, $cast (task form), $monitoron/off
    //    (Medium bundle rank 4 shape bump; semantics landed in ranks 5-6).
    //    All five are WIRED & emitted by elaborate: WritememB, WritememH, Cast
    //    (TASK form), MonitorOn, MonitorOff. ──
    /// `$writememb(file, mem[, start[, finish]])` (v9) — binary memory dump,
    /// the write-side mirror of `ReadmemB`.
    WritememB,
    /// `$writememh(file, mem[, start[, finish]])` (v9) — hex memory dump.
    WritememH,
    /// `$cast(dest, source)` TASK form (v9) — performs the cast for side effect
    /// (an illegal cast is a runtime error). The FUNCTION form that returns a
    /// success flag is `SysFuncId::Cast`.
    Cast,
    /// `$monitoron` (v9) — (re)enable the `$monitor` change-watch (default on).
    MonitorOn,
    /// `$monitoroff` (v9) — disable the `$monitor` change-watch.
    MonitorOff,
    /// N7-REST `obj.randomize()` (v10) — draw the receiver's `rand` fields per the
    /// folded constraint bounds (`SimOpts.class_rand`). args = `[obj_handle]`. The
    /// success flag (always 1 in Phase B1, with feasible constraints) is assigned by
    /// a sibling `BlockingAssign` the lowering emits for `r = obj.randomize()`.
    ClassRandomize,
    // ── v16 (2026-06-24): ⓑ-breadth array ordering methods (IEEE §7.12.2).
    //    In-place mutators on a dyn array / queue handle. args = [handle]. Sort
    //    uses the element's signed/unsigned numeric order; x/z elements fall
    //    back to a deterministic raw-bit total order (never panics). ──
    /// array `.sort()` — ascending in-place sort.
    ArrSort,
    /// array `.rsort()` — descending in-place sort.
    ArrRsort,
    /// array `.reverse()` — in-place element reversal.
    ArrReverse,
    // ── v18 (2026-06-24): ⓑ-breadth number→string conversion methods
    //    (IEEE §6.16.14-18). In-place mutators: write the ASCII rendering of the
    //    value argument into the string handle. args = [str handle, value]. ──
    /// string `.itoa(i)` — decimal rendering (leading `-` for negatives).
    StrItoa,
    /// string `.hextoa(i)` — lowercase hex rendering.
    StrHextoa,
    /// string `.octtoa(i)` — octal rendering.
    StrOcttoa,
    /// string `.bintoa(i)` — binary rendering.
    StrBintoa,
    /// ⓑ-breadth (v17): array LOCATOR methods that yield a queue (IEEE §7.12.1).
    /// Statement-level (only valid as the direct rhs of a blocking assign — the
    /// result is a whole queue, written into the dst handle). args =
    /// `[dst_handle, src_handle, kind_const, with_expr?]` where `kind_const` is a
    /// Const expr selecting the locator (0=min 1=max 2=unique 3=find 4=find_index
    /// 5=find_first 6=find_last 7=find_first_index 8=find_last_index 9=unique_index)
    /// and `with_expr` (present for the find* family) is the per-element predicate
    /// expression referencing `ArrayItem`.
    ArrLocator,
}

/// Disable kind (§2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum DisableKind {
    Fork,
    Scope,
}

/// Statement arena node (§2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum Stmt {
    BlockingAssign {
        lhs: sim_ir::Lvalue,
        rhs: u32,
    },
    NonblockingAssign {
        lhs: sim_ir::Lvalue,
        rhs: u32,
        /// `<= #d` transport delay (v5): ExprId evaluated at EXECUTION time
        /// (the v4 runtime-delay model). Each activation carries its own
        /// captured value to `t+d` (overlapping activations stay independent).
        /// `None` ⇒ plain same-tick NBA (the v4 byte path).
        delay: Option<u32>,
    },
    SysTask {
        which: sim_ir::SysTaskId,
        fmt: Option<u32>,
        args: Vec<u32>,
    },
    Disable {
        scope_kind: sim_ir::DisableKind,
        target: u32,
    },
    /// `force lhs = rhs` (shape reserved at format_version 4; elaborate still
    /// loud-rejects until the force/release semantics increment lands — the
    /// engine treats it as a defensive no-op meanwhile).
    Force {
        lhs: sim_ir::Lvalue,
        rhs: u32,
    },
    /// `release lhs` (shape reserved at format_version 4 — see `Force`).
    Release {
        lhs: sim_ir::Lvalue,
    },
}

/// Assignment target (§3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Lvalue {
    pub chunks: Vec<sim_ir::LvalChunk>,
}

/// One chunk of an lvalue (§3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct LvalChunk {
    pub net: u32,
    pub word: Option<u32>,
    pub offset: Option<u32>,
    pub width: Option<u32>,
    pub kind: sim_ir::SelKind,
}

/// Delay scheduling region (§4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum DelayRegion {
    Active,
    Inactive,
}

/// In-body wait cause (§4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum WaitCause {
    Edge {
        net: u32,
        kind: sim_ir::EdgeKind,
    },
    Level {
        nets: Vec<u32>,
    },
    Expr {
        expr: u32,
    },
    Named {
        ev: u32,
    },
    /// `wait fork;` (IEEE §9.6.1) — block until all child processes forked by
    /// the current process complete. v8: unit variant (no payload — the
    /// `Terminator::Wait { resume }` already carries the resume block). Never
    /// satisfied by a net/expr edge; resolved by an implicit join barrier in
    /// the scheduler (the feature slice wires it; the bump leaves it inert).
    Fork,
}

/// Basic-block terminator (§4, RULE-D2 Fork/Call verbatim).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum Terminator {
    Goto {
        target: u32,
    },
    Branch {
        cond: u32,
        then_bb: u32,
        else_bb: u32,
    },
    Delay {
        /// ExprId of the delay VALUE in the process's module TIME UNITS — raw
        /// and unscaled (format_version 4; was a pre-scaled literal tick count
        /// in v3). The engine evaluates it AT SUSPENSION TIME and converts:
        /// real → round(v × M), integral → v × M (u64-saturating), any X/Z
        /// → 0 ticks (iverilog parity), where M is the per-process timescale
        /// multiplier. A const `#5`/`#2.5` simply folds to a Const expr.
        amount: u32,
        region: sim_ir::DelayRegion,
        resume: u32,
    },
    Wait {
        cond: sim_ir::WaitCause,
        resume: u32,
    },
    Fork {
        children: Vec<u32>,
        join: u32,
        resume_bb: u32,
    },
    Call {
        target: u32,
        ret_bb: u32,
    },
    Return,
}

/// Sensitivity kind (§6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum SensKind {
    Initial,
    Comb,
    Latch,
    Edge,
    Level,
}

/// One edge entry in a sensitivity list (§6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct EdgeTerm {
    pub net: u32,
    pub kind: sim_ir::EdgeKind,
}

/// Process sensitivity (§6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Sensitivity {
    pub kind: sim_ir::SensKind,
    pub edges: Vec<sim_ir::EdgeTerm>,
}

/// Net/variable kind (§6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum NetKind {
    Wire,
    Reg,
    Logic,
    Integer,
    /// IEEE-754 f64 net (`real`/`realtime`). 64-bit, signed, 2-state. The f64 is
    /// stored as `f64::to_bits()` in `init.val[0]`, `init.unk` all-zero. No f64
    /// field is introduced — the V-PRIM derive guard sees only `u64` inside
    /// `BitPacked`. `realtime` is a synonym and ALSO maps here (no 6th variant).
    Real,
    /// SV dynamic-array HANDLE (v5). `width` = ELEMENT width, `array_len = 0`;
    /// storage lives in the engine heap (`dyn_heap`), never the flat BitPacked
    /// store. Shape reserved by the v5 bump; front-end emission lands with the
    /// dynamic-storage increments (design doc 2026-06-10).
    DynArray,
    /// SV queue HANDLE (v5) — see `DynArray`.
    Queue,
    /// SV associative-array HANDLE (v5; integer keys ≤64 bit) — see `DynArray`.
    Assoc,
    /// SV associative-array HANDLE with STRING keys (v6). Keys live in the
    /// engine heap as raw byte strings (`Vec<u8>`, lexicographic BTree order =
    /// IEEE string compare); integral key expressions convert by stripping
    /// leading 0x00 bytes (packed-ASCII surface, §6.16 family).
    AssocStr,
    /// SV `string` variable (v7). HANDLE-style like `DynArray`: the bytes live
    /// in the engine heap (`Vec<u8>`); `width` = 0, `array_len` = 0. Reads
    /// materialize a packed-ASCII value (8×len bits, MSB-first); writes strip
    /// leading 0x00 bytes (IEEE §6.16 packed↔string conversion).
    String,
}

/// Port direction (§6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum PortDir {
    Input,
    Output,
    Inout,
    Internal,
}

/// 2-plane 4-state bit vector (§6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct BitPacked {
    pub val: Vec<u64>,
    pub unk: Vec<u64>,
}

/// Net/variable arena entry (§6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct NetVar {
    pub kind: sim_ir::NetKind,
    pub width: u32,
    pub msb: u32,
    pub lsb: u32,
    pub signed: bool,
    pub array_len: u32,
    pub dir: sim_ir::PortDir,
    pub init: sim_ir::BitPacked,
}

/// Constant representation tag (§6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum ConstRepr {
    Numeric,
    StrUtf8,
    /// IEEE-754 f64 literal. `ConstVal.width = 64`, `signed = true`,
    /// `bits.val[0] = literal.to_bits()`, `bits.unk = [0]`. No f64 field.
    Real,
}

/// Constant pool entry (§6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ConstVal {
    pub width: u32,
    pub signed: bool,
    pub repr: sim_ir::ConstRepr,
    pub bits: sim_ir::BitPacked,
}

/// Control-flow basic block (§7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct BasicBlock {
    pub stmts: Vec<u32>,
    pub term: sim_ir::Terminator,
}

/// Process (§7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Process {
    pub sensitivity: sim_ir::Sensitivity,
    pub body: Vec<sim_ir::BasicBlock>,
    pub entry: u32,
    pub suspend: sim_ir::SuspendState,
}

/// Continuous assignment (§7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ContAssign {
    pub lhs: sim_ir::Lvalue,
    pub rhs: u32,
    pub delay: Option<u32>,
}

/// Module instance (§7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Instance {
    pub parent: Option<u32>,
    pub module: u32,
    pub first_net: u32,
    pub net_count: u32,
}

/// Function/task definition (§7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct FuncDef {
    pub entry: u32,
    pub n_params: u32,
    pub locals_len: u32,
    pub is_task: bool,
}

/// Golden root — `schema_hash::<sim_ir::SimIr>()` is the pinned gate (§7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct SimIr {
    pub instances: Vec<sim_ir::Instance>,
    pub nets: Vec<sim_ir::NetVar>,
    pub processes: Vec<sim_ir::Process>,
    pub cont_assigns: Vec<sim_ir::ContAssign>,
    pub funcs: Vec<sim_ir::FuncDef>,
    pub exprs: Vec<sim_ir::Expr>,
    pub stmts: Vec<sim_ir::Stmt>,
    pub blocks: Vec<sim_ir::BasicBlock>,
    pub consts: Vec<sim_ir::ConstVal>,
}

/// Phase B2 constraint-solver predicate, compiled to a flat POSTFIX program over
/// candidate `rand`-field values (as i64), evaluated by the `randomize()`
/// rejection-sampling solver. NOT part of the frozen `SimIr` root and NOT
/// `SchemaHash`-derived — it rides the out-of-band `class_constraints` sidecar
/// (no schema-hash / format-version impact), exactly like the `class_rand`
/// `[lo,hi]` bounds. `inside`/implication lower to these ops (set membership ⇒
/// `||` of `==`/range tests; `a -> b` ⇒ `!a || b`), so one evaluator covers them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum COp {
    /// Push the candidate value (sign-extended i64) of rand field `field_idx`.
    Field(u32),
    /// Push a constant.
    Const(i64),
    /// Pop `b`, then `a`; push `a OP b` (relational/logical results are 0/1).
    Bin(CBinOp),
    /// Pop `a`; push `1` if `a == 0` else `0` (logical NOT).
    Not,
    /// SOFT-constraint marker (IEEE §18.5.14): a no-op stack effect that tags this
    /// predicate as soft. The rejection solver first tries to satisfy hard+soft; if
    /// that is infeasible it DROPS the soft predicates and retries with hard only.
    /// Always the FIRST op of a soft predicate. Appended variant ⇒ existing
    /// (hard-only) sidecars decode unchanged (no format/schema impact).
    SoftMarker,
}

/// Binary operator for a [`COp::Bin`] step (Phase B2 constraint predicate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}
