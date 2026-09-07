//! §2 🆕 P: a CONTINUOUS assign whose lvalue is a hierarchical name.
//!
//! Both hierarchical-write resolvers patched their sentinel chunks by scanning
//! `stmts` only. A continuous assign is not a `Stmt` — `ContAssign.lhs` is a
//! SECOND lvalue arena — so `assign u1.x = v;` carried the sentinel net id
//! `HIER_WRITE_SENTINEL_BASE` (`0xFF00_0000`) into the engine, where
//! `chunk_width` indexes `nets[c.net]` and PANICKED (exit 101, "index out of
//! bounds: the len is 2 but the index is 4278190080"). A panic is below loud.
//!
//! The queue row named a generate block and an array index as the trigger.
//! Re-measured at HEAD, BOTH are irrelevant: the panic reproduces with a literal
//! RHS outside any generate block, and in every direction (self `top.o`, down
//! `u1.x`, up from a child). The single trigger is the hierarchical LVALUE.
//!
//! The `wire` guard moved with it. E3018 ("procedural hierarchical write to net
//! `x`") is a rule about PROCEDURAL writes; driving a wire is what `assign` is
//! FOR, and both oracles run `assign u1.w = v;`. So the guard is now keyed on the
//! lane, and the lane is read off the arena the sentinel landed in.
//!
//! Every value below is pinned to LIVE iverilog 13.0 and verilator 5.052 (both
//! agree on every case here unless the docstring says otherwise).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns `(stdout, exit code)`. The CODE matters here: the defect was a panic
/// (101), so asserting only on stdout would pass on a crash that printed the
/// warning banner first.
fn run(src: &str) -> (String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hca_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
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
        out.status.code().unwrap_or(-1),
    )
}

/// The queue row's own design: a generate-block continuous assign whose RHS
/// indexes a module-scope array, through a SELF-hierarchical target.
#[test]
fn generate_scope_self_hier_target_with_an_array_index() {
    let (out, code) = run("module top;\n\
           logic [31:0] A[0:3]; logic [31:0] o;\n\
           initial begin A[0]=32'd10; A[1]=32'd11; end\n\
           generate if (1) begin : g assign top.o = A[1]; end endgenerate\n\
           initial begin #1; $display(\"o=%0d\", o); $finish; end\n\
         endmodule\n");
    assert_eq!(code, 0, "expected exit 0, got {code}:\n{out}");
    assert!(out.contains("o=11"), "{out}");
}

/// …and the two axes the row named that turn out NOT to matter: no generate
/// block, and a literal RHS. Both panicked identically before the fix, which is
/// why the row's stated cause could not be the cause.
#[test]
fn neither_the_generate_block_nor_the_array_index_is_the_trigger() {
    let (out, code) = run("module top;\n\
           logic [31:0] A[0:3]; logic [31:0] o;\n\
           initial begin A[1]=32'd11; end\n\
           assign top.o = A[1];\n\
           initial begin #1; $display(\"o=%0d\", o); $finish; end\n\
         endmodule\n");
    assert_eq!(code, 0, "no generate block:\n{out}");
    assert!(out.contains("o=11"), "{out}");

    let (out, code) = run("module top;\n\
           logic [31:0] o;\n\
           generate if (1) begin : g assign top.o = 32'd11; end endgenerate\n\
           initial begin #1; $display(\"o=%0d\", o); $finish; end\n\
         endmodule\n");
    assert_eq!(code, 0, "literal rhs:\n{out}");
    assert!(out.contains("o=11"), "{out}");
}

/// Every DIRECTION the deferral lane serves: down into a child instance, and up
/// from a child into the parent. A whole-net write each time.
#[test]
fn the_target_may_be_below_or_above_the_writing_scope() {
    let (out, code) = run("module leaf; logic [31:0] x; endmodule\n\
         module top; leaf u1();\n\
           assign u1.x = 32'd11;\n\
           initial begin #1; $display(\"o=%0d\", u1.x); $finish; end\n\
         endmodule\n");
    assert_eq!(code, 0, "downward:\n{out}");
    assert!(out.contains("o=11"), "{out}");

    let (out, code) = run("module leaf; assign top.o = 32'd11; endmodule\n\
         module top; logic [31:0] o; leaf u1();\n\
           initial begin #1; $display(\"o=%0d\", o); $finish; end\n\
         endmodule\n");
    assert_eq!(code, 0, "upward:\n{out}");
    assert!(out.contains("o=11"), "{out}");
}

