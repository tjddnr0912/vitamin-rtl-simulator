//! §3 ⑤ⓙ: a string literal in a generate-if or generate-for CONDITION is its §5.9 value —
//! an unsigned constant of eight bits per character, the first character most
//! significant (IEEE 1800-2017 §5.9). It was E3010 (`generate-if condition is not a
//! constant: a string literal has no integral constant value`), which stopped the corpus
//! row `ibex` twice: `ibex_counter` declares `localparam int UseDsp = "no";` and picks its
//! flop with `if (UseDsp == "yes")` (reported for `mcycle_counter_i` and
//! `minstret_counter_i`).
//!
//! The parameter binder already read a literal that way; a condition now does too,
//! because §11.6.1 makes it self-determined, so nothing around it can make the literal a
//! `string`. A name in such a condition is read only when the module's source leaves
//! one object it can be — declared once, at the module's top level, as a parameter with
//! a written integral type, or only as a genvar, or not at all (a package's) — because
//! elaborate binds a generate block's names by position and phase, and three shapes
//! read an outer object that way (the last tests). A `string` parameter keeps its
//! refusal in an ordering (`S < "ab"`: verilator compares text, sv2v → iverilog
//! numbers), and so does a literal holding an escape IEEE 1800-2017 Table 5-1 does not
//! define (`"\r"` is 0x0D in verilator and the letter in iverilog).
//!
//! Oracles: verilator 5.052 (`--binary --timing`), iverilog 13.0 (`-g2012`) and sv2v
//! 0.0.13 → iverilog. sv2v mis-converts escape and NUL cells, so there the two direct
//! tools are the oracle.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_slc_{}_{n}", std::process::id()));
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

/// The design's own lines, without vita's warnings and end-of-run lines.
fn prints(src: &str, want: &[&str]) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    let got: Vec<&str> = out
        .lines()
        .filter(|l| {
            !l.starts_with("warning[")
                && !l.starts_with("simulation ended")
                && !l.starts_with("errors=")
        })
        .collect();
    assert_eq!(got, want, "{out}");
}

fn is_loud(src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(1), "expected a refusal\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in\n{out}");
}

/// A module whose generate-ifs each print `G<k> 1` or `G<k> 0`.
fn conds(decls: &str, cs: &[&str]) -> String {
    let mut s = format!("module t;\n{decls}\n");
    for (k, c) in cs.iter().enumerate() {
        s += &format!(
            "  if ({c}) begin : g{k} initial $display(\"G{k} 1\"); end \
             else begin : h{k} initial $display(\"G{k} 0\"); end\n"
        );
    }
    s + "  initial #1 $finish;\nendmodule\n"
}

// ───────────────────────────── values ─────────────────────────────

/// `ibex_counter`'s DSP switch, reduced: the branches differ in reset style, and the
/// counter's initial value shows which one was built. All three oracles print both
/// variants exactly as pinned.
#[test]
fn the_ibex_counter_dsp_switch_picks_its_branch() {
    let src = |v: &str| {
        format!(
            r#"
module ibex_counter_r #(parameter int CounterWidth = 8) (
  input  logic clk_i, input logic rst_ni, input logic inc_i,
  output logic [CounterWidth-1:0] q_o
);
  localparam int UseDsp = "{v}";
  logic [CounterWidth-1:0] counter_q = 8'h55;
  if (UseDsp == "yes") begin : g_cnt_dsp
    always_ff @(posedge clk_i) begin
      if (!rst_ni) counter_q <= '0;
      else if (inc_i) counter_q <= counter_q + 1'b1;
    end
  end else begin : g_cnt_no_dsp
    always_ff @(posedge clk_i or negedge rst_ni) begin
      if (!rst_ni) counter_q <= '0;
      else if (inc_i) counter_q <= counter_q + 1'b1;
    end
  end
  assign q_o = counter_q;
endmodule

module t;
  logic clk = 0, rst_n = 1, inc = 1;
  logic [7:0] q;
  ibex_counter_r u (.clk_i(clk), .rst_ni(rst_n), .inc_i(inc), .q_o(q));
  initial begin
    $display("U %h", u.UseDsp);
    #1 rst_n = 0;
    #1 $display("A %h", q);
    #1 clk = 1; #1 clk = 0; rst_n = 1;
    #1 clk = 1; #1 clk = 0; #1 clk = 1; #1 clk = 0;
    #1 $display("B %h", q);
    $finish;
  end
endmodule
"#
        )
    };
    // "no": the asynchronous reset clears the counter at the falling edge.
    prints(&src("no"), &["U 00006e6f", "A 00", "B 02"]);
    // "yes": the synchronous one waits for the clock, so `55` survives the edge.
    prints(&src("yes"), &["U 00796573", "A 55", "B 02"]);
}

