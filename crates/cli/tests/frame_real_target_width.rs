//! A `real` / `realtime` TARGET on the FRAME route is not a §11.6.1 width
//! context — the mirror image of §4.5.491, which closed the inline route.
//!
//! IEEE §6.12 / §11.8.1: a real has no bit width, so the integral right-hand side
//! of an assignment to one is SELF-determined and its RESULT converts. The engine
//! already keyed that on the destination's `NetKind::Real` in the three
//! module-process evaluators (`width::lvalue_targets_real`), but the FRAME
//! evaluator (`frame_rhs_value_with`) asked `lvalue_width` unguarded, and the
//! return slot of a real function was a 64-bit `Reg`, invisible to the guard
//! anyway. So `function automatic real f; f = a8 * b8;` with `a8 = b8 = 8'hFF`
//! printed `65025.000000` where both oracles print `1.000000` — and so did a real
//! body local, an `output real` formal, `return a8 * b8`, and an `input real`
//! formal bound at its 64-bit slot width (`width::formal_lends_width`, one
//! spelling for the three argument-binding sites).
//!
//! Two consequences of the return slot being a real net are pinned too: a
//! real-returning call is REAL to the width table, so `f() / 2` divides as reals
//! (PRE: `0.000000`), and the frame slot write applies the same real↔int
//! coercion the module write funnel does (`coerce_real_frame`), so a real local
//! or return holds a real, not the integral value it was handed.
//!
//! ORACLES: iverilog 13.0 (-g2012) and verilator 5.052 (--binary --timing). Every
//! value asserted below was measured in BOTH unless the docstring says otherwise
//! (iverilog rejects an `output` formal on a FUNCTION, so those cells are
//! verilator's). PRE values are from a release binary built at the parent commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_frtw_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_vita"));
    for a in args {
        c.arg(a);
    }
    let out = c
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let so = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::remove_dir_all(&d);
    so
}

fn run(src: &str) -> String {
    run_args(src, &[])
}

/// The same source on all three backends; diverging is the failure.
fn agrees_across_backends(src: &str) -> String {
    let mut first: Option<String> = None;
    for be in ["native", "vm", "interp"] {
        let s = run_args(src, &["--backend", be]);
        match &first {
            None => first = Some(s),
            Some(f) => assert_eq!(f, &s, "backend {be} diverged"),
        }
    }
    first.unwrap()
}

/// ① THE HEADLINE: every frame site that carries a real target, on all three
/// backends. Return-by-name, `return e`, a body local, a `realtime` return, an
/// `input real` formal, a task's `input real` and `output real`.
#[test]
fn a_real_frame_target_does_not_widen_its_rhs() {
    let o = agrees_across_backends(
        r#"module t;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  real r;
  function automatic real     armul; armul = a8 * b8; endfunction
  function automatic real     aradd; aradd = a8 + b8; endfunction
  function automatic real     fr;    return a8 * b8;  endfunction
  function automatic real     fl;    real l; l = a8 * b8; return l; endfunction
  function automatic realtime ft;    ft = a8 * b8;    endfunction
  function automatic real     fi(input real i); return i; endfunction
  function automatic real     fn2(input [7:0] k); return k * k; endfunction
  function          real      srmul; return a8 * b8; endfunction
  task automatic tr(input real i); $display("TR=%f", i); endtask
  task automatic to(output real o); o = a8 * b8; endtask
  task automatic trt(input realtime i); $display("TRT=%f", i); endtask
  initial begin
    $display("M=%f A=%f R=%f L=%f T=%f I=%f N=%f S=%f",
             armul(), aradd(), fr(), fl(), ft(), fi(a8 * b8), fn2(255), srmul());
    r = fr(); $display("r=%f", r);
    tr(a8 * b8); to(r); $display("to=%f", r); trt(a8 * b8);
    #1 $finish;
  end
endmodule
"#,
    );
    // PRE: `M=65025.000000 A=510.000000 R=65025.000000 L=65025.000000
    // T=65025.000000 I=65025.000000 N=65025.000000 S=65025.000000`, then
    // `r=65025.000000`, `TR=65025.000000`, `to=65025.000000`, `TRT=65025.000000`.
    // (`srmul` is a STATIC function whose `return` pre-frames it onto this route.)
    assert_eq!(
        o,
        "M=1.000000 A=254.000000 R=1.000000 L=1.000000 T=1.000000 I=1.000000 N=1.000000 S=1.000000\n\
         r=1.000000\n\
         TR=1.000000\n\
         to=1.000000\n\
         TRT=1.000000"
    );
}

