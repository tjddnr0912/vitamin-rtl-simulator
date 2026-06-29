//! An inlined (non-automatic) function's return value and body locals were not
//! resized to their declared width/sign. The inline path folds each `lhs = rhs;`
//! to a pure ExprId and never writes a net, so — unlike the frame-call/net path,
//! which the engine resizes on every write — the assigned value kept its
//! self-determined rhs width and sign. This was a silent-wrong: `function
//! logic[3:0] f; f=8'hAB` returned `ab` (no truncate, should be `b`); `function
//! logic[7:0] f; f=4'hF` returned `f` (no zero-extend, should be `0f`); a signed
//! local widened into the return did not sign-extend (`f` not `ff`); a signed
//! return read in a wider signed context did not sign-extend (`-1`); and
//! `function logic[7:0] f; f=42` returned a 32-bit `0000002a` not `2a`.
//!
//! Fixed by applying §10.7 assignment truncation/extension(+sign) in
//! `resize_inline_assign`, threaded through `fold_straight_line` for both the
//! return var and each body local. A genuine no-op when the rhs already matches
//! the declared width and sign (the common case stays byte-identical). Pinned to
//! iverilog 13.0.
//!
//! Known residual (separate concern, NOT covered here — pre-existing, unchanged by
//! this fix): context-width is not propagated to NON-fill sub-expressions in an
//! inline body, so a value narrower than the assignment context fed through a
//! context-determined operator keeps the operator's narrow self-width. Two shapes:
//! a shift left operand (`function logic[7:0] f(input[3:0]a); f=a<<2` gives vita
//! `0c`, iverilog `2c` — shift width = left-operand width, IEEE Table 11-21); and
//! narrow-local arithmetic (`logic[7:0] t1,t2; f16=t1*t2` evaluates the multiply
//! at 8 bits then zero-extends to `0002` instead of the 16-bit assignment-context
//! product `fd02`). This is the engine's net-write context-width mechanism, which
//! the inline SSA path lacks. It is independent of the value-resize fixed here:
//! when the locals are DECLARED at the context width, `resize_inline_assign` sizes
//! them correctly and the arithmetic is then full-width (that case IS fixed).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ifrw_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    // Each test does exactly one $display before $finish — the value is the first
    // stdout line (vita then prints a "simulation ended …" trailer).
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned()
}

#[test]
fn return_value_truncated_to_declared_width() {
    // f is 4-bit; 8'hAB truncates to its low nibble.
    let o = run("module top; function logic [3:0] f; f=8'hAB; endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule");
    assert_eq!(o.trim(), "b", "got:\n{o}");
}

#[test]
fn return_value_zero_extended_to_declared_width() {
    // f is 8-bit; an unsigned 4-bit value zero-extends.
    let o = run("module top; function logic [7:0] f; f=4'hF; endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule");
    assert_eq!(o.trim(), "0f", "got:\n{o}");
}

#[test]
fn signed_local_sign_extended_into_return() {
    // A signed[3:0] local holding 4'hF (= -1) widens to the 8-bit return as 0xFF
    // (sign-extend), proving the local carries its declared sign.
    let o = run(
        "module top; function logic [7:0] f; logic signed [3:0] s; s=4'hF; f=s; endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule",
    );
    assert_eq!(o.trim(), "ff", "got:\n{o}");
}

#[test]
fn signed_return_read_in_wider_signed_context() {
    // f is signed[3:0]; 4'hF reads back as -1, sign-extended into signed[7:0].
    let o = run("module top; function logic signed [3:0] f; f=4'hF; endfunction\n\
         initial begin logic signed [7:0] r; r=f(); $display(\"%0d\", r); #1 $finish; end endmodule");
    assert_eq!(o.trim(), "-1", "got:\n{o}");
}

#[test]
fn unsigned_value_into_signed_wide_return() {
    // f is signed[7:0]; an UNSIGNED 4'hF zero-extends to 0x0F, read as +15 (the
    // extension follows the RHS sign, the readback the TARGET sign).
    let o = run(
        "module top; function logic signed [7:0] f; f=4'hF; endfunction\n\
         initial begin $display(\"%0d\", f()); #1 $finish; end endmodule",
    );
    assert_eq!(o.trim(), "15", "got:\n{o}");
}

