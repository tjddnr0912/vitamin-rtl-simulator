//! The time-0 run of a process-header level list that names a constant
//! (`const_level_header.rs`, companion of `const_level_event_t0.rs`): WHEN it runs
//! against `#0` continuations, the header waiter its live terms keep, the bodies
//! admitted beyond the first cut, and a select of a constant whose index is a
//! variable (never a constant, so loud in every lane).
//!
//! Every expected text below is iverilog 13.0 (`-g2012`) and verilator 5.052
//! (`--binary --timing`); each test says where they differ.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_cleo_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut all = String::from_utf8_lossy(&out.stdout).into_owned();
    all.push_str(&String::from_utf8_lossy(&out.stderr));
    let mut s = String::new();
    for l in all.lines().filter(|l| {
        !l.starts_with("simulation ended")
            && !l.starts_with("errors=")
            && !l.contains("W-PP-TIMESCALE-DEFAULT")
    }) {
        s.push_str(l);
        s.push('\n');
    }
    (s, out.status.success())
}

fn run(src: &str) -> String {
    let (s, ok) = vita(src);
    assert!(ok, "expected exit 0, got:\n{s}");
    s
}

fn loud(src: &str, needle: &str) {
    let (s, ok) = vita(src);
    assert!(!ok, "expected a loud reject, got exit 0:\n{s}");
    assert!(s.contains(needle), "expected `{needle}` in:\n{s}");
}

/// The expected value of a cell BACK ON vita_pre's ROUTE: a header list with no live
/// term is refused (`const_level_header.rs`: vita's `$finish` can end time 0 before
/// its one run). Both oracles' text for such a cell is in the comment beside it.
const REFUSED: &str = "<refused: no live term>";
const NO_LIVE: &str = "a process sensitive only to constants runs once at time 0 in both \
                       reference tools, and vita does not run it";

fn check(cells: &[(&str, &str)]) {
    for (src, want) in cells {
        if *want == REFUSED {
            loud(src, NO_LIVE);
        } else {
            assert_eq!(run(src), *want, "design:\n{src}");
        }
    }
}

/// Order-free comparison for cells whose oracles print one time step in two orders.
fn check_sorted(src: &str, want: &str) {
    let mut got: Vec<String> = run(src).lines().map(str::to_string).collect();
    let mut want: Vec<String> = want.lines().map(str::to_string).collect();
    got.sort();
    want.sort();
    assert_eq!(got, want, "design:\n{src}");
}

