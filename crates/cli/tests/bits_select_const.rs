//! `$bits` of a select — a bit-select, a part-select, an indexed part-select, a
//! packed-struct member (which the parser lays out as a part-select), or a
//! concatenation of them — in a constant position: a `localparam` value, a range, a
//! parameter override, a generate `if` / `for` / `case`, an unpacked dimension.
//! IEEE 1800-2017 §20.6.2 makes `$bits` a constant function of its argument's type;
//! a select's width is the select's (§11.5), whatever the object holds.
//!
//! Oracles: Icarus Verilog 13.0 (`-g2012`), sv2v 0.0.13 → iverilog and Verilator
//! 5.052 (`--binary`) agree on every pinned value below (the row-9 census cells R31
//! and V4 among them; Icarus Verilog cannot parse the interface-port case). vita's
//! own runtime `$bits` (a replication count inside a process) already answered the
//! same values; only the constant domain refused them. A select of a
//! multi-dimensional packed array (`logic [3:0][7:0] p; $bits(p[1])`, an 8-bit
//! element) is not this rule and stays refused in a constant position.
//!
//! Witnesses: VeeR EH1's `#(2*$bits(sig[5:4]))` override and OpenTitan otbn's
//! `prim_edn_req` (`$bits({edn_i.edn_fips, edn_i.edn_bus})` over a struct port).
//!
//! The rule answers only in the module's own top level — a declaration, a range, an
//! override, a generate construct's condition or loop header — and only over a name
//! no nested scope of the module (generate block, routine, begin-block) declares.
//! Everywhere else (inside a generate block, a routine, an interface, a package
//! function) it declines and the position answers as before: a runtime position
//! by its lowering (right), a constant one loudly.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bsc_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["-Wno-W1017", "t.sv"])
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// Every `R ` line of a run that must succeed.
fn lines(src: &str) -> Vec<String> {
    let (out, err, code) = run(src);
    assert_eq!(code, Some(0), "{src}\nstdout:\n{out}\nstderr:\n{err}");
    out.lines()
        .filter(|l| l.starts_with("R "))
        .map(str::to_string)
        .collect()
}

const DECLS: &str = "\
  typedef struct packed { logic f; logic [15:0] bus; } e_t;
  typedef struct packed { logic [2:0] x; e_t in; } o_t;
  e_t e; o_t o;
  logic [31:0] addr = 32'h30;
  logic [7:0] v; logic [6:0] w;
  logic [3:0][7:0] p2;
  logic [7:0] m [0:3];
  logic [0:15] up;
  logic [11:4] nz;
  int i = 1;
";

/// (select, width) — all three oracles.
const SHAPES: &[(&str, u32)] = &[
    ("addr[3]", 1),
    ("addr[5:4]", 2),
    ("addr[4 +: 3]", 3),
    ("addr[7 -: 5]", 5),
    ("addr[i]", 1),
    ("addr[i +: 6]", 6),
    ("up[2:9]", 8),
    ("nz[9:6]", 4),
    ("m[2][5]", 1),
    ("m[2][5:2]", 4),
    ("e.f", 1),
    ("e.bus", 16),
    ("e.bus[11:4]", 8),
    ("o.in.bus", 16),
    ("{e.f, e.bus}", 17),
    ("{addr[5:4], v[0], w}", 10),
    ("{3{addr[5:4]}}", 6),
];

