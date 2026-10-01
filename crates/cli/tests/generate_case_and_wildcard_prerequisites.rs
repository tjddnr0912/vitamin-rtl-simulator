//! Held cells of §4.5.581, which attempted ROADMAP §2 🆕 T (a generate-case label compared
//! in the 4-state domain at full width) and 🆕 S (a) (the constant `==?` / `inside` at the
//! comparison's common width) and reverted both after three review rounds. Each design
//! here is pinned at the output vita gives today, with every oracle's raw line beside the
//! assertion, so a re-attempt changes an expected value and moves a comment rather than
//! re-grounding a cell.
//!
//! - The KNOWN-WRONG cells are the witnesses of the prerequisites the reviews found:
//!   P1 one current binding per key, P2 a generate-case arm chosen identically in every
//!   elaboration phase, P3 the sign of a constant typed by an overridden type parameter,
//!   and the 🆕 T label cells those three hold back.
//! - The PRE-RIGHT cells guard the regressions the attempt's reviews found (a stale wide
//!   entry read first, the opposite choice of entry, a width the i64 routine declined, a
//!   parameter-count replication as the left operand); a re-attempt must keep them.
//!
//! Oracles, re-run for this file on each design as written: iverilog 13.0 (`-g2012`,
//! `vvp -n`), sv2v 0.0.13 → iverilog 13.0, verilator 5.052 (`--binary --timing`; it
//! refuses x/z/? generate-case labels). Each test says which ran and what each printed.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Run {
    out: String,
    err: String,
    code: i32,
}

