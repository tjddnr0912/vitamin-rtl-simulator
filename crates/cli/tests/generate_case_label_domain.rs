//! A generate-case label is compared with the case expression by §12.5 case equality in
//! the 4-state bit domain at full width (ROADMAP §2 🆕 T, `elaborate/src/gen_case.rs`).
//! Before, the label was folded as an i64 and compared by value: a label only the bit
//! domain reads was skipped as a non-match (`default` at exit 0), and a mixed-sign pair
//! (`-1` against `32'hFFFFFFFF`) compared as two integers.
//!
//! Oracles, recorded per test: iverilog 13.0 (`-g2012`, `vvp -n`), sv2v 0.0.13 →
//! iverilog 13.0, verilator 5.052 (`--binary --timing`; it refuses x/z/? generate-case
//! labels, so x cells are iverilog's alone). PRE = the binary before 🆕 T (4ace7617).
//!
//! The SIZING of the comparison is an oracle split (census cells W01–W20): verilator
//! sizes the case expression and every item together at the longest width, unsigned if
//! any is unsigned — what all three tools and vita's runtime do for a procedural `case`
//! — while iverilog sizes each expression at its own width and compares one item at a
//! time, contradicting its own procedural `case` (`case (4'd15 + 4'd1) 0: … 16: …` takes
//! `16` procedurally and `0` in a generate region). A label decides only where the two
//! readings agree; where they disagree, and where the bit domain cannot read the case, it
//! keeps exactly PRE's answer (its i64 value compared, else a non-match). Nothing is
//! refused.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Run {
    out: String,
    err: String,
    code: i32,
}

impl Run {
    /// The `$display` lines, sorted (one line per generate-case; their print order is
    /// not what these pins measure), without the simulator's trailer.
    fn lines(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .out
            .lines()
            .filter(|l| !l.starts_with("simulation ended"))
            .collect();
        v.sort_unstable();
        v
    }
}

fn run(src: &str) -> Run {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_gencase_dom_{}_{n}", std::process::id()));
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
    Run {
        out: String::from_utf8_lossy(&out.stdout).into_owned(),
        err: String::from_utf8_lossy(&out.stderr).into_owned(),
        code: out.status.code().unwrap_or(-1),
    }
}

/// One generate-case per `(id, scrutinee, label)`: `<id> item` when the label matches,
/// `<id> default` otherwise.
fn cases(decls: &str, cells: &[(&str, &str, &str)]) -> String {
    let mut s = format!("`timescale 1ns/1ns\nmodule t;\n{decls}\n");
    for (id, scrut, lab) in cells {
        s += &format!(
            "  case ({scrut})\n    {lab}: begin : i_{id} initial $display(\"{id} item\"); end\n    \
             default: begin : d_{id} initial $display(\"{id} default\"); end\n  endcase\n"
        );
    }
    s + "endmodule\n"
}

fn check(src: &str, want: &[&str]) {
    let r = run(src);
    assert_eq!(r.code, 0, "stderr:\n{}\nsrc:\n{src}", r.err);
    let mut want = want.to_vec();
    want.sort_unstable();
    assert_eq!(r.lines(), want, "stdout:\n{}\nsrc:\n{src}", r.out);
}