#[test]
fn row9_census_r31_concatenations_of_member_selects() {
    let got = lines(
        "module t;\n\
         typedef struct packed { logic f; logic [15:0] bus; } e_t;\n\
         e_t e; logic [3:0] v; logic [6:0] w;\n\
         localparam int A = $bits({v, w});\n\
         localparam int B = $bits({e.f, e.bus});\n\
         initial begin #1 $display(\"R A=%0d B=%0d\", A, B); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(got, ["R A=11 B=17"]);
}

#[test]
fn row9_census_v4_override() {
    // VeeR EH1's spelling; was VITA-E3009 with W3056 (the child default kept).
    let got = lines(
        "module dff #(parameter WIDTH = 1) (input logic clk, input logic [WIDTH-1:0] din, output logic [WIDTH-1:0] dout);\n\
         always_ff @(posedge clk) dout <= din;\n\
         initial $display(\"R WIDTH=%0d\", WIDTH);\n\
         endmodule\n\
         module top;\n\
         logic clk = 0;\n\
         logic [31:0] addr = 32'h30;\n\
         logic [3:0] q;\n\
         dff #(2*$bits(addr[5:4])) f (.clk(clk), .din({addr[5:4], addr[5:4]}), .dout(q));\n\
         initial begin #1 clk = 1; #1 $display(\"R q=%h\", q); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(got, ["R WIDTH=4", "R q=f"]);
}

#[test]
fn every_select_shape_in_a_localparam_and_a_range() {
    for &(s, w) in SHAPES {
        let src = format!(
            "module t;\n{DECLS}  localparam int B = $bits({s});\n  logic [$bits({s})-1:0] q;\n  \
             initial begin #1 $display(\"R B=%0d qb=%0d\", B, $bits(q)); $finish; end\nendmodule\n"
        );
        assert_eq!(lines(&src), [format!("R B={w} qb={w}")], "{s}");
    }
}

#[test]
fn generate_conditions_loops_and_unpacked_dimensions() {
    for &(s, w) in SHAPES {
        let big = if w > 6 { "big" } else { "small" };
        let src = format!(
            "module t;\n{DECLS}  if ($bits({s}) > 6) begin : g1 initial #1 $display(\"R big\"); end\n  \
             else begin : g2 initial #1 $display(\"R small\"); end\n  \
             localparam int L = $bits({s});\n  \
             for (genvar k = 0; k < $bits({s}); k++) begin : gf if (k == L - 1) begin : l \
             initial #1 $display(\"R last=%0d\", k); end end\n  \
             logic [3:0] a [$bits({s})];\n  \
             initial begin #2 $display(\"R n=%0d\", $size(a)); $finish; end\nendmodule\n"
        );
        let mut got = lines(&src);
        got.sort();
        let mut want = vec![
            format!("R {big}"),
            format!("R last={}", w - 1),
            format!("R n={w}"),
        ];
        want.sort();
        assert_eq!(got, want, "{s}");
    }
}

/// The override only: a `$bits` of a select inside a part-select bound is decided by
/// the constant-edge gate (§4.5.601), which reads the select's base as a variable
/// and refuses it — loud, so not pinned here.
#[test]
fn every_select_shape_in_an_override() {
    for &(s, w) in SHAPES {
        let src = format!(
            "module dff #(parameter WIDTH = 1) (input logic [WIDTH-1:0] din, output logic [WIDTH-1:0] dout);\n  \
             assign dout = din;\n  initial #1 $display(\"R WIDTH=%0d\", WIDTH);\nendmodule\n\
             module t;\n{DECLS}  logic [63:0] dd, qq;\n  \
             dff #(2*$bits({s})) f (.din(dd), .dout(qq));\n  \
             initial #2 $finish;\nendmodule\n"
        );
        assert_eq!(lines(&src), [format!("R WIDTH={}", 2 * w)], "{s}");
    }
}

#[test]
fn signed_members_array_elements_and_edges() {
    let got = lines(
        "module t;\n\
         typedef struct packed { logic signed [5:0] sf; logic [3:0] u; } s_t;\n\
         s_t s;\n\
         logic [7:0] m [0:3]; logic [2:0] v;\n\
         logic [31:0] addr;\n\
         typedef enum logic [5:0] {A, B} e_t;\n\
         e_t en;\n\
         localparam int P = $bits(s.sf);\n\
         localparam int Q = $bits({s.sf, s.u});\n\
         localparam int R = $bits($signed(s.u));\n\
         localparam int S = $bits({m[2], v});\n\
         localparam int T = $bits({m[2][5:1], v[1]});\n\
         localparam int U = $bits(addr[4:5]);\n\
         localparam int V = $bits(addr[40:33]);\n\
         localparam int X = $bits(en[3:1]);\n\
         initial begin #1 $display(\"R %0d %0d %0d %0d %0d %0d %0d %0d\", P, Q, R, S, T, U, V, X); $finish; end\n\
         endmodule\n",
    );
    // A select written against the vector's direction (`addr[4:5]`) is 2 in all
    // three oracles; one past the vector's end (`addr[40:33]`) is 8.
    assert_eq!(got, ["R 6 10 4 11 6 2 8 3"]);
}

#[test]
fn select_bounds_are_self_determined() {
    // `C + D` over two 4-bit constants is 0 as a bound (§11.5.1), so `[C+D:0]` is one
    // bit; `C+D+2` widens to the 32-bit `2` first (18). iverilog and verilator; sv2v
    // folds the bound unbounded (17).
    let got = lines(
        "module t;\n\
         localparam logic [3:0] C = 4'hF, D = 4'h1;\n\
         logic [31:0] addr;\n\
         localparam int A = $bits(addr[C+D:0]);\n\
         localparam int B = $bits(addr[0 +: C+D+2]);\n\
         logic [63:0] r;\n\
         initial begin r = {$bits(addr[C+D:0]){1'b1}}; #1 $display(\"R A=%0d B=%0d r=%h\", A, B, r); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(got, ["R A=1 B=18 r=0000000000000001"]);
}

#[test]
fn a_generate_case() {
    let got = lines(
        "module t;\n\
         logic [31:0] sig;\n\
         case ($bits(sig[5:4]))\n\
           2: begin : two initial #1 $display(\"R two\"); end\n\
           default: begin : other initial #1 $display(\"R other\"); end\n\
         endcase\n\
         initial #2 $finish;\n\
         endmodule\n",
    );
    assert_eq!(got, ["R two"]);
}

#[test]
fn ports_struct_ports_and_non_ansi_ports() {
    // OpenTitan `prim_edn_req`'s own spelling: an ANSI port of a package struct type.
    let got = lines(
        "package edn_pkg; typedef struct packed { logic edn_fips; logic [31:0] edn_bus; } edn_req_t; endpackage\n\
         module prim_edn_req (input edn_pkg::edn_req_t edn_i, input logic [11:0] din);\n\
         localparam int SyncWidth = $bits({edn_i.edn_fips, edn_i.edn_bus});\n\
         localparam int X = $bits(din[7:4]);\n\
         initial #1 $display(\"R SyncWidth=%0d X=%0d\", SyncWidth, X);\n\
         endmodule\n\
         module t; edn_pkg::edn_req_t r; logic [11:0] w; prim_edn_req u(.edn_i(r), .din(w)); initial #2 $finish; endmodule\n",
    );
    assert_eq!(got, ["R SyncWidth=33 X=4"]);
    // A non-ANSI port is in neither the decl prescan nor the header census: these
    // two positions read it from the net table.
    let got = lines(
        "module d #(parameter W = 1) (); initial #1 $display(\"R W=%0d\", W); endmodule\n\
         module c(din);\n\
         input logic [11:0] din;\n\
         if ($bits(din[7:4]) == 4) begin : four initial #1 $display(\"R four\"); end\n\
         d #(.W($bits(din[6:4]))) u();\n\
         endmodule\n\
         module t; logic [11:0] w; c u(.din(w)); initial #2 $finish; endmodule\n",
    );
    let mut got = got;
    got.sort();
    assert_eq!(got, ["R W=3", "R four"]);
}

/// A select inside a nested scope, or over a name a nested scope declares, is not
/// sized by the constant domain: the flat name tables would answer with the outer
/// declaration (a module `logic [31:0] x` for a block's `logic [3:0][7:0] x`).
#[test]
fn a_nested_scope_declines() {
    // Runtime positions keep their lowering's (right) answer — all three oracles.
    let runtime = [
        (
            "module t;\n  logic [31:0] x;\n  logic [63:0] r, r2;\n  if (1) begin : g\n    \
             logic [3:0][7:0] x;\n    assign r = {$bits(x[1]){1'b1}};\n    \
             initial #1 r2 = {$bits(x[1]){1'b1}};\n  end\n  \
             initial begin #2 $display(\"R %h %h\", r, r2); $finish; end\nendmodule\n",
            "R 00000000000000ff 00000000000000ff",
        ),
        (
            "module t;\n  logic [31:0] x;\n  function automatic logic [63:0] f();\n    \
             logic [3:0][7:0] x;\n    return {$bits(x[1]){1'b1}};\n  endfunction\n  \
             initial begin #1 $display(\"R %h\", f()); $finish; end\nendmodule\n",
            "R 00000000000000ff",
        ),
        (
            "module t;\n  logic [31:0] x;\n  logic [63:0] r;\n  initial begin : b\n    \
             logic [3:0][7:0] x;\n    #1 r = {$bits(x[1]){1'b1}};\n    \
             $display(\"R %h\", r);\n    $finish;\n  end\nendmodule\n",
            "R 00000000000000ff",
        ),
        (
            "interface ifc;\n  logic [3:0][7:0] d;\n  logic [63:0] r;\n  \
             initial begin #1 r = {$bits(d[1]){1'b1}}; $display(\"R %h\", r); end\n\
             endinterface\nmodule t;\n  logic [31:0] d;\n  ifc i();\n  initial #2 $finish;\nendmodule\n",
            "R 00000000000000ff",
        ),
        (
            "package pk;\n  function automatic logic [63:0] pf();\n    logic [3:0][7:0] x;\n    \
             return {$bits(x[1]){1'b1}};\n  endfunction\nendpackage\n\
             module t;\n  logic [31:0] x;\n  initial begin #1 $display(\"R %h\", pk::pf()); $finish; end\nendmodule\n",
            "R 00000000000000ff",
        ),
    ];
    for (src, want) in runtime {
        assert_eq!(lines(src), [want], "{src}");
    }
    // Constant positions stay loud (all three oracles answer 8 or 16).
    let loud = [
        // A generate block's own declaration.
        "module t;\n  logic [31:0] x;\n  if (1) begin : g\n    logic [3:0][7:0] x;\n    \
         localparam int A = $bits(x[1]);\n    initial #1 $display(\"R A=%0d\", A);\n  end\nendmodule\n",
        // The module's `x` declared after a generate block that declares its own.
        "module t;\n  if (1) begin : g\n    logic [3:0][7:0] x;\n    \
         localparam int A = $bits(x[1]);\n    initial #1 $display(\"R A=%0d\", A);\n  end\n  \
         logic [31:0] x;\nendmodule\n",
        // The new signing arm under a shadow.
        "module t;\n  logic [31:0] x;\n  if (1) begin : g\n    logic [15:0] x;\n    \
         localparam int A = $bits($signed(x));\n    initial #1 $display(\"R A=%0d\", A);\n  end\nendmodule\n",
        // An interface's own declaration (the parent declares the same name).
        "interface ifc;\n  logic [3:0][7:0] sig;\n  localparam int B = $bits(sig[1]);\n  \
         initial #1 $display(\"R B=%0d\", B);\nendinterface\n\
         module t;\n  logic [31:0] sig;\n  ifc u();\n  initial #2 $finish;\nendmodule\n",
        // A top-level use of a name a nested scope declares: declined too.
        "module t;\n  logic [31:0] x;\n  localparam int A = $bits(x[3:0]);\n  \
         function automatic int f(); logic [3:0][7:0] x; return 0; endfunction\n  \
         initial #1 $display(\"R A=%0d\", A);\nendmodule\n",
        // A routine's return range is inside the routine.
        "module t;\n  logic [31:0] sig;\n  \
         function automatic logic [$bits(sig[7:0])-1:0] f(); return '1; endfunction\n  \
         initial #1 $display(\"R %0d\", $bits(f()));\nendmodule\n",
    ];
    for src in loud {
        let (out, err, code) = run(src);
        assert_eq!(code, Some(1), "{src}\nstdout:\n{out}\nstderr:\n{err}");
        assert!(err.contains("error[VITA-E3009]"), "{src}\nstderr:\n{err}");
    }
}

#[test]
fn shapes_outside_the_select_rule_stay_loud() {
    let cases = [
        // A string's index is a byte (both runnable oracles 8): not this rule.
        "string s = \"abc\";\n  localparam int B = $bits(s[0]);",
        // A real has no selectable bits (verilator refuses; iverilog 1).
        "real r;\n  localparam int B = $bits(r[3]);",
        // `$signed` takes an integral operand (verilator refuses; sv2v rejects).
        "real r;\n  localparam int B = $bits($signed(r));",
        // A slice of an unpacked array (all three 16).
        "logic [7:0] m [0:3];\n  localparam int B = $bits(m[1:2]);",
        // A select of a multi-dimensional packed array is an element (all three 8);
        // the constant domain does not size it yet.
        "logic [3:0][7:0] p2;\n  localparam int B = $bits(p2[1]);",
    ];
    for c in cases {
        let src = format!("module t;\n  {c}\n  initial begin #1 $display(\"R B=%0d\", B); $finish; end\nendmodule\n");
        let (out, err, code) = run(&src);
        assert_eq!(code, Some(1), "{c}\nstdout:\n{out}\nstderr:\n{err}");
        assert!(err.contains("error[VITA-E3009]"), "{c}\nstderr:\n{err}");
    }
}

/// The fold must run in the module's own scope, not only the select's text sit at
/// its top level: a module typedef's range is re-folded where the type is used, and
/// inside a generate block that declares an enum label `x` the module's
/// `logic [3:0][7:0] x` is not what the block sees (round-2 review a25, a36: vita
/// 2, all three oracles 16). Both now decline, as before the rule.
#[test]
fn a_module_typedef_used_inside_a_generate_block_declines() {
    let cases = [
        "module t;\n  logic [3:0][7:0] x;\n  typedef logic [$bits(x[2:1])-1:0] t_t;\n  \
         if (1) begin : g\n    typedef enum logic [1:0] {x, y} e_t;\n    t_t v;\n    \
         initial begin #1 $display(\"R vb=%0d\", $bits(v)); $finish; end\n  end\nendmodule\n",
        "module t;\n  logic [3:0][7:0] x;\n  typedef enum logic [$bits(x[2:1])-1:0] {A0, A1} e2_t;\n  \
         if (1) begin : g\n    typedef enum logic [1:0] {x, y} e_t;\n    e2_t v;\n    \
         initial begin #1 $display(\"R vb=%0d\", $bits(v)); $finish; end\n  end\nendmodule\n",
    ];
    for src in cases {
        let (out, err, code) = run(src);
        assert_eq!(code, Some(1), "{src}\nstdout:\n{out}\nstderr:\n{err}");
        assert!(err.contains("error[VITA-E3009]"), "{src}\nstderr:\n{err}");
    }
}

/// In an interface body the select and signing arms decline (no census there), so
/// a select keeps PRE's refusal, as every other `$bits` there keeps PRE's reading.
#[test]
fn an_interface_body_keeps_the_refusal() {
    for e in ["addr[3:0]", "$signed(addr)", "addr[2 +: 4]"] {
        let src = format!(
            "interface ifc; logic [31:0] addr; localparam int A = $bits({e}); \
             initial #1 $display(\"R A=%0d\", A); endinterface\n\
             module t;\n  ifc i();\n  initial #2 $finish;\nendmodule\n"
        );
        let (out, err, code) = run(&src);
        assert_eq!(code, Some(1), "{e}\nstdout:\n{out}\nstderr:\n{err}");
        assert!(err.contains("error[VITA-E3009]"), "{e}\nstderr:\n{err}");
    }
}

/// `$signed` takes an integral operand: an `event` is refused (Verilator rejects it,
/// iverilog answers 0), as a `real` is.
#[test]
fn signed_of_an_event_is_refused() {
    let (out, err, code) = run(
        "module t;\n  event ev;\n  localparam int A = $bits($signed(ev));\n  \
         initial begin #1 $display(\"R A=%0d\", A); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(1), "stdout:\n{out}\nstderr:\n{err}");
    assert!(err.contains("error[VITA-E3009]"), "stderr:\n{err}");
}

/// A declaring block under a timing control is a nested scope too: the census takes
/// its scopes with the walker the block-local lowering classifies with
/// (`gather_nested_block_locals`), so a size cast over the block's own
/// `logic [3:0][7:0] x` is not sized against the module's `logic [31:0] x` (round-3
/// review w16–w29: vita `r=…01`, all three oracles `ef`). PRE's refusal stays.
#[test]
fn a_declaring_block_under_a_timing_control_declines() {
    let heads = [
        "initial #1 begin",
        "initial wait (go) begin",
        "initial @(go) begin",
        "initial begin @(go) begin",
        "initial repeat (1) @(go) begin",
        "initial if (1) #1 begin",
        "always begin #1 begin",
        "initial forever #1 begin",
    ];
    for head in heads {
        let tail = if head.matches("begin").count() == 2 {
            "end end"
        } else {
            "end"
        };
        let src = format!(
            "module t;\n  logic [31:0] x;\n  logic [63:0] r;\n  logic go = 0;\n  \
             logic [63:0] val = 64'h0123_4567_89AB_CDEF;\n  \
             {head}\n    logic [3:0][7:0] x;\n    r = $bits(x[1])'(val);\n  {tail}\n  \
             initial begin #1 go = 1; #2 $display(\"R r=%h\", r); $finish; end\nendmodule\n"
        );
        let (out, err, code) = run(&src);
        assert_eq!(code, Some(1), "{head}\nstdout:\n{out}\nstderr:\n{err}");
        assert!(err.contains("error[VITA-E3009]"), "{head}\nstderr:\n{err}");
    }
}
