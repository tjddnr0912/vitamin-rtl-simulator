//! A subroutine-local net whose name collides with a module-scope parameter was
//! silently ignored: `localparam W = 4;` plus `function int f(); int W; W = 9;
//! return W; endfunction` returned 4, not 9, at exit 0. Task locals, block
//! locals and `localparam int` collided identically; a local with NO colliding
//! parameter always worked, which is what pinned the trigger.
//!
//! Root cause was a half-applied rule rather than a missing one. `lower_expr`
//! already re-derives the innermost binding over the combined
//! params/nets/string-param/real-param set, and its own comment says an inner net
//! should win — but the fall-through then called `lookup_scoped`, which runs a
//! params-ONLY walk and so ignored the key just derived. The parameter branch is
//! now skipped when that innermost binding is a net, and resolution falls to
//! `resolve_net` as the comment always intended.
//!
//! Every value pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ins_{}_{n}", std::process::id()));
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

/// Every scope that declares a local: a function body, a task body (read back
/// through an output), and a `begin…end` block — each colliding with a parameter.
#[test]
fn a_subroutine_local_shadows_a_colliding_parameter() {
    let (out, c) = run("module t;\n  localparam W = 4;\n  localparam int E = 3;\n\
           function automatic int fn();  int W; W = 9; return W; endfunction\n\
           function automatic int blk(); begin int W; W = 9; return W; end endfunction\n\
           function automatic int en();  int E; E = 8; return E; endfunction\n\
           task automatic tk(output int o); int W; W = 7; o = W; endtask\n\
           int a, b, cc, d;\n\
           initial begin a = fn(); b = blk(); cc = en(); tk(d);\n\
             $display(\"S=%0d %0d %0d %0d\", a, b, cc, d); #1 $finish; end\nendmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(out.contains("S=9 9 8 7"), "locals must win; got:\n{out}");
}

/// The counterpart that must NOT change: with no local of that name, the outer
/// parameter still resolves — inside a function, inside a task, and as a width.
#[test]
fn an_outer_parameter_still_resolves_without_a_local() {
    let (out, c) = run("module t;\n  localparam W = 8;\n  logic [W-1:0] bus;\n\
           function automatic int use_outer(); return W; endfunction\n\
           task automatic wr(input int v); bus = v[W-1:0]; endtask\n\
           initial begin wr(9);\n\
             $display(\"O=%0d %0d %0d\", use_outer(), $bits(bus), bus); #1 $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("O=8 8 9"),
        "outer param still wins; got:\n{out}"
    );
}

/// A local that collides with NOTHING was always correct and must stay so — this
/// is the control that isolates the collision as the trigger.
#[test]
fn a_non_colliding_local_is_unaffected() {
    let (out, c) = run("module t;\n  localparam W = 4;\n\
           function automatic int f(); int X; X = 9; return X; endfunction\n\
           int a;\n  initial begin a = f(); $display(\"N=%0d %0d\", a, W); #1 $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(out.contains("N=9 4"), "no-collision control; got:\n{out}");
}

/// A block-local hoisted into a GENERATE scope must NOT shadow the outer constant
/// for other readers of that scope. The v1 flatten publishes such a net under the
/// enclosing prefix's bare name (`t.g.W`), which is a DIFFERENT key from the
/// module constant (`t.W`) — so the inner-net-wins rule saw a legitimate-looking
/// shadow and handed every other reader in the generate block one process's
/// private variable. Adversarial review caught this; each design below was
/// byte-correct before the shadow fix and must stay so.
#[test]
fn a_hoisted_block_local_does_not_shadow_the_scope_it_was_flattened_into() {
    // A continuous assign reading the constant, beside a write-only block-local.
    let (out, c) = run("module t;\n  localparam W = 4;\n  logic [7:0] y;\n\
           generate if (1) begin : g\n\
             initial begin int W; W = 9; end\n\
             assign y = W;\n\
           end endgenerate\n\
           initial begin #1 $display(\"D=%0d\", y); $finish; end\nendmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("D=4"),
        "the constant must win here; got:\n{out}"
    );

    // A GENVAR read in the same generate body.
    let (out, c) = run("module t;\n  genvar i;\n\
           generate for (i = 0; i < 2; i = i + 1) begin : g\n\
             initial begin int i; i = 77; end\n\
             initial #1 $display(\"K=%0d\", i);\n\
           end endgenerate\n  initial #2 $finish;\nendmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("K=0") && out.contains("K=1"),
        "genvar; got:\n{out}"
    );

    // A PER-INSTANCE parameter override — the leak defeated the override entirely.
    let (out, c) = run("module c #(parameter W = 0) (output logic [7:0] o);\n\
           generate if (1) begin : g\n\
             initial begin int W; W = 9; end\n\
             assign o = W;\n\
           end endgenerate\nendmodule\n\
         module t; logic [7:0] a, b; c #(.W(4)) u1(a); c #(.W(5)) u2(b);\n\
           initial begin #1 $display(\"I=%0d %0d\", a, b); $finish; end\nendmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(out.contains("I=4 5"), "per-instance overrides; got:\n{out}");
}

/// Regression guard for §4.5.218's S1 failure, where an earlier attempt at this
/// fix made a nested generate body vanish silently at exit 0. A generate-scope
/// localparam drives an inner loop bound and an inner if — all bodies must run.
#[test]
fn nested_generate_bodies_are_not_dropped() {
    let (out, c) = run("module t;\n  localparam N = 2;\n  genvar i, j;\n\
           generate for (i = 0; i < N; i = i + 1) begin : o\n\
             localparam M = i + 1;\n\
             for (j = 0; j < M; j = j + 1) begin : n\n\
               initial $display(\"G=%0d.%0d\", i, j); end\n\
             if (M > 1) begin : c initial $display(\"C=%0d\", i); end\n\
           end endgenerate\n  initial #1 $finish;\nendmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    for w in ["G=0.0", "G=1.0", "G=1.1", "C=1"] {
        assert!(
            out.contains(w),
            "expected `{w}` (S1 regression); got:\n{out}"
        );
    }
}
