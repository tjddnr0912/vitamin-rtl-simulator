//! §3.a ⑤ⓚ (§4.5.574): an assignment pattern `'{…}` as an arm of `?:` (or inside `( … )`)
//! whose target is a packed struct. IEEE 1800-2017 §10.8 puts the second and third operands
//! of a conditional operator, and a parenthesized operand, in the same assignment-like
//! context as the whole value, so the pattern is typed by the target; the corpus row `ibex`
//! stopped on one, `ibex_controller.sv:737`:
//! `exc_cause_o = irq_nm_ext_i ? ExcCauseIrqNm : '{irq_ext: 1'b0, irq_int: 1'b1, …};`.
//!
//! §4.5.571 resolved the arms in the parser and was reverted (its struct bindings are keyed
//! by NAME); §4.5.572 recorded each declaration's members and resolved the arm in elaborate,
//! and was reverted on two prerequisites: the block-local scope-leak check walks no `force`,
//! and a wildcard import replaced a local typedef in the parser's binding (closed in §4.5.573).
//! §4.5.574 is §4.5.572's design without `force` (elaborate `struct_arm.rs`): the members the
//! parser recorded at the target's declaration, on the net the lowering resolves the target to.
//!
//! A value cell is pinned to the oracles' lines; a refusal keeps the oracles' lines in its
//! comment. Oracles: verilator 5.052 (`--binary --timing`,
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

/// The design's own output lines (a clean run, exit 0): vita's diagnostics and status
/// lines dropped.
fn runs(src: &str) -> Vec<String> {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "expected a clean run\n{src}\n{out}");
    out.lines()
        .filter(|l| {
            !l.contains("[VITA-") && !l.starts_with("simulation ended") && !l.starts_with("errors=")
        })
        .map(str::to_string)
        .collect()
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
fn the_ibex_exc_cause_arm_is_typed_by_its_target() {
    // sv2v → iverilog prints every line; verilator prints `a` and `e` alike and is 2-state
    // (it reads the `x` condition and every `x` / `z` bit as 0).
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
    assert_eq!(
        runs(&src),
        [
            "a 0111111",
            "b 1000011",
            "c xxxxx11",
            "d 1000011",
            "e 0111111",
            "f 10x1z10"
        ]
    );
}

#[test]
fn positional_default_nested_and_parenthesized_arms_are_typed() {
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
    assert_eq!(
        runs(&src),
        [
            "a 1000011",
            "b 0101000",
            "c 1100001",
            "d 0000010",
            "e 1000011",
            "f 0111111",
            "g 1111111",
            "h 0100001",
            "i 0000000",
            "j 0000110",
            "k 1000011",
            "l 0101001",
            "m 1000001",
            "n 1010011",
            "o 1001011"
        ]
    );
}

#[test]
fn continuous_combinational_and_nonblocking_arms_are_typed() {
    // verilator and sv2v → iverilog print these lines. `force` is not rewritten: the
    // block-local scope-leak check walks no `force` statement (docs/PROBE_CATALOG.md), so
    // the same arm under `force` keeps the refusal (the oracles print `d 0100010`).
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
    is_loud(&src, KEYED);
    let src = src
        .replace("    force e5 = c ? P : '{irq_ext: 1'b1, irq_int: 1'b0, lower_cause: 5'd2}; #1 $display(\"d %b\", e5);\n", "")
        .replace("    release e5;\n", "");
    assert_eq!(
        runs(&src),
        [
            "a 1000011 0111111 1100100",
            "b 0111111 1000011 0111111",
            "c 1001001 0111111 1101010"
        ]
    );
}

#[test]
fn the_other_arms_width_sign_and_real_cells_are_typed() {
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
    assert_eq!(
        runs(&src),
        [
            "a 0000101",
            "b 1111111",
            "c 1000011",
            "d 1111111",
            "e 0000111",
            "f 1111111",
            "g 0000011",
            "h 1000011",
            "i 0000001",
            "j 1111111"
        ]
    );
}

#[test]
fn a_signed_and_a_two_state_struct_are_typed() {
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
    assert_eq!(
        runs(src),
        ["a 11111", "b 00111", "c 10011", "d 11111", "f 00110", "g 11001", "h 00111"]
    );
}

