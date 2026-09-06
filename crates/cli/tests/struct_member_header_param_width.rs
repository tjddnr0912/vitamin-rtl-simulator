//! A packed struct whose member width names a HEADER (overridable) parameter —
//! ROADMAP §3 ⑤ ⓒ, the ibex_lockstep rung (§4.5.431).
//!
//! `module c #(parameter int unsigned W = 4) (…); typedef struct packed { logic g;
//! logic [W-1:0] d; } t_t;` was a parse error ("struct member width must be a named
//! integer type or a constant-literal range") because the parser lays a packed
//! struct out at parse time and a header parameter has no value there — both
//! oracles lay it out PER INSTANCE. The parser now keeps such a layout as
//! EXPRESSIONS (`SymStructLayout`: member width `E + 1` for a `[E:0]` range, offsets
//! as sums, the typedef's range `[total-1:0]`) and emits them into the part-selects
//! a member access desugars to and into the declared range; elaborate folds them
//! with each instance's parameter value. Every consumer keyed on the numeric layout
//! (`'{…}`, a union, `$bits(T)`, a nested chain, a member sub-select) stays loud.
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.050 (two instances
//! per design, `W = 4` default and `W = 7` override; 28-cell census, 16 loud→correct,
//! 0 silent); the lines are the oracles' output, copied.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_smhpw_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// `module c #(parameter int unsigned W = 4) (input logic [W-1:0] a); <body>` under a
/// `top` that instantiates it with the default and with `W = 7`.
fn design(body: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule c #(parameter int unsigned W = 4) (input logic [W-1:0] \
         a);\n{body}\nendmodule\nmodule top;\n  logic [3:0] a4 = 4'hA; logic [6:0] a7 = 7'h55;\n  \
         c u4(.a(a4)); c #(.W(7)) u7(.a(a7));\n  initial #4 $finish;\nendmodule\n"
    )
}

fn d_lines(src: &str) -> Vec<String> {
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "exit\n{out}");
    let mut v: Vec<String> = out
        .lines()
        .filter(|l| l.starts_with("D="))
        .map(str::to_string)
        .collect();
    v.sort();
    v
}

fn prints(body: &str, want: &[&str]) {
    let mut want: Vec<&str> = want.to_vec();
    want.sort();
    assert_eq!(d_lines(&design(body)), want);
}

fn loud(body: &str, needle: &str) {
    let (out, code) = run(&design(body));
    assert_ne!(code, Some(0), "expected a refusal\n{out}");
    assert!(out.contains(needle), "expected `{needle}`\n{out}");
}

const T: &str = "  typedef struct packed { logic g; logic [W-1:0] d; } t_t;\n";

#[test]
fn members_read_and_write_at_each_instances_width() {
    // s01
    prints(
        "  typedef struct packed { logic g; logic [W-1:0] d; logic [1:0] t; } t_t;\n  t_t s;\n  \
         assign s.g = 1'b1; assign s.d = a; assign s.t = 2'b10;\n  initial #1 $display(\"D=W%0d \
         s=%h g=%b d=%h t=%b\", W, s, s.g, s.d, s.t);",
        &["D=W4 s=6a g=1 d=a t=10", "D=W7 s=356 g=1 d=55 t=10"],
    );
    // s25 · two symbolic members: the second's OFFSET is symbolic too
    prints(
        "  typedef struct packed { logic [W-1:0] x; logic [W-1:0] y; logic z; } t_t;\n  t_t s; \
         assign s = {a, ~a, 1'b1};\n  initial #1 $display(\"D=W%0d x=%h y=%h z=%b\", W, s.x, s.y, \
         s.z);",
        &["D=W4 x=a y=5 z=1", "D=W7 x=55 y=2a z=1"],
    );
    // s04 · `[W:0]`, `[2*W-1:0]`, a localparam derived from the header parameter
    prints(
        "  localparam int X = W * 2;\n  typedef struct packed { logic [W:0] p; logic [2*W-1:0] \
         q; logic [X-1:0] r; } t_t;\n  t_t s;\n  assign s.p = {1'b1, a}; assign s.q = {a, a}; \
         assign s.r = {a, ~a};\n  initial #1 $display(\"D=W%0d p=%h q=%h r=%h s=%h\", W, s.p, \
         s.q, s.r, s);",
        &[
            "D=W4 p=1a q=aa r=a5 s=1aaaa5",
            "D=W7 p=d5 q=2ad5 r=2aaa s=d5ab56aaa",
        ],
    );
    // s22 · `$clog2(W)`
    prints(
        "  typedef struct packed { logic g; logic [$clog2(W)-1:0] d; } t_t;\n  t_t s; assign s.g \
         = 1'b0; assign s.d = a[$clog2(W)-1:0];\n  initial #1 $display(\"D=W%0d s=%h d=%h\", W, \
         s, s.d);",
        &["D=W4 s=2 d=2", "D=W7 s=5 d=5"],
    );
    // s13 · atom, symbolic, localparam-folded and byte members side by side
    prints(
        "  localparam int K = 3;\n  typedef struct packed { int i; logic [W-1:0] d; logic [K-1:0] \
         k; byte b; } t_t;\n  t_t s; assign s.i = 32'h1234_5678; assign s.d = a; assign s.k = \
         3'b101; assign s.b = 8'hEE;\n  initial #1 $display(\"D=W%0d s=%h i=%h d=%h k=%b b=%h\", \
         W, s, s.i, s.d, s.k, s.b);",
        &[
            "D=W4 s=091a2b3c55ee i=12345678 d=a k=101 b=ee",
            "D=W7 s=048d159e2adee i=12345678 d=55 k=101 b=ee",
        ],
    );
    // s16 · NBA member writes in an `always_ff`, compare and concatenate whole values
    prints(
        &format!(
            "{T}  t_t q, s; logic clk = 0; always #1 clk = ~clk;\n  assign s.g = 1'b1; assign \
             s.d = a;\n  always_ff @(posedge clk) begin q.d <= s.d; q.g <= ~s.g; end\n  initial \
             #3 $display(\"D=W%0d q=%h eq=%b cat=%h\", W, q, q == s, {{s, q}});"
        ),
        &["D=W4 q=0a eq=0 cat=34a", "D=W7 q=55 eq=0 cat=d555"],
    );
    // s24 · the whole struct read hierarchically from the parent
    let (out, code) = run(
        &design(&format!("{T}  t_t s; assign s.g = 1'b1; assign s.d = a;")).replace(
            "initial #4 $finish;",
            "initial #1 $display(\"D=h4 %h h7 %h\", u4.s, u7.s);\n  initial #4 $finish;",
        ),
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.lines().any(|l| l == "D=h4 1a h7 d5"), "{out}");
}