impl Run {
    /// The `$display` lines, sorted (several processes print at the same time; their
    /// order is not what these pins measure), without the simulator's trailer.
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
    let d = std::env::temp_dir().join(format!("vita_gcwp_{}_{n}", std::process::id()));
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

fn check(src: &str, want: &[&str]) {
    let r = run(src);
    assert_eq!(r.code, 0, "stderr:\n{}\nsrc:\n{src}", r.err);
    let mut want = want.to_vec();
    want.sort_unstable();
    assert_eq!(r.lines(), want, "stdout:\n{}\nsrc:\n{src}", r.out);
}

// ── 🆕 T: a generate-case label is compared as an i64 value (KNOWN-WRONG) ─────────

/// KNOWN-WRONG. A label only the 4-state fold reads (T1, a 65-bit parameter of value 1;
/// L06, `"a" + 1` over `P = 8'h62`) is skipped as a non-match, and a mixed-sign pair
/// (S06, `-1` against `32'hFFFFFFFF`, unsigned at 32 bits by §12.5) compares as two
/// integers.
/// - iverilog: `L06 item`, `S06 item`, `T1 item`.
/// - sv2v → iverilog: `L06 item`, `S06 item`, `T1 item`.
/// - verilator: `L06 item`, `S06 item`, `T1 item`.
#[test]
fn t_labels_compare_as_i64_values() {
    check(
        "`timescale 1ns/1ns\nmodule t;\n  localparam P1 = (4'b1100 ==? 4'b1?00);\n  \
         localparam [64:0] LPA = {64'd0, P1};\n  localparam logic [7:0] P = 8'h62;\n  \
         case (1) LPA: begin : i_T1 initial $display(\"T1 item\"); end default: begin : d_T1 \
         initial $display(\"T1 default\"); end endcase\n  \
         case (P) \"a\" + 1: begin : i_L06 initial $display(\"L06 item\"); end default: begin : \
         d_L06 initial $display(\"L06 default\"); end endcase\n  \
         case (-1) 32'hFFFFFFFF: begin : i_S06 initial $display(\"S06 item\"); end default: \
         begin : d_S06 initial $display(\"S06 default\"); end endcase\nendmodule\n",
        &["L06 default", "S06 default", "T1 default"],
    );
}

/// KNOWN-WRONG. A constant `inside` element with x/z bits is compared with `==` by the
/// constant domains (x), so the label is skipped; its `==?` twin X11 folds.
/// - iverilog: refuses `inside` (`sorry: "inside" expressions not supported yet`); its
///   own `==?` twin prints `X11 item`.
/// - sv2v → iverilog: `X11 item`, `X13 item`.
/// - verilator: refuses (`Use of x/? constant in generate case statement`).
#[test]
fn t_an_inside_label_with_a_wildcard_is_skipped() {
    check(
        "`timescale 1ns/1ns\nmodule t;\n  \
         case (1) 4'b1100 inside {4'b1?00}: begin : i_X13 initial $display(\"X13 item\"); end \
         default: begin : d_X13 initial $display(\"X13 default\"); end endcase\n  \
         case (1) (4'b1100 ==? 4'b1?00): begin : i_X11 initial $display(\"X11 item\"); end \
         default: begin : d_X11 initial $display(\"X11 default\"); end endcase\nendmodule\n",
        &["X11 item", "X13 default"],
    );
}

/// KNOWN-WRONG, an oracle split ruled for the genvar policy (a genvar is a signed 32-bit
/// integer, IEEE 1800 §27.4): `i - 1` at i = 0 is `-1`, which matches `32'hFFFFFFFF`.
/// vita's generate-case (`N09 default 0`) and its own procedural case (`N09p item 0`)
/// disagree, and its run-time `===` agrees with the procedural case.
/// - iverilog: `N09 default 0`, `N09 default 1`, `N09p default 1`, `N09p item 0`,
///   `N09q i=0 eq=1 bits=3`, `N09q i=1 eq=0 bits=3` — the same self-contradiction, and
///   a 3-bit `$bits(i - 1)`: not an oracle here.
/// - sv2v → iverilog (it rewrites the genvar): `N09 default 0`, `N09 default 1`,
///   `N09p default 0`, `N09p default 1`, `N09q i=0 eq=0 bits=32`, `N09q i=1 eq=0 bits=32`.
/// - verilator: `N09 default 1`, `N09 item 0`, `N09p default 1`, `N09p item 0`,
///   `N09q i=0 eq=1 bits=32`, `N09q i=1 eq=0 bits=32`.
#[test]
fn t_a_genvar_difference_disagrees_with_the_procedural_case() {
    check(
        "`timescale 1ns/1ns\nmodule t;\n  for (genvar i = 0; i < 2; i++) begin : g\n    \
         case (i - 1)\n      32'hFFFFFFFF: begin : c initial $display(\"N09 item %0d\", i); end\n      \
         default: begin : d initial $display(\"N09 default %0d\", i); end\n    endcase\n    \
         initial case (i - 1) 32'hFFFFFFFF: $display(\"N09p item %0d\", i); \
         default: $display(\"N09p default %0d\", i); endcase\n    \
         initial #1 $display(\"N09q i=%0d eq=%b bits=%0d\", i, (i - 1) === 32'hFFFFFFFF, \
         $bits(i - 1));\n  end\nendmodule\n",
        &[
            "N09 default 0",
            "N09 default 1",
            "N09p default 1",
            "N09p item 0",
            "N09q i=0 eq=1 bits=32",
            "N09q i=1 eq=0 bits=32",
        ],
    );
}

// ── P1: one current binding per key ────────────────────────────────────────────────

/// A genvar under a same-named 65-bit localparam: the genvar rebinds the key narrow and
/// leaves the wide entry, which the run-time name route and the bit domain ask FIRST.
/// - A1D, KNOWN-WRONG: `$display(i)` and `localparam int K = i` read the wide entry.
/// - A1S, PRE-RIGHT (the round-1 regression guard): the `case (i)` arm is chosen by the
///   i64 lane, which reads the genvar; a bit-domain label lane took `default` twice.
/// - iverilog, sv2v → iverilog, verilator: `A1D i=0 K=0`, `A1D i=1 K=1`, `A1S one 2`,
///   `A1S zero 1`.
#[test]
fn p1_a_genvar_under_a_wide_constant() {
    check(
        "`timescale 1ns/1ns\nmodule t;\n  localparam [64:0] i = 65'h1_0000_0000_0000_0009;\n  \
         for (genvar i = 0; i < 2; i++) begin : g\n    case (i)\n      \
         0: begin : z wire [7:0] w = 8'd1; initial #1 $display(\"A1S zero %0d\", w); end\n      \
         1: begin : o wire [7:0] w = 8'd2; initial #1 $display(\"A1S one %0d\", w); end\n      \
         default: begin : d wire [7:0] w = 8'd99; initial #1 $display(\"A1S def %0d\", w); end\n    \
         endcase\n    localparam int K = i;\n    \
         initial #2 $display(\"A1D i=%0d K=%0d\", i, K);\n  end\nendmodule\n",
        &[
            "A1D i=18446744073709551625 K=9",
            "A1D i=18446744073709551625 K=9",
            "A1S one 2",
            "A1S zero 1",
        ],
    );
}

/// PRE-RIGHT, the round-2 regression guard: `import pa::*; import pb::W;` binds a narrow
/// `W` (5) beside the CURRENT wide `W` (2^64 + 7) at one key; skipping the wide entry
/// whenever a narrow one shares its key read 5.
/// - iverilog: `W=10000000000000007 D=10000000000000008 bits=65 hi=1`.
/// - sv2v → iverilog: `W=10000000000000007 D=10000000000000008 bits=65 hi=1`.
/// - verilator: `W=00000005 D=00000000000000006 bits=32 hi=1` (it binds the wildcard
///   import's `W` over the explicit one; the two other oracles and the LRM's explicit
///   import precedence agree with vita).
#[test]
fn p1_an_explicit_import_beside_a_wildcard_one() {
    check(
        "`timescale 1ns/1ns\npackage pa; localparam W = 5; endpackage\n\
         package pb; localparam [64:0] W = 65'h1_0000_0000_0000_0007; endpackage\n\
         module t;\n  import pa::*;\n  import pb::W;\n  localparam [64:0] D = W + 65'd1;\n  \
         initial begin\n    $display(\"W=%h D=%h bits=%0d hi=%b\", W, D, $bits(W), W[64]);\n    \
         #1 $finish;\n  end\nendmodule\n",
        &["W=10000000000000007 D=10000000000000008 bits=65 hi=1"],
    );
}

// ── P2: a generate-case arm chosen identically in every phase ──────────────────────

/// KNOWN-WRONG, a phase mix: label `K` refers forward to a generate-scope localparam
/// declared after the case, so the Nets walk (where `K` is not yet bound) builds
/// `default`'s 4-bit net and the Logic walk runs arm `k`'s process.
/// - iverilog: refuses (`Unable to bind parameter `K' in `t.gb'` … `Check for
///   declaration after use`).
/// - sv2v → iverilog: `D1P k 200 bits=8`.
/// - verilator: `D1P k 200 bits=8`.
#[test]
fn p2_a_forward_referenced_matching_label_mixes_arms() {
    check(
        "`timescale 1ns/1ns\nmodule t;\n  if (1) begin : gb\n    case (8'd99)\n      \
         8'd1: begin : g wire [7:0] w = 8'd1; initial #1 $display(\"D1P one %0d\", w); end\n      \
         K: begin : g wire [7:0] w = 8'd200; initial #1 \
         $display(\"D1P k %0d bits=%0d\", w, $bits(w)); end\n      \
         default: begin : g wire [3:0] w = 4'd9; initial #1 \
         $display(\"D1P def %0d bits=%0d\", w, $bits(w)); end\n    endcase\n    \
         localparam logic [7:0] K = 8'd99;\n  end\nendmodule\n",
        &["D1P k 8 bits=4"],
    );
}

/// KNOWN-WRONG but phase-consistent: the forward label `K` is not the match; the first
/// label (`32'hFFFFFFFF` against `-1`) is, and the i64 lane skips it, so every phase
/// takes `default`. A bit-domain region available only after the Nets walk re-decided
/// the first label and mixed arms (`D1W a 8 bits=4`, review round 1).
/// - iverilog, sv2v → iverilog, verilator: `D1W a 200 bits=8`.
#[test]
fn p2_a_forward_label_does_not_change_the_other_labels() {
    check(
        "`timescale 1ns/1ns\nmodule t;\n  if (1) begin : gb\n    case (-1)\n      \
         32'hFFFFFFFF: begin : g wire [7:0] w = 8'd200; initial #1 \
         $display(\"D1W a %0d bits=%0d\", w, $bits(w)); end\n      \
         K: begin : g wire [7:0] w = 8'd2; initial #1 $display(\"D1W k %0d\", w); end\n      \
         default: begin : g wire [3:0] w = 4'd9; initial #1 \
         $display(\"D1W def %0d bits=%0d\", w, $bits(w)); end\n    endcase\n    \
         localparam logic [7:0] K = 8'd99;\n  end\nendmodule\n",
        &["D1W def 9 bits=4"],
    );
}

// ── P3: the sign of a constant typed by an overridden type parameter ───────────────

/// KNOWN-WRONG: `parameter T PV` with `#(.T(logic signed [63:0]), .PV(-64'sd4))` reads
/// unsigned — `PV < 0` is 0.
/// - iverilog, sv2v → iverilog, verilator: `lt0=1 bits=64`.
#[test]
fn p3_a_parameter_typed_by_an_overridden_type_reads_unsigned() {
    check(
        "`timescale 1ns/1ns\nmodule sub #(parameter type T = logic [3:0], parameter T PV = '0);\n  \
         initial $display(\"lt0=%0d bits=%0d\", PV < 0, $bits(PV));\nendmodule\n\
         module t;\n  sub #(.T(logic signed [63:0]), .PV(-64'sd4)) u();\nendmodule\n",
        &["lt0=0 bits=64"],
    );
}

/// KNOWN-WRONG: `localparam T TP = '1;` under `#(.T(logic signed [3:0]))` is 15, not -1.
/// - iverilog, sv2v → iverilog, verilator: `TP=-1 lt0=1 bits=4`.
#[test]
fn p3_a_fill_typed_by_an_overridden_type_reads_unsigned() {
    check(
        "`timescale 1ns/1ns\nmodule m #(parameter type T = logic [3:0]) ();\n  \
         localparam T TP = '1;\n  \
         initial $display(\"TP=%0d lt0=%0d bits=%0d\", TP, TP < 0, $bits(TP));\nendmodule\n\
         module t;\n  m #(.T(logic signed [3:0])) u ();\nendmodule\n",
        &["TP=15 lt0=0 bits=4"],
    );
}

// ── 🆕 S (a): hazards the i64 common-width routine broke (held at today's output) ──

/// PRE-RIGHT, a round-3 regression guard: a parameter-count replication as the left
/// operand. The common-width routine needed the left operand's width and declined
/// (E3009) where today's own-width masked compare answers.
/// - iverilog, sv2v → iverilog, verilator: `R=1`.
#[test]
fn s_a_replication_left_operand_keeps_its_value() {
    check(
        "`timescale 1ns/1ns\nmodule t;\n  localparam int N2 = 2;\n  \
         localparam R = ({N2{4'b1100}} ==? 8'b1?00_1100);\n  initial $display(\"R=%0d\", R);\n\
         endmodule\n",
        &["R=1"],
    );
}

/// PRE-RIGHT, a round-2 regression guard: a left operand DECLARED wider than 64 bits
/// whose value fits; the common-width routine declined past 64 bits and the label took
/// `default`, the localparams and the generate-`if` went loud.
/// - iverilog: `L=1 LN=0`, `gcase item`, `gif then`.
/// - sv2v → iverilog: `L=1 LN=0`, `gcase item`, `gif then`.
/// - verilator: refuses (`Use of x/? constant in generate case statement`).
#[test]
fn s_a_wide_declaration_with_a_fitting_value_keeps_its_value() {
    check(
        "`timescale 1ns/1ns\nmodule t;\n  localparam logic [67:0] P68 = 68'hC;\n  \
         localparam L = (P68 ==? 4'b1?00);\n  localparam LN = (P68 !=? 4'b1?00);\n  \
         if (P68 ==? 4'b1?00) begin : g initial $display(\"gif then\"); end\n  \
         else begin : h initial $display(\"gif else\"); end\n  \
         case (1) (P68 ==? 4'b1?00): begin : c initial $display(\"gcase item\"); end\n  \
         default: begin : d initial $display(\"gcase default\"); end endcase\n  \
         initial #1 $display(\"L=%b LN=%b\", L, LN);\nendmodule\n",
        &["L=1 LN=0", "gcase item", "gif then"],
    );
}

/// REFUSED (E3009), a round-3 guard: `PV ==? 4'sb1?00` over a parameter typed by an
/// overridden signed type. The common-width routine folded it, but read `PV` unsigned
/// (P3) and printed `R=0`; loud is the safe answer until P3 closes.
/// - iverilog, sv2v → iverilog, verilator: `R=1 lt0=1`.
#[test]
fn s_a_wildcard_over_a_type_parameter_typed_constant_stays_loud() {
    let r = run(
        "`timescale 1ns/1ns\nmodule sub #(parameter type T = logic [3:0], parameter T PV = '0);\n  \
         localparam R = (PV ==? 4'sb1?00);\n  \
         initial $display(\"R=%0d lt0=%0d\", R, PV < 0);\nendmodule\n\
         module t;\n  sub #(.T(logic signed [63:0]), .PV(-64'sd4)) u();\nendmodule\n",
    );
    assert_eq!(r.code, 1, "stdout:\n{}", r.out);
    assert!(
        r.err.contains("[VITA-E3009]") && r.err.contains("parameter `R` value is not a constant"),
        "stderr:\n{}",
        r.err
    );
    assert!(r.lines().is_empty(), "stdout:\n{}", r.out);
}
