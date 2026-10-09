//! A deferred immediate assertion written as a module item (IEEE 1800-2017 §16.4,
//! A.6.10 `deferred_immediate_assertion_item`): `[L :] assert #0 (c) …;`,
//! `assert final`, and the `assume` forms. The LRM treats each "as if it were
//! contained in an always_comb procedure", and the parser desugars it onto exactly
//! that, so every value below equals the hand-written `always_comb begin … end`
//! twin's (the `*_equals_its_always_comb_twin` tests assert it byte for byte).
//!
//! Oracles: verilator 5.052 (`--binary`) prints the same reports, each one twice
//! and some once more after `$finish` (its always_comb re-evaluation, the same for
//! the hand-written twin); the expectations below are its de-duplicated reports.
//! It is not an oracle on x/z (2-state) or on the deferred flush: in the glitch
//! test it reports `F t=10 a=0 b=1`, the transient a `#0` (Inactive region) write
//! cancels before the Observed region (§16.4.1, §4.4), so that test is hand-IEEE.
//! iverilog 13 rejects deferred assertions (`sorry: Deferred assertions are not
//! supported`) and sv2v drops them.
//!
//! The witness is VeeR EH1 under its default `ASSERT_ON` (`dma_ctrl.sv:501`,
//! `assert_done_and_novalid: assert #0 (…);` inside a `for (genvar …)` loop).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dami_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["-Wno-W1017", "t.sv"])
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// The `$display` lines, without the end-of-run summary.
fn lines(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| !l.starts_with("simulation ended") && !l.starts_with("errors="))
        .collect()
}

fn ok(src: &str) -> Vec<String> {
    let (out, err, code) = run(src);
    assert_eq!(code, Some(0), "stdout:\n{out}\nstderr:\n{err}");
    lines(&out).into_iter().map(str::to_string).collect()
}

#[test]
fn eh1_census_repro_runs_the_failing_item_only() {
    // row-9 census repro (VeeR O1); verilator: `b_fail fired`, `v=0100`.
    let got = ok("module top;\n\
         logic [3:0] v = 4'b0100;\n\
         a_onehot: assert #0 ($onehot0(v));\n\
         b_fail:   assert #0 ($onehot0(4'b0110)) else $display(\"b_fail fired\");\n\
         initial #1 begin $display(\"v=%b\", v); $finish; end\n\
         endmodule\n");
    assert_eq!(got, ["b_fail fired", "v=0100"]);
}

const TRACK: &str = "module top;\n\
     logic [3:0] a = 0, b = 0;\n\
     {ITEM}\n\
     initial begin\n\
       #5 a = 3;\n\
       #5 b = 3;\n\
       #5 a = 7; b = 7;\n\
       #5 b = 1;\n\
       #5 $display(\"end t=%0t\", $time); $finish;\n\
     end\n\
     endmodule\n";
const TRACK_ITEM: &str = "assert #0 (a == b) else $display(\"F t=%0t a=%0d b=%0d\", $time, a, b);";

#[test]
fn reports_once_per_settled_failing_time_slot() {
    // `a = 7; b = 7;` in one process settles before the always_comb runs: no report
    // at 15. verilator (de-duplicated): F t=5, F t=20.
    let got = ok(&TRACK.replace("{ITEM}", TRACK_ITEM));
    assert_eq!(got, ["F t=5 a=3 b=0", "F t=20 a=7 b=1", "end t=25"]);
}

#[test]
fn tracking_item_equals_its_always_comb_twin() {
    let item = run(&TRACK.replace("{ITEM}", TRACK_ITEM));
    let twin = run(&TRACK.replace("{ITEM}", &format!("always_comb begin {TRACK_ITEM} end")));
    assert_eq!(item, twin);
}

#[test]
fn a_transient_cancelled_by_a_zero_delay_write_does_not_report() {
    // hand-IEEE §16.4.1: at 10, `a = 0` re-runs the always_comb with b still 1 (a
    // pending failure); `#0 b = 0` (Inactive region) re-runs it before the Observed
    // region, flushing the pending report. At 5 the two writes come from two
    // processes in one slot: the second reach flushes the first. verilator reports
    // `F t=10 a=0 b=1` (no flush), for the hand-written always_comb as well.
    let got = ok("module top;\n\
         logic a = 0, b = 0;\n\
         assert #0 (a == b) else $display(\"F t=%0t a=%b b=%b\", $time, a, b);\n\
         initial begin #5 a = 1; end\n\
         initial begin #5 b = 1; end\n\
         initial begin #10 a = 0; #0 b = 0; end\n\
         initial #20 begin $display(\"end\"); $finish; end\n\
         endmodule\n");
    assert_eq!(got, ["end"]);
}

