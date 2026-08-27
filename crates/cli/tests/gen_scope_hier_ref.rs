//! A hierarchical reference INTO a singleton generate scope, by its bare label.
//!
//! THE REPORT was "vita can't do hierarchical references into generate blocks
//! (VITA-E3010)", and the census refuted its scope twice over before confirming
//! the defect underneath it. `for`-generate references (`u.g[0].x`) already
//! worked, in every form measured — net, localparam, an instance nested inside
//! the block, read and write. So did the INDEXED spelling of a conditional block
//! (`u.g[0].x`), and so did the bare spelling from inside the SAME module
//! (`gblk.x`, pinned since the initial commit by
//! `hier_ref.rs::named_generate_block_read`). Exactly one axis was broken, and
//! that existing green test is why it stayed invisible: the bare label ONE DOT
//! FURTHER OUT.
//!
//! ROOT 1 — `hier.rs::hier_resolve`. vita stores a singleton generate scope as
//! `label[0]`, so the bare hierarchical spelling has to be mapped onto it. Arm
//! (b) did that and said, in as many words, "Map only the leading segment". For
//! `gblk.x` the block IS the leading segment; for `u.gblk.x` the leading segment
//! is the INSTANCE, arm (a) commits to it, and the remainder was then looked up
//! verbatim as `u.gblk.x` while the net lives at `u.gblk[0].x`. 19 census cells:
//! `if` / `if…else` / `case` / bare `begin : g`, × net / localparam /
//! instance-inside, × read / write, plus depth 2 — all E3010, all correct in
//! iverilog.
//!
//! ROOT 2 — `hdl-parser::generate::parse_gen_case_item`. Both its arms called
//! `parse_gen_branch().1`, taking the items and DISCARDING `.0`, the label, where
//! the `if` and `for` arms bind it. A named generate-case block therefore minted
//! no scope at all and its members landed in the enclosing one — `u.g.x` AND
//! `u.g[0].x` both E3010, the only generate kind unreachable by either spelling.
//! Measured in the VCD: the `case` spelling emits scopes `tb u` where the `if`
//! spelling of the same design emits `tb u g[0]`. When the name collides with a
//! parent declaration it was worse than unreachable — E3009 "redeclared" on a
//! design both iverilog and verilator accept.
//!
//! ⚠️ WHAT MUST STAY LOUD, and why the guard needs two tests. §27.4 makes a
//! generate-FOR's blocks an ARRAY, whose name is illegal unindexed at ANY trip
//! count. Storage cannot recover that: a one-trip loop and a conditional block
//! both leave exactly `g[0]`. The first draft of the fix used only "does `[1]`
//! exist", which answered `u.g.x` on a one-trip loop (iverilog REJECTS it) while
//! still refusing two trips — a correct-or-loud refusal traded for a silent pick,
//! and one that would start failing the day a parameter moved from 1 to 2. The
//! `gen_loop_labels` set records the syntactic fact instead.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_genhier_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// `dut` body wrapped in each singleton generate spelling.
fn dut_with(kind: &str, body: &str) -> String {
    match kind {
        "if" => format!("generate if (1) begin : g {body} end endgenerate"),
        "ifelse" => {
            format!("generate if (0) begin : g logic [7:0] d=0; end else begin : g {body} end endgenerate")
        }
        "case" => format!(
            "generate case (1) 1: begin : g {body} end \
             default: begin : g logic [7:0] d=0; end endcase endgenerate"
        ),
        "bare" => format!("generate begin : g {body} end endgenerate"),
        _ => unreachable!(),
    }
}

const KINDS: [&str; 4] = ["if", "ifelse", "case", "bare"];

/// A NET inside a singleton generate scope, read across an instance boundary by
/// the bare label. iverilog prints `a5` for all four spellings.
#[test]
fn bare_label_cross_instance_net_read() {
    for k in KINDS {
        let src = format!(
            "module dut; {} endmodule\n\
             module tb; dut u(); initial #1 $display(\"V=%h\", u.g.x); endmodule\n",
            dut_with(k, "logic [7:0] x = 8'hA5;")
        );
        let (out, err, _) = run(&src);
        assert!(!err.contains("VITA-E"), "{k}: must resolve:\n{err}");
        assert!(out.contains("V=a5"), "{k}: iverilog says a5:\n{out}\n{err}");
    }
}