#[test]
fn wide_literal_truncated_to_narrow_return() {
    // `f=42` into a logic[7:0] return: the 32-bit literal narrows to 8 bits (was
    // a silent 32-bit `0000002a`).
    let o = run("module top; function logic [7:0] f; f=42; endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule");
    assert_eq!(o.trim(), "2a", "got:\n{o}");
}

#[test]
fn signed_local_arithmetic_keeps_sign() {
    // signed[3:0] t = 4'hF (= -1); t*2 = -2 in the signed[7:0] return.
    let o = run(
        "module top; function logic signed [7:0] f; logic signed [3:0] t; t=4'hF; f=t*2; endfunction\n\
         initial begin $display(\"%0d\", f()); #1 $finish; end endmodule",
    );
    assert_eq!(o.trim(), "-2", "got:\n{o}");
}

#[test]
fn block_local_truncated() {
    // A block-local declared narrower than its assigned value truncates, and the
    // truncated value feeds the (matching-width) return.
    let o = run(
        "module top; function logic [3:0] f; logic [3:0] t; t=8'hAB; f=t; endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule",
    );
    assert_eq!(o.trim(), "b", "got:\n{o}");
}

#[test]
fn real_returning_inline_function_not_resized() {
    // An inline `real` function whose body CALLS a real-returning FRAME function
    // (the callee has control flow, so it is framed → an `Expr::Call` node that
    // the IR-level real check cannot see) must not be bit-resized / $signed-
    // stamped — the value stays real. Guarded by the AST-aware `cast_operand_is_real`.
    let o = run("module top;\n\
         function real g(input real x); if (x>0) g=x*2.0; else g=0.0; endfunction\n\
         function real f; f=g(2.5); endfunction\n\
         initial begin $display(\"%f\", f()); #1 $finish; end endmodule");
    assert_eq!(o, "5.000000", "got:\n{o}");
    // And the real result survives an integral readback (real→int conversion).
    let oi = run("module top;\n\
         function real g(input real x); if (x>0) g=x*2.0; else g=0.0; endfunction\n\
         function real f; f=g(2.5); endfunction\n\
         initial begin int r; r=f(); $display(\"%0d\", r); #1 $finish; end endmodule");
    assert_eq!(oi, "5", "int-readback got:\n{oi}");
}

#[test]
fn common_inline_functions_unchanged() {
    // Byte-identity: functions whose rhs already matches the declared width/sign
    // are untouched (no spurious resize / sign stamp).
    let add = run(
        "module top; function logic [7:0] f(input [7:0] a, b); f=a+b; endfunction\n\
         initial begin $display(\"%h\", f(8'h10, 8'h05)); #1 $finish; end endmodule",
    );
    assert_eq!(add.trim(), "15", "add got:\n{add}");

    let int_ret = run("module top; function int f; f=42; endfunction\n\
         initial begin $display(\"%0d\", f()); #1 $finish; end endmodule");
    assert_eq!(int_ret.trim(), "42", "int got:\n{int_ret}");

    let signed_add = run(
        "module top; function int f(input int a, b); f=a+b; endfunction\n\
         initial begin $display(\"%0d\", f(-3, 5)); #1 $finish; end endmodule",
    );
    assert_eq!(signed_add.trim(), "2", "signed-add got:\n{signed_add}");

    let mask = run(
        "module top; function logic [7:0] f(input [7:0] a); f=a&8'h0F; endfunction\n\
         initial begin $display(\"%h\", f(8'hAB)); #1 $finish; end endmodule",
    );
    assert_eq!(mask.trim(), "0b", "mask got:\n{mask}");

    let tern = run(
        "module top; function logic [3:0] f(input c); f=c?4'hA:4'h5; endfunction\n\
         initial begin $display(\"%h\", f(1'b1)); #1 $finish; end endmodule",
    );
    assert_eq!(tern.trim(), "a", "ternary got:\n{tern}");

    let implicit = run("module top; function f; f=1'b1; endfunction\n\
         initial begin $display(\"%b\", f()); #1 $finish; end endmodule");
    assert_eq!(implicit.trim(), "1", "implicit got:\n{implicit}");
}
