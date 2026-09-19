//! §3 `pkg-callee-blocal` — the TRANSITIVE package callees of a scoped `pk::g()` call
//! are framed with the same predicate the step-6.5 barrier uses.
//!
//! ## The defect
//!
//! `inline_pkg_function` (`elaborate/src/inline_fn.rs`) lowers a scoped call in pass 7.
//! On its `frame_idx` MISS it calls `inject_pkg_callees` (`elaborate/src/package.rs`),
//! which puts every transitive same-package callee into `func_table` under its scoped
//! key `pk::h`. That happens AFTER step 6.5 (`lower_frame_funcs`,
//! `elaborate/src/instance.rs`), the barrier that classifies `func_table` with
//! `build_frame_set` and reserves a frame for every member — so `pk::h` was never
//! reserved. When the root's body was lowered, `h(m)` resolved (`resolve_rtn_key`) to
//! `pk::h`, found no `frame_idx` entry, and took the INLINE fold.
//!
//! The inline fold is a straight-line SSA substitution. It cannot carry a body-local
//! that is declared WITH an initializer (`int x = a*2;` → `VITA-E3010` undeclared
//! `top.$func$pk::g.x`), a static local read before it is written, control flow, a
//! loop, or an unpacked local (`VITA-E3009` "body is not reducible to an expression").
//! Every one of those was LOUD on the scoped spelling while the IMPORT twin
//! (`import pk::g;`) ran and printed the oracle value, because step (3.6) injects the
//! same callees BEFORE the barrier.
//!
//! ## The fix
//!
//! `elaborate/src/pkg_scoped_frames.rs`: the MISS branch classifies the just-injected
//! callees with `build_frame_set` itself, reserves every qualifying callee AND the root
//! before lowering any body (which is what makes a mutual recursion between them
//! resolve), lowers the callee bodies, mirrors the R22 §3.1 statement-executor marking
//! for them, then lowers the root. A callee the predicate does NOT qualify keeps the
//! inline fold it has always taken, so no route is widened.
//!
//! ## Oracles
//!
//! Every value pinned here was measured three-way against iverilog 13.0 (`-g2012` +
//! `vvp -n`) and verilator 5.052 (`--binary --timing`); both agree on every line.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkgcallee_{}_{n}", std::process::id()));
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

/// A clean run (exit 0) whose output contains every `want` line and none of `absent`.
fn lines(src: &str, want: &[&str], absent: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
    for a in absent {
        assert!(!o.contains(a), "did NOT expect {a:?} in:\n{o}");
    }
}

