//! §3.b cont-array-typedef-elem: a whole-array continuous `assign` copying arrays whose
//! element type comes from a typedef (IEEE 1800-2017 §10.3, §7.6, §6.22.2). §4.5.566
//! lowered only element types written in the array's own declaration, so the corpus
//! row `ibex` stopped twice on its lint sink `assign unused_csr_pmp_cfg = csr_pmp_cfg;`
//! over `pmp_cfg_t`, a packed struct with an enum member.
//!
//! The parser now marks a declaration whose type is a typedef of an integral packed
//! type other than an enum — a packed struct or union, or a vector or packed-array
//! alias — with every part in the state its recorded kind says and name-free bounds
//! (`NetVarDecl::integral_typedef`). Such an element is equivalent (§6.22.2) to every
//! integral packed type of its width, state and signedness, so a COPY may read or
//! write such arrays; a pattern into one stays refused.
//!
//! Oracles: verilator 5.052 (`--binary --timing`, 2-state), iverilog 13.0 (`-g2012`)
//! and sv2v 0.0.13 → iverilog 13.0. iverilog aborts on several of these designs (a
//! struct member of an array element), where sv2v → iverilog is the 4-state oracle;
//! sv2v drops a signed typedef's sign and reads a never-written variable as an
//! undriven wire, so on those cells verilator or iverilog alone decides, and each test
//! says which ran.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_catd_{}_{n}", std::process::id()));
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

/// The design's own lines that start with `tag`.
fn prints(src: &str, tag: &str, want: &[&str]) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with(tag)).collect();
    assert_eq!(got, want, "{out}");
}

fn is_loud(src: &str, needle: &str) -> String {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(1), "expected a refusal\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in\n{out}");
    out
}

const WRITE: &str = "a whole unpacked array cannot be the write target in this context";

#[test]
fn the_ibex_pmp_cfg_sink_copies() {
    // ibex_core.sv `g_no_pmp`, reduced: `pmp_cfg_t` holds a `logic [1:0]` enum member.
    // sv2v → iverilog prints these lines (iverilog alone aborts on `u[3].mode`);
    // verilator prints `00 …`, `2a 15 00 110100`, `1 10 01`, `08 01` — the same
    // wherever a bit is known.
    let src = r#"
package pkg;
  typedef enum logic [1:0] { OFF=2'b00, TOR=2'b01, NA4=2'b10, NAPOT=2'b11 } pmp_cfg_mode_e;
  typedef struct packed {
    logic          lock;
    pmp_cfg_mode_e mode;
    logic          exec;
    logic          write;
    logic          read;
  } pmp_cfg_t;
endpackage
module t;
  import pkg::*;
  pmp_cfg_t csr_pmp_cfg [4];
  if (1) begin : g_no_pmp
    pmp_cfg_t unused_csr_pmp_cfg [4];
    assign unused_csr_pmp_cfg = csr_pmp_cfg;
    initial begin
      #1 $display("A0 %h %h %h %h", unused_csr_pmp_cfg[0], unused_csr_pmp_cfg[1], unused_csr_pmp_cfg[2], unused_csr_pmp_cfg[3]);
      #2 $display("A1 %h %h %h %b", unused_csr_pmp_cfg[0], unused_csr_pmp_cfg[1], unused_csr_pmp_cfg[2], unused_csr_pmp_cfg[3]);
      $display("A2 %b %b %b", unused_csr_pmp_cfg[3].lock, unused_csr_pmp_cfg[3].mode, unused_csr_pmp_cfg[0].mode);
      #2 $display("A3 %h %b", unused_csr_pmp_cfg[2], unused_csr_pmp_cfg[2].mode);
    end
  end
  initial begin
    #2 csr_pmp_cfg[0] = 6'h2a; csr_pmp_cfg[1] = 6'h15; csr_pmp_cfg[3] = 6'b11010x;
    #2 csr_pmp_cfg[2].mode = TOR;
  end
endmodule
"#;
    prints(
        src,
        "A",
        &[
            "A0 xx xx xx xx",
            "A1 2a 15 xx 11010x",
            "A2 1 10 01",
            "A3 XX 01",
        ],
    );
}