/// A LOCALPARAM in the same position. Separate from the net case because it
/// resolves through a different table (`hier_params`, not `symbols`) — the two
/// are distinct arguments to `hier_resolve` and only one of them was exercised
/// by the pre-existing local test. iverilog: `5a`.
#[test]
fn bare_label_cross_instance_param_read() {
    for k in KINDS {
        let src = format!(
            "module dut; {} endmodule\n\
             module tb; dut u(); initial #1 $display(\"V=%h\", u.g.P); endmodule\n",
            dut_with(k, "localparam [7:0] P = 8'h5A;")
        );
        let (out, err, _) = run(&src);
        assert!(!err.contains("VITA-E"), "{k}: must resolve:\n{err}");
        assert!(out.contains("V=5a"), "{k}: iverilog says 5a:\n{out}\n{err}");
    }
}

/// A MODULE INSTANCE nested inside the generate scope — the generate segment is
/// then an intermediate, not the last, so it exercises the walk rather than the
/// leaf. iverilog: `3c`.
#[test]
fn bare_label_cross_instance_nested_instance_read() {
    for k in KINDS {
        let src = format!(
            "module sub; logic [7:0] r = 8'h3C; endmodule\n\
             module dut; {} endmodule\n\
             module tb; dut u(); initial #1 $display(\"V=%h\", u.g.u_s.r); endmodule\n",
            dut_with(k, "sub u_s();")
        );
        let (out, err, _) = run(&src);
        assert!(!err.contains("VITA-E"), "{k}: must resolve:\n{err}");
        assert!(out.contains("V=3c"), "{k}: iverilog says 3c:\n{out}\n{err}");
    }
}

/// The WRITE twin. Fixing a read and leaving its write sibling broken is this
/// project's most repeated regression shape, and both directions land in
/// `hier_resolve` through different callers (`hier_defer/read.rs` vs
/// `write.rs`). iverilog: `a5`.
#[test]
fn bare_label_cross_instance_write() {
    for k in KINDS {
        let src = format!(
            "module dut; {} endmodule\n\
             module tb; dut u(); initial begin #1; u.g.x = 8'hA5; #1; \
             $display(\"V=%h\", u.g.x); end endmodule\n",
            dut_with(k, "logic [7:0] x = 8'h00;")
        );
        let (out, err, _) = run(&src);
        assert!(!err.contains("VITA-E"), "{k}: must resolve:\n{err}");
        assert!(out.contains("V=a5"), "{k}: iverilog says a5:\n{out}\n{err}");
    }
}

/// Two levels of instance, then the bare label. The outward walk commits at the
/// first level, so a depth-2 path proves the remainder walk runs per segment
/// rather than once. iverilog: `a5`.
#[test]
fn bare_label_depth_two() {
    let (out, err, _) = run(
        "module leaf; generate if (1) begin : g logic [7:0] x = 8'hA5; end endgenerate endmodule\n\
         module mid; leaf l(); endmodule\n\
         module tb; mid m(); initial #1 $display(\"V=%h\", m.l.g.x); endmodule\n",
    );
    assert!(!err.contains("VITA-E"), "must resolve:\n{err}");
    assert!(out.contains("V=a5"), "out:\n{out}\nerr:\n{err}");
}

/// Nested singleton scopes, both spelled bare — every segment of the remainder
/// needs the mapping, not just the first one after the commit. iverilog: `a5`.
#[test]
fn bare_label_nested_generate_scopes() {
    let (out, err, _) = run("module dut; generate if (1) begin : g\n\
           if (1) begin : h logic [7:0] x = 8'hA5; end\n\
         end endgenerate endmodule\n\
         module tb; dut u(); initial #1 $display(\"V=%h\", u.g.h.x); endmodule\n");
    assert!(!err.contains("VITA-E"), "must resolve:\n{err}");
    assert!(out.contains("V=a5"), "out:\n{out}\nerr:\n{err}");
}