/// A label whose value only the bit domain reads — a 65-bit parameter, a string-literal
/// sum, a shift of a wide name, `$isunknown`, a genvar against a 65-bit concatenation —
/// is compared, not skipped. iverilog and sv2v → iverilog print exactly these lines;
/// verilator refuses this design for X09's x literal and prints the same line for every
/// other cell run alone (census T1, L06, L26, R05, L30, M08, N08). PRE printed `default`
/// for every one of them, and `N08 default 1`.
#[test]
fn a_label_only_the_bit_domain_reads_is_compared() {
    let mut src = cases(
        "  localparam P1 = (4'b1100 ==? 4'b1?00);\n  localparam [64:0] LPA = {64'd0, P1};\n  \
         localparam logic [7:0] P = 8'h62;\n  localparam [64:0] W = {1'b1, 64'd0};\n  \
         localparam M = 8'h62;",
        &[
            ("T1", "1", "LPA"),
            ("L06", "P", "\"a\" + 1"),
            ("L26", "8'h62", "\"a\" + 1"),
            ("R05", "M", "\"a\" + 1"),
            ("L30", "1", "W >> 64"),
            ("X09", "1", "$isunknown(4'b1x00)"),
            ("M08", "1", "65'd2, LPA"),
        ],
    );
    src = src.replace(
        "endmodule\n",
        "  for (genvar i = 0; i < 3; i++) begin : g\n    case (i)\n      {64'd0, 1'b1}: begin : c \
         initial $display(\"N08 item %0d\", i); end\n      default: begin : d initial \
         $display(\"N08 default %0d\", i); end\n    endcase\n  end\nendmodule\n",
    );
    check(
        &src,
        &[
            "T1 item",
            "L06 item",
            "L26 item",
            "R05 item",
            "L30 item",
            "X09 item",
            "M08 item",
            "N08 default 0",
            "N08 item 1",
            "N08 default 2",
        ],
    );
}

/// §12.5 sign and width of the pair: unsigned unless both are signed, the narrower side
/// extended. Every cell below is answered identically by iverilog, sv2v → iverilog and
/// verilator, and by both sizings. PRE compared the two i64 values, so a negative signed
/// side never matched its unsigned bit pattern (`-1` vs `32'hFFFFFFFF`, `4'sb1111` vs
/// `8'd15`); it printed the opposite line for each of the first 20.
#[test]
fn case_equality_signs_and_sizes_the_pair() {
    let src = cases(
        "  localparam int PI = -1;\n  localparam logic signed [7:0] PS8 = -1;\n  \
         localparam logic signed [3:0] PS = -1;\n  localparam UP = 4'sb1111;\n  \
         localparam M = -1;\n  localparam P0 = 0;\n  localparam int PI0 = 0;\n  \
         localparam logic [3:0] P4 = 4'd15;\n  \
         typedef enum logic signed [3:0] {EM = -1, EZ = 0} e_t;\n  parameter e_t PE = EM;",
        &[
            ("S06", "-1", "32'hFFFFFFFF"),
            ("S07", "4'sb1111", "4'b1111"),
            ("S11", "4'sb1111", "8'd15"),
            ("S16", "32'hFFFFFFFF", "-1"),
            ("S21", "4'sb1000", "4'd8"),
            ("S23", "4'd8", "4'sb1000"),
            ("S29", "PI", "32'hFFFFFFFF"),
            ("S32", "PS8", "8'hFF"),
            ("S36", "4'b1111", "UP"),
            ("S37", "4'b1111", "PS"),
            ("L24", "4'd15", "PS"),
            ("E05", "PE", "4'b1111"),
            ("E06", "4'b1111", "EM"),
            ("R04", "M", "32'hFFFFFFFF"),
            ("R06", "P0 - 1", "32'hFFFFFFFF"),
            ("R07", "PI0 - 1", "32'hFFFFFFFF"),
            ("W03", "4'd15 + 4'd1", "4'd0"),
            ("W11", "P4 + 4'd1", "4'd0"),
            ("W16", "~4'd0", "4'hF"),
            ("W19", "4'd8 << 1", "4'd0"),
            // Controls PRE already answered right: the same pairs without the sign split.
            ("S05", "-1", "-1"),
            ("S08", "4'sb1111", "-1"),
            ("S09", "4'b1111", "-1"),
            ("S10", "4'sb1111", "8'd255"),
            ("S12", "8'sb11111111", "4'sb1111"),
            ("S14", "64'hFFFF_FFFF_FFFF_FFFF", "-64'sd1"),
            ("S22", "4'sb1000", "8"),
        ],
    );
    check(
        &src,
        &[
            "S06 item",
            "S07 item",
            "S11 item",
            "S16 item",
            "S21 item",
            "S23 item",
            "S29 item",
            "S32 item",
            "S36 item",
            "S37 item",
            "L24 item",
            "E05 item",
            "E06 item",
            "R04 item",
            "R06 item",
            "R07 item",
            "W03 item",
            "W11 item",
            "W16 item",
            "W19 item",
            "S05 item",
            "S08 item",
            "S09 default",
            "S10 default",
            "S12 item",
            "S14 item",
            "S22 default",
        ],
    );
}