#[test]
fn packed_and_unpacked_arrays_of_the_struct_and_the_cast() {
    // s02 · a packed array: element member read/write, a whole-element copy, `t_t'('0)`
    prints(
        &format!(
            "{T}  t_t [1:0] arr; t_t e;\n  assign arr[0].g = 1'b1; assign arr[0].d = a;\n  \
             assign arr[1] = t_t'('0);\n  assign e = arr[0];\n  initial #1 $display(\"D=W%0d \
             arr=%h a0d=%h a1=%h e=%h eg=%b\", W, arr, arr[0].d, arr[1], e, e.g);"
        ),
        &[
            "D=W4 arr=01a a0d=a a1=00 e=1a eg=1",
            "D=W7 arr=00d5 a0d=55 a1=00 e=d5 eg=1",
        ],
    );
    // s17 · a genvar index
    prints(
        &format!(
            "{T}  t_t [1:0] arr; genvar i;\n  for (i = 0; i < 2; i++) begin : g assign arr[i].d \
             = a + i; assign arr[i].g = i[0]; end\n  initial #1 $display(\"D=W%0d arr=%h \
             a1d=%h\", W, arr, arr[1].d);"
        ),
        &["D=W4 arr=36a a1d=b", "D=W7 arr=d655 a1d=56"],
    );
    // s03 · `t_t'(wide)` truncates to the instance's width
    prints(
        &format!(
            "{T}  t_t s; logic [15:0] wide = 16'hFFFF;\n  assign s = t_t'(wide);\n  initial #1 \
             $display(\"D=W%0d s=%h d=%h\", W, s, s.d);"
        ),
        &["D=W4 s=1f d=f", "D=W7 s=ff d=7f"],
    );
    // s15 · an unpacked array (verilator; iverilog refuses the `'0` element write)
    prints(
        &format!(
            "{T}  t_t arr [2];\n  assign arr[1].g = 1'b1; assign arr[1].d = a; assign arr[0] = \
             '0;\n  initial #1 $display(\"D=W%0d a1=%h a1d=%h a0=%h\", W, arr[1], arr[1].d, \
             arr[0]);"
        ),
        &["D=W4 a1=1a a1d=a a0=00", "D=W7 a1=d5 a1d=55 a0=00"],
    );
    // s21 · the whole struct on a child's port, sized by the same parameter
    let src = format!(
        "`timescale 1ns/1ns\nmodule ch #(parameter int N = 5) (input logic [N-1:0] v);\n  initial \
         #1 $display(\"D=N%0d v=%h\", N, v);\nendmodule\n{}",
        design(&format!(
            "{T}  t_t s; assign s.g = 1'b1; assign s.d = a;\n  ch #(.N(W+1)) u(.v(s));"
        ))
    );
    assert_eq!(d_lines(&src), ["D=N5 v=1a", "D=N8 v=d5"]);
}