/// The SELECT lane (`resolve_deferred_hier_sel_write`) had the same one-arena
/// scan, so a hierarchical PART-select continuous assign panicked too.
#[test]
fn a_hierarchical_part_select_target_rebuilds_its_chunk() {
    let (out, code) = run("module leaf; logic [31:0] x; endmodule\n\
         module top; leaf u1();\n\
           assign u1.x[3:0] = 4'hb;\n\
           initial begin #1; $display(\"o=%0d\", u1.x[3:0]); $finish; end\n\
         endmodule\n");
    assert_eq!(code, 0, "part-select target:\n{out}");
    assert!(out.contains("o=11"), "{out}");
}

/// A `wire` destination. E3018 is a PROCEDURAL-write rule; the continuous lane is
/// exempt, so this went from a false-loud (with a message that called an `assign`
/// "procedural") straight to the oracles' value.
#[test]
fn a_wire_destination_is_legal_through_the_continuous_lane() {
    let (out, code) = run("module leaf; wire [31:0] x; endmodule\n\
         module top; leaf u1();\n\
           assign u1.x = 32'd11;\n\
           initial begin #1; $display(\"o=%0d\", u1.x); $finish; end\n\
         endmodule\n");
    assert_eq!(code, 0, "wire destination:\n{out}");
    assert!(out.contains("o=11"), "{out}");
}

/// …and the PROCEDURAL twin of that same design stays loud, which is the half
/// that says the guard was narrowed by lane and not simply deleted. iverilog
/// rejects it too.
#[test]
fn the_procedural_write_to_a_hierarchical_wire_is_still_loud() {
    let (out, code) = run("module leaf; wire [31:0] x; endmodule\n\
         module top; leaf u1();\n\
           initial begin u1.x = 32'd11; #1; $display(\"o=%0d\", u1.x); $finish; end\n\
         endmodule\n");
    assert_eq!(code, 1, "expected a loud exit 1:\n{out}");
    assert!(!out.contains("o=11"), "{out}");
}

/// The resolvers run BEFORE the whole-net multidriver scan on purpose, and until
/// now that scan never saw a hierarchical continuous assign at all (every one of
/// them carried a sentinel). Two continuous drivers on one wire must therefore
/// still RESOLVE rather than become E3001: `11` against `5` is `x` in every bit
/// that differs — vita = iverilog (verilator, being 2-state here, prints 5).
#[test]
fn two_continuous_drivers_on_the_target_resolve_rather_than_error() {
    let (out, code) = run("module leaf; wire [31:0] x; assign x = 32'd5; endmodule\n\
         module top; leaf u1();\n\
           assign u1.x = 32'd11;\n\
           initial begin #1; $display(\"o=%0d\", u1.x); $finish; end\n\
         endmodule\n");
    assert_eq!(code, 0, "resolved multidriver:\n{out}");
    assert!(out.contains("o=X") || out.contains("o=x"), "{out}");
}

/// Control: the hierarchical READ side inside a continuous assign was never
/// broken (reads live in the EXPR arena, which both resolvers already scanned),
/// so it must be byte-identical here. Whole-net and array-element spellings.
#[test]
fn the_hierarchical_read_side_of_a_continuous_assign_is_unchanged() {
    let (out, code) = run(
        "module leaf; logic [31:0] x; initial x = 32'd11; endmodule\n\
         module top; leaf u1(); logic [31:0] o;\n\
           assign o = u1.x;\n\
           initial begin #1; $display(\"o=%0d\", o); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("o=11"), "{out}");

    let (out, code) = run(
        "module leaf; logic [31:0] m[0:3]; initial m[1] = 32'd11; endmodule\n\
         module top; leaf u1(); logic [31:0] o;\n\
           assign o = u1.m[1];\n\
           initial begin #1; $display(\"o=%0d\", o); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("o=11"), "{out}");
}