/// The sign of an OVERRIDDEN parameter is its final value's (a typed parameter: the
/// declaration's). iverilog, sv2v → iverilog and verilator: `O01 item`, `O02 item`,
/// `O21 item`, `N11 item 0`, `N11 default 1`. PRE: `default` for all four.
#[test]
fn an_overridden_or_derived_parameter_compares_with_its_own_sign() {
    check(
        "`timescale 1ns/1ns\n\
         module s1 #(parameter int P = 0); case (P) 32'hFFFFFFFF: begin : a initial \
         $display(\"O01 item\"); end default: begin : d initial $display(\"O01 default\"); end \
         endcase endmodule\n\
         module s2 #(parameter logic signed [3:0] P = 0); case (P) 4'b1111: begin : a initial \
         $display(\"O02 item\"); end default: begin : d initial $display(\"O02 default\"); end \
         endcase endmodule\n\
         module s3 #(parameter P = 4'd15); case (4'b1111) P: begin : a initial \
         $display(\"O21 item\"); end default: begin : d initial $display(\"O21 default\"); end \
         endcase endmodule\n\
         module t;\n  s1 #(.P(-1)) u1();\n  s2 #(.P(-1)) u2();\n  s3 #(.P(4'sb1111)) u3();\n  \
         for (genvar i = 0; i < 2; i++) begin : g\n    localparam logic signed [3:0] Q = i - 1;\n    \
         case (Q)\n      4'b1111: begin : c initial $display(\"N11 item %0d\", i); end\n      \
         default: begin : d initial $display(\"N11 default %0d\", i); end\n    endcase\n  end\n\
         endmodule\n",
        &[
            "O01 item",
            "O02 item",
            "O21 item",
            "N11 item 0",
            "N11 default 1",
        ],
    );
}

/// ORACLE SPLIT, ruled: a genvar is a signed 32-bit integer (IEEE 1800 §27.4, vita's genvar
/// policy), so `i - 1` at i = 0 is `-1` and matches `32'hFFFFFFFF` (§12.5: unsigned at 32
/// bits). verilator takes the item in both positions: `N09 item 0`, `N09p item 0`.
/// iverilog 13.0 prints `N09 default 0` for the generate-case, but it is not an oracle
/// here: in the same block its procedural `case (i - 1)` takes the item (`N09p item 0`),
/// its `(i - 1) === 32'hFFFFFFFF` is 1, and its `$bits(i - 1)` is 3. sv2v → iverilog,
/// which rewrites the genvar, defaults in both positions.
/// PRE disagreed with itself the same way (`N09p item 0`, `N09 default 0`); both positions
/// now agree. A change aligning the generate-case to iverilog fails here.
#[test]
fn a_genvar_difference_compares_as_a_signed_integer() {
    check(
        "`timescale 1ns/1ns\nmodule t;\n  for (genvar i = 0; i < 2; i++) begin : g\n    \
         case (i - 1)\n      32'hFFFFFFFF: begin : c initial $display(\"N09 item %0d\", i); end\n      \
         default: begin : d initial $display(\"N09 default %0d\", i); end\n    endcase\n    \
         initial case (i - 1) 32'hFFFFFFFF: $display(\"N09p item %0d\", i); \
         default: $display(\"N09p default %0d\", i); endcase\n  end\nendmodule\n",
        &["N09 item 0", "N09 default 1", "N09p item 0", "N09p default 1"],
    );
}