#[test]
fn signedness_and_a_body_parameter() {
    // s10 · a signed member reads back negative at 32 bits
    prints(
        "  typedef struct packed { logic g; logic signed [W-1:0] d; } t_t;\n  t_t s; integer r; \
         assign s.g = 1'b0; assign s.d = a;\n  initial #1 begin r = s.d; $display(\"D=W%0d r=%0d \
         d=%h\", W, r, s.d); end",
        &["D=W4 r=-6 d=a", "D=W7 r=-43 d=55"],
    );
    // s11 · `struct packed signed` whole value
    prints(
        "  typedef struct packed signed { logic g; logic [W-1:0] d; } t_t;\n  t_t s; integer r; \
         assign s.g = 1'b1; assign s.d = a;\n  initial #1 begin r = s; $display(\"D=W%0d r=%0d\", \
         W, r); end",
        &["D=W4 r=-6", "D=W7 r=-43"],
    );
    // s18 · a BODY `parameter` (overridable, no header)
    let src = "`timescale 1ns/1ns\nmodule c (input logic [7:0] a);\n  parameter int unsigned W = \
               4;\n  typedef struct packed { logic g; logic [W-1:0] d; } t_t;\n  t_t s; assign s.g \
               = 1'b1; assign s.d = a[W-1:0];\n  initial #1 $display(\"D=W%0d s=%h d=%h\", W, s, \
               s.d);\nendmodule\nmodule top;\n  logic [7:0] a = 8'h5A;\n  c u4(.a(a)); c #(.W(7)) \
               u7(.a(a));\n  initial #4 $finish;\nendmodule\n";
    assert_eq!(d_lines(src), ["D=W4 s=1a d=a", "D=W7 s=da d=5a"]);
    // s14 · a symbolic struct as a MEMBER of another: the whole member reads
    prints(
        "  typedef struct packed { logic g; logic [W-1:0] d; } in_t;\n  typedef struct packed { \
         logic f; in_t i; } out_t;\n  out_t o; assign o.f = 1'b1; assign o.i = '0;\n  initial #1 \
         $display(\"D=W%0d o=%h\", W, o);",
        &["D=W4 o=20", "D=W7 o=100"],
    );
}

#[test]
fn a_zero_or_negative_msb_is_two_or_more_bits() {
    // review B B1 · `[W-1:0]` with `W = 0` is `[-1:0]` = TWO bits (§7.4.1: |msb − lsb| + 1);
    // the first draft's `msb + 1` gave 0 and lost the member — both oracles `bits=5`
    let src = |w: &str| {
        format!(
            "`timescale 1ns/1ns\nmodule dut #(parameter int W = 4) ();\n  typedef struct packed {{ \
             logic [W-1:0] a; logic [1:0] b; logic c; }} t;\n  t s;\n  initial begin\n    s = '0;\n    \
             s.b = 2'b11; s.c = 1'b1;\n    $display(\"D=W%0d bits=%0d s=%b b=%b\", W, $bits(s), s, \
             s.b);\n  end\nendmodule\nmodule top;\n  dut #(.W({w})) u0();\n  initial begin #1; \
             $finish; end\nendmodule\n"
        )
    };
    assert_eq!(d_lines(&src("0")), ["D=W0 bits=5 s=00111 b=11"]);
    assert_eq!(d_lines(&src("-2")), ["D=W-2 bits=7 s=0000111 b=11"]);
    assert_eq!(d_lines(&src("1")), ["D=W1 bits=4 s=0111 b=11"]);
}

#[test]
fn what_the_symbolic_layout_does_not_carry_stays_loud() {
    // s05 / s05b · a non-zero LSB and an ascending range (both oracles value them)
    loud(
        "  typedef struct packed { logic [W:1] p; } t_t;\n  t_t s; assign s.p = a;",
        "struct member width must be",
    );
    loud(
        "  typedef struct packed { logic [0:W-1] p; } t_t;\n  t_t s; assign s.p = a;",
        "struct member width must be",
    );
    // s06 / s06b · a member sub-select, read and write
    loud(
        &format!("{T}  t_t s; assign s.d = a; assign s.g = 1'b0;\n  initial #1 $display(\"D=%h\", s.d[1:0]);"),
        "a sub-select of a packed-struct member whose width names a header parameter",
    );
    loud(
        &format!("{T}  t_t s; assign s.d[1:0] = a[1:0]; assign s.g = 1'b0;"),
        "a sub-select write of a packed-struct member whose width names a header parameter",
    );
    // s14b · a chain into the nested symbolic member
    loud(
        "  typedef struct packed { logic g; logic [W-1:0] d; } in_t;\n  typedef struct packed { \
         logic f; in_t i; } out_t;\n  out_t o; assign o.f = 1'b1; assign o.i.d = a; assign o.i.g \
         = 1'b0;",
        "E-ELAB-UNRESOLVED-NAME",
    );
    // s23 · a union member
    loud(
        "  typedef union packed { logic [W-1:0] d; logic [W-1:0] e; } u_t;\n  u_t u; assign u.d = a;",
        "union member width must be",
    );
    // s20 · a width naming a VARIABLE (both oracles refuse): not an overridable
    // parameter, so the parser's refusal stands
    loud(
        "  int v = 3;\n  typedef struct packed { logic g; logic [v-1:0] d; } t_t;\n  t_t s; assign s.d = a;",
        "struct member width must be",
    );
}