/// ② Every context-determined operator folds at its OWN width under a real
/// target, and the controls that were already right stay right: a real literal
/// return, a real body read-back, a real operand in the rhs (the integral
/// sub-expression is self-determined there already), a 64-bit operand, a signed
/// product against a 32-bit literal, a division, a comparison and a concat.
#[test]
fn every_operator_is_self_determined_under_a_real_target() {
    let o = run(r#"module t;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  logic signed [7:0] sa = -3;
  logic [63:0] w64 = 64'hFFFF_FFFF_FFFF_FFFF;
  real r = 2.5;
  function automatic real fh;   return 1.5;            endfunction
  function automatic real fm;   fm = 1.5; fm = fm * 3; endfunction
  function automatic real fx;   return a8 * b8 + 0.5;  endfunction
  function automatic real fneg; fneg = -b8;            endfunction
  function automatic real fsh;  fsh = a8 << 4;         endfunction
  function automatic real f64;  f64 = w64 + 1;         endfunction
  function automatic real fs;   fs = sa * 100;         endfunction
  function automatic real fs1;  fs1 = sa * 2;          endfunction
  function automatic real fd;   fd = a8 / 2;           endfunction
  function automatic real fcmp; fcmp = (a8 * b8) > 100; endfunction
  function automatic real fcat; fcat = {a8, b8};       endfunction
  function automatic real fmix; fmix = a8 * b8 + r;    endfunction
  function automatic real fine(input real i); return i; endfunction
  initial begin
    $display("%f %f %f %f %f %f", fh(), fm(), fx(), fneg(), fsh(), f64());
    $display("%f %f %f %f %f %f %f", fs(), fs1(), fd(), fcmp(), fcat(), fmix(), fine(-b8));
    #1 $finish;
  end
endmodule
"#);
    // PRE: `fneg` −255 (a 64-bit `-b8`), `fsh` 4080, `fine(-b8)` −255; the rest
    // were already both oracles' values.
    assert_eq!(
        o,
        "1.500000 4.500000 1.500000 1.000000 240.000000 0.000000\n\
         -300.000000 -6.000000 127.000000 1.000000 65535.000000 3.500000 1.000000"
    );
}

/// ③ A real-returning CALL is real to the width table: `f() / 2` is a real
/// division, `f ** 2` a real power (PRE: `nan`), and a real return slot holds a
/// REAL — `f = a8 * b8; f = f / 4;` divided as integers before the frame write
/// applied the module funnel's real coercion.
#[test]
fn a_real_returning_call_is_real_and_its_slot_holds_a_real() {
    let o = run(r#"module t;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  int i; bit [7:0] b; real r; logic c = 1;
  function automatic real fr; return a8 * b8; endfunction
  function automatic real fx; return a8 * b8 + 0.5; endfunction
  function automatic real fh; return 2.5; endfunction
  function automatic real fq; fq = a8 * b8; fq = fq / 4; endfunction
  function automatic real fs(input real x); return x * 2; endfunction
  initial begin
    $display("div=%f half=%f fq=%f fs=%f", fr() / 2, fh() / 2, fq(), fs(a8 * b8));
    $display("d=%0d", fx());
    i = fx() + 1; b = fh(); $display("i=%0d b=%0d", i, b);
    $display("eq=%0d lt=%0d rtoi=%0d pow=%f not=%0d", fx() == 1.5, fx() < 2, $rtoi(fh()), fh() ** 2, !fx());
    r = c ? fx() : 2; $display("tern=%f", r);
    $display("%s", $sformatf("%f|%0d", fh(), fh()));
    r = fh() * fx(); $display("mul=%f", r);
    #1 $finish;
  end
endmodule
"#);
    // PRE: `div=0.000000`, `fq=0.000000` (integer division of the untagged slot
    // value / of the call's value), `pow=nan`; the rest were already right.
    assert_eq!(
        o,
        "div=0.500000 half=1.250000 fq=0.250000 fs=2.000000\n\
         d=2\n\
         i=3 b=3\n\
         eq=1 lt=1 rtoi=2 pow=6.250000 not=0\n\
         tern=1.500000\n\
         2.500000|3\n\
         mul=3.750000"
    );
}

/// ④ A FILL into a real target: `return '1` sized the fill to the 64-bit slot
/// (`−1.000000`); `f = '1` and an `output real o = '1` take the same
/// `resize_rhs_for_lvalue` guard the module funnel has. The `fo` cell is
/// verilator's (iverilog rejects an `output` formal on a function).
#[test]
fn a_fill_into_a_real_target_is_one_bit() {
    let o = run(r#"module t;
  real r;
  function automatic real ff; return '1; endfunction
  function automatic real fw; fw = '1; endfunction
  function automatic real fo(output real o); o = '1; return 0.0; endfunction
  initial begin
    void'(fo(r));
    $display("%f %f %f", ff(), fw(), r);
    #1 $finish;
  end
endmodule
"#);
    // PRE: `-1.000000 1.000000 1.000000`.
    assert_eq!(o, "1.000000 1.000000 1.000000");
}

/// ⑤ A real body LOCAL inside a BIT-VECTOR function, and a real-returning call
/// as an ACTUAL into a 2-state formal: the local no longer carries 65025 into
/// the return (`fe01` → `0001`), and the call is real to the inline bind, so a
/// bare-name, a package-scoped and a class-method real call all convert
/// (`4.0 + 1` → 5) where PRE handed the formal the f64 payload (`0 0 0`). The
/// second design closes the ROADMAP §2 entry "`cast_operand_is_real`'s AST half
/// sees only a bare single segment".
#[test]
fn a_real_local_in_a_bit_vector_function_and_a_real_call_as_an_actual() {
    let o = run(r#"module t;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  function automatic [15:0] fbv; real l; l = a8 * b8; fbv = l; endfunction
  initial begin
    $display("%h", fbv());
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "0001");
    let o = run(
        r#"package p; function automatic real f(input int k); f = 4.0 + k; endfunction endpackage
class C; function real cm(); return 4.0; endfunction endclass
module t;
  function [7:0] pa(input byte x); pa = x + 1; endfunction
  function automatic real rf(input d); rf = 4.0; endfunction
  C c;
  initial begin
    c = new;
    $display("A=%0d B=%0d C=%0d", pa(rf(0)), pa(p::f(0)), pa(c.cm()));
    #1 $finish;
  end
endmodule
"#,
    );
    assert_eq!(o, "A=5 B=5 C=5");
}

/// ⑥ The INLINE route (§4.5.491) is unchanged beside this: a static real
/// function without `return`, and the module-process twin of the headline.
#[test]
fn the_inline_route_and_the_module_twin_are_unchanged() {
    let o = run(r#"module t;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  real r1;
  function real sr; sr = a8 * b8; endfunction
  initial begin
    r1 = a8 * b8;
    $display("%f %f", sr(), r1);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "1.000000 1.000000");
}

/// ⑦ ROUND-1 SOUNDNESS FINDING (fixed): `reserve_class_method` is a SECOND copy
/// of the return-slot construction and was missed by the first draft — a class
/// method declared `real` kept a `Reg` slot, so the new frame-write coercion
/// rounded `c.ch()` (2.5) to 3 where PRE and both oracles print 2.5, and the
/// body's `cm = a8 * b8` still took the slot's 64-bit width (65025 in PRE and in
/// the first draft; 1 in both oracles). Both halves are pinned.
#[test]
fn a_class_method_real_return_is_a_real_slot() {
    let o = run(r#"class C;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  function real cm(); cm = a8 * b8; endfunction
  function real ch(); ch = 2.5;     endfunction
  function real cr(); return 3.75;  endfunction
endclass
module t;
  C c;
  real r1, r2, r3;
  initial begin
    c = new();
    r1 = c.cm(); r2 = c.ch(); r3 = c.cr();
    $display("%f %f %f %f", r1, r2, r3, c.cr() / 2);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "1.000000 2.500000 3.750000 1.875000");
}

/// ⑧ A real-returning PACKAGE function read inside an inline body's region
/// (ROADMAP §2 "`pk::gr()` reads 0"): the scoped package call goes through the
/// same frame reserve, so its return slot is real too. PRE printed `F=0 F2=0`.
#[test]
fn a_real_returning_package_function_is_real_in_an_inline_region() {
    let o = run(
        r#"package pk; function real gr; gr = 4.0; endfunction endpackage
module t;
  logic [7:0] a8 = 8'h02;
  function [31:0] f;  f  = a8 + pk::gr(); endfunction
  function [31:0] f2; f2 = pk::gr();      endfunction
  initial begin $display("F=%0d F2=%0d", f(), f2()); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F=6 F2=4");
}
