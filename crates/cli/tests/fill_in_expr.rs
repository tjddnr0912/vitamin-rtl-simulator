//! Unsized fill literals (`'0`/`'1`/`'x`/`'z`) are CONTEXT-determined in width
//! (IEEE §11.6/§11.8.1), not a fixed 32 bits. The adversarial hunt found vita
//! capped a fill at 32 bits whenever it appeared INSIDE an expression (binary op,
//! ternary, comparison, function arg, class field, return) and used 32 bits in a
//! concatenation where it should be 1 bit. These cases all silently produced
//! wrong values. Oracle: iverilog -g2012.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_fie_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

// ── concatenation: a fill operand is SELF-determined to 1 bit ──────────────
#[test]
fn fill_in_concat_is_one_bit() {
    let out = run("module t; initial begin\n\
           $display(\"%h\", {'1, 4'h5});\n\
           $display(\"%h\", {'0, 4'h5});\n\
           $display(\"%h\", {8'hAB, '1});\n\
         end endmodule\n");
    assert_eq!(out, "15\n05\n157\n");
}

#[test]
fn fill_in_replication_is_one_bit() {
    let out = run("module t; initial $display(\"%b\", {2{'1}}); endmodule\n");
    assert_eq!(out, "11\n");
}

// ── binary-op / shift operand: context-determined to the assignment width ──
#[test]
fn fill_in_binary_op_assignment_width() {
    let out = run("module t;\n\
           logic [63:0] r; logic [39:0] r40; logic [99:0] r100;\n\
           initial begin\n\
             r = '1 + 64'd0;    $display(\"%h\", r);\n\
             r40 = '1 ^ 40'h0;  $display(\"%h\", r40);\n\
             r100 = '1 | 100'h0; $display(\"%h\", r100);\n\
             r = '1 >> 2;       $display(\"%h\", r);\n\
             r = '1 << 1;       $display(\"%h\", r);\n\
             r = '1 - 1;        $display(\"%h\", r);\n\
           end endmodule\n");
    assert_eq!(
        out,
        "ffffffffffffffff\nffffffffff\nfffffffffffffffffffffffff\n3fffffffffffffff\nfffffffffffffffe\nfffffffffffffffe\n"
    );
}

#[test]
fn fill_into_33bit_keeps_top_bit() {
    let out = run("module t;\n\
           logic [32:0] r33;\n\
           initial begin r33 = '1 + 1'b0; $display(\"%h\", r33); end\n\
         endmodule\n");
    assert_eq!(out, "1ffffffff\n");
}

#[test]
fn signed_fill_in_expr_is_minus_one() {
    let out = run("module t;\n\
           logic signed [63:0] s;\n\
           initial begin s = '1 + 0; $display(\"%0d\", s); end\n\
         endmodule\n");
    assert_eq!(out, "-1\n");
}

#[test]
fn fill_overflow_wraps_at_context_width() {
    let out = run("module t;\n\
           logic [63:0] r;\n\
           initial begin r = '1 + 64'h1; $display(\"%h\", r); end\n\
         endmodule\n");
    assert_eq!(out, "0000000000000000\n");
}

#[test]
fn fill_in_comparison_sized_to_sibling() {
    let out = run("module t;\n\
           logic res;\n\
           initial begin\n\
             res = (('1 + 64'd0) == 64'hFFFFFFFFFFFFFFFF);\n\
             $display(\"%b\", res);\n\
           end endmodule\n");
    assert_eq!(out, "1\n");
}

#[test]
fn z_fill_in_binary_op() {
    let out = run("module t;\n\
           logic [63:0] r;\n\
           initial begin r = 'z & 64'hFFFFFFFFFFFFFFFF; $display(\"%h\", r); end\n\
         endmodule\n");
    assert_eq!(out, "xxxxxxxxxxxxxxxx\n");
}

#[test]
fn fill_in_ternary_and_or_operand() {
    let out = run("module t;\n\
           logic [63:0] r; logic sel;\n\
           initial begin\n\
             sel = 1;\n\
             r = sel ? '1 : 64'd0; $display(\"%h\", r);\n\
             r = {64{1'b0}} | '1;  $display(\"%h\", r);\n\
           end endmodule\n");
    assert_eq!(out, "ffffffffffffffff\nffffffffffffffff\n");
}

#[test]
fn fill_as_function_argument() {
    let out = run("module t;\n\
           function logic [63:0] f(logic [63:0] x); return x; endfunction\n\
           logic [63:0] r;\n\
           initial begin r = f('1); $display(\"%h\", r); end\n\
         endmodule\n");
    assert_eq!(out, "ffffffffffffffff\n");
}

#[test]
fn fill_in_class_field_and_return() {
    let out = run("module t;\n\
           class C;\n\
             logic [63:0] f;\n\
             function void set(); f = '1 + 1'b0; endfunction\n\
           endclass\n\
           function logic [63:0] g(); return '1 + 1'b0; endfunction\n\
           C c; logic [63:0] r;\n\
           initial begin c = new(); c.set(); $display(\"%h\", c.f);\n\
             r = g(); $display(\"%h\", r); end\n\
         endmodule\n");
    assert_eq!(out, "ffffffffffffffff\nffffffffffffffff\n");
}

