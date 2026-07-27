//! Procedural block-local declarations inside a GENERATE scope
//! (`generate for … begin : blk initial begin int k; … end end`).
//!
//! A plain scope asymmetry, not a missing feature: `elaborate_instance` hoists block-local
//! nets for every top-level `module.body` process, and the generate walk had no such arm —
//! so the identical process one level up worked while `int k` inside a generate scope was
//! an E3010 at every use. (A generate-scope net DECLARATION already worked; only a decl
//! inside the process did not.) This is the same class as the known "pre-scan only sees
//! top-level proc blocks" gap for hierarchical task calls.
//!
//! ORACLE: iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn compile(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("vita_gbl_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    let txt = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (txt, out.status.success())
}

fn run(src: &str) -> String {
    let (o, ok) = compile(src);
    assert!(ok, "expected success:\n{o}");
    assert!(!o.contains("E3010"), "undeclared block-local:\n{o}");
    o
}

#[test]
fn a_block_local_in_a_generate_process_resolves() {
    let o = run("module t; genvar g;\n\
        generate for (g=0; g<1; g=g+1) begin : blk\n\
          initial begin int k; k=5; $display(\"k=%0d\", k); end\n\
        end endgenerate\n\
        endmodule\n");
    assert!(o.contains("k=5"), "{o}");
}

#[test]
fn a_named_block_local_in_a_generate_process_resolves() {
    let o = run("module t; genvar g;\n\
        generate for (g=0; g<1; g=g+1) begin : blk\n\
          initial begin : nm int k; k=5; $display(\"k=%0d\", k); end\n\
        end endgenerate\n\
        endmodule\n");
    assert!(o.contains("k=5"), "{o}");
}

// ── each unrolled ITERATION gets its own net: no cross-iteration coalescing ──
#[test]
fn each_generate_iteration_gets_its_own_block_local() {
    let o = run("module t; genvar g;\n\
        generate for (g=0; g<3; g=g+1) begin : blk\n\
          initial begin int k; k = g*10; #1; $display(\"g=%0d k=%0d\", g, k); end\n\
        end endgenerate\n\
        endmodule\n");
    for want in ["g=0 k=0", "g=1 k=10", "g=2 k=20"] {
        assert!(o.contains(want), "missing `{want}`:\n{o}");
    }
}

#[test]
fn nested_generate_and_generate_if() {
    let o = run("module t; genvar g, h;\n\
        generate for (g=0; g<2; g=g+1) begin : outer\n\
          if (1) begin : inner\n\
            for (h=0; h<2; h=h+1) begin : deep\n\
              initial begin int z; z = g*10+h; $display(\"z=%0d\", z); end\n\
            end\n\
          end\n\
        end endgenerate\n\
        endmodule\n");
    for want in ["z=0", "z=1", "z=10", "z=11"] {
        assert!(o.contains(want), "missing `{want}`:\n{o}");
    }
}

// ── two SIBLING blocks in one iteration reusing a name: the module-scope coalescing
// model applies unchanged (they never overlap in time). iverilog agrees.
#[test]
fn sibling_blocks_in_one_iteration_reuse_a_local_name() {
    let o = run("module t; genvar g;\n\
        generate for (g=0; g<2; g=g+1) begin : blk\n\
          initial begin\n\
            begin int u; u = g;     $display(\"a g=%0d u=%0d\", g, u); end\n\
            begin int u; u = g+100; $display(\"b g=%0d u=%0d\", g, u); end\n\
          end\n\
        end endgenerate\n\
        endmodule\n");
    for want in ["a g=0 u=0", "a g=1 u=1", "b g=0 u=100", "b g=1 u=101"] {
        assert!(o.contains(want), "missing `{want}`:\n{o}");
    }
}

// ── declaration forms that were all E3010 before ──
#[test]
fn multi_name_loop_var_and_container_block_locals() {
    let o = run("module t; genvar g;\n\
        generate for (g=0; g<1; g=g+1) begin : blk\n\
          int arr[2];\n\
          initial begin int i; for (i=0;i<2;i=i+1) arr[i]=i*5;\n\
            $display(\"arr=%0d,%0d\", arr[0], arr[1]); end\n\
          initial begin int a, b; a=1; b=a+1; $display(\"ab=%0d,%0d\", a, b); end\n\
        end endgenerate\n\
        endmodule\n");
    assert!(o.contains("arr=0,5") && o.contains("ab=1,2"), "{o}");
}

#[test]
fn string_and_queue_block_locals_in_a_generate_process() {
    let o = run("module t; genvar g;\n\
        generate for (g=0; g<2; g=g+1) begin : blk\n\
          initial begin\n\
            string s; int q[$];\n\
            s = (g==0) ? \"aa\" : \"bb\";\n\
            q.push_back(g); q.push_back(g*7);\n\
            $display(\"g=%0d s=%s q=%0d,%0d\", g, s, q[0], q[1]);\n\
          end\n\
        end endgenerate\n\
        endmodule\n");
    assert!(
        o.contains("g=0 s=aa q=0,0") && o.contains("g=1 s=bb q=1,7"),
        "{o}"
    );
}

// ── a block-local whose name matches a generate-scope net still coalesces onto it, the
// same v1 flatten model as module scope — correct when the two never interleave …
#[test]
fn a_block_local_colliding_with_a_generate_scope_net_still_runs() {
    let o = run("module t; genvar g;\n\
        generate for (g=0; g<1; g=g+1) begin : blk\n\
          int k;\n\
          initial begin k=1; $display(\"outer=%0d\", k); end\n\
          initial begin int k; k=2; $display(\"inner=%0d\", k); end\n\
        end endgenerate\n\
        endmodule\n");
    assert!(o.contains("outer=1") && o.contains("inner=2"), "{o}");
}

// ── … and LOUD when they do. The existing flatten guard now covers generate scopes too;
// before the hoist this shape resolved the block-local straight onto the generate net and
// printed a silently wrong `2` where iverilog prints `1`.
#[test]
fn an_interleaving_collision_in_a_generate_scope_is_loud() {
    let (o, ok) = compile(
        "module t; genvar g;\n\
         generate for (g=0; g<1; g=g+1) begin : blk\n\
           int k;\n\
           initial begin k=1; #2; $display(\"outer-late=%0d\", k); end\n\
           initial begin int k; #1; k=2; $display(\"inner=%0d\", k); end\n\
         end endgenerate\n\
         endmodule\n",
    );
    assert!(!ok, "expected the flattened-net collision reject:\n{o}");
    assert!(o.contains("E3009"), "{o}");
}
