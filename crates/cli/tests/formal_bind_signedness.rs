//! An assignment-like context lends the actual its WIDTH, not its SIGN. All three frame
//! argument funnels passed the FORMAL's declared signedness instead.
//!
//! ## How it surfaced
//!
//! Not by anyone looking at argument binding. A different slice made `>>>`'s fill follow
//! the RESULT TYPE (§11.4.10), which meant the `AShr` arm honoured `ctx_signed` for the
//! first time. Until then `>>>` was accidentally immune to a wrong context sign, and the
//! moment it stopped being immune the latent defect became a wrong VALUE:
//!
//! ```text
//!   reg signed [7:0] b = 8'shB3;
//!   function automatic [31:0] au(input [31:0] x); au = x; endfunction
//!   au(b >>> 2)      was 4294967276 (right)  ->  44 (wrong)   exit 0
//! ```
//!
//! ⭐ That is a correct→silent-wrong of the fixing slice's own making, and the adversarial
//! soundness lens graded it BLOCKING. The rule itself was wrong all along: `au(b)` read
//! 179 where both oracles say 4294967219.
//!
//! ## The rule
//!
//! §13.5.1 makes an argument bind an assignment, so §11.8.3 lends the actual the formal's
//! WIDTH — but §11.8.1 then says an expression's signedness does not depend on the
//! left-hand side. So the actual is evaluated at the formal's width with its OWN sign, and
//! the formal's sign applies only at the store into the formal net.
//!
//! ⚠️ vitamin's own comments cited "§13.4.3 — the formal type is the assignment context"
//! in five places, including a `Kernel` trait contract. Measured against iverilog, the
//! formal's declared sign has ZERO effect: `fu16(8'shf7)` and `fs16(8'shf7)` are both
//! `fff7`. The comments are corrected.
//!
//! ## Three funnels, found one at a time
//!
//! ⚠️⚠️ THE LESSON. The fix went in at two sites. The re-review's soundness lens then ran a
//! twelve-site census and concluded "two needed the change and both got it" — and the
//! DIFFERENTIAL lens found a third by measurement:
//!
//! * `eval_core`'s `Expr::Call` arm — a frame FUNCTION's actuals.
//! * `exec/frame_call.rs`'s `split_in_binds` — a frame TASK called from a process.
//! * `state/task_frames.rs`'s `Terminator::Call` arm — a task called from INSIDE another
//!   frame body, and only when the callee is NON-suspendable. One `$display` in the callee
//!   routes it to the second funnel instead, which is why every earlier nested-call probe
//!   missed it.
//!
//! A census that enumerates sites can miss one; a sweep that varies the design finds it.
//! That is the whole argument for requiring both lenses.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn agrees_across_backends(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fbs_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let mut first: Option<String> = None;
    for be in ["native", "vm", "interp"] {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .args(["t.sv", "--backend", be])
            .current_dir(&d)
            .output()
            .expect("run vita");
        let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&out.stderr));
        match &first {
            None => first = Some(s),
            Some(f) => assert_eq!(f, &s, "backend {be} diverged"),
        }
    }
    let _ = std::fs::remove_dir_all(&d);
    first.unwrap()
}

/// ⭐ THE THIRD FUNNEL, with the controls that isolate it. `A` is the nested call to a
/// NON-suspendable task — the only cell that was wrong after the first two sites were
/// fixed. `B` is its suspendable twin (one `$display` in the callee moves it to funnel
/// two), `C` the function twin, `D` the top-level twin: all three were already right, so
/// a regression here moves `A` alone.
///
/// `E` is the plain extension at the same site with no shift involved — the §11.8.3 half,
/// which was wrong in BOTH earlier binaries.
///
/// iverilog: `A=fffd B=fffd C=fffd D=fffd E=fff7`.
#[test]
fn a_nested_call_to_a_subset_task_takes_the_actuals_signedness() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         reg signed [7:0] sb; logic [15:0] a, b, c, d, e1;\n  \
         task automatic q_sub(input [15:0] x, output [15:0] y); y = x; endtask\n  \
         task automatic q_sus(input [15:0] x, output [15:0] y); y = x; $display(\".\"); endtask\n  \
         task automatic viaSub(input signed [7:0] p, output [15:0] r); q_sub(p >>> 2, r); endtask\n  \
         task automatic viaSus(input signed [7:0] p, output [15:0] r); q_sus(p >>> 2, r); endtask\n  \
         function automatic [15:0] direct(input signed [7:0] p); direct = p >>> 2; endfunction\n  \
         task automatic inner(input [15:0] x, output [15:0] y); y = x; endtask\n  \
         task automatic outer(input signed [7:0] q, output [15:0] r); inner(q, r); endtask\n  \
         initial begin\n    sb = 8'shf7;\n    \
         viaSub(sb, a); viaSus(sb, b); c = direct(sb); q_sub(sb >>> 2, d); outer(sb, e1);\n    \
         $display(\"A=%h B=%h C=%h D=%h E=%h\", a, b, c, d, e1);\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(
        out.contains("A=fffd B=fffd C=fffd D=fffd E=fff7"),
        "the nested subset-task call is the differentiator; B/C/D are its controls:\n{out}"
    );
}

