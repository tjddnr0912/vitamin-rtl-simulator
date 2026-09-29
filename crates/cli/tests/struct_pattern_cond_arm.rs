//! §3.a ⑤ⓚ, held: an assignment pattern `'{…}` as an arm of `?:` (or inside `( … )`)
//! whose target is a packed struct stays E3009. IEEE 1800-2017 §10.8 puts the second
//! and third operands of a conditional operator, and a parenthesized operand, in the
//! same assignment-like context as the whole value, so the pattern is typed by the
//! target; the corpus row `ibex` stops on one, `ibex_controller.sv:737`:
//! `exc_cause_o = irq_nm_ext_i ? ExcCauseIrqNm : '{irq_ext: 1'b0, irq_int: 1'b1, …};`.
//!
//! §4.5.571 resolved the arms against the target's struct in the parser three ways and
//! reverted all three after three review rounds on one axis: the parser keys its struct
//! bindings (`var_struct`, `struct_scalar_vars`, `struct_layouts`) by NAME, and a
//! function formal, a class property, an import, a later typedef or a block-local in a
//! labeled assertion's action block rewrites them, so an arm resolved against them was
//! wrong where the pre-slice binary was loud. The prerequisite is a certified binding of
//! the target (ROADMAP §3.a ⑤ⓚ).
//!
//! Every cell is pinned REFUSED with the oracles' lines beside it, so the admission slice
//! turns each comment into its pin. Oracles: verilator 5.052 (`--binary --timing`,
//! 2-state) and sv2v 0.0.13 → iverilog 13.0 (`-g2012`, 4-state). iverilog alone rejects
//! every keyed pattern; sv2v turns a positional pattern into an UNSIZED concatenation, so
//! on unsized positional elements verilator alone decides; verilator reads an `x` literal
//! as 0 and, measured, also zeroed earlier lines of a design that assigned one, so the
//! `x` cells are sv2v → iverilog's. Each test says which ran.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_spca_{}_{n}", std::process::id()));
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
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (all, out.status.code())
}

fn is_loud(src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(1), "expected a refusal\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in\n{out}");
}

const KEYED: &str =
    "a keyed assignment pattern `'{k: v, …}` is supported for a packed-struct target";
/// Every held cell: the refusal the pre-slice binary gives.
const E3009: &str = "error[VITA-E3009]";
const POS: &str = "an assignment pattern `'{…}` is supported only as the whole right-hand side";

/// ibex_pkg's `exc_cause_t` and one of its constants.
const PKG: &str = r#"
package p;
  typedef struct packed {
    logic       irq_int;
    logic       irq_ext;
    logic [4:0] lower_cause;
  } exc_cause_t;
  localparam exc_cause_t P = '{irq_ext: 1'b1, irq_int: 1'b0, lower_cause: 5'd31};
endpackage
"#;

#[test]
fn the_ibex_exc_cause_arm_is_held() {
    // sv2v → iverilog prints every line; verilator prints `a`, `b`, `d`, `e` alike
    // (and reads the `x` condition of `c` as 0, `1000011`).
    let src = format!(
        "{PKG}{}",
        r#"
module t;
  import p::*;
  exc_cause_t e;
  logic c;
  logic [4:0] cause;
  initial begin
    cause = 5'd3;
    c = 1'b1; e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: cause}; $display("a %b", e);
    c = 1'b0; e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: cause}; $display("b %b", e);
    c = 1'bx; e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: cause}; $display("c %b", e);
    c = 1'b1; e = c ? '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: cause} : P; $display("d %b", e);
    c = 1'b0; e = c ? '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: cause} : P; $display("e %b", e);
    cause = 5'bx1z10;
    c = 1'b0; e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: cause}; $display("f %b", e);
  end
endmodule
"#
    );
    // REFUSED; the oracles' lines: a 0111111 · b 1000011 · c xxxxx11 · d 1000011 · e 0111111 ·
    // f 10x1z10
    is_loud(&src, E3009);
}

