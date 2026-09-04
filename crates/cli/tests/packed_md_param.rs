//! A parameter with MORE THAN ONE packed dimension — `localparam lfsr_perm_t X =
//! {160'h…}` over `typedef logic [W-1:0][$clog2(W)-1:0] lfsr_perm_t` (ROADMAP §3 ⑤
//! ⓐ, ibex_pkg.sv:742) and the keyword spelling `parameter logic [Dw-1:0][Iw-1:0]
//! StatePerm` (prim_lfsr.sv:53), in a body, a package, or an ANSI header.
//!
//! `ParamDecl` carries one range and the override channel moves one flat value, so
//! the parser declares the parameter FLAT (`[total-1:0]`) and rewrites every
//! select chain on its name — `P[i]`, `P[i][j]`, `P[i][a:b]`, `P[i][o+:w]`,
//! `P[a:b]` — to the flat bit/part-select those bits occupy (`packed_md.rs`, the
//! packed-struct member precedent). Offsets and widths are expressions, so a
//! header whose dims name other parameters (`[Dw-1:0][Iw-1:0]`) is right under
//! every override. Whole-value reads, `$bits`, the header default and an instance
//! override see exactly the packed bits.
//!
//! Every value here was measured on verilator 5.050; iverilog 13.0 cannot parse a
//! packed-array parameter ("sorry: packed array parameters are not supported yet")
//! and aborts on a multi-dimensional packed typedef VARIABLE, so verilator is the
//! sole oracle. The slice's census (195 cells, a keyword-spelled control twin and a
//! variable twin per position) is in ROADMAP_ARCHIVE §4.5.412.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pmd_{}_{n}", std::process::id()));
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

fn digest(src: &str) -> String {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "expected exit 0, got {rc:?}:\n{out}");
    out.lines()
        .find_map(|l| l.strip_prefix("DIGEST="))
        .unwrap_or_else(|| panic!("no DIGEST line:\n{out}"))
        .to_string()
}

fn loud(src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "expected a loud reject:\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in:\n{out}");
}

/// `[3:0][4:0]`: P[3]=3, P[2]=17, P[1]=9, P[0]=30.
const V: &str = "20'b00011_10001_01001_11110";

fn body(decl: &str, fmt: &str, args: &str) -> String {
    format!(
        "module tb;\n{decl}\n  initial begin #1 $display(\"DIGEST={fmt}\", {args}); #1 $finish; end\nendmodule"
    )
}

// ───────────────────────── the ibex shape, element and bit reads ─────────────────────────