/// ROOT 2, standalone: a named generate-CASE block must mint a scope, so a
/// same-named declaration in the parent is NOT a redeclaration. PRE emitted
/// E3009 `net/variable tb.u.x redeclared`; iverilog and verilator both run it
/// and print the parent's `11`.
#[test]
fn generate_case_block_mints_its_own_scope() {
    let (out, err, _) = run("module dut;\n\
           logic [7:0] x = 8'h11;\n\
           generate case (1)\n\
             1: begin : g logic [7:0] x = 8'h22; end\n\
             default: begin : g logic [7:0] x = 8'h33; end\n\
           endcase endgenerate\n\
           initial #1 $display(\"V=%h\", x);\n\
         endmodule\n\
         module tb; dut u(); endmodule\n");
    assert!(
        !err.contains("VITA-E"),
        "a case block's members belong to ITS scope, not the parent's:\n{err}"
    );
    assert!(out.contains("V=11"), "parent's own x:\n{out}\nerr:\n{err}");
}

/// An INSTANCE named `g` must keep resolving — the `[0]` spelling is a fallback
/// tried only after the plain name misses, so it can never shadow a real name.
#[test]
fn a_real_instance_named_g_still_resolves() {
    let (out, err, _) = run("module sub; logic [7:0] r = 8'h3C; endmodule\n\
         module dut; sub g(); endmodule\n\
         module tb; dut u(); initial #1 $display(\"V=%h\", u.g.r); endmodule\n");
    assert!(!err.contains("VITA-E"), "plain name must win:\n{err}");
    assert!(out.contains("V=3c"), "out:\n{out}\nerr:\n{err}");
}

/// A generate scope in one module and an instance of the same name in a SIBLING
/// module resolve independently. iverilog: `V=11 W=3c`.
#[test]
fn sibling_scope_and_instance_do_not_contaminate() {
    let (out, err, _) = run("module sub; logic [7:0] r = 8'h3C; endmodule\n\
         module a; generate if (1) begin : g logic [7:0] x = 8'h11; end endgenerate endmodule\n\
         module b; sub g(); endmodule\n\
         module tb; a ua(); b ub();\n\
           initial #1 $display(\"V=%h W=%h\", ua.g.x, ub.g.r);\n\
         endmodule\n");
    assert!(!err.contains("VITA-E"), "must resolve:\n{err}");
    assert!(out.contains("V=11 W=3c"), "out:\n{out}\nerr:\n{err}");
}

/// ⚠️ THE GUARD. A generate-FOR's blocks are an array (§27.4), so the bare label
/// is illegal — iverilog refuses to compile it — no matter how many times the
/// loop runs. The ONE-TRIP case is the one that matters: it leaves exactly the
/// same `g[0]` storage a conditional block leaves, so a fallback keyed on "is
/// there a `[1]`" accepted it. Both trip counts must stay loud, and they must
/// stay loud for the SAME reason.
#[test]
fn bare_label_on_a_for_generate_stays_loud_at_every_trip_count() {
    for trips in [1, 2, 3] {
        let src = format!(
            "module dut; generate for (genvar i=0;i<{trips};i++) begin : g \
             logic [7:0] x = 8'hA5; end endgenerate endmodule\n\
             module tb; dut u(); initial #1 $display(\"V=%h\", u.g.x); endmodule\n"
        );
        let (out, err, code) = run(&src);
        assert_ne!(code, Some(0), "trips={trips}: must be loud:\n{err}\n{out}");
        assert!(
            err.contains("VITA-E3010"),
            "trips={trips}: unresolved name:\n{err}"
        );
        assert!(
            !out.contains("V="),
            "trips={trips}: must not silently pick element 0:\n{out}"
        );
    }
}

/// A missing leaf inside a REAL singleton scope is still unresolved — committing
/// to the scope must not make its contents optimistic.
#[test]
fn missing_leaf_inside_a_resolved_scope_stays_loud() {
    let (out, err, code) = run(
        "module dut; generate if (1) begin : g logic [7:0] x = 8'hA5; end endgenerate endmodule\n\
         module tb; dut u(); initial #1 $display(\"V=%h\", u.g.nope); endmodule\n",
    );
    assert_ne!(code, Some(0), "must be loud:\n{err}\n{out}");
    assert!(err.contains("VITA-E3010"), "err:\n{err}");
}

/// A scope that does not exist at all stays loud (the fallback must not invent
/// one).
#[test]
fn missing_scope_stays_loud() {
    let (out, err, code) = run("module dut; logic [7:0] x = 8'hA5; endmodule\n\
         module tb; dut u(); initial #1 $display(\"V=%h\", u.nosuch.x); endmodule\n");
    assert_ne!(code, Some(0), "must be loud:\n{err}\n{out}");
    assert!(err.contains("VITA-E3010"), "err:\n{err}");
}