#[test]
fn positional_default_nested_and_parenthesized_arms_are_held() {
    // verilator prints these lines; sv2v → iverilog agrees except `a` (`0000011`) and
    // `c` (`0000001`), where it turns the unsized positional pattern into an unsized
    // concatenation.
    let src = format!(
        "{PKG}{}",
        r#"
module t;
  import p::*;
  exc_cause_t e;
  logic c, d;
  logic [4:0] cause;
  initial begin
    cause = 5'd3;
    c = 0; e = c ? P : '{1, 0, 3}; $display("a %b", e);
    c = 0; e = c ? P : '{2, 3, 40}; $display("b %b", e);
    c = 1; e = c ? '{1, 1, 1} : '{0, 0, 2}; $display("c %b", e);
    c = 0; e = c ? '{1, 1, 1} : '{0, 0, 2}; $display("d %b", e);
    c = 0; d = 1; e = c ? '{irq_ext: 1, irq_int: 1, lower_cause: 1} : d ? '{irq_ext: 0, irq_int: 1, lower_cause: cause} : P; $display("e %b", e);
    c = 0; d = 0; e = c ? '{irq_ext: 1, irq_int: 1, lower_cause: 1} : d ? '{irq_ext: 0, irq_int: 1, lower_cause: cause} : P; $display("f %b", e);
    c = 0; e = c ? P : '{default: '1}; $display("g %b", e);
    c = 0; e = c ? P : '{irq_int: 0, default: 1}; $display("h %b", e);
    c = 0; e = c ? P : '{default: 0}; $display("i %b", e);
    c = 0; e = c ? P : '{default: 5'd6}; $display("j %b", e);
    c = 0; e = (c ? P : '{irq_ext: 0, irq_int: 1, lower_cause: cause}); $display("k %b", e);
    c = 1; e = c ? (d ? P : '{0, 1, 5'd9}) : P; $display("l %b", e);
    c = 0; e = c ? P : '{irq_ext: 0, irq_int: 1, lower_cause: cause + 5'd30}; $display("m %b", e);
    c = 0; e = c ? P : '{irq_ext: 0, irq_int: 1, lower_cause: {1'b1, cause[3:0]}}; $display("n %b", e);
    e = ('{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd11}); $display("o %b", e);
  end
endmodule
"#
    );
    // REFUSED; the oracles' lines: a 1000011 · b 0101000 · c 1100001 · d 0000010 · e 1000011 ·
    // f 0111111 · g 1111111 · h 0100001 · i 0000000 · j 0000110 · k 1000011 · l 0101001 ·
    // m 1000001 · n 1010011 · o 1001011
    is_loud(&src, E3009);
}

#[test]
fn continuous_combinational_nonblocking_and_force_are_held() {
    // verilator and sv2v → iverilog print these lines.
    let src = format!(
        "{PKG}{}",
        r#"
module t;
  import p::*;
  exc_cause_t e2, e3, e4, e5;
  logic c, clk;
  logic [4:0] cause;
  assign e2 = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: cause};
  always_comb e3 = c ? '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: cause} : P;
  always @(posedge clk) e4 <= c ? P : '{irq_ext: 1'b1, irq_int: 1'b1, lower_cause: cause + 5'd1};
  initial begin
    clk = 0; cause = 5'd3; c = 0;
    #1 clk = 1; #1 $display("a %b %b %b", e2, e3, e4);
    c = 1; clk = 0; #1 clk = 1; #1 $display("b %b %b %b", e2, e3, e4);
    cause = 5'd9; c = 0; clk = 0; #1 clk = 1; #1 $display("c %b %b %b", e2, e3, e4);
    force e5 = c ? P : '{irq_ext: 1'b1, irq_int: 1'b0, lower_cause: 5'd2}; #1 $display("d %b", e5);
    release e5;
    $finish;
  end
endmodule
"#
    );
    // REFUSED; the oracles' lines: a 1000011 0111111 1100100 · b 0111111 1000011 0111111 ·
    // c 1001001 0111111 1101010 · d 0100010
    is_loud(&src, E3009);
}

