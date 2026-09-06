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

/// §4.5.440 (§2 🆕 N residue): a class method's scope is its class's, which lives
/// in the declaring INSTANCE — not in the calling generate block, not under the
/// frames of the call chain. iverilog prints `top.u1.C.show` / `top.gg.u3.C.show`
/// from a process in `top.u1.gi`, from a `generate for` element, from a named
/// block, a `fork`, an automatic task frame and a static task; verilator prints
/// the FIRST instance (`top.u1`) for every one and contradicts itself on the
/// second instance, so the multi-instance cells pin iverilog. vita printed the
/// calling scope (`top.u1.gi.C.show`, `top.u1.ta.C.show`).
#[test]
fn a_class_method_called_from_a_generate_scope_or_a_frame_names_its_instance() {
    const SUB: &str = "module sub;
  class C;
    task show(); $display(\"M=%m\"); endtask
    task lab(); begin : lb $display(\"M=%m\"); end endtask
    task err(); $error(\"E\"); endtask
  endclass
  C c; initial c = new;
  task automatic ta; c.show(); endtask
  task st; c.show(); endtask
  initial #1 ta();
  generate if (1) begin : gi
    initial #2 c.show();
    initial #3 begin : nb c.show(); end
    initial #4 fork c.show(); join
    initial #5 c.lab();
    initial #6 st();
    initial #7 ta();
  end endgenerate
  generate for (genvar g = 0; g < 2; g++) begin : gl
    initial #(8+g) c.show();
  end endgenerate
endmodule
";
    let src = format!(
        "`timescale 1ns/1ns\n{SUB}module top;\n  sub u1();\n  generate if (1) begin : gg sub u3(); end endgenerate\n  initial #20 $finish;\nendmodule\n"
    );
    // the same cell per time step, once per instance (declaration order)
    let mut want: Vec<String> = Vec::new();
    for _ in 0..4 {
        want.push("M=top.u1.C.show".into());
        want.push("M=top.gg.u3.C.show".into());
    }
    // `lab`: the method body's own label follows (verilator; iverilog drops it)
    want.push("M=top.u1.C.lab.lb".into());
    want.push("M=top.gg.u3.C.lab.lb".into());
    for _ in 0..4 {
        want.push("M=top.u1.C.show".into());
        want.push("M=top.gg.u3.C.show".into());
    }
    let want: Vec<&str> = want.iter().map(String::as_str).collect();
    prints_all(&src, &want);
    // single instance: `top.C.show` from `top.gi` (both oracles)
    prints_all(
        &top("  class C; task show(); $display(\"M=%m\"); endtask endclass\n  C c; initial c = new;\n  generate if (1) begin : gi initial #1 c.show(); end endgenerate\n  task automatic ta; c.show(); endtask\n  initial #2 ta();"),
        &["M=top.C.show", "M=top.C.show"],
    );
    // a `$error` inside the method called from the generate block: `[in top.u1.C.err]`
    // (iverilog `Scope: top.u1.C.err`)
    error_contexts(
        "`timescale 1ns/1ns\nmodule sub;\n  class C; task err(); $error(\"E\"); endtask endclass\n  C c; initial c = new;\n  generate if (1) begin : gi initial #1 c.err(); end endgenerate\nendmodule\nmodule top;\n  sub u1(); sub u2();\n  initial #5 $finish;\nendmodule\n",
        &["top.u1.C.err", "top.u2.C.err"],
    );
    // control: a process's own `%m` in the generate block is unchanged
    prints_all(
        &top("  generate if (1) begin : gi initial $display(\"M=%m\"); end endgenerate"),
        &["M=top.gi"],
    );
}

/// The instance prefix rides the `.velab` trailer (format 31): vcmp → velab → vrun
/// prints the same `M=` lines as the one-shot run (the STAGED-DROP hazard).
#[test]
fn staged_vrun_prints_the_same_class_method_scope() {
    let src = "`timescale 1ns/1ns\nmodule sub;\n  class C; task show(); $display(\"M=%m\"); endtask endclass\n  C c; initial c = new;\n  task automatic ta; c.show(); endtask\n  generate if (1) begin : gi initial #1 c.show(); initial #2 ta(); end endgenerate\nendmodule\nmodule top;\n  sub u1(); sub u2();\n  initial #5 $finish;\nendmodule\n";
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cmms_staged_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("d.sv"), src).unwrap();
    let run = |args: &[&str]| -> (String, i32) {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .args(args)
            .current_dir(&d)
            .output()
            .expect("spawn vita");
        (
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
            out.status.code().unwrap_or(-1),
        )
    };
    let m_lines = |s: &str| -> Vec<String> {
        s.lines()
            .filter(|l| l.starts_with("M="))
            .map(str::to_string)
            .collect()
    };
    let (one, code) = run(&["d.sv"]);
    assert_eq!(code, 0, "one-shot:\n{one}");
    let (c, code) = run(&["vcmp", "d.sv", "-o", "d.vu"]);
    assert_eq!(code, 0, "vcmp:\n{c}");
    let (e, code) = run(&["velab", "d.vu", "-o", "d.velab"]);
    assert_eq!(code, 0, "velab:\n{e}");
    let (staged, code) = run(&["vrun", "d.velab"]);
    assert_eq!(code, 0, "vrun:\n{staged}");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        m_lines(&one),
        vec![
            "M=top.u1.C.show",
            "M=top.u2.C.show",
            "M=top.u1.C.show",
            "M=top.u2.C.show"
        ]
    );
    assert_eq!(
        m_lines(&one),
        m_lines(&staged),
        "one-shot vs staged diverged"
    );
}

/// Review (§4.5.441 B1): a class method evaluated at a `$strobe` / `$monitor` flush
/// — outside any process body — takes the REGISTERING process's instance, carried
/// on the capture beside its scope. A second module with no class C runs its
/// processes after the registration; the stale instance was `top.o2` (verilator
/// `top.u1.C.val`; iverilog cannot take a function call in `$strobe`).
#[test]
fn a_class_method_in_a_strobe_or_monitor_argument_names_the_registering_instance() {
    let src = "`timescale 1ns/1ns\nmodule sub;\n  logic [3:0] x = 0;\n  class C; function int val(); $display(\"M=%m\"); return 1; endfunction endclass\n  C c; initial c = new;\n  initial begin #1 x = 1; $strobe(\"S=%m v=%0d\", c.val()); end\n  initial begin #2 $monitor(\"N=%0d\", c.val() + x); #1 x = 2; #1 x = 3; end\nendmodule\nmodule other; initial #1 $display(\"other\"); initial #3 $display(\"other\"); endmodule\nmodule top;\n  sub u1(); other o1(); other o2();\n  initial #6 $finish;\nendmodule\n";
    for b in ["native", "interp", "vm"] {
        let (out, code) = run_backend(src, b);
        assert_eq!(code, Some(0), "[{b}] exit\n{out}");
        let m: Vec<&str> = out.lines().filter(|l| l.starts_with("M=")).collect();
        assert!(m.len() >= 4, "[{b}]\n{out}");
        assert!(
            m.iter().all(|l| *l == "M=top.u1.C.val"),
            "[{b}] every evaluation names u1:\n{out}"
        );
        assert!(out.contains("S=top.u1 v=1"), "[{b}]\n{out}");
    }
}
