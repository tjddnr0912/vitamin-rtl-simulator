//! A function's DECLARED return width is a self-determined boundary — the twin of
//! §4.5.320's size cast, on the inline (SSA-fold) path.
//!
//! `resize_inline_assign` applies §10.7 to every assignment the inline fold
//! substitutes, because that path writes no net and so gets no engine-side resize.
//! Its `$signed`/`$unsigned` stamp was CONDITIONAL — added only when the target's
//! signedness differed from the rhs's — and in the case where it was skipped
//! nothing sealed the value: a bare `Binary`/`Unary`/`Ternary` node is
//! CONTEXT-determined to the engine, so a wider call-site context re-ran the whole
//! body expression. `function [7:0] fm(input [7:0] x); fm = x*x;` read into a
//! 64-bit destination printed `fe01` where iverilog prints `0001`.
//!
//! A 1,440-cell sweep (5 declared widths x signed/unsigned x 9 body expressions x
//! 4 arguments x 4 destinations) measured **48 wrong cells, every one of them with
//! declared width == rhs self width AND an unsigned return** — a SIGNED return was
//! sealed by accident, because an unsigned rhs made the stamp differ and appear.
//! The fix makes the stamp unconditional.
//!
//! ⚠️ `automatic` is a CORRELATE of the frame path, not the discriminator — that
//! is `frames_classify.rs`'s disjunction (`automatic || ret_two_state ||
//! body_needs_frame || an unpacked formal || (body_needs_context_width &&
//! body_reads_only_locals)` plus recursion), and a STATIC function can be framed
//! by it (measured: `function [7:0] g(input [3:0] x); g = x*x;` frames via
//! `body_needs_context_width` while its sibling `f = x` inlines). Every function
//! below was checked to be on the inline path.
//!
//! ORACLE: iverilog 13.0 — every expected value here was run through it, except
//! the class-field row, which iverilog cannot compile and which says so.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_irs_{}_{n}", std::process::id()));
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
    let r = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::remove_dir_all(&d);
    r
}

fn run(src: &str) -> String {
    run_args(src, &[])
}