/// An x-VALUED label never matches a known case expression (case equality compares x
/// as x), so the next label or `default` is taken — PRE's skip, now a decision.
/// iverilog and sv2v → iverilog print exactly these lines; verilator refuses x/z labels.
#[test]
fn an_x_valued_label_is_a_non_match() {
    check(
        &cases(
            "",
            &[
                ("X01", "4'b1100", "4'b1x00"),
                ("X03", "1", "4'bz"),
                ("X04", "4", "4'b1z00"),
                ("X06", "1", "4'b1x00 == 4'b1100"),
                ("C3", "1", "4'bx"),
                ("X16", "0", "|4'b000x"),
                // Known answers the 4-state fold gives exactly.
                ("X07", "0", "4'b1x00 == 4'b0000"),
                ("X08", "1", "4'b1x00 === 4'b1x00"),
                ("X10", "1", "|4'b100x"),
                ("X02", "1", "1'bx, 1"),
                ("X17", "1", "{64'd0, 1'bx}, 1"),
                ("X18", "1", "65'bx, 1"),
            ],
        ),
        &[
            "X01 default",
            "X03 default",
            "X04 default",
            "X06 default",
            "C3 default",
            "X16 default",
            "X07 item",
            "X08 item",
            "X10 item",
            "X02 item",
            "X17 item",
            "X18 item",
        ],
    );
}

/// The same rule where the case has no bit-domain width (a constant-function case
/// expression): the x-valued label is skipped and `3` is taken. iverilog and sv2v →
/// iverilog: `S47 item`; verilator refuses the x label; PRE `S47 item`.
#[test]
fn an_x_valued_label_beside_a_call_case_expression_is_a_non_match() {
    check(
        "`timescale 1ns/1ns\nmodule t;\n  \
         function automatic integer f(input integer x); f = x + 1; endfunction\n  \
         case (f(2)) {64'd0, 1'bx}: begin : x initial $display(\"S47 x\"); end \
         3: begin : a initial $display(\"S47 item\"); end default: begin : d initial \
         $display(\"S47 default\"); end endcase\nendmodule\n",
        &["S47 item"],
    );
}

/// Where the two sizings disagree a label keeps PRE's i64 answer: the oracles split, so
/// neither answer is adopted over the other.
/// - W01 `case (4'd15 + 4'd1) 0: … 16: …`: verilator `16` (b), iverilog `0` (a); kept b.
/// - W04 `case (4'sb1111) -1: … 8'd0: …`: verilator default, iverilog a; kept a.
/// - W08 `case (5'd16) 4'd15 + 4'd1`: verilator item, iverilog default; kept item.
/// - W09 `case (8'd0) 4'd15 + 4'd1`: verilator default, iverilog item; kept default.
/// - F01 `case (4'b1111) '1`: verilator item, iverilog default; kept default.
/// - S13 `case (64'hFFFF_FFFF_FFFF_FFFF) -1` and S15 (the pair reversed): verilator item
///   (`-1` negates at 64 bits), iverilog default; kept item.
/// - S17 `case (33'h1FFFFFFFF) -1`: verilator item, iverilog default; kept default.
/// - S27 `case (-1) 33'h0FFFFFFFF`: verilator default, iverilog item; kept default.
/// - O15 `parameter P = 4'sb1111` overridden by `4'd15`, `case (P) -1`: verilator item,
///   iverilog default; kept default.
///
/// A change that aligns the arm choice to either tool fails at least one of these.
#[test]
fn a_split_sizing_keeps_the_i64_answer() {
    check(
        "`timescale 1ns/1ns\nmodule t;\n  \
         case (4'd15 + 4'd1) 0: begin : a1 initial $display(\"W01 a\"); end \
         16: begin : b1 initial $display(\"W01 b\"); end default: begin : d1 initial \
         $display(\"W01 default\"); end endcase\n  \
         case (4'sb1111) -1: begin : a2 initial $display(\"W04 a\"); end \
         8'd0: begin : b2 initial $display(\"W04 b\"); end default: begin : d2 initial \
         $display(\"W04 default\"); end endcase\n\
         endmodule\n",
        &["W01 b", "W04 a"],
    );
    check(
        &cases(
            "",
            &[
                ("W08", "5'd16", "4'd15 + 4'd1"),
                ("W09", "8'd0", "4'd15 + 4'd1"),
                ("F01", "4'b1111", "'1"),
                ("S13", "64'hFFFF_FFFF_FFFF_FFFF", "-1"),
                ("S15", "-1", "64'hFFFF_FFFF_FFFF_FFFF"),
                ("S17", "33'h1FFFFFFFF", "-1"),
                ("S27", "-1", "33'h0FFFFFFFF"),
            ],
        ),
        &[
            "W08 item",
            "W09 default",
            "F01 default",
            "S13 item",
            "S15 item",
            "S17 default",
            "S27 default",
        ],
    );
    check(
        "`timescale 1ns/1ns\nmodule s #(parameter P = 4'sb1111); case (P) -1: begin : a initial \
         $display(\"O15 item\"); end default: begin : d initial $display(\"O15 default\"); end \
         endcase endmodule\nmodule t; s #(.P(4'd15)) u(); endmodule\n",
        &["O15 default"],
    );
}