#[test]
fn struct_and_union_elements_copy() {
    // A packed union, and a package struct nesting another. iverilog and sv2v →
    // iverilog print the union lines (verilator `D0 0 0`, `D1 9 1001`); sv2v → iverilog
    // prints `E3` (iverilog alone aborts on `d[1].lo.a`; verilator `100110 100 10`).
    let union = r#"
typedef union packed { logic [3:0] w; logic [3:0] v; } u4_t;
module t;
  u4_t s4 [2]; u4_t d4 [2];
  assign d4 = s4;
  initial begin
    #1 $display("D0 %h %h", d4[0], d4[1]);
    s4[0] = 4'h9; s4[1] = 4'b10x1;
    #1 $display("D1 %h %b", d4[0], d4[1]);
  end
endmodule
"#;
    prints(union, "D", &["D0 x x", "D1 9 10x1"]);
    let nested = r#"
package p;
  typedef struct packed { logic [1:0] a; logic b; } in_t;
  typedef struct packed { in_t lo; logic [2:0] hi; } out_t;
endpackage
module t; p::out_t s [2]; p::out_t d [2]; assign d = s;
  initial begin s[1] = 6'b10x_110; #1 $display("E3 %b %b %b", d[1], d[1].lo, d[1].lo.a); end
endmodule
"#;
    prints(nested, "E3", &["E3 10x110 10x 10"]);
}

#[test]
fn a_struct_copies_to_and_from_an_equivalent_type() {
    // §6.22.2: packed structs and vectors of one width, state and signedness are
    // equivalent. sv2v → iverilog prints these lines (iverilog alone aborts on
    // `a[0].x`); verilator runs every copy and prints `9 1001 6 0011 6 0011`, `100 110`.
    let src = r#"
typedef struct packed { logic [2:0] x; logic y; } sa_t;
typedef struct packed { logic p; logic [2:0] q; } sb_t;
module t;
  logic [3:0] v [2];
  sa_t a [2]; sb_t b [2]; logic [3:0] w [2]; sa_t a2 [2];
  assign a = v;
  assign w = a2;
  assign b = a2;
  initial begin
    v[0] = 4'h9; v[1] = 4'b1x01; a2[0] = 4'h6; a2[1] = 4'b0x11;
    #1 $display("G1 %h %b %h %b %h %b", a[0], a[1], w[0], w[1], b[0], b[1]);
    $display("G2 %b %b", a[0].x, b[0].q);
  end
endmodule
"#;
    prints(src, "G", &["G1 9 1x01 6 0x11 6 0x11", "G2 100 110"]);
}

#[test]
fn alias_typedef_elements_copy() {
    // A vector, a signed vector, a packed 2-D and a chained struct alias. verilator
    // prints `F1 5a -3 c5 c 1001` and `F2 1 -2 100`; sv2v → iverilog prints the `x`
    // lines but reads the signed alias unsigned (`253`, `0 254`), so the sign is
    // verilator's.
    let src = r#"
typedef logic [7:0] byte_t;
typedef logic signed [7:0] sbyte_t;
typedef logic [1:0][3:0] pa_t;
typedef struct packed { logic [2:0] x; logic y; } st_t;
typedef st_t st2_t;
module t;
  byte_t s1 [2]; byte_t d1 [2];
  sbyte_t s3 [2]; sbyte_t d3 [2];
  pa_t s4 [2]; pa_t d4 [2];
  st_t s5 [2]; st2_t d5 [2];
  assign d1 = s1; assign d3 = s3; assign d4 = s4; assign d5 = s5;
  initial begin
    #1 $display("F0 %h %h %h %h", d1[0], d3[0], d4[0], d5[0]);
    s1[0] = 8'h5a; s3[1] = -8'sd3; s4[1] = 8'hc5; s5[0] = 4'b1x01;
    #1 $display("F1 %h %0d %h %h %b", d1[0], d3[1], d4[1], d4[1][1], d5[0]);
    $display("F2 %0d %0d %b", d3[1] < 0, d3[1] + 1, d5[0].x);
  end
endmodule
"#;
    prints(
        src,
        "F",
        &["F0 xx xx xx x", "F1 5a -3 c5 c 1x01", "F2 1 -2 1x0"],
    );
}