#[test]
fn a_typedef_and_a_keyword_spelling_read_the_same_elements() {
    // verilator 5.050: `30 9 17 3 | 30 3 | 1 1` / `20 5 00011100010100111110`.
    let src = format!(
        "module tb;
  typedef logic [3:0][4:0] perm_t;
  localparam perm_t P = {V};
  localparam logic [3:0][4:0] K = {V};
  initial begin
    $display(\"DIGEST=%0d %0d %0d %0d | %0d %0d | %0d %0d\", P[0], P[1], P[2], P[3], K[0], K[3], P[1][0], K[3][1]);
    $display(\"DIGEST2=%0d %0d %b\", $bits(P), $bits(P[0]), P);
    #1 $finish;
  end
endmodule"
    );
    let (out, rc) = run(&src);
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("DIGEST=30 9 17 3 | 30 3 | 1 1"), "{out}");
    assert!(out.contains("DIGEST2=20 5 00011100010100111110"), "{out}");
}

#[test]
fn part_and_indexed_selects_on_an_element_and_on_the_outer_dimension() {
    // verilator 5.050.
    let d = format!("  localparam logic [3:0][4:0] P = {V};");
    assert_eq!(
        digest(&body(&d, "%b %b %b", "P[1][3:0], P[0][4:2], P[2][1:0]")),
        "1001 111 01"
    );
    assert_eq!(
        digest(&body(&d, "%b %b %b", "P[1][1+:3], P[0][4-:2], P[3][0+:5]")),
        "100 11 00011"
    );
    assert_eq!(
        digest(&body(&d, "%b %b %b", "P[1:0], P[3:2], P[2:1]")),
        "0100111110 0001110001 1000101001"
    );
    assert_eq!(
        digest(&body(&d, "%b %b", "P[0+:2], P[3-:2]")),
        "0100111110 0001110001"
    );
}

#[test]
fn a_runtime_index_in_every_position() {
    // verilator 5.050: `17 1 100 3 1` — `e = P[i]` (i=2), `P[i][j]` (j=4 = the MSB of
    // 17 = 10001), `P[i][j-:3]`, `P[i+1]` (= P[3]), `P[3-i][j-1]`.
    let src = format!(
        "module tb;
  typedef logic [3:0][4:0] perm_t;
  localparam perm_t P = {V};
  logic [1:0] i = 2'd2; logic [2:0] j = 3'd4; logic [4:0] e; logic b; logic [2:0] s;
  initial begin
    e = P[i]; b = P[i][j]; s = P[i][j-:3];
    #1 $display(\"DIGEST=%0d %0d %b %0d %0d\", e, b, s, P[i+1], P[3-i][j-1]);
    #1 $finish;
  end
endmodule"
    );
    assert_eq!(digest(&src), "17 1 100 3 1");
}

#[test]
fn an_element_indexes_another_vector_the_prim_lfsr_way() {
    // verilator 5.050: the generate-for reads `mem[P[k]]`.
    let src = format!(
        "module tb;
  localparam logic [3:0][4:0] P = {V};
  logic [31:0] mem = 32'hA5C3_0F1E; logic [3:0] o;
  for (genvar k = 0; k < 4; k++) begin : g
    assign o[k] = mem[P[k]];
  end
  initial begin #1 $display(\"DIGEST=%b\", o); #1 $finish; end
endmodule"
    );
    assert_eq!(digest(&src), "1110");
}

// ───────────────────────── constant contexts ─────────────────────────

#[test]
fn an_element_folds_in_a_constant_a_generate_condition_and_a_case_item() {
    let d = format!("  localparam logic [3:0][4:0] P = {V};");
    // verilator 5.050: `9 0 001 5 20 9`.
    assert_eq!(
        digest(&format!(
            "module tb;
{d}
  localparam int X = P[1];
  localparam int Y = P[1][2];
  localparam logic [2:0] Z = P[2][2:0];
  localparam int B = $bits(P[0]);
  localparam int W = $bits(P);
  logic [X-1:0] v = '1;
  initial begin #1 $display(\"DIGEST=%0d %0d %b %0d %0d %0d\", X, Y, Z, B, W, $bits(v)); #1 $finish; end
endmodule"
        )),
        "9 0 001 5 20 9"
    );
    // verilator 5.050: `1 31 10 18 4` — generate-if on `P[1] == 9`, per-block widths
    // `logic [P[k]:0]`.
    assert_eq!(
        digest(&format!(
            "module tb;
{d}
  logic [7:0] o;
  generate if (P[1] == 9) begin : a assign o = 8'd1; end else begin : b assign o = 8'd2; end endgenerate
  for (genvar k = 0; k < 4; k++) begin : g
    localparam int E = P[k];
    logic [E:0] w = '1;
  end
  initial begin #1 $display(\"DIGEST=%0d %0d %0d %0d %0d\", o, $bits(g[0].w), $bits(g[1].w), $bits(g[2].w), $bits(g[3].w)); #1 $finish; end
endmodule"
        )),
        "1 31 10 18 4"
    );
    // verilator 5.050: `2` — case items are elements.
    assert_eq!(
        digest(&format!(
            "module tb;
{d}
  logic [4:0] x = 5'd17; int r;
  always_comb case (x) P[0]: r = 0; P[1]: r = 1; P[2]: r = 2; P[3]: r = 3; default: r = 9; endcase
  initial begin #1 $display(\"DIGEST=%0d\", r); #1 $finish; end
endmodule"
        )),
        "2"
    );
}

// ───────────────────────── dimension shapes ─────────────────────────

#[test]
fn three_dimensions_ascending_and_non_zero_lsb() {
    // verilator 5.050: `001110 10 01 | 1 7 1 0` — a 3-dim `[1:0][2:0][1:0]`, an
    // ascending outer `[0:2]` (index 0 is the MSB element) with a `[3:1]` inner.
    let src = "module tb;
  typedef logic [1:0][2:0][1:0] t3;
  typedef logic [0:2][3:1] asc_t;
  localparam t3 A = 12'b10_11_01_00_11_10;
  localparam asc_t B = {3'd1, 3'd2, 3'd7};
  logic [1:0] r;
  always_comb r = A[1][0];
  initial begin
    #1 $display(\"DIGEST=%b %b %b | %0d %0d %0d %0d\", A[0], A[1][2], r, B[0], B[2], B[1][2], B[0][3]);
    #1 $finish;
  end
endmodule";
    assert_eq!(digest(src), "001110 10 01 | 1 7 1 0");
}

#[test]
fn dimensions_that_name_parameters_fold_and_survive_an_override() {
    // verilator 5.050: `0110 0000` — the prim_lfsr shape: a header
    // `logic [Dw-1:0][Iw-1:0] SP` whose dims are the instance's own parameters, its
    // default overridden with a package constant of a `$clog2` typedef, and a second
    // instance overriding the aggregate.
    let src = "package p;
  parameter int W = 4;
  typedef logic [W-1:0][$clog2(W)-1:0] perm_t;
  parameter perm_t Dflt = {8'h1b};
endpackage
module lfsr #(parameter int Dw = 4, parameter int Iw = 2, parameter logic [Dw-1:0][Iw-1:0] SP = '0) (input logic [Dw-1:0] i, output logic [Dw-1:0] o);
  for (genvar k = 0; k < Dw; k++) begin : g
    assign o[k] = i[SP[k]];
  end
endmodule
module top #(parameter p::perm_t Perm = p::Dflt) (input logic [3:0] i, output logic [3:0] o);
  lfsr #(.Dw(p::W), .Iw($clog2(p::W)), .SP(Perm)) u (.i(i), .o(o));
endmodule
module tb;
  logic [3:0] i = 4'b0110, o, o2;
  top u (.i(i), .o(o));
  top #(.Perm({2'd3,2'd3,2'd0,2'd0})) u2 (.i(i), .o(o2));
  initial begin #1 $display(\"DIGEST=%b %b\", o, o2); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "0110 0000");
    // verilator 5.050: `0110 10100011` — two instances with DIFFERENT dims.
    let src2 = "module lfsr #(parameter int Dw = 4, parameter int Iw = 2, parameter logic [Dw-1:0][Iw-1:0] SP = '0) (input logic [Dw-1:0] i, output logic [Dw-1:0] o);
  for (genvar k = 0; k < Dw; k++) begin : g
    assign o[k] = i[SP[k]];
  end
endmodule
module tb;
  logic [3:0] i4 = 4'b0110, o4; logic [7:0] i8 = 8'b1010_0011, o8;
  lfsr #(.Dw(4), .Iw(2), .SP({2'd0,2'd1,2'd2,2'd3})) a (.i(i4), .o(o4));
  lfsr #(.Dw(8), .Iw(3), .SP({3'd7,3'd6,3'd5,3'd4,3'd3,3'd2,3'd1,3'd0})) b (.i(i8), .o(o8));
  initial begin #1 $display(\"DIGEST=%b %b\", o4, o8); #1 $finish; end
endmodule";
    assert_eq!(digest(src2), "0110 10100011");
}

// ───────────────────────── package carry ─────────────────────────

#[test]
fn a_package_parameter_crosses_a_wildcard_import_and_a_scoped_read() {
    // verilator 5.050: `30 9 1 9 | 9 17 5` — the variable twin `perm_t v` (elaborate's
    // own packed-md path) reads the same as the parameter, and the constant contexts
    // fold through the import and through `p::P`.
    let src = format!(
        "package p;
  typedef logic [3:0][4:0] perm_t;
  localparam perm_t P = {V};
endpackage
module tb;
  import p::*;
  perm_t v = {V};
  localparam int X = P[1];
  localparam int Y = p::P[2];
  localparam int Z = $bits(P[0]);
  logic [4:0] e;
  initial begin
    e = v[1];
    $display(\"DIGEST=%0d %0d %0d %0d | %0d %0d %0d\", v[0], v[1], v[3][0], e, X, Y, Z);
    #1 $finish;
  end
endmodule"
    );
    assert_eq!(digest(&src), "30 9 1 9 | 9 17 5");
}

#[test]
fn a_local_declaration_shadows_the_imported_parameter() {
    // verilator 5.050: `30 3 20` in every position — a module-local variable of the
    // same name declared before or after `import p::*`, a block-local one, and a
    // local SCALAR parameter (`P[0]` is then a bit) — IEEE §26.3.
    let pkg = "package p;
  parameter int W = 4;
  typedef logic [W-1:0][$clog2(W)-1:0] perm_t;
  parameter perm_t P = {2'd3,2'd3,2'd0,2'd1};
endpackage
";
    for (decl, imp) in [
        (format!("  logic [3:0][4:0] P = {V};\n  import p::*;"), ""),
        (
            "  import p::*;".to_string(),
            &*format!("  logic [3:0][4:0] P = {V};"),
        ),
    ] {
        let src = format!(
            "{pkg}module tb;
{decl}
{imp}
  initial begin #1 $display(\"DIGEST=%0d %0d %0d\", P[0], P[3], $bits(P)); #1 $finish; end
endmodule"
        );
        assert_eq!(digest(&src), "30 3 20", "{src}");
    }
    // A BLOCK-local shadow: the values are the local's. (`$bits(P)` there still
    // answers the imported constant's 8 — pre-existing on the keyword spelling,
    // both oracles 20: the wildcard-shadow skip set omits block-local names, ROADMAP
    // §2 🆕 L ⓙ — so it is not pinned here.)
    let blk = format!(
        "{pkg}module tb;
  import p::*;
  initial begin : blk
    logic [3:0][4:0] P = {V};
    #1 $display(\"DIGEST=%0d %0d\", P[0], P[3]);
    #1 $finish;
  end
endmodule"
    );
    assert_eq!(digest(&blk), "30 3");
    let scalar = format!(
        "{pkg}module tb;
  import p::*;
  localparam logic [7:0] P = 8'hA5;
  initial begin #1 $display(\"DIGEST=%0d %0d %0d\", P[0], P[7], $bits(P)); #1 $finish; end
endmodule"
    );
    assert_eq!(digest(&scalar), "1 1 8");
    // A block-local plain declaration shadows the parameter for the BLOCK only:
    // the read after the block is the parameter's element again (verilator 5.050:
    // `0 9`; before the scope fix the outer read was the flat bit 1).
    let blk_shadow = format!(
        "module tb;
  localparam logic [3:0][4:0] P = {V};
  logic [4:0] o1, o2;
  initial begin : b
    logic [7:0] P = 8'hA5;
    o1 = P[1];
  end
  initial o2 = P[1];
  initial begin #1 $display(\"DIGEST=%0d %0d\", o1, o2); #1 $finish; end
endmodule"
    );
    assert_eq!(digest(&blk_shadow), "0 9");
}

#[test]
fn a_port_or_a_formal_named_like_the_parameter_shadows_it() {
    // Review A-1: a module port and a function/task formal named `P` must read the
    // port/formal, not the parameter's element. verilator 5.050 (keyword-scalar
    // control identical on PRE, POST, verilator, iverilog): `00 0 a 0011 0011`.
    let src = "package p;
  parameter logic [1:0][3:0] P = 8'hA5;
endpackage
module sub import p::*; (input logic [7:0] P, output logic o);
  assign o = P[1];
endmodule
module na (P, o);
  input logic [7:0] P; output logic o;
  assign o = P[1];
endmodule
module tb;
  localparam logic [1:0][3:0] P = 8'hA5;
  function automatic logic [7:0] f(input logic [7:0] P);
    return {7'b0, P[1]};
  endfunction
  task automatic t(input logic [7:0] P, output logic [3:0] r);
    r = P[3:0];
  endtask
  logic o1, o2; logic [3:0] tr;
  sub u (.P(8'h3C), .o(o1));
  na v (.P(8'h3C), .o(o2));
  initial begin
    t(8'h3C, tr);
    #1 $display(\"DIGEST=%h %0d %h %b %b %0d %0d\", f(8'h3C), o1, P[1], tr, P[0], o2, P[1][1]);
    #1 $finish;
  end
endmodule";
    assert_eq!(digest(src), "00 0 a 1100 0101 0 1");
}

#[test]
fn an_explicit_import_wins_over_a_wildcard_and_dims_cross_a_scoped_read() {
    // IEEE §26.3/§26.8, pinned in §4.5.410 for a scalar (PRE `a5`, iverilog `a5`;
    // verilator 5.050 alone answers the WILDCARD's `1b` on the scalar control and
    // p's dims here — disqualified on this axis by its own scalar answer).
    let src = "package p;
  parameter int W = 4;
  parameter logic [W-1:0][$clog2(W)-1:0] P = {2'd3,2'd3,2'd0,2'd1};
endpackage
package q;
  parameter logic [1:0][2:0] P = {3'd5, 3'd6};
endpackage
module tb;
  import p::*;
  import q::P;
  initial begin #1 $display(\"DIGEST=%0d %0d %0d\", P[0], P[1], $bits(P)); #1 $finish; end
endmodule";
    assert_eq!(digest(src), "6 5 6");
    // A scoped read of a package parameter whose dims name the package's own
    // constants (`[W-1:0][$clog2(W)-1:0]`), with NO import of `W` here — the dims
    // are re-spelled `p::W` at `endpackage`. verilator 5.050: `1 3 2 1 3 8`.
    let scoped = "package p;
  parameter int W = 4;
  parameter logic [W-1:0][$clog2(W)-1:0] P = {2'd3,2'd3,2'd0,2'd1};
  localparam logic [W-1:0][$clog2(W)-1:0] L = {2'd1,2'd0,2'd2,2'd3};
endpackage
module tb;
  logic [1:0] i = 2'd2;
  initial begin
    #1 $display(\"DIGEST=%0d %0d %0d %0d %0d %0d\", p::P[0], p::P[3], p::L[1], p::P[2][1], p::P[i], $bits(p::P));
    #1 $finish;
  end
endmodule";
    assert_eq!(digest(scoped), "1 3 2 1 3 8");
}

#[test]
fn a_signed_typedef_element_is_read_like_the_variable_twin() {
    // `typedef logic signed [3:0][4:0]`: `$signed(P[2]) < 0` on element 10001 is 1.
    // iverilog 13.0 and verilator 5.050 both answer `17 10 0 1` for the VARIABLE
    // twin; verilator answers `… 0` for the parameter spelling only (contradicting
    // its own variable answer) — the variable twin is the pin.
    for decl in [
        "typedef logic signed [3:0][4:0] t; localparam t P = {5'd3, 5'd17, 5'd9, 5'd30};",
        "localparam logic signed [3:0][4:0] P = {5'd3, 5'd17, 5'd9, 5'd30};",
        "logic signed [3:0][4:0] P = {5'd3, 5'd17, 5'd9, 5'd30};",
    ] {
        assert_eq!(
            digest(&body(
                &format!("  {decl}"),
                "%0d %0d %0d %0d",
                "P[2], P[1] + 1, P < 0, $signed(P[2]) < 0"
            )),
            "17 10 0 1",
            "{decl}"
        );
    }
}

// ───────────────────────── the v1 limits stay loud ─────────────────────────

#[test]
fn the_v1_limits_are_loud_not_silent() {
    let d = format!("  localparam logic [3:0][4:0] P = {V};");
    // More selects than dimensions (the flat twin would answer `x`).
    loud(
        &body(&d, "%0d", "P[1][2][3]"),
        "at most one select per packed dimension",
    );
    // A range that is not the last select.
    loud(
        &body(&d, "%b", "P[1:0][2]"),
        "an index in every select but the last",
    );
    // An array parameter of a multi-dimensional packed element type.
    loud(
        &body(
            &format!("  localparam logic [3:0][4:0] A[2] = '{{{V}, {V}}};"),
            "%0d",
            "A[0][1]",
        ),
        "an array parameter of a multi-dimensional packed type",
    );
    // A write, an assignment pattern and `foreach` stay as loud as on a scalar
    // parameter.
    loud(
        &format!(
            "module tb;\n{d}\n  initial begin P[1] = 5'd1; #1 $display(\"DIGEST=%0d\", P[1]); #1 $finish; end\nendmodule"
        ),
        "E3010",
    );
    // `$size(P)` is answered by the parser from the recorded dimensions (§3 ⑤ ⓔ):
    // the OUTERMOST packed dimension `[3:0]` — 4, as verilator (was a loud E3009 pin;
    // an elaborate-side answer would have read the flattened `[19:0]` as 20).
    assert_eq!(digest(&body(&d, "%0d", "$size(P)")), "4");
    assert_eq!(
        digest(&body(
            &d,
            "%0d %0d %0d",
            "$size(P, 2), $high(P, 2), $dimensions(P)"
        )),
        "5 4 2"
    );
    // A REVERSED range select (review B1): the flat/variable twins are loud
    // ("part-select bounds … out of order"); the parameter must not answer 0 bits.
    loud(&body(&d, "%b", "P[1][2:4]"), "out of order");
    loud(&body(&d, "%b", "P[1:2]"), "out of order");
    loud(
        &body(
            "  localparam logic [1:3][4:1] B = 12'b1001_0110_1100;",
            "%b",
            "B[3:2]",
        ),
        "out of order",
    );
    // A block-local plain decl shadowing a package struct VARIABLE keeps PRE's loud
    // (review B2): a block-local flattens to a module net of the same name, so a
    // restored struct binding would read the local through the package layout.
    loud(
        "package p;
  typedef struct packed { logic [3:0] f; logic [3:0] g; } s_t;
  s_t SS = 8'hA5;
endpackage
module tb;
  import p::*;
  initial begin : blk
    logic [7:0] SS;
    SS = 8'h11;
  end
  initial begin #1 $display(\"DIGEST=%h %h\", SS.g, SS); #1 $finish; end
endmodule",
        "E3010",
    );
    loud(
        &body(
            "  localparam logic [3:0][4:0] P = '{5'd3, 5'd17, 5'd9, 5'd30};",
            "%0d",
            "P[0]",
        ),
        "E3009",
    );
}