#[test]
fn the_other_arms_width_sign_and_real_cells_are_held() {
    // verilator and sv2v → iverilog print these lines: the pattern arm is an unsigned
    // 7-bit value, so a narrower signed variable arm zero-extends (`e`) while a
    // negated literal is negated at 7 bits (`d`, `f`, `j`), and a real arm makes the
    // conditional real (`g`: 2.6 rounds to 3).
    let src = format!(
        "{PKG}{}",
        r#"
module t;
  import p::*;
  exc_cause_t e;
  logic c;
  logic [4:0] cause;
  logic signed [2:0] sn;
  initial begin
    cause = 5'd3; sn = -3'sd1;
    c = 1; e = c ? 3'b101 : '{irq_ext: 0, irq_int: 1, lower_cause: cause}; $display("a %b", e);
    c = 1; e = c ? 9'h1ff : '{irq_ext: 0, irq_int: 1, lower_cause: cause}; $display("b %b", e);
    c = 0; e = c ? 9'h1ff : '{irq_ext: 0, irq_int: 1, lower_cause: cause}; $display("c %b", e);
    c = 1; e = c ? -3'sd1 : '{irq_ext: 0, irq_int: 1, lower_cause: cause}; $display("d %b", e);
    c = 1; e = c ? sn : '{irq_ext: 0, irq_int: 1, lower_cause: cause}; $display("e %b", e);
    c = 1; e = c ? -3'sd1 : '{0, 1, cause}; $display("f %b", e);
    c = 1; e = c ? 2.6 : '{irq_ext: 0, irq_int: 1, lower_cause: cause}; $display("g %b", e);
    c = 0; e = c ? 2.6 : '{irq_ext: 0, irq_int: 1, lower_cause: cause}; $display("h %b", e);
    c = 1; e = c ? 1 : '{irq_ext: 0, irq_int: 1, lower_cause: cause}; $display("i %b", e);
    c = 1; e = c ? -1 : '{irq_ext: 0, irq_int: 1, lower_cause: cause}; $display("j %b", e);
  end
endmodule
"#
    );
    // REFUSED; the oracles' lines: a 0000101 · b 1111111 · c 1000011 · d 1111111 · e 0000111 ·
    // f 1111111 · g 0000011 · h 1000011 · i 0000001 · j 1111111
    is_loud(&src, E3009);
}

#[test]
fn a_signed_and_a_two_state_struct_are_held() {
    // verilator and sv2v → iverilog print these lines. Both read the pattern arm of a
    // `struct packed signed` target as unsigned: the signed 3-bit `sn` zero-extends
    // (`b`), where a `ss_t` variable arm would sign-extend it.
    let src = r#"
package p;
  typedef struct packed signed {
    logic       a;
    logic [3:0] b;
  } ss_t;
  typedef struct packed {
    bit       a;
    bit [3:0] b;
  } s2_t;
endpackage
module t;
  import p::*;
  ss_t s;
  s2_t z;
  logic c;
  logic signed [2:0] sn;
  logic [3:0] v;
  initial begin
    sn = -3'sd1; v = 4'b0110;
    c = 1; s = c ? -3'sd1 : '{a: 0, b: 4'd3}; $display("a %b", s);
    c = 1; s = c ? sn : '{a: 0, b: 4'd3}; $display("b %b", s);
    c = 0; s = c ? sn : '{a: 1, b: 4'd3}; $display("c %b", s);
    c = 1; s = c ? -3'sd1 : '{0, 4'd3}; $display("d %b", s);
    c = 0; z = c ? '{a: 1, b: 4'd9} : '{a: 0, b: v}; $display("f %b", z);
    c = 1; z = c ? '{a: 1, b: 4'd9} : '{a: 0, b: v}; $display("g %b", z);
    c = 1; z = c ? 3'sb111 : '{a: 0, b: v}; $display("h %b", z);
  end
endmodule
"#;
    // REFUSED; the oracles' lines: a 11111 · b 00111 · c 10011 · d 11111 · f 00110 · g 11001 ·
    // h 00111
    is_loud(src, E3009);
}