/// RESIDUE: a label the bit domain cannot fold, or a case it cannot size, keeps exactly
/// PRE's answer — its i64 value, else a non-match — rather than going loud (ROADMAP §2 🆕 T:
/// close each class by making the shape foldable). Each class is pinned with the cell PRE
/// gets wrong and its twin whose true value differs from the case expression, which PRE
/// gets right by the same skip:
/// - 4-state `&` / `?:` with x (no 4-state arm in the bit domain): X05 `4'b1x00 & 4'b0011`
///   against 0 and X20 `1'bx ? 1 : 1` against 1 — iverilog and sv2v `item` (verilator
///   refuses x), vita `default`; twins X05T against 1 and X20T against 0: `default` in all.
/// - a real label: L13 `R` (`localparam real R = 1.0`) against 1 — verilator `item`,
///   iverilog and sv2v refuse the design, vita `default`; twin L13T against 2: verilator
///   `default`.
/// - a string parameter as an arithmetic leaf: L07 `S1 + 1` (`S1 = "ab"`) against
///   `16'h6163` — all three `item`, vita `default`; twin L07T against `16'h6164`: all three
///   `default`.
/// - a hierarchical name: L14 `t.P` (`P = 3`) against 3 — sv2v and verilator `item`,
///   iverilog refuses, vita `default`; twin L14T against 4: sv2v and verilator `default`
///   (census L14 / L14T, one cell per design: iverilog refuses this combined design for
///   its real and hierarchical labels).
/// - a case expression with no bit-domain width (a constant-function call): S48, a 65-bit
///   label beside `f(2)` — all three `default`, vita `default`.
/// - an x-left `==?` (no constant `==?` fold yet, §2 🆕 S (a)): T2
///   `$isunknown(4'bx100 ==? 4'b1?00)` against 1 — iverilog and sv2v `item`, vita `default`.
#[test]
fn a_label_the_bit_domain_cannot_read_keeps_the_pre_answer() {
    check(
        &cases(
            "  localparam real R = 1.0;\n  localparam S1 = \"ab\";\n  localparam P = 3;\n  \
             localparam [64:0] W = {1'b1, 64'd0};\n  \
             function automatic integer f(input integer x); f = x + 1; endfunction",
            &[
                ("X05", "0", "4'b1x00 & 4'b0011"),
                ("X05T", "1", "4'b1x00 & 4'b0011"),
                ("X20", "1", "1'bx ? 1 : 1"),
                ("X20T", "0", "1'bx ? 1 : 1"),
                ("L13", "1", "R"),
                ("L13T", "2", "R"),
                ("L07", "16'h6163", "S1 + 1"),
                ("L07T", "16'h6164", "S1 + 1"),
                ("L14", "3", "t.P"),
                ("L14T", "4", "t.P"),
                ("S48", "f(2)", "W"),
                ("T2", "1", "$isunknown(4'bx100 ==? 4'b1?00)"),
            ],
        ),
        &[
            "X05 default",
            "X05T default",
            "X20 default",
            "X20T default",
            "L13 default",
            "L13T default",
            "L07 default",
            "L07T default",
            "L14 default",
            "L14T default",
            "S48 default",
            "T2 default",
        ],
    );
}