#[test]
fn a_merged_x_into_a_two_state_struct_is_zero() {
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
    assert_eq!(
        runs(src),
        ["a 00000", "b 00000", "c 10001", "d 10001", "e 10001"]
    );
}

#[test]
fn an_alias_and_a_module_local_struct_are_typed() {
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
    assert_eq!(runs(&src), ["a 1110101", "b 10101", "c 01110"]);
}

#[test]
fn other_binders_of_a_whole_pattern_stay_refused() {
    // Both oracles run every one of these; the values are verilator's (sv2v → iverilog
    // agrees): `L1 1000100`, `arr[1] 1001000`, `w 01010011010` after the member write,
    // `11010100011` after the nested one, `ai 0000111`; the queue `1101111` (verilator
    // only). A module variable's declaration initializer is lowered as its first write
    // and takes the arm like `=` (`d1 1100110`, both oracles). A member of a 4-state struct whose own type is
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
    assert_eq!(
        runs(&one(
            "  exc_cause_t d1 = C0 ? P : '{irq_ext: 1'b1, irq_int: 1'b1, lower_cause: 5'd6};",
            "$display(\"%b\", d1);",
        )),
        ["1100110"]
    );
    let cases = [
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
fn a_target_the_declaration_record_does_not_reach_stays_refused() {
    // §4.5.571's cells. The members come from the target's declaration and the net the
    // lowering resolves the target to, so a process in a generate block, a named block
    // that declares a name, a typed `for` and a `foreach` writing a module variable take
    // the arm (`the_module_scopes_ports_and_values_of_4_5_572` prints their values), and
    // so does a class property named like a module variable (`s=12`, verilator; sv2v has
    // no classes). A function body, a class method, a struct with a nested struct member,
    // a function formal, a port or alias of a non-struct type and a type parameter keep
    // the refusal although both oracles run them.
    let in_module = |body: &str| {
        format!(
            "{PKG}\nmodule t;\n  import p::*;\n  typedef struct packed {{ exc_cause_t n; logic [3:0] z; }} nest_t;\n  exc_cause_t e;\n  nest_t w;\n  logic c;\n{body}\nendmodule\n"
        )
    };
    for body in [
        "  if (1) begin : g\n    always @(c) e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd2};\n  end",
        "  initial begin : blk\n    int k;\n    k = 0;\n    e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd3};\n  end",
        "  initial for (int i = 0; i < 1; i++) e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd4};",
        "  int a [2];\n  initial foreach (a[j]) e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd5};",
    ] {
        assert!(runs(&in_module(body)).is_empty());
    }
    let bodies = [
        // A function body.
        "  function automatic void f(input logic cc);\n    e = cc ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd1};\n  endfunction\n  initial f(1'b0);",
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
    assert_eq!(
        runs(
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
"#
        ),
        ["s=12"]
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
fn a_shape_or_layout_changed_since_the_declaration() {
    // A packed ARRAY of the struct whose name a class property of the struct type later
    // binds as a scalar stays refused (verilator `C 1234`, sv2v cannot parse the class):
    // no packed array records members. A wildcard import after the declaration leaves
    // the module's own typedef alone (§4.5.573), so the arm takes it (both oracles `A
    // 06`). A later typedef of the name drops the declaration's record (sv2v → iverilog
    // `v=12`; verilator refuses the reference before the declaration, IEEE 1800-2023
    // §6.18).
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
    assert_eq!(
        runs(
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
"#
        ),
        ["A 06"]
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
fn a_block_local_in_a_labeled_assertions_action_block() {
    // The third design of §4.5.571 recorded a unit-level declaration where it was
    // classified; a labeled concurrent assertion is a unit-level item, and a block-local
    // of the same name in its action block re-recorded the name. The members are the
    // module variable's own, recorded at its declaration: sv2v → iverilog prints the
    // line below, verilator the same but `y=46` at A1 (a time-0 `always @(c)` event it
    // runs and iverilog does not). The packed array stays refused (verilator and sv2v →
    // iverilog `G1 bits=16 x=1234` / `G0 x=5678`).
    assert_eq!(
        runs(
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
"#
        ),
        ["A1 x=46 a=11 b=2 bits=8 y=xx", "A0 x=0d a=03 b=1 y=0d"]
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

#[test]
fn the_module_scopes_ports_and_values_of_4_5_572() {
    // §4.5.572's cells, each with the oracles' lines. A process in a generate block, a named
    // block that declares a name, a typed `for`, a `foreach` and a nested named block
    // writing module variables (verilator and sv2v → iverilog: `A 1000010 1000011
    // 1000100 1000001 1101001` · `B 1001100` · `C 0111111`).
    assert_eq!(
        runs(&format!(
            "{PKG}{}",
            r#"
module t;
  import p::*;
  exc_cause_t e1, e2, e3, e4, e5;
  logic c;
  logic [4:0] cause;
  int a [2];
  if (1) begin : g
    always @(c or cause) e1 = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: cause};
  end
  initial begin : blk
    int k;
    #1 k = 0;
    e2 = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd3};
    for (int i = 0; i < 1; i++) e3 = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'd4};
    foreach (a[j]) e4 = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: 5'(j)};
    begin : inner
      e5 = !c ? '{irq_ext: 1'b1, irq_int: 1'b1, lower_cause: 5'd9} : P;
    end
    #1 $display("A %b %b %b %b %b", e1, e2, e3, e4, e5);
    cause = 5'd12; #1 $display("B %b", e1);
    c = 1; #1 $display("C %b", e1);
    $finish;
  end
  initial begin c = 0; cause = 5'd2; end
endmodule
"#
        )),
        [
            "A 1000010 1000011 1000100 1000001 1101001",
            "B 1001100",
            "C 0111111"
        ]
    );
    // The ibex shape: an ANSI output port of a package struct connected to a parent net
    // of another struct type (`po.q` reads the child's bits through the parent's
    // layout), `always_comb` following `cause` (verilator and sv2v → iverilog `A 1000011
    // 1000011 q=4` · `B 1001100 1001100` · `C 0111111 0111111`); a non-ANSI port whose
    // type a body declaration gives (verilator `A 1100111` · `C 0111111`; sv2v cannot
    // convert it).
    assert_eq!(
        runs(&format!(
            "{PKG}{}",
            r#"
module sub import p::*; (input logic c, input logic [4:0] cause, output exc_cause_t o);
  always_comb o = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: cause};
endmodule
module sub2(c, o);
  import p::*;
  input logic c;
  output o;
  exc_cause_t o;
  always_comb o = c ? P : '{irq_ext: 1'b1, irq_int: 1'b1, lower_cause: 5'd7};
endmodule
module t;
  import p::*;
  typedef struct packed { logic [2:0] q; logic [3:0] r; } other_t;
  logic c; logic [4:0] cause;
  other_t po;
  exc_cause_t e, e2;
  sub u(.c(c), .cause(cause), .o(po));
  sub2 u2(.c(c), .o(e2));
  always_comb e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: cause};
  initial begin
    c = 0; cause = 5'd3;
    #1 $display("A %b %b q=%h %b", e, po, po.q, e2);
    cause = 5'd12;
    #1 $display("B %b %b", e, po);
    c = 1;
    #1 $display("C %b %b %b", e, po, e2);
    $finish;
  end
endmodule
"#
        )),
        [
            "A 1000011 1000011 q=4 1100111",
            "B 1001100 1001100",
            "C 0111111 0111111 0111111"
        ]
    );
    // Fills on 4-state and 2-state members, and a call in an arm evaluated only when the
    // arm is taken: sv2v → iverilog `a xxxxxxx` · `b zzzzzzz` (verilator reads the `x` /
    // `z` as 0); no oracle keeps `bit`, and iverilog prints `00101` for the
    // member-by-member twin of `d`, which IEEE 1800-2017 §6.11.3 gives `c 00000` · `d
    // 00101` too; verilator and sv2v → iverilog `A 0111111 n=0` · `f called n=1` · `B
    // 1000011 n=1`.
    assert_eq!(
        runs(&format!(
            "{PKG}{}",
            r#"
module t;
  import p::*;
  typedef struct packed { bit a; bit [3:0] b; } z_t;
  exc_cause_t e;
  z_t z;
  logic c;
  int n;
  function automatic logic [4:0] f(input logic [4:0] x);
    n = n + 1;
    $display("f called n=%0d", n);
    return x + 5'd1;
  endfunction
  initial begin
    c = 0; n = 0;
    e = c ? P : '{default: 'x}; $display("a %b", e);
    e = c ? P : '{default: 'z}; $display("b %b", e);
    z = c ? '{a: 1, b: 4'b1001} : '{default: 'x}; $display("c %b", z);
    z = c ? '{a: 1, b: 4'b1001} : '{a: 1'bx, b: 4'bx1z1}; $display("d %b", z);
    c = 1; e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: f(5'd2)}; $display("A %b n=%0d", e, n);
    c = 0; e = c ? P : '{irq_ext: 1'b0, irq_int: 1'b1, lower_cause: f(5'd2)}; $display("B %b n=%0d", e, n);
  end
endmodule
"#
        )),
        [
            "a xxxxxxx",
            "b zzzzzzz",
            "c 00000",
            "d 00101",
            "A 0111111 n=0",
            "f called n=1",
            "B 1000011 n=1"
        ]
    );
}