#[test]
fn a_merged_x_into_a_two_state_struct_is_held() {
    // An `x` condition merges the arms bit by bit (IEEE 1800-2017 §11.4.11) and the
    // all-2-state struct stores each unknown bit as 0 (§6.11.3). No oracle keeps `bit`
    // (sv2v turns it into `logic`, verilator reads the `x` condition as 0); iverilog
    // prints the lines `b` and `d` of the same merge between two constants, `00000`
    // and `10001`, and the pattern arms print the same.
    let src = r#"
module t;
  typedef struct packed { bit a; bit [3:0] b; } z_t;
  localparam z_t Z1 = 5'b11001;
  localparam z_t Z2 = 5'b00110;
  localparam z_t Z3 = 5'b10001;
  z_t z; logic cx;
  initial begin
    cx = 1'bx;
    z = cx ? '{a: 1, b: 4'b1001} : '{a: 0, b: 4'b0110}; $display("a %b", z);
    z = cx ? Z1 : Z2; $display("b %b", z);
    z = cx ? '{a: 1, b: 4'b1001} : '{a: 1, b: 4'b0001}; $display("c %b", z);
    z = cx ? Z1 : Z3; $display("d %b", z);
    z = cx ? '{a: 1, b: 4'b1001} : Z3; $display("e %b", z);
  end
endmodule
"#;
    // REFUSED; the oracles' lines: a 00000 · b 00000 · c 10001 · d 10001 · e 10001
    is_loud(src, E3009);
}

#[test]
fn an_alias_and_a_unit_local_struct_are_held() {
    // verilator and sv2v → iverilog print these lines.
    let src = format!(
        "{PKG}{}",
        r#"
module t;
  import p::*;
  typedef exc_cause_t my_t;
  typedef struct packed { logic [1:0] a; logic [2:0] b; } loc_t;
  my_t e3;
  loc_t e4;
  logic c;
  initial begin
    c = 0;
    e3 = c ? P : '{irq_ext: 1'b1, irq_int: 1'b1, lower_cause: 5'd21}; $display("a %b", e3);
    e4 = c ? 5'd0 : '{a: 2'd2, b: 3'd5}; $display("b %b", e4);
    e4 = !c ? '{2'd1, 3'd6} : 5'd0; $display("c %b", e4);
  end
endmodule
"#
    );
    // REFUSED; the oracles' lines: a 1110101 · b 10101 · c 01110
    is_loud(&src, E3009);
}

#[test]
fn other_binders_of_a_whole_pattern_stay_refused() {
    // Both oracles run every one of these; the values are verilator's (sv2v → iverilog
    // agrees): `L1 1000100`, `d1 1100110`, `arr[1] 1001000`, `w 01010011010` after
    // the member write, `11010100011` after the nested one, `ai 0000111`; the queue
    // `1101111` (verilator only). A member of a 4-state struct whose own type is
    // 2-state (`m.i = cx ? … : …`) and a 2-state queue element keep a merged `x` that
    // IEEE 1800-2017 §7.2.1 / §6.11.3 convert to 0.
    let one = |decl: &str, stmt: &str| {
        format!(
            "{PKG}\nmodule t;\n  import p::*;\n  typedef struct packed {{ exc_cause_t x; logic [3:0] y; }} w_t;\n  localparam bit C0 = 1'b0;\n{decl}\n  logic c;\n  integer i;\n  initial begin c = 0; i = 1; {stmt} end\nendmodule\n"
        )
    };
    // The localparam is refused by the parameter fold instead.
    is_loud(
        &one(
            "  localparam exc_cause_t L1 = C0 ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd4};",
            "$display(\"%b\", L1);",
        ),
        "parameter `L1` value is not a constant",
    );
    let cases = [
        (
            "  exc_cause_t d1 = C0 ? P : '{irq_ext: 1'b1, irq_int: 1'b1, lower_cause: 5'd6};",
            "$display(\"%b\", d1);",
        ),
        (
            "  exc_cause_t arr [2];",
            "arr[i] = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd8}; $display(\"%b\", arr[1]);",
        ),
        (
            "  w_t w;",
            "w.x = c ? P : '{irq_ext: 1'b1, irq_int: 1'b0, lower_cause: 5'd9}; $display(\"%b\", w);",
        ),
        (
            "  w_t w;",
            "w = '{x: c ? P : '{irq_ext: 1'b1, irq_int: 1'b1, lower_cause: 5'd10}, y: 4'h3}; $display(\"%b\", w);",
        ),
        (
            "  exc_cause_t ai [2] = '{C0 ? P : '{irq_ext: 1'b0, irq_int: 1'b0, lower_cause: 5'd7}, P};",
            "$display(\"%b\", ai[0]);",
        ),
        (
            "  exc_cause_t q[$];",
            "q.push_back(c ? P : '{irq_ext: 1'b1, irq_int: 1'b1, lower_cause: 5'd15}); $display(\"%b\", q[0]);",
        ),
    ];
    for (decl, stmt) in cases {
        is_loud(&one(decl, stmt), KEYED);
    }
}

