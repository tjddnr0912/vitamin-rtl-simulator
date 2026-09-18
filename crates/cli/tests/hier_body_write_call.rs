//! §3.b `frame-body-write-sites`, the HIERARCHICAL call: `u.fw(3)` to a function whose body
//! writes a module net.
//!
//! The local call to such a function is hoisted to a copy-out `Terminator::Call` so the
//! write runs on the statement executor (§4.5.508). A hierarchical call has the same route
//! and one obstacle: the callee's FuncId does not exist while the CALLING module is lowered.
//! The calling module now decides the route from the callee's DECLARATION (the per-module
//! fact table, `hier_body_write_callee`), lowers the actuals in its own scope sized to the
//! declared formals, seals the block with a placeholder terminator the way a hierarchical
//! task enable does, and `resolve_deferred_hier_task_call` binds the callee's per-instance
//! FuncId and the return slot once every instance exists.
//!
//! ORACLES: iverilog 13.0 (`-g2012`) and verilator 5.052 (`--binary --timing`); every value
//! asserted below was printed by both. PRE (the release binary at the parent commit) was
//! `error[VITA-E3009] … hierarchical call `u.fw(...)` is unsupported because `fw` assigns a
//! module net from its BODY` for every cell that runs now.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hbwc_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (all, out.status.success())
}

/// The design's own printed lines (a routed design carries the W4030 backend-fallback
/// warning, whose continuation lines are indented).
fn run(src: &str) -> String {
    let (all, ok) = vita(src);
    assert!(ok, "must run:\n{all}");
    all.lines()
        .filter(|l| {
            !l.trim().is_empty()
                && !l.contains("simulation ended")
                && !l.starts_with("warning[")
                && !l.starts_with("note[")
                && !l.starts_with("errors=")
                && !l.starts_with(' ')
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn loud(src: &str) -> String {
    let (all, ok) = vita(src);
    assert!(!ok, "must be refused:\n{all}");
    all
}

const CHILD: &str = r#"module ch(input logic [7:0] src, output logic [7:0] acc2);
  logic [7:0] accp;
  function automatic logic [7:0] fw(input logic [7:0] v); acc2 = v + 2; return v; endfunction
  function automatic int fi(input int v); acc2 = v + 2; return v; endfunction
  function automatic logic [7:0] fps(input logic [7:0] v); accp[3:0] = v[3:0]; return v + 1; endfunction
  function automatic logic [7:0] hf(input logic [7:0] v); return v + 1; endfunction
endmodule
"#;

fn with_child(tb: &str) -> String {
    format!("{CHILD}{tb}")
}

/// ① The row's cell, and the statement positions around it: a blocking rhs, a `$display`
/// argument, an `if` condition, a `case` selector, a `while` condition and a `for` step
/// (re-evaluated per iteration, each evaluation carrying the write), a `$sformatf`
/// argument and a `?:` arm, an edge-triggered block.
#[test]
fn a_hierarchical_call_to_a_body_writing_function_runs_in_every_statement_position() {
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 8'd1, a; logic [7:0] r; ch u(.src(src), .acc2(a));
  initial begin #1; r = u.fw(3); $display("HIER r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#
        )),
        "HIER r=3 acc2=5"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 8'd1, a; ch u(.src(src), .acc2(a));
  initial begin #1; $display("DISP %0d", u.fw(4)); $display("acc2=%0d", u.acc2); #1 $finish; end
endmodule"#
        )),
        "DISP 4\nacc2=6"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 8'd1, a; logic [7:0] r; ch u(.src(src), .acc2(a));
  initial begin #1; if (u.fw(2) > 1) r = 9; else r = 1; $display("IF r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#
        )),
        "IF r=9 acc2=4"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 1, a; int r; ch u(.src(src), .acc2(a));
  initial begin #1; case (u.fw(2)) 2: r = 20; default: r = 1; endcase $display("CASE r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#
        )),
        "CASE r=20 acc2=4"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 8'd1, a; logic [7:0] r = 0; int n = 0; ch u(.src(src), .acc2(a));
  initial begin #1; while (u.fw(n) < 3) begin n = n + 1; r = r + 1; end $display("WHILE r=%0d n=%0d acc2=%0d", r, n, u.acc2); #1 $finish; end
endmodule"#
        )),
        "WHILE r=3 n=3 acc2=5"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 1, a; int r = 0; int i; ch u(.src(src), .acc2(a));
  initial begin #1; for (i = 0; i < 3; i = u.fw(i) + 1) r = r + 1; $display("FOR r=%0d i=%0d acc2=%0d", r, i, u.acc2); #1 $finish; end
