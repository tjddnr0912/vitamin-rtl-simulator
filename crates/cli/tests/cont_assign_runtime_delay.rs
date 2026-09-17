//! A RUNTIME (variable) structural delay is evaluated at the scheduling point —
//! `assign #(dv) y = a;`, `wire #(dv) w = a;`, `buf #(dv) g(y,a);` where `dv` is
//! not an elaboration constant (IEEE 1364-2005 §6.1.3 / §7.14).
//!
//! MECHANISM. `elaborate::fold_ca_delay` answered `uniform = None` for a delay
//! that does not const-fold, and every caller consumes `None` as a SILENT
//! default: no delay at all. So `int dv = 5; assign #(dv) y = a;` propagated in
//! the same time step, at exit 0. The same `None` also killed the
//! `(rise, fall, turnoff)` sidecar for a MIXED spec (`#(2, dv)`), because that
//! triple requires EVERY value to fold — so the fall silently collapsed onto
//! the rise.
//!
//! FIX. When any value of the delay list is not an elaboration constant, the
//! delay VALUE expressions are lowered into the same expression arena the
//! assign's rhs uses and recorded in the `ca_delay_exprs` sidecar
//! (cont-assign index → `(rise_eid, fall_eid, toff_eid, time_mult,
//! prec_mult)`); `ContAssign.delay` becomes `Some(0)`, which is the routing
//! flag that puts the assign on the delayed lane. The engine evaluates the ids
//! in `Scheduler::schedule_delayed_cas` — at the moment the rhs is found to
//! have changed, through the same reader the rhs was read with — converts them
//! with the shared `eval::delay_ticks_of` and feeds `transition_delay`. The
//! delay expression is NOT part of the assign's sensitivity: writing the delay
//! variable alone re-schedules nothing. The multipliers ride the sidecar
//! because a continuous assign has no process; see
//! `the_multiplier_is_the_declaring_modules` below, which fails if the engine
//! reads the ambient `cur_time_mult` instead. A fully constant delay takes the
//! pre-slice path verbatim — `constant_delay_control_is_byte_identical` asserts
//! the whole VCD.
//!
//! CONSEQUENCE FOR EVERY GATE THAT ENUMERATES A DESIGN'S EXPRESSIONS: a delayed
//! continuous assign now has some, where it had a folded tick count. Both walks
//! in `native::frames::frames_admitted` take them — pinned by
//! `a_call_in_a_runtime_delay_falls_back_instead_of_panicking` and its control,
//! without which `assign #(dly(dv)) y = a;` aborted the DEFAULT backend.
//!
//! ORACLES. iverilog 13.0 (`iverilog -g2012 -o x.vvp x.sv && vvp -n x.vvp`) and
//! verilator 5.052 (`verilator --binary --timing -Wno-fatal x.sv`). Every
//! expected line below is the raw output of BOTH unless its test says
//! otherwise. PRE values are from the binary built at the parent commit.
//!
//! WHAT THE ORACLES DO NOT ARBITRATE, measured, not assumed:
//!
//!   * THE INITIAL WINDOW. Before a delayed assign's first write LANDS, vita
//!     drives the target `x` (`delayed_owes_initial_x`). For a CONSTANT delay
//!     iverilog does the same. For a VARIABLE delay it does not — it drives the
//!     t0 rhs value immediately, because the delay variable is still 0 when it
//!     first evaluates the assign. `initial_window_is_not_arbitrable` is that
//!     contradiction in ONE design with ONE delay value: iverilog prints
//!     `yv=1 yc=x` for `#(dv)` and `#3` with `dv = 3`. verilator is 2-state and
//!     prints `0` for both, so it cannot separate them either. vita answers the
//!     two spellings identically, which is the internal-equivalence oracle this
//!     area has. Every probe below is therefore taken AFTER the first landed
//!     write, except where a test says it is pinning the window.
//!   * A ZERO runtime delay inherits the pre-existing `#0` lag (ROADMAP §2): the
//!     write lands after the Postponed region of the same time value, so only a
//!     same-time-step `#0` observer can see it.
//!     `a_zero_runtime_delay_inherits_the_zero_tick_lag` pins both halves — the
//!     lag, and the fact that the constant `#0` twin has it too.
//!   * A RESOLVED net — two or more whole-net continuous drivers, which the
//!     engine folds by 4-state wire resolution — cannot be on the delayed lane
//!     at all (its constant twin is E3001 today), so the lane is handed back
//!     there and the delay stays dropped. `a_resolved_net_keeps_its_pre_slice_delay`
//!     pins that, and `disjoint_part_select_drivers_keep_the_runtime_delay`
//!     pins the boundary.
//!   * `1ns/100ps` DISQUALIFIES BOTH ORACLES for a 2-value delay spec:
//!     verilator drops `#(2, 5)` — a fully CONSTANT control that it gets right
//!     under `1ns/1ns` — and iverilog drops the mixed `#(2, dv)` while getting
//!     the same source right under `1ns/1ns`. Recorded in
//!     `nine_runtime_delay_forms`, which is the `1ns/100ps` grounding design.
//!
//! VCD/probe bytes move ONLY for designs that have a non-constant structural
//! delay; those were silent-wrong. A constant-delay design's VCD is byte-
//! identical PRE vs POST (measured on the design in
//! `constant_delay_control_is_byte_identical`: `cmp` of the two dumps is
//! clean), which is why that test asserts the dump's literal text rather than a
//! digest — PRE is not available in CI.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn dir_for(tag: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cadly_{tag}_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_in(d: &std::path::Path, src: &str) -> (String, Option<i32>) {
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