/// A select of a constant whose INDEX is a variable is not a constant: `K[i]` changes
/// when `i` does, and both oracles wake on it. vita has no index-tracking wait, so
/// every lane refuses it (the header lane refused it before as a "single-bit level"
/// select; the in-body lane dropped it as a constant and never woke — silent):
/// - sR04 `always @(K[i])`: both `KI at 0` / 1 / 2 / 3;
/// - sR05 `@(K[i] or clk)`: both 0 / 1 / 2 / 3 / 4;
/// - sR06 `@(K[i] or clk)` with a `#1` body: iverilog `KS at 1` / 4 / 7 / 10,
///   verilator 4 / 7 / 10;
/// - sR21 `@(K[i] or posedge clk)`: iverilog 1 / 2 / 3 / 4, verilator also 0;
/// - sR08 in-body `forever begin @(K[i]); … end`: both `IB at 1` / 2 / 3;
/// - dR60 `always @(p::C[i])` (both 0 / 1 / 3), in-body `@(K[i])` and `@(p::C[i])`
///   (both wake at 1), `@(K[i +: 2] or clk)` (both 0 / 1 / 2 / 3 / 4).
#[test]
fn a_select_of_a_constant_with_a_variable_index_is_loud() {
    let needle = "a select of a constant indexed by a net that can change, is not supported";
    let d = |decls: &str, body: &str| {
        format!(
            "{decls}module top;\n  localparam [3:0] K = 4'b0101;\n  reg [1:0] i = 0;\n  reg clk = 0;\n\
               {body}\n  initial begin #1 i = 1; #1 i = 2; #1 clk = 1; #1 i = 3; #2 $display(\"DONE\"); $finish; end\nendmodule\n"
        )
    };
    for (decls, body) in [
        ("", "always @(K[i]) $display(\"KI at %0t\", $time);"),
        ("", "always @(K[i] or clk) $display(\"KI at %0t\", $time);"),
        (
            "",
            "always @(K[i] or clk) begin #1 $display(\"KS at %0t\", $time); end",
        ),
        (
            "",
            "always @(K[i] or posedge clk) $display(\"KE at %0t\", $time);",
        ),
        (
            "",
            "initial begin #0; forever begin @(K[i]); $display(\"IB at %0t\", $time); end end",
        ),
        (
            "",
            "always @(K[i +: 2] or clk) $display(\"KP at %0t\", $time);",
        ),
        (
            "package p; localparam logic [3:0] C = 4'b0101; endpackage\n",
            "always @(p::C[i]) $display(\"PA at %0t\", $time);",
        ),
        (
            "package p; localparam logic [3:0] C = 4'b0101; endpackage\n",
            "initial begin @(p::C[i]) $display(\"IP at %0t\", $time); end",
        ),
    ] {
        loud(&d(decls, body), needle);
    }
    // The term prints as written, operators included.
    loud(
        &d("", "always @(K[i] or clk) $display(\"KI at %0t\", $time);"),
        "an event control on `K[i]`, a select of a constant",
    );
    loud(
        &d(
            "",
            "always @(K[i + 1] or clk) $display(\"KI at %0t\", $time);",
        ),
        "an event control on `K[i + 1]`, a select of a constant",
    );
}

/// Index constness is decided by the index's LEAVES through the lowering's own name
/// funnel, never by a constant fold: the fold resolves a name through the parameter
/// walk alone, which a generate-scope NET of the same name does not stop. With
/// `localparam int i = 0;` outside and `reg [1:0] i` inside `generate … begin : g`,
/// `K[i]` reads the NET:
/// - n01 `always @(K[i])`: both `KI at 0` / 1 / 2 / 3 (vita_pre: E3009);
/// - n02 `@(K[i] or clk)`: both 0 / 1 / 2 / 3 / 4 (vita_pre: E3009);
/// - n25 the same with a `#1` body: iverilog `KS at 1` / 4 / 7 / 10, verilator 4 / 7
///   / 10 (vita_pre: E3009);
/// - n03 in-body `forever begin @(K[i]); … end`: both `IB at 1` / 2 / 3 (vita_pre:
///   `DONE` alone — silent);
/// - n06 `@(posedge K[i] or posedge clk)`: both `KE at 1` / 3 / 4 (vita_pre: `KE at
///   4` alone — silent).
///
/// An index that is neither provably live nor provably constant keeps the answer
/// the lanes gave before: n10 in-body `@(K[$size(arr)-1])` never wakes (all four
/// tools print `DONE`); in a header LEVEL list it stays on the net path (dS14
/// `always @(K[$clog2(P)])`, dS16 `always @(K[f1(1)])`: loud, as in vita_pre).
#[test]
fn index_constness_is_asked_of_the_leaves() {
    let shadow = |sens: &str, body: &str| {
        format!(
            "module top;\n  localparam [3:0] K = 4'b0101;\n  localparam int i = 0;\n  reg clk = 0;\n\
               generate if (1) begin : g\n    reg [1:0] i = 0;\n    {sens} {body}\n\
                 initial begin #1 i = 1; #1 i = 2; #1 i = 3; #1 $display(\"DONE\"); $finish; end\n\
               end endgenerate\nendmodule\n"
        )
    };
    let needle =
        "an event control on `K[i]`, a select of a constant indexed by a net that can change";
    for (sens, body) in [
        ("always @(K[i])", "$display(\"KI at %0t\", $time);"),
        ("always @(K[i] or clk)", "$display(\"KL at %0t\", $time);"),
        (
            "always @(K[i] or clk)",
            "begin #1 $display(\"KS at %0t\", $time); end",
        ),
        (
            "initial forever begin @(K[i]);",
            "$display(\"IB at %0t\", $time); end",
        ),
        (
            "always @(posedge K[i] or posedge clk)",
            "$display(\"KE at %0t\", $time);",
        ),
    ] {
        loud(&shadow(sens, body), needle);
    }
    check(&[(
        "module top;\n  reg [7:0] arr [0:2];\n  localparam [3:0] K = 4'b0101;\n\
           initial begin @(K[$size(arr)-1]); $display(\"NEVER at %0t\", $time); end\n\
           initial begin #3 $display(\"DONE\"); $finish; end\nendmodule\n",
        "DONE\n",
    )]);
    let hdr = |sens: &str| {
        format!(
            "module top;\n  localparam logic [7:0] K = 8'b1010_0101;\n  localparam int P = 4;\n\
               function automatic int f1(input int a); return a + 1; endfunction\n\
               always @({sens}) $display(\"K at %0t\", $time);\n\
               initial begin #3 $display(\"DONE\"); $finish; end\nendmodule\n"
        )
    };
    for sens in ["K[$clog2(P)]", "K[f1(1)]"] {
        loud(
            &hdr(sens),
            "a level (non-edge) event control on a bit or element select is not supported",
        );
    }
}