#[test]
fn a_binding_the_parser_cannot_certify_stays_refused() {
    // Each is refused although both oracles run it. The parser keys struct bindings by
    // name: a function formal, a class property, a block-local, a loop variable or a
    // generate scope may declare the name again without unbinding it, an import binds
    // a package variable under a port of another type, and a vector alias or a type
    // parameter may leave a struct's layout under its name. The same writes spelled
    // as a whole `'{…}` are already wrong on the pre-slice binary where the binding is
    // wrong (a function formal `logic [1:0][3:0] s`: `42` where both oracles read
    // `12`); they are the class a scope-correct binding closes.
    let in_module = |body: &str| {
        format!(
            "{PKG}\nmodule t;\n  import p::*;\n  typedef struct packed {{ exc_cause_t n; logic [3:0] z; }} nest_t;\n  exc_cause_t e;\n  nest_t w;\n  logic c;\n{body}\nendmodule\n"
        )
    };
    let bodies = [
        // A function body.
        "  function automatic void f(input logic cc);\n    e = cc ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd1};\n  endfunction\n  initial f(1'b0);",
        // A generate block.
        "  if (1) begin : g\n    always @(c) e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd2};\n  end",
        // A block that declares a name.
        "  initial begin : blk\n    int k;\n    k = 0;\n    e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd3};\n  end",
        // A typed `for` and a `foreach`.
        "  initial for (int i = 0; i < 1; i++) e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd4};",
        "  int a [2];\n  initial foreach (a[j]) e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd5};",
        // A class method.
        "  class C;\n    exc_cause_t m;\n    function void set(bit cc); m = cc ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd6}; endfunction\n  endclass",
        // A struct with a nested struct member.
        "  initial w = c ? 11'd0 : '{n: P, z: 4'd5};",
    ];
    for body in bodies {
        is_loud(&in_module(body), KEYED);
    }
    // A function formal of another type named like a module struct variable (both
    // oracles `f1=12 f0=34`).
    is_loud(
        r#"
module t;
  typedef struct packed { logic [1:0] a; logic [5:0] b; } st;
  st s;
  function automatic logic [7:0] f(input logic c, input logic [1:0][3:0] s);
    s = c ? '{4'h1, 4'h2} : '{4'h3, 4'h4};
    return s;
  endfunction
  initial $display("f1=%h f0=%h", f(1'b1, 8'h0), f(1'b0, 8'h0));
endmodule
"#,
        POS,
    );
    // An ANSI port of another type after a header import that binds a package struct
    // variable of the same name (both oracles `s=12`).
    is_loud(
        r#"
package p;
  typedef struct packed { logic [1:0] a; logic [5:0] b; } st;
  st s;
endpackage
module t import p::*; (output logic [1:0][3:0] s);
  logic c;
  initial begin
    c = 1;
    s = c ? '{4'h1, 4'h2} : '{4'h3, 4'h4};
    $display("s=%h", s);
  end
endmodule
"#,
        POS,
    );
    // A class property of another struct type named like the module's variable
    // (verilator `s=12`).
    is_loud(
        r#"
module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st1;
  typedef struct packed { logic [1:0] a; logic [5:0] b; } st2;
  st1 s;
  class C;
    st2 s;
  endclass
  logic c;
  initial begin
    c = 1; s = c ? '{4'h1, 4'h2} : '{4'h3, 4'h4};
    $display("s=%h", s);
  end
endmodule
"#,
        POS,
    );
    // A vector alias redeclaring an imported struct's name, and a type parameter
    // named like a `$unit` struct (both oracles `v=12`, `m2 v=12`).
    is_loud(
        r#"
package p;
  typedef struct packed { logic [1:0] a; logic [5:0] b; } st;
endpackage
module t;
  import p::*;
  typedef logic [1:0][3:0] st;
  st v;
  logic c;
  initial begin
    c = 1;
    v = c ? '{4'h1, 4'h2} : '{4'h3, 4'h4};
    $display("v=%h", v);
  end
endmodule
"#,
        POS,
    );
    is_loud(
        r#"
typedef struct packed { logic [1:0] a; logic [5:0] b; } st;
module m2 #(parameter type st = logic [1:0][3:0]) ();
  st v;
  logic c;
  initial begin
    c = 1;
    v = c ? '{4'h1, 4'h2} : '{4'h3, 4'h4};
    #1 $display("m2 v=%h", v);
  end
endmodule
module t;
  m2 u2();
endmodule
"#,
        POS,
    );
}

