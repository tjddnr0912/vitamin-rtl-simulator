//! `%m` inside a subroutine body names the scope the subroutine is DECLARED in
//! (IEEE §21.2.1), not the process that called it — ROADMAP §2 🆕 N, review A F2
//! of §4.5.426 (the dynamic-chain class).
//!
//! vita rendered the EXECUTING process's scope plus every frame on the call path:
//! a module task called from a generate-block process printed `top.gi.t`, a
//! recursion `top.f.f.f`, a task called from another task `top.b.a`, a function
//! called from a generate loop `top.gl[0].f` — iverilog 13.0 and verilator 5.050
//! print `top.t` / `top.f` / `top.a` / `top.f` for all of them. Elaborate now roots
//! the recorded scope chain at the declaring instance (`block_scope_root`, an
//! absolute `.top.t…` string) for an inlined body, and `func_names` carries the
//! frame's own `<inst>.<name>`; the engine renders either verbatim.
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.050 (43 cells);
//! where the two disagree on a label inside the body (iverilog drops a named block
//! inside a task, verilator keeps it) the line says which one vita follows.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_backend(src: &str, backend: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mdecl_{}_{n}", std::process::id()));
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
        let got: Vec<&str> = out.lines().filter(|l| l.starts_with("D=")).collect();
        assert_eq!(got, want, "[{b}]\n{out}");
    }
}

/// A module `top` with `items` and one `initial #5 $finish`.
fn top(items: &str) -> String {
    format!("`timescale 1ns/1ns\nmodule top;\n{items}\n  initial #5 $finish;\nendmodule\n")
}

const GI_CALL: &str = "generate if (1) begin : gi initial #1 t(); end endgenerate";

#[test]
fn a_task_called_from_a_generate_block_names_the_module() {
    // n01 (inlined) / n14 (automatic = frame) · both oracles `top.t` (PRE `top.gi.t`)
    prints_all(
        &top(&format!(
            "  task t; $display(\"D=%m\"); endtask\n  {GI_CALL}"
        )),
        &["D=top.t"],
    );
    prints_all(
        &top(&format!(
            "  task automatic t; $display(\"D=%m\"); endtask\n  {GI_CALL}"
        )),
        &["D=top.t"],
    );
    // n15 · through another task; n34 · from a fork arm inside the block
    prints_all(
        &top(
            "  task t; $display(\"D=%m\"); endtask\n  task u; t(); endtask\n  \
             generate if (1) begin : gi initial #1 u(); end endgenerate",
        ),
        &["D=top.t"],
    );
    prints_all(
        &top(
            "  task t; $display(\"D=%m\"); endtask\n  generate if (1) begin : gi initial begin \
             #1 fork begin : fa t(); end join end end endgenerate",
        ),
        &["D=top.t"],
    );
    // n39 · from every iteration of a generate loop; n08 · a function likewise
    prints_all(
        &top(
            "  task t; $display(\"D=%m\"); endtask\n  generate for (genvar i = 0; i < 2; i++) \
             begin : gl initial #1 t(); end endgenerate",
        ),
        &["D=top.t", "D=top.t"],
    );
    prints_all(
        &top(
            "  function int f(int n); $display(\"D=%m\"); return n; endfunction\n  generate for \
             (genvar i = 0; i < 2; i++) begin : gl initial #1 void'(f(i)); end endgenerate",
        ),
        &["D=top.f", "D=top.f"],
    );
    // n33 · two instances: each names ITS instance, without the generate segment
    let src = "`timescale 1ns/1ns\nmodule ch; task automatic t; $display(\"D=%m\"); endtask\n  \
               generate if (1) begin : gi initial #1 t(); end endgenerate endmodule\n\
               module top; ch u1(); ch u2(); initial #5 $finish; endmodule\n";
    prints_all(src, &["D=top.u1.t", "D=top.u2.t"]);
}

#[test]
fn a_frame_names_its_declaring_scope_not_the_call_chain() {
    // n02 · a recursion prints the same scope at every level (PRE `top.f.f.f`)
    prints_all(
        &top(
            "  function automatic int f(int n); $display(\"D=%m\"); return n <= 0 ? 0 : f(n-1); \
             endfunction\n  initial #1 void'(f(2));",
        ),
        &["D=top.f", "D=top.f", "D=top.f"],
    );
    // n11 · a task called from another task (PRE `top.b.a`); n38 · through the
    // caller's own named block, from a generate block (PRE `top.gi.b.a`)
    prints_all(
        &top(
            "  task automatic a; $display(\"D=%m\"); endtask\n  task automatic b; a(); endtask\n  \
             initial #1 b();",
        ),
        &["D=top.a"],
    );
    prints_all(
        &top(
            "  task automatic a; $display(\"D=%m\"); endtask\n  task automatic b; begin : bb a(); \
             end endtask\n  generate if (1) begin : gi initial #1 b(); end endgenerate",
        ),
        &["D=top.a"],
    );
    // n35 / n28 · called from inside a named block of the caller (PRE == POST)
    prints_all(
        &top("  task automatic t; $display(\"D=%m\"); endtask\n  initial begin : blk #1 t(); end"),
        &["D=top.t"],
    );
    prints_all(
        &top("  task t; $display(\"D=%m\"); endtask\n  initial begin : blk #1 t(); end"),
        &["D=top.t"],
    );
}