/// dS14's provably constant twins beside a live term (parameter, operator over
/// parameters, generate index, part and indexed-part selects, `p::I`): each runs at
/// 0 and again at 1 and 2 (clk), the edge list at 1, and the in-body waits never
/// wake. iverilog and verilator print the same line set in a different order
/// (compared sorted). dS14's ALL-CONSTANT lists (`@(K[2])`, `@(K[p::I])`,
/// `@(p::C[P])`: both oracles `K2` / `KI` / `CP at 0`) are BACK ON vita_pre's ROUTE:
/// refused (the `$finish` drain limitation).
#[test]
fn a_provably_constant_index_keeps_the_time_zero_run() {
    check_sorted(
        "package p;\n  localparam int I = 5;\n  localparam logic [7:0] C = 8'b0110_1001;\nendpackage\n\
         module top;\n  localparam logic [7:0] K = 8'b1010_0101;\n  localparam int P = 4;\n  reg clk = 0;\n\
           int x = 0, y = 7;\n\
           always @(K[P] or clk) $display(\"KP at %0t clk=%b\", $time, clk);\n\
           always @(K[P-1:0] or clk) $display(\"KR at %0t clk=%b\", $time, clk);\n\
           always @(K[P +: 2] or clk) $display(\"KX at %0t clk=%b\", $time, clk);\n\
           always @(p::C[p::I -: 3] or clk) $display(\"CI at %0t clk=%b\", $time, clk);\n\
           for (genvar g = 0; g < 2; g++) begin : gg\n\
             always @(K[g] or clk) $display(\"KG%0d at %0t clk=%b\", g, $time, clk);\n  end\n\
           initial begin @(K[P]); $display(\"IB woke at %0t\", $time); end\n\
           initial begin wait (K[0]); $display(\"W0 at %0t\", $time); end\n\
           initial begin x = @(K[P]) y; $display(\"IA x=%0d at %0t\", x, $time); end\n\
           always @(posedge K[2] or posedge clk) $display(\"PE at %0t clk=%b\", $time, clk);\n\
           initial begin #1 clk = 1; #1 clk = 0; #1 $display(\"DONE x=%0d\", x); $finish; end\n\
         endmodule\n",
        "W0 at 0\nKR at 0 clk=0\nKX at 0 clk=0\nCI at 0 clk=0\n\
         KP at 0 clk=0\nKG0 at 0 clk=0\nKG1 at 0 clk=0\nKG1 at 1 clk=1\nKG0 at 1 clk=1\nKP at 1 clk=1\n\
         KR at 1 clk=1\nKX at 1 clk=1\nCI at 1 clk=1\nPE at 1 clk=1\nKG1 at 2 clk=0\nKG0 at 2 clk=0\n\
         KP at 2 clk=0\nKR at 2 clk=0\nKX at 2 clk=0\nCI at 2 clk=0\nDONE x=0\n",
    );
    let alone = |sens: &str| {
        format!(
            "package p;\n  localparam int I = 5;\n  localparam logic [7:0] C = 8'b0110_1001;\nendpackage\n\
             module top;\n  localparam logic [7:0] K = 8'b1010_0101;\n  localparam int P = 4;\n\
               always @({sens}) $display(\"A at %0t\", $time);\n\
               initial begin #3 $display(\"DONE\"); $finish; end\nendmodule\n"
        )
    };
    for sens in ["K[2]", "K[p::I]", "p::C[P]"] {
        check(&[(alone(sens).as_str(), REFUSED)]); // both oracles: A at 0 / DONE
    }
}