/// Equality and ordering against a parameter of each integral shape: a signed `int`, a
/// vector the literal fits, one it is truncated into (`"no"` → `"o"`) and one it is
/// padded into, a 2-state vector, and an untyped parameter (a 16-bit value, `$bits` 16
/// in all three oracles).
#[test]
fn a_string_literal_in_a_condition_is_its_bytes() {
    let decls = r#"
  localparam int PI = "no";
  localparam int Y = "yes";
  localparam logic [15:0] P16 = "no";
  localparam logic [7:0] P8 = "no";
  localparam logic [31:0] P32 = "no";
  localparam bit [23:0] PB = "yes";
  localparam PU = "no";"#;
    prints(
        &conds(
            decls,
            &[
                r#"PI == "yes""#,
                r#"Y == "yes""#,
                r#"PI != "yes""#,
                r#""no" == PI"#,
                r#"PI < "yes""#,
                r#"P16 == "no""#,
                r#"P8 == "no""#,
                r#"P8 == "o""#,
                r#"P32 == "no""#,
                r#"PB == "yes""#,
                r#"PU != "yes""#,
                r#"PI === "no""#,
            ],
        ),
        &[
            "G0 0", "G1 1", "G2 1", "G3 1", "G4 1", "G5 1", "G6 0", "G7 1", "G8 1", "G9 1",
            "G10 1", "G11 1",
        ],
    );
}

/// The literal is an operand like a sized one: it is unsigned, it takes the context of
/// the operator around it, an empty literal is one NUL byte, and escapes are bytes.
#[test]
fn a_string_literal_operand_follows_the_operator_rules() {
    prints(
        &conds(
            "  localparam byte PY = \"\\377\";",
            &[
                r#""b" < "ab""#,
                r#""ab" == 16'h6162"#,
                r#""a" + 1 == 8'h62"#,
                r#""a" + 8'hFF == 9'h160"#,
                r#"-"a" == 32'hFFFFFF9F"#,
                r#""""#,
                r#"!"""#,
                r#""a" && 1"#,
                r#""\n" == 10"#,
                r#"PY == "\377""#,
                r#"PY < "\001""#,
                r#"{"a", "b"} == "ab""#,
                r#"{2{"a"}} == "aa""#,
                r#"&"\377""#,
                r#""\377" == -1"#,
            ],
        ),
        &[
            "G0 1", "G1 1", "G2 1", "G3 1", "G4 1", "G5 0", "G6 1", "G7 1", "G8 1", "G9 1",
            "G10 0", "G11 1", "G12 1", "G13 1", "G14 0",
        ],
    );
}

/// A literal as a replication count, a bit index, an indexed width, a `$clog2` argument
/// and a ternary condition, and literals past 64 bits (`"abcdefghi"` is 72 bits).
#[test]
fn counts_indices_and_wide_literals_inside_a_condition() {
    prints(
        &conds(
            "  localparam int P = 3;\n  localparam logic [71:0] PW = \"abcdefghi\";",
            &[
                r#"{"\002"{1'b1}} == 2'b11"#,
                r#"P["\001"]"#,
                r#"P[1 +: "\002"] == 2'b01"#,
                r#"$clog2("\020") == 4"#,
                r#"P == ("a" ? 3 : 4)"#,
                r#"PW == "abcdefghi""#,
                r#"PW > "abcdefghh""#,
                r#"P == "abcdefghij""#,
            ],
        ),
        &[
            "G0 1", "G1 1", "G2 1", "G3 1", "G4 1", "G5 1", "G6 1", "G7 0",
        ],
    );
}

const ONCE: &str = "a condition that holds a string literal reads only a parameter declared \
                    once in the module, outside every `generate` region and block, as an `int`, \
                    an `integer` or a type with a packed range; it does not read";

