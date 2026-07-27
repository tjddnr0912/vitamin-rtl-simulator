//! String / queue / dynamic-array DECL-INITs inside a GENERATE or INTERFACE scope.
//!
//! These were loud through an `allow_string_init = false` flag on those two scopes. The
//! flag was standing in for a real defect rather than describing one: a string or
//! dynamic-handle declaration does part of its work at DECLARATION time — a scalar
//! string's t0 init, a routed string array's `new[n]` pre-size — and both pushed a
//! BARE-NAME lvalue into the flat module-scope pending list. That list is flushed with
//! `cur_prefix` empty, so a declaration inside `begin : g` emitted a write to `t.s`
//! instead of `t.g[0].s`.
//!
//! Both pushes are keyed by the declaring scope's prefix now
//! (`pending_scoped_presize` / `pending_scoped_bl_strings`) and drained at that scope's
//! own flush, which every scope with a flush already had. Module scope is the `""` key,
//! in the same position it always occupied.
//!
//! ORACLE: iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("vita_gisdi_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    let o = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "expected success:\n{o}");
    o
}

// ── generate scope ──────────────────────────────────────────────────────────

#[test]
fn generate_scope_scalar_string_decl_init() {
    let o = run("module t; genvar g;\n\
        generate for (g=0; g<1; g=g+1) begin : blk\n\
          string s = \"hi\";\n\
          initial $display(\"%s\", s);\n\
        end endgenerate\n\
        endmodule\n");
    assert!(o.contains("hi"), "{o}");
}

#[test]
fn generate_scope_string_array_decl_init_and_runtime_index() {
    let o = run("module t; genvar i;\n\
        generate for (i=0; i<2; i=i+1) begin : g\n\
          string s[2] = '{\"aa\",\"bb\"};\n\
          string sc = \"sc\";\n\
          initial begin int k; k=1;\n\
            $display(\"i=%0d %s %s %s\", i, s[0], s[k], sc); end\n\
        end endgenerate\n\
        endmodule\n");
    assert!(
        o.contains("i=0 aa bb sc") && o.contains("i=1 aa bb sc"),
        "{o}"
    );
}

// ── the queue / dyn-array twins that shared the same gate ──
#[test]
fn generate_scope_queue_and_dyn_array_decl_init() {
    let o = run("module t;\n\
        generate if (1) begin : g\n\
          int q[$] = '{1,2,3};\n\
          int d[] = '{7,8};\n\
          string sq[$] = '{\"p\",\"q\"};\n\
          initial $display(\"q=%0d,%0d,%0d sz=%0d d=%0d,%0d sq=%s,%s\",\n\
                           q[0], q[1], q[2], q.size(), d[0], d[1], sq[0], sq[1]);\n\
        end endgenerate\n\
        initial #1 $finish;\n\
        endmodule\n");
    assert!(o.contains("q=1,2,3 sz=3 d=7,8 sq=p,q"), "{o}");
}

// ── each unrolled iteration initializes its OWN container ──
#[test]
fn each_generate_iteration_initializes_its_own_string_array() {
    let o = run("module t; genvar g;\n\
        generate for (g=0; g<2; g=g+1) begin : blk\n\
          string s[2] = '{\"a1\",\"b2\"};\n\
          initial begin if (g==0) s[0] = \"MUT\";\n\
            #1; $display(\"g=%0d [%s][%s]\", g, s[0], s[1]); end\n\
        end endgenerate\n\
        endmodule\n");
    assert!(o.contains("g=0 [MUT][b2]"), "{o}");
    assert!(
        o.contains("g=1 [a1][b2]"),
        "iteration 1 saw 0's write:\n{o}"
    );
}

// ── a BLOCK-LOCAL string inside a generate process (the same scoped drain) ──
#[test]
fn block_local_strings_inside_a_generate_process() {
    let o = run("module t; genvar g;\n\
        generate for (g=0; g<2; g=g+1) begin : gb\n\
          initial begin\n\
            string s = \"SCA\";\n\
            string a[2] = '{\"a1\",\"b2\"};\n\
            $display(\"g=%0d [%s][%s][%s]\", g, s, a[0], a[1]);\n\
          end\n\
        end endgenerate\n\
        initial #1 $finish;\n\
        endmodule\n");
    assert!(
        o.contains("g=0 [SCA][a1][b2]") && o.contains("g=1 [SCA][a1][b2]"),
        "{o}"
    );
}

// ── interface scope: the same flush, so the same fix ─────────────────────────

#[test]
fn interface_scope_string_and_string_array_decl_init() {
    let o = run("interface ifc;\n\
          string nm = \"IFC\";\n\
          string tags[2] = '{\"t0\",\"t1\"};\n\
        endinterface\n\
        module t; ifc u();\n\
          initial $display(\"%s %s %s\", u.nm, u.tags[0], u.tags[1]);\n\
        endmodule\n");
    assert!(o.contains("IFC t0 t1"), "{o}");
}

#[test]
fn interface_scope_queue_decl_init() {
    // Read from INSIDE the interface: a HIERARCHICAL read of a queue element
    // (`u.q[0]`) is a separate, still-loud gap and is not what this covers.
    let o = run("interface ifc;\n\
          int q[$] = '{5,6};\n\
          initial $display(\"%0d %0d sz=%0d\", q[0], q[1], q.size());\n\
        endinterface\n\
        module t; ifc u(); endmodule\n");
    assert!(o.contains("5 6 sz=2"), "{o}");
}

// ── ORDERING: the pre-size must still precede its element writes, and a block-local
// string init must still follow the module-scope strings it may read. Both are what the
// two separate drain points exist for.
#[test]
fn scoped_drains_keep_their_relative_order() {
    let o = run("module t;\n\
        string m = \"MOD\";\n\
        genvar g;\n\
        generate for (g=0; g<1; g=g+1) begin : blk\n\
          string arr[3] = '{\"x\",\"y\",\"z\"};\n\
          initial begin\n\
            string bl;\n\
            bl = m;\n\
            $display(\"[%s][%s][%s][%s]\", arr[0], arr[2], bl, m);\n\
          end\n\
        end endgenerate\n\
        endmodule\n");
    assert!(o.contains("[x][z][MOD][MOD]"), "{o}");
}
