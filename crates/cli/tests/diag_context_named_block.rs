//! The `[in …]` context of a runtime diagnostic is the statement's `%m` string —
//! ROADMAP §2 🆕 N residue.
//!
//! `initial begin : blk $error("boom"); end` reported `[in top]` where both
//! oracles name the block (iverilog `Scope: top.blk`, verilator `in top.blk`),
//! and a singleton generate scope kept the synthetic `[0]` vita stores
//! (`[in top.g[0]]` for iverilog's `top.g`). The proof it was one defect and not
//! a family: vita printed `[in top]` and `m=top.blk` on two consecutive lines of
//! one block — the marker was built from the raw storage prefix while `%m` went
//! through the scope chain three lines away in the same lowering.
//!
//! So every test below asserts BOTH halves: the marker and the `%m` of a
//! `$display` in the same scope must be the same string. Values are pinned to
//! iverilog 13.0 (`-g2012`); verilator 5.052 agrees where it gets that far (it
//! aborts at the first `$error`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// vita's stdout+stderr, `$finish`/epilogue lines dropped, SORTED — two
/// processes and two generate arms print in scheduling order.
fn run(src: &str) -> Vec<String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_dcnb_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut all = String::from_utf8_lossy(&out.stdout).into_owned();
    all.push_str(&String::from_utf8_lossy(&out.stderr));
    let mut v: Vec<String> = all
        .lines()
        .filter(|l| {
            !l.starts_with("simulation ended")
                && !l.starts_with("errors=")
                && !l.contains("W-PP-TIMESCALE-DEFAULT")
        })
        .map(|l| l.to_string())
        .collect();
    v.sort();
    v
}

/// A named block, a nested pair, an unnamed block (the control that must not
/// move) and a SINGLETON generate scope.
#[test]
fn severity_context_names_the_block_chain() {
    let out = run("module top;\n\
           initial begin : blk\n\
             $error(\"boom\");\n\
             $display(\"m=%m\");\n\
           end\n\
           initial begin : outer\n\
             begin : inner\n\
               $warning(\"w2\");\n\
               $display(\"m2=%m\");\n\
             end\n\
           end\n\
           initial begin\n\
             $info(\"plain\");\n\
             $display(\"m3=%m\");\n\
           end\n\
           generate if (1) begin : g\n\
             initial begin\n\
               $error(\"ing\");\n\
               $display(\"m4=%m\");\n\
             end\n\
           end endgenerate\n\
           initial #20 $finish;\n\
         endmodule\n");
    // iverilog: Scope: top.blk / top.outer.inner / top / top.g.
    let joined = out.join("\n");
    for needle in [
        "E-RUN-USER-ERROR: boom [in top.blk]",
        "m=top.blk",
        "W-RUN-USER-WARNING: w2 [in top.outer.inner]",
        "m2=top.outer.inner",
        "I-RUN-USER-INFO: plain [in top]",
        "m3=top",
        // The synthetic `[0]` of a singleton generate scope is stripped, exactly
        // as `%m` strips it — `display_of`, keyed on `gen_singleton_labels`.
        "E-RUN-USER-ERROR: ing [in top.g]",
        "m4=top.g",
    ] {
        assert!(joined.contains(needle), "missing `{needle}` in:\n{joined}");
    }
    assert!(
        !joined.contains("[in top.g[0]]"),
        "the singleton generate `[0]` survived:\n{joined}"
    );
}

/// A generate LOOP keeps its index — the marker is stripped by a positive record
/// of the singleton labels, never by "it ends in `[0]`".
#[test]
fn generate_loop_keeps_its_index_and_gains_the_block_label() {
    let out = run("module top;\n\
           generate for (genvar i = 0; i < 2; i++) begin : gl\n\
             initial begin : ib\n\
               $error(\"loop%0d\", i);\n\
               $display(\"ml=%m\");\n\
             end\n\
           end endgenerate\n\
           initial #20 $finish;\n\
         endmodule\n");
    let joined = out.join("\n");
    // iverilog: Scope: top.gl[0].ib and top.gl[1].ib.
    for needle in [
        "E-RUN-USER-ERROR: loop0 [in top.gl[0].ib]",
        "E-RUN-USER-ERROR: loop1 [in top.gl[1].ib]",
        "ml=top.gl[0].ib",
        "ml=top.gl[1].ib",
    ] {
        assert!(joined.contains(needle), "missing `{needle}` in:\n{joined}");
    }
}

/// `stmt_locs` is shared: the out-of-range array-word report reads the same
/// record, so it moves with the severity family and stays equal to `%m`.
#[test]
fn the_shared_runtime_diagnostics_move_with_it() {
    let out = run("module top;\n\
           logic [7:0] a [0:3];\n\
           int j;\n\
           initial begin : uc\n\
             j = 7;\n\
             a[j] = 8'h11;\n\
             $display(\"mu=%m\");\n\
           end\n\
           initial #20 $finish;\n\
         endmodule\n");
    let joined = out.join("\n");
    assert!(
        joined.contains("E-RUN-RANGE: array word index of `top.a` (out of range; read X / write ignored) [in top.uc]"),
        "the out-of-range report did not follow the block chain:\n{joined}"
    );
    assert!(joined.contains("mu=top.uc"), "{joined}");
}

/// A SUBROUTINE body was already right (§4.5.437) and must not move; an
/// assertion action block reports the assertion label, the side of the
/// iverilog/verilator split vita's `%m` already stands on (verilator prints
/// `Assertion failed in top.b1.ap`, iverilog drops the label).
#[test]
fn subroutine_body_unchanged_and_the_assertion_label_is_kept() {
    let out = run("module top;\n\
           task automatic t; $error(\"in-task\"); $display(\"mt=%m\"); endtask\n\
           initial begin : b1\n\
             ap: assert (0) else $error(\"assert-fail\");\n\
             $display(\"ma=%m\");\n\
           end\n\
           initial begin : b2\n\
             t();\n\
           end\n\
           initial begin : b3\n\
             $info(\"i3\");\n\
             $warning(\"w3\");\n\
             $display(\"m3=%m\");\n\
           end\n\
           initial #20 $finish;\n\
         endmodule\n");
    let joined = out.join("\n");
    for needle in [
        // The DECLARING scope of the task, not the calling block `b2`.
        "E-RUN-USER-ERROR: in-task [in top.t]",
        "mt=top.t",
        "E-RUN-USER-ERROR: assert-fail [in top.b1.ap]",
        "ma=top.b1",
        "I-RUN-USER-INFO: i3 [in top.b3]",
        "W-RUN-USER-WARNING: w3 [in top.b3]",
        "m3=top.b3",
    ] {
        assert!(joined.contains(needle), "missing `{needle}` in:\n{joined}");
    }
}