#[test]
fn fill_case_label_sized_to_scrutinee() {
    // A `'1` case label is sized to the case-expression width, so `case(8'hFF)`
    // matches `'1:` (label = 8'hFF). Previously the 32-bit label never matched
    // and silently fell through to default — a control-flow silent-wrong.
    let out = run("module t;\n\
           logic [7:0] x;\n\
           initial begin\n\
             x = 8'hFF;\n\
             case (x) '1: $display(\"ones\"); '0: $display(\"zero\"); default: $display(\"def\"); endcase\n\
             x = 8'h00;\n\
             case (x) '1: $display(\"b-ones\"); '0: $display(\"b-zero\"); default: $display(\"b-def\"); endcase\n\
           end endmodule\n");
    assert_eq!(out, "ones\nb-zero\n");
}

#[test]
fn fill_in_signed_unsigned_is_one_bit() {
    // $signed/$unsigned arg is self-determined ⇒ a bare fill is 1 bit.
    let out = run("module t; initial begin\n\
           $display(\"%0d\", $unsigned('1));\n\
           $display(\"%0d\", $unsigned('1) >> 1);\n\
         end endmodule\n");
    assert_eq!(out, "1\n0\n");
}

#[test]
fn fill_ternary_in_self_determined_context() {
    // A ternary that is itself a self-determined operand ($display arg) still
    // sizes a fill branch to its sibling branch's width.
    let out = run("module t;\n\
           logic [31:0] a = 32'd10;\n\
           initial begin\n\
             $display(\"%h\", (a > 5) ? '1 : 32'd7);\n\
             $display(\"%h\", (a > 5) ? '1 : 8'd7);\n\
             $display(\"%h\", (a > 5) ? '1 : 64'd7);\n\
           end endmodule\n");
    assert_eq!(out, "ffffffff\nff\nffffffffffffffff\n");
}

#[test]
fn fill_as_select_index_is_value_one() {
    let out = run("module t;\n\
           reg [15:0] vec; reg [3:0] ps;\n\
           initial begin vec = 16'hABCD; ps = vec['1 +: 4]; $display(\"%h\", ps); end\n\
         endmodule\n");
    assert_eq!(out, "6\n");
}

#[test]
fn fill_sized_to_function_call_sibling() {
    let out = run("module t;\n\
           function automatic [7:0] f(); f = 8'hFF; endfunction\n\
           initial case (f()) '1: $display(\"ones\"); default: $display(\"def\"); endcase\n\
         endmodule\n");
    assert_eq!(out, "ones\n");
}

#[test]
fn fill_as_task_input_arg() {
    let out = run("module t;\n\
           logic [63:0] g;\n\
           task tset(input logic [63:0] x); g = x; endtask\n\
           initial begin tset('1); $display(\"%h\", g); tset('x); $display(\"%h\", g); end\n\
         endmodule\n");
    assert_eq!(out, "ffffffffffffffff\nxxxxxxxxxxxxxxxx\n");
}

#[test]
fn fill_in_static_function_return_and_local() {
    // A bare fill assigned to a STATIC (non-automatic) function's return var or a
    // body-local sizes to the LHS width (was 1-bit).
    let out = run("module t;\n\
           function [7:0]  s_direct(); s_direct = '1; endfunction\n\
           function [15:0] wide();     wide     = '1; endfunction\n\
           function [31:0] f_arith; logic [15:0] hw; begin hw = '1; f_arith = hw + 1; end endfunction\n\
           initial begin\n\
             $display(\"%h\", s_direct());\n\
             $display(\"%h\", wide());\n\
             $display(\"%h\", f_arith());\n\
           end endmodule\n");
    assert_eq!(out, "ff\nffff\n00010000\n");
}

#[test]
fn fill_as_repeat_count_is_one_iteration() {
    let out = run("module t;\n\
           reg [7:0] c;\n\
           initial begin\n\
             c = 0; repeat ('1) c = c + 1; $display(\"%0d\", c);\n\
             c = 0; repeat ('0) c = c + 1; $display(\"%0d\", c);\n\
           end endmodule\n");
    assert_eq!(out, "1\n0\n");
}

#[test]
fn fill_as_port_connection() {
    let out = run("module child(input [63:0] p);\n\
           initial #1 $display(\"%h\", p);\n\
         endmodule\n\
         module t; child c(.p('1)); endmodule\n");
    assert_eq!(out, "ffffffffffffffff\n");
}

#[test]
fn fill_in_continuous_assign_binary_op() {
    let out = run("module t;\n\
           logic [63:0] w;\n\
           assign w = '1 & 64'hFFFFFFFFFFFFFFFF;\n\
           initial begin #1; $display(\"%h\", w); end\n\
         endmodule\n");
    assert_eq!(out, "ffffffffffffffff\n");
}
