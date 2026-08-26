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
/// Re-pinned 2026-08-26 R6 `TfPort.dir_spelling: TfDirSpelling` — the parser maps
/// `ref` and `const ref` onto `PortDir::Inout`, which left every diagnostic about
/// such a formal printing "inout formal" for source that contains no such keyword.
/// The field carries the user's own word for message fidelity ONLY; nothing
/// branches on it semantically and `Declared` (every non-`ref` formal) reproduces
/// the prior behaviour exactly. All `.vu` artifacts are stale; no sim-ir change,
/// format_version unchanged.
/// Re-pinned 2026-08-19 §16.15 `default disable iff (expr);`
/// (`ModuleItem::DefaultDisableIff(Expr)` — the scope-level reset every concurrent
/// assertion inherits unless it writes its own; all `.vu` artifacts are stale, no
/// sim-ir/format_version change, pure IR-0: elaborate records it per module and the
/// assertion drain fills an absent `disable_iff` from it).
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
/// 2026-07-10 round-5 Gap B body-local enum (§6.18): adds `body_enums:
/// Vec<TypedefDecl>` to `FunctionDef` and `TaskDef` — a body-local `typedef enum`
/// in a function/task body carries its labels to elaborate for label-constant
/// registration scoped to the function (the type NAME + `e'(x)` casts were already
/// parse-resolved). Pure front-end + elaborate (no sim-ir change; format_version
/// stays 19). Re-pins this .vu hash.
/// 2026-07-10 round-6 UARR unpacked-array tf-port (§13.3): adds `unpacked:
/// Vec<Dim>` to `TfPort` — an unpacked-array subroutine formal
/// (`input logic [63:0] words [0:7]`) carries its dims so elaborate lowers a
/// single-dim zero-based formal as an md-packed frame slot (call-site concat) or
/// loud-classifies the rest. Pure front-end + elaborate (no sim-ir change;
/// format_version stays 19). Re-pins this .vu hash.
/// 2026-07-10 round-9 bind (§23.11): adds `TopItem::Bind(BindDecl)` — a top-level
/// `bind <target> <checker> u(...)` attaches an observer checker instance inside
/// every instantiation of the target module (elaborate reuses the ordinary
/// child-instantiation path; no sim-ir change; format_version stays 19). Re-pins
/// this .vu hash.
/// 2026-07-13 round-10 report gaps: adds `FunctionDef.ret_string` (G4 `string`
/// return type), `EventExpr.iff` (G7 `@(edge sig iff g)` event guard), and three
/// `ExprKind` variants — `TimeLit{num,unit_exp}` (G11 `1ns`/`10ps` time literals),
/// `NamedArg{formal,value}` (G10 `.formal(v)` named call args), `MethodCall{recv,
/// method,args}` (G8 chained method call `s.substr(a,b).atoi()`). All desugar in the
/// front end / elaborate (no sim-ir change; format_version stays 19). Re-pins this hash.
/// 2026-07-19 §4.5.158 enum-label sign: adds `TypedefKind::Enum.signed` (the base's
/// declared signedness) so a label reference lowers with the enum's sign — a POSITIVE
/// label of a signed enum stays signed in a relational/collective context. Front-end +
/// elaborate only (no sim-ir change; format_version stays 22). Re-pins this .vu hash.
/// Re-pinned 2026-07-21 §4.5.184 multi-dim packed struct/union member
/// (`StructMember.packed_dims: Vec<Range>` — the INNER packed dims of a
/// `logic [1:0][3:0] m` member; empty for a single-dim member). Front-end only (the
/// member desugars to a flat part-select in the parser; no sim-ir change,
/// format_version stays 22). All `.vu` artifacts are stale. Re-pins this .vu hash.
/// Re-pinned 2026-08-03 §4.5.284 IEEE 1364-2005 §3.5 implicit nets — THREE fields:
/// `ModuleDecl.nettype_none` (was `` `default_nettype none `` in effect where this module
/// was declared — it rides the AST so the staged `.vu` carries the policy that governed
/// the source `velab` never sees), `ContinuousAssign.from_gate` (a gate-primitive desugar,
/// whose read terminals are §3.5 positions while an ordinary `assign` rhs is not), and
/// `PortConn.implicit_name` (the `.name` shorthand, which desugars to `.name(name)` but
/// is NOT a §3.5 position — IEEE 1800 §23.3.2.2 requires a declared object). Front-end
/// only; no sim-ir change, format_version stays 26. All `.vu` artifacts are stale.
/// Re-pinned 2026-08-07 (aes_top report §3.2) UNPACKED ARRAY PORTS — two fields:
/// `AnsiPort.unpacked: Vec<Dim>` and `PortDecl.unpacked: Vec<Vec<Dim>>` (per name,
/// parallel to `names`, so the frozen `names` shape is untouched). A port written
/// `output logic [7:0] o [4]` is legal IEEE 1800 §23.2.2.3 and iverilog accepts it;
/// vita rejected it at the `[` in BOTH the ANSI header and the non-ANSI body form,
/// so a design passing 15×128-bit round keys had to be rewritten to a flat packed
/// bus. The dims ride the AST the same way `TfPort.unpacked` (round-6 UARR) does,
/// elaborate sizes the port net from them, and a port connection wires ELEMENT BY
/// ELEMENT (there is no whole-array value in this IR — one whole-net cont-assign
/// would silently connect word 0 only). Front-end + elaborate; no sim-ir change,
/// format_version stays 26. All `.vu` artifacts are stale.
/// Re-pinned 2026-08-26 (V34-3) KEYED assignment patterns (IEEE 1800 §10.9.1/
/// §10.9.2) — `ExprKind::AssignPatternKeyed(Vec<(AssignPatternKey, Expr)>)` plus
/// the `AssignPatternKey` enum (`Default` / `Member(String)`). A SIBLING of the
/// positional `AssignPattern`, not a widening of it: the positional payload is read
/// as "one expression per position" at ~46 sites, and a keyed pattern has no
/// position. `'{mode: 4'h3, en: 1'b1, len: 8'd7}` and `'{default: v}` used to be
/// parse errors, so a config struct had to be written positionally — where
/// inserting a member silently shifts every later value. Front-end + elaborate
/// only: a packed-struct target is resolved to the existing field-width concat in
/// the PARSER and an unpacked-array `'{default: v}` expands to the existing
/// positional lowering in elaborate, so no sim-ir change — `format_version` STAYS
/// 29 and the SimIr schema hash / canonical / RON goldens are untouched (verified:
/// the only test this slice moves is this one). All `.vu` artifacts are stale.
const EXPECTED: [u8; 32] = [
    13, 232, 81, 64, 26, 92, 92, 2, 89, 14, 127, 239, 98, 187, 188, 150, 168, 75, 123, 113, 149,
    77, 77, 121, 167, 183, 141, 71, 17, 218, 155, 209,
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