#[test]
fn a_shape_or_layout_changed_since_the_declaration_stays_refused() {
    // The unit's declaration is recorded with its layout; nothing read at the
    // assignment may replace it. A packed ARRAY of the struct whose name a class
    // property of the struct type later binds as a scalar (verilator `C 1234`, sv2v
    // cannot parse the class), a wildcard import after the declaration that swaps
    // the name's type and layout together (both oracles `A 06`), and a later typedef
    // of the name (sv2v → iverilog `v=12`; verilator refuses the reference before the
    // declaration).
    is_loud(
        r#"
typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
module t;
  logic c;
  st [1:0] s;
  class C;
    st s;
  endclass
  initial begin
    c = 1; s = c ? '{8'h12, 8'h34} : '{8'h56, 8'h78}; $display("C %h", s);
  end
endmodule
"#,
        POS,
    );
    is_loud(
        r#"
typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
package p;
  typedef struct packed { logic [3:0] a; logic [7:0] b; } st;
endpackage
module t;
  logic c;
  typedef struct packed { logic [5:0] a; logic [1:0] b; } st;
  st s;
  import p::*;
  initial begin
    c = 1; s = c ? '{a: 6'h1, b: 2'h2} : '{a: 6'h3, b: 2'h1}; $display("A %h", s);
  end
endmodule
"#,
        KEYED,
    );
    is_loud(
        r#"
typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
module t;
  st v;
  typedef struct packed { logic [1:0] a; logic [5:0] b; } st;
  logic c;
  initial begin
    c = 1;
    v = c ? '{4'h1, 4'h2} : '{4'h3, 4'h4};
    $display("v=%h", v);
  end
endmodule
"#,
        POS,
    );
}