endmodule"#
        )),
        "FOR r=3 i=3 acc2=4"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 8'd1, a; logic [7:0] r; string s; ch u(.src(src), .acc2(a));
  initial begin #1; s = $sformatf("%0d", u.fw(5)); r = src ? u.fw(8) : 8'd0; $display("SF s=%s r=%0d acc2=%0d", s, r, u.acc2); #1 $finish; end
endmodule"#
        )),
        "SF s=5 r=8 acc2=10"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 1, a; int r; ch u(.src(src), .acc2(a));
  always @(posedge src) r = u.fw(9);
  initial begin #1; src = 0; #1; src = 1; #1; $display("FF r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#
        )),
        "FF r=9 acc2=11"
    );
}

/// ② `always_comb`: the block wakes on its input and each evaluation carries the write.
/// (`src` carries no initializer: with one, Rule A's conservative walk counts an actual
/// bound to an unresolvable callee as a driver and refuses `src` — pre-existing, and the
/// same with the call refused.)
#[test]
fn an_always_comb_caller_re_evaluates_the_write() {
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src, a; logic [7:0] r; ch u(.src(src), .acc2(a));
  always_comb r = u.fw(src);
  initial begin src = 1; #1; $display("COMB r=%0d acc2=%0d", r, u.acc2); src = 5; #1; $display("COMB r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#
        )),
        "COMB r=1 acc2=3\nCOMB r=5 acc2=7"
    );
}

/// ③ Shapes of the call itself: a two-level path, a part-select body write, a read to the
/// LEFT of the call (its snapshot needs no repair — the callee's formals are all inputs), a
/// nested call, two calls in one expression, a short-circuit right operand (the general
/// hoister's guarded lift), a call from a frame TASK body.
#[test]
fn the_call_shapes() {
    assert_eq!(
        run(&with_child(
            r#"module mid(input logic [7:0] s, output logic [7:0] a2); ch u2(.src(s), .acc2(a2)); endmodule
module tb; logic [7:0] src = 8'd1, a; logic [7:0] r; mid m(.s(src), .a2(a));
  initial begin #1; r = m.u2.fw(3); $display("TWO r=%0d acc2=%0d", r, m.u2.acc2); #1 $finish; end
endmodule"#
        )),
        "TWO r=3 acc2=5"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 8'd1, a; logic [7:0] r; ch u(.src(src), .acc2(a));
  initial begin #1; u.accp = 0; r = u.fps(5); $display("HIERPS r=%0d accp=%h", r, u.accp); #1 $finish; end
endmodule"#
        )),
        "HIERPS r=6 accp=05"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 1, a; int r; ch u(.src(src), .acc2(a));
  initial begin #1; r = src + u.fw(src); $display("LEFT r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#
        )),
        "LEFT r=2 acc2=3"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 1, a; int r; ch u(.src(src), .acc2(a));
  initial begin #1; r = u.fw(u.fw(1)); $display("NEST r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#
        )),
        "NEST r=1 acc2=3"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 1, a; int r; ch u(.src(src), .acc2(a));
  initial begin #1; r = u.fw(1) + u.fw(2); $display("TWOCALL r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#
        )),
        "TWOCALL r=3 acc2=4"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 1, a; int r; ch u(.src(src), .acc2(a));
  initial begin #1; r = (src > 0) && (u.fw(3) > 0); $display("SC r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#
        )),
        "SC r=1 acc2=5"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 8'd1, a; logic [7:0] r; ch u(.src(src), .acc2(a));
  task automatic tk(input logic [7:0] v); r = u.fw(v); endtask
  initial begin #1; tk(6); $display("TASK r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#
        )),
        "TASK r=6 acc2=8"
    );
}

/// ④ Widths and signs across the path: the callee's return and formal widths come from
/// its declaration folded in the INSTANCE's parameter environment (`#(.W(16))`), a signed
/// return keeps its sign in the caller's arithmetic, a narrow signed actual is
/// sign-extended into a wider formal.
#[test]
fn the_declared_widths_and_signs_cross_the_instance_path() {
    assert_eq!(
        run(
            r#"module chp #(parameter W = 8)(input logic [W-1:0] src, output logic [W-1:0] acc2);
  function automatic logic [W-1:0] fw(input logic [W-1:0] v); acc2 = v + 2; return v; endfunction
endmodule
module tb; logic [15:0] src = 16'd1, a; logic [15:0] r; chp #(.W(16)) u(.src(src), .acc2(a));
  initial begin #1; r = u.fw(16'h1ff); $display("PARAM r=%h acc2=%h bits=%0d", r, u.acc2, $bits(r)); #1 $finish; end
endmodule
"#
        ),
        "PARAM r=01ff acc2=0201 bits=16"
    );
    assert_eq!(
        run(
            r#"module chs(input logic [7:0] src, output logic signed [7:0] acc2);
  function automatic logic signed [7:0] fw(input logic signed [7:0] v); acc2 = v - 2; return v; endfunction
endmodule
module tb; logic [7:0] src = 8'd1; logic signed [7:0] a; int r; chs u(.src(src), .acc2(a));
  initial begin #1; r = u.fw(-3) * 2; $display("SIGN r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule
"#
        ),
        "SIGN r=-6 acc2=-5"
    );
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 1, a; int r; logic signed [3:0] s4 = -1; ch u(.src(src), .acc2(a));
  initial begin #1; r = u.fi(s4); $display("NARROW r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#
        )),
        "NARROW r=-1 acc2=1"
    );
}

