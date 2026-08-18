//! An inner procedural block-local shadows an outer constant — for readers INSIDE its
//! block, and only those.
//!
//! ROADMAP §2 (DEEP), and the prerequisite the roadmap called "an order-INDEPENDENT,
//! AST-gathered per-scope name set". `localparam N = 7; initial begin : blk int N;
//! N = 3; $display(N); end` printed 7 where iverilog prints 3 — silently. A function-
//! or task-local of the same name already won; only the BLOCK-local lost.
//!
//! WHY IT LOST, AND WHY THE OBVIOUS FIX IS WRONG. v1 flattens a procedural block-local
//! to a module net under its BARE name, so the binding it appears to make is wider than
//! the block that declared it. The shadow test therefore carved hoisted block-locals
//! out entirely — because letting them shadow made every OTHER reader in the scope (a
//! sibling `initial`, a continuous assign, an inner generate) resolve to one process's
//! private variable. Both directions are wrong, and iverilog says so: r1 (inside) must
//! be 3 and r2 (outside) must be 7, in the same design.
//!
//! The discriminator is the declaring block's SOURCE SPAN, which `hoist_block_local_nets`
//! already holds — and which is an AST fact, so nothing here depends on how far
//! elaboration has progressed. That property is not decoration: the previous attempt at
//! this fix keyed on `symbols`, which is populated DURING elaboration, and silently
//! deleted a whole generate body (§4.5.218). The last test below is that hazard.
//!
//! ORACLE: iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_blsp_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Every case prints r1 from INSIDE the declaring block and r2 from OUTSIDE it, so one
/// design tests both directions — which is the only way to tell the two silent-wrongs
/// apart (a fix that shadows everywhere passes an r1-only test).
#[test]
fn a_block_local_wins_inside_its_block_and_loses_outside() {
    for (name, src) in [
        (
            "named block",
            "module tb; localparam N = 7;\n\
             \x20 initial begin : blk int N; N = 3; $display(\"r1=%0d\", N); end\n\
             \x20 initial begin #1; $display(\"r2=%0d\", N); end\n\
             endmodule\n",
        ),
        (
            "unnamed block",
            "module tb; localparam N = 7;\n\
             \x20 initial begin int N; N = 3; $display(\"r1=%0d\", N); end\n\
             \x20 initial begin #1; $display(\"r2=%0d\", N); end\n\
             endmodule\n",
        ),
        (
            // Inside a generate the flattened key is `tb.gb[0].N`, DIFFERENT from the
            // module constant's — the shape the old carve-out was written for.
            "inside a generate",
            "module tb; localparam N = 7; genvar g;\n\
             \x20 generate for (g=0;g<1;g=g+1) begin : gb\n\
             \x20   initial begin : blk int N; N = 3; $display(\"r1=%0d\", N); end\n\
             \x20   initial begin #1; $display(\"r2=%0d\", N); end\n\
             \x20 end endgenerate\n\
             endmodule\n",
        ),
    ] {
        let out = run(src);
        assert!(
            out.contains("r1=3"),
            "{name}: inside the block the LOCAL wins:\n{out}"
        );
        assert!(
            out.contains("r2=7"),
            "{name}: outside it the CONSTANT wins:\n{out}"
        );
    }
}

#[test]
fn the_local_type_is_the_locals_not_the_constants() {
    // A width discriminator on top of the value one: `logic [3:0] W` truncates where
    // the 32-bit constant would not. iverilog prints a / 8.
    let out = run("module tb; localparam W = 8;\n\
         \x20 initial begin : b logic [3:0] W; W = 4'hA; $display(\"r1=%0h\", W); end\n\
         \x20 initial begin #1; $display(\"r2=%0d\", W); end\n\
         endmodule\n");
    assert!(
        out.contains("r1=a"),
        "the local's own width applies:\n{out}"
    );
    assert!(
        out.contains("r2=8"),
        "…and the constant is untouched outside:\n{out}"
    );
}

