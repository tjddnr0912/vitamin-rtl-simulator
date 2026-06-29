//! Block-local scope leak (IEEE §6.21). A local declared in a nested `begin…end`
//! is visible only within that block; a reference outside it must resolve to the
//! lexically-enclosing binding (a module variable / a formal / an outer local).
//! vita keeps function/task/procedural block-locals in a FLAT per-body table, so
//! such an outside reference would SILENTLY read the block-local instead — a
//! scope silent-wrong (iverilog resolves it correctly). Proper per-block scope
//! lowering is a follow-on (ROADMAP §4.5.18); until then vita rejects the exact
//! leak LOUD (correct-or-loud), while a block-local used only inside its own block
//! keeps working. Loud cases are pinned to "vita errors"; the working cases are
//! pinned to iverilog 13.0 values.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_blkscope_{}_{n}", std::process::id()));
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
        out.status.code(),
    )
}

// ── silent-wrong patterns → now LOUD ──────────────────────────────────────

#[test]
fn leak_shadow_read_after_block_is_loud() {
    // Inner block local `x` read at function-body level: should be module `x`=10
    // (iverilog), vita used to silently read the block-local (5).
    let (_o, code) = run("module top; int x=10; function int f(); int y; \
         begin: ib int x; x=5; end y=x; return y; endfunction \
         initial begin $display(\"r=%0d\",f()); #1 $finish; end endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "block-local read after its block must be loud"
    );
}

#[test]
fn leak_sibling_blocks_is_loud() {
    let (_o, code) = run("module top; int x=10; function int f(); int y; \
         begin: a int x; x=1; end begin: b int x; x=2; end y=x; return y; endfunction \
         initial begin $display(\"r=%0d\",f()); #1 $finish; end endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "sibling block-locals read outside must be loud"
    );
}

#[test]
fn leak_nested_block_read_in_outer_is_loud() {
    let (_o, code) = run("module top; int x=10; function int f(); int y; \
         begin: m begin: inner int x; x=3; end y=x; end return y; endfunction \
         initial begin $display(\"r=%0d\",f()); #1 $finish; end endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "deeper block-local read in outer block must be loud"
    );
}

#[test]
fn leak_procedural_block_is_loud() {
    // The same leak in an initial/always procedural block.
    let (_o, code) = run("module top; int x=10; \
         initial begin begin int x; x=5; end #1 $display(\"r=%0d\",x); $finish; end endmodule\n");
    assert_ne!(code, Some(0), "procedural block-local leak must be loud");
}

#[test]
fn leak_ancestor_redeclares_name_is_loud() {
    // An outer block declares `x` and a nested INNER block re-declares `x`; a read
    // in the outer block (after inner) hits the coalesced slot (vita=99) instead
    // of the outer's own `x` (iverilog=5). A second adversarial round caught the
    // first sibling-fix wrongly suppressing this (ancestor ≠ sibling) — it must
    // stay loud, not silently read 99.
    let (_o, code) = run("module top; function automatic int foo(); int r; \
         begin: outer int x; x=5; begin: inner int x; x=99; end r=x; end \
         return foo+r; endfunction \
         initial begin $display(\"v=%0d\",foo()); #1 $finish; end endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "ancestor-redeclare coalescing hazard must be loud"
    );
}

// ── valid patterns → still correct (no false loud) ────────────────────────

#[test]
fn block_local_read_inside_own_block_ok() {
    // The block-local is read only inside its own block → resolves correctly.
    let (out, code) = run("module top; int x=10; function int f(); int y; \
         begin: ib int x; x=5; y=x; end return y; endfunction \
         initial begin $display(\"r=%0d\",f()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("r=5"), "inner-only read ok; got:\n{out}");
}

#[test]
fn function_level_local_shadows_module_ok() {
    // A function-body-level local (not nested) legitimately shadows a module var.
    let (out, code) = run(
        "module top; int x=10; function int f(); int x; x=7; return x; endfunction \
         initial begin $display(\"r=%0d\",f()); #1 $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(out.contains("r=7"), "function-level shadow ok; got:\n{out}");
}

#[test]
fn sibling_blocks_self_contained_ok() {
    // Two sibling blocks each declaring their OWN `x`, each used only inside —
    // valid SV (iverilog runs it). An adversarial review caught the first cut
    // wrongly flagging this because the detector recursed into a sibling block
    // that re-declares the name. Now each self-contained sibling resolves.
    let (out, code) = run("module top; function automatic int foo(); int r; \
         begin begin int x; x=1; end begin int x; x=2; r=x; end foo=r+40; end \
         return foo; endfunction \
         initial begin $display(\"v=%0d\",foo()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("v=42"),
        "self-contained siblings ok; got:\n{out}"
    );
}

#[test]
fn if_else_branch_locals_ok() {
    // The same name declared in both branches of an if/else, each branch-local —
    // valid (not a leak).
    let (out, code) = run(
        "module top; function automatic int foo(input int sel); int r; \
         begin if(sel) begin int v; v=10; r=v; end else begin int v; v=20; r=v; end end \
         return r; endfunction \
         initial begin $display(\"v=%0d\",foo(1)); #1 $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(
        out.contains("v=10"),
        "if/else branch locals ok; got:\n{out}"
    );
}

#[test]
fn for_loop_local_ok() {
    // A `for(int i…)` body local used only in the loop is unaffected.
    let (out, code) = run("module top; function int f(); int t=0; \
         for(int i=0;i<4;i++) t+=i; return t; endfunction \
         initial begin $display(\"s=%0d\",f()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("s=6"), "for-loop local ok; got:\n{out}");
}