#[test]
fn a_named_block_inside_the_body_follows_the_declaring_scope() {
    // n17 (inlined) / n18 (frame) · verilator `top.t.ib` (iverilog drops the label:
    // `top.t`); PRE `top.gi.t.ib`
    prints_all(
        &top(&format!(
            "  task t; begin : ib $display(\"D=%m\"); end endtask\n  {GI_CALL}"
        )),
        &["D=top.t.ib"],
    );
    prints_all(
        &top(&format!(
            "  task automatic t; begin : ib $display(\"D=%m\"); end endtask\n  {GI_CALL}"
        )),
        &["D=top.t.ib"],
    );
    // n37 · a recursion's label (verilator `top.f.ib` twice; PRE `top.f.f.ib`)
    prints_all(
        &top(
            "  function automatic int f(int n); begin : ib $display(\"D=%m\"); end return n <= 0 \
             ? 0 : f(n-1); endfunction\n  initial #1 void'(f(1));",
        ),
        &["D=top.f.ib", "D=top.f.ib"],
    );
    // n24 · a fork label inside the body (verilator `top.t.fa`)
    prints_all(
        &top(&format!(
            "  task t; fork begin : fa $display(\"D=%m\"); end join endtask\n  {GI_CALL}"
        )),
        &["D=top.t.fa"],
    );
}

#[test]
fn sformatf_strobe_and_error_inside_the_body_agree() {
    // n21 · `$sformatf("%m")` inlined; n31 · in a frame under a label (verilator)
    prints_all(
        &top(&format!(
            "  task t; $display(\"D=%s\", $sformatf(\"%m\")); endtask\n  {GI_CALL}"
        )),
        &["D=top.t"],
    );
    prints_all(
        &top(&format!(
            "  task automatic t; begin : ib $display(\"D=%s\", $sformatf(\"%m\")); end endtask\n  \
             {GI_CALL}"
        )),
        &["D=top.t.ib"],
    );
    // n22 · a strobe registered inside the body keeps the declaring scope
    // (iverilog `top.t`; verilator `top`)
    prints_all(
        &top(&format!(
            "  task t; $strobe(\"D=%m\"); endtask\n  {GI_CALL}"
        )),
        &["D=top.t"],
    );
    // review B B-1 · a `$monitor` registered inside a FRAME body with no named block
    // captures the declaring scope too (iverilog `top.t`; PRE `top.gi`)
    prints_all(
        &top(
            "  logic [1:0] x = 0;\n  task automatic t; $monitor(\"D=%m x=%0d\", x); endtask\n  \
             generate if (1) begin : gi initial #1 t(); end endgenerate\n  initial begin #2 x = 1; \
             #1 x = 2; end",
        ),
        &["D=top.t x=0", "D=top.t x=1", "D=top.t x=2"],
    );
    // n23 · `$error` inside the body: verilator "Assertion failed in top.t: D=top.t"
    let src = top(&format!("  task t; $error(\"D=%m\"); endtask\n  {GI_CALL}"));
    for b in ["native", "interp", "vm"] {
        let (out, _) = run_backend(&src, b);
        assert!(out.contains("E-RUN-USER-ERROR: D=top.t "), "[{b}]\n{out}");
    }
}

#[test]
fn scopes_that_were_right_stay_byte_identical() {
    // n12 / n29 / n30 · an instance's own task names the instance path, including a
    // generate scope the INSTANCE sits in (both oracles)
    let src = "`timescale 1ns/1ns\nmodule ch; task t; $display(\"D=%m\"); endtask initial #1 t(); \
               endmodule\nmodule top; ch u1(); ch u2(); initial #5 $finish; endmodule\n";
    prints_all(src, &["D=top.u1.t", "D=top.u2.t"]);
    let src = "`timescale 1ns/1ns\nmodule ch; task t; $display(\"D=%m\"); endtask initial #1 t(); \
               endmodule\nmodule top; generate if (1) begin : gi ch u(); end endgenerate initial \
               #5 $finish; endmodule\n";
    prints_all(src, &["D=top.gi.u.t"]);
    let src =
        "`timescale 1ns/1ns\nmodule ch; task automatic t; $display(\"D=%m\"); endtask initial \
               #1 t(); endmodule\nmodule top; generate for (genvar i = 0; i < 2; i++) begin : gl \
               ch u(); end endgenerate initial #5 $finish; endmodule\n";
    prints_all(src, &["D=top.gl[0].u.t", "D=top.gl[1].u.t"]);
    // n36 · a process in the generate block itself; n40 · a strobe outside any body
    // keeps the registering block; n42 · a function in a continuous assign
    prints_all(
        &top("  generate if (1) begin : gi initial #1 $display(\"D=%m\"); end endgenerate"),
        &["D=top.gi"],
    );
    prints_all(
        &top(
            "  task t; $display(\"D=%m\"); endtask\n  initial begin #1 $strobe(\"D=%m\"); t(); end\n  \
             initial begin : mb #2 $strobe(\"D=%m\"); end",
        ),
        &["D=top.t", "D=top", "D=top.mb"],
    );
    prints_all(
        &top(
            "  function int f(int n); $display(\"D=%m\"); return n; endfunction\n  wire [31:0] w = \
             f(3);\n  generate if (1) begin : gi wire [31:0] v = f(4); end endgenerate\n  initial \
             #1 $display(\"D=%0d %0d\", w, gi.v);",
        ),
        &["D=top.f", "D=top.f", "D=3 4"],
    );
}