#[test]
fn default_action_is_the_assertion_error_at_the_item() {
    let (out, err, code) = run("module top;\n\
         logic c = 1;\n\
         assert #0 (c);\n\
         initial begin #5 c = 0; #5 c = 1; #5 $display(\"end\"); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(1), "stdout:\n{out}\nstderr:\n{err}");
    assert!(
        err.contains(
            "t.sv:3:1: error[VITA-E4003] E-RUN-USER-ERROR: Assertion failed [in top] [at time 5]"
        ),
        "stderr:\n{err}"
    );
    assert_eq!(err.matches("Assertion failed").count(), 1, "stderr:\n{err}");
    assert_eq!(lines(&out), ["end"]);
}

#[test]
fn assert_final_and_assume_forms_are_items_too() {
    // verilator (de-duplicated): F t=5 s=2 / F t=5 a=9.
    let fin = ok("module top;\n\
         logic [1:0] s = 0;\n\
         assert final (s != 2) else $display(\"F t=%0t s=%0d\", $time, s);\n\
         initial begin #5 s = 2; #5 s = 1; #5 $display(\"end\"); $finish; end\n\
         endmodule\n");
    assert_eq!(fin, ["F t=5 s=2", "end"]);
    let asm = ok("module top;\n\
         logic [3:0] a = 4'd3;\n\
         assume #0 (a < 8) else $display(\"F t=%0t a=%0d\", $time, a);\n\
         initial begin #5 a = 9; #5 a = 2; #5 $display(\"end\"); $finish; end\n\
         endmodule\n");
    assert_eq!(asm, ["F t=5 a=9", "end"]);
}

#[test]
fn the_item_runs_at_time_zero_with_its_pass_action() {
    // always_comb's time-zero pass (§9.2.2.2.2). verilator (de-duplicated):
    // P t=0, F t=5, P t=10.
    let got = ok("module top;\n\
         logic [3:0] a = 4'd3;\n\
         assert #0 (a != 5) $display(\"P t=%0t a=%0d\", $time, a); else $display(\"F t=%0t a=%0d\", $time, a);\n\
         initial begin #5 a = 5; #5 a = 6; #5 $display(\"end\"); $finish; end\n\
         endmodule\n");
    assert_eq!(got, ["P t=0 a=3", "F t=5 a=5", "P t=10 a=6", "end"]);
    let t0 = ok("module top;\n\
         logic [3:0] a = 4'd9;\n\
         assert #0 (a < 8) else $display(\"F t=%0t a=%0d\", $time, a);\n\
         initial begin #5 a = 1; #5 $display(\"end\"); $finish; end\n\
         endmodule\n");
    assert_eq!(t0, ["F t=0 a=9", "end"]);
}

#[test]
fn an_x_or_z_condition_fails() {
    // hand-IEEE §16.3: a condition that is x or z fails. verilator is 2-state (it
    // reads both as 0 and fails as well).
    let got = ok("module top;\n\
         logic c;\n\
         logic [1:0] z = 2'bz1;\n\
         assert #0 (c) else $display(\"Fc t=%0t c=%b\", $time, c);\n\
         assert #0 (z[1]) else $display(\"Fz t=%0t z=%b\", $time, z);\n\
         initial begin #5 c = 1; #5 z = 2'b11; #5 $display(\"end\"); $finish; end\n\
         endmodule\n");
    assert_eq!(got, ["Fc t=0 c=x", "Fz t=0 z=z1", "end"]);
}

#[test]
fn a_labelled_item_in_a_generate_loop_is_one_check_per_iteration() {
    // The EH1 shape. verilator (de-duplicated): F t=5 i=0, F t=5 i=2, F t=10 i=2.
    let got = ok("module top;\n\
         localparam DEPTH = 3;\n\
         logic [DEPTH-1:0] done = 0, valid = 0;\n\
         for (genvar i = 0; i < DEPTH; i++) begin\n\
           assert_done_and_novalid: assert #0 (~done[i] | valid[i]) else $display(\"F t=%0t i=%0d\", $time, i);\n\
         end\n\
         initial begin #5 done = 3'b101; #5 valid = 3'b001; #5 valid = 3'b101; #5 $display(\"end\"); $finish; end\n\
         endmodule\n");
    assert_eq!(got, ["F t=5 i=0", "F t=5 i=2", "F t=10 i=2", "end"]);
}

#[test]
fn a_label_names_the_scope() {
    // verilator: `F top.L1 t=5`, `G top t=5`.
    let got = ok("module top;\n\
         logic c = 1;\n\
         L1: assert #0 (c) else $display(\"F %m t=%0t\", $time);\n\
         assert #0 (c) else $display(\"G %m t=%0t\", $time);\n\
         initial begin #5 c = 0; #5 $display(\"end\"); $finish; end\n\
         endmodule\n");
    assert_eq!(got, ["F top.L1 t=5", "G top t=5", "end"]);
}

#[test]
fn an_action_block_read_is_part_of_the_sensitivity() {
    // §9.2.2.2.1: the always_comb is sensitive to `b`, read only by the action
    // block. verilator (de-duplicated): F t=5 b=0, F t=10 b=4, F t=15 b=5.
    let got = ok("module top;\n\
         logic a = 1; logic [3:0] b = 0;\n\
         assert #0 (a) else $display(\"F t=%0t b=%0d\", $time, b);\n\
         initial begin #5 a = 0; #5 b = 4; #5 b = 5; #5 a = 1; #5 b = 6; #5 $display(\"end\"); $finish; end\n\
         endmodule\n");
    assert_eq!(got, ["F t=5 b=0", "F t=10 b=4", "F t=15 b=5", "end"]);
}

#[test]
fn interface_and_per_instance_items() {
    // verilator (de-duplicated): IF t=5 d=15 / F top.u1 t=5 x=5, F top.u2 t=10 x=6.
    let ifc = ok("interface bus_if; logic [3:0] d = 0; assert #0 (d != 4'hf) else $display(\"IF t=%0t d=%0d\", $time, d); endinterface\n\
         module top;\n\
         bus_if b();\n\
         initial begin #5 b.d = 4'hf; #5 b.d = 1; #5 $display(\"end\"); $finish; end\n\
         endmodule\n");
    assert_eq!(ifc, ["IF t=5 d=15", "end"]);
    let inst = ok("module chk(input logic [3:0] x); assert #0 (x < 4) else $display(\"F %m t=%0t x=%0d\", $time, x); endmodule\n\
         module top;\n\
         logic [3:0] p = 0, q = 0;\n\
         chk u1(.x(p)); chk u2(.x(q));\n\
         initial begin #5 p = 5; #5 q = 6; p = 1; #5 $display(\"end\"); $finish; end\n\
         endmodule\n");
    assert_eq!(inst, ["F top.u1 t=5 x=5", "F top.u2 t=10 x=6", "end"]);
}

#[test]
fn a_fatal_action_ends_the_run() {
    let (out, err, code) = run("module top;\n\
         logic c = 1;\n\
         assert #0 (c) else $fatal(1, \"boom t=%0t\", $time);\n\
         initial begin #5 c = 0; #5 $display(\"not reached\"); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(1), "stdout:\n{out}\nstderr:\n{err}");
    assert!(
        err.contains("fatal[VITA-F4004] F-RUN-FATAL: boom t=5 [in top] [at time 5]"),
        "stderr:\n{err}"
    );
    assert!(!out.contains("not reached"), "stdout:\n{out}");
}

#[test]
fn forms_that_are_not_module_items_stay_loud() {
    let cases = [
        // A simple immediate assertion is procedural only (§16.3).
        (
            "module top; logic c = 0; assert (c) else $display(\"F\"); initial #1 $finish; endmodule\n",
            "expected `property` after `assert`/`assume` at module level",
        ),
        // Only `#0` is a deferred form.
        (
            "module top; logic c = 0; assert #1 (c) else $display(\"F\"); initial #1 $finish; endmodule\n",
            "a deferred-assertion delay must be `#0`",
        ),
        // A program has no deferred assertion item (A.1.7).
        (
            "program pr; logic c = 0; assert #0 (c) else $display(\"PF\"); initial #1 $display(\"p\"); endprogram\n\
             module top; pr u(); initial #2 $finish; endmodule\n",
            "expected `property` after `assert`/`assume` at module level",
        ),
        // Nor does a package (A.1.11).
        (
            "package pk; logic c = 0; assert #0 (c) else $display(\"KF\"); endpackage\n\
             module top; import pk::*; initial #2 $finish; endmodule\n",
            "expected `property` after `assert`/`assume` at module level",
        ),
        // A declaration in the action block would bind to the module's same-named
        // variable (Verilator `b=8`; vita read the module's `x`, 1).
        (
            "module t;\n  logic [31:0] x;\n  \
             assert final (0) else begin : b logic [3:0][7:0] x; $display(\"R b=%0d\", $bits(x[1])); end\n  \
             initial begin #1 $finish; end\nendmodule\n",
            "a declaration in its action block is unsupported",
        ),
    ];
    for (src, msg) in cases {
        let (out, err, code) = run(src);
        assert_eq!(code, Some(1), "{src}\nstdout:\n{out}\nstderr:\n{err}");
        assert!(err.contains("error[VITA-E2002]"), "{src}\nstderr:\n{err}");
        assert!(err.contains(msg), "{src}\nstderr:\n{err}");
        assert!(!out.contains("simulation ended"), "{src}\nstdout:\n{out}");
    }
}

/// vita's global assertion control does not reach an immediate or deferred
/// assertion's actions, so a module-item deferred assertion would keep reporting
/// while assertions are off (Verilator: `F1 t=15` only). A design that holds both is
/// refused at elaboration, naming the item and the control call.
#[test]
fn assertion_control_over_an_item_is_refused() {
    let item = "module top;\n\
                logic c = 1;\n\
                L1: assert #0 (c) else $display(\"F1 t=%0t\", $time);\n\
                initial begin\n\
                  {ctl};\n\
                  #5 c = 0;\n\
                  #5 $display(\"end\"); $finish;\n\
                end\n\
                endmodule\n";
    for ctl in [
        "$assertoff",
        "$asserton",
        "$assertkill",
        "$assertcontrol(4)",
        "$assertfailoff",
        "$assertpasson",
    ] {
        let src = item.replace("{ctl}", ctl);
        let (out, err, code) = run(&src);
        assert_eq!(code, Some(1), "{ctl}\nstdout:\n{out}\nstderr:\n{err}");
        assert!(
            err.contains("t.sv:3:1: error[VITA-E3009]")
                && err.contains(
                    "assertion control over a module-item deferred assertion is not supported yet"
                )
                && err.contains("t.sv:5:1: note[VITA-E3009]"),
            "{ctl}\nstderr:\n{err}"
        );
        assert!(!out.contains("simulation ended"), "{ctl}\nstdout:\n{out}");
    }
    // The same control over a procedural deferred assertion runs as before.
    let got = ok("module top;\n\
         logic c = 1;\n\
         always_comb begin L1: assert #0 (c) else $display(\"F1 t=%0t\", $time); end\n\
         initial begin $assertoff; #5 c = 0; #5 $display(\"end\"); $finish; end\n\
         endmodule\n");
    assert_eq!(got, ["F1 t=5", "end"]);
}

/// The refusal is decided over the whole elaborated design, so it holds when the
/// item and the control call come from separately compiled work-library units.
#[test]
fn assertion_control_over_an_item_is_refused_across_work_library_units() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dami_wl_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("dut.sv"),
        "module dut(input logic e);\n  L1: assert #0 (e) else $display(\"F t=%0t\", $time);\nendmodule\n",
    )
    .unwrap();
    std::fs::write(
        d.join("tb.sv"),
        "module top;\n  logic e = 1;\n  dut u(.e(e));\n  \
         initial begin $assertoff; #5 e = 0; #5 $display(\"end\"); $finish; end\nendmodule\n",
    )
    .unwrap();
    let vita = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_vita"))
            .args(args)
            .current_dir(&d)
            .output()
            .expect("run vita")
    };
    for unit in ["dut.sv", "tb.sv"] {
        let o = vita(&["vcmp", "--work", "lib", unit]);
        assert_eq!(
            o.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
    let o = vita(&[
        "velab",
        "-Wno-W1017",
        "-L",
        "lib",
        "--top",
        "top",
        "-o",
        "t.velab",
    ]);
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(o.status.code(), Some(1), "stderr:\n{err}");
    assert!(
        err.contains("error[VITA-E3009]")
            && err.contains(
                "assertion control over a module-item deferred assertion is not supported yet"
            ),
        "stderr:\n{err}"
    );
}