#[test]
fn ports_interfaces_and_generate_blocks_carry_typedef_arrays() {
    // ANSI ports of a package typedef, one inheriting the previous port's type; an
    // interface's members; a generate block whose own typedef hides the module's.
    // sv2v → iverilog prints `Q1` (iverilog alone refuses `src[2].b = …`; verilator
    // `9 1001 0100 | 9 1001 0100`); iverilog and sv2v → iverilog print `E7` and `Q2`
    // (verilator `0000 1010`, `b 0010`).
    let ports = r#"
package pkg;
  typedef struct packed { logic a; logic [1:0] b; logic c; } s_t;
endpackage
module child (input pkg::s_t ci [3], output pkg::s_t co [3], x [3]);
  assign co = ci;
  assign x = co;
endmodule
module t;
  import pkg::*;
  s_t src [3]; s_t d1 [3]; s_t d2 [3]; s_t d3 [3];
  child u (.ci(src), .co(d1), .x(d2));
  assign d3 = d2;
  initial begin
    #1 $display("Q0 %h %h %h", d1[0], d2[1], d3[2]);
    src[0] = 4'h9; src[1] = 4'b1x01; src[2].b = 2'b10;
    #1 $display("Q1 %h %b %b | %h %b %b", d1[0], d2[1], d3[2], d3[0], d1[1], d2[2]);
  end
endmodule
"#;
    prints(ports, "Q", &["Q0 x x x", "Q1 9 1x01 x10x | 9 1x01 x10x"]);
    let iface = r#"
typedef struct packed { logic [2:0] a; logic b; } s_t;
interface ifc; s_t x [2]; s_t y [2]; assign y = x; endinterface
module t; ifc i ();
  initial begin i.x[1] = 4'b1x10; #1 $display("E7 %b %b", i.y[0], i.y[1]); end
endmodule
"#;
    prints(iface, "E7", &["E7 xxxx 1x10"]);
    let generate = r#"
module t;
  typedef struct packed { logic [2:0] a; logic b; } s_t;
  s_t src [2];
  if (1) begin : g
    typedef struct packed { logic [3:0] w; } s_t;
    s_t dst [2];
    assign dst = src;
  end
  initial begin
    src[0] = 4'hb; src[1] = 4'b0x10;
    #1 $display("Q2 %h %b", g.dst[0], g.dst[1]);
  end
endmodule
"#;
    prints(generate, "Q2", &["Q2 b 0x10"]);
}

#[test]
fn multi_dimensional_arrays_of_a_typedef_copy() {
    // Two unpacked dimensions, and packed dimensions after the type name. iverilog and
    // sv2v → iverilog print this line (verilator `000 010 101 000 | 30 2c 110`).
    let src = r#"
typedef struct packed { logic [1:0] a; logic b; } s_t;
module t;
  s_t src [2][2]; s_t dst [2][2];
  s_t [1:0] ps [2]; s_t [1:0] pd [2];
  assign dst = src;
  assign pd = ps;
  initial begin
    src[1][0] = 3'b101; src[0][1] = 3'b01x; ps[1] = 6'h2c; ps[0][1] = 3'b110;
    #1 $display("Q3 %b %b %b %b | %h %h %b", dst[0][0], dst[0][1], dst[1][0], dst[1][1], pd[0], pd[1], pd[0][1]);
  end
endmodule
"#;
    prints(src, "Q3", &["Q3 xxx 01x 101 xxx | 3X 2c 110"]);
}