fn run(src: &str) -> (String, Option<i32>) {
    let d = dir_for("r");
    run_in(&d, src)
}

/// stdout AND stderr, for the two tests whose subject is a DIAGNOSTIC: the backend
/// fallback warning is a diagnostic, so it goes to the diagnostic stream. Asserting
/// its presence — or its absence — against `run`'s stdout would be vacuous either way.
fn run_with_diags(src: &str) -> (String, Option<i32>) {
    let d = dir_for("rd");
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.code(),
    )
}

/// Assert every `want` line appears verbatim, reporting the whole output once.
fn want_lines(out: &str, code: Option<i32>, want: &[&str], why: &str) {
    assert_eq!(code, Some(0), "{why}: nonzero exit; got:\n{out}");
    for w in want {
        assert!(out.contains(w), "{why}: missing `{w}`; got:\n{out}");
    }
}

// ─────────────────────────────────────────────────────────────────────────
// The three spellings share one funnel
// ─────────────────────────────────────────────────────────────────────────

const TWIN: &str = "`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0; int dv = 3; wire #(dv) w = a; wire yb, yg;\n\
\x20 assign #(dv) yb = a;\n\
\x20 buf #(dv) g(yg, a);\n\
\x20 initial begin\n\
\x20   #1 a = 1;\n\
\x20   #4 $display(\"T5 w=%b yb=%b yg=%b\", w, yb, yg);\n\
\x20   #1 $display(\"T6 w=%b yb=%b yg=%b\", w, yb, yg);\n\
\x20   a = 0;\n\
\x20   #2 $display(\"T8 w=%b yb=%b yg=%b\", w, yb, yg);\n\
\x20   #3 $display(\"T11 w=%b yb=%b yg=%b\", w, yb, yg);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n";

#[test]
fn every_structural_spelling_takes_the_runtime_lane() {
    // `a` rises at 1 → all three rise at 4; `a` falls at 6 → all three fall at 9.
    // A net-declaration delay, a spelled `assign` and a gate primitive are one
    // construct (IEEE §6.1.3; the parser desugars the primitive into a
    // ContinuousAssign), so they must answer identically — which is why the fold
    // and the runtime lowering share `fold_ca_delay_rt`.
    //
    // BOTH ORACLES, all three lines:
    //   T6  w=1 yb=1 yg=1
    //   T8  w=1 yb=1 yg=1
    //   T11 w=0 yb=0 yg=0
    // PRE printed `T8 w=0 yb=0 yg=0` — the fall landed at t=6 with no delay.
    // (T5 is at t=5, one unit after the rise at 4; both oracles print 1.)
    let (out, code) = run(TWIN);
    want_lines(
        &out,
        code,
        &[
            "T5 w=1 yb=1 yg=1",
            "T6 w=1 yb=1 yg=1",
            "T8 w=1 yb=1 yg=1",
            "T11 w=0 yb=0 yg=0",
        ],
        "wire / assign / gate primitive share the runtime lane",
    );
}

#[test]
fn staged_vcmp_velab_vrun_carries_the_runtime_delay() {
    // STAGED-DROP: without the `ca_delay_exprs` trailer field a staged run fires
    // the assign with NO delay while the one-shot run delays it — the hazard the
    // format_version 32 bump exists for. The guards are the same t=8 and t=11
    // cells the one-shot test above asserts (both oracles), written as `$fatal`
    // so the staged entry points, which return an exit code rather than stdout,
    // can carry them. Exit 0 ⇒ the sidecar survived the `.velab`.
    let src = "`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0; int dv = 3; wire #(dv) w = a; wire yb, yg;\n\
\x20 assign #(dv) yb = a;\n\
\x20 buf #(dv) g(yg, a);\n\
\x20 initial begin\n\
\x20   #1 a = 1;\n\
\x20   #5;\n\
\x20   if (w !== 1'b1 || yb !== 1'b1 || yg !== 1'b1)\n\
\x20     $fatal(1, \"runtime rise delay dropped on the staged path\");\n\
\x20   a = 0;\n\
\x20   #2;\n\
\x20   if (w !== 1'b1 || yb !== 1'b1 || yg !== 1'b1)\n\
\x20     $fatal(1, \"runtime fall landed early on the staged path\");\n\
\x20   #3;\n\
\x20   if (w !== 1'b0 || yb !== 1'b0 || yg !== 1'b0)\n\
\x20     $fatal(1, \"runtime fall never landed on the staged path\");\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n";
    // The one-shot twin first, so a failure below is attributable to the staged
    // path and not to the design.
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "one-shot twin must pass first; got:\n{out}");

    let dir = dir_for("staged");
    let sv = dir.join("t.sv");
    std::fs::write(&sv, src).unwrap();
    let s = |p: &std::path::Path| p.to_str().unwrap().to_string();
    let vu = dir.join("t.vu");
    let velab = dir.join("t.velab");
    let o = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[s(&sv)], Some(&s(&vu)), &o),
        0,
        "vcmp failed"
    );
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &o), 0, "velab failed");
    assert_eq!(
        cli::run_vrun(&s(&velab), &o),
        0,
        "staged run dropped the ca_delay_exprs sidecar"
    );
}