#[test]
fn a_width_context_reader_outside_the_block_still_folds_the_constant() {
    // A declaration range is elaborated outside any block, so it must keep resolving to
    // the constant. If it took the local instead, the net would silently change width.
    let out = run("module tb; localparam N = 7; logic [N-1:0] w;\n\
         \x20 initial begin : blk int N; N = 3; $display(\"r1=%0d\", N); end\n\
         \x20 initial begin #1; $display(\"r2=%0d\", $bits(w)); end\n\
         endmodule\n");
    assert!(
        out.contains("r1=3"),
        "the block reader sees the local:\n{out}"
    );
    assert!(
        out.contains("r2=7"),
        "the declaration width stays the constant's:\n{out}"
    );
}

#[test]
fn a_generate_body_does_not_vanish() {
    // §4.5.218: the previous attempt at this rule keyed on `symbols` — populated DURING
    // elaboration — so a generate control expression folded in one phase and failed in
    // another, deleting the body at exit 0. A generate control is lexically OUTSIDE the
    // block, so the span test never sees the local; these bodies must run.
    let cond = run("module tb; localparam N = 1;\n\
         \x20 initial begin : blk int N; N = 0; $display(\"r1=%0d\", N); end\n\
         \x20 generate if (N) begin : gb\n\
         \x20   initial begin #1; $display(\"r2=alive\"); end\n\
         \x20 end endgenerate\n\
         endmodule\n");
    assert!(
        cond.contains("r1=0"),
        "the block reader sees the local:\n{cond}"
    );
    assert!(
        cond.contains("r2=alive"),
        "the generate body must NOT vanish — it folds the constant:\n{cond}"
    );

    let loop_ = run("module tb; localparam N = 2; genvar i;\n\
         \x20 initial begin : blk int N; N = 0; $display(\"r1=%0d\", N); end\n\
         \x20 generate for (i=0;i<N;i=i+1) begin : gb\n\
         \x20   initial begin #1; $display(\"r2=%0d\", i); end\n\
         \x20 end endgenerate\n\
         endmodule\n");
    assert!(loop_.contains("r1=0"), "block reader:\n{loop_}");
    assert!(
        loop_.contains("r2=0") && loop_.contains("r2=1"),
        "both generate iterations must exist — the bound folds the constant:\n{loop_}"
    );
}

#[test]
fn a_function_or_task_local_is_unchanged() {
    // REGRESSION GUARD: these already won before this slice (they are not hoisted
    // block-locals), so the new predicate must not have moved them.
    let f = run("module tb; localparam N = 7;\n\
         \x20 function int f; int N; begin N = 3; f = N; end endfunction\n\
         \x20 initial begin $display(\"r1=%0d\", f()); $display(\"r2=%0d\", N); end\n\
         endmodule\n");
    assert!(
        f.contains("r1=3") && f.contains("r2=7"),
        "function-local unchanged:\n{f}"
    );
}

#[test]
fn a_duplicate_param_and_net_declaration_is_unchanged_by_this_slice() {
    // NOT a correctness claim — `localparam N = 7; logic [3:0] N;` is ILLEGAL, and both
    // oracles say so ("'N' has already been declared in this scope" / "Duplicate
    // declaration of signal"). vita accepts it and resolves the name to the parameter,
    // before this slice and after.
    //
    // It is pinned because it is the ONE reachable design where the shadow rule's
    // `!params` clause changes the answer: without it the net would win and this would
    // print x. So the clause is load-bearing, this slice must not move the answer, and
    // the real fix — refusing the duplicate declaration, which is a separate
    // vita-accepts-what-every-oracle-rejects item — is recorded in ROADMAP §3.
    let out = run(
        "module tb; localparam N = 7; logic [3:0] N;\n         \x20 initial $display(\"r=%0d\", N);\n         endmodule\n",
    );
    assert!(
        out.contains("r=7"),
        "unchanged: the parameter still wins for a duplicate declaration:\n{out}"
    );
}