/// vita ends a time step at `$finish` without running the processes already woken
/// in it (pre-existing; both oracles run them), and a `$finish` reaches time 0
/// through any process, task or event (a first static scan of `initial` bodies missed
/// tasks, `->` wakes, `always` / `always_comb` bodies and `$exit`). So every header
/// list with NO live term is BACK ON vita_pre's ROUTE: refused, naming that
/// limitation, whatever the design's `$finish` does:
/// - n18 `$finish` in an `initial`'s first batch: both `I at 0` / `A at 0`;
/// - f02 in a child module's `initial`: both `C at 0` / `A at 0`;
/// - f01 after `#1`: both `I at 0` / `A at 0`.
#[test]
fn an_all_constant_list_stays_refused_whatever_finish_does() {
    check(&[
        (
            "module top;\n  localparam int K = 1;\n  always @(K) $display(\"A at %0t\", $time);\n\
               initial begin $display(\"I at %0t\", $time); $finish; end\nendmodule\n",
            REFUSED,
        ),
        (
            "module child;\n  initial begin $display(\"C at %0t\", $time); $finish; end\nendmodule\n\
             module top;\n  localparam int K = 99;\n  child u();\n  always @(K) $display(\"A at %0t\", $time);\nendmodule\n",
            REFUSED,
        ),
        (
            "module top;\n  localparam int K = 99;\n\
               initial begin $display(\"I at %0t\", $time); #1 $finish; end\n\
               always @(K) $display(\"A at %0t\", $time);\nendmodule\n",
            REFUSED,
        ),
    ]);
    loud(
        "module top;\n  localparam int K = 1;\n  always @(K) $display(\"A at %0t\", $time);\n\
           initial begin $display(\"I at %0t\", $time); $finish; end\nendmodule\n",
        "vita's `$finish` can end time 0 before that run (a vita limitation)",
    );
}

/// Every `fork` keeps the header lane. dS13's F (`always @(K or clk)` forking a `#2`
/// child, then `if (clk) disable fork;`) splits the oracles — iverilog `F at 0` / 1 /
/// 3, verilator `F at 1` / 3. Its sibling A (`always @(K or a)` forking `#1 a = a +
/// 1`) runs at 0 in both (`A at 0` / 1 / 2, `DONE a=2`) and was admitted; it is BACK
/// ON vita_pre's ROUTE now (a `join_none` child against a `disable` of the block
/// split the oracles and left a line neither prints), so the whole cell prints
/// vita_pre's text: F as verilator, A never runs (it never did on vita_pre).
#[test]
fn a_fork_keeps_the_header_lane() {
    check(&[(
        "module top;\n  localparam int K = 3;\n  reg clk = 0;\n  int a = 0;\n\
           always @(K or clk) begin\n    $display(\"F at %0t clk=%b\", $time, clk);\n\
             fork\n      begin #2 $display(\"CHILD from clk=%b done at %0t\", clk, $time); end\n    join_none\n\
             if (clk) disable fork;\n  end\n\
           always @(K or a) begin\n    $display(\"A at %0t a=%0d\", $time, a);\n\
             fork\n      begin #1 if (a < 2) a = a + 1; end\n    join_none\n  end\n\
           initial begin #1 clk = 1; #2 clk = 0; #4 $display(\"DONE a=%0d\", a); $finish; end\nendmodule\n",
        "F at 1 clk=1\nF at 3 clk=0\nCHILD from clk=0 done at 5\nDONE a=0\n",
    )]);
}