#[test]
fn typedef_elements_this_ir_cannot_vouch_for_stay_refused() {
    // Each keeps PRE's refusal. An enum, or an alias of one: this IR keeps no enum
    // identity, and iverilog refuses an enum array copied to or from another type
    // (`Element types are not compatible`) where verilator runs it. A struct with an
    // `enum bit` member: the member is recorded 4-state, so the struct is too, where
    // both oracles read the never-written copy `0` (verilator and iverilog `C0 0 0`).
    // A struct mixing an `int` and a `logic` member (a 2-state member of a 4-state
    // struct reads through a conversion; both oracles run it). A member width that
    // follows an overridable parameter (both oracles `3f 0f`). A NAME in a typedef's
    // bound: a `$unit` `typedef logic [W-1:0] w_t;` under a module's `localparam W = 8`
    // is 8 bits here where both oracles read 4. An unpacked-array typedef.
    for (decl, body) in [
        (
            "typedef enum logic [1:0] { L0, L1, L2 } e_t;",
            "e_t s [2]; e_t d [2]; assign d = s;",
        ),
        (
            "typedef enum logic [1:0] { L0, L1, L2 } e_t; typedef e_t e2_t;",
            "e2_t s [2]; e2_t d [2]; assign d = s;",
        ),
        (
            "typedef enum logic [1:0] { L0, L1, L2 } e_t;",
            "logic [1:0] v [2]; e_t d [2]; assign d = v;",
        ),
        (
            "typedef enum logic [1:0] { L0, L1, L2 } e_t;",
            "e_t s [2]; logic [1:0] w [2]; assign w = s;",
        ),
        (
            "typedef enum bit [1:0] { E0, E1 } eb_t; typedef struct packed { bit a; eb_t m; bit c; } s_t;",
            "s_t s [2]; s_t d [2]; assign d = s;",
        ),
        (
            "typedef struct packed { int i; logic [3:0] l; } s_t;",
            "s_t s [2]; s_t d [2]; assign d = s;",
        ),
        (
            "localparam W = 4; typedef logic [W-1:0] w_t;",
            "localparam W = 8; w_t s [2]; w_t d [2]; assign d = s;",
        ),
        (
            "typedef logic [3:0] arr_t [2];",
            "arr_t s; arr_t d; assign d = s;",
        ),
    ] {
        let src = format!(
            "{decl}\nmodule t;\n  {body}\n  initial begin #1 $display(\"T %h\", d[0]); $finish; end\nendmodule\n"
        );
        is_loud(&src, WRITE);
    }
    let param = r#"
module m #(parameter W = 2) (output logic [7:0] o);
  localparam W2 = W * 2;
  typedef struct packed { logic [W2-1:0] a; } s_t;
  s_t s [2]; s_t d [2];
  assign d = s;
  initial begin s[0] = '1; #1 o = d[0]; end
endmodule
module t; logic [7:0] o1, o2; m #(.W(3)) u1 (o1); m u2 (o2);
  initial #2 $display("E8 %h %h", o1, o2);
endmodule
"#;
    is_loud(param, WRITE);
}