/// The headline. `x*x` = 65025 needs 16 bits; the declared return says 8, so the
/// product is 8-bit and the answer is `01`. `x+x` overflows by one bit, `x<<3` by
/// three. A 64-bit destination must not reopen any of them.
#[test]
fn a_declared_return_width_is_not_reopened_by_a_wider_call_site() {
    let o = run(r#"module t;
  function [7:0] fm(input [7:0] x); fm = x*x;  endfunction
  function [7:0] fa(input [7:0] x); fa = x+x;  endfunction
  function [7:0] fs(input [7:0] x); fs = x<<3; endfunction
  function [7:0] fp(input [7:0] x); fp = x**2; endfunction
  logic [63:0] r; logic [7:0] e;
  initial begin
    #1;
    e = fm(8'hFF); $display("%h", e);
    r = fm(8'hFF); $display("%h", r);
    r = fa(8'hFF); $display("%h", r);
    r = fs(8'hFF); $display("%h", r);
    r = fp(8'h11); $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    // iverilog. PRE: 01 / fe01 / 01fe / 07f8 / 0121 — the row whose destination is
    // as wide as the declared return was already right, which is why the suite
    // never noticed the other four.
    assert_eq!(
        o,
        "01\n0000000000000001\n00000000000000fe\n\
         00000000000000f8\n0000000000000021"
    );
}

/// A context-determined UNARY is the same hole and the loudest one: `-x`/`~x` at a
/// declared width of 8 spread ones all the way to bit 63.
#[test]
fn a_context_determined_unary_return_is_sealed_too() {
    let o = run(r#"module t;
  function [7:0] fn(input [7:0] x); fn = -x; endfunction
  function [7:0] ft(input [7:0] x); ft = ~x; endfunction
  logic [63:0] r;
  initial begin
    #1;
    r = fn(8'h03); $display("%h", r);
    r = ft(8'h0F); $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "00000000000000fd\n00000000000000f0"); // PRE: ffff…fd / ffff…f0
}

/// The seal must hold through the fold's other two shapes — an SSA body local and
/// an explicit `return` — and through a ternary, whose branches are also
/// context-determined.
#[test]
fn the_seal_holds_for_every_inline_body_shape() {
    let o = run(r#"module t;
  function [7:0] fl(input [7:0] x);
    reg [7:0] t; begin t = x*x; fl = t + 8'd0; end
  endfunction
  function [7:0] fr(input [7:0] x); return x*x; endfunction
  function [7:0] fc(input [7:0] x); fc = x[0] ? x*x : x+x; endfunction
  function [7:0] fg(input [7:0] x); fg = x*x; endfunction
  function [7:0] fo(input [7:0] x); fo = fg(x) + 8'd0; endfunction
  logic [63:0] r;
  initial begin
    #1;
    r = fl(8'hFF); $display("%h", r);
    r = fr(8'hFF); $display("%h", r);
    r = fc(8'hFF); $display("%h", r);
    r = fo(8'hFF); $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(
        o,
        "0000000000000001\n0000000000000001\n\
         0000000000000001\n0000000000000001"
    );
}

/// Every consuming position that pushes a width inward leaked; the four
/// self-determined ones did not, and they are pinned so an over-eager seal that
/// changed the VALUE could not pass either.
#[test]
fn the_seal_holds_in_every_context_determined_consumer() {
    let o = run(r#"module t;
  function [7:0] f(input [7:0] x); f = x*x; endfunction
  logic [63:0] r; logic [63:0] mem [0:255];
  initial begin
    mem[1] = 64'hcafe;
    #1;
    r = f(8'hFF) + 64'd0;              $display("ADD  %h", r);
    r = f(8'hFF) | 64'h0;              $display("OR   %h", r);
    r = {63'b0, (f(8'hFF) == 64'd1)};  $display("CMP  %h", r);
    case (f(8'hFF)) 64'd1: r = 64'h1; default: r = 64'hbeef; endcase
    $display("CASE %h", r);
    r = {56'b0, f(8'hFF)};             $display("CAT  %h", r);
    r = mem[f(8'hFF)];                 $display("IDX  %h", r);
    $display("DISP %h", f(8'hFF));
    #1 $finish;
  end
endmodule
"#);
    // iverilog. PRE: fe01 / fe01 / 0 / beef, and the last three already correct.
    assert_eq!(
        o,
        "ADD  0000000000000001\nOR   0000000000000001\n\
         CMP  0000000000000001\nCASE 0000000000000001\n\
         CAT  0000000000000001\nIDX  000000000000cafe\nDISP 01"
    );
}

/// ANTI-VACUITY on the sign axis. A SIGNED return was already sealed — the stamp
/// appeared because the unsigned rhs made it differ — and a narrowing or widening
/// return was sealed by the `Select`/`Concat` the resize builds. All three must
/// keep their values, so a seal that stamped the wrong sign cannot pass.
#[test]
fn the_already_sealed_halves_are_byte_identical() {
    let o = run(r#"module t;
  function signed [7:0] fs(input [7:0] x); fs = x*x;  endfunction
  function        [3:0] fn(input [7:0] x); fn = x*x;  endfunction
  function       [15:0] fw(input [7:0] x); fw = x*x;  endfunction
  function signed [3:0] fq(input [7:0] x); fq = x+8'd9; endfunction
  logic [63:0] r;
  initial begin
    #1;
    r = fs(8'hFF); $display("%h", r);
    r = fn(8'hFF); $display("%h", r);
    r = fw(8'hFF); $display("%h", r);
    r = fq(8'hFF); $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    // iverilog; PRE identical on all four. Row 3 is the widening resize (`x*x`
    // fits 16 bits, so `fe01` is the whole product) and row 4 the narrowing one
    // whose 4-bit result has its top bit set, so it must SIGN-extend.
    assert_eq!(
        o,
        "0000000000000001\n0000000000000001\n\
         000000000000fe01\nfffffffffffffff8"
    );
}

/// 4-state must survive the seal: an x-bearing argument keeps its x in the low
/// eight bits and must NOT spread past the declared width.
#[test]
fn the_seal_preserves_x_within_the_declared_width() {
    let o = run(r#"module t;
  function [7:0] f(input [7:0] x); f = x + 8'd1; endfunction
  logic [63:0] r;
  initial begin
    #1;
    r = f(8'bxxxx_0011); $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "00000000000000xx"); // iverilog; PRE: xxxxxxxxxxxxxxxx
}

/// Above the word boundary — the seal must not depend on one lane.
#[test]
fn the_seal_holds_above_the_word_boundary() {
    let o = run(r#"module t;
  function [99:0] f(input [99:0] x); f = x + 100'd1; endfunction
  logic [99:0] r;
  logic [99:0] wa = 100'hF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF;
  initial begin
    #1;
    r = f(wa); $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "0000000000000000000000000"); // iverilog
}

/// The seal adds a `SysFunc` node, so the three execution backends must still
/// agree on it.
#[test]
fn the_three_backends_agree_on_a_sealed_inline_return() {
    let src = r#"module t;
  function [7:0] fm(input [7:0] x); fm = x*x; endfunction
  function [7:0] fn(input [7:0] x); fn = -x;  endfunction
  function signed [3:0] fq(input [7:0] x); fq = x+8'd9; endfunction
  logic [63:0] r;
  initial begin
    #1;
    r = fm(8'hFF); $display("%h", r);
    r = fn(8'h03); $display("%h", r);
    r = fq(8'hFF); $display("%h", r);
    #1 $finish;
  end
endmodule
"#;
    let want = "0000000000000001\n00000000000000fd\nfffffffffffffff8";
    assert_eq!(run_args(src, &["--backend", "interp"]), want);
    assert_eq!(run_args(src, &["--backend", "bytecode"]), want);
    assert_eq!(run_args(src, &["--backend", "native"]), want);
}

/// The `trusted_w` guard's ONLY teeth. A class field lowers to a `Signal` on the
/// 32-bit HANDLE net with its real width in a sidecar, so `ir_bits_of` answers a
/// FABRICATED `Some(32)` that happens to equal the declared return width — the
/// same-width arm. Sealing there would evaluate the body at the field's canonical
/// 8 bits and truncate the 32-bit product. The value must not depend on the
/// destination width.
///
/// No iverilog oracle (no classes). Hand-IEEE: the declared return is the body's
/// assignment context, so `c.fld * x` runs at 32 bits — 255 * 255 = `fe01`.
/// Dropping the canonical cross-check, or sealing unconditionally, gives `0001`.
#[test]
fn a_fabricated_rhs_width_declines_the_seal() {
    let o = run(r#"module t;
  class C; bit [7:0] fld; endclass
  C c;
  function [31:0] fh(input [7:0] x); fh = c.fld * x; endfunction
  logic [63:0] r; logic [31:0] q;
  initial begin
    c = new; c.fld = 8'hFF;
    #1;
    q = fh(8'hFF); $display("%h", q);
    r = fh(8'hFF); $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "0000fe01\n000000000000fe01"); // PRE identical
}

/// ⚠️ The sign half of this function is NOT fixed and this row says so. The
/// extension direction still comes from `expr_self_signed`, which calls a frame
/// function's declared return unsigned; the canonical rule would say signed, but
/// `extend_to`'s sign fill names the operand a second time and the only leaf that
/// can reach a WIDENING resize here is exactly that (impure) call. iverilog says
/// `fc` / `fffc`. ROADMAP §2.
#[test]
fn the_extension_sign_of_a_function_actual_is_a_documented_gap() {
    let o = run(r#"module t;
  function automatic signed [3:0] fs(input d); fs = -4'sd4; endfunction
  function        [7:0] inl(input signed [3:0] x); inl = x; endfunction
  function       [15:0] wid(input signed [3:0] x); wid = x; endfunction
  logic [63:0] r;
  initial begin
    #1;
    r = inl(fs(0));  $display("%h", r);
    r = wid(fs(0));  $display("%h", r);
    r = inl(-4'sd4); $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    // Rows 1-2 are the gap (iverilog: 00000000000000fc / 000000000000fffc);
    // row 3 shows a LITERAL actual is already right, i.e. the leaf is the whole
    // discriminator.
    assert_eq!(o, "000000000000000c\n000000000000000c\n00000000000000fc");
}