/// The time-0 run happens in the ACTIVE region after the first batch of `initial`
/// statements and BEFORE any `#0` continuation — where a leading `#0` in the body
/// ran it after an `initial` declared earlier:
/// - sR01 `initial begin #0 c = 1; end` before `always @(K or c)`: iverilog `A at 0
///   c=x` / `A at 0 c=1` / `A at 2 c=2`, verilator the same with `c=0` first;
/// - sR02 `initial begin #0 $display(y); end` before `always @(K) y = 7;`: both `I y=7
///   at 0`;
/// - sR35 `initial begin #0 @(y) …` before it: both `DONE y=7` alone (no wake);
/// - dR03 `initial #0 v1 = 5;` before, `initial #0 v2 = 7;` after `always @(K)`: both
///   `A at 0 v1=0 v2=0`;
/// - dR02 A1 `initial #0 c1 = 1;` before `always @(K or c1)`: two runs at 0 in both
///   (`c1=x` then `c1=1`; verilator `c1=0` first), lines in a different order.
///
/// sR02, sR35 and dR03 are `always @(K)` with no live term: BACK ON vita_pre's ROUTE
/// (refused — the `$finish` drain limitation); their oracle text stays quoted above.
#[test]
fn the_time_zero_run_precedes_every_zero_delay_continuation() {
    check(&[
        (
            "module top;\n  localparam int K = 1;\n  reg [3:0] c;\n  initial begin #0 c = 1; end\n\
               always @(K or c) $display(\"A at %0t c=%0d\", $time, c);\n\
               initial begin #2 c = 2; #2 $display(\"DONE\"); $finish; end\nendmodule\n",
            "A at 0 c=x\nA at 0 c=1\nA at 2 c=2\nDONE\n",
        ),
        (
            "module top;\n  localparam int K = 1;\n  reg [3:0] y;\n\
               initial begin #0 $display(\"I y=%0d at %0t\", y, $time); end\n  always @(K) y = 7;\n\
               initial begin #3 $display(\"DONE y=%0d\", y); $finish; end\nendmodule\n",
            REFUSED, // both oracles: I y=7 at 0 / DONE y=7
        ),
        (
            "module top;\n  localparam int K = 1;\n  reg [3:0] y;\n\
               initial begin #0 @(y) $display(\"SAW y=%0d at %0t\", y, $time); end\n  always @(K) y = 7;\n\
               initial begin #3 $display(\"DONE y=%0d\", y); $finish; end\nendmodule\n",
            REFUSED, // both oracles: DONE y=7
        ),
        (
            "module top;\n  int v1 = 0, v2 = 0;\n  localparam int K = 3;\n  initial #0 v1 = 5;\n\
               always @(K) $display(\"A at %0t v1=%0d v2=%0d\", $time, v1, v2);\n  initial #0 v2 = 7;\n\
               initial #5 begin $display(\"DONE v1=%0d v2=%0d\", v1, v2); $finish; end\nendmodule\n",
            REFUSED, // both oracles: A at 0 v1=0 v2=0 / DONE v1=5 v2=7
        ),
    ]);
    check_sorted(
        "module top;\n  localparam int K = 3;\n  reg c1, c2, r3;\n  wire c3 = r3;\n  initial #0 c1 = 1;\n\
           initial c2 <= 1;\n  initial r3 = 1;\n\
           always @(K or c1) $display(\"A1 at %0t c1=%b\", $time, c1);\n\
           always @(K or c2) $display(\"A2 at %0t c2=%b\", $time, c2);\n\
           always @(K or c3) $display(\"A3 at %0t c3=%b\", $time, c3);\n\
           initial #5 begin $display(\"DONE\"); $finish; end\nendmodule\n",
        "A3 at 0 c3=1\nA1 at 0 c1=x\nA2 at 0 c2=x\nA1 at 0 c1=1\nA2 at 0 c2=1\nDONE\n",
    );
}