/// The scan stops at the first match, so a label after it is never read, and a label the
/// i64 fold reads keeps its answer beside one only the bit domain reads. iverilog and
/// sv2v → iverilog: `M05 a`, `M01 a`, `M02 b`, `M03 a` (verilator refuses M05's x literal;
/// it prints `M01 a`, `M02 b`, `M03 a` for census M01–M03). PRE the same.
#[test]
fn the_first_matching_label_wins_before_any_refusal() {
    check(
        "`timescale 1ns/1ns\nmodule t;\n  \
         case (1) 1: begin : a5 initial $display(\"M05 a\"); end \
         $isunknown(4'bx100 ==? 4'b1?00): begin : b5 initial $display(\"M05 b\"); end endcase\n  \
         case (2) 1, 2: begin : a1 initial $display(\"M01 a\"); end \
         2: begin : b1 initial $display(\"M01 b\"); end endcase\n  \
         case (3) 1: begin : a2 initial $display(\"M02 a\"); end \
         3: begin : b2 initial $display(\"M02 b\"); end 3: begin : c2 initial \
         $display(\"M02 c\"); end endcase\n  \
         case (1) default: begin : d3 initial $display(\"M03 def\"); end \
         1: begin : a3 initial $display(\"M03 a\"); end endcase\n\
         endmodule\n",
        &["M05 a", "M01 a", "M02 b", "M03 a"],
    );
}

/// Nested scopes: a genvar case expression, a generate-block localparam shadowing a
/// module one of another width, a case inside a case. All three oracles: `N01 c0 0`,
/// `N01 c1 1`, `N01 cd 2`, `N03 a`, `N04 a`, `N05 default`, `N06 a`, `N07 default 0`,
/// `N07 a 1`, `N07 default 2` (PRE the same).
#[test]
fn nested_and_shadowed_scopes_compare_their_own_names() {
    check(
        "`timescale 1ns/1ns\nmodule t;\n  localparam [64:0] L = 65'd5;\n  \
         localparam logic [3:0] K = 4'd5;\n  \
         for (genvar i = 0; i < 3; i++) begin : g1\n    case (i)\n      \
         0: begin : c0 initial $display(\"N01 c0 %0d\", i); end\n      \
         65'd1: begin : c1 initial $display(\"N01 c1 %0d\", i); end\n      \
         default: begin : cd initial $display(\"N01 cd %0d\", i); end\n    endcase\n  end\n  \
         if (1) begin : b3\n    localparam [64:0] L = 65'd7;\n    case (7) L: begin : a initial \
         $display(\"N03 a\"); end default: begin : d initial $display(\"N03 default\"); end \
         endcase\n  end\n  \
         if (1) begin : b4\n    localparam logic [3:0] L = 4'd7;\n    case (7) L: begin : a \
         initial $display(\"N04 a\"); end default: begin : d initial $display(\"N04 default\"); \
         end endcase\n  end\n  \
         if (1) begin : b5\n    localparam [64:0] K = 65'd7;\n    case (5) K: begin : a \
         initial $display(\"N05 a\"); end default: begin : d initial $display(\"N05 default\"); \
         end endcase\n  end\n  \
         case (1) 1: begin : o case (2) 65'd2: begin : a initial $display(\"N06 a\"); end \
         default: begin : d initial $display(\"N06 default\"); end endcase end endcase\n  \
         for (genvar i = 0; i < 3; i++) begin : g7\n    localparam [64:0] LG = i;\n    \
         case (1) LG: begin : a initial $display(\"N07 a %0d\", i); end default: begin : d \
         initial $display(\"N07 default %0d\", i); end endcase\n  end\n\
         endmodule\n",
        &[
            "N01 c0 0",
            "N01 c1 1",
            "N01 cd 2",
            "N03 a",
            "N04 a",
            "N05 default",
            "N06 a",
            "N07 default 0",
            "N07 a 1",
            "N07 default 2",
        ],
    );
}
