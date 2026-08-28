//! The inline function fold substitutes a body-local's defining RHS at every
//! REFERENCE to that local, and the arena keeps ONE node that the evaluator then
//! walks once per reference. That is the same defect as a shared `case` scrutinee
//! and an eager `&&` right operand: a DAG walked as a tree.
//!
//! Measured on the inline path at HEAD, each with its `automatic` twin (which is
//! framed) as the control:
//!
//! ```text
//!   reg u; u = $random;  g = u ^ u;         vita 3533466533   ivl 0
//!          u = $urandom; g = u ^ u;         vita 2354591315   ivl 0
//!          u = sf(a);    g = u ^ u ^ u;     vita prints 3x    ivl 1x
//!          u = mem[99];  g = u ^ u ^ u;     vita E4002 x3,    ivl silent, exit 0
//!                                                exit 1
//! ```
//!
//! ## Which functions are on the inline path at all
//!
//! Not obvious, and worth writing down because three attempts at a repro missed it.
//! `build_frame_set` frames a function when it is `automatic`, when its return type
//! is 2-STATE (`int`/`bit`/`byte`/`shortint`/`longint` — so `function int f` is
//! framed whether or not you write `automatic`), when its body is not straight-line,
//! or when it has an unpacked formal. What is left — a static `function [W:0]` /
//! `function logic [W:0]` with a `Blocking`-only body — is the inline path.
//!
//! ## The fix routes rather than memoises
//!
//! A body that assigns a local from a non-repeatable RHS and then names that local
//! twice or more is sent to the FRAME path, which binds the RHS to a slot once and
//! reads the slot thereafter. Frame ⊇ inline in capability since §4.5.198/199, so
//! this is a pure routing change with no new machinery.
//!
//! The cost model says the same thing independently: inline is Θ(refs^depth) and
//! frame is Θ(refs·depth) — measured, with the fitted base landing on 1.99 / 2.98 /
//! 3.95 for 2 / 3 / 4 references, and the arena growing strictly LINEARLY (+20 bytes
//! per chained statement per reference) while the evaluator's visit count explodes.
//! At 4 references and depth 8 that is 774 bytes of arena and 305,831 node visits per
//! call. So the routing removes an exponential as a side effect, but the reason it
//! exists is the four rows above.
//!
//! ⚠️ `expr_is_repeatable` FAILS CLOSED, and the one arm worth naming is the array
//! element read: `a[i]` is deliberately NOT repeatable, because an out-of-range read
//! REPORTS and vita files that report once per evaluation.
//!
//! ⚠️ NOT closed here: a body that ALSO reads a module net or calls another function.
//! Routing needs `body_reads_only_locals`, because a framed call contributes only its
//! ARGUMENTS to an implicit `always_comb` sensitivity list — routing such a body would
//! drop a read from that list, which is a different silent-wrong. The predicate is
//! shared with two other routing clauses, so widening it is not a local change. Those
//! shapes keep today's behaviour and are recorded rather than traded.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ifd_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// `codegen.frame_bodies` from `run.json` — which path the function actually took.
fn frame_bodies(src: &str) -> u64 {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ifd_ob_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let _ = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["t.sv", "--obs-dir", "ob"])
        .current_dir(&d)
        .output()
        .expect("run vita");
    let j = std::fs::read_to_string(d.join("ob/run.json")).expect("run.json");
    let _ = std::fs::remove_dir_all(&d);
    // No JSON dep in this crate's tests; the field is a bare integer.
    let k = j.find("\"frame_bodies\"").expect("frame_bodies present");
    j[k..]
        .split(':')
        .nth(1)
        .unwrap()
        .trim_start()
        .split(|c: char| !c.is_ascii_digit())
        .find(|s| !s.is_empty())
        .unwrap()
        .parse()
        .unwrap()
}

/// A static, 4-state-return, straight-line function: the inline path's exact shape.
fn design(decl: &str, body: &str, call: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule t;\n{decl}  \
         function [31:0] g(input [31:0] a);\n    reg [31:0] u;\n    begin {body} end\n  \
         endfunction\n  initial $display(\"R=%0d\", {call});\nendmodule\n"
    )
}