/// ⭐ THE RULE ITSELF, stated as a pair: the formal's declared sign must make NO
/// difference. Each row is the same actual through an unsigned and a signed formal, and
/// the two must agree — which is what iverilog does and what the old code did not.
///
/// `S1`/`S2` a signed literal: `fff7` through both.
/// `U1`/`U2` an unsigned literal: `00f7` through both.
/// `B1`/`B2` a signed variable: `ffb3` through both.
#[test]
fn the_formals_declared_sign_makes_no_difference() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  reg signed [7:0] b; reg [7:0] u;\n  \
         function automatic [15:0] fu(input [15:0] x); fu = x; endfunction\n  \
         function automatic [15:0] fs(input signed [15:0] x); fs = x; endfunction\n  \
         initial begin b = 8'shb3; u = 8'hf7;\n    \
         $display(\"S1=%h S2=%h U1=%h U2=%h B1=%h B2=%h\",\n      \
         fu(8'shf7), fs(8'shf7), fu(8'hf7), fs(8'hf7), fu(b), fs(b));\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(
        out.contains("S1=fff7 S2=fff7 U1=00f7 U2=00f7 B1=ffb3 B2=ffb3"),
        "each pair must agree — the formal lends width, not sign:\n{out}"
    );
}

/// ⚠️ THE WIDTH HALF MUST NOT HAVE MOVED. Only the sign argument changed, so a formal
/// NARROWER than the actual must still truncate exactly as before.
///
/// iverilog: `N1=3 N2=3 N3=f7 N4=b3`.
#[test]
fn the_formals_width_still_truncates_a_wider_actual() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  reg signed [7:0] b; reg [15:0] w;\n  \
         function automatic [3:0] f4(input [3:0] x); f4 = x; endfunction\n  \
         function automatic [7:0] f8(input [7:0] x); f8 = x; endfunction\n  \
         initial begin b = 8'shb3; w = 16'hbeef;\n    \
         $display(\"N1=%0d N2=%0d N3=%h N4=%h\", f4(b), f4(8'shb3), f8(16'h00f7), f8(b));\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(out.contains("N1=3 N2=3 N3=f7 N4=b3"), "{out}");
}

/// The `>>>` interaction that started it, at all three funnels plus the inline path, which
/// was correct throughout. iverilog: all four `ffffffec`.
#[test]
fn a_shift_as_an_actual_keeps_its_own_sign_at_every_funnel() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  reg signed [7:0] b; logic [31:0] r1, r2;\n  \
         function automatic [31:0] fa(input [31:0] x); fa = x; endfunction\n  \
         function [31:0] fi(input [31:0] x); fi = x; endfunction\n  \
         task automatic ta(input [31:0] x, output [31:0] y); y = x; endtask\n  \
         task automatic nest(input signed [7:0] p, output [31:0] y); ta(p >>> 2, y); endtask\n  \
         initial begin b = 8'shB3;\n    ta(b >>> 2, r1); nest(b, r2);\n    \
         $display(\"F=%h I=%h T=%h N=%h\", fa(b >>> 2), fi(b >>> 2), r1, r2);\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(
        out.contains("F=ffffffec I=ffffffec T=ffffffec N=ffffffec"),
        "{out}"
    );
}