/// ⑤ Positions the hoist declines stay LOUD, naming the position: a continuous assign
/// (both oracles run it: `CA w=1 acc2=3`), an ABSOLUTE path from a sibling (`ABS r=7
/// acc2=9`), a call inside another function's BODY (`INFN r=4 acc2=5`). Each is a §3.b
/// residue; none may reach the run.
#[test]
fn the_positions_the_hoist_declines_are_loud_by_name() {
    for (tb, tag) in [
        (
            r#"module tb; logic [7:0] src = 8'd1, a; wire [7:0] w; ch u(.src(src), .acc2(a));
  assign w = u.fw(src);
  initial begin #1; $display("CA w=%0d acc2=%0d", w, u.acc2); #1 $finish; end
endmodule"#,
            "CA w=",
        ),
        (
            r#"module sib(output logic [7:0] r); initial begin #1; r = tb.u.fw(7); end endmodule
module tb; logic [7:0] src = 8'd1, a, r; ch u(.src(src), .acc2(a)); sib s(.r(r));
  initial begin #2; $display("ABS r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#,
            "ABS r=",
        ),
        (
            r#"module tb; logic [7:0] src = 1, a; int r; ch u(.src(src), .acc2(a));
  function automatic int f2(input int v); return u.fw(v) + 1; endfunction
  initial begin #1; r = f2(3); $display("INFN r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#,
            "INFN r=",
        ),
    ] {
        let e = loud(&with_child(tb));
        assert!(e.contains("E3009"), "{tag}: {e}");
        assert!(e.contains("unsupported in this position"), "{tag}: {e}");
        assert!(
            !e.contains(tag),
            "{tag}: no output may precede the refusal:\n{e}"
        );
    }
}

/// ⑥ Rule B across the instance path: two `always_comb` processes of the parent both calling
/// `u.fw` write the child's `acc2` from two combinational drivers — verilator MULTIDRIVEN,
/// and the two oracles then disagree on the value (iverilog `acc2=3`, verilator `acc2=4`);
/// the local twin is E3001 (`frame_function_body_write.rs` ⑨), so the hierarchical pair is
/// too. The pair is `always_comb` × `always_comb` only, as the local rule: an `initial` plus
/// an `always_comb` runs (BOTH ORACLES `MD3 r=1 q=2 acc2=3`, verilator silent), and a bare
/// self-timed `always` is not an `always_comb`.
#[test]
fn two_always_comb_through_one_hierarchical_body_write_is_e3001() {
    let e = loud(&with_child(
        r#"module tb; logic [7:0] src, a; logic [7:0] r, q; ch u(.src(src), .acc2(a));
  always_comb r = u.fw(src);
  always_comb q = u.fw(src + 1);
  initial begin src = 1; #1; $display("MD2 r=%0d q=%0d acc2=%0d", r, q, u.acc2); #1 $finish; end
endmodule"#,
    ));
    assert!(e.contains("E3001"), "{e}");
    assert!(e.contains("hierarchical call `u.fw(...)`"), "{e}");
    assert!(!e.contains("MD2"), "{e}");
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src, a; logic [7:0] r, q; ch u(.src(src), .acc2(a));
  always_comb r = u.fw(src);
  initial begin src = 1; q = u.fw(2); #1; $display("MD3 r=%0d q=%0d acc2=%0d", r, q, u.acc2); #1 $finish; end
endmodule"#
        )),
        "MD3 r=1 q=2 acc2=3"
    );
}

/// ⑦ Control: a hierarchical call to a function whose body writes NOTHING outside is not
/// this route (byte-identical to PRE: `CTRL r=4 acc2=x`, iverilog's value; verilator
/// prints 0 for the never-written 4-state net).
#[test]
fn a_pure_hierarchical_call_is_untouched() {
    assert_eq!(
        run(&with_child(
            r#"module tb; logic [7:0] src = 8'd1, a; logic [7:0] r; ch u(.src(src), .acc2(a));
  initial begin #1; r = u.hf(3); $display("CTRL r=%0d acc2=%0d", r, u.acc2); #1 $finish; end
endmodule"#
        )),
        "CTRL r=4 acc2=x"
    );
}
