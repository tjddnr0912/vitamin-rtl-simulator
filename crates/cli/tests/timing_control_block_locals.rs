//! A `begin … end` under a timing control declares its own variables (ROADMAP §5.2 row
//! 72).
//!
//! ## What was wrong
//!
//! `initial #1 begin logic [7:0] x; x = 8'hAA; … end` beside a module `x` printed
//! `inner=000000aa outer=000000aa`: the block's declaration was dropped, so `x` inside
//! it was the module's 32-bit variable and the write landed there. With no module `x`
//! the same block was E3010 `undeclared net/variable t.x`, and a run-time `$bits` of a
//! selected local read the module's (`r=01` where the oracles print `ff`). The same
//! held under `@(e) begin`, `wait (c) begin`, a `#d begin` inside a block, a `fork`, a
//! generate block, an interface, and a task body (static, automatic, package).
//!
//! The walkers that find declaring blocks each spelled their own `match`. The census
//! (`gather_nested_block_locals`, which feeds the row-21 `select_scope` census, the
//! scope-leak gate and the coalesce set) entered a timing control's body; the Nets-phase
//! hoist (`hoist_block_local_nets`), the scoping classifier (`gather_auto_block_locals`)
//! and the subroutine collectors (`collect_block_local_decls_spanned`) did not.
//!
//! ## The fix
//!
//! One descent, `block_local::for_each_nested_stmt`, which every one of those walkers
//! (and the per-entry classifier, the package-routine scope walk, the census's action
//! walk and the multidriver shadow guard) now takes. In a MODULE process (an `initial`,
//! `always*` or `final` of a module, generate block or interface) a block under a timing
//! control is then hoisted and scoped exactly as the same block is without one, so each
//! value here equals its control twin's (the same block after a bare `#1;`).
//!
//! Round 1 of the row's review showed what that twin is in a TASK body: a static task's
//! argument substitution over a block-local of the same name, an `automatic` initializer
//! run once at the first call, a package routine's routine-wide skip list for its own
//! constants — values the oracles do not print. So a task or function body opts out: a
//! block-local declaration under a timing control there is E3009 ("… not supported yet"),
//! where it was E3010 or silently took a same-named variable's value (review rounds 3–4
//! tried a name rule instead, and each round found a name kind or read position it missed;
//! lifting the refusal is §5.2 row 72b). Three refusals and one fix keep the
//! module-process lane right where the twin is not:
//! * an `automatic` local in a `fork … join_none` arm that a loop or an `always` spawns
//!   again while an earlier one is live (one flattened net for both activations);
//! * a `$strobe` / `$monitor` read of a local whose net another block shares;
//! * `%m` no longer prints the internal `$blk$<n>` scope of a shadowing local's block;
//! * the hierarchical size census keeps its walk, so `u.hs` beside such a local keeps the
//!   module variable's width.
//!
//! Opening those blocks reached two walkers of their own, each pinned below:
//! * the static (inline) task lane's local reservation carried a multi-dim packed local
//!   (`logic [3:0][7:0] x`) as its OUTER range wide with no extents, so `x[1]` read bit 1
//!   (`e=0 b=1`) even with no timing control at all — it now mirrors the frame lane;
//! * the call-graph walker (`collect_callee_stmt`) had no timing-control arm, so a
//!   package routine's call inside `#d` / `wait (c)` was never injected and bound the
//!   calling module's same-named function (`r=2` for `r=1001`), and a static task
//!   recursing only through `#1` was refused as recursive.
//!
//! ## Oracles
//!
//! Every value is iverilog 13 (`-g2012`, `vvp -n`), sv2v → iverilog 13 and verilator
//! 5.052 (`--binary`), all three identical, except where a test says otherwise. sv2v is
//! not an oracle for a static block-local's initializer (it runs it at block entry:
//! `02` where iverilog and verilator print `01`) nor for an initializer-free `int`'s
//! retention (`x`); verilator is 2-state (`il_retain`).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// (stdout + stderr, exit code, the VCD the run wrote if any).
fn run_files(src: &str) -> (String, Option<i32>, Option<String>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tcbl_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let vcd = std::fs::read_to_string(d.join("t.vcd")).ok();
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code(), vcd)
}

/// A clean run (exit 0) whose output holds every `want` line exactly, as its own line.
fn lines(src: &str, want: &[&str]) {
    let (o, code, _) = run_files(src);
    assert_eq!(
        code,
        Some(0),
        "expected a clean run:\n{o}\n--- design\n{src}"
    );
    for w in want {
        assert!(
            o.lines().any(|l| l == *w),
            "expected the line {w:?} in:\n{o}\n--- design\n{src}"
        );
    }
}

/// A clean run whose `R `-prefixed lines are exactly `want`, in order.
fn exactly(src: &str, want: &[&str]) {
    let (o, code, _) = run_files(src);
    assert_eq!(
        code,
        Some(0),
        "expected a clean run:\n{o}\n--- design\n{src}"
    );
    let got: Vec<&str> = o.lines().filter(|l| l.starts_with("R ")).collect();
    assert_eq!(got, want, "in:\n{o}\n--- design\n{src}");
}

/// A LOUD run (exit 1) whose output contains every `want` fragment.
fn loud(src: &str, want: &[&str]) {
    let (o, code, _) = run_files(src);
    assert_eq!(
        code,
        Some(1),
        "expected a loud run:\n{o}\n--- design\n{src}"
    );
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}\n--- design\n{src}");
    }
}

/// The census's process forms: `{B}` is the declaring block, `go` rises at `#2`, the
/// closing display runs at `#5`.
const FORMS: &[(&str, &str, &str)] = &[
    ("delay", "initial #1 {B}", ""),
    ("event", "initial @(go) {B}", ""),
    ("wait", "initial wait (go) {B}", ""),
    (
        "event in a block",
        "initial begin\n    @(go) {B}\n  end",
        "",
    ),
    ("delay in a block", "initial begin\n    #1 {B}\n  end", ""),
    ("repeat event", "initial repeat (1) @(go) {B}", ""),
    ("if delay", "initial if (go == 0) #1 {B}", ""),
    ("delay then event", "initial #1 @(go) {B}", ""),
    ("fork arm", "initial fork\n    #1 {B}\n  join", ""),
    (
        "fork block arm",
        "initial fork\n    begin #1 {B} end\n  join",
        "",
    ),
    (
        "generate",
        "if (1) begin : g\n    initial #1 {B}\n  end",
        "",
    ),
    ("always event delay", "always @(go) #1 {B}", ""),
    (
        "case item",
        "initial case (go)\n    1'b0: #1 {B}\n    default: ;\n  endcase",
        "",
    ),
    (
        "while",
        "initial while (k < 1) begin\n    k++;\n    #1 {B}\n  end",
        "int k = 0;",
    ),
    (
        "nested declaring blocks",
        "initial #1 begin\n    logic [3:0] q;\n    q = 4'h3;\n    @(go) {B}\n  end",
        "",
    ),
];

fn cell(form: &str, extra: &str, module: &str, body: &str, closing: &str) -> String {
    let p = form.replace("{B}", &format!("begin\n{body}\n  end"));
    let mut items = vec![module, "logic go = 0;", "initial #2 go = 1;", extra];
    items.retain(|s| !s.is_empty());
    format!(
        "module t;\n  {}\n  {p}\n  initial begin #5 {closing} $finish; end\nendmodule\n",
        items.join("\n  ")
    )
}

// ── (1) the row's own cells ─────────────────────────────────────────────────

/// w14: the block's `x` is its own 8-bit variable and the module's keeps its value.
/// PRE: `inner=000000aa outer=000000aa`.
#[test]
fn w14_the_write_stays_in_the_block() {
    exactly(
        r#"module t;
  logic [31:0] x = 32'h11111111;
  initial #1 begin
    logic [7:0] x;
    x = 8'hAA;
    $display("R inner=%h", x);
  end
  initial begin #2 $display("R outer=%h", x); $finish; end
endmodule
"#,
        &["R inner=aa", "R outer=11111111"],
    );
}