/// The live terms keep the HEADER waiter, which sees a same-step glitch (dR11): with
/// `a`, `b` going 0→1 at 1, a blocking 1→0→1 on `a` at 2 and an NBA `b <= 0; b <= 1`
/// at 3, iverilog runs `always @(K or a or b)` at 0 / 1 / 2 / 3, verilator at 0 / 1
/// (cycle-based). vita lands on iverilog, like its own `always @(a or b)`.
#[test]
fn a_live_term_keeps_the_header_waiter() {
    let out = run(
        "module top;\n  localparam int K = 3;\n  reg a = 0, b = 0;\n\
           always @(K or a or b) $display(\"L at %0t a=%b b=%b\", $time, a, b);\n\
           initial begin\n    #1 a = 1; b = 1;\n    #1 a = 0; a = 1;\n    #1 b <= 0; b <= 1;\n\
             #1 $display(\"DONE\"); $finish;\n  end\nendmodule\n",
    );
    assert_eq!(
        out,
        "L at 0 a=0 b=0\nL at 1 a=1 b=1\nL at 2 a=1 b=1\nL at 3 a=1 b=1\nDONE\n"
    );
}

/// Bodies that cannot suspend and that the first cut refused, measured both-agree:
/// - dR44 named-block `disable`: both `D at 0 clk=0` / `D at 1 clk=1` / `D2 at 1`;
/// - dR66 a `for` loop with `break;`: both `BR at 0 s=2 clk=0` / `BR at 1 …`;
/// - k10 `f(1);` (a void function): both `F at 0 n=1` / 5 / 9;
/// - k09 `return` inside the enabled task: both `R2 at 0 clk=x` (verilator `clk=0`) /
///   `R1 at 5` / `R2 at 9 clk=0`.
///
/// A RECURSIVE automatic task (dR54b) keeps the header lane, unchanged from vita_pre:
/// both oracles print `REC at 0` / `REC at 1`, vita `REC at 1` (recorded).
///
/// sR16 (`always @(K) begin : b … disable b; end`, both `D at 0`) and sR17
/// (`always @(K) void'(f(1));`, both `n=1`) have no live term: BACK ON vita_pre's
/// ROUTE, refused (the `$finish` drain limitation).
#[test]
fn a_disable_return_or_function_statement_body_runs_at_time_zero() {
    check(&[
        (
            "module top;\n  localparam int K = 3;\n  reg clk = 0;\n\
               always @(K or clk) begin : db $display(\"D at %0t clk=%b\", $time, clk); if (!clk) disable db; $display(\"D2 at %0t\", $time); end\n\
               initial begin #1 clk = 1; #1 $display(\"DONE\"); $finish; end\nendmodule\n",
            "D at 0 clk=0\nD at 1 clk=1\nD2 at 1\nDONE\n",
        ),
        (
            "module top;\n  localparam int K = 3;\n  reg clk = 0;\n  int s;\n\
               always @(K or clk) begin\n    s = 0;\n    for (int i = 0; i < 5; i++) begin if (i == 2) break; s = s + 1; end\n\
                 $display(\"BR at %0t s=%0d clk=%b\", $time, s, clk);\n  end\n\
               initial begin #1 clk = 1; #1 $display(\"DONE\"); $finish; end\nendmodule\n",
            "BR at 0 s=2 clk=0\nBR at 1 s=2 clk=1\nDONE\n",
        ),
        (
            "module top;\n  localparam int K = 1;\n\
               always @(K) begin : b $display(\"D at %0t\", $time); disable b; end\n\
               initial begin #3 $display(\"DONE\"); $finish; end\nendmodule\n",
            REFUSED, // both oracles: D at 0 / DONE
        ),
        (
            "module top;\n  localparam int K = 99;\n  reg clk;\n\
               task t; begin if (clk === 1'b1) begin $display(\"R1 at %0t\", $time); return; end \
               $display(\"R2 at %0t clk=%b\", $time, clk); end endtask\n  always @(K or clk) t;\n\
               initial begin #5 clk = 1; #4 clk = 0; #6 $display(\"DONE\"); $finish; end\nendmodule\n",
            "R2 at 0 clk=x\nR1 at 5\nR2 at 9 clk=0\nDONE\n",
        ),
        (
            "module top;\n  localparam int K = 99;\n  reg clk;\n  int n = 0;\n\
               function void f(input int a); n = n + a; endfunction\n\
               always @(K or clk) begin f(1); $display(\"F at %0t n=%0d\", $time, n); end\n\
               initial begin #5 clk = 1; #4 clk = 0; #6 $display(\"DONE\"); $finish; end\nendmodule\n",
            "F at 0 n=1\nF at 5 n=2\nF at 9 n=3\nDONE\n",
        ),
        (
            "module top;\n  localparam int K = 3;\n  reg clk = 0;\n\
               task automatic trec(int n); if (n > 0) trec(n - 1); else $display(\"REC at %0t\", $time); endtask\n\
               always @(K or clk) trec(2);\n\
               initial begin #1 clk = 1; #1 $display(\"DONE\"); $finish; end\nendmodule\n",
            "REC at 1\nDONE\n",
        ),
    ]);
    // sR17: both oracles `n=1`.
    check(&[(
        "module top;\n  localparam int K = 1;\n  int n = 0;\n\
           function int f(input int a); n = n + a; return n; endfunction\n\
           always @(K) void'(f(1));\n  initial begin #3 $display(\"n=%0d\", n); $finish; end\nendmodule\n",
        REFUSED,
    )]);
}

