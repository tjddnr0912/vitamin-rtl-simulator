//! r18 (E1): an `enum`-typedef task/function FORMAL (`input e_t m`) now has its enum
//! methods (`m.name()`, `m.next()`, `m.first`, …) desugar inside the body — was E3009
//! "unsupported hierarchical function call `m.name`" because only a module-scope enum
//! VARIABLE (via `var_enum`) or a body-LOCAL enum var was registered; a formal was not.
//!
//! The fix threads the enum type name of a tf-port through the `TfPortType` inheritance
//! (so a bare continuation `input e_t a, b` binds both) and registers the port name in
//! `var_enum` — scoped to the tf body by the existing snapshot/restore around the port
//! list, exactly like the struct-port `bind_tf_port_struct`.
//!
//! ORACLE: iverilog 13.0 runs enum methods on a formal, so these are iverilog-verified.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_emf_{}_{n}", std::process::id()));
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

fn is_loud(o: &str) -> bool {
    o.contains("E3009") || o.contains("E2002")
}

// ── the report's E1: `.name()` on an enum formal inside a suspending task ──
#[test]
fn name_on_formal_in_suspending_task() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        typedef enum logic [1:0] { A=0, B=1 } e_t;\n\
        task automatic run (input e_t m);\n\
          @(posedge clk); $display(\"mode=%s\", m.name());\n\
        endtask\n\
        initial begin run(B); $finish; end\n\
        endmodule\n");
    assert!(!is_loud(&o) && o.contains("mode=B"), "E1 repro:\n{o}");
}

// ── `.name()` on an enum formal inside a plain FUNCTION ──
#[test]
fn name_on_formal_in_function() {
    let o = run("module t;\n\
        typedef enum logic [1:0] { RED=0, GRN=1, BLU=2 } col_t;\n\
        function automatic string label (input col_t c);\n\
          label = c.name();\n\
        endfunction\n\
        initial begin if (label(GRN) == \"GRN\") $display(\"PASS\"); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("PASS"),
        "name on fn formal:\n{o}"
    );
}

// ── a bare-continuation `input e_t a, b` binds BOTH names ──
#[test]
fn name_on_continuation_formals() {
    let o = run("module t;\n\
        typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;\n\
        function automatic string nm2 (input e_t a, b);\n\
          nm2 = {a.name(), \"-\", b.name()};\n\
        endfunction\n\
        initial begin if (nm2(A,C) == \"A-C\") $display(\"PASS\"); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("PASS"),
        "continuation formals:\n{o}"
    );
}

// ── `.next()`/`.first` on an enum formal (constant-foldable methods) ──
#[test]
fn next_and_first_on_formal() {
    let o = run("module t;\n\
        typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;\n\
        task automatic show (input e_t m);\n\
          e_t nx; nx = m.next();\n\
          $display(\"cur=%s nxt=%s first=%0d\", m.name(), nx.name(), m.first);\n\
        endtask\n\
        initial begin show(B); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("cur=B nxt=C first=0"),
        "next/first on formal:\n{o}"
    );
}

// ── regression: module-scope enum var `.name()` still works ──
#[test]
fn module_scope_enum_name_unchanged() {
    let o = run("module t;\n\
        typedef enum logic [1:0] { A=0, B=1 } e_t;\n\
        e_t m;\n\
        initial begin m = B; $display(\"mode=%s\", m.name()); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("mode=B"),
        "module-scope enum name:\n{o}"
    );
}

// ── regression: a body-LOCAL enum var `.name()` still works ──
#[test]
fn body_local_enum_name_unchanged() {
    let o = run("module t;\n\
        typedef enum logic [1:0] { A=0, B=1 } e_t;\n\
        task automatic run;\n\
          e_t m; m = B; $display(\"mode=%s\", m.name());\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("mode=B"),
        "body-local enum name:\n{o}"
    );
}

// ── a NON-enum formal named like a method access stays a normal (loud) error ──
// (guards that the enum-name binding didn't over-apply to a plain int formal.)
#[test]
fn non_enum_formal_dot_name_still_loud() {
    let o = run("module t;\n\
        function automatic int f (input int x);\n\
          f = x.name();\n\
        endfunction\n\
        initial begin int r; r = f(3); $display(\"r=%0d\", r); $finish; end\n\
        endmodule\n");
    assert!(is_loud(&o), "int formal .name() must stay loud:\n{o}");
}