#[test]
fn the_review_shapes_of_4_5_572() {
    // The cells §4.5.572's three rounds found, with the oracles' lines. A keyed read of an
    // outer block-local, the nested block declaring its own `v` (verilator and sv2v →
    // iverilog `s=12`): in a blocking assignment the scope-leak check now reads the arm's
    // keyed values and refuses the nested `v` (`struct_arm::arm_reads_ident`) instead of
    // letting it share the outer `v`'s net; `force` is not rewritten (the check walks no
    // `force`, and the concatenation twin `force s = c ? {v, 4'd2} : 8'h00;` prints `52`).
    for stmt in ["s = ", "force s = "] {
        is_loud(
            &format!(
                r#"
module t;
  typedef struct packed {{ logic [3:0] a; logic [3:0] b; }} s_t;
  s_t s;
  logic c;
  initial begin
    logic [3:0] v = 4'd1;
    c = 1;
    #1;
    begin
      logic [3:0] v;
      v = 4'd5;
    end
    {stmt}c ? '{{a: v, b: 4'd2}} : 8'h00;
    #1 $display("s=%h", s);
  end
endmodule
"#
            ),
            E3009,
        );
    }
    // A wildcard import over a local typedef of the name: a `$unit` one (verilator and
    // sv2v → iverilog `U1 06 bits=8`) and a module's own (`G4 06 bits=8`). The parser's
    // binding is right since §4.5.573, and the arm takes the declaration's members.
    for (src, want) in [
        (
            r#"
package p;
  typedef struct packed { logic [3:0] a; logic [7:0] b; } st;
endpackage
typedef struct packed { logic [5:0] a; logic [1:0] b; } st;
import p::*;
module t;
  logic c;
  st s2;
  initial begin
    c = 0; #1;
    s2 = c ? '{a: 6'h3, b: 2'h1} : '{a: 6'h1, b: 2'h2};
    $display("U1 %h bits=%0d", s2, $bits(s2));
  end
endmodule
"#,
            "U1 06 bits=8",
        ),
        (
            r#"
typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
package p;
  typedef struct packed { logic [3:0] a; logic [7:0] b; } st;
endpackage
module t;
  logic c;
  typedef struct packed { logic [5:0] a; logic [1:0] b; } st;
  import p::*;
  st s2;
  initial begin
    c = 1; s2 = c ? '{a: 6'h1, b: 2'h2} : '{a: 6'h3, b: 2'h1};
    $display("G4 %h bits=%0d", s2, $bits(s2));
  end
endmodule
"#,
            "G4 06 bits=8",
        ),
    ] {
        assert_eq!(runs(src), [want]);
    }
    // An arm driving an `inout` port, which vita connects from the parent only (W3056):
    // verilator and sv2v → iverilog `inout 56`.
    is_loud(
        r#"
typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
module child(inout s_t io, input logic en);
  assign io = en ? '{a: 4'h5, b: 4'h6} : 'z;
endmodule
module t;
  logic en;
  wire [7:0] pio;
  child u(.io(pio), .en(en));
  initial begin en = 1; #1 $display("inout %h", pio); end
endmodule
"#,
        E3009,
    );
}