/// A dotted name in an index is PROVABLY LIVE only when the event-control resolver's
/// own predicate (`lookup_dotted_net`) resolves it to a net; otherwise it is unknown,
/// so the in-body drop keeps vita_pre's answer and the header lane stays on its net
/// path:
/// - q01 in-body `@(K[SP.a])`, `SP` a packed-struct localparam (the member access is a
///   constant part-select): verilator `DONE` (iverilog rejects the struct literal),
///   vita_pre `DONE`, and vita `DONE`;
/// - q02 header `@(K[SP.a] or clk)`: verilator `H at 0` / 1 / 2; vita_pre and vita loud;
/// - q03 in-body `@(K[u.x])` with `u.x` a child net written at 1, 2, 3, and q04 the
///   same through a generate-scope `g.x`: both oracles `IB at 1` / 2 / 3; the name
///   does not resolve when the wait is lowered, so vita keeps vita_pre's `DONE`
///   alone (recorded: silent, unchanged from vita_pre).
#[test]
fn a_dotted_index_is_live_only_when_it_resolves_to_a_net() {
    let st = "  typedef struct packed { logic [1:0] a; logic [1:0] b; } st_t;\n\
              localparam st_t SP = '{a: 2'd2, b: 2'd1};\n  localparam [3:0] K = 4'b0101;\n";
    check(&[
        (
            format!(
                "module top;\n{st}  initial begin @(K[SP.a]); $display(\"NEVER at %0t\", $time); end\n\
                   initial begin #3 $display(\"DONE\"); $finish; end\nendmodule\n"
            )
            .as_str(),
            "DONE\n",
        ),
        (
            "module child;\n  reg [1:0] x = 0;\n  initial begin #1 x = 1; #1 x = 2; #1 x = 3; end\nendmodule\n\
             module top;\n  localparam [3:0] K = 4'b0101;\n  child u();\n\
               initial forever begin @(K[u.x]); $display(\"IB at %0t x=%0d\", $time, u.x); end\n\
               initial begin #5 $display(\"DONE\"); $finish; end\nendmodule\n",
            "DONE\n",
        ),
        (
            "module top;\n  localparam [3:0] K = 4'b0101;\n  generate if (1) begin : g\n    reg [1:0] x = 0;\n  end endgenerate\n\
               initial forever begin @(K[g.x]); $display(\"IB at %0t x=%0d\", $time, g.x); end\n\
               initial begin #1 g.x = 1; #1 g.x = 2; #1 g.x = 3; #2 $display(\"DONE\"); $finish; end\nendmodule\n",
            "DONE\n",
        ),
    ]);
    loud(
        &format!(
            "module top;\n{st}  reg clk = 0;\n  always @(K[SP.a] or clk) $display(\"H at %0t\", $time);\n\
               initial begin #1 clk = 1; #1 clk = 0; #1 $display(\"DONE\"); $finish; end\nendmodule\n"
        ),
        "a level (non-edge) event control on a bit or element select is not supported",
    );
}