// ── the value corruptions ─────────────────────────────────────────────────

/// ⭐ `$random` drew TWICE. iverilog draws once and xors the value with itself, so
/// the answer is 0 — a number this test can assert rather than a self-comparison.
#[test]
fn a_duplicated_random_draws_once() {
    let src = design("", "u = $random; g = u ^ u;", "g(1)");
    let (out, code) = run(&src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=0"), "one draw, xored with itself:\n{out}");
    assert_eq!(
        frame_bodies(&src),
        1,
        "must have been routed to the frame path"
    );
}

/// The `$urandom` twin — a different RNG cell, same shape.
#[test]
fn a_duplicated_urandom_draws_once() {
    let (out, code) = run(&design("", "u = $urandom; g = u ^ u;", "g(1)"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=0"), "{out}");
}

/// ⚠️ THE CONTROL. A repeatable RHS must STILL be inlined — the routing has to be
/// reachable only from a non-repeatable one, or it is a blanket capability change
/// wearing a correctness argument.
#[test]
fn a_repeatable_rhs_stays_on_the_inline_path() {
    let src = design("", "u = a + 1; g = u ^ u ^ u;", "g(5)");
    let (out, code) = run(&src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=6"), "6 ^ 6 ^ 6 == 6:\n{out}");
    assert_eq!(frame_bodies(&src), 0, "a pure RHS must not be routed");
}

/// ⚠️ ONE reference is exactly what the inline fold handles correctly, so a single
/// use of an impure local must not route either. `$random`'s first draw is
/// iverilog's `303379748`; asserting the value proves the draw happened once, not
/// that it was skipped.
#[test]
fn a_single_reference_to_an_impure_local_stays_inline() {
    let src = design("", "u = $random; g = u;", "g(1)");
    let (out, code) = run(&src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("R=303379748"),
        "the first draw of the stream:\n{out}"
    );
    assert_eq!(frame_bodies(&src), 0, "one reference needs no routing");
}

// ── what counts as non-repeatable ─────────────────────────────────────────

/// An INDEXED read. It has no side effect on its value, but an out-of-range one
/// reports — and vita files that report per evaluation. This is the arm a purely
/// value-based repeatability test would have got wrong.
///
/// The index is a FORMAL, so the body still reads only locals and the routing gate is
/// not the thing under test here.
#[test]
fn a_duplicated_indexed_read_is_routed() {
    let src = "`timescale 1ns/1ns\nmodule t;\n           function [31:0] g(input [31:0] a, input [31:0] i);\n    reg [31:0] u;\n             begin u = a[i]; g = u ^ u ^ u; end\n  endfunction\n           initial $display(\"R=%0d\", g(6, 1));\nendmodule\n";
    assert_eq!(frame_bodies(src), 1, "an indexed read is not repeatable");
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("R=1"),
        "bit 1 of 6 is 1, xored three times:\n{out}"
    );
}

/// A duplicated `$time` is repeatable in principle (constant within an activation)
/// but the predicate fails closed on every `SysCall`, so it routes. Asserted so the
/// conservatism is deliberate and visible rather than accidental.
#[test]
fn a_duplicated_system_function_is_routed_even_when_it_is_constant() {
    let src = design("", "u = $time; g = u ^ u;", "g(1)");
    assert_eq!(frame_bodies(&src), 1, "fail-closed on SysCall:\n");
    let (out, code) = run(&src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=0"), "{out}");
}

/// The reference count is per-STATEMENT-CHAIN, not per-statement: a local defined
/// once and read once in each of two later statements is still two references.
#[test]
fn two_references_across_two_statements_are_still_two() {
    let src = design("", "u = $random; g = u; g = g ^ u;", "g(1)");
    assert_eq!(
        frame_bodies(&src),
        1,
        "the reads are counted across the body"
    );
    let (out, code) = run(&src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=0"), "one draw, so g ^ g == 0:\n{out}");
}

/// Redefining the local from a REPEATABLE RHS clears it: the references after that
/// point are references to something safe to duplicate.
#[test]
fn a_redefinition_from_a_repeatable_rhs_clears_the_local() {
    let src = design("", "u = $random; u = a + 1; g = u ^ u ^ u;", "g(5)");
    assert_eq!(frame_bodies(&src), 0, "the impure value is dead by then");
    let (out, code) = run(&src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=6"), "{out}");
}

// ── the exponential, as a bounded regression guard ────────────────────────

/// ⚠️ A TIME-FREE guard for the Θ(refs^depth) blow-up. Asserting wall-clock would be
/// flaky; asserting the ROUTE is exact. A body with three references per level and
/// six chained levels is 3^6 = 729 evaluations of the leaf on the inline path and 18
/// on the frame path — but only if the leaf is non-repeatable, which is what makes
/// this cell reachable at all. The pure twin below is the one that stays exponential
/// and is left alone deliberately: it is only wasted work, not a wrong answer, and
/// the corpus census found 25 of 27 inline functions have no body local at all.
#[test]
fn a_deep_impure_chain_is_routed_and_a_deep_pure_chain_is_not() {
    // ⚠️ The chain runs through BODY LOCALS, not through the function name. Reading `g`
    // back is a frame-path-only capability — `g = g ^ g` on the inline path is
    // `E3010 undeclared net/variable`, since the inline fold has no return slot to read
    // — so a chain written that way would compare a framed design against a design that
    // does not elaborate at all.
    let chain = |seed: &str| {
        format!(
            "`timescale 1ns/1ns\nmodule t;\n               function [31:0] g(input [31:0] a);\n                 reg [31:0] u, v, w, x, y, z;\n    begin\n                   u = {seed}; v = u ^ u; w = v ^ v; x = w ^ w; y = x ^ x; z = y ^ y;              g = z ^ z;\n    end\n  endfunction\n               initial $display(\"R=%0d\", g(1));\nendmodule\n"
        )
    };
    assert_eq!(
        frame_bodies(&chain("$random")),
        1,
        "six levels, two refs each"
    );

    assert_eq!(
        frame_bodies(&chain("a + 1")),
        0,
        "a pure chain keeps today's behaviour — wasted work, not a wrong answer"
    );
    let (out, code) = run(&chain("$random"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=0"), "every level xors with itself:\n{out}");
}

// ── the residue, pinned so it is visible when someone widens the gate ─────

/// ⚠️ STILL WRONG, deliberately and recorded. The body reads a MODULE net, so
/// `body_reads_only_locals` is false and routing is blocked: a framed call
/// contributes only its arguments to an implicit `always_comb` sensitivity list, so
/// routing this would drop `mem` from that list — trading one silent-wrong for
/// another, which the accuracy ladder forbids.
///
/// The cell asserts TODAY's behaviour so that whoever lifts the gate sees this test
/// go red and re-measures instead of discovering it later.
#[test]
fn a_body_reading_a_module_net_is_a_recorded_residue() {
    let src = design(
        "  reg [31:0] mem [0:3];\n",
        "u = mem[99]; g = u ^ u ^ u;",
        "g(1)",
    );
    assert_eq!(
        frame_bodies(&src),
        0,
        "blocked by body_reads_only_locals — if this is now 1, the residue is closed \
         and the assertions below should become the oracle's answer (iverilog: R=x, exit 0)"
    );
    let (out, code) = run(&src);
    assert_ne!(code, Some(0), "today: reports once per reference:\n{out}");
}

// ── what the adversarial review found, pinned ─────────────────────────────

/// ⚠️⚠️ CORRECT → LOUD, caught by the soundness lens. `body_needs_frame` answers `false`
/// for a `Blocking` regardless of an INTRA-ASSIGNMENT DELAY, and `fold_straight_line`
/// deliberately accepts one with a warning. `classify_frame_body` does not — it refuses
/// the body. So routing a delayed body handed a design the inline path RAN to a path
/// that rejects it: this was `r=42` plus a warning, and became E3009 exit 1.
///
/// The RHS here is `$signed(…)`, which `expr_is_repeatable` fails closed on even though
/// it is deterministic — so the inline path's answer was the ORACLE's, and the descent
/// was real rather than a wrong answer being replaced by a loud one.
#[test]
fn an_intra_assignment_delay_is_never_routed() {
    let src = "`timescale 1ns/1ns\nmodule t;\n  \
         function [31:0] fn(input [31:0] a);\n    reg [31:0] u;\n    \
         begin u = #3 $signed(a); fn = u + u; end\n  endfunction\n  \
         initial $display(\"R=%0d\", fn(21));\nendmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "must not become loud:\n{out}");
    assert!(out.contains("R=42"), "iverilog's answer:\n{out}");
    assert_eq!(frame_bodies(src), 0, "a delayed body stays inline");
}

/// ⚠️ BODY-WIDE, not per-statement. The delayed assignment need not be the
/// non-repeatable one: here `u = #3 a;` is repeatable and `v = $signed(a[7:0]);` is what
/// triggers the routing, so a guard that skipped only the delayed STATEMENT would still
/// route the body and still refuse the design.
#[test]
fn a_delay_anywhere_in_the_body_blocks_routing() {
    let src = "`timescale 1ns/1ns\nmodule t;\n  \
         function [31:0] fn(input [31:0] a);\n    reg [31:0] u, v;\n    \
         begin u = #3 a; v = $signed(a[7:0]); fn = u + v + v; end\n  endfunction\n  \
         initial $display(\"R=%0d\", fn(21));\nendmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=63"), "iverilog's answer:\n{out}");
    assert_eq!(frame_bodies(src), 0, "{out}");
}

/// ⚠️⚠️ THE TRANSITIVE HOP. Repeatability has to be judged AFTER substitution:
/// `fold_straight_line` binds each local to the arena node its RHS lowered to under the
/// CURRENT substitution, so `v = u` gives `v` the same impure node `u` had. Judging
/// `v = u` on its own text calls it repeatable and loses the chain — seven of a
/// twenty-six-cell census survived the first version of this predicate, through `+`,
/// `{}`, `&`, `[m:l]` and a bare copy.
#[test]
fn an_impure_value_carried_through_a_pure_hop_is_still_routed() {
    for (label, body) in [
        ("copy", "u = $random; v = u; g = v ^ v;"),
        ("add", "u = $random; v = u + 0; g = v ^ v;"),
        ("concat", "u = $random; v = {u}; g = v ^ v;"),
        ("two hops", "u = $random; v = u; w = v; g = w ^ w;"),
        (
            "mask then or",
            "u = $random; v = u & 32'hFFFF; w = v | 0; g = w ^ w;",
        ),
    ] {
        let src = format!(
            "`timescale 1ns/1ns\nmodule t;\n  \
             function [31:0] g(input [31:0] a);\n    reg [31:0] u, v, w;\n    \
             begin {body} end\n  endfunction\n  \
             initial $display(\"R=%0d\", g(1));\nendmodule\n"
        );
        let (out, code) = run(&src);
        assert_eq!(code, Some(0), "{label}:\n{out}");
        assert!(
            out.contains("R=0"),
            "{label}: one draw, xored with itself:\n{out}"
        );
    }
}

/// ⚠️ `Cast` was the ONE `ExprKind` that `expr_reads_only_locals` admits and
/// `walk_expr_refs` had no arm for, so `int'(u) ^ int'(u)` counted ZERO references to
/// `u` and the body was not routed. Both siblings in the same file already had the arm.
#[test]
fn references_inside_a_cast_are_counted() {
    let src = "`timescale 1ns/1ns\nmodule t;\n  \
         function [31:0] g(input [31:0] a);\n    reg [31:0] u;\n    \
         begin u = $random; g = int\'(u) ^ int\'(u); end\n  endfunction\n  \
         initial $display(\"R=%0d\", g(1));\nendmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=0"), "{out}");
    assert_eq!(frame_bodies(src), 1, "the cast's operand is a reference");
}