#[test]
fn the_census_arms_of_4_5_574_are_typed() {
    // §4.5.574's census, each with the lines verilator 5.050 and sv2v 0.0.13 → iverilog 13.0
    // print: `always @*` following a value read only in the pattern, an intra-assignment
    // delay (`xx`: sv2v → iverilog; verilator is 2-state, `00`), `always_ff` with a reset,
    // `case` items, an ANSI port continuation, `int` and signed members, a package-scoped,
    // explicitly imported and chained-alias type, member names read as values, `fork`, a
    // variable `default:`, an `x` condition with a real arm (sv2v → iverilog; verilator
    // reads the `x` as 0, `00100001`), two instances, generate-`for` processes, a
    // declaration initializer, nested parentheses, a module typedef over a unit one of the
    // name, and a `program` (verilator; sv2v cannot parse it).
    let cases: [(&str, &[&str]); 21] = [
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c; logic [4:0] v;
  always @* e = c ? 8'h00 : '{a: 3'd5, b: v};
  initial begin c = 0; v = 5'd1; #1 $display("A %h", e); v = 5'd7; #1 $display("B %h", e); c = 1; #1 $display("C %h", e); end
endmodule
"#,
            &["A a1", "B a7", "C 00"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e, f; logic c;
  initial begin c = 0; e = #1 c ? 8'hff : '{a: 3'd1, b: 5'd2}; f <= #2 c ? 8'hff : '{b: 5'd3, a: 3'd4}; $display("A %h %h %0t", e, f, $time); #3 $display("B %h %h %0t", e, f, $time); end
endmodule
"#,
            &["A 22 xx 1", "B 22 83 4"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t q; logic clk = 0, rst = 1; logic [4:0] n = 0;
  always_ff @(posedge clk or posedge rst) if (rst) q <= '{default: '0}; else q <= (n[0] ? '{a: 3'd7, b: n} : '{a: 3'd1, b: ~n});
  initial begin #1 rst = 0; repeat (4) begin #1 clk = 1; #1 clk = 0; n = n + 1; $display("Q %h", q); end $finish; end
endmodule
"#,
            &["Q 3f", "Q e1", "Q 3d", "Q e3"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic [1:0] sel; logic c;
  initial begin
    c = 1;
    for (int i = 0; i < 3; i++) begin
      sel = i;
      case (sel)
        2'd0: e = c ? '{a: 3'd1, b: 5'd1} : 8'h0;
        2'd1: e = !c ? 8'h0 : '{b: 5'd9, a: 3'd6};
        default: e = c ? '{default: '1} : 8'h0;
      endcase
      $display("S%0d %h", i, e);
    end
  end
endmodule
"#,
            &["S0 21", "S1 c9", "S2 ff"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module child(input logic c, output s_t x, y);
  assign x = c ? '{a: 3'd1, b: 5'd2} : 8'h00;
  always_comb y = !c ? 8'h11 : '{a: 3'd3, b: 5'd4};
endmodule
module t;
  logic c; s_t x, y;
  child u(.c(c), .x(x), .y(y));
  initial begin c = 1; #1 $display("X %h %h", x, y); c = 0; #1 $display("Y %h %h", x, y); end
endmodule
"#,
            &["X 22 64", "Y 00 11"],
        ),
        (
            r#"
typedef struct packed { int a; int b; } s_t;
module t;
  s_t e; logic c = 0; int n = -3;
  initial begin e = c ? 64'd0 : '{a: n, b: -1}; $display("I %h %0d %0d", e, e.a, e.b); e = c ? 64'd0 : '{default: 'x}; $display("J %h", e); end
endmodule
"#,
            &["I fffffffdffffffff -3 -1", "J 0000000000000000"],
        ),
        (
            r#"
typedef struct packed { logic signed [3:0] a; logic [3:0] b; } s_t;
module t;
  s_t e; logic c = 0; logic signed [1:0] m = -2'sd1;
  initial begin e = c ? 8'h0 : '{a: m, b: -1}; $display("S %h %0d", e, e.a); e = c ? 8'h0 : '{a: 2'b11, b: 5'h1f}; $display("T %h", e); end
endmodule
"#,
            &["S ff -1", "T 3f"],
        ),
        (
            r#"
package p;
  typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
endpackage
module t;
  p::s_t e; logic c = 0;
  initial begin e = c ? 8'h0 : '{a: 3'd6, b: 5'd3}; $display("P %h", e); end
endmodule
"#,
            &["P c3"],
        ),
        (
            r#"
package p;
  typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
endpackage
module t;
  import p::s_t;
  s_t e; logic c = 1;
  initial begin e = c ? '{b: 5'd1, a: 3'd2} : 8'h0; $display("P %h", e); end
endmodule
"#,
            &["P 41"],
        ),
        (
            r#"
package p;
  typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
endpackage
package q;
  import p::*;
  typedef s_t q_t;
endpackage
module t;
  q::q_t e; logic c = 1;
  initial begin e = c ? '{b: 5'd1, a: 3'd2} : 8'h0; $display("Q %h", e); end
endmodule
"#,
            &["Q 41"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c = 0; logic [2:0] a = 3'd4; logic [4:0] b = 5'd17;
  initial begin e = c ? 8'h0 : '{a: a, b: b}; $display("N %h", e); end
endmodule
"#,
            &["N 91"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e, f; logic c = 0;
  initial begin fork e = c ? 8'h0 : '{a: 3'd1, b: 5'd3}; #1 f = !c ? '{a: 3'd2, b: 5'd4} : 8'h0; join $display("F %h %h", e, f); end
endmodule
"#,
            &["F 23 44"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c = 0; logic [4:0] d = 5'd9;
  initial begin e = c ? 8'h0 : '{a: 3'd7, default: d}; $display("V %h", e); e = c ? 8'h0 : '{default: d}; $display("W %h", e); end
endmodule
"#,
            &["V e9", "W 29"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c = 1'bx;
  initial begin e = c ? 1.5 : '{a: 3'd1, b: 5'd1}; $display("R %b", e); end
endmodule
"#,
            &["R 00000000"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module m #(parameter int K = 1) (output s_t o);
  logic c = (K == 2);
  assign o = c ? '{a: 3'(K), b: 5'(K)} : '{a: 3'd0, b: 5'(K+10)};
endmodule
module t;
  s_t o1, o2;
  m #(1) u1(.o(o1)); m #(2) u2(.o(o2));
  initial #1 $display("O %h %h", o1, o2);
endmodule
"#,
            &["O 0b 42"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e [0:1]; s_t e0, e1; logic c = 0;
  for (genvar i = 0; i < 2; i++) begin : g
    if (i == 0) begin : z always @(c) e0 = c ? 8'h0 : '{a: 3'(i), b: 5'(i + 3)}; end
    else begin : o always @(c) e1 = c ? 8'h0 : '{a: 3'(i), b: 5'(i + 3)}; end
  end
  initial begin #1 c = 1; #1 c = 0; #1 $display("G %h %h", e0, e1); end
endmodule
"#,
            &["G 03 24"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  logic c = 0; logic [4:0] v = 5'd6;
  s_t d = c ? 8'h0 : '{a: 3'd2, b: v};
  initial #1 $display("D %h", d);
endmodule
"#,
            &["D 46"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c = 0, d = 1;
  initial begin e = ((c) ? (8'h1) : ((d ? ('{a: 3'd3, b: 5'd3}) : ('{3'd4, 5'd4})))); $display("P %h", e); end
endmodule
"#,
            &["P 63"],
        ),
        (
            r#"
typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
module t;
  typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
  s_t e; logic c = 0;
  initial begin e = c ? 8'h0 : '{a: 3'd1, b: 5'd2}; $display("U %h", e); end
endmodule
"#,
            &["U 22"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
program t;
  s_t e; logic c = 0;
  initial begin e = c ? 8'h0 : '{a: 3'd1, b: 5'd2}; $display("PR %h", e); end
endprogram
"#,
            &["PR 22"],
        ),
        (
            r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
typedef struct packed { logic [4:0] a; logic [2:0] b; } r_t;
module t;
  r_t e; logic c = 0;
  initial begin e = c ? 8'h0 : '{a: 5'd1, b: 3'd2}; $display("R %h", e); end
endmodule
"#,
            &["R 0a"],
        ),
    ];
    for (src, want) in cases {
        assert_eq!(runs(src), want, "{src}");
    }
}

#[test]
fn the_census_refusals_of_4_5_574_stay_refused() {
    // Each keeps the refusal although both oracles run most of them: a generate block's
    // own variable of another struct type named like the module's, a block-local of the
    // name, a 2-state member of a 4-state struct, a duplicated, unknown or missing key, a
    // wrong element count, a task body, a call in `default:`, a function-local typedef of
    // the type's name (the declaration's record is dropped), an interface member and a
    // select of the target.
    for src in [
        r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
typedef struct packed { logic [4:0] a; logic [2:0] b; } r_t;
module t;
  s_t e; logic c = 0;
  if (1) begin : g
    r_t e;
    initial begin #1 e = c ? 8'h00 : '{a: 5'd3, b: 3'd1}; $display("G %h", e); end
  end
  initial begin #2 e = c ? 8'h00 : '{a: 3'd3, b: 5'd1}; $display("M %h", e); end
endmodule
"#,
        r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c = 0;
  initial begin : b
    s_t e;
    e = c ? 8'h00 : '{a: 3'd2, b: 5'd2};
    $display("L %h", e);
  end
  initial #1 $display("M %h", e);
endmodule
"#,
        r#"
typedef struct packed { int a; logic [3:0] b; } s_t;
module t;
  s_t e; logic c = 0;
  initial begin e = c ? 36'd0 : '{a: 5, b: 4'd3}; $display("I %h", e); end
endmodule
"#,
        r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c = 0;
  initial begin e = c ? 8'h0 : '{a: 3'd1, a: 3'd2, b: 5'd0}; $display("D %h", e); end
endmodule
"#,
        r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c = 0;
  initial begin e = c ? 8'h0 : '{a: 3'd1, z: 5'd0}; $display("D %h", e); end
endmodule
"#,
        r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c = 0;
  initial begin e = c ? 8'h0 : '{3'd1, 5'd2, 1'b1}; $display("D %h", e); end
endmodule
"#,
        r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c = 0;
  initial begin e = c ? 8'h0 : '{a: 3'd1}; $display("D %h", e); end
endmodule
"#,
        r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c = 0;
  task automatic wr(); e = c ? 8'h0 : '{a: 3'd1, b: 5'd1}; endtask
  initial begin wr(); $display("T %h", e); end
endmodule
"#,
        r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c = 0;
  function automatic logic [4:0] f(); return 5'd3; endfunction
  initial begin e = c ? 8'h0 : '{a: 3'd7, default: f()}; $display("V %h", e); end
endmodule
"#,
        r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c = 0;
  function automatic int f(); typedef logic [7:0] s_t; s_t z; z = 1; return z; endfunction
  initial begin e = c ? 8'h0 : '{a: 3'd1, b: 5'd1}; $display("R %h %0d", e, f()); end
endmodule
"#,
        r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
interface ifc; s_t v; logic c; endinterface
module t;
  ifc i();
  initial begin i.c = 0; i.v = i.c ? 8'h0 : '{a: 3'd1, b: 5'd2}; $display("I %h", i.v); end
endmodule
"#,
        r#"
typedef struct packed { logic [2:0] a; logic [4:0] b; } s_t;
module t;
  s_t e; logic c = 0;
  initial begin e = '0; e[7:0] = c ? 8'h0 : '{a: 3'd1, b: 5'd2}; $display("S %h", e); end
endmodule
"#,
    ] {
        is_loud(src, E3009);
    }
}