#[test]
fn a_runtime_delay_inside_a_generate_scope() {
    // The generate Logic phase routes net-decl drivers through its own path; a
    // constant net-decl delay is pinned in `net_delay.rs`, this is the runtime
    // twin. `a` rises at 1 → both rise at 4, falls at 6 → both fall at 9.
    //
    // BOTH ORACLES: `T8 w=1 z=1`, `T10 w=0 z=0`. PRE printed `T8 w=0 z=0`.
    // (T4 is the write's own tick and the oracles split there — iverilog applies
    // it before the probe, verilator after — so it is not asserted.)
    let (out, code) = run("`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0; int dv = 3;\n\
\x20 generate if (1) begin : g wire #(dv) w = a; wire z; assign #(dv) z = a; end endgenerate\n\
\x20 initial begin\n\
\x20   #1 a = 1;\n\
\x20   #5 a = 0;\n\
\x20   #2 $display(\"T8 w=%b z=%b\", g.w, g.z);\n\
\x20   #2 $display(\"T10 w=%b z=%b\", g.w, g.z);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &["T8 w=1 z=1", "T10 w=0 z=0"],
        "generate-scope runtime delay",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The rule: evaluated when the RHS changes
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_delay_is_read_when_the_rhs_changes() {
    // `dv` goes 5 → 2 in the SAME time step `a` rises in, and changes nothing
    // that is already scheduled: the delay expression is not part of the
    // assign's sensitivity. `a8` then changes at t=7 with `dv = 2`, so `y8`
    // takes the value of `dv` AT THAT MOMENT.
    //
    // BOTH ORACLES:
    //   T6 y1=1 y8=10     (y8 = a8+1 = 0x10, scheduled at t=0 with dv=5, landed at 5)
    //   T9 y8=21          (a8 = 0x20 at t=7, +dv(2) → 0x21 at t=9)
    // PRE printed both of these too — by ACCIDENT, because a zero delay also
    // has the right value once enough time has passed. The discriminating cells
    // of this design are the two the oracles SPLIT on, recorded here and not
    // asserted: at t=4 iverilog reads `y1=0 y8=xx` (it scheduled y1 with the
    // dv=5 that was live when `a` changed) and verilator `y1=1 y8=00` (dv=2);
    // at t=9 iverilog reads `y8=21` and verilator `y8=10`. vita lands on
    // verilator's side at t=4 and iverilog's at t=9. Both are delta-ordering
    // inside one time step, which is §4.7 territory, not this slice's.
    let (out, code) = run("`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0; int dv = 5; wire y1; logic [7:0] a8 = 8'h0f; wire [7:0] y8;\n\
\x20 assign #(dv) y1 = a;\n\
\x20 assign #(dv) y8 = a8 + 1;\n\
\x20 initial begin\n\
\x20   #1 a = 1; dv = 2; #2.5 $display(\"T3 y1=%b y8=%h\", y1, y8);\n\
\x20   #3 $display(\"T6 y1=%b y8=%h\", y1, y8);\n\
\x20   a8 = 8'h20; #1.5 $display(\"T8 y8=%h\", y8); #1 $display(\"T9 y8=%h\", y8);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &["T6 y1=1 y8=10", "T9 y8=21"],
        "the delay value is read at the scheduling point",
    );
}

#[test]
fn a_delay_variable_write_alone_schedules_nothing() {
    // The teeth for the sentence above. `a` falls at t=7 with `dv = 5`, so the
    // fall is due at t=12; `dv` is then written at t=9 and t=11 and the rhs
    // never moves again. If the delay expression were part of the assign's
    // sensitivity — or if the pending write were re-timed when the variable
    // changed — the t=11 write of 90 would push the fall out to t=101.
    //
    // BOTH ORACLES: `T11 y=1` then `T13 y=0`. PRE printed `T11 y=0`, having
    // dropped `a` at t=7 with no delay. Both probes are after this assign's
    // first landed write (the rise at t=6), so neither is in the initial window.
    let (out, code) = run("`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0; int dv = 5; wire y;\n\
\x20 assign #(dv) y = a;\n\
\x20 initial begin\n\
\x20   #1 a = 1;\n\
\x20   #6 a = 0;\n\
\x20   #2 dv = 1;\n\
\x20   #2 dv = 90;\n\
\x20   $display(\"T11 y=%b\", y);\n\
\x20   #2 $display(\"T13 y=%b\", y);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &["T11 y=1", "T13 y=0"],
        "a write to the delay variable is not a reschedule",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The value domains `delay_ticks_of` owns
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn a_real_and_an_x_valued_runtime_delay() {
    // `real rv = 2.5` under `1ns/100ps` is 25 ticks, and an all-x delay is ZERO
    // ticks (`delay_ticks_of`: any X/Z → 0, iverilog parity) — the same rule the
    // procedural `#rv` / `#dx` takes, which is why the engine calls the shared
    // function rather than restating it.
    //
    // `a` rises at 1 → yr at 3.5, yx at once; `a` falls at 12.5 → yr at 15,
    // yx at once. BOTH ORACLES:
    //   T125 yr=1 yx=1
    //   T13  yr=1 yx=0
    //   T15  yr=0 yx=0
    // PRE printed `T13 yr=0 yx=0` — the real delay was dropped entirely.
    // (T35, at exactly 3.5, is the write's own tick and the oracles split:
    // iverilog `yr=1`, verilator `yr=0`.)
    let (out, code) = run("`timescale 1ns/100ps\n\
module t;\n\
\x20 logic a = 0; real rv = 2.5; logic [3:0] dx = 4'bx; wire yr, yx;\n\
\x20 assign #(rv) yr = a;\n\
\x20 assign #(dx) yx = a;\n\
\x20 initial begin\n\
\x20   #1 a = 1;\n\
\x20   #2.5 $display(\"T35 yr=%b yx=%b\", yr, yx);\n\
\x20   #9   $display(\"T125 yr=%b yx=%b\", yr, yx);\n\
\x20   a = 0;\n\
\x20   #0.5 $display(\"T13 yr=%b yx=%b\", yr, yx);\n\
\x20   #2   $display(\"T15 yr=%b yx=%b\", yr, yx);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &["T125 yr=1 yx=1", "T13 yr=1 yx=0", "T15 yr=0 yx=0"],
        "real and x-valued runtime delays",
    );
}

#[test]
fn a_negative_runtime_delay_never_fires() {
    // `delay_ticks_of` maps a negative amount — integral or real — to
    // `u64::MAX`, the never-fires sentinel, and `schedule_delayed_cas` must not
    // enqueue it: `u64::MAX` is not a tick, and `next_delayed_ca` would hand the
    // run loop a time to advance to.
    //
    // The cell the oracles DO arbitrate is whether the net ever follows the rhs,
    // so that is what the design asks. BOTH ORACLES: `FOLLOWED v=0 r=0 c=0`.
    // PRE printed `FOLLOWED v=1 r=1 c=0` — the two runtime spellings followed
    // `a` with no delay while the constant `#(NEG)` twin beside them already did
    // not. What the oracles do NOT arbitrate is what such a net reads instead:
    // see `initial_window_is_not_arbitrable`.
    let (out, code) = run("`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0; int dn = -1; real rn = -2.5; wire yv, yr, yc;\n\
\x20 localparam int NEG = -1;\n\
\x20 assign #(dn)  yv = a;\n\
\x20 assign #(rn)  yr = a;\n\
\x20 assign #(NEG) yc = a;\n\
\x20 initial begin\n\
\x20   #2 a = 1;\n\
\x20   #20 $display(\"FOLLOWED v=%0d r=%0d c=%0d\", yv === 1'b1, yr === 1'b1, yc === 1'b1);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &["FOLLOWED v=0 r=0 c=0"],
        "a negative runtime delay never fires",
    );
}

#[test]
fn precision_scaling_under_a_sub_unit_precision() {
    // `1ns/1ps` with `real dv = 2.5`: the two-stage conversion rounds to the
    // module's own precision first, so the delay is 2500 ticks, not 2 or 3 units.
    // `a` rises at 1 → yr at 3.5, yi (int 2) at 3; `a` falls at 6.2 → yr at 8.7,
    // yi at 8.2.
    //
    // BOTH ORACLES:
    //   T62 yr=1 yi=1
    //   T74 yr=1 yi=1
    //   T94 yr=0 yi=0
    // PRE printed `T74 yr=0 yi=0` — both delays were dropped, so the fall landed
    // at 6.2. (T32, at 3.2, is inside yr's initial window.)
    let (out, code) = run("`timescale 1ns/1ps\n\
module t;\n\
\x20 logic a = 0; real dv = 2.5; int di = 2; wire yr, yi;\n\
\x20 assign #(dv) yr = a;\n\
\x20 assign #(di) yi = a;\n\
\x20 initial begin\n\
\x20   #1 a = 1;\n\
\x20   #2.2 $display(\"T32 yr=%b yi=%b\", yr, yi);\n\
\x20   #3   $display(\"T62 yr=%b yi=%b\", yr, yi);\n\
\x20   a = 0;\n\
\x20   #1.2 $display(\"T74 yr=%b yi=%b\", yr, yi);\n\
\x20   #2   $display(\"T94 yr=%b yi=%b\", yr, yi);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &["T62 yr=1 yi=1", "T74 yr=1 yi=1", "T94 yr=0 yi=0"],
        "1ns/1ps precision scaling of a runtime delay",
    );
}

#[test]
fn the_multiplier_is_the_declaring_modules() {
    // A continuous assign has no process, so the engine has no `cur_time_mult`
    // to read for it — the sidecar carries the DECLARING module's. `sub` is
    // `1ns/1ps` and the testbench is `1ps/1ps`; `dv = 3` inside `sub` is 3 ns.
    // `a` rises at 1000 ps → `y` at 4000 ps.
    //
    // BOTH ORACLES agree on both lines (iverilog prints `x` and verilator `0` at
    // T3000 — the initial window, see the header — so only T5000 is asserted as
    // a two-oracle cell; T3000 is asserted as NOT-1, which both agree on and
    // which is the cell that fails if the ambient multiplier is used: with the
    // testbench's 1 ps the delay would be 3 ps and `y` would be 1 by 1003 ps).
    // PRE printed `T3000 y=1`.
    let (out, code) = run("`timescale 1ns/1ps\n\
module sub(input a, output wire y);\n\
\x20 int dv = 3;\n\
\x20 assign #(dv) y = a;\n\
endmodule\n\
`timescale 1ps/1ps\n\
module t;\n\
\x20 logic a = 0; wire y;\n\
\x20 sub u(a, y);\n\
\x20 initial begin\n\
\x20   #1000 a = 1;\n\
\x20   #2000 $display(\"T3000 follows=%0d\", y === 1'b1);\n\
\x20   #2000 $display(\"T5000 y=%b\", y);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &["T3000 follows=0", "T5000 y=1"],
        "the runtime delay scales by the declaring module's timescale",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The list form, and the mixed constant/runtime list
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn a_two_value_runtime_list_keeps_rise_and_fall_apart() {
    // `#(dr, df)` with both values runtime: rise 2, fall 6 (IEEE §7.14). `a`
    // rises at 1 → y2 at 3; `a` falls at 7 → y2 at 13. `#(dn)` with a 4-bit
    // `logic` delay of 3 is the single-value control in the same design.
    //
    // BOTH ORACLES:
    //   T7  y=1 y2=1 y3=1
    //   T12 y=0 y2=1 y3=0
    //   T14 y=0 y2=0 y3=0
    // PRE printed `T12 y=0 y2=0 y3=0` and `T7 y=1 y2=1 y3=1` (the last by
    // accident — with no delay everything had settled by t=7).
    // T8 splits on `y` alone (its write is due at exactly 8: iverilog applies it
    // before the probe, verilator after), so it is not asserted.
    let (out, code) = run("`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0; wire y, y2, y3; int dv = 5; int dr = 2, df = 6; logic [3:0] dn = 4'd3;\n\
\x20 assign #(dv) y = a;\n\
\x20 assign #(dr, df) y2 = a;\n\
\x20 assign #(dn) y3 = a;\n\
\x20 initial begin\n\
\x20   #1 a = 1;\n\
\x20   #6 $display(\"T7 y=%b y2=%b y3=%b\", y, y2, y3);\n\
\x20   dv = 1; a = 0;\n\
\x20   #5 $display(\"T12 y=%b y2=%b y3=%b\", y, y2, y3);\n\
\x20   #2 $display(\"T14 y=%b y2=%b y3=%b\", y, y2, y3);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &["T7 y=1 y2=1 y3=1", "T12 y=0 y2=1 y3=0", "T14 y=0 y2=0 y3=0"],
        "a two-value runtime delay list",
    );
}

#[test]
fn a_mixed_constant_and_runtime_list_takes_the_runtime_lane() {
    // The reason `ca_delay_is_runtime` asks ANY value and not just the first.
    // `#(2, dv)`: the rise folds, the fall does not — and the constant
    // `(rise, fall, turnoff)` sidecar needs EVERY value to fold, so before this
    // slice the fall silently collapsed onto the uniform rise of 2.
    //
    // `a` rises at 1 → ym at 3; `a` falls at 6 → ym at 11 (fall = dv = 5).
    // `#(2, 5)` is the fully constant control twin: same design, same bits, and
    // it was already correct — which is what makes this cell the predicate's.
    //
    // BOTH ORACLES: `T9 ym=1 yb=1`, `T12 ym=0 yb=0`.
    // PRE printed `T9 ym=0 yb=1` — the control right, the mixed spec wrong.
    let (out, code) = run("`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0; int dv = 5; wire ym, yb;\n\
\x20 assign #(2, dv) ym = a;\n\
\x20 assign #(2, 5)  yb = a;\n\
\x20 initial begin\n\
\x20   #1 a = 1;\n\
\x20   #5 a = 0;\n\
\x20   #3 $display(\"T9 ym=%b yb=%b\", ym, yb);\n\
\x20   #3 $display(\"T12 ym=%b yb=%b\", ym, yb);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &["T9 ym=1 yb=1", "T12 ym=0 yb=0"],
        "a mixed constant/runtime delay list",
    );
}

#[test]
fn a_zero_rise_with_a_runtime_fall_keeps_the_fall() {
    // The sharp edge of the ANY rule. `fold_ca_delay` deliberately suppresses a
    // SCOPE-FOLDED rise of zero (it keeps such an assign off the delayed lane to
    // avoid the `#0` lag, a documented trade) — but that choice only exists for
    // a WHOLLY constant delay. With a runtime fall the delayed lane is the only
    // way to deliver the fall at all, so `#(ZP, dv)` and `#(0, dv)` take it.
    //
    // `a` rises at 1 (rise = 0), falls at 6 → all three fall at 11.
    // BOTH ORACLES: `T3 yp=1 yl=1 yc=1`, `T9 yp=1 yl=1 yc=1`,
    //               `T12 yp=0 yl=0 yc=0`.
    // PRE printed `T9 yp=0 yl=0 yc=1` — the constant `#(0, 5)` control right,
    // both runtime-fall spellings wrong.
    let (out, code) = run("`timescale 1ns/1ns\n\
module t;\n\
\x20 parameter ZP = 0;\n\
\x20 logic a = 0; int dv = 5; wire yp, yl, yc;\n\
\x20 assign #(ZP, dv) yp = a;\n\
\x20 assign #(0,  dv) yl = a;\n\
\x20 assign #(0,  5)  yc = a;\n\
\x20 initial begin\n\
\x20   #1 a = 1;\n\
\x20   #2 $display(\"T3 yp=%b yl=%b yc=%b\", yp, yl, yc);\n\
\x20   #3 a = 0;\n\
\x20   #3 $display(\"T9 yp=%b yl=%b yc=%b\", yp, yl, yc);\n\
\x20   #3 $display(\"T12 yp=%b yl=%b yc=%b\", yp, yl, yc);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &[
            "T3 yp=1 yl=1 yc=1",
            "T9 yp=1 yl=1 yc=1",
            "T12 yp=0 yl=0 yc=0",
        ],
        "a zero rise with a runtime fall",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The one shape the lane hands back
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn a_resolved_net_keeps_its_pre_slice_delay() {
    // ⚠️ A COVERAGE NARROWING, pinned. A net with two whole-net continuous
    // drivers is resolved by 4-state wire resolution, and both spellings of
    // that eligibility rule — `sim_engine::multi_driver_groups` and elaborate's
    // `check_whole_net_multidriver` — exclude a net with any DELAYED driver.
    // The runtime lane's `Some(0)` routing flag is a delay to both, so without
    // `demote_runtime_delay_on_resolved_nets` this design stops elaborating
    // with E3001 — correct-to-loud, measured: PRE ran it and printed exactly
    // iverilog's two lines.
    //
    //   iverilog 13.0   T6 y=x   T11 y=1
    //   verilator 5.052 T6 y=1   T11 y=1   (2-state: it has no x to resolve to)
    //   PRE             T6 y=x   T11 y=1
    //
    // So this asserts PRE's own values: the delay is still dropped here (the
    // residue), and the design still runs. The constant twin — `assign #2 y = a;`
    // twice — is E3001 today; giving the delayed lane a resolved net is that
    // row's slice.
    let (out, code) = run("`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0, b = 0; int dv = 2; wire y;\n\
\x20 assign #(dv) y = a;\n\
\x20 assign #(dv) y = b;\n\
\x20 initial begin\n\
\x20   #1 a = 1;\n\
\x20   #5 $display(\"T6 y=%b\", y);\n\
\x20   b = 1;\n\
\x20   #5 $display(\"T11 y=%b\", y);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &["T6 y=x", "T11 y=1"],
        "two whole-net runtime-delayed drivers must still elaborate",
    );
}

#[test]
fn disjoint_part_select_drivers_keep_the_runtime_delay() {
    // The other side of that narrowing, so it cannot quietly widen: two drivers
    // on DISJOINT part-selects of one net are not a resolved group (they were
    // already excluded from `multi_driver_groups` by the select), so nothing is
    // handed back and the runtime delay stands.
    //
    // `a`/`b` rise at 1 → both nibbles land at 4; `a` falls at 6 → the low
    // nibble lands at 9. BOTH ORACLES: `T8 y=a5`, `T11 y=a0`.
    // PRE printed `T8 y=a0` — the fall landed at 6 with no delay.
    let (out, code) = run("`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0, b = 0; int dv = 3; wire [7:0] y;\n\
\x20 assign #(dv) y[3:0] = a ? 4'h5 : 4'h0;\n\
\x20 assign #(dv) y[7:4] = b ? 4'hA : 4'h0;\n\
\x20 initial begin\n\
\x20   #1 a = 1; b = 1;\n\
\x20   #5 a = 0;\n\
\x20   #2 $display(\"T8 y=%h\", y);\n\
\x20   #3 $display(\"T11 y=%h\", y);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &["T8 y=a5", "T11 y=a0"],
        "disjoint part-select drivers are not a resolved group",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The nine-form grounding grid (1ns/100ps)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn nine_runtime_delay_forms() {
    // The grounding census, kept whole: nine delay forms in one design under
    // `1ns/100ps`, probed at half-unit times. `a` rises at 1, and at 12.0
    // `dv = 1; a = 0`.
    //
    // y1 `#(dv)` · y2 `#(dr,df)` · y3 `#(2,dv)` · y4 `#(dv+1)` · y5 `#(P*dv)` ·
    // y6 `#(d8)` (8-bit reg) · y7 `#(dx)` (all-x) · y8 `#(rv)` (real 2.5) ·
    // y9 `#(dn)` (int -1).
    //
    // RAW ORACLE OUTPUT, both tools, line for line:
    //
    //   probe   iverilog 13.0   verilator 5.052   vita (this build)
    //   T1      000000100       000000100         xxxxxx1xx
    //   T3      010000110       000000110         x11xxx11x
    //   T4      010001110       000001110         x11xx111x
    //   T5      010001110       000001110         x11xx111x
    //   T6      110001110       100001110         111xx111x
    //   T7      110101110       100101110         1111x111x
    //   T8      110101110       100101110         1111x111x
    //   T11     110111110       100111110         11111111x
    //   T12     110111010       100111010         11111101x
    //   T13     010111010       000111010         01011101x
    //   T14     010001000       000001000         01000100x
    //   T16     010000000       000000000         01000000x
    //   T18     000000000       000000000         00000000x
    //   T22     000000000       000000000         00000000x
    //
    // vita differs from iverilog in exactly three places, each measured:
    //   * `x` where iverilog reads 0 — the initial window, before this assign's
    //     first write lands. Not arbitrable: see the header and
    //     `initial_window_is_not_arbitrable`.
    //   * y3 (`#(2,dv)`), where both tools stay 0 for the whole run. This
    //     timescale disqualifies both for a 2-value spec: verilator drops even
    //     the fully constant `#(2,5)` control here, and iverilog answers the
    //     same `#(2,dv)` source correctly under `1ns/1ns` (that is
    //     `a_mixed_constant_and_runtime_list_takes_the_runtime_lane`, where the
    //     two tools agree with each other and with this build). vita answers
    //     the two timescales alike.
    //   * y9 (`#(dn)`, -1), where both tools read 0 — the same initial-window
    //     question, for a net whose write never comes at all. The arbitrable
    //     half ("it never follows `a`") is
    //     `a_negative_runtime_delay_never_fires`.
    // y2 is iverilog-only under this timescale (verilator never raises it); the
    // two-oracle twin is `a_two_value_runtime_list_keeps_rise_and_fall_apart`.
    //
    // PRE printed `T1 11x111111` and `T3`..`T11` all `111111111` — every column
    // zero-delay — and `T12 001000000`.
    let (out, code) = run("`timescale 1ns/100ps\n\
module t;\n\
\x20 logic a = 0; int dv = 5; int dr = 2, df = 6; reg [7:0] d8 = 3; logic [3:0] dx = 4'bx; int dn = -1; real rv = 2.5;\n\
\x20 parameter P = 2;\n\
\x20 wire y1, y2, y3, y4, y5, y6, y7, y8, y9;\n\
\x20 assign #(dv) y1 = a;\n\
\x20 assign #(dr, df) y2 = a;\n\
\x20 assign #(2, dv) y3 = a;\n\
\x20 assign #(dv + 1) y4 = a;\n\
\x20 assign #(P * dv) y5 = a;\n\
\x20 assign #(d8) y6 = a;\n\
\x20 assign #(dx) y7 = a;\n\
\x20 assign #(rv) y8 = a;\n\
\x20 assign #(dn) y9 = a;\n\
\x20 task show(input int t); $display(\"T%0d %b%b%b%b%b%b%b%b%b\", t, y1, y2, y3, y4, y5, y6, y7, y8, y9); endtask\n\
\x20 initial begin\n\
\x20   #1 a = 1;\n\
\x20   #0.5 show(1); #2 show(3); #1 show(4); #1 show(5); #1 show(6); #1 show(7); #1 show(8); #3 show(11);\n\
\x20   #0.5 dv = 1; a = 0;\n\
\x20   #0.5 show(12); #1 show(13); #1 show(14); #2 show(16); #2 show(18); #4 show(22);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &[
            "T1 xxxxxx1xx",
            "T3 x11xxx11x",
            "T4 x11xx111x",
            "T5 x11xx111x",
            "T6 111xx111x",
            "T7 1111x111x",
            "T8 1111x111x",
            "T11 11111111x",
            "T12 11111101x",
            "T13 01011101x",
            "T14 01000100x",
            "T16 01000000x",
            "T18 00000000x",
            "T22 00000000x",
        ],
        "the nine-form runtime-delay grid",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The delay is an EXPRESSION now, so every gate that enumerates a design's
// expressions has to walk it
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn a_call_in_a_runtime_delay_falls_back_instead_of_panicking() {
    // ⭐ `assign #(dly(dv)) y = a;` — a SUBROUTINE CALL inside the delay. The
    // tier-3 (default `native`) gate refuses a call in a delayed continuous
    // assign, because `schedule_delayed_cas` evaluates through the bare arena
    // and `NetArena::eval_call` panics rather than X-poisoning. That row walked
    // the rhs and the lvalue index expressions only — the delay used to be a
    // folded tick count, and the comment saying so outlived the fact — so on the
    // DEFAULT backend this design aborted: `thread 'vita-main' panicked at
    // native/arena.rs:576`, rc 101, no diagnostic, a message naming an internal
    // seam. `--obs-dir` too. Now the gate sees it and the run falls back to `vm`.
    //
    // BOTH ORACLES (iverilog 13.0, verilator 5.052): `T7 y=1` / `T12 y=1`.
    // T4 is inside the initial window (`initial_window_is_not_arbitrable`), so
    // it is not asserted. BOTH SPELLINGS of the callee are pinned: `automatic`
    // (framed) and the plain STATIC one an inline fold could take — the hole was
    // the walk, not the routine's storage class.
    for (what, auto) in [("automatic", "automatic "), ("static", "")] {
        let (out, code) = run_with_diags(&format!(
            "`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0; int dv = 5; wire y;\n\
\x20 function {auto}int dly(input int k); dly = k + 1; endfunction\n\
\x20 assign #(dly(dv)) y = a;\n\
\x20 initial begin\n\
\x20   #1 a = 1;\n\
\x20   #6 $display(\"T7 y=%b\", y);\n\
\x20   #5 $display(\"T12 y=%b\", y);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n"
        ));
        want_lines(
            &out,
            code,
            &["T7 y=1", "T12 y=1", "a call in a delayed continuous assign"],
            &format!("a {what} call in a runtime delay must be loud-and-correct, never a panic"),
        );
    }
}

#[test]
fn a_runtime_delay_beside_a_frame_call_still_runs_natively() {
    // The CONTROL for the row above, and for the frame-local-net walk beside it:
    // without one, "the gate refuses that design" and "the gate refuses every
    // design with a runtime delay" read the same. Both halves are here — a framed
    // function the design calls, and a runtime structural delay — but the delay
    // names a MODULE net (`dv`), the only thing it can name: a delay is lowered
    // in module scope (`elaborate/ca_delay_rt.rs`), so it cannot reach a frame
    // window. Walking it must leave this design ADMITTED: no W4030, no panic.
    //
    // BOTH ORACLES: `C6 y=1 r=5` / `C7 y=1` / `C10 y=0`.
    // PRE printed `C7 y=0` — the fall was undelayed, the silent-wrong this file
    // is about.
    let (out, code) = run_with_diags(
        "`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0; int dv = 3; wire y; int r;\n\
\x20 function automatic int f(input int k); f = k + 1; endfunction\n\
\x20 assign #(dv) y = a;\n\
\x20 initial begin\n\
\x20   r = f(4);\n\
\x20   #1 a = 1;\n\
\x20   #5 $display(\"C6 y=%b r=%0d\", y, r);\n\
\x20   a = 0;\n\
\x20   #1 $display(\"C7 y=%b\", y);\n\
\x20   #3 $display(\"C10 y=%b\", y);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n",
    );
    want_lines(
        &out,
        code,
        &["C6 y=1 r=5", "C7 y=1", "C10 y=0"],
        "a runtime delay over module nets keeps its oracle values beside a frame call",
    );
    assert!(
        !out.contains("W4030"),
        "the delay names no frame-local net, so tier-3 must still take it; got:\n{out}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Residues, pinned at the value they were measured at
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn initial_window_is_not_arbitrable() {
    // ⚠️ A RESIDUE PIN, and the proof that it is one. Two spellings of the SAME
    // delay value in ONE design: `#(dv)` with `dv = 3`, and the literal `#3`.
    // `a` is 1 from t=0 and never changes.
    //
    //   iverilog 13.0   T1 yv=1 yc=x      ← the same delay, two answers
    //   verilator 5.052 T1 yv=0 yc=0      ← 2-state: cannot separate x from 0
    //   vita            T1 yv=x yc=x
    //
    // A tool that answers one question two ways is not the oracle for it. What
    // iverilog is doing is visible in its own numbers: it evaluates the assign
    // once before the `int dv = 3;` initializer has run, reads 0, and drives
    // immediately. vita answers both spellings with its `delayed_owes_initial_x`
    // drive, which iverilog agrees with on the constant spelling.
    //
    // Both oracles DO agree once the write has landed: `T5 yv=1 yc=1`.
    // PRE printed `T1 yv=1 yc=x` — it matched iverilog here, by having no delay
    // at all, which is the same mechanism that made every cell of
    // `nine_runtime_delay_forms` wrong.
    let (out, code) = run("`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 1; int dv = 3; wire yv, yc;\n\
\x20 assign #(dv) yv = a;\n\
\x20 assign #3    yc = a;\n\
\x20 initial begin\n\
\x20   #1 $display(\"T1 yv=%b yc=%b\", yv, yc);\n\
\x20   #4 $display(\"T5 yv=%b yc=%b\", yv, yc);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &["T1 yv=x yc=x", "T5 yv=1 yc=1"],
        "the initial window answers both spellings alike",
    );
}

#[test]
fn a_zero_runtime_delay_inherits_the_zero_tick_lag() {
    // ⚠️ A RESIDUE PIN. A runtime delay that evaluates to 0 lands after the
    // Postponed region of its own time value — ROADMAP §2's `#0` row, which the
    // constant `#0` spelling has had all along. Only a same-time-step `#0`
    // observer can see it; the `NEXT` probe, one time step later, is right.
    //
    //   iverilog 13.0   SAME 1 1   P0 1 1   P00 1 1   NEXT 1 1
    //   verilator 5.052 SAME 0 0   P0 0 0   P00 1 1   NEXT 1 1
    //   vita            SAME 0 0   P0 0 0   P00 0 0   NEXT 1 1
    //   PRE             SAME 1 0   P0 1 0   P00 1 0   NEXT 1 1
    //
    // The two oracles agree only at P00 and NEXT. P00 is therefore the one cell
    // this slice MOVED the wrong way: PRE read the runtime `#(dz)` there as 1
    // because it had no delay at all, and it now reads 0 like its constant `#0`
    // twin — which PRE also read as 0. The slice does not close that lag (its
    // fix is the constant `#0` row's, whose blast radius is every delayed
    // assign), it makes the runtime spelling share it.
    let (out, code) = run("`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0; int dz = 0; wire yv, yc;\n\
\x20 assign #(dz) yv = a;\n\
\x20 assign #0    yc = a;\n\
\x20 initial begin\n\
\x20   #1 a = 1;\n\
\x20   $display(\"SAME %b %b\", yv, yc);\n\
\x20   #0 $display(\"P0 %b %b\", yv, yc);\n\
\x20   #0 $display(\"P00 %b %b\", yv, yc);\n\
\x20   #1 $display(\"NEXT %b %b\", yv, yc);\n\
\x20   $finish;\n\
\x20 end\n\
endmodule\n");
    want_lines(
        &out,
        code,
        &["SAME 0 0", "P0 0 0", "P00 0 0", "NEXT 1 1"],
        "a zero runtime delay shares the constant #0 lag",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The control: a fully constant delay is untouched
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn constant_delay_control_is_byte_identical() {
    // ⭐ The non-vacuity control for "VCD bytes move only for designs with a
    // non-constant delay". Three constant structural delays — a literal, a
    // rise/fall pair through the `ca_delays` sidecar, and a localparam through
    // the scope fold — with no runtime value anywhere, so `ca_delay_exprs` must
    // stay EMPTY and every byte of the dump must be what it was.
    //
    // Measured, not asserted from theory: the PRE binary and this build produce
    // a byte-identical `ctl.vcd` for this design (`cmp` clean, both stdout and
    // dump). PRE is not available in CI, so the expected text is spelled out
    // here; if the runtime lane ever starts claiming a constant delay, the
    // `$dumpvars` block gains an x or a transition moves and this fails.
    let d = dir_for("ctl");
    let (out, code) = run_in(
        &d,
        "`timescale 1ns/1ns\n\
module t;\n\
\x20 logic a = 0; wire y1, y2, y3;\n\
\x20 localparam int D = 5;\n\
\x20 assign #5     y1 = a;\n\
\x20 assign #(2,6) y2 = a;\n\
\x20 assign #(D)   y3 = a;\n\
\x20 initial begin\n\
\x20   $dumpfile(\"ctl.vcd\"); $dumpvars(0, t);\n\
\x20   #2 a = 1; #10 a = 0; #10 $finish;\n\
\x20 end\n\
endmodule\n",
    );
    assert_eq!(code, Some(0), "constant-delay control; got:\n{out}");
    let vcd = std::fs::read_to_string(d.join("ctl.vcd")).expect("ctl.vcd must exist");
    let (_, body) = vcd
        .split_once("$enddefinitions $end\n")
        .expect("VCD header; got:\n{vcd}");
    // Rise 5 / fall 5 on y1 and y3 (a^ at 2 → 7, a_ at 12 → 17); rise 2 / fall 6
    // on y2 (4 and 18). All three oracle-pinned by `structural_delay_scope_fold`.
    assert_eq!(
        body,
        "$dumpvars\n0!\nx\"\nx#\nx$\n$end\n#0\n#2\n1!\n#4\n1#\n#7\n1\"\n1$\n#12\n0!\n#17\n0\"\n0$\n#18\n0#\n",
        "a constant structural delay must not move a single VCD byte"
    );
}
