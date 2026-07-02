//! Golden SchemaHash gate for the hdl-ast `.vu` root (`SourceUnit`).
//!
//! The hash collapses the whole AST type-reachability closure into one blake3
//! value. Box-recursive `Expr` is handled by the derive's transparent `Box<T>`
//! arm + the registry's `insert_once` cycle guard, so the recursive AST hashes
//! without infinite recursion. Any field add/remove/reorder anywhere in the
//! reachable AST flips this hash, signalling that every `.vu` artifact is stale.
//!
//! When this fails after a DELIBERATE AST shape change, re-pin `EXPECTED` to the
//! new value (and bump the `.vu` format_version when the staged flow lands).

use vita_schema::schema_hash;

/// Pinned root hash of `hdl_ast::SourceUnit`'s full type closure.
/// Re-pinned 2026-06-19 N7 `return` statement (`Stmt::Return{value: Option<Expr>}`
/// — SV `return [expr];`, used pervasively by class methods; all `.vu` artifacts
/// stale, no sim-ir/format_version change, pure IR-0: lowers to a return-var
/// assign + jump to the body exit block). (Previous re-pins:
/// 2026-06-19 N7 class/OOP skeleton (`TopItem::Class` +
/// `ModuleItem::Class` + `ClassDecl`/`ClassItem` + `NetVarKind::ClassHandle` +
/// `NetVarDecl.class_type: Option<Ident>` + `ExprKind::{ClassNew,Null}` —
/// `class`/`extends`/`virtual`/`new`/`null`; all `.vu` artifacts are stale, no
/// sim-ir/format_version change, pure IR-0: class objects live in the engine
/// `class_heap` with `NetKind::Integer` handle nets + layout/vtable sidecars).
/// (Previous re-pins:
/// 2026-06-19 SVPART 2-state integer types (`NetVarKind::{Bit,Byte,
/// Shortint,Int,Longint}` — `bit`/`byte`/`shortint`/`int`/`longint`; all `.vu`
/// artifacts are stale, no sim-ir/format_version change, pure IR-0: these map to
/// `NetKind::Reg` storage with fixed widths/sign and a 2-state 0-init). (Previous re-pins:
/// 2026-06-19 N5 slice D coverage options (`Coverpoint.at_least`/`weight:
/// Option<Expr>` + `CovergroupDecl.at_least: Option<Expr>` — `option.at_least`/
/// `option.weight`; all `.vu` artifacts are stale, no sim-ir/format_version change,
/// pure IR-0: at_least>1 uses per-bin saturating counters, weight enters the
/// get_coverage weighted average). (Previous re-pins:
/// 2026-06-19 N5 slice C cross coverage (`CovergroupDecl.crosses:
/// Vec<CrossSpec>` + the `CrossSpec` struct — `cross cp_a, cp_b;`; all `.vu`
/// artifacts are stale, no sim-ir/format_version change, pure IR-0: a product
/// hit-bitmap whose bit fires when every constituent coverpoint's bin matches the
/// same sample). (Previous re-pins:
/// 2026-06-19 N5 slice F covergroup sampling event (`CovergroupDecl.clock:
/// Option<Sensitivity>` — `covergroup cg @(posedge clk);` auto-samples each instance;
/// all `.vu` artifacts are stale, no sim-ir/format_version change, pure IR-0: a
/// synthesized `always @(clk) inst.sample();` per clocked instance). (Previous re-pins:
/// 2026-06-19 N5 slice A explicit coverage bins (`Coverpoint.iff:
/// Option<Expr>` + `Coverpoint.bins: Vec<BinSpec>` + the `BinSpec`/`BinKind`/
/// `BinArray`/`CoverRange`/`RangeEnd` types — `coverpoint x [iff(g)] { bins a =
/// {0,[2:4]}; ignore_bins/illegal_bins/default … }`; all `.vu` artifacts are stale,
/// no sim-ir/format_version change, pure IR-0: per-bin membership predicate →
/// counting-bin bits in the existing 64-bit hit-bitmap. `iff` is reserved here
/// (parsed, elaborate loud-rejects) for the guard slice. (Previous re-pins:
/// 2026-06-17 N2d recursive-property + property-level `and`/`or`
/// (`PropExpr` enum + `ConcurrentAssert.prop_expr` / `PropDecl.prop_expr:
/// Option<PropExpr>` — the `and`/`or`/recursion layer above a flat implication;
/// `None` = the byte-identical flat path; all `.vu` artifacts are stale, no
/// sim-ir/format_version change, pure IR-0: `synth_prop_expr` reduces the tree to
/// a per-clock boolean violation check). (Previous re-pins:
/// 2026-06-17 B4 frame-call variable-lifetime override
/// (`NetVarDecl.lifetime: Option<bool>` — `automatic int x;` in a frame
/// function/task body_decl gives that local fresh-per-call storage; all `.vu`
/// artifacts are stale, no sim-ir/format_version change, pure IR-0: the per-slot
/// lifetime rides the engine routing side table out-of-band).
/// 2026-06-17 N2a-1 multi-clock SVA sequence boundary
/// (`Sequence::Clocked { clock, seq }` — a `##`-boundary re-clocking event
/// `a ##1 @(c2) b`; all `.vu` artifacts are stale, no sim-ir/format_version change,
/// pure IR-0: a dedicated cross-clock two-process handoff synthesis). (Previous re-pins:
/// 2026-06-17 N1 non-blocking intra-assignment event control
/// (`Stmt::NonBlocking.event: Option<IntraEvent>` — `lhs <= [repeat(n)] @(ev) rhs`;
/// all `.vu` artifacts are stale, no sim-ir/format_version change, pure IR-0:
/// capture-now / `fork … join_none` / NBA-write desugar).
/// 2026-06-17 B4 per-variable lifetime override (`NetVarDecl.lifetime:
/// Option<bool>` — block-/variable-level `automatic` for frame functions/tasks);
/// 2026-06-16 deferred immediate assertions (`Stmt::DeferredAssert` +
/// `AssertDefer` enum — `assert #0` (Observed) / `assert final` (Reactive); all
/// `.vu` artifacts are stale, no sim-ir/format_version change, pure IR-0: a
/// flush-marker + region maturation queues carried out-of-band).
/// 2026-06-19 N5 functional coverage (`ModuleItem::{Covergroup,CoverInstance}` plus
/// the `CovergroupDecl`/`Coverpoint`/`CoverInstance` structs — `covergroup … endgroup`
/// and `cg c = new;`; all `.vu` artifacts are stale, no sim-ir/format_version change,
/// pure IR-0 bitmap-synthesis). (Previous re-pins:
/// 2026-06-16 intra-assignment event control (`Stmt::Blocking.event:
/// Option<IntraEvent>` + new `IntraEvent` struct — `lhs = [repeat(n)] @(ev) rhs`;
/// all `.vu` artifacts are stale, no sim-ir/format_version change, pure IR-0
/// capture/wait/write desugar). (Previous re-pins:
/// 2026-06-16 multi-clock slice A3 (`ConcurrentAssert.consequent_clock:
/// Option<Sensitivity>` + `PropDecl.consequent_clock` — the `@(c2)` consequent clock
/// of `@(c1) ante |=> @(c2) cons`; all `.vu` artifacts are stale, no
/// sim-ir/format_version change, pure IR-0). (Previous re-pins:
/// 2026-06-16 named-SVA slice (`Sequence::Instance` +
/// `ModuleItem::{SequenceDecl,PropertyDecl}` + `SeqDecl`/`PropDecl` — named
/// `sequence`/`property` declarations & instantiation);
/// 2026-06-15 SVA slice S14 (`ConcurrentAssert.consequent: Expr →
/// Sequence` — sequence consequent AST flip; all `.vu` artifacts are stale, no
/// sim-ir/format_version change); 2026-06-15 S12
/// `ConcurrentAssert.disable_iff: Option<Expr>`; 2026-06-15 S11
/// `ConcurrentAssert.{pass,fail}: Option<Box<Stmt>>`; 2026-06-15 S9
/// `Sequence::Within`; 2026-06-15 S8 `Sequence::Repeat.kind: RepeatKind`;
/// 2026-06-15 S7 `Sequence::Throughout`; 2026-06-15 S4 `Sequence` enum +
/// `ConcurrentAssert.antecedent: Expr → Sequence`;
/// 2026-06-14 v8 `Stmt::WaitFork`+`ConcurrentAssert`+`ImplicationKind`;
/// 2026-06-12 P2-E `ProcKind::Final`; 2026-06-12 v7 P2-C/P2-D flip
/// `TopItem::{Package,Import}`+`ImportDecl`+`ModuleItem::Import`+
/// `ExprKind::PkgScoped`+`NetVarKind::String`; 2026-06-11 v6 `AssocKey::Str`;
/// 2026-06-11 v5 ⑥ front-end batch; 2026-06-11 `NetVarKind::Event`;
/// 2026-06-05 `TypedefKind::Struct`; 2026-06-20 SVA-REST property operators
/// `PropExpr::{Not,Until,Eventually,Always}` + `Stmt::CoverProperty` +
/// `ModuleItem::LetDecl`/`LetDecl`; 2026-06-23 N7-REST `ClassItem::{RandProperty,
/// Constraint}` + `ConstraintDecl`; 2026-06-24 N7-REST B-CRV final
/// `ExprKind::RandomizeWith` + `Stmt::RandomizeWith` (inline `randomize() with`);
/// 2026-06-24 ⓑ-breadth array locator + `with` iterator
/// `ExprKind::ArrayMethodWith(Box<ArrayMethodWithExpr>)` + the `ArrayMethodWithExpr`
/// struct — `arr.sum() with (item*2)` / `arr.find() with (item>2)`;
/// 2026-06-24 ⓑ-breadth parameterized classes: `ClassDecl.params: Vec<ClassParam>`,
/// the `ClassParam` struct, and `NetVarDecl.class_args: Vec<Expr>` (`class C #(int
/// W=8)` / `C #(16) h;`). Pure parser monomorphization — no sim-ir/format change.
/// 2026-06-24 ⓑ-breadth virtual interface: `NetVarKind::VirtualIface`
/// (`virtual bus_if vif;`) — elaborate static-alias, IR-0.
/// 2026-06-25 return-slot 2-state: `FunctionDef.ret_two_state: bool` — records
/// that a `function int/byte/shortint/longint/bit` return is 2-state (can't hold
/// X/Z, §6.11.3) so the frame return slot coerces; `ParamType` could not carry it.
/// Pure parser/elaborate (the function routes to the frame path); no sim-ir change.
/// 2026-06-25 N4 clocking: `ModuleItem::Clocking(ClockingDecl)` + the `ClockingDecl`
/// / `ClockingItem` / `ClockingDir` types — `clocking cb @(clk); input/output sig;
/// endclocking` (§14). Front-end foundation; the preponed-region sampler is a
/// pending engine slice (elaborate honest-loud until then). No sim-ir change.)
/// 2026-06-25 SV cast `casting_type'(expr)` (§6.24): adds `ExprKind::Cast`
/// (target, expr) plus the `CastTarget` / `CastPrim` types — covering `int'(e)`,
/// `8'(e)`, `signed'(e)`, `name'(e)`. Pure front-end and elaborate lowering to
/// existing IR (no sim-ir change; format_version stays 19). Re-pins this .vu hash.
/// 2026-06-26 N2c SVA sequence/property LOCAL VARIABLES (§16.10): adds
/// `Sequence::MatchItem { seq, assigns }` (a `(b, x = e)` capture), the
/// `SvaLocalDecl` struct, and `local_vars: Vec<SvaLocalDecl>` on `Stmt::Concurrent
/// Assert` + `PropDecl`. The data-tracking single-capture idiom
/// `(req, d=data) ##1 grant |-> (rdata == d)` lowers to a parallel DATA shift
/// register (elaborate IR-0 — wider regs + NBA shifts + a read substitution);
/// ranges / multi-write / cross-clock are loud. No sim-ir change (format_version
/// stays 19). Re-pins this .vu hash.
/// 2026-06-26 N2c fix: `SvaLocalDecl.unsupported_type: bool` — records that the
/// declared local-var type is NON-integral (`real`/`realtime`/`string`/`event`/
/// class/net) and has no fixed-width data-tracking register. The parser sets it
/// (the width/sign fields are a 1-bit placeholder) and elaborate's
/// `synth_local_var_assert` loud-rejects the capture, closing a silent 1-bit
/// truncation that flipped the assertion verdict. No sim-ir change (format_version
/// stays 19). Re-pins this .vu hash.
/// 2026-06-26 G5 class member access control `local`/`protected` (§8.18): adds
/// the `Visibility` enum and threads it onto `ClassItem::Property(Visibility, …)`
/// as well as a `vis` field on `ClassItem::Func`/`Task`. The parser records the
/// `local`/`protected`/public access qualifier; elaborate enforces it
/// (correct-or-loud: an out-of-scope read/write/call of a local/protected member
/// is a loud E3009, never a silent read of inaccessible storage). Pure front-end
/// and elaborate (no sim-ir change; format_version stays 19). Re-pins this .vu hash.
/// 2026-06-27 net-declaration delays (§6.1.3): adds `delay: Option<Delay>` to
/// `NetVarDecl` — `wire #3 w = a;` / `wire #(2,4) w = a;`. Parsed only for NET
/// kinds; elaborate desugars each net-decl-assignment through the SAME delayed
/// continuous-assign path (uniform `ContAssign.delay` + distinct rise/fall/turnoff
/// `ca_delays` sidecar) it already uses for `assign #d`. Pure front-end + elaborate
/// (no sim-ir change; format_version stays 19). Re-pins this .vu hash.
/// 2026-06-28 tf-port default arguments (§13.5.3): adds `default: Option<Expr>` to
/// `TfPort` — `function f(int a, int b = 10)`. Parsed only for ANSI tf-ports;
/// elaborate fills omitted trailing actuals at the call site. Pure front-end +
/// elaborate (no sim-ir change; format_version stays 19). Re-pins this .vu hash.
/// 2026-07-02 wildcard equality `==?`/`!=?` (§11.4.6): adds `BinOp::WildEq`/
/// `WildNe` — lowered by elaborate to a const-pattern mask & compare (plain
/// `Eq`/`Ne` in the frozen IR; no sim-ir change, format_version stays 19).
/// Re-pins this .vu hash.
/// 2026-07-02 A2a array parameter (§6.20.2): adds `const_param: bool` to
/// `NetVarDecl` — a body `localparam int RHO [0:4] = '{…}` desugars in the
/// parser to the equivalent const variable-array decl; elaborate registers
/// the net as an elaboration constant (every later write is loud). Pure
/// front-end + elaborate (no sim-ir change; format_version stays 19).
/// Re-pins this .vu hash.
const EXPECTED: [u8; 32] = [
    233, 20, 54, 169, 195, 132, 13, 112, 10, 180, 185, 169, 226, 21, 205, 200, 149, 130, 236, 139,
    148, 198, 220, 164, 222, 183, 77, 33, 251, 32, 160, 1,
];

#[test]
fn schema_hash_is_pinned() {
    assert_eq!(
        schema_hash::<hdl_ast::SourceUnit>(),
        EXPECTED,
        "hdl-ast SourceUnit schema shape changed — re-pin EXPECTED and treat all \
         existing .vu artifacts as stale"
    );
}

#[test]
fn schema_hash_is_deterministic() {
    // Same input → identical hash within a process (the registry DFS is
    // order-stable; the recursive Box<Expr> closure terminates via insert_once).
    assert_eq!(
        schema_hash::<hdl_ast::SourceUnit>(),
        schema_hash::<hdl_ast::SourceUnit>()
    );
}