/// Round-3 shapes, each BACK ON vita_pre's ROUTE:
/// - dT20 a `join_none` child with `#2` against `disable me` of the block: iverilog
///   runs F and G at 0 and prints `GCH from clk=1 done at 2`, verilator neither; the
///   time-0 run no longer happens (every fork keeps the header lane), and vita
///   prints vita_pre's text (its `FCH … done at 3` is the pre-existing survival of a
///   disabled block's child, not this slice's);
/// - p09 in-body `@(K[ARR[1]])`, `ARR` an unpacked array PARAMETER: verilator `DONE
///   r=1` (iverilog rejects unpacked array parameters); the index names a constant
///   parameter net, which is not live, so the wait never wakes, as on vita_pre;
/// - p01 `$finish` in a task an `initial` enables at time 0, beside `always @(K)`:
///   both oracles `I at 0` / `T at 0` / `A at 0`; refused (no live term).
#[test]
fn round_three_shapes_are_back_on_the_pre_route() {
    check(&[
        (
            "module top;\n  localparam int K = 3;\n  reg clk = 0;\n\
               always @(K or clk) begin : me\n    $display(\"F at %0t clk=%b\", $time, clk);\n\
                 fork\n      begin #2 $display(\"FCH from clk=%b done at %0t\", clk, $time); end\n    join_none\n\
                 if (clk) disable me;\n  end\n\
               task td; disable fork; endtask\n\
               always @(K or clk) begin\n    $display(\"G at %0t clk=%b\", $time, clk);\n\
                 fork\n      begin #2 $display(\"GCH from clk=%b done at %0t\", clk, $time); end\n    join_none\n\
                 if (clk) td;\n  end\n\
               initial begin #1 clk = 1; #2 clk = 0; #4 $display(\"DONE\"); $finish; end\nendmodule\n",
            "F at 1 clk=1\nG at 1 clk=1\nFCH from clk=0 done at 3\nF at 3 clk=0\nG at 3 clk=0\n\
             FCH from clk=0 done at 5\nGCH from clk=0 done at 5\nDONE\n",
        ),
        (
            "`timescale 1ns/1ns\nmodule top;\n  localparam [3:0] K = 4'b0110;\n\
               localparam int ARR [0:1] = '{1, 2};\n  reg r = 0;\n\
               initial forever begin @(K[ARR[1]]); $display(\"IB at %0t\", $time); end\n\
               initial begin #1 r = 1; #1 $display(\"DONE r=%0d\", r); $finish; end\nendmodule\n",
            "DONE r=1\n",
        ),
        (
            "module top;\n  localparam int K = 99;\n\
               task t; begin $display(\"T at %0t\", $time); $finish; end endtask\n\
               initial begin $display(\"I at %0t\", $time); t(); end\n\
               always @(K) $display(\"A at %0t\", $time);\nendmodule\n",
            REFUSED,
        ),
    ]);
}