#[test]
fn a_block_local_in_a_labeled_assertions_action_block_stays_refused() {
    // The third design of §4.5.571 recorded a unit-level declaration where it was
    // classified; a labeled concurrent assertion is a unit-level item, and a block-local
    // of the same name in its action block re-recorded the name. verilator prints
    // `A1 x=46 a=11 b=2 bits=8 y=46` / `A0 x=0d a=03 b=1 y=0d` (sv2v → iverilog agrees
    // but for `y=xx` at A1, a time-0 `always @(c)` event), and `G1 bits=16 x=1234` /
    // `G0 x=5678` for the packed array.
    is_loud(
        r#"
module t;
  typedef struct packed { logic [5:0] a; logic [1:0] b; } st1;
  typedef struct packed { logic [1:0] a; logic [5:0] b; } st2;
  logic clk = 0;
  logic c = 1;
  st1 x;
  st1 y;
  a1: assert property (@(posedge clk) 1'b1) else begin st2 x; x = '0; $display("fx %h", x); end
  a2: assert property (@(posedge clk) 1'b1) else begin st2 y; y = '0; $display("fy %h", y); end
  assign x = c ? '{a: 6'h11, b: 2'h2} : '{a: 6'h3, b: 2'h1};
  always @(c) y = c ? '{6'h11, 2'h2} : '{6'h3, 2'h1};
  initial begin
    #1 $display("A1 x=%h a=%h b=%h bits=%0d y=%h", x, x.a, x.b, $bits(x), y);
    c = 0;
    #1 $display("A0 x=%h a=%h b=%h y=%h", x, x.a, x.b, y);
    $finish;
  end
endmodule
"#,
        E3009,
    );
    is_loud(
        r#"
typedef struct packed { logic [5:0] a; logic [1:0] b; } st1;
module t;
  logic clk = 0;
  logic c = 1;
  st1 [1:0] x;
  a1: assert property (@(posedge clk) 1'b1) begin st1 x; x = '0; end
  assign x = c ? '{8'h12, 8'h34} : '{8'h56, 8'h78};
  initial begin #1 $display("G1 bits=%0d x=%h", $bits(x), x); c = 0; #1 $display("G0 x=%h", x); $finish; end
endmodule
"#,
        POS,
    );
}

#[test]
fn an_inexact_or_non_struct_target_stays_refused() {
    let wrap = |decl: &str, stmt: &str| {
        format!("module t;\n{decl}\n  logic c;\n  initial begin c = 0; {stmt} end\nendmodule\n")
    };
    // A 2-state member of a 4-state struct, and an `enum bit` member: a merged `x`
    // would stay in it.
    is_loud(
        &wrap(
            "  typedef struct packed { bit a; logic [3:0] b; } m_t;\n  m_t m;",
            "m = c ? 5'd1 : '{a: 1, b: 4'd2}; $display(\"%b\", m);",
        ),
        KEYED,
    );
    is_loud(
        &wrap(
            "  typedef enum bit [1:0] {E0, E1, E2, E3} eb_t;\n  typedef struct packed { eb_t st; logic [2:0] b; } e_t;\n  e_t q;",
            "q = c ? 5'd1 : '{st: E2, b: 3'd1}; $display(\"%b\", q);",
        ),
        KEYED,
    );
    // A member bound written as a name.
    is_loud(
        &wrap(
            "  localparam int W = 4;\n  typedef struct packed { logic a; logic [W-1:0] b; } n_t;\n  n_t n;",
            "n = c ? 5'd1 : '{a: 1, b: 4'd2}; $display(\"%b\", n);",
        ),
        KEYED,
    );
    // A union, and a target that is not a struct.
    is_loud(
        &wrap(
            "  typedef union packed { logic [3:0] a; logic [3:0] b; } u_t;\n  u_t u;",
            "u = c ? 4'd1 : '{a: 4'd2}; $display(\"%b\", u);",
        ),
        KEYED,
    );
    is_loud(
        &wrap(
            "  logic [4:0] v;",
            "v = c ? 5'd1 : '{a: 1, b: 4'd2}; $display(\"%b\", v);",
        ),
        KEYED,
    );
}

#[test]
fn a_pattern_outside_the_arms_stays_refused() {
    let wrap = |stmt: &str| {
        format!(
            "module t;\n  typedef struct packed {{ logic a; logic [3:0] b; }} s_t;\n  s_t s, r;\n  logic c;\n  initial begin c = 0; r = 5'd3; {stmt} end\nendmodule\n"
        )
    };
    // Inside an operator, a concatenation, and the condition.
    is_loud(
        &wrap("s = (c ? '{a: 1, b: 4'd2} : '{a: 0, b: 4'd2}) | 5'd0; $display(\"%b\", s);"),
        KEYED,
    );
    is_loud(
        &wrap("s = {c ? '{a: 1, b: 4'd2} : 5'd3}; $display(\"%b\", s);"),
        KEYED,
    );
    is_loud(
        &wrap("s = ('{a: 1, b: 4'd2} == r) ? r : 5'd4; $display(\"%b\", s);"),
        KEYED,
    );
    // A `return` and a function argument refuse a whole pattern too.
    let src = format!(
        "{PKG}{}",
        r#"
module t;
  import p::*;
  exc_cause_t e;
  function automatic exc_cause_t f2(input logic cc);
    return cc ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd13};
  endfunction
  function automatic exc_cause_t f3(input exc_cause_t a);
    return a;
  endfunction
  initial begin
    e = f2(1'b0); $display("b %b", e);
    e = f3(1'b0 ? P : '{irq_ext: 1'b0, irq_int: 1'b0, lower_cause: 5'd17}); $display("d %b", e);
  end
endmodule
"#
    );
    is_loud(&src, KEYED);
}