#[test]
fn a_bound_not_written_as_a_literal_stays_refused() {
    // A NAME in a bound is read where the parser folds it, which need not be where the
    // type was declared. A struct member typed by a typedef whose bound is a name is laid
    // out with the name the parser sees where the STRUCT is declared: each of the first
    // five designs is 9 bits here (`9 1ff 1a5`) where verilator and iverilog read 5
    // (`5 1f 05`), and the same wrong width shows when the elements are copied one by one
    // — a `$unit` alias or enum under a module's `localparam W = 8`, a module alias under
    // a generate block's, a `$unit` alias inside a package struct, a `$clog2(N)` bound. So
    // the whole-array copy keeps its refusal unless every bound was WRITTEN as an integer
    // literal (`layout_exact`).
    let designs = [
        "localparam W = 4;\ntypedef logic [W-1:0] v_t;\nmodule t;\n  localparam W = 8;\n  typedef struct packed { v_t a; logic b; } s_t;\n  s_t x [2]; s_t y [2];\n  assign y = x;\n  initial #1 $display(\"W1 %h\", y[1]);\nendmodule\n",
        "localparam W = 4;\ntypedef enum logic [W-1:0] {A = 1, B = 2} e_t;\nmodule t;\n  localparam W = 8;\n  typedef struct packed { e_t f; logic g; } s_t;\n  s_t x [2]; s_t y [2];\n  assign y = x;\n  initial #1 $display(\"W4 %h\", y[1]);\nendmodule\n",
        "module t;\n  localparam W = 4;\n  typedef logic [W-1:0] v_t;\n  if (1) begin : g\n    localparam W = 8;\n    typedef struct packed { v_t a; logic b; } s_t;\n    s_t x [2]; s_t y [2];\n    assign y = x;\n    initial #1 $display(\"W8 %h\", y[1]);\n  end\nendmodule\n",
        "localparam W = 4;\ntypedef logic [W-1:0] v_t;\npackage p;\n  localparam W = 8;\n  typedef struct packed { v_t a; logic b; } s_t;\nendpackage\nmodule t;\n  import p::*;\n  s_t x [2]; s_t y [2];\n  assign y = x;\n  initial #1 $display(\"W11 %h\", y[1]);\nendmodule\n",
        "localparam N = 16;\ntypedef logic [$clog2(N)-1:0] c_t;\nmodule t;\n  localparam N = 256;\n  typedef struct packed { c_t a; logic b; } s_t;\n  s_t x [2]; s_t y [2];\n  assign y = x;\n  initial #1 $display(\"W12 %h\", y[1]);\nendmodule\n",
    ];
    for src in designs {
        is_loud(src, WRITE);
    }
    // `$bits(v_t)` is folded to a literal where it is read, re-reading `v_t`'s `[W-1:0]`
    // with the reader's `W` (here 8, both oracles 4): as a keyword member's bound, a
    // keyword alias's, and an enum member's base (`B1 8 9 1ff 001` / `B6` / `B7` here,
    // verilator and iverilog `4 5 1f 01`). A keyword member naming a localparam declared
    // after the struct in the same block: iverilog `K1 9 1a5 0f3` with vita, sv2v `5 17
    // 13`, verilator refuses the copy — the bound is a name, so it waits. Dims written
    // after the type name are the declaration's own and follow the same rule.
    let folded = [
        "localparam W = 4;\ntypedef logic [W-1:0] v_t;\nmodule t;\n  localparam W = 8;\n  typedef struct packed { logic [$bits(v_t)-1:0] a; logic b; } s_t;\n  s_t d [2]; s_t s [2];\n  assign d = s;\n  initial #1 $display(\"B1 %h\", d[1]);\nendmodule\n",
        "module t;\n  localparam W = 4;\n  typedef logic [W-1:0] v_t;\n  if (1) begin : g\n    localparam W = 8;\n    typedef logic [$bits(v_t):0] w_t;\n    w_t d [2]; w_t s [2];\n    assign d = s;\n    initial #1 $display(\"B6 %h\", d[1]);\n  end\nendmodule\n",
        "module t;\n  localparam W = 4;\n  typedef logic [W-1:0] v_t;\n  if (1) begin : g\n    localparam W = 8;\n    typedef enum logic [$bits(v_t)-1:0] {A = 1, B = 2} e_t;\n    typedef struct packed { e_t e; logic b; } s_t;\n    s_t d [2]; s_t s [2];\n    assign d = s;\n    initial #1 $display(\"B7 %h\", d[1]);\n  end\nendmodule\n",
        "module t;\n  localparam W = 8;\n  if (1) begin : g\n    typedef struct packed { logic [W-1:0] a; logic b; } s_t;\n    localparam W = 4;\n    s_t d [2];\n    logic [8:0] s [2];\n    assign d = s;\n    initial #1 $display(\"K1 %h\", d[1]);\n  end\nendmodule\n",
        "typedef struct packed { logic [1:0] a; } s_t;\nmodule t;\n  localparam N = 2;\n  s_t [N-1:0] d [2]; s_t [N-1:0] s [2];\n  assign d = s;\n  initial #1 $display(\"D6 %h\", d[1]);\nendmodule\n",
    ];
    for src in folded {
        is_loud(src, WRITE);
    }
}