/// A `string` parameter has no integral binding, and its ordering is an oracle split:
/// `"b" < "ab"` as text is 0 (verilator), as numbers 1 (sv2v → iverilog). An untyped
/// one too: all three oracles read `PU` as the 16-bit `6e6f` and print `G0 1`, and vita
/// records no width for it (ROADMAP `string-literal-condition-residue`).
#[test]
fn a_string_parameter_in_an_ordering_stays_refused() {
    is_loud(
        &conds("  parameter string S = \"b\";", &[r#"S < "ab""#]),
        &format!("generate-if condition is not a constant: {ONCE} `S`"),
    );
    is_loud(
        &conds("  localparam PU = \"no\";", &[r#""b" < PU"#]),
        &format!("{ONCE} `PU`"),
    );
}

/// Elaborate binds a generate block's names by POSITION and once per phase, and its name
/// walk sees through an instance-array element and ignores a block's `import` and `let`,
/// so a condition inside a block reads an outer object in shapes the oracles scope
/// apart. Such a condition keeps its old refusal. A block's untyped string `localparam X`
/// above the condition: all three oracles read the inner `62` (`G0 1`), the outer `int X`
/// would answer `6e6f`.
#[test]
fn a_condition_inside_a_generate_block_stays_refused() {
    const NO_VALUE: &str =
        "generate-if condition is not a constant: a string literal has no integral constant value";
    is_loud(
        r#"
module t;
  localparam int X = "no";
  if (1) begin : blk
    localparam X = "b";
    if (X < "ab") begin : g0 initial $display("G0 1"); end
    else begin : h0 initial $display("G0 0"); end
  end
endmodule
"#,
        NO_VALUE,
    );
    // Declared AFTER the condition: unbound in the first generate phase and bound in
    // the later ones. verilator and sv2v → iverilog build the then branch (`G1 then 11`),
    // iverilog the else (`G1 else 22`) — every tool builds one.
    is_loud(
        r#"
module t;
  localparam int X = 100;
  if (1) begin : b
    if (X < "c") begin : h1 initial $display("G1 then 11"); end
    else begin : h2 initial $display("G1 else 22"); end
    localparam X = "b";
  end
endmodule
"#,
        NO_VALUE,
    );
    // A generate `typedef enum` label no map binds (`gen_enum.rs`): verilator and sv2v
    // → iverilog read the label `97` (`G1 then`); the outer localparam is `98`.
    is_loud(
        r#"
module t;
  localparam int LBL = 98;
  localparam int W = 97;
  if (1) begin : b
    typedef enum int {LBL = W, OTHER} e_t;
    if (LBL == "a") begin : g1 initial $display("G1 then"); end
    else begin : h1 initial $display("G1 else"); end
  end
endmodule
"#,
        NO_VALUE,
    );
    // A block's own import of a package the module already imports: all three oracles
    // read `p::X = 97` (`G1 then`); the module's `localparam int X = 98` is outside.
    is_loud(
        r#"
package p; localparam int X = 97; endpackage
module t;
  import p::*;
  localparam int X = 98;
  if (1) begin : b
    import p::X;
    if (X == "a") begin : g1 initial $display("G1 then"); end
    else begin : h1 initial $display("G1 else"); end
  end
endmodule
"#,
        "generate-if condition is not a constant: a string literal has no integral constant \
         value",
    );
    // A generate-for inside a loop body reading the OUTER loop's variable.
    is_loud(
        r#"
module t;
  for (genvar i = 1; i < 2; i++) begin : l
    for (genvar j = 0; j < i + "\000"; j++) begin : m
      initial $display("M %0d", j);
    end
  end
endmodule
"#,
        "generate-for condition is not a constant: a string literal has no integral constant \
         value",
    );
}

/// A generate-for condition never reads a string literal: the genvar setup leaves a
/// same-named WIDE constant (≥ 2^64) where the name walk asks first, so the loop below
/// would run zero times. All three oracles print `D1 0`, `D1 1` for both loops.
#[test]
fn a_generate_for_condition_keeps_its_refusal() {
    const NO_VALUE: &str =
        "generate-for condition is not a constant: a string literal has no integral constant value";
    is_loud(
        r#"
module t;
  for (genvar i = 0; i < "\003"; i++) begin : g
    initial $display("I %0d", i);
  end
endmodule
"#,
        NO_VALUE,
    );
    is_loud(
        r#"
localparam logic [127:0] U = 128'h1_0000_0000_0000_0000;
module t;
  for (genvar U = 0; U < "\002"; U++) begin : l initial $display("D1 %0d", U); end
endmodule
"#,
        NO_VALUE,
    );
}

/// At the module's top level a name is read only when the module declares it once, at
/// its top level, with a written integral type.
#[test]
fn a_top_level_name_the_census_refuses_stays_refused() {
    // Declared twice — a block does not enclose the condition: all three oracles
    // print `G6 then`, and it stays loud.
    is_loud(
        r#"
module t;
  localparam int X = "no";
  if (1) begin : blk2
    localparam int X = "ab";
  end
  if (X == "no") begin : g6 initial $display("G6 then"); end
  else begin : h6 initial $display("G6 else"); end
endmodule
"#,
        &format!("{ONCE} `X`"),
    );
    // Not declared by the module: an instance-array element's name walk reaches its
    // parent's `X`. verilator: "Can't find definition of variable: 'X'"; iverilog:
    // "Unable to bind parameter `X' in `t.u[0]'".
    is_loud(
        r#"
module m;
  if (X == "\005") begin : g1 initial #1 $display("G1 then %m"); end
  else begin : h1 initial #1 $display("G1 else %m"); end
endmodule
module t;
  localparam int X = 5;
  m u[1:0] ();
endmodule
"#,
        &format!("{ONCE} `X`"),
    );
    // Imported: the package's string `X` answers in all three oracles (`G1 then`
    // twice); the parent's `int X` is what the walk would reach.
    is_loud(
        r#"
package p; localparam X = "a"; endpackage
module m;
  import p::*;
  if (X == "a") begin : g1 initial #1 $display("G1 then %m"); end
  else begin : h1 initial #1 $display("G1 else %m"); end
endmodule
module t;
  localparam int X = 5;
  m u[1:0] ();
endmodule
"#,
        &format!("{ONCE} `X`"),
    );
}

/// `\r` is not in Table 5-1: verilator reads 0x0D (then), iverilog the letter `r`
/// (else). The condition declines the literal and says why.
#[test]
fn a_nonstandard_escape_in_a_condition_stays_refused() {
    is_loud(
        &conds("", &[r#""\r" == 13"#]),
        "generate-if condition is not a constant: `\\r` is not a string escape in IEEE \
         1800-2017 Table 5-1, so the literal's value differs between tools",
    );
}

/// `==?` has no arm in the wide domain, and the constant domain's wildcard compare reads
/// its left side as an i64, which has no literal: all three oracles print `G0 1`.
#[test]
fn a_wildcard_compare_of_a_string_literal_stays_refused() {
    is_loud(
        &conds("", &[r#""a" ==? 8'b0110_000x"#]),
        "generate-if condition is not a constant: a wildcard equality (`==?`, `!=?`) has \
         no constant-fold arm for a string-literal operand",
    );
}

/// A constant function call folds only in the integer domain, and a condition holding a
/// string literal is read in the wide one: `f(1)` is 97, and all three oracles print
/// `G1 then`.
#[test]
fn a_constant_call_beside_a_string_literal_stays_refused() {
    is_loud(
        r#"
module t;
  function automatic int f(int a); return a + 96; endfunction
  if (f(1) == "a") begin : g1 initial $display("G1 then"); end
  else begin : h1 initial $display("G1 else"); end
endmodule
"#,
        "generate-if condition is not a constant: `f(…)` has no constant-fold arm in a \
         condition that holds a string literal",
    );
}

/// A generate-case SCRUTINEE is not a condition and still reads no literal. All three
/// oracles take the `P` arm (`G3 P`).
#[test]
fn a_string_literal_case_scrutinee_stays_refused() {
    is_loud(
        r#"
module t;
  localparam int P = "no";
  case ("no")
    P: begin : g_g initial $display("G3 P"); end
    default: begin : g_h initial $display("G3 def"); end
  endcase
endmodule
"#,
        "generate-case scrutinee is not a constant: a string literal has no integral \
         constant value",
    );
}