/// A run that must FAIL with the given diagnostic CODE (not its wording), plus the one
/// phrase of the message that carries the reason.
fn loud(src: &str, code_str: &str, want: &[&str]) {
    let (o, code) = run(src);
    assert_ne!(code, Some(0), "expected a loud refusal:\n{o}");
    assert!(o.contains(code_str), "expected {code_str:?} in:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
}

// ------------------------------------------------- cells this slice moves (were loud)

/// c4: the row's own cell — an `automatic` callee whose local is declared WITH an
/// initializer. Was `VITA-E3010 undeclared net/variable top.$func$pk::g.x`; both
/// oracles print 44.
#[test]
fn scoped_callee_decl_init_local() {
    lines(
        r#"package pk;
  function automatic int h(input int a); int x = a * 2; return x + 2; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  initial $display("V=%0d", pk::g(21));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=44"],
        &["VITA-E3010", "VITA-E3009"],
    );
}

/// c14: a STATIC callee whose local is read before it is written. It is NOT widened onto
/// the frame path — the frame lane would give this scope its own copy of a variable IEEE
/// §6.21 keeps for the whole design — so it keeps the pre-slice inline fold and that
/// fold's own `VITA-E3010`. The IMPORT twin of the same body (pinned below) keeps its
/// pre-slice `V=21 W=22`; that lane is the ROADMAP §2 row and is untouched here.
#[test]
fn scoped_callee_static_local_read_before_write() {
    loud(
        r#"package pk;
  function int h(input int a); int x; x = x + a; return x; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  initial $display("V=%0d W=%0d", pk::g(21), pk::g(1));
  initial begin #1 $finish; end
endmodule
"#,
        "VITA-E3010",
        &[],
    );
}

/// c15: a callee with `if`/`else`. Was `VITA-E3009 function h body is not reducible to
/// an expression (control flow)`; both oracles print 44.
#[test]
fn scoped_callee_control_flow() {
    lines(
        r#"package pk;
  function automatic int h(input int a); int x; if (a > 10) x = a * 2; else x = a; return x + 2; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  initial $display("V=%0d", pk::g(21));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=44"],
        &["VITA-E3009"],
    );
}

/// x1: MUTUAL recursion `g` ↔ `h`, both with a decl-init local. This is the cell that
/// makes the reserve-before-lower ordering load-bearing: `h`'s body calls `g` back, so
/// `g`'s own frame must exist under its scoped key before `h`'s body is lowered. Was
/// `VITA-E3009 ... (control flow)`; both oracles print 10.
#[test]
fn scoped_mutual_recursion_both_decl_init() {
    lines(
        r#"package pk;
  function automatic int h(input int a); int x = a - 1; if (x <= 0) return 0; return g(x); endfunction
  function automatic int g(input int m); int y = m; return y + h(y); endfunction
endpackage
module top;
  initial $display("V=%0d", pk::g(4));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=10"],
        &["VITA-E3009", "VITA-E3010"],
    );
}

/// x2: the framed callee still reads its own package's VARIABLE (`pv`) from inside the
/// frame body — framing must not cost the §26.3 package-scope binding. Was
/// `VITA-E3010`; both oracles print 47.
#[test]
fn scoped_callee_reads_a_package_variable() {
    lines(
        r#"package pk;
  int pv = 5;
  function automatic int h(input int a); int x = a * 2; return x + pv; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  initial $display("V=%0d", pk::g(21));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=47"],
        &["VITA-E3010"],
    );
}

/// x4: the scoped call sits inside a generate-for, so the frames are reserved under the
/// generate scope's prefix and the SECOND iteration reuses them. Was `VITA-E3010
/// undeclared net/variable top.gb[0].$func$pk::g.x`; both oracles print both lines.
#[test]
fn scoped_call_inside_generate_for() {
    lines(
        r#"package pk;
  function automatic int h(input int a); int x = a * 2; return x + 2; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  genvar gi; generate for (gi = 0; gi < 2; gi++) begin : gb
    initial $display("V%0d=%0d", gi, pk::g(21 + gi)); end endgenerate
  initial begin #1 $finish; end
endmodule
"#,
        &["V0=44", "V1=46"],
        &["VITA-E3010"],
    );
}

/// x6: the same scoped call from TWO child instances and the parent. Each instance
/// elaborates with its own routine tables, so the callee is injected and framed three
/// times. Was three `VITA-E3010`s; both oracles print all three lines.
#[test]
fn scoped_call_from_two_instances_and_top() {
    lines(
        r#"package pk;
  function automatic int h(input int a); int x = a * 2; return x + 2; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module sub #(parameter int K = 1);
  initial $display("S%0d=%0d", K, pk::g(K));
endmodule
module top;
  sub #(1) s1(); sub #(2) s2();
  initial $display("V=%0d", pk::g(21));
  initial begin #1 $finish; end
endmodule
"#,
        &["S1=4", "S2=6", "V=44"],
        &["VITA-E3010"],
    );
}

/// x8: a callee with an UNPACKED local (`int arr[2]`), which the inline fold has no
/// binding for. Was `VITA-E3009 ... (control flow)`; both oracles print 44.
#[test]
fn scoped_callee_unpacked_local() {
    lines(
        r#"package pk;
  function automatic int h(input int a); int arr[2]; arr[0] = a; arr[1] = a * 2; return arr[0] + arr[1] + 2; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  initial $display("V=%0d", pk::g(14));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=44"],
        &["VITA-E3009"],
    );
}

/// x9: a callee with a `for` loop. Was `VITA-E3009 ... (control flow)`; both oracles
/// print 44.
#[test]
fn scoped_callee_for_loop() {
    lines(
        r#"package pk;
  function automatic int h(input int a); int x = 0; for (int i = 0; i < a; i++) x += 2; return x + 2; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  initial $display("V=%0d", pk::g(21));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=44"],
        &["VITA-E3009"],
    );
}

// ------------------------------------------------ controls: correct BEFORE and after

/// c1: the callee's local is declared and THEN assigned, which the inline fold already
/// handled. The value must not move now that the same callee takes a frame instead.
#[test]
fn control_callee_declared_then_assigned() {
    lines(
        r#"package pk;
  function automatic int h(input int a); int x; x = a * 2; return x + 2; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  initial $display("V=%0d", pk::g(21));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=44"],
        &["VITA-E3010", "VITA-E3009"],
    );
}

/// c8: TWO-level transitive `g` → `h` → `k`. The walk that injects the callees is
/// transitive, so the classification that frames them must be too.
#[test]
fn control_two_level_transitive_callee() {
    lines(
        r#"package pk;
  function automatic int k(input int a); int x; x = a * 2; return x + 2; endfunction
  function int h(input int a); return k(a); endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  initial $display("V=%0d", pk::g(21));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=44"],
        &["VITA-E3010", "VITA-E3009"],
    );
}

/// c12: TWO scoped roots sharing one callee. The second root's MISS branch must find
/// `pk::h` already in `frame_idx` and reserve nothing twice.
#[test]
fn control_two_roots_share_one_callee() {
    lines(
        r#"package pk;
  function automatic int h(input int a); int x; x = a * 2; return x + 2; endfunction
  function int g(input int m); return h(m); endfunction
  function int g2(input int m); return h(m) + 1; endfunction
endpackage
module top;
  initial $display("V=%0d W=%0d", pk::g(21), pk::g2(21));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=44 W=45"],
        &["VITA-E3010", "VITA-E3009"],
    );
}

/// c13: an IMPORT of one routine beside a SCOPED call of another, both reaching the
/// same callee. The import framed `pk::h` at the step-6.5 barrier, so the scoped root's
/// MISS branch must leave that frame alone.
#[test]
fn control_import_and_scoped_call_share_a_callee() {
    lines(
        r#"package pk;
  function automatic int h(input int a); int x; x = a * 2; return x + 2; endfunction
  function int g(input int m); return h(m); endfunction
  function int g2(input int m); return h(m) + 1; endfunction
endpackage
module top;
  import pk::g2;
  initial $display("V=%0d W=%0d", pk::g(21), g2(21));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=44 W=45"],
        &["VITA-E3010", "VITA-E3009"],
    );
}

/// i4: the IMPORT twin of `scoped_callee_decl_init_local` — correct before this slice
/// (step (3.6) injects ahead of the barrier) and the anchor the scoped cell is now
/// equal to. It must stay exactly where it was.
#[test]
fn control_import_twin_decl_init_local() {
    lines(
        r#"package pk;
  function automatic int h(input int a); int x = a * 2; return x + 2; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  import pk::g;
  initial $display("V=%0d", g(21));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=44"],
        &["VITA-E3010", "VITA-E3009"],
    );
}

/// i14: the IMPORT twin of `scoped_callee_static_local_read_before_write`. Same anchor
/// role — the scoped spelling now prints what this one always printed.
#[test]
fn control_import_twin_static_local_read_before_write() {
    lines(
        r#"package pk;
  function int h(input int a); int x; x = x + a; return x; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  import pk::g;
  initial $display("V=%0d W=%0d", g(21), g(1));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=21 W=22"],
        &["VITA-E3010", "VITA-E3009"],
    );
}

// ------------------------------- the one shape this arm REFUSES rather than frames

/// s2: the same static, read-before-write callee reached from THREE scopes (two child
/// instances and the parent). A frame is reserved per scope, but IEEE §6.21 keeps ONE
/// such variable for the whole design — both oracles print `S1=1 S2=3 V=13`, where the
/// carried-over value crosses the instance boundary. The import twin and the
/// single-root twin of this body print `S1=1 S2=2 V=10` on every build (the ROADMAP §2
/// row "A PACKAGE task's static local is ONE variable"), so framing the second scope
/// would route this cell onto that known silent. It is refused instead.
#[test]
fn static_callee_reached_from_two_scopes_keeps_the_inline_loud() {
    loud(
        r#"package pk;
  function int h(input int a); int x; x = x + a; return x; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module sub #(parameter int K = 1);
  initial begin #(K) $display("S%0d=%0d", K, pk::g(K)); end
endmodule
module top;
  sub #(1) s1(); sub #(2) s2();
  initial begin #3 $display("V=%0d", pk::g(10)); end
  initial begin #5 $finish; end
endmodule
"#,
        "VITA-E3010",
        &[],
    );
}

/// s4: the control twin of the cell above — same two instances plus the parent, same
/// STATIC callee, but its local is assigned before it is read, so every scope's copy is
/// byte-identical and the guard must not fire. Three-tool measured `S1=4 S2=6 V=44`.
#[test]
fn control_static_callee_assigned_before_read_from_two_scopes() {
    lines(
        r#"package pk;
  function int h(input int a); int x; x = a * 2; return x + 2; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module sub #(parameter int K = 1);
  initial begin #(K) $display("S%0d=%0d", K, pk::g(K)); end
endmodule
module top;
  sub #(1) s1(); sub #(2) s2();
  initial begin #3 $display("V=%0d", pk::g(21)); end
  initial begin #5 $finish; end
endmodule
"#,
        &["S1=4", "S2=6", "V=44"],
        &["VITA-E3010", "VITA-E3009"],
    );
}

// ---------------------- round-1 lens findings: the callee walkers see declarations

/// A call that appears ONLY in a block-local DECLARATION INITIALIZER was invisible to
/// `collect_callee_stmt`, so `pk::k` was never injected and the frame body's bare `k`
/// resolved in the CALLING module — `V=1002` from the module's own `k`, where both
/// oracles print 44. The walkers now read declaration initializers too.
#[test]
fn decl_init_call_binds_to_the_package_sibling() {
    lines(
        r#"package pk;
  function automatic int k(input int a); return a * 2; endfunction
  function automatic int h(input int a); int y = k(a) + 2; return y; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  function automatic int k(input int a); return 1000; endfunction
  initial $display("V=%0d", pk::g(21));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=44"],
        &["V=1002", "VITA-E3010", "VITA-E3009"],
    );
}

/// The same leak through an `import` of ANOTHER package's `k`: it crosses packages, so
/// the fix cannot be "prefer the same package's name at the use site".
#[test]
fn decl_init_call_is_not_taken_by_an_imported_name() {
    lines(
        r#"package pq;
  function automatic int k(input int a); return 1000; endfunction
endpackage
package pk;
  function automatic int k(input int a); return a * 2; endfunction
  function automatic int h(input int a); int y = k(a) + 2; return y; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  import pq::k;
  initial $display("V=%0d", pk::g(21));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=44"],
        &["V=1002"],
    );
}

/// A four-level chain whose every edge is a declaration initializer. With the walk
/// blind to declarations only the first edge was injected and the rest were "call to
/// undeclared function". Oracles 46.
#[test]
fn decl_init_chain_injects_every_level() {
    lines(
        r#"package pk;
  function automatic int j(input int a); int z = a + 1; return z; endfunction
  function automatic int k(input int a); int y = j(a) * 2; return y; endfunction
  function automatic int h(input int a); int x = k(a) + 2; return x; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  initial $display("V=%0d", pk::g(21));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=46"],
        &["VITA-E3010", "VITA-E3009"],
    );
}

/// The ROOT's own declaration initializer: `inject_pkg_callees` starts its walk from
/// the root, so the root's `body_decls` had the same blind spot. Oracles `R1=220`; was
/// `R1=10210` from the module's own `h`.
#[test]
fn root_decl_init_call_binds_to_the_package_sibling() {
    lines(
        r#"package pk;
  function automatic int h(input int a); return a + 1; endfunction
  function automatic int g(input int m);
    int x = h(m);
    return x * 10;
  endfunction
endpackage
module top;
  function automatic int h(input int a); return a + 1000; endfunction
  initial begin #1 $display("R1=%0d", pk::g(21)); end
  initial begin #5 $finish; end
endmodule
"#,
        &["R1=220"],
        &["R1=10210"],
    );
}

/// A RECURSIVE callee: its own name appears as a CALL, which the definite-assignment
/// reference walker cannot tell from a read. Without the precise presence check the
/// return-variable refusal fired on every recursive package function. Oracles 10.
#[test]
fn control_recursive_callee_is_not_a_return_variable_read() {
    lines(
        r#"package pk;
  function automatic int h(input int a); int x = a - 1; if (x <= 0) return 0; return x + h(x); endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  initial $display("V=%0d", pk::g(5));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=10"],
        &["VITA-E3009", "VITA-E3010"],
    );
}

// ---------------------- round-1 lens findings: the guard covers both lanes and the root

/// A design with BOTH lanes: `import pk::g;` in one module, `pk::g()` in another. The
/// scoped call is not widened (its callee is the static persistent one), so it keeps the
/// pre-slice `VITA-E3010`; the import lane is left exactly as it was. Both oracles print
/// `S=1 V=3`, and vita printed `S=1 V=2` when the scoped call was framed.
#[test]
fn static_callee_reached_through_both_lanes_keeps_the_inline_loud() {
    loud(
        r#"package pk;
  function int h(input int a); int x; x = x + a; return x; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module sub;
  import pk::g;
  initial begin #1 $display("S=%0d", g(1)); end
endmodule
module top;
  sub s1();
  initial begin #2 $display("V=%0d", pk::g(2)); end
  initial begin #4 $finish; end
endmodule
"#,
        "VITA-E3010",
        &[],
    );
}

/// The scoped ROOT's own static local is a per-scope frame net exactly like a callee's;
/// the guard covers it too (`S=4 V=6` where both oracles print `S=4 V=10`).
#[test]
fn static_scoped_root_keeps_the_inline_loud() {
    loud(
        r#"package pk;
  function automatic int h(input int a); int x = a * 2; return x + 2; endfunction
  function int g(input int m); int s; s = s + h(m); return s; endfunction
endpackage
module sub;
  initial begin #1 $display("S=%0d", pk::g(1)); end
endmodule
module top;
  sub s1();
  initial begin #2 $display("V=%0d", pk::g(2)); end
  initial begin #4 $finish; end
endmodule
"#,
        "VITA-E3010",
        &[],
    );
}

/// A body that reads its own RETURN VARIABLE before assigning it is answered 0 by the
/// frame lowering with ONE call in ONE scope — both oracles print the accumulated value
/// — so it is refused on sight rather than counted like a §6.21 per-scope local.
#[test]
fn return_variable_read_before_write_keeps_the_inline_loud() {
    loud(
        r#"package pk;
  function int h(input int a); h = h + a; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module sub #(parameter int K = 1);
  initial begin #(K) $display("S%0d=%0d", K, pk::g(K)); end
endmodule
module top;
  sub #(1) u1(); sub #(2) u2();
  initial #10 $finish;
endmodule
"#,
        "VITA-E3010",
        &[],
    );
}

/// The same body in a SINGLE scope, which the scope count would have let through:
/// measured `V=0 W=1` against both oracles' `V=21 W=1`.
#[test]
fn return_variable_read_before_write_keeps_the_inline_loud_in_one_scope() {
    loud(
        r#"package pk;
  function int h(input int a); h = h + a; endfunction
  function int g(input int m); return h(m); endfunction
endpackage
module top;
  initial begin #1 $display("V=%0d", pk::g(21)); #1 $display("W=%0d", pk::g(1)); end
  initial #10 $finish;
endmodule
"#,
        "VITA-E3010",
        &[],
    );
}

/// A refused root must not pollute the next root's candidate list. `pk::g2` has no
/// callee and nothing to do with `pk::h`, but `late` was selected by key PREFIX over a
/// `func_table` that accumulates, so the refused root's callees were attributed to it
/// and its innocent call site carried a second copy of the other root's message.
/// Exactly ONE diagnostic, and it names the refused call.
#[test]
fn a_refused_root_does_not_refuse_the_next_one() {
    let (o, code) = run(r#"package pk;
  function int h(input int a); int x; x = x + a; return x; endfunction
  function int g1(input int m); return h(m); endfunction
  function int g2(input int m); int y; y = m + 1; return y; endfunction
endpackage
module mb;
  initial begin #4 $display("B=%0d", pk::g1(2)); end
  initial begin #5 $display("C=%0d", pk::g2(3)); end
endmodule
module top;
  mb ub();
  initial begin #1 $display("A=%0d", pk::g1(1)); end
  initial #10 $finish;
endmodule
"#);
    assert_ne!(code, Some(0), "expected a loud refusal:\n{o}");
    // One diagnostic per refused CALL SITE of `pk::g1` (`top` and `top.ub`), and none
    // naming `pk::g2`, which has no callee and nothing to do with `pk::h`.
    assert_eq!(
        o.matches("VITA-E3010").count(),
        2,
        "expected the pre-slice E3010 per call site:\n{o}"
    );
    assert!(
        !o.contains("`pk::g2`"),
        "no diagnostic may name the unrelated root `pk::g2`:\n{o}"
    );
}

/// The control the cell above is measured against: the same design with only ONE
/// `pk::g1` call site carries exactly ONE diagnostic, and it still does not name
/// `pk::g2`. (Both oracles print `A=1 C=4`; the `pk::g1` half is the §6.21 refusal.)
#[test]
fn control_unrelated_scoped_root_is_refused_once() {
    let (o, code) = run(r#"package pk;
  function int h(input int a); int x; x = x + a; return x; endfunction
  function int g1(input int m); return h(m); endfunction
  function int g2(input int m); int y; y = m + 1; return y; endfunction
endpackage
module mb;
  initial begin #5 $display("C=%0d", pk::g2(3)); end
endmodule
module top;
  mb ub();
  initial begin #1 $display("A=%0d", pk::g1(1)); end
  initial #10 $finish;
endmodule
"#);
    assert_ne!(code, Some(0), "expected a loud refusal:\n{o}");
    assert_eq!(
        o.matches("VITA-E3010").count(),
        1,
        "expected the pre-slice E3010, once:\n{o}"
    );
    assert!(
        !o.contains("`pk::g2`"),
        "no diagnostic may name the unrelated root `pk::g2`:\n{o}"
    );
}

// ------------------- round-2 lens findings: the refusal is the SCOPED lane's alone

/// The IMPORT lane of the same hazard class must be untouched. `import pk::h;` in two
/// instances of one module frames its own copy per instance and prints `S1=3 S2=5` —
/// which is also what both oracles print, so a guard on the shared step-6.5 barrier was
/// a value-to-loud regression. This pins that the barrier is not guarded.
#[test]
fn control_import_lane_static_local_is_untouched() {
    lines(
        r#"package pk;
  function int h(input int a); int x; if (x == 0) x = 1; return a * 2 + x; endfunction
endpackage
module sub #(parameter int K = 1);
  import pk::h;
  initial begin #(K) $display("S%0d=%0d", K, h(K)); end
endmodule
module top;
  sub #(1) s1(); sub #(2) s2();
  initial begin #5 $finish; end
endmodule
"#,
        &["S1=3", "S2=5"],
        &["VITA-E3009", "VITA-E3010"],
    );
}

/// The MODULE lane of the same body: two instances, one static local read before it is
/// written, no package at all. Nothing in this slice reaches it. All three tools print
/// `S1=1 S2=2`.
#[test]
fn control_module_lane_static_local_is_untouched() {
    lines(
        r#"module sub #(parameter int K = 1);
  function int h(input int a); int x; x = x + a; return x; endfunction
  initial begin #(K) $display("S%0d=%0d", K, h(K)); end
endmodule
module top;
  sub #(1) s1(); sub #(2) s2();
  initial begin #5 $finish; end
endmodule
"#,
        &["S1=1", "S2=2"],
        &["VITA-E3009", "VITA-E3010"],
    );
}

/// A scoped ROOT that frames NO late callee is lowered exactly as it was before this
/// slice, so it is not asked at all: `pk::g` whose whole body is the persistent static
/// prints `V=21 W=22`, its pre-slice value and both oracles'. This is the measured limit
/// of the refusal above (census `r14`).
#[test]
fn control_scoped_root_with_no_late_callee_is_untouched() {
    lines(
        r#"package pk;
  function int g(input int m); int x; x = x + m; return x; endfunction
endpackage
module top;
  initial $display("V=%0d W=%0d", pk::g(21), pk::g(1));
  initial begin #1 $finish; end
endmodule
"#,
        &["V=21 W=22"],
        &["VITA-E3009", "VITA-E3010"],
    );
}

/// A scoped ROOT that is itself the static persistent routine and DOES have a
/// same-package callee. Excluding the root excludes the whole call, so the root keeps
/// the one frame it always had and its callee keeps the inline fold: `V=23 W=24`, the
/// pre-slice value and both oracles'. (`ret_two_state` puts every `int`-returning
/// sibling in the frame set, so "the root has no callee" is almost never true — the
/// exclusion has to be decided from the root, not from the candidate list.)
#[test]
fn control_excluded_root_with_a_callee_keeps_its_value() {
    lines(
        r#"package pk;
  function automatic int h(input int a); int y; y = a * 2; return y; endfunction
  function int g(input int m); int x; x = x + m; return x + h(1); endfunction
endpackage
module top;
  initial begin #1 $display("V=%0d", pk::g(21)); #1 $display("W=%0d", pk::g(1)); end
  initial begin #4 $finish; end
endmodule
"#,
        &["V=23", "W=24"],
        &["VITA-E3009", "VITA-E3010"],
    );
}

/// The same exclusion where the root assigns its RETURN NAME (`g = x + k(0) - 1;`) and
/// the callee is a plain static sibling. Both oracles and the pre-slice build print
/// `V=21 W=22`.
#[test]
fn control_excluded_root_assigning_its_return_name_keeps_its_value() {
    lines(
        r#"package pk;
  function int k(input int a); return a + 1; endfunction
  function int g(input int m);
    int x;
    x = x + m;
    g = x + k(0) - 1;
  endfunction
endpackage
module top;
  int v, w;
  initial begin
    v = pk::g(21);
    w = pk::g(1);
    $display("V=%0d W=%0d", v, w);
    #1 $finish;
  end
endmodule
"#,
        &["V=21 W=22"],
        &["VITA-E3009", "VITA-E3010"],
    );
}