#[test]
fn a_pattern_into_a_typedef_element_stays_refused() {
    // A copy only: a `'{default: v}` or positional item into a struct element would
    // need the struct's own pattern rules (verilator and sv2v → iverilog read `'{default:
    // 1}` into `struct packed { logic a; logic [1:0] b; }` elements as `001`, and a
    // 2-state member inside a 4-state struct has no 4-state oracle).
    for rhs in ["'{default: 1}", "'{3'd1, 3'd2}", "'{default: 'x}"] {
        let src = format!(
            "typedef struct packed {{ logic a; logic [1:0] b; }} s_t;\nmodule t;\n  s_t x [2];\n  assign x = {rhs};\n  initial #1 $display(\"P %b %b\", x[0], x[1]);\nendmodule\n"
        );
        is_loud(&src, WRITE);
    }
    let alias = "typedef logic [7:0] b_t;\nmodule t;\n  b_t d [2];\n  assign d = '{default: 8'h5};\n  initial #1 $display(\"P %h\", d[0]);\nendmodule\n";
    is_loud(alias, WRITE);
}

#[test]
fn a_copy_that_is_not_equivalent_stays_refused() {
    // §6.22.2: a signed struct is not equivalent to an unsigned vector, nor a 4-bit
    // struct to a 5-bit vector (verilator: `Array element types are not equivalent`;
    // iverilog: `Element types are not compatible`), nor a 4-state struct to a `bit`
    // vector (both oracles refuse). A 2-state struct target is a variable this IR does
    // not drive continuously (E3018, as for `bit [3:0] d [2]`; both oracles print `9`).
    let signed = "typedef struct packed signed { logic [3:0] a; } s_t;\nmodule t; logic [3:0] v [2]; s_t d [2]; assign d = v;\n  initial begin v[0] = 4'hf; #1 $display(\"E4 %0d\", d[0]); end\nendmodule\n";
    is_loud(
        signed,
        "unpacked-array assignment requires identical element types",
    );
    let width = "typedef struct packed { logic [3:0] a; } s_t;\nmodule t; logic [4:0] v [2]; s_t d [2]; assign d = v;\n  initial begin v[0] = 5'h1f; #1 $display(\"E5 %h\", d[0]); end\nendmodule\n";
    is_loud(
        width,
        "unpacked-array assignment requires identical element types",
    );
    let two_state_src = "typedef struct packed { logic [3:0] a; } s_t;\nmodule t; bit [3:0] v [2]; s_t d [2]; assign d = v;\n  initial begin v[0] = 4'h9; #1 $display(\"E6 %h\", d[0]); end\nendmodule\n";
    is_loud(two_state_src, WRITE);
    let two_state_dst = "module t;\n  typedef struct packed { bit [2:0] a; bit b; } s_t;\n  s_t s [2]; s_t d [2];\n  assign d = s;\n  initial begin s[0] = 4'h9; #1 $display(\"N2 %h\", d[0]); end\nendmodule\n";
    is_loud(two_state_dst, "continuous assign drives variable `t.d`");
}

#[test]
fn another_writer_keeps_a_typedef_copy_loud() {
    // The sole-writer rule of `cont_assign_whole_array.rs`, on a typedef element.
    let src = r#"
typedef struct packed { logic [2:0] a; logic b; } s_t;
module t;
  s_t s [2]; s_t d [2];
  assign d = s;
  initial begin #1 d[0] = 4'h3; #1 $display("W %h", d[0]); end
endmodule
"#;
    is_loud(
        src,
        "a whole unpacked array driven by a continuous `assign` has another writer",
    );
}
