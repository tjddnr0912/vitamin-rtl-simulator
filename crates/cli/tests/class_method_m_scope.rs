//! `%m` inside a CLASS METHOD names the class and the method (`top.C.show`, IEEE
//! §21.2.1 — the method's declaring scope), and the `[in …]` context of a runtime
//! diagnostic raised inside a subroutine body names the same scope `%m` prints.
//! ROADMAP §2 🆕 N residue (review of §4.5.435).
//!
//! The class table is global, so a method's frame name was the bare method name and
//! the engine printed `<process scope>.show`; the body's block-label chain was rooted
//! at the `$class$C$show` STORAGE segment (`$class$C$show.lb` leaked); and the
//! diagnostic context was the frame's storage prefix (`[in top.$func$t]`) or that
//! same class segment (`[in $class$C$show]`). Elaborate now records `<class>.<method>`
//! as the frame name (the engine prefixes the calling process's scope), keeps the
//! class body's chain relative, and records the DECLARING scope as the context of a
//! statement inside a frame / inlined task body (a class method records a
//! `.<class>.<method>` marker the engine completes with the process scope).
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.050 (28 cells);
//! the lines are the oracles' output, copied. Where the two split (a label inside a
//! class method — iverilog `top.C.show`, verilator `top.C.show.lb`; a virtual call
//! — iverilog names the BASE class) vita keeps the §4.5.435 label policy
//! (verilator's spelling) and the dispatched method's own class (hand-IEEE §8.20).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_backend(src: &str, backend: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cmms_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg(backend)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

fn prints_all(src: &str, want: &[&str]) {
    for b in ["native", "interp", "vm"] {
        let (out, code) = run_backend(src, b);
        assert_eq!(code, Some(0), "[{b}] exit\n{out}");
        let got: Vec<&str> = out.lines().filter(|l| l.starts_with("M=")).collect();
        assert_eq!(got, want, "[{b}]\n{out}");
    }
}

/// The `[in …]` context of every `error[VITA-E4003]` line, on every backend.
fn error_contexts(src: &str, want: &[&str]) {
    for b in ["native", "interp", "vm"] {
        let (out, code) = run_backend(src, b);
        assert_eq!(code, Some(1), "[{b}] exit\n{out}");
        let got: Vec<&str> = out
            .lines()
            .filter(|l| l.contains("error[VITA-E4003]"))
            .map(|l| {
                let s = l.find("[in ").expect("context") + 4;
                let e = l[s..].find(']').unwrap() + s;
                &l[s..e]
            })
            .collect();
        assert_eq!(got, want, "[{b}]\n{out}");
    }
}

fn top(body: &str) -> String {
    format!("`timescale 1ns/1ns\nmodule top;\n{body}\n  initial #5 $finish;\nendmodule\n")
}

#[test]
fn a_class_method_names_its_class() {
    prints_all(
        &top("  class C; task show(); $display(\"M=%m\"); endtask endclass\n  C c;\n  initial begin c = new; c.show(); end"),
        &["M=top.C.show"],
    );
    // `$sformatf`, the constructor, a class declared in a sub-module (one line per
    // instance — iverilog's `top.u1` / `top.u2`; verilator prints `top.u1` twice)
    prints_all(
        &top("  class C; task show(); string s; s = $sformatf(\"%m\"); $display(\"M=%s\", s); endtask endclass\n  C c;\n  initial begin c = new; c.show(); end"),
        &["M=top.C.show"],
    );
    prints_all(
        &top("  class C; function new(); $display(\"M=%m\"); endfunction endclass\n  C c;\n  initial begin c = new; end"),
        &["M=top.C.new"],
    );
    prints_all(
        "`timescale 1ns/1ns\nmodule sub;\n  class C; task show(); $display(\"M=%m\"); endtask endclass\n  C c;\n  \
         initial begin c = new; c.show(); end\nendmodule\nmodule top;\n  sub u1(); sub u2();\n  initial #5 $finish;\nendmodule\n",
        &["M=top.u1.C.show", "M=top.u2.C.show"],
    );
    // a virtual call names the DISPATCHED method's class (verilator; iverilog prints
    // the base class — its virtual dispatch is known-wrong, hand-IEEE §8.20)
    prints_all(
        &top("  class B; virtual task show(); $display(\"M=%m\"); endtask endclass\n  class D extends B; virtual task show(); $display(\"M=%m\"); endtask endclass\n  B b; D d;\n  initial begin d = new; b = d; b.show(); end"),
        &["M=top.D.show"],
    );
    // the method body's own labels follow (verilator's spelling; iverilog drops a
    // label inside a subroutine) — the `$class$C$show` storage segment never shows
    prints_all(
        &top("  class C; task show(); begin : lb $display(\"M=%m\"); end endtask endclass\n  C c;\n  initial begin c = new; c.show(); end"),
        &["M=top.C.show.lb"],
    );
    prints_all(
        &top("  class C; task show(); begin : a begin : b $display(\"M=%m\"); end end endtask endclass\n  C c;\n  initial begin c = new; c.show(); end"),
        &["M=top.C.show.a.b"],
    );
    // control: a module task is unchanged
    prints_all(
        &top("  task show(); $display(\"M=%m\"); endtask\n  initial show();"),
        &["M=top.show"],
    );
}

#[test]
fn a_diagnostic_inside_a_subroutine_body_names_the_declaring_scope() {
    // a frame task (`[in top.$func$t]` before), a static (inlined) task (`[in top]`
    // before), from a generate-block process, and a class method (`[in $class$C$show]`)
    error_contexts(
        &top("  task automatic t(); $error(\"boom\"); endtask\n  initial t();"),
        &["top.t"],
    );
    error_contexts(
        &top("  task t(); $error(\"boom\"); endtask\n  initial t();"),
        &["top.t"],
    );
    error_contexts(
        &top("  task automatic t(); $error(\"boom\"); endtask\n  generate if (1) begin : gi initial t(); end endgenerate"),
        &["top.t"],
    );
    error_contexts(
        &top("  task t(); $error(\"boom\"); endtask\n  generate if (1) begin : gi initial t(); end endgenerate"),
        &["top.t"],
    );
    error_contexts(
        &top("  class C; task show(); $error(\"boom\"); endtask endclass\n  C c;\n  initial begin c = new; c.show(); end"),
        &["top.C.show"],
    );
    error_contexts(
        "`timescale 1ns/1ns\nmodule sub;\n  class C; task show(); $error(\"boom\"); endtask endclass\n  C c;\n  \
         initial begin c = new; c.show(); end\nendmodule\nmodule top;\n  sub u1();\n  initial #5 $finish;\nendmodule\n",
        &["top.u1.C.show"],
    );
    // after a labelled block closed, the body scope again; inside one, the label
    error_contexts(
        &top("  task automatic t(); begin : ib $display(\"x\"); end $error(\"boom\"); endtask\n  initial t();"),
        &["top.t"],
    );
    error_contexts(
        &top("  task automatic t(); begin : ib $error(\"boom\"); end endtask\n  initial t();"),
        &["top.t.ib"],
    );
    // control: a process's own context is unchanged (the instance path)
    error_contexts(&top("  initial $error(\"boom\");"), &["top"]);
}