/// w15: with no module twin the block's local is declared. PRE: E3010 `t.x`.
#[test]
fn w15_no_module_twin_is_declared() {
    exactly(
        r#"module t;
  logic [63:0] r;
  initial #1 begin
    logic [3:0][7:0] x;
    r = {$bits(x[1]){1'b1}};
  end
  initial begin #2 $display("R r=%h", r); $finish; end
endmodule
"#,
        &["R r=00000000000000ff"],
    );
}

/// w02 / w03 / w07 / w11: a run-time `$bits` of a selected local reads the local's
/// element width. PRE `r=01` (the module's 32-bit `x`, whose `x[1]` is one bit). w09 (the
/// same block in an automatic task) is refused: `task_bodies_refuse_a_timing_block_local`.
#[test]
fn runtime_bits_reads_the_block_local() {
    let tail = "  initial begin #1 go = 1; #1 $display(\"R r=%h\", r); $finish; end\nendmodule\n";
    let head = "module t;\n  logic [31:0] x;\n  logic [63:0] r;\n  logic go = 0;\n";
    let body = "begin\n    logic [3:0][7:0] x;\n    r = {$bits(x[1]){1'b1}};\n  end";
    for (name, proc) in [
        ("w02 delay", format!("  initial #1 {body}\n")),
        ("w03 wait", format!("  initial wait (go) {body}\n")),
        (
            "w07 inner event",
            format!("  initial begin\n    @(go) {body}\n  end\n"),
        ),
    ] {
        let src = format!("{head}{proc}{tail}");
        let (o, code, _) = run_files(&src);
        assert_eq!(code, Some(0), "{name}:\n{o}");
        assert!(
            o.lines().any(|l| l == "R r=00000000000000ff"),
            "{name}:\n{o}"
        );
    }
    // w11: an unpacked local's element width sizes an indexed part-select.
    exactly(
        r#"module t;
  logic [31:0] x;
  logic [63:0] r;
  logic [63:0] val = 64'h0123_4567_89AB_CDEF;
  initial #1 begin
    logic [7:0] x [0:3];
    r = val[0 +: $bits(x[1])];
  end
  initial begin #2 $display("R r=%h", r); $finish; end
endmodule
"#,
        &["R r=00000000000000ef"],
    );
}

// ── (2) every form, shadowing and not ───────────────────────────────────────

/// Every timing-control form of the census, with a module `x` the block shadows.
#[test]
fn every_timing_control_form_shadows_the_module_variable() {
    for (name, form, extra) in FORMS {
        let src = cell(
            form,
            extra,
            "logic [31:0] x = 32'h11111111;",
            "    logic [7:0] x;\n    x = 8'hAA;\n    $display(\"R inner=%h\", x);",
            "$display(\"R outer=%h\", x);",
        );
        let (o, code, _) = run_files(&src);
        assert_eq!(code, Some(0), "{name}:\n{o}");
        assert!(o.lines().any(|l| l == "R inner=aa"), "{name}:\n{o}");
        assert!(o.lines().any(|l| l == "R outer=11111111"), "{name}:\n{o}");
        assert!(!o.contains("000000aa"), "{name}:\n{o}");
    }
}

/// The same forms with no module `x`: each was E3010 and now runs.
#[test]
fn every_timing_control_form_declares_without_a_module_twin() {
    for (name, form, extra) in FORMS {
        let src = cell(
            form,
            extra,
            "",
            "    logic [7:0] x;\n    x = 8'hAA;\n    $display(\"R inner=%h\", x);",
            "$display(\"R done\");",
        );
        let (o, code, _) = run_files(&src);
        assert_eq!(code, Some(0), "{name}:\n{o}");
        assert!(o.lines().any(|l| l == "R inner=aa"), "{name}:\n{o}");
    }
}

/// A process that re-enters the block (`always #2`, `forever #2`) runs it twice.
#[test]
fn a_repeating_process_reenters_its_block() {
    for form in ["always #2 {B}", "initial forever #2 {B}"] {
        exactly(
            &cell(
                form,
                "",
                "logic [31:0] x = 32'h11111111;",
                "    logic [7:0] x;\n    x = 8'hAA;\n    $display(\"R inner=%h\", x);",
                "$display(\"R outer=%h\", x);",
            ),
            &["R inner=aa", "R inner=aa", "R outer=11111111"],
        );
    }
}

// ── (3) declaration kinds ───────────────────────────────────────────────────

/// One kind per cell under `initial #1` and under `@(go)`, each beside a module `x`.
#[test]
fn declaration_kinds_under_a_timing_control() {
    let kinds: &[(&str, &str, &str, &[&str])] = &[
        (
            "logic [3:0] x = 4'hF;",
            "    int x;\n    x = -5;\n    $display(\"R inner=%0d\", x);",
            "$display(\"R outer=%h\", x);",
            &["R inner=-5", "R outer=f"],
        ),
        (
            "logic [31:0] x = 32'h11111111;",
            "    logic [7:0] x [0:3];\n    x[2] = 8'h5A;\n    $display(\"R inner=%h\", x[2]);",
            "$display(\"R outer=%h\", x);",
            &["R inner=5a", "R outer=11111111"],
        ),
        (
            "logic [31:0] x = 32'h11111111;",
            "    string x;\n    x = \"hi\";\n    $display(\"R inner=%s\", x);",
            "$display(\"R outer=%h\", x);",
            &["R inner=hi", "R outer=11111111"],
        ),
        (
            "typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;\n  logic [31:0] x = 32'h11111111;",
            "    s_t x;\n    x.a = 4'h3;\n    x.b = 4'hC;\n    $display(\"R inner=%h a=%h\", x, x.a);",
            "$display(\"R outer=%h\", x);",
            &["R inner=3c a=3", "R outer=11111111"],
        ),
        (
            "logic [31:0] x = 32'h11111111;",
            "    int x [$];\n    x.push_back(7);\n    x.push_back(9);\n    $display(\"R inner=%0d %0d\", x.size(), x[1]);",
            "$display(\"R outer=%h\", x);",
            &["R inner=2 9", "R outer=11111111"],
        ),
        (
            "logic [31:0] x = 32'h11111111;",
            "    logic [15:0] x;\n    x = 16'h1234;\n    begin\n      logic [7:0] x;\n      x = 8'hAA;\n    end\n    $display(\"R mid=%h\", x);",
            "$display(\"R outer=%h\", x);",
            &["R mid=1234", "R outer=11111111"],
        ),
        (
            "wire [7:0] x = 8'h11;",
            "    logic [7:0] x;\n    x = 8'hAA;\n    $display(\"R inner=%h\", x);",
            "$display(\"R outer=%h\", x);",
            &["R inner=aa", "R outer=11"],
        ),
    ];
    for form in ["initial #1 {B}", "initial @(go) {B}"] {
        for (module, body, closing, want) in kinds {
            lines(&cell(form, "", module, body, closing), want);
        }
    }
}

/// A static declarator's initializer runs once, before time 0 (IEEE 1800-2017 §6.21):
/// `x = y` reads `y`'s declaration value `01`, not the `02` an `initial` writes at time
/// 0; the second entry of a repeating process sees the first one's leftover. iverilog
/// and verilator; sv2v → iverilog prints `02` (it runs the initializer at entry).
#[test]
fn a_static_initializer_runs_once_at_time_zero() {
    let module = "logic [31:0] x = 32'h11111111;\n  logic [7:0] y = 8'h01;\n  initial y = 8'h02;";
    let body = "    logic [7:0] x = y;\n    $display(\"R inner=%h\", x);\n    x = x + 8'h10;";
    exactly(
        &cell(
            "initial #1 {B}",
            "",
            module,
            body,
            "$display(\"R outer=%h\", x);",
        ),
        &["R inner=01", "R outer=11111111"],
    );
    exactly(
        &cell(
            "always #2 {B}",
            "",
            module,
            body,
            "$display(\"R outer=%h\", x);",
        ),
        &["R inner=01", "R inner=11", "R outer=11111111"],
    );
    // An initialized `int` and an initializer-free one (sv2v prints `x` for the latter).
    exactly(
        &cell(
            "always #2 {B}",
            "",
            "",
            "    int c = 5;\n    c++;\n    $display(\"R c=%0d\", c);",
            "$display(\"R done\");",
        ),
        &["R c=6", "R c=7", "R done"],
    );
    exactly(
        &cell(
            "always #2 {B}",
            "",
            "",
            "    int c;\n    c = c + 1;\n    $display(\"R c=%0d\", c);",
            "$display(\"R done\");",
        ),
        &["R c=1", "R c=2", "R done"],
    );
}

/// Two levels of declaring block, each under its own timing control.
#[test]
fn two_levels_of_timing_control_each_declare() {
    exactly(
        r#"module t;
  logic [31:0] x = 32'h11111111;
  logic go = 0;
  initial #2 go = 1;
  initial #1 begin
    logic [15:0] x;
    x = 16'h1234;
    @(go) begin
      logic [7:0] x;
      x = 8'hAA;
      $display("R in=%h", x);
    end
    $display("R mid=%h", x);
  end
  initial begin #5 $display("R outer=%h", x); $finish; end
endmodule
"#,
        &["R in=aa", "R mid=1234", "R outer=11111111"],
    );
}

// ── (4) controls: a block with no timing control above it is unchanged ──────

/// The control twins the census measured PRE-correct; every one keeps its value.
#[test]
fn blocks_without_a_timing_control_are_unchanged() {
    for form in [
        "initial {B}",
        "initial begin\n    {B}\n  end",
        "always @(go) {B}",
        "initial begin\n    #1;\n    {B}\n  end",
    ] {
        lines(
            &cell(
                form,
                "",
                "logic [31:0] x = 32'h11111111;",
                "    logic [7:0] x;\n    x = 8'hAA;\n    $display(\"R inner=%h\", x);",
                "$display(\"R outer=%h\", x);",
            ),
            &["R inner=aa", "R outer=11111111"],
        );
        lines(
            &cell(
                form,
                "",
                "logic [31:0] x;\n  logic [63:0] r;",
                "    logic [3:0][7:0] x;\n    r = {$bits(x[1]){1'b1}};",
                "$display(\"R r=%h\", r);",
            ),
            &["R r=00000000000000ff"],
        );
    }
}

/// What the control refuses, the timing twin refuses the same way: an `automatic`
/// local with an initializer shadowing a module name (E3009, per-entry lifetime), and
/// two same-named locals in two processes that read after a suspension (E3009, one
/// flattened net). Both were E3010 before.
#[test]
fn the_controls_refusals_carry_over() {
    loud(
        &cell(
            "initial #1 {B}",
            "",
            "logic [31:0] x = 32'h11111111;",
            "    automatic int x = 3;\n    x = x + 1;\n    $display(\"R inner=%0d\", x);",
            "$display(\"R outer=%h\", x);",
        ),
        &["error[VITA-E3009]", "an `automatic` block-local `x`"],
    );
    loud(
        r#"module t;
  initial #1 begin logic [7:0] x; x = 8'hAA; #2 $display("R a=%h", x); end
  initial #2 begin logic [7:0] x; x = 8'hBB; $display("R b=%h", x); end
  initial #5 $finish;
endmodule
"#,
        &["error[VITA-E3009]", "shares one flattened net"],
    );
}

// ── (5) other scopes and readers ────────────────────────────────────────────

/// A generate loop's two unrolls each own the block's local.
#[test]
fn per_unroll_storage() {
    lines(
        r#"module t;
  logic [31:0] x = 32'h11111111;
  for (genvar i = 0; i < 2; i++) begin : g
    initial #1 begin
      logic [7:0] x;
      x = 8'hA0 + i;
      #1 $display("R g%0d=%h", i, x);
    end
  end
  initial begin #5 $display("R outer=%h", x); $finish; end
endmodule
"#,
        &["R g0=a0", "R g1=a1", "R outer=11111111"],
    );
}

/// The module variable's other drivers and readers do not see the block's write: an
/// `always_ff` driver, a continuous-assign reader, and an interface's own variable.
#[test]
fn the_module_variable_keeps_its_drivers_and_readers() {
    lines(
        r#"module t;
  logic [7:0] x;
  logic clk = 0;
  always_ff @(posedge clk) x <= 8'h11;
  initial #3 begin
    logic [7:0] x;
    x = 8'hAA;
    $display("R inner=%h", x);
  end
  initial begin #1 clk = 1; #4 $display("R outer=%h", x); $finish; end
endmodule
"#,
        &["R inner=aa", "R outer=11"],
    );
    lines(
        r#"module t;
  logic [31:0] x = 32'h11111111;
  wire [31:0] w = x;
  initial #1 begin
    logic [7:0] x;
    x = 8'hAA;
  end
  initial begin #5 $display("R w=%h", w); $finish; end
endmodule
"#,
        &["R w=11111111"],
    );
    lines(
        r#"interface ifc;
  logic [31:0] x = 32'h11111111;
  initial #1 begin
    logic [7:0] x;
    x = 8'hAA;
    $display("R inner=%h", x);
  end
endinterface
module t;
  ifc u();
  initial begin #5 $display("R outer=%h", u.x); $finish; end
endmodule
"#,
        &["R inner=aa", "R outer=11111111"],
    );
}

/// The local's own `$bits`, and a module constant beside it, each read their own
/// declaration (PRE: `in=32`).
#[test]
fn bits_inside_and_beside_the_block() {
    lines(
        r#"module t;
  logic [31:0] x;
  initial #1 begin
    logic [7:0] x;
    x = 8'hAA;
    $display("R in=%0d v=%h", $bits(x), x);
  end
  localparam int L = $bits(x);
  initial begin #5 $display("R L=%0d", L); $finish; end
endmodule
"#,
        &["R in=8 v=aa", "R L=32"],
    );
}

/// The waveform scope is the control twin's: the shadowing local under its block's
/// `$blk$` scope, 8 bits wide, the module variable beside it.
#[test]
fn vcd_scope_matches_the_control() {
    let (o, code, vcd) = run_files(
        r#"module t;
  logic [31:0] x = 32'h11111111;
  initial begin $dumpfile("t.vcd"); $dumpvars(0, t); end
  initial #1 begin : nm
    logic [7:0] x;
    x = 8'hAA;
  end
  initial begin #5 $display("R outer=%h", x); $finish; end
endmodule
"#,
    );
    assert_eq!(code, Some(0), "{o}");
    let vcd = vcd.expect("a VCD");
    assert!(vcd.contains("$var wire 32 ! x [31:0] $end"), "{vcd}");
    assert!(vcd.contains("$scope module $blk$"), "{vcd}");
    assert!(vcd.contains("$var wire 8 \" x [7:0] $end"), "{vcd}");
    assert!(vcd.contains("b10101010 \""), "{vcd}");
}

// ── (6) the static (inline) task lane's multi-dim packed locals ─────────────

/// A static task's `logic [3:0][7:0] x` is 32 bits with four 8-bit elements, in a
/// block, at the body's top and as a typedef. PRE: the net was the outer range wide
/// (`e=0 b=1 w=8`).
#[test]
fn static_task_multi_dim_packed_local() {
    let probe =
        "      x = 32'h12345678;\n      $display(\"R e=%h b=%0d w=%h\", x[1], $bits(x[1]), x);";
    for (name, src) in [
        (
            "block",
            format!("module t;\n  task tk;\n    begin\n      logic [3:0][7:0] x;\n{probe}\n    end\n  endtask\n  initial #1 tk();\n  initial #5 $finish;\nendmodule\n"),
        ),
        (
            "body top",
            format!("module t;\n  task tk;\n    logic [3:0][7:0] x;\n{probe}\n  endtask\n  initial #1 tk();\n  initial #5 $finish;\nendmodule\n"),
        ),
        (
            "typedef",
            format!("module t;\n  typedef logic [3:0][7:0] w_t;\n  task tk;\n    begin\n      w_t x;\n{probe}\n    end\n  endtask\n  initial #1 tk();\n  initial #5 $finish;\nendmodule\n"),
        ),
    ] {
        let (o, code, _) = run_files(&src);
        assert_eq!(code, Some(0), "{name}:\n{o}");
        assert!(o.lines().any(|l| l == "R e=56 b=8 w=12345678"), "{name}:\n{o}");
    }
}

/// Element and nested writes, the dimension queries, an unpacked array of packed
/// elements, ascending and signed packed dims, and two call sites sharing the static
/// local. PRE: wrong values, and `x[0][3:0] = …` E3009 "nested lvalue select".
#[test]
fn static_task_multi_dim_packed_local_shapes() {
    let task = |body: &str, calls: &str| {
        format!("module t;\n  task tk;\n    begin\n{body}\n    end\n  endtask\n  initial begin #1 {calls} end\n  initial #5 $finish;\nendmodule\n")
    };
    exactly(
        &task(
            "      logic [3:0][7:0] x;\n      x = '0;\n      x[2] = 8'hC3;\n      x[0][3:0] = 4'h9;\n      $display(\"R w=%h p=%h\", x, x[2][7:4]);",
            "tk();",
        ),
        &["R w=00c30009 p=c"],
    );
    exactly(
        &task(
            "      logic [3:0][7:0] x;\n      $display(\"R d=%0d s1=%0d s2=%0d l2=%0d r2=%0d\", $dimensions(x), $size(x,1), $size(x,2), $left(x,2), $right(x,2));",
            "tk();",
        ),
        &["R d=2 s1=4 s2=8 l2=7 r2=0"],
    );
    exactly(
        &task(
            "      logic [1:0][3:0] a [0:1];\n      a[0] = 8'h00;\n      a[1] = 8'hA5;\n      a[0][1] = 4'h3;\n      $display(\"R a0=%h a1=%h s=%h b=%0d\", a[0], a[1], a[1][0], $bits(a[1]));",
            "tk();",
        ),
        &["R a0=30 a1=a5 s=5 b=8"],
    );
    exactly(
        &task(
            "      logic [0:3][7:0] x;\n      x = 32'h12345678;\n      $display(\"R e=%h w=%h\", x[1], x);",
            "tk();",
        ),
        &["R e=34 w=12345678"],
    );
    exactly(
        &task(
            "      logic signed [1:0][7:0] x;\n      x = 16'hFF80;\n      $display(\"R w=%0d e=%h\", x, x[0]);",
            "tk();",
        ),
        &["R w=-128 e=80"],
    );
    exactly(
        &task(
            "      logic [1:0][3:0] x = 8'h5A;\n      $display(\"R w=%h e=%h\", x, x[1]);",
            "tk();",
        ),
        &["R w=5a e=5"],
    );
    // Static retention across three calls (iverilog and sv2v -> iverilog; verilator is
    // 2-state and prints `0000 1122 2222`).
    exactly(
        &task(
            "      logic [1:0][7:0] x;\n      $display(\"R x=%h\", x);\n      x[1] = 8'h11 + x[1];\n      x[0] = 8'h22;",
            "tk(); tk(); tk();",
        ),
        &["R x=xxxx", "R x=xx22", "R x=xx22"],
    );
}

// ── (7) the call-graph walker ───────────────────────────────────────────────

/// A package routine's callee reached only inside a timing control binds the
/// routine's own package (PRE: the calling module's `k`, `r=2`; with none, E3010 "call to
/// undeclared function `k`").
#[test]
fn a_package_callee_under_a_timing_control_binds_its_package() {
    let src = |task_body: &str, module_k: bool| {
        format!(
            "package p;\n  function automatic int k(int v); return v + 1000; endfunction\n  task automatic a(output int o);\n{task_body}\n  endtask\nendpackage\nmodule t;\n  import p::a;\n{}  int r;\n  initial a(r);\n  initial #5 begin $display(\"R r=%0d\", r); $finish; end\nendmodule\n",
            if module_k {
                "  function automatic int k(int v); return v + 1; endfunction\n"
            } else {
                ""
            }
        )
    };
    exactly(&src("    #1 o = k(1);", true), &["R r=1001"]);
    exactly(&src("    #1 o = k(1);", false), &["R r=1001"]);
    // A delay value: the package `k(1)` is 2, so `o` is written at time 2, before the
    // display at 3 (PRE: the module's `k(1)` = 4, `r=0`).
    exactly(
        r#"package p;
  function automatic int k(int v); return v + 1; endfunction
  task automatic a(output int o);
    #(k(1)) o = 7;
  endtask
endpackage
module t;
  import p::a;
  function automatic int k(int v); return v + 3; endfunction
  int r;
  initial a(r);
  initial begin #3 $display("R r=%0d t=%0t", r, $time); #3 $finish; end
endmodule
"#,
        &["R r=7 t=3"],
    );
    // A wait condition: verilator alone runs it (iverilog, directly and after sv2v:
    // "internal error: Empty NexusSet from wait expression").
    exactly(&src("    wait (k(0) > 500) o = 7;", true), &["R r=7"]);
}

/// A static task recursing only through a timing control is recursive. PRE: E3009
/// "recursive task `r` (frame-call deferred)". A static task's formal is one variable,
/// so every activation prints the last value written (`n=0`): iverilog and sv2v →
/// iverilog, and the same task recursing after a bare `#1;`; verilator prints
/// `n=0 1 2` for both (it gives a static task's formal per-call storage).
#[test]
fn a_static_task_recursing_through_a_delay_is_framed() {
    exactly(
        r#"module t;
  task r(input int n);
    if (n > 0) #1 r(n - 1);
    $display("R n=%0d t=%0t", n, $time);
  endtask
  initial r(2);
  initial #9 $finish;
endmodule
"#,
        &["R n=0 t=2", "R n=0 t=2", "R n=0 t=2"],
    );
}

// ── (8) round 5: a task or function body refuses a timing-body declaration ──

/// The refusal text for a declaration of `names` in the body of subroutine `routine`.
fn refusal(routine: &str, names: &str) -> String {
    format!(
        "declaration of {names} under a timing control (`#d`, `@(e)`, `wait (c)`) in the body of \
         subroutine `{routine}`: a declaration under a timing control is supported in a process, \
         not yet in a task or function — declare it at the top of `{routine}`"
    )
}

/// Refused: exit 1, `errors` errors in all, the refusal once for each of `names` (one
/// declaration each), no `R` line and no panic.
fn refused(src: &str, routine: &str, names: &[&str], errors: usize) {
    let (o, code, _) = run_files(src);
    assert_eq!(
        code,
        Some(1),
        "expected a loud run:\n{o}\n--- design\n{src}"
    );
    for n in names {
        let want = refusal(routine, &format!("`{n}`"));
        assert_eq!(
            o.matches(&want).count(),
            1,
            "the refusal of `{n}` once in:\n{o}\n--- design\n{src}"
        );
    }
    assert_eq!(
        o.matches("under a timing control (`#d`, `@(e)`, `wait (c)`) in the body of")
            .count(),
        names.len(),
        "one refusal per declaration:\n{o}"
    );
    assert!(
        o.lines()
            .any(|l| l.starts_with(&format!("errors={errors} "))),
        "errors={errors}:\n{o}"
    );
    assert!(!o.lines().any(|l| l.starts_with("R ")), "no R line:\n{o}");
    assert!(!o.contains("panicked"), "no panic:\n{o}");
}

/// Every subroutine lane refuses a block-local declared under a timing control: one
/// E3009 at the declaration, wherever the body is elaborated (each was E3010 before row 72;
/// round 4 ran them). A static task that is never called is never elaborated, so its
/// declaration stays unrefused (`R ok`, as before the row).
#[test]
fn subroutine_bodies_refuse_a_timing_block_local() {
    for (lane, src, routine, names) in [
        (
            r#"T01 static module task"#,
            r#"module t;
  int a;
  task tk(output int o); #1 begin int x; x = 3; o = x; end endtask
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        (
            r#"T02 automatic module task"#,
            r#"module t;
  int a;
  task automatic tk(output int o); #1 begin int x; x = 3; o = x; end endtask
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        (
            r#"T20 static package task"#,
            r#"package p;
  task tk(output int o); #1 begin int Z; Z = 9; o = Z; end endtask
endpackage
module t;
  import p::tk;
  int a;
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"Z"#][..],
        ),
        (
            r#"automatic package task"#,
            r#"package p;
  task automatic tk(output int o);
    #1 begin
      int x;
      x = 9;
      o = x;
    end
  endtask
endpackage
module t;
  import p::tk;
  int a;
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        (
            r#"generate-block task called inside its block"#,
            r#"module t;
  if (1) begin : g
    int r;
    task tk(output int o);
      #1 begin
        int y;
        y = 4;
        o = y;
      end
    endtask
    initial begin tk(r); $display("R r=%0d", r); $finish; end
  end
endmodule
"#,
            r#"tk"#,
            &[r#"y"#][..],
        ),
        (
            r#"`$unit` static task"#,
            r#"task tk(output int o);
  #1 begin
    int x;
    x = 3;
    o = x;
  end
endtask
module t;
  int a;
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        (
            r#"`$unit` automatic task"#,
            r#"task automatic tk(output int o);
  #1 begin
    int x;
    x = 3;
    o = x;
  end
endtask
module t;
  int a;
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        (
            r#"hierarchical call of a module's automatic task"#,
            r#"module sub;
  int g;
  task automatic tk(input int v);
    #1 begin
      int y;
      y = v * 2;
      g = y;
    end
  endtask
endmodule
module t;
  sub u();
  initial begin u.tk(4); $display("R g=%0d", u.g); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"y"#][..],
        ),
        (
            r#"uncalled automatic task"#,
            r#"module t;
  task automatic tk(output int o);
    #1 begin
      int x;
      x = 3;
      o = x;
    end
  endtask
  initial begin $display("R ok"); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        (
            r#"automatic task called only under `if (0)`"#,
            r#"module t;
  int a = 7;
  task automatic tk(output int o);
    #1 begin
      int x;
      x = 3;
      o = x;
    end
  endtask
  initial begin if (0) tk(a); #2 $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        (
            r#"k05 `repeat (2) @(posedge clk) begin int k`"#,
            r#"module t;
  logic clk = 0;
  always #5 clk = ~clk;
  task automatic burst(input int id, input int n);
    int it;
    it = 0;
    repeat (n) @(posedge clk) begin
      automatic int k = id * 10 + it;
      it = it + 1;
      #1 $display("R id=%0d k=%0d t=%0t", id, k, $time);
    end
  endtask
  initial burst(1, 2);
  initial #3 burst(2, 2);
  initial #60 $finish;
endmodule
"#,
            r#"burst"#,
            &[r#"k"#][..],
        ),
        (
            r#"fork arm in a task"#,
            r#"module t;
  int a;
  task automatic tk(output int o);
    fork
      #1 begin
        int x;
        x = 3;
        o = x;
      end
    join
  endtask
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        (
            r#"`wait (c) begin`"#,
            r#"module t;
  int a;
  logic go = 0;
  task automatic tk(output int o);
    wait (go) begin
      int x;
      x = 3;
      o = x;
    end
  endtask
  initial #1 go = 1;
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        (
            r#"a declaration two blocks below the timing control"#,
            r#"module t;
  int a;
  int go = 1;
  task automatic tk(output int o);
    #1 begin
      if (go != 0) begin
        int x;
        x = 3;
        o = x;
      end
    end
  endtask
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        (
            r#"program-block task"#,
            r#"program p;
  int a;
  task tk(output int o);
    #1 begin
      int x;
      x = 3;
      o = x;
    end
  endtask
  initial begin tk(a); $display("R a=%0d", a); end
endprogram
module t;
  p u();
  initial #5 $finish;
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        (
            r#"task spawned by a `fork … join_none` in a function"#,
            r#"module t;
  int a;
  task automatic tk(output int o);
    #1 begin
      int x;
      x = 3;
      o = x;
    end
  endtask
  function void f();
    fork
      tk(a);
    join_none
  endfunction
  initial begin f(); #5 $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
    ] {
        assert!(!lane.is_empty());
        refused(src, routine, names, 1);
    }
    // A function: the refusal names the function; a timing control is illegal there and
    // its own two diagnostics stay.
    refused(
        r#"module t;
  function automatic int f(input int v);
    #1 begin
      int y;
      y = v;
      return y;
    end
  endfunction
  initial begin $display("R f=%0d", f(3)); $finish; end
endmodule
"#,
        "f",
        &["y"],
        3,
    );
    exactly(
        r#"module t;
  task tk(output int o);
    #1 begin
      int x;
      x = 3;
      o = x;
    end
  endtask
  initial begin $display("R ok"); $finish; end
endmodule
"#,
        &["R ok"],
    );
}

/// Ruled cells (row 72's re-plan, S0): each printed the right value before the row only
/// because the dropped declaration aliased a same-typed variable nothing read afterwards,
/// and each is one edit from a wrong value (the twins, wrong before the row). All are
/// refused; the value each design should print is beside it.
#[test]
fn ruled_cells_right_only_by_accident_are_refused() {
    for (cell, src, routine, names) in [
        // T03 a module variable, unread after the task: a=3 (PRE, iverilog, sv2v → iverilog, verilator)
        (
            r#"T03"#,
            r#"module t;
  int x = 7; int a;
  task tk(output int o); #1 begin int x; x = 3; o = x; end endtask
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        // T05 the same in an automatic task: a=3 (PRE and all three)
        (
            r#"T05"#,
            r#"module t;
  int x = 7; int a;
  task automatic tk(output int o); #1 begin int x; x = 3; o = x; end endtask
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        // T09 a task variable of the same type: a=3 (PRE and all three)
        (
            r#"T09"#,
            r#"module t;
  int a;
  task tk(output int o); int w = 5; #1 begin int w; w = 3; o = w; end endtask
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"w"#][..],
        ),
        // T11 an input argument, written first: g=3 (PRE and all three)
        (
            r#"T11"#,
            r#"module t;
  int g;
  task automatic tk(input int v, output int o); #1 begin int v; v = 3; o = v; end endtask
  initial begin tk(7, g); $display("R g=%0d", g); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"v"#][..],
        ),
        // T14 sibling `int i` loops in a static task (k01 shape): s=32 (PRE and all three)
        (
            r#"T14"#,
            r#"module t;
  logic clk = 0; always #1 clk = ~clk;
  int s;
  task tk;
    begin int i; for (i = 0; i < 2; i++) s += 1; end
    @(posedge clk) begin int i; for (i = 0; i < 3; i++) s += 10; end
  endtask
  initial begin s = 0; tk(); $display("R s=%0d", s); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"i"#][..],
        ),
        // T15 the same in an automatic task: s=32 (PRE and all three)
        (
            r#"T15"#,
            r#"module t;
  logic clk = 0; always #1 clk = ~clk;
  int s;
  task automatic tk;
    begin int i; for (i = 0; i < 2; i++) s += 1; end
    @(posedge clk) begin int i; for (i = 0; i < 3; i++) s += 10; end
  endtask
  initial begin s = 0; tk(); $display("R s=%0d", s); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"i"#][..],
        ),
        // T24 an interface task's input argument, written first: g=3 (PRE and all three)
        (
            r#"T24"#,
            r#"interface ifc;
  task tk(input int v, output int o); #1 begin int v; v = 3; o = v; end endtask
endinterface
module t;
  ifc i(); int g;
  initial begin i.tk(7, g); $display("R g=%0d", g); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"v"#][..],
        ),
        // T30 a process local beside an unused task local: s=4 (PRE and all three)
        (
            r#"T30"#,
            r#"module t;
  int s;
  task tk; begin int z; z = 1; s = s + z; end #2 begin int z; z = 3; s = s + z; end endtask
  initial #1 begin int z; end
  initial begin s = 0; tk(); $display("R s=%0d", s); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"z"#][..],
        ),
        // k01 plain loop, then timed loop: i=2 acc=22 t=25, done acc=22 t=25 (PRE and all three)
        (
            r#"k01"#,
            r#"module t;
  logic clk = 0;
  always #5 clk = ~clk;
  int acc = 0;
  task drive(input int n);
    begin
      int i;
      for (i = 0; i < n; i++) acc = acc + 1;
    end
    @(posedge clk) begin
      int i;
      for (i = 0; i < n; i++) begin
        @(posedge clk);
        acc = acc + 10;
      end
      $display("R i=%0d acc=%0d t=%0t", i, acc, $time);
    end
  endtask
  initial begin drive(2); $display("R done acc=%0d t=%0t", acc, $time); end
  initial #100 $finish;
endmodule
"#,
            r#"drive"#,
            &[r#"i"#][..],
        ),
        // k02 timed loop, then plain loop: i=2 acc=20 t=25, j=2 acc=22, done acc=22 t=25 (PRE and all three)
        (
            r#"k02"#,
            r#"module t;
  logic clk = 0;
  always #5 clk = ~clk;
  int acc = 0;
  task drive(input int n);
    @(posedge clk) begin
      int i;
      for (i = 0; i < n; i++) begin
        @(posedge clk);
        acc = acc + 10;
      end
      $display("R i=%0d acc=%0d t=%0t", i, acc, $time);
    end
    begin
      int i;
      for (i = 0; i < n; i++) acc = acc + 1;
      $display("R j=%0d acc=%0d", i, acc);
    end
  endtask
  initial begin drive(2); $display("R done acc=%0d t=%0t", acc, $time); end
  initial #100 $finish;
endmodule
"#,
            r#"drive"#,
            &[r#"i"#][..],
        ),
        // k03 automatic task, plain loop then timed loop: i=2 acc=22 t=25, done acc=22 t=25 (PRE and all three)
        (
            r#"k03"#,
            r#"module t;
  logic clk = 0;
  always #5 clk = ~clk;
  int acc = 0;
  task automatic drive(input int n);
    begin
      int i;
      for (i = 0; i < n; i++) acc = acc + 1;
    end
    @(posedge clk) begin
      int i;
      for (i = 0; i < n; i++) begin
        @(posedge clk);
        acc = acc + 10;
      end
      $display("R i=%0d acc=%0d t=%0t", i, acc, $time);
    end
  endtask
  initial begin drive(2); $display("R done acc=%0d t=%0t", acc, $time); end
  initial #100 $finish;
endmodule
"#,
            r#"drive"#,
            &[r#"i"#][..],
        ),
        // t_later_m a package constant's name in a module task: a=9 (PRE, iverilog, verilator; round 4 E3009)
        (
            r#"t_later_m"#,
            r#"package p;
  task tk(output int o1);
    #1 begin
      int C;
      C = 9;
      o1 = C;
    end
  endtask
endpackage
package q;
  localparam int C = 5;
endpackage
module t;
  import p::tk;
  int C = 3;
  int a;
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"C"#][..],
        ),
        // v6 a package task: a=3 (PRE, iverilog, verilator; round 4 printed no line)
        (
            r#"v6"#,
            r#"package p;
  int V = 5;
  task tk(output int o1);
    @(V) o1 = $time;
    #1 begin
      int V;
      V = 9;
    end
  endtask
  task kick();
    #3 V = 6;
  endtask
endpackage
module t;
  import p::tk;
  import p::kick;
  int a;
  initial fork kick(); begin tk(a); $display("R a=%0d", a); $finish; end join
  initial #50 $finish;
endmodule
"#,
            r#"tk"#,
            &[r#"V"#][..],
        ),
    ] {
        assert!(!cell.is_empty());
        refused(src, routine, names, 1);
    }
    for (cell, src, routine, names) in [
        // T04 = T03 + a later read of the module variable: PRE a=3 x=3; iverilog, sv2v → iverilog, verilator a=3 x=7
        (
            r#"T04"#,
            r#"module t;
  int x = 7; int a;
  task tk(output int o); #1 begin int x; x = 3; o = x; end endtask
  initial begin tk(a); $display("R a=%0d x=%0d", a, x); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"x"#][..],
        ),
        // T09b = T09 at another width: PRE a=4; all three a=0
        (
            r#"T09b"#,
            r#"module t;
  int a;
  task tk(output int o); int w = 5; #1 begin logic [1:0] w; w = 3; w = w + 1; o = w; end endtask
  initial begin tk(a); $display("R a=%0d", a); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"w"#][..],
        ),
        // T12 = T11 read first: PRE g=3 in=7; iverilog, verilator g=3 in=0 (sv2v in=x)
        (
            r#"T12"#,
            r#"module t;
  int g, h;
  task automatic tk(input int v, output int o, output int q); #1 begin int v; q = v; v = 3; o = v; end endtask
  initial begin tk(7, g, h); $display("R g=%0d in=%0d", g, h); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"v"#][..],
        ),
        // T14b = T14 at another width: PRE s=304; all three s=300
        (
            r#"T14b"#,
            r#"module t;
  logic clk = 0; always #1 clk = ~clk;
  int s;
  task tk;
    begin int i; i = 300; s = i; end
    @(posedge clk) begin logic [1:0] i; i = 3; i = i + 1; s = s + i; end
  endtask
  initial begin s = 0; tk(); $display("R s=%0d", s); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"i"#][..],
        ),
        // T15b = T15 at another width: PRE s=304; all three s=300
        (
            r#"T15b"#,
            r#"module t;
  logic clk = 0; always #1 clk = ~clk;
  int s;
  task automatic tk;
    begin int i; i = 300; s = i; end
    @(posedge clk) begin logic [1:0] i; i = 3; i = i + 1; s = s + i; end
  endtask
  initial begin s = 0; tk(); $display("R s=%0d", s); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"i"#][..],
        ),
        // T24b = T24 read first: PRE g=3 in=7; iverilog, verilator g=3 in=0 (sv2v in=x)
        (
            r#"T24b"#,
            r#"interface ifc;
  task tk(input int v, output int o, output int q); #1 begin int v; q = v; v = 3; o = v; end endtask
endinterface
module t;
  ifc i(); int g, h;
  initial begin i.tk(7, g, h); $display("R g=%0d in=%0d", g, h); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"v"#][..],
        ),
        // t_later_mc = t_later_m + a module variable: PRE a=9 mc=9; iverilog, verilator a=9 mc=3
        (
            r#"t_later_mc"#,
            r#"package p;
  task tk(output int o1);
    #1 begin
      int C;
      C = 9;
      o1 = C;
    end
  endtask
endpackage
package q;
  localparam int C = 5;
endpackage
module t;
  import p::tk;
  int C = 3;
  int a;
  initial begin tk(a); $display("R a=%0d mc=%0d", a, C); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"C"#][..],
        ),
        // s6 = v6 + a package variable: PRE a=5 pv=9; round 4 a=0 pv=5; iverilog, verilator a=5 pv=5
        (
            r#"s6"#,
            r#"package p;
  int V = 5;
  task automatic tk(output int o1);
    #(V) o1 = $time;
    #1 begin
      int V;
      V = 9;
    end
  endtask
endpackage
module t;
  import p::tk;
  int a;
  initial begin tk(a); $display("R a=%0d pv=%0d", a, p::V); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"V"#][..],
        ),
    ] {
        assert!(!cell.is_empty());
        refused(src, routine, names, 1);
    }
}

/// T27: a module `integer i` loop drives a task whose `@(posedge clk) begin integer i; …`
/// loops; before row 72 the task's `i` was the module's (`n=4 i=5`; iverilog, sv2v →
/// iverilog and verilator `n=12 i=3`). Refused.
#[test]
fn a_module_loop_variable_named_like_a_task_local_is_refused() {
    refused(
        r#"module t;
  logic clk = 0; always #1 clk = ~clk;
  integer i; int n;
  task send(input int d);
    @(posedge clk) begin integer i; for (i = 0; i < 4; i = i + 1) n = n + 1; end
  endtask
  initial begin n = 0; for (i = 0; i < 3; i = i + 1) send(i); $display("R n=%0d i=%0d", n, i); $finish; end
endmodule
"#,
        r#"send"#,
        &[r#"i"#],
        1,
    );
}

/// Round 4's package-routine repros, now refused with the rest (oracle text beside each).
#[test]
fn round_four_package_repros_are_refused() {
    for (cell, src, routine, names) in [
        // v1: round 4 a=9 b=2 pv=5 mv=2; iverilog, verilator a=9 b=5 pv=5 mv=2 (PRE E3009)
        (
            r#"v1"#,
            r#"package p;
  int V = 5;
  task tk(output int o1, output int o2);
    o2 = V;
    #1 begin
      int V;
      V = 9;
      o1 = V;
    end
  endtask
endpackage
module t;
  import p::tk;
  int V = 2;
  int a, b;
  initial begin tk(a, b); $display("R a=%0d b=%0d pv=%0d mv=%0d", a, b, p::V, V); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"V"#][..],
        ),
        // v2: round 4 a=9 b=7 pv=5 mv=7; iverilog, verilator a=9 b=7 pv=7 mv=2 (PRE E3009)
        (
            r#"v2"#,
            r#"package p;
  int V = 5;
  task tk(output int o1, output int o2);
    #1 begin
      int V;
      V = 9;
      o1 = V;
    end
    V = 7;
    o2 = V;
  endtask
endpackage
module t;
  import p::tk;
  int V = 2;
  int a, b;
  initial begin tk(a, b); $display("R a=%0d b=%0d pv=%0d mv=%0d", a, b, p::V, V); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"V"#][..],
        ),
        // m_imp: round 4 a=0 b=9; iverilog, verilator a=5 b=9 (PRE E3009)
        (
            r#"m_imp"#,
            r#"package p;
  localparam int C = 5;
endpackage
module t;
  import p::*;
  task tk(output int o1, output int o2);
    #(C) o1 = $time;
    #1 begin
      int C;
      C = 9;
      o2 = C;
    end
  endtask
  int a, b;
  initial begin tk(a, b); $display("R a=%0d b=%0d", a, b); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"C"#][..],
        ),
        // m_enum: round 4 a=0 b=9; iverilog, verilator a=6 b=9 (PRE E3009)
        (
            r#"m_enum"#,
            r#"module t;
  typedef enum int {RED = 5, GREEN = 6} col_t;
  task tk(output int o1, output int o2);
    #(GREEN) o1 = $time;
    #1 begin
      int GREEN;
      GREEN = 9;
      o2 = GREEN;
    end
  endtask
  int a, b;
  initial begin tk(a, b); $display("R a=%0d b=%0d", a, b); $finish; end
endmodule
"#,
            r#"tk"#,
            &[r#"GREEN"#][..],
        ),
    ] {
        assert!(!cell.is_empty());
        refused(src, routine, names, 1);
    }
}

/// A declaring block in a task that CONTAINS a timing control but is not under one, and a
/// task-top declaration followed by `#1`, run as before row 72 (`a=8`, all three).
#[test]
fn task_declarations_not_under_a_timing_control_run() {
    for src in [
        r#"module t;
  int a;
  task tk(input int v, output int o);
    begin
      int y;
      #1 y = v * 2;
      o = y;
    end
  endtask
  initial begin tk(4, a); $display("R a=%0d", a); $finish; end
endmodule
"#,
        r#"module t;
  int a;
  task automatic tk(input int v, output int o);
    begin
      int y;
      #1 y = v * 2;
      o = y;
    end
  endtask
  initial begin tk(4, a); $display("R a=%0d", a); $finish; end
endmodule
"#,
        r#"module t;
  int a;
  task automatic tk(input int v, output int o);
    int y;
    #1 y = v * 2;
    o = y;
  endtask
  initial begin tk(4, a); $display("R a=%0d", a); $finish; end
endmodule
"#,
    ] {
        exactly(src, &["R a=8"]);
    }
}

/// A process local named like a refused task local: the task half is refused (X_mod_static,
/// X_pkg_auto); without the task's timed block the process local keeps its own storage
/// (`task a=7`, `proc z=5`, all three; before row 72 the process was E3010).
#[test]
fn a_process_local_beside_a_task_local() {
    refused(
        r#"module t;
  int a;
  task tk(output int o); #2 begin int z; z = 7; o = z; end endtask
  initial #1 begin int z; z = 5; #5 $display("R proc z=%0d", z); end
  initial begin tk(a); $display("R task a=%0d", a); end
  initial #20 $finish;
endmodule
"#,
        r#"tk"#,
        &["z"],
        1,
    );
    refused(
        r#"package p;
  task automatic tk(output int o); #2 begin int z; z = 7; o = z; end endtask
endpackage
module t;
  import p::tk;
  int a;
  initial #1 begin int z; z = 5; #5 $display("R proc z=%0d", z); end
  initial begin tk(a); $display("R task a=%0d", a); end
  initial #20 $finish;
endmodule
"#,
        r#"tk"#,
        &["z"],
        1,
    );
    for src in [
        r#"module t;
  int a;
  task tk(output int o); #2 o = 7; endtask
  initial #1 begin int z; z = 5; #5 $display("R proc z=%0d", z); end
  initial begin tk(a); $display("R task a=%0d", a); end
  initial #20 $finish;
endmodule
"#,
        r#"package p;
  task automatic tk(output int o); #2 o = 7; endtask
endpackage
module t;
  import p::tk;
  int a;
  initial #1 begin int z; z = 5; #5 $display("R proc z=%0d", z); end
  initial begin tk(a); $display("R task a=%0d", a); end
  initial #20 $finish;
endmodule
"#,
    ] {
        exactly(src, &["R task a=7", "R proc z=5"]);
    }
}

/// Process-lane binding kinds round 4 never pinned: a timing-body local named like each
/// kind takes its own storage and the outside reads keep the named object.
#[test]
fn process_locals_named_like_other_kinds() {
    // P2_pw_var_always: a package variable named in an `always` process (PRE g=29 cnt=9: the wrong net); iverilog, verilator
    exactly(
        r#"package p; int X = 5; endpackage
module t;
  import p::*;
  int a, b, g, h, cnt; logic clk = 0;
  always #1 clk = ~clk;
  always @(posedge clk) begin
    if ($time == 1) begin #(X) a = $time; end
    @(negedge clk) begin int X; X = 9; b = X; end
  end
  initial begin
    #20 g = #(X) 1; g = $time; repeat (X) cnt++;
    $display("R a=%0d b=%0d g=%0d cnt=%0d h=%0d", a, b, g, cnt, h); $finish;
  end
endmodule
"#,
        &[r#"R a=6 b=9 g=25 cnt=5 h=0"#],
    );
    // P2_pe_const_always: a package constant, `always`; iverilog, verilator (PRE E3009)
    exactly(
        r#"package p; localparam int X = 5; endpackage
module t;
  import p::X;
  int a, b, g, h, cnt; logic clk = 0;
  always #1 clk = ~clk;
  always @(posedge clk) begin
    if ($time == 1) begin #(X) a = $time; end
    @(negedge clk) begin int X; X = 9; b = X; end
  end
  initial begin
    #20 g = #(X) 1; g = $time; repeat (X) cnt++;
    begin logic [X-1:0] z; z = '1; h = $bits(z); end
    $display("R a=%0d b=%0d g=%0d cnt=%0d h=%0d", a, b, g, cnt, h); $finish;
  end
endmodule
"#,
        &[r#"R a=6 b=9 g=25 cnt=5 h=5"#],
    );
    // P2_m_lp_fork: a module localparam, fork arm; iverilog, verilator (sv2v cnt=x; PRE E3009)
    exactly(
        r#"module t;
  localparam int X = 5;
  int a, b, g, h, cnt;
  initial begin
    fork
      begin #(X) a = $time; g = #(X) 1; g = $time; repeat (X) cnt++; end
      #1 begin int X; X = 9; b = X; end
    join
    begin logic [X-1:0] z; z = '1; h = $bits(z); end
    $display("R a=%0d b=%0d g=%0d cnt=%0d h=%0d", a, b, g, cnt, h); $finish;
  end
endmodule
"#,
        &[r#"R a=5 b=9 g=10 cnt=5 h=5"#],
    );
    // P2_u_lp_init: a `$unit` localparam, `initial`; iverilog, verilator (sv2v cnt=x; PRE E3009)
    exactly(
        r#"localparam int X = 5;
module t;
  int a, b, g, h, cnt;
  initial begin
    #(X) a = $time;
    #1 begin int X; X = 9; b = X; end
    g = #(X) 1; g = $time;
    repeat (X) cnt++;
    begin logic [X-1:0] z; z = '1; h = $bits(z); end
    $display("R a=%0d b=%0d g=%0d cnt=%0d h=%0d", a, b, g, cnt, h); $finish;
  end
endmodule
"#,
        &[r#"R a=5 b=9 g=11 cnt=5 h=5"#],
    );
    // P2_pw_arr_init: a package unpacked-array localparam; verilator (iverilog, sv2v refuse it; PRE E3009)
    exactly(
        r#"package p; localparam int X [0:1] = '{5, 6}; endpackage
module t;
  import p::*;
  int a, b, g, h, cnt;
  initial begin
    #(X[0]) a = $time;
    #1 begin int X [0:1]; X[0] = 9; b = X[0]; end
    g = #(X[0]) 1; g = $time;
    repeat (X[0]) cnt++;
    begin logic [X[0]-1:0] z; z = '1; h = $bits(z); end
    $display("R a=%0d b=%0d g=%0d cnt=%0d h=%0d", a, b, g, cnt, h); $finish;
  end
endmodule
"#,
        &[r#"R a=5 b=9 g=11 cnt=5 h=5"#],
    );
    // P3_inst: an instance name; all three (PRE E3010)
    exactly(
        r#"module ch(output int o); assign o = 5; endmodule
module t;
  int b; int w1;
  ch X(.o(w1));
  initial #1 begin int X; X = 9; b = X; end
  initial #3 begin $display("R b=%0d h=%0d w=%0d", b, X.o, w1); $finish; end
endmodule
"#,
        &[r#"R b=9 h=5 w=5"#],
    );
    // P3_genblk: a generate-block name; all three (PRE E3010)
    exactly(
        r#"module t;
  int b;
  if (1) begin : X wire [31:0] q = 5; end
  initial #1 begin int X; X = 9; b = X; end
  initial #3 begin $display("R b=%0d q=%0d", b, X.q); $finish; end
endmodule
"#,
        &[r#"R b=9 q=5"#],
    );
    // P3_typedef: a typedef name; sv2v → iverilog, verilator (iverilog refuses; PRE E3010)
    exactly(
        r#"module t;
  int b; int h;
  typedef logic [4:0] X;
  initial #1 begin int X; X = 9; b = X; end
  initial #3 begin X v; v = '1; h = $bits(v); $display("R b=%0d h=%0d v=%0d", b, h, v); $finish; end
endmodule
"#,
        &[r#"R b=9 h=5 v=31"#],
    );
    // P3_port: a port name (PRE e=9: the port); all three
    exactly(
        r#"module ch(input int X, output int b, output int e);
  initial #1 begin int X; X = 9; b = X; end
  initial #2 e = X;
endmodule
module t;
  int b, e;
  ch u(.X(5), .b(b), .e(e));
  initial #3 begin $display("R b=%0d e=%0d", b, e); $finish; end
endmodule
"#,
        &[r#"R b=9 e=5"#],
    );
    // g_program_proc: a program block's `initial` beside a module `x` (iverilog, verilator; PRE E3010)
    exactly(
        r#"module t;
  logic [7:0] x = 8'h11;
  p u();
  initial #5 begin $display("R outer=%h", x); $finish; end
endmodule
program p;
  initial #1 begin
    logic [7:0] x;
    x = 8'hAA;
    $display("R inner=%h", x);
  end
endprogram
"#,
        &[r#"R inner=aa"#, r#"R outer=11"#],
    );
}

/// The capture refusal text for block-local `name` whose block names `path` (`` `a` ``
/// or `` `a` → `b` ``: a by-name chain, not a proven call).
fn capture(name: &str, path: &str) -> String {
    format!(
        "declaration of `{name}` under a timing control (`#d`, `@(e)`, `wait (c)`) in a block \
         whose text names {path} (a chain of names, each mentioned in the text before it — not \
         a proven call), and the last is a callable whose definition names `{name}` too: vita \
         lowers a static task's, a package routine's or a `let`'s body inside the calling block, \
         where `{name}` would be this declaration — not supported yet; rename it"
    )
}

/// Refused for capture: exit 1, the capture refusal once per (local, path), no `R` line.
fn captured(src: &str, pairs: &[(&str, &str)]) {
    let (o, code, _) = run_files(src);
    assert_eq!(
        code,
        Some(1),
        "expected a loud run:\n{o}\n--- design\n{src}"
    );
    for (n, t) in pairs {
        assert_eq!(
            o.matches(&capture(n, t)).count(),
            1,
            "capture of `{n}` by `{t}` in:\n{o}"
        );
    }
    assert!(!o.lines().any(|l| l.starts_with("R ")), "no R line:\n{o}");
    assert!(!o.contains("panicked"), "no panic:\n{o}");
}

/// Round 5's review (P1): a STATIC task's body is lowered inside the calling block, so a
/// free name of the task that a local of a block under a timing control shares binds the
/// local (b7: `a=9`). Before row 72 each of these was loud (the timed declaration was
/// dropped and an outside read hit the scope-leak gate) or wrong; now refused wherever the
/// block reaches such a task (through static tasks and functions) and the task's text
/// names the local anywhere. The value each design should print is beside it.
#[test]
fn a_timed_local_named_like_a_called_static_tasks_name_is_refused() {
    // b7 a module variable read by the task: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  task tk(output int o);
    o = z;
  endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // b7d the task reads it in a delay `#(z)`: iverilog `a=6 b=2 mz=5`; sv2v → iverilog `a=6 b=2 mz=5`; verilator `a=6 b=2 mz=5`
    captured(
        r#"module t;
  int z = 5;
  task tk(output int o);
    #(z) o = $time;
  endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 2;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // b7w the task writes it: iverilog `b=9 mz=7`; sv2v → iverilog `b=9 mz=7`; verilator `b=9 mz=7`
    captured(
        r#"module t;
  int z = 5;
  task tk();
    z = 7;
  endtask
  int b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk();
      b = z;
    end
    $display("R b=%0d mz=%0d", b, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // b7f the block is a fork arm: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  task tk(output int o);
    o = z;
  endtask
  int a, b;
  initial begin
    fork
      #1 begin
        int z;
        z = 9;
        tk(a);
        b = z;
      end
    join
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // b19 a named block: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  task tk(output int o);
    o = z;
  endtask
  int a, b;
  initial begin
    #1 begin : nb
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // b22 one call before the block, one inside: iverilog `a=5 b=9 c=5 mz=5`; sv2v → iverilog `a=5 b=9 c=5 mz=5`; verilator `a=5 b=9 c=5 mz=5`
    captured(
        r#"module t;
  int z = 5;
  task tk(output int o);
    o = z;
  endtask
  int a, b, c;
  initial begin
    tk(c);
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d c=%0d mz=%0d", a, b, c, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // b17 a module net: iverilog `a=5 b=9 mw=5`; sv2v → iverilog `a=5 b=9 mw=5`; verilator `a=5 b=9 mw=5`
    captured(
        r#"module t;
  wire [7:0] w = 8'd5;
  task tk(output int o);
    o = w;
  endtask
  int a, b;
  initial begin
    #1 begin
      int w;
      w = 9;
      tk(a);
      b = w;
    end
    $display("R a=%0d b=%0d mw=%0d", a, b, w);
    $finish;
  end
endmodule
"#,
        &[(r#"w"#, r#"`tk`"#)],
    );
    // b18 a port (PRE wrong too): iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"module m(input int pi);
  task tk(output int o);
    o = pi;
  endtask
  int a, b;
  initial begin
    #1 begin
      int pi;
      pi = 9;
      tk(a);
      b = pi;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
endmodule
module t;
  m u(5);
endmodule
"#,
        &[(r#"pi"#, r#"`tk`"#)],
    );
    // b21 a generate-block process (PRE wrong too): iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  task tk(output int o);
    o = z;
  endtask
  int a, b;
  if (1) begin : g
    initial begin
      #1 begin
        int z;
        z = 9;
        tk(a);
        b = z;
      end
      $display("R a=%0d b=%0d mz=%0d", a, b, z);
      $finish;
    end
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // b6 an imported package variable (PRE wrong too): iverilog `no run`; sv2v → iverilog `a=5 b=9 pv=5`; verilator `a=5 b=9 pv=5`
    captured(
        r#"package p;
  int V = 5;
  localparam int C = 5;
  localparam int ROT [0:1] = '{5, 6};
endpackage
module t;
  import p::*;
  task tk(output int o);
    o = V;
  endtask
  int a, b;
  initial begin
    #1 begin
      int V;
      V = 9;
      tk(a);
      b = V;
    end
    $display("R a=%0d b=%0d pv=%0d", a, b, p::V);
    $finish;
  end
endmodule
"#,
        &[(r#"V"#, r#"`tk`"#)],
    );
    // b7n no read after the block (PRE wrong too): iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"module t;
  int z = 5;
  task tk(output int o);
    o = z;
  endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // an explicit `task static`: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  task static tk(output int o); o = z; endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // a generate-block task and process: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int a, b;
  if (1) begin : g
    int z = 5;
    task tk(output int o); o = z; endtask
    initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
      $display("R a=%0d b=%0d mz=%0d", a, b, z);
      $finish;
    end
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // an interface task called from the interface's own process: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    captured(
        r#"interface ifc;
  int z = 5;
  int a, b;
  task tk(output int o); o = z; endtask
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endinterface
module t;
  ifc u();
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // a static task reached through another static task: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  task tk(output int o); o = z; endtask
  task tk2(output int o); tk(o); endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk2(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk2` → `tk`"#)],
    );
    // a static task forked by a function the block calls: iverilog `no run`; sv2v → iverilog `no run`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  int a, b;
  task tk(output int o); o = z; endtask
  function void f(); fork tk(a); join_none endfunction
  initial begin
    #1 begin
      int z;
      z = 9;
      f(); #1;
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`f` → `tk`"#)],
    );
    // a static task forked by a non-void function (that function is refused on its own too: a fork in a frame function): iverilog `no run`; sv2v → iverilog `no run`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  int a, b, r;
  task tk(output int o); o = z; endtask
  function int f(); fork tk(a); join_none return 1; endfunction
  initial begin
    #1 begin
      int z;
      z = 9;
      r = f();
      #1;
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`f` → `tk`"#)],
    );
    // the call in a fork inside the block: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  task tk(output int o); o = z; endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      fork tk(a); join;
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // the call in a nested `if`: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=x b=9 mz=5`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  task tk(output int o); o = z; endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      if (b == 0) begin tk(a); end;
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // a `$unit` task reading a `$unit` localparam: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"localparam int z = 5;
task tk(output int o); o = z; endtask
module t;
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // a parameter: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"module t;
  parameter int z = 5;
  task tk(output int o); o = z; endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // a localparam: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"module t;
  localparam int z = 5;
  task tk(output int o); o = z; endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // the task names it only in `@(posedge z)` (round 5 waited forever): iverilog `a=4 b=0`; sv2v → iverilog `a=4 b=0`; verilator `a=4 b=0`
    captured(
        r#"module t;
  logic z = 0;
  initial #4 z = 1;
  task tk(output int o); @(posedge z) o = $time; endtask
  int a, b;
  initial begin
    #1 begin
      logic z;
      z = 0;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #30 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // a `function void` reading it: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  int a;
  function void f(); a = z; endfunction
  int b;
  initial begin
    #1 begin
      int z;
      z = 9;
      f();
      b = z;
    end
    #1 $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`f`"#)],
    );
    // a `function void` writing it: iverilog `a=0 b=9 mz=7`; sv2v → iverilog `a=z b=9 mz=7`; verilator `a=0 b=9 mz=7`
    captured(
        r#"module t;
  int z = 5;
  function void f(); z = 7; endfunction
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      f();
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`f`"#)],
    );
    // an enum label: right on round 5; its one-edit twin K_localparam (a localparam) is captured: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"module t;
  typedef enum int {z = 5, Y = 6} e_t;
  task tk(output int o); o = z; endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // a genvar: right on round 5; twin K_localparam captured: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"module t;
  int a, b;
  for (genvar z = 5; z < 6; z++) begin : g
    task tk(output int o); o = z; endtask
    initial begin
      #1 begin
        int z;
        z = 9;
        tk(a);
        b = z;
      end
      $display("R a=%0d b=%0d", a, b);
      $finish;
    end
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // named only in a range of the task: right on round 5; twin K_delay_only (named in a delay) captured: iverilog `a=15 b=9`; sv2v → iverilog `a=15 b=9`; verilator `a=15 b=9`
    captured(
        r#"module t;
  localparam int z = 4;
  task tk(output int o); logic [z-1:0] v; v = '1; o = v; endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // an instance name in `z.v`: right on round 5; twin K_param captured: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"module sub;
  int v = 5;
endmodule
module t;
  sub z();
  task tk(output int o); o = z.v; endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // an imported package constant: right on round 5; twin K_localparam captured: iverilog `no run`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"package p;
  int V = 5;
  localparam int C = 5;
  localparam int ROT [0:1] = '{5, 6};
endpackage
module t;
  import p::*;
  task tk(output int o);
    o = C;
  endtask
  int a, b;
  initial begin
    #1 begin
      int C;
      C = 9;
      tk(a);
      b = C;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
endmodule
"#,
        &[(r#"C"#, r#"`tk`"#)],
    );
    // a module enum label: right on round 5; twin K_localparam captured: iverilog `a=6 b=9`; sv2v → iverilog `a=6 b=9`; verilator `a=6 b=9`
    captured(
        r#"module t;
  typedef enum int {RED = 5, GREEN = 6} col_t;
  task tk(output int o);
    o = GREEN;
  endtask
  int a, b;
  initial begin
    #1 begin
      int GREEN;
      GREEN = 9;
      tk(a);
      b = GREEN;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
endmodule
"#,
        &[(r#"GREEN"#, r#"`tk`"#)],
    );
    // an imported constant in a delay: right on round 5; twin K_delay_only captured: iverilog `no run`; sv2v → iverilog `a=6 b=9`; verilator `a=6 b=9`
    captured(
        r#"package p;
  int V = 5;
  localparam int C = 5;
  localparam int ROT [0:1] = '{5, 6};
endpackage
module t;
  import p::*;
  task tk(output int o);
    #(C) o = $time;
  endtask
  int a, b;
  initial begin
    #1 begin
      int C;
      C = 9;
      tk(a);
      b = C;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
endmodule
"#,
        &[(r#"C"#, r#"`tk`"#)],
    );
}

/// The same capture in a block with NO timing control above it is pre-existing (it was
/// wrong before row 72 too; ROADMAP §2): pinned as known-wrong, the oracle text beside.
/// The callee lanes that do not capture run right.
#[test]
fn static_task_free_name_capture_without_a_timing_control_and_the_lanes_that_run() {
    // KNOWN-WRONG b7_task_free_modvar_plain, b7 without the timing control: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    exactly(
        r#"module t;
  int z = 5;
  task tk(output int o);
    o = z;
  endtask
  int a, b;
  initial begin
    begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[r#"R a=9 b=9 mz=5"#],
    );
    // KNOWN-WRONG b7d_task_delay_modvar_plain, b7d without the timing control: iverilog `a=5 b=2 mz=5`; sv2v → iverilog `a=5 b=2 mz=5`; verilator `a=5 b=2 mz=5`
    exactly(
        r#"module t;
  int z = 5;
  task tk(output int o);
    #(z) o = $time;
  endtask
  int a, b;
  initial begin
    begin
      int z;
      z = 2;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[r#"R a=2 b=2 mz=5"#],
    );
    // KNOWN-WRONG b7w_task_write_modvar_plain, b7w without the timing control: iverilog `b=9 mz=7`; sv2v → iverilog `b=9 mz=7`; verilator `b=9 mz=7`
    exactly(
        r#"module t;
  int z = 5;
  task tk();
    z = 7;
  endtask
  int b;
  initial begin
    begin
      int z;
      z = 9;
      tk();
      b = z;
    end
    $display("R b=%0d mz=%0d", b, z);
    $finish;
  end
endmodule
"#,
        &[r#"R b=7 mz=5"#],
    );
    // KNOWN-WRONG b7n_task_free_modvar_noread_plain, b7n without the timing control: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    exactly(
        r#"module t;
  int z = 5;
  task tk(output int o);
    o = z;
  endtask
  int a, b;
  initial begin
    begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
endmodule
"#,
        &[r#"R a=9 b=9"#],
    );
    // KNOWN-WRONG b7g_always, `always @(posedge clk) begin int z;` (the process top block): iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    exactly(
        r#"module t;
  int z = 5;
  logic clk = 0;
  task tk(output int o);
    o = z;
  endtask
  int a, b, n;
  always @(posedge clk) begin
    int z;
    z = 9;
    tk(a);
    b = z;
  end
  initial begin #1 clk = 1; #1 $display("R a=%0d b=%0d mz=%0d", a, b, z); $finish; end
endmodule
"#,
        &[r#"R a=9 b=9 mz=5"#],
    );
    // KNOWN-WRONG b6_task_free_pv_plain, b6 without the timing control: iverilog `no run`; sv2v → iverilog `a=5 b=9 pv=5`; verilator `a=5 b=9 pv=5`
    exactly(
        r#"package p;
  int V = 5;
  localparam int C = 5;
  localparam int ROT [0:1] = '{5, 6};
endpackage
module t;
  import p::*;
  task tk(output int o);
    o = V;
  endtask
  int a, b;
  initial begin
    begin
      int V;
      V = 9;
      tk(a);
      b = V;
    end
    $display("R a=%0d b=%0d pv=%0d", a, b, p::V);
    $finish;
  end
endmodule
"#,
        &[r#"R a=9 b=9 pv=5"#],
    );
    // b7_task_free_modvar_none, b7 with the local named `w`: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    exactly(
        r#"module t;
  int z = 5;
  task tk(output int o);
    o = z;
  endtask
  int a, b;
  initial begin
    #1 begin
      int W;
      W = 9;
      tk(a);
      b = W;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[r#"R a=5 b=9 mz=5"#],
    );
    // L1a_mod_auto, an `automatic` task: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    exactly(
        r#"module t;
  int z = 5;
  task automatic tk(output int o); o = z; endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[r#"R a=5 b=9 mz=5"#],
    );
    // L5b_static_via_auto, a static task reached only through an `automatic` task: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    exactly(
        r#"module t;
  int z = 5;
  task tk(output int o); o = z; endtask
  task automatic tk2(output int o); tk(o); endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk2(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[r#"R a=5 b=9 mz=5"#],
    );
    // L4_hier_static, a hierarchical call of another module's static task: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `no run`; verilator `a=5 b=9 mz=5`
    exactly(
        r#"module sub;
  int z = 5;
  task tk(output int o); o = z; endtask
endmodule
module t;
  sub u();
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      u.tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, u.z);
    $finish;
  end
endmodule
"#,
        &[r#"R a=5 b=9 mz=5"#],
    );
    // L3b_iface_hier, an interface's static task called hierarchically: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    exactly(
        r#"interface ifc;
  int z = 5;
  task tk(output int o); o = z; endtask
endinterface
module t;
  ifc u();
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      u.tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, u.z);
    $finish;
  end
endmodule
"#,
        &[r#"R a=5 b=9 mz=5"#],
    );
    // A4_static_func_read_nonvoid_write, a non-void static function writing the module variable: iverilog `a=1 b=9 mz=7`; sv2v → iverilog `a=1 b=9 mz=7`; verilator `a=1 b=9 mz=7`
    exactly(
        r#"module t;
  int z = 5;
  function int f(); z = 7; return 1; endfunction
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      a = f();
      b = z;
    end
    #1 $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[r#"R a=1 b=9 mz=7"#],
    );
    // F6_func_calls_func, a non-void function calling a function: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    exactly(
        r#"module t;
  int z = 5;
  function int g(); return z; endfunction
  function int f(); return g(); endfunction
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      a = f();
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
endmodule
"#,
        &[r#"R a=5 b=9 mz=5"#],
    );
    // P3_task, a local named like a task the block does not call (the task text names itself only): iverilog `b=9 d=5`; sv2v → iverilog `b=9 d=5`; verilator `b=9 d=5`
    exactly(
        r#"module t;
  int b, d;
  task X(output int o); o = 5; endtask
  initial #1 begin int X; X = 9; b = X; end
  initial #3 begin X(d); $display("R b=%0d d=%0d", b, d); $finish; end
endmodule
"#,
        &[r#"R b=9 d=5"#],
    );
}

/// Ruled cells of round 5's differential review (main): each was right before row 72
/// only by accident and its one-edit twin was wrong before the row; both are refused. The
/// PRE value and the twin's oracle text are beside each.
#[test]
fn ruled_cells_of_the_round_five_review_are_refused() {
    // ruled t01_task_timed_unused_modtwin: PRE `inner=aa | done`; iverilog `inner=aa | done`; sv2v → iverilog `inner=aa | done`; verilator `inner=aa | done` (twin t01w_modvar_read_after: PRE `inner=aa | outer=aa`; iverilog `inner=aa | outer=11`; sv2v → iverilog `inner=aa | outer=11`; verilator `inner=aa | outer=11`)
    refused(
        r#"module t;
  logic [7:0] x;
  task tk;
    #1 begin
      logic [7:0] x;
      x = 8'hAA;
      $display("R inner=%h", x);
    end
  endtask
  initial tk();
  initial #5 begin $display("R done"); $finish; end
endmodule
"#,
        r#"tk"#,
        &[r#"x"#],
        1,
    );
    // twin t01w_modvar_read_after: PRE `inner=aa | outer=aa`; iverilog `inner=aa | outer=11`; sv2v → iverilog `inner=aa | outer=11`; verilator `inner=aa | outer=11`
    refused(
        r#"module t;
  logic [7:0] x = 8'h11;
  task tk;
    #1 begin
      logic [7:0] x;
      x = 8'hAA;
      $display("R inner=%h", x);
    end
  endtask
  initial tk();
  initial #5 begin $display("R outer=%h", x); $finish; end
endmodule
"#,
        r#"tk"#,
        &[r#"x"#],
        1,
    );
    // ruled t02_task_plain_and_timed_same_name: PRE `a=1 | b=2`; iverilog `a=1 | b=2`; sv2v → iverilog `a=1 | b=2`; verilator `a=1 | b=2` (twin t02w_sibling_other_width: PRE `a=1 | b=4`; iverilog `a=1 | b=0`; sv2v → iverilog `a=1 | b=0`; verilator `a=1 | b=0`)
    refused(
        r#"module t;
  task tk;
    begin
      int i;
      i = 1;
      $display("R a=%0d", i);
    end
    #1 begin
      int i;
      i = 2;
      $display("R b=%0d", i);
    end
  endtask
  initial tk();
  initial #5 $finish;
endmodule
"#,
        r#"tk"#,
        &[r#"i"#],
        1,
    );
    // twin t02w_sibling_other_width: PRE `a=1 | b=4`; iverilog `a=1 | b=0`; sv2v → iverilog `a=1 | b=0`; verilator `a=1 | b=0`
    refused(
        r#"module t;
  task tk;
    begin
      int i;
      i = 1;
      $display("R a=%0d", i);
    end
    #1 begin
      logic [1:0] i;
      i = 3;
      i = i + 1;
      $display("R b=%0d", i);
    end
  endtask
  initial tk();
  initial #5 $finish;
endmodule
"#,
        r#"tk"#,
        &[r#"i"#],
        1,
    );
    // ruled o16_pkgtask_i_caller_has_i: PRE `r=20 t=5`; iverilog `r=20 t=5`; sv2v → iverilog `r=20 t=5`; verilator `r=20 t=5` (twin o16w_caller_var_read_after: PRE `r=20 t=5 i=2`; iverilog `r=20 t=5 i=7`; sv2v → iverilog `r=20 t=5 i=7`; verilator `r=20 t=5 i=7`)
    refused(
        r#"package q;
  parameter int i = 3;
endpackage
package p;
  task automatic drive(output int s);
    s = 0;
    #5 begin
      int i;
      for (i = 0; i < 2; i++) s = s + 10;
    end
  endtask
endpackage
module t;
  import p::*;
  int i;
  int r;
  initial begin drive(r); $display("R r=%0d t=%0t", r, $time); end
  initial #100 $finish;
endmodule
"#,
        r#"drive"#,
        &[r#"i"#],
        1,
    );
    // twin o16w_caller_var_read_after: PRE `r=20 t=5 i=2`; iverilog `r=20 t=5 i=7`; sv2v → iverilog `r=20 t=5 i=7`; verilator `r=20 t=5 i=7`
    refused(
        r#"package q;
  parameter int i = 3;
endpackage
package p;
  task automatic drive(output int s);
    s = 0;
    #5 begin
      int i;
      for (i = 0; i < 2; i++) s = s + 10;
    end
  endtask
endpackage
module t;
  import p::*;
  int i = 7;
  int r;
  initial begin drive(r); $display("R r=%0d t=%0t i=%0d", r, $time, i); end
  initial #100 $finish;
endmodule
"#,
        r#"drive"#,
        &[r#"i"#],
        1,
    );
    // twin o16v_no_const_name_ctl: PRE `r=20 t=5 i=2`; iverilog `r=20 t=5 i=7`; sv2v → iverilog `r=20 t=5 i=7`; verilator `r=20 t=5 i=7`
    refused(
        r#"package p;
  task automatic drive(output int s);
    s = 0;
    #5 begin
      int i;
      for (i = 0; i < 2; i++) s = s + 10;
    end
  endtask
endpackage
module t;
  import p::*;
  int i = 7;
  int r;
  initial begin drive(r); $display("R r=%0d t=%0t i=%0d", r, $time, i); end
  initial #100 $finish;
endmodule
"#,
        r#"drive"#,
        &[r#"i"#],
        1,
    );
    // ruled X8_formal_auto_emu: PRE `task s=3 | proc i=50`; iverilog `task s=3 | proc i=50`; sv2v → iverilog `task s=3 | proc i=50`; verilator `task s=3 | proc i=50` (twin X8_formal_static_emu: PRE `task s=1 | proc i=3`; iverilog `task s=3 | proc i=50`; sv2v → iverilog `task s=3 | proc i=50`; verilator `task s=3 | proc i=50`)
    refused(
        r#"module t;
  int s;
  task automatic tk(input int i);
    #2 begin int i; i = 3; s = s + i; end
  endtask
  initial begin #1; begin int i; i = 50; #9 $display("R proc i=%0d", i); end end
  initial begin s = 0; tk(1); $display("R task s=%0d", s); end
  initial #20 $finish;
endmodule
"#,
        r#"tk"#,
        &[r#"i"#],
        1,
    );
    // twin X8_formal_static_emu: PRE `task s=1 | proc i=3`; iverilog `task s=3 | proc i=50`; sv2v → iverilog `task s=3 | proc i=50`; verilator `task s=3 | proc i=50`
    refused(
        r#"module t;
  int s;
  task tk(input int i);
    #2 begin int i; i = 3; s = s + i; end
  endtask
  initial begin #1; begin int i; i = 50; #9 $display("R proc i=%0d", i); end end
  initial begin s = 0; tk(1); $display("R task s=%0d", s); end
  initial #20 $finish;
endmodule
"#,
        r#"tk"#,
        &[r#"i"#],
        1,
    );
    // ruled X8_sib_auto_emu: PRE `task s=4 | proc i=50`; iverilog `task s=4 | proc i=50`; sv2v → iverilog `task s=4 | proc i=50`; verilator `task s=4 | proc i=50` (twin X8_sib_auto_w: PRE `task s=5 | proc i=50`; iverilog `task s=1 | proc i=50`; sv2v → iverilog `task s=1 | proc i=50`; verilator `task s=1 | proc i=50`)
    refused(
        r#"module t;
  int s;
  task automatic tk;
    begin int i; i = 1; s = s + i; end
    #2 begin int i; i = 3; s = s + i; end
  endtask
  initial begin #1; begin int i; i = 50; #9 $display("R proc i=%0d", i); end end
  initial begin s = 0; tk(); $display("R task s=%0d", s); end
  initial #20 $finish;
endmodule
"#,
        r#"tk"#,
        &[r#"i"#],
        1,
    );
    // twin X8_sib_auto_w: PRE `task s=5 | proc i=50`; iverilog `task s=1 | proc i=50`; sv2v → iverilog `task s=1 | proc i=50`; verilator `task s=1 | proc i=50`
    refused(
        r#"module t;
  int s;
  task automatic tk;
    begin int i; i = 1; s = s + i; end
    #2 begin logic [1:0] i; i = 3; i = i + 1; s = s + i; end
  endtask
  initial begin #1; begin int i; i = 50; #9 $display("R proc i=%0d", i); end end
  initial begin s = 0; tk(); $display("R task s=%0d", s); end
  initial #20 $finish;
endmodule
"#,
        r#"tk"#,
        &[r#"i"#],
        1,
    );
    // ruled X8_sib_static_emu: PRE `task s=4 | proc i=50`; iverilog `task s=4 | proc i=50`; sv2v → iverilog `task s=4 | proc i=50`; verilator `task s=4 | proc i=50` (twin X8_sib_static_w: PRE `task s=5 | proc i=50`; iverilog `task s=1 | proc i=50`; sv2v → iverilog `task s=1 | proc i=50`; verilator `task s=1 | proc i=50`)
    refused(
        r#"module t;
  int s;
  task tk;
    begin int i; i = 1; s = s + i; end
    #2 begin int i; i = 3; s = s + i; end
  endtask
  initial begin #1; begin int i; i = 50; #9 $display("R proc i=%0d", i); end end
  initial begin s = 0; tk(); $display("R task s=%0d", s); end
  initial #20 $finish;
endmodule
"#,
        r#"tk"#,
        &[r#"i"#],
        1,
    );
    // twin X8_sib_static_w: PRE `task s=5 | proc i=50`; iverilog `task s=1 | proc i=50`; sv2v → iverilog `task s=1 | proc i=50`; verilator `task s=1 | proc i=50`
    refused(
        r#"module t;
  int s;
  task tk;
    begin int i; i = 1; s = s + i; end
    #2 begin logic [1:0] i; i = 3; i = i + 1; s = s + i; end
  endtask
  initial begin #1; begin int i; i = 50; #9 $display("R proc i=%0d", i); end end
  initial begin s = 0; tk(); $display("R task s=%0d", s); end
  initial #20 $finish;
endmodule
"#,
        r#"tk"#,
        &[r#"i"#],
        1,
    );
}

/// The refusal names a declaration by the name the source wrote, in any container: a
/// `for (int i …)` loop variable (`i`, not the parser's `__forvar_i_<n>`), a `foreach`
/// index, a `fork … join` declaration.
#[test]
fn the_refusal_names_the_source_declaration_in_any_container() {
    refused(
        r#"module t;
  logic clk = 0;
  always #1 clk = ~clk;
  int s = 0;
  task tk;
    @(posedge clk) for (int i = 0; i < 3; i++) s = s + i;
  endtask
  initial begin tk(); $display("R s=%0d t=%0t", s, $time); end
  initial #20 $finish;
endmodule
"#,
        r#"tk"#,
        &[r#"i"#],
        1,
    );
    refused(
        r#"module t;
  int q[3] = '{1, 2, 3};
  int s = 0;
  task tk;
    #1 foreach (q[k]) s = s + q[k];
  endtask
  initial begin tk(); $display("R s=%0d", s); #5 $finish; end
endmodule
"#,
        r#"tk"#,
        &[r#"k"#],
        1,
    );
    refused(
        r#"module t;
  task tk(output int o);
    #1 fork
      int fz;
      begin fz = 5; o = fz; end
    join
  endtask
  int r;
  initial begin tk(r); $display("R r=%0d", r); #5 $finish; end
endmodule
"#,
        r#"tk"#,
        &[r#"fz"#],
        1,
    );
}

/// Round 6's review (F1, F2): a package routine — static or automatic, task or function —
/// and a `let` are lowered inside the calling block too, so their free names bind a timed
/// local; and the closure runs through the allow-listed module lanes (an automatic task,
/// a non-void function), which round 6 stopped at. Refused, with the call path. The value
/// each design should print is beside it (verilator alone for `let`).
#[test]
fn package_routines_and_lets_reached_from_a_timed_block_are_refused() {
    // F1 r8: a package task reads a name its package imports: iverilog `a=5 b=9 qz=5`; sv2v → iverilog `a=5 b=9 qz=5`; verilator `a=5 b=9 qz=5`
    captured(
        r#"package q;
  int z = 5;
endpackage
package p;
  import q::*;
  task tk(output int o);
    o = z;
  endtask
endpackage
module t;
  import p::tk;
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d qz=%0d", a, b, q::z);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // F1 r8a: an automatic package task: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"package q;
  int z = 5;
endpackage
package p;
  import q::*;
  task automatic tk(output int o); o = z; endtask
endpackage
module t;
  import p::*;
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // F1 r8f: a non-void package function: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"package q;
  int z = 5;
endpackage
package p;
  import q::*;
  function int f(); return z; endfunction
endpackage
module t;
  import p::*;
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      a = f();
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`f`"#)],
    );
    // F1 r8c: an imported localparam: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"package q;
  localparam int z = 5;
endpackage
package p;
  import q::*;
  task tk(output int o); o = z; endtask
endpackage
module t;
  import p::*;
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // F1 r8x: `import q::z;`: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"package q;
  int z = 5;
endpackage
package p;
  import q::z;
  task tk(output int o); o = z; endtask
endpackage
module t;
  import p::*;
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // F1 r8d: `#(z)` in the package task: iverilog `a=6 b=9`; sv2v → iverilog `a=6 b=9`; verilator `a=6 b=9`
    captured(
        r#"package q;
  int z = 5;
endpackage
package p;
  import q::*;
  task tk(output int o); #(z) o = $time; endtask
endpackage
module t;
  import p::*;
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // F1 r8w: the package task writes z: iverilog `a=1 b=9 qz=7`; sv2v → iverilog `a=1 b=9 qz=7`; verilator `a=1 b=9 qz=7`
    captured(
        r#"package q;
  int z = 5;
endpackage
package p;
  import q::*;
  task tk(output int o); o = 1; z = 7; endtask
endpackage
module t;
  import p::*;
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d qz=%0d", a, b, q::z);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // F2 l2: `let L = z + 0;` used in the block (iverilog has no `let`; sv2v cannot parse it): iverilog `no run`; sv2v → iverilog `no run`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  let L = z + 0;
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      a = L;
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`L`"#)],
    );
    // F2 l1b: a static task returns L: iverilog `no run`; sv2v → iverilog `no run`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  let L = z + 0;
  task tk(output int o);
    o = L;
  endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk` → `L`"#)],
    );
    // F2 l3: `#(L)`: iverilog `no run`; sv2v → iverilog `no run`; verilator `a=6 b=2 mz=5`
    captured(
        r#"module t;
  int z = 5;
  let L = z + 0;
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 2;
      #(L) a = $time;
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`L`"#)],
    );
    // F2 l1: iverilog `no run`; sv2v → iverilog `no run`; verilator `a=5 b=9`
    captured(
        r#"module t;
  int z = 5;
  let L = z + 0;
  task tk(output int o);
    o = L;
  endtask
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk` → `L`"#)],
    );
    // an allow-listed automatic task calling a package task that reads an imported name (round 6 captured it): iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"package q;
  int z = 5;
endpackage
package p;
  import q::*;
  task pt(output int o); o = z; endtask
  function int pf(); return z; endfunction
endpackage
module t;
  import p::*;
  int a, b;
  task automatic tk(output int o); pt(o); endtask
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk` → `pt`"#)],
    );
    // an automatic task calling such a package function (round 6 captured it): iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"package q;
  int z = 5;
endpackage
package p;
  import q::*;
  task pt(output int o); o = z; endtask
  function int pf(); return z; endfunction
endpackage
module t;
  import p::*;
  int a, b;
  task automatic tk(output int o); o = pf(); endtask
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk` → `pf`"#)],
    );
    // a non-void module function calling such a package function (round 6 captured it): iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"package q;
  int z = 5;
endpackage
package p;
  import q::*;
  task pt(output int o); o = z; endtask
  function int pf(); return z; endfunction
endpackage
module t;
  import p::*;
  int a, b;
  function int f(); return pf(); endfunction
  initial begin
    #1 begin
      int z;
      z = 9;
      a = f();
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`f` → `pf`"#)],
    );
    // a static task reached through an automatic task, calling such a package task (round 6 captured it): iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"package q;
  int z = 5;
endpackage
package p;
  import q::*;
  task pt(output int o); o = z; endtask
  function int pf(); return z; endfunction
endpackage
module t;
  import p::*;
  int a, b;
  task st(output int o); pt(o); endtask
  task automatic tk(output int o); st(o); endtask
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk` → `st` → `pt`"#)],
    );
    // i09: the block calls `send_nib`, which reaches `send_bit`; the message gives the path: iverilog `i=2 tot=6 t=75`; sv2v → iverilog `i=2 tot=6 t=75`; verilator `i=2 tot=6 t=75`
    captured(
        r#"module t;
  logic clk = 0;
  always #5 clk = ~clk;
  int tot = 0;
  task send_bit(input bit i);
    @(posedge clk);
    tot = tot + i;
  endtask
  task send_nib(input [3:0] v);
    send_bit(v[0]); send_bit(v[1]); send_bit(v[2]); send_bit(v[3]);
  endtask
  initial begin
    #1 begin
      int i;
      for (i = 0; i < 2; i++) send_nib(4'hB);
      $display("R i=%0d tot=%0d t=%0t", i, tot, $time);
    end
    $finish;
  end
endmodule
"#,
        &[(r#"i"#, r#"`send_nib` → `send_bit`"#)],
    );
    // a package task reading its OWN package variable: right on rounds 5–6, refused by the fail-closed default for package routines; twin A10 (a name the package imports) captured: iverilog `a=5 b=9 mz=5`; sv2v → iverilog `a=5 b=9 mz=5`; verilator `a=5 b=9 mz=5`
    captured(
        r#"package p;
  int z = 5;
  task tk(output int o); o = z; endtask
endpackage
module t;
  import p::*;
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, p::z);
    $finish;
  end
endmodule
"#,
        &[(r#"z"#, r#"`tk`"#)],
    );
    // the same, explicit import of the task; twin A10 captured: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    captured(
        r#"package p;
  int z = 5;
  task pt(output int o); o = z; endtask
endpackage
module t;
  import p::pt;
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      pt(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`pt`"#)],
    );
    // a `let` reached through an automatic task: right on round 6; twin l1b (through a static task) captured: iverilog `no run`; sv2v → iverilog `no run`; verilator `a=5 b=9`
    captured(
        r#"module t;
  int z = 5;
  let L = z + 0;
  int a, b;
  task automatic tk(output int o); o = L; endtask
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk` → `L`"#)],
    );
    // a `let` reached through a non-void function: right on round 6; twin l1b captured: iverilog `no run`; sv2v → iverilog `no run`; verilator `a=5 b=9`
    captured(
        r#"module t;
  int z = 5;
  let L = z + 0;
  int a, b;
  function int f(); return L; endfunction
  initial begin
    #1 begin
      int z;
      z = 9;
      a = f();
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`f` → `L`"#)],
    );
    // a `let` reached through a static task behind an automatic one: right on round 6; twin l1b captured: iverilog `no run`; sv2v → iverilog `no run`; verilator `a=5 b=9`
    captured(
        r#"module t;
  int z = 5;
  let L = z + 0;
  int a, b;
  task st(output int o); o = L; endtask
  task automatic tk(output int o); st(o); endtask
  initial begin
    #1 begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`tk` → `st` → `L`"#)],
    );
    // l4: a function using L: right on round 6; twin l1b captured: iverilog `no run`; sv2v → iverilog `no run`; verilator `a=5 b=9 mz=5`
    captured(
        r#"module t;
  int z = 5;
  let L = z + 0;
  function int f();
    return L;
  endfunction
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      a = f();
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[(r#"z"#, r#"`f` → `L`"#)],
    );
    // p01: a package task whose formal is named like the local: right on round 6 (PRE loud); round 7 differential NOTE 2: iverilog `LOG m0 | LOG m1 | i=2 nmsg=2`; sv2v → iverilog `LOG m0 | LOG m1 | i=2 nmsg=2`; verilator `LOG m0 | LOG m1 | i=2 nmsg=2`
    captured(
        r#"package logp;
  int verbosity = 1;
  int nmsg = 0;
  task automatic log_msg(input string msg);
    nmsg++;
    if (verbosity > 0) $display("R LOG %s", msg);
  endtask
endpackage
module t;
  import logp::*;
  initial begin
    #10 begin
      int i;
      string msg;
      for (i = 0; i < 2; i++) begin
        msg = $sformatf("m%0d", i);
        log_msg(msg);
      end
      $display("R i=%0d nmsg=%0d", i, nmsg);
    end
    $finish;
  end
endmodule
"#,
        &[(r#"msg"#, r#"`log_msg`"#)],
    );
    // p01c: the package's own variable: right on round 6 (PRE wrong): iverilog `LOG hello | total=0 nmsg=1`; sv2v → iverilog `LOG hello | total=0 nmsg=1`; verilator `LOG hello | total=0 nmsg=1`
    captured(
        r#"package logp;
  int verbosity = 1;
  int nmsg = 0;
  task automatic log_msg(input string s);
    nmsg++;
    if (verbosity > 0) $display("R LOG %s", s);
  endtask
endpackage
module t;
  import logp::*;
  int total = 0;
  initial begin
    #10 begin
      int verbosity;
      verbosity = 0;
      log_msg("hello");
      total = verbosity;
    end
    $display("R total=%0d nmsg=%0d", total, nmsg);
    $finish;
  end
endmodule
"#,
        &[(r#"verbosity"#, r#"`log_msg`"#)],
    );
    // p02: a package function's own local `i`: right on round 6 (PRE loud): iverilog `i=4 crc=8e`; sv2v → iverilog `i=4 crc=8e`; verilator `i=4 crc=8e`
    captured(
        r#"package crcp;
  function automatic logic [7:0] crc8(input logic [7:0] d, input logic [7:0] c);
    int i;
    logic fb;
    for (i = 0; i < 8; i++) begin
      fb = c[7] ^ d[7 - i];
      c = {c[6:0], 1'b0} ^ (fb ? 8'h07 : 8'h00);
    end
    return c;
  endfunction
endpackage
module t;
  import crcp::*;
  logic clk = 0;
  always #5 clk = ~clk;
  initial begin
    @(posedge clk) begin
      int i;
      logic [7:0] crc;
      crc = 0;
      for (i = 0; i < 4; i++) crc = crc8(8'hA0 + i, crc);
      $display("R i=%0d crc=%h", i, crc);
    end
    $finish;
  end
endmodule
"#,
        &[(r#"i"#, r#"`crc8`"#)],
    );
    // p02c: the same in a static package function: right on round 6 (PRE loud): iverilog `i=4 crc=8e`; sv2v → iverilog `i=4 crc=8e`; verilator `i=4 crc=8e`
    captured(
        r#"package crcp;
  function logic [7:0] crc8(input logic [7:0] d, input logic [7:0] c);
    int i;
    logic fb;
    for (i = 0; i < 8; i++) begin
      fb = c[7] ^ d[7 - i];
      c = {c[6:0], 1'b0} ^ (fb ? 8'h07 : 8'h00);
    end
    return c;
  endfunction
endpackage
module t;
  import crcp::*;
  logic clk = 0;
  always #5 clk = ~clk;
  initial begin
    @(posedge clk) begin
      int i;
      logic [7:0] crc;
      crc = 0;
      for (i = 0; i < 4; i++) crc = crc8(8'hA0 + i, crc);
      $display("R i=%0d crc=%h", i, crc);
    end
    $finish;
  end
endmodule
"#,
        &[(r#"i"#, r#"`crc8`"#)],
    );
    // p03: UVM-like package helpers: right on round 6 (PRE loud): iverilog `a=0x00001000 | a=0x00001004 | a=0x00001008`; sv2v → iverilog `a=0x00001000 | a=0x00001004 | a=0x00001008`; verilator `a=0x00001000 | a=0x00001004 | a=0x00001008`
    captured(
        r#"package tbh;
  function automatic bit [31:0] aligned(input bit [31:0] base, input int idx);
    return base + idx * 4;
  endfunction
  function automatic string addr2str(input bit [31:0] addr);
    return $sformatf("0x%08h", addr);
  endfunction
endpackage
module t;
  import tbh::*;
  logic clk = 0;
  always #5 clk = ~clk;
  int idx_cnt = 0;
  initial begin
    repeat (3) begin
      @(posedge clk) begin
        bit [31:0] addr;
        int idx;
        idx = idx_cnt;
        addr = aligned(32'h1000, idx);
        $display("R a=%s", addr2str(addr));
        idx_cnt++;
      end
    end
    $finish;
  end
endmodule
"#,
        &[(r#"addr"#, r#"`addr2str`"#), (r#"idx"#, r#"`aligned`"#)],
    );
    // p04: a `let` formal named like the local: right on round 6 (PRE loud): iverilog `no run`; sv2v → iverilog `no run`; verilator `a=5 m=10`
    captured(
        r#"module t;
  let max2(a, b) = (a > b) ? a : b;
  int m = 0;
  initial begin
    #1 begin
      int a;
      a = 5;
      m = max2(a, 3) + max2(1, a);
      $display("R a=%0d m=%0d", a, m);
    end
    $finish;
  end
endmodule
"#,
        &[(r#"a"#, r#"`max2`"#)],
    );
}

/// The untimed twins of F1 / F2 are pre-existing (wrong before row 72 too; ROADMAP §2):
/// known-wrong, the oracle text beside. A hierarchical call into a child that uses a
/// `let` (an allow-listed lane) runs.
#[test]
fn package_and_let_capture_without_a_timing_control_and_the_hierarchical_lane() {
    // KNOWN-WRONG r8_plain, r8 without the timing control: iverilog `a=5 b=9 qz=5`; sv2v → iverilog `a=5 b=9 qz=5`; verilator `a=5 b=9 qz=5`
    exactly(
        r#"package q;
  int z = 5;
endpackage
package p;
  import q::*;
  task tk(output int o);
    o = z;
  endtask
endpackage
module t;
  import p::tk;
  int a, b;
  initial begin
    begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d qz=%0d", a, b, q::z);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[r#"R a=9 b=9 qz=5"#],
    );
    // KNOWN-WRONG r8a_pkg_auto_task_imported_plain, r8a without the timing control: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    exactly(
        r#"package q;
  int z = 5;
endpackage
package p;
  import q::*;
  task automatic tk(output int o); o = z; endtask
endpackage
module t;
  import p::*;
  int a, b;
  initial begin
    begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[r#"R a=9 b=9"#],
    );
    // KNOWN-WRONG r8f_pkg_func_imported_plain, r8f without the timing control: iverilog `a=5 b=9`; sv2v → iverilog `a=5 b=9`; verilator `a=5 b=9`
    exactly(
        r#"package q;
  int z = 5;
endpackage
package p;
  import q::*;
  function int f(); return z; endfunction
endpackage
module t;
  import p::*;
  int a, b;
  initial begin
    begin
      int z;
      z = 9;
      a = f();
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[r#"R a=9 b=9"#],
    );
    // KNOWN-WRONG l2_let_direct_plain, l2 without the timing control: iverilog `no run`; sv2v → iverilog `no run`; verilator `a=5 b=9 mz=5`
    exactly(
        r#"module t;
  int z = 5;
  let L = z + 0;
  int a, b;
  initial begin
    begin
      int z;
      z = 9;
      a = L;
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[r#"R a=9 b=9 mz=5"#],
    );
    // KNOWN-WRONG l1b_plain, l1b without the timing control: iverilog `no run`; sv2v → iverilog `no run`; verilator `a=5 b=9 mz=5`
    exactly(
        r#"module t;
  int z = 5;
  let L = z + 0;
  task tk(output int o);
    o = L;
  endtask
  int a, b;
  initial begin
    begin
      int z;
      z = 9;
      tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d mz=%0d", a, b, z);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[r#"R a=9 b=9 mz=5"#],
    );
    // A9_hier_child_uses_let, a hierarchical call of a child task that uses the child's `let`: iverilog `no run`; sv2v → iverilog `no run`; verilator `a=5 b=9`
    exactly(
        r#"module sub;
  int z = 5;
  let L = z + 0;
  task tk(output int o); o = L; endtask
endmodule
module t;
  sub u();
  int a, b;
  initial begin
    #1 begin
      int z;
      z = 9;
      u.tk(a);
      b = z;
    end
    $display("R a=%0d b=%0d", a, b);
    $finish;
  end
  initial #40 begin $display("R timeout"); $finish; end
endmodule
"#,
        &[r#"R a=5 b=9"#],
    );
}

/// Ruled after round 7's differential review (main): p07a was right before the row only
/// by accident; its one-edit twin p07b was wrong before the row. Both are refused.
#[test]
fn ruled_cell_of_the_round_seven_review_is_refused() {
    // ruled p07a_modvar_i_pkg_crc_local_i: PRE `i=4 crc=8e n0=2`; iverilog `i=4 crc=8e n0=2`; sv2v → iverilog `i=4 crc=8e n0=2`; verilator `i=4 crc=8e n0=2` (twin p07b_twin_modi_read_after: PRE `i=4 crc=8e n0=2 | modi=4`; iverilog `i=4 crc=8e n0=2 | modi=2`; sv2v → iverilog `i=4 crc=8e n0=2 | modi=2`; verilator `i=4 crc=8e n0=2 | modi=2`)
    captured(
        r#"package crcp;
  function automatic logic [7:0] crc8(input logic [7:0] d, input logic [7:0] c);
    int i;
    logic fb;
    for (i = 0; i < 8; i++) begin
      fb = c[7] ^ d[7 - i];
      c = {c[6:0], 1'b0} ^ (fb ? 8'h07 : 8'h00);
    end
    return c;
  endfunction
endpackage
module t;
  import crcp::*;
  logic clk = 0;
  always #5 clk = ~clk;
  integer i;
  logic [7:0] crc = 0;
  int n0 = 0;
  initial for (i = 0; i < 2; i = i + 1) n0 = n0 + 1;
  initial begin
    @(posedge clk) begin
      int i;
      for (i = 0; i < 4; i++) crc = crc8(8'hA0 + i, crc);
      $display("R i=%0d crc=%h n0=%0d", i, crc, n0);
    end
    $finish;
  end
endmodule
"#,
        &[(r#"i"#, r#"`crc8`"#)],
    );
    // twin p07b_twin_modi_read_after: PRE `i=4 crc=8e n0=2 | modi=4`; iverilog `i=4 crc=8e n0=2 | modi=2`; sv2v → iverilog `i=4 crc=8e n0=2 | modi=2`; verilator `i=4 crc=8e n0=2 | modi=2`
    captured(
        r#"package crcp;
  function automatic logic [7:0] crc8(input logic [7:0] d, input logic [7:0] c);
    int i;
    logic fb;
    for (i = 0; i < 8; i++) begin
      fb = c[7] ^ d[7 - i];
      c = {c[6:0], 1'b0} ^ (fb ? 8'h07 : 8'h00);
    end
    return c;
  endfunction
endpackage
module t;
  import crcp::*;
  logic clk = 0;
  always #5 clk = ~clk;
  integer i;
  logic [7:0] crc = 0;
  int n0 = 0;
  initial for (i = 0; i < 2; i = i + 1) n0 = n0 + 1;
  initial begin
    @(posedge clk) begin
      int i;
      for (i = 0; i < 4; i++) crc = crc8(8'hA0 + i, crc);
      $display("R i=%0d crc=%h n0=%0d", i, crc, n0);
    end
    #1 $display("R modi=%0d", t.i);
    $finish;
  end
endmodule
"#,
        &[(r#"i"#, r#"`crc8`"#)],
    );
}

/// The same task bodies without a timing control above the block are untouched.
#[test]
fn task_blocks_without_a_timing_control_still_run() {
    exactly(
        "module t;\n  logic [31:0] x = 32'h11111111;\n  task tk;\n    begin\n      logic [7:0] x;\n      x = 8'hAA;\n      $display(\"R inner=%h\", x);\n    end\n  endtask\n  initial #1 tk();\n  initial begin #5 $display(\"R outer=%h\", x); $finish; end\nendmodule\n",
        &["R inner=aa", "R outer=11111111"],
    );
    exactly(
        "module t;\n  task automatic tk();\n    #1;\n    begin\n      int c = 5;\n      c = c + 1;\n      $display(\"R c=%0d\", c);\n    end\n  endtask\n  initial begin tk(); tk(); end\n  initial #5 $finish;\nendmodule\n",
        &["R c=6", "R c=6"],
    );
}

/// `%m` in a named block whose local shadows a module variable: `t.blk`, not the internal
/// `t.$blk$<n>.blk` (the control after a bare `#1;` printed that too, and now does not).
#[test]
fn percent_m_skips_the_internal_block_scope() {
    for src in [
        "module t;\n  logic [31:0] x = 32'h11111111;\n  initial #1 begin : blk\n    logic [7:0] x;\n    x = 8'h1;\n    $display(\"R m=%m x=%h\", x);\n  end\n  initial #5 $finish;\nendmodule\n",
        "module t;\n  logic [7:0] x;\n  initial begin\n    #1;\n    begin : blk\n      logic [7:0] x;\n      x = 8'h1;\n      $display(\"R m=%m x=%h\", x);\n    end\n  end\n  initial #5 $finish;\nendmodule\n",
    ] {
        exactly(src, &["R m=t.blk x=01"]);
    }
}

/// Two processes' same-named locals under a timing control share one flattened net; a
/// `$strobe` / `$monitor` of one renders later, after the other block's write (iverilog,
/// sv2v and verilator `a=aa`, `m=01` only) — refused where every block on the net is
/// under a timing control (before row 72 each such design was E3010). A pair with one
/// plain block keeps its previous behaviour (p02: `a=aa`, all three).
#[test]
fn a_deferred_read_of_a_shared_timing_local_is_refused() {
    for task in ["$strobe(\"R a=%h\", x);", "$monitor(\"R a=%h\", x);"] {
        loud(
            &format!("module t;\n  initial #1 begin\n    logic [7:0] x;\n    x = 8'hAA;\n    {task}\n  end\n  initial #3 begin\n    logic [7:0] x;\n    x = 8'hBB;\n  end\n  initial #5 $finish;\nendmodule\n"),
            &["error[VITA-E3009]", "shares one flattened net", "`$strobe` / `$monitor`-family"],
        );
    }
    exactly(
        "module t;\n  initial begin\n    #1;\n    begin\n      logic [7:0] x;\n      x = 8'hAA;\n      $strobe(\"R a=%h t=%0t\", x, $time);\n    end\n  end\n  initial #5 begin\n    logic [7:0] x;\n    x = 8'hBB;\n    $display(\"R b=%h t=%0t\", x, $time);\n  end\n  initial #9 $finish;\nendmodule\n",
        &["R a=aa t=1", "R b=bb t=5"],
    );
    // An immediate read in the same pair runs: the definite-assignment gate covers it.
    exactly(
        "module t;\n  initial #1 begin\n    logic [7:0] x;\n    x = 8'hAA;\n    $display(\"R a=%h\", x);\n  end\n  initial #3 begin\n    logic [7:0] x;\n    x = 8'hBB;\n    $display(\"R b=%h\", x);\n  end\n  initial #5 $finish;\nendmodule\n",
        &["R a=aa", "R b=bb"],
    );
}

/// An `automatic` local in a `fork … join_none` arm a loop spawns again while the first
/// is live would share one net (verilator `x=100 x=200`) — refused. The `join` loop, a
/// `wait fork` before the next spawn (o8), an arm block that cannot wait (p20), and the
/// static local (one variable by definition) run.
#[test]
fn an_automatic_local_live_twice_is_refused() {
    let src = |join: &str, lifetime: &str| {
        format!("module t;\n  int n = 0;\n  initial begin\n    for (int j = 0; j < 2; j++) begin\n      fork\n        #1 begin\n          {lifetime}int x;\n          n = n + 1;\n          x = n * 100;\n          #3 $display(\"R x=%0d t=%0t\", x, $time);\n        end\n      {join}\n      #2;\n    end\n    #20 $finish;\n  end\nendmodule\n")
    };
    loud(
        &src("join_none", "automatic "),
        &[
            "error[VITA-E3009]",
            "an `automatic` block-local `x` in a `fork … join_none`",
        ],
    );
    exactly(&src("join", "automatic "), &["R x=100 t=4", "R x=200 t=10"]);
    exactly(
        &src("join_none\n      wait fork;", "automatic "),
        &["R x=100 t=4", "R x=200 t=10"],
    );
    // A static local is one variable: both activations print the last write (all three).
    exactly(&src("join_none", ""), &["R x=200 t=4", "R x=200 t=6"]);
    // p20: the arm's block runs from entry to exit without waiting (verilator).
    exactly(
        "module t;\n  int x;\n  int n = 0;\n  initial begin\n    repeat (2) begin\n      fork\n        #1 begin\n          automatic int x;\n          n = n + 1;\n          x = n;\n          $display(\"R x=%0d t=%0t\", x, $time);\n        end\n      join_none\n      #2;\n    end\n  end\n  initial #20 $finish;\nendmodule\n",
        &["R x=1 t=1", "R x=2 t=3"],
    );
}

/// A child's local under a timing control named like its module variable does not make
/// the parent's `u.hs` lose that variable's width (the hierarchical size census keeps its
/// walk): `16'(u.hs * q8)` is `00000120` (all three), in a case selector too.
#[test]
fn a_hierarchical_width_keeps_the_module_variable() {
    let child = "module sub;\n  logic signed [7:0] hs = 8'sd16;\n  initial #5 begin\n    logic [3:0] hs;\n    hs = 4'h3;\n  end\nendmodule\n";
    exactly(
        &format!("{child}module t;\n  sub u();\n  logic [7:0] q8 = 8'd18;\n  logic [31:0] o1, o2;\n  initial begin\n    #2;\n    o1 = 16'(u.hs * q8);\n    o2 = u.hs * q8;\n    $display(\"R o1=%h o2=%h\", o1, o2);\n    $finish;\n  end\nendmodule\n"),
        &["R o1=00000120 o2=00000120"],
    );
    exactly(
        &format!("{child}module t;\n  sub u();\n  logic signed [7:0] sq8 = 8'sd18;\n  logic [31:0] o1;\n  initial begin\n    #2;\n    case (16'(u.hs * sq8))\n      16'h0120: o1 = 1;\n      default: o1 = 2;\n    endcase\n    $display(\"R o1=%0d\", o1);\n    $finish;\n  end\nendmodule\n"),
        &["R o1=1"],
    );
}

/// A `do … while` body under a timing control declares like any block (a0, a1); the
/// desugar's second copy of a body whose local shadows a module variable is refused as a
/// duplicate, as its no-timing twin is (ROADMAP §2; one such design ran by accident).
#[test]
fn do_while_timing_bodies() {
    exactly(
        "module t;\n  int n = 0;\n  initial do #1 begin\n    logic [7:0] y;\n    y = 8'hA0 + n;\n    n++;\n    $display(\"R inner=%h\", y);\n  end while (n < 2);\n  initial begin #9 $display(\"R done\"); $finish; end\nendmodule\n",
        &["R inner=a0", "R inner=a1", "R done"],
    );
    loud(
        "module t;\n  logic [7:0] x;\n  initial begin\n    do #1 begin\n      logic [7:0] x;\n      x = 8'hAA;\n      $display(\"R inner=%h\", x);\n    end while (0);\n    $display(\"R done\");\n  end\n  initial #5 $finish;\nendmodule\n",
        &["error[VITA-E3009]", "redeclared (duplicate declaration)"],
    );
}
