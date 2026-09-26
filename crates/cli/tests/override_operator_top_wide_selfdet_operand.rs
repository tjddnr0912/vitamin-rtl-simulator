//! A ≤64-bit OPERATOR-topped override over a wide SELF-determined sub-node binds its own
//! width (ROADMAP §2 "Index sealing", the operator-top bullet; §6.20.2).
//!
//! `override_self_meta` — the i64 operator channel's `(width, sign)` for an untyped target —
//! fenced the whole tree with `const_ctx_within_i64`, which refused ANY node wider than 64
//! bits, including one in a self-determined position whose width never enters the context
//! (a shift count, a `**` exponent, a comparison's operands, a ternary condition, a
//! reduction's operand). So `#(.P(8'd1 << 128'd2))` onto `parameter P = 1` bound 32 bits
//! (the default literal's) with the right value, where both oracles bind 8 bits `04`. The
//! fence now walks only the context-determined operands (`const_ctx_within_i64_context`);
//! the value channel already read those positions through the wide domain's bridges.
//! A comparison whose operand the i64 lane cannot hold now reaches the wide domain from the
//! plain walk too (`8'd3 + (128'h1_0000_0000_0000_0000 > 128'd1)` is 4; the override was
//! E3009 on every channel and target). Oracles: iverilog 13.0 `-g2012`, verilator 5.052
//! `--binary --timing`; where they split on the WIDTH of a `+` (iverilog one bit wider —
//! its §4.5.466 self-contradiction, its own localparam of the same text agreeing with
//! verilator) the pin is verilator's, which vita's `localparam` lane already gives.
//! Typed targets are unchanged: the i64 route folds them in the target's context (iverilog's
//! side of the ROADMAP §2 row 16 split) and the loud→value cells there are two-oracle.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_any(src: &str) -> (bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_ovsd_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), s)
}

fn run(src: &str) -> String {
    let (ok, s) = run_any(src);
    assert!(ok, "expected exit 0, got:\n{s}");
    s
}

fn loud(src: &str, code: &str) -> String {
    let (ok, s) = run_any(src);
    assert!(
        !ok && s.contains(code),
        "expected a {code} rejection, got:\n{s}"
    );
    s
}

/// PRE: 32 bits on every one (`ashr` fffffffc, `not1` fffffffe, `powe` 00000008, `shcnt` 00000004, `tern` 00000002, `wname` 00000004); `cmpw*` E3009. `addcmp` / `k2` / `mixed` / `nest` / `red` / `redw` / `wcmp` / `dbl` / `clog` are verilator's width (iverilog one bit wider on `+`; `dbl` iverilog 2/2, verilator 1/0).
#[test]
fn an_untyped_target_binds_the_tree_own_width_over_a_wide_self_determined_sub_node() {
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T shcnt bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(8'd1 << 128'd2)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T shcnt bits=8 hex=04"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T not1 bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(~(128'd1 != 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T not1 bits=1 hex=0"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T ashr bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P((-8'sd8) >>> 128'sd1)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ashr bits=8 hex=fc"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T powe bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(8'd2 ** 128'd3)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T powe bits=8 hex=08"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T tern bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(128'd0 ? 8'd1 : 8'd2)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T tern bits=8 hex=02"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T ternw bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(128'h1_0000_0000_0000_0000 ? 8'd1 : 8'd2)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ternw bits=8 hex=01"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T wname bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(8'd1 << W)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T wname bits=8 hex=04"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T cmpw bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(8'd3 + (128'h1_0000_0000_0000_0000 > 128'd1))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T cmpw bits=8 hex=04"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T cmpw2 bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(8'd3 + (128'hFFFF_FFFF_FFFF_FFFF_0 == 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T cmpw2 bits=8 hex=03"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T cmpw3 bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(8'd3 + (128'h1_0000_0000_0000_0001 == 128'd1))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T cmpw3 bits=8 hex=03"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T addcmp bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(8'hFF + (128'd1 > 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T addcmp bits=8 hex=00"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T k2 bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P((8'hFF + 8'd1) + 16'd0 + (128'd1 > 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T k2 bits=16 hex=0101"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T mixed bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(16'd5 + (128'd7 > 128'd6) + 8'd1)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T mixed bits=16 hex=0007"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T nest bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P((8'd1 << (128'd1 + 128'd1)) + 8'd0)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T nest bits=8 hex=04"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T red bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(8'hF0 + (|128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T red bits=8 hex=f0"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T redw bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(8'hF0 + (|128'h1_0000_0000_0000_0000))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T redw bits=8 hex=f1"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T wcmp bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(8'hFF + (W > 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T wcmp bits=8 hex=00"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T dbl bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P((128'd1 > 128'd0) + (128'd2 > 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T dbl bits=1 hex=0"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T clog bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P($clog2(128'd8) + 8'd0)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T clog bits=32 hex=00000003"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T ctrl bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(8'd1 << 2)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ctrl bits=8 hex=04"));
}

/// The four channels share `override_self_meta`; a `$bits(P)`-sized net inside the child (`c_*`) shows the bound width reaching a declaration. PRE: 32 bits everywhere, `cmpw` E3009.
#[test]
fn the_defparam_positional_and_interface_channels_and_a_downstream_width_consumer_agree() {
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T shcnt bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  su u();\n  defparam u.P = 8'd1 << 128'd2;\n  initial begin #2 $finish; end\nendmodule\n").contains("T shcnt bits=8 hex=04"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T not1 bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  su u();\n  defparam u.P = ~(128'd1 != 128'd0);\n  initial begin #2 $finish; end\nendmodule\n").contains("T not1 bits=1 hex=0"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T ashr bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  su u();\n  defparam u.P = (-8'sd8) >>> 128'sd1;\n  initial begin #2 $finish; end\nendmodule\n").contains("T ashr bits=8 hex=fc"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T powe bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  su u();\n  defparam u.P = 8'd2 ** 128'd3;\n  initial begin #2 $finish; end\nendmodule\n").contains("T powe bits=8 hex=08"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T tern bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  su u();\n  defparam u.P = 128'd0 ? 8'd1 : 8'd2;\n  initial begin #2 $finish; end\nendmodule\n").contains("T tern bits=8 hex=02"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T cmpw bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  su u();\n  defparam u.P = 8'd3 + (128'h1_0000_0000_0000_0000 > 128'd1);\n  initial begin #2 $finish; end\nendmodule\n").contains("T cmpw bits=8 hex=04"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T shcnt bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  su #(8'd1 << 128'd2) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T shcnt bits=8 hex=04"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T not1 bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  su #(~(128'd1 != 128'd0)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T not1 bits=1 hex=0"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T ashr bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  su #((-8'sd8) >>> 128'sd1) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ashr bits=8 hex=fc"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T powe bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  su #(8'd2 ** 128'd3) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T powe bits=8 hex=08"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T tern bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  su #(128'd0 ? 8'd1 : 8'd2) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T tern bits=8 hex=02"));
    assert!(run("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T cmpw bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  su #(8'd3 + (128'h1_0000_0000_0000_0000 > 128'd1)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T cmpw bits=8 hex=04"));
    assert!(run("interface ifc #(parameter P = 1) ();\n  initial begin #1; $display(\"T shcnt bits=%0d hex=%h\", $bits(P), P); end\nendinterface\nmodule top;\n  ifc #(.P(8'd1 << 128'd2)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T shcnt bits=8 hex=04"));
    assert!(run("interface ifc #(parameter P = 1) ();\n  initial begin #1; $display(\"T not1 bits=%0d hex=%h\", $bits(P), P); end\nendinterface\nmodule top;\n  ifc #(.P(~(128'd1 != 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T not1 bits=1 hex=0"));
    assert!(run("interface ifc #(parameter P = 1) ();\n  initial begin #1; $display(\"T ashr bits=%0d hex=%h\", $bits(P), P); end\nendinterface\nmodule top;\n  ifc #(.P((-8'sd8) >>> 128'sd1)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ashr bits=8 hex=fc"));
    assert!(run("interface ifc #(parameter P = 1) ();\n  initial begin #1; $display(\"T powe bits=%0d hex=%h\", $bits(P), P); end\nendinterface\nmodule top;\n  ifc #(.P(8'd2 ** 128'd3)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T powe bits=8 hex=08"));
    assert!(run("interface ifc #(parameter P = 1) ();\n  initial begin #1; $display(\"T tern bits=%0d hex=%h\", $bits(P), P); end\nendinterface\nmodule top;\n  ifc #(.P(128'd0 ? 8'd1 : 8'd2)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T tern bits=8 hex=02"));
    assert!(run("interface ifc #(parameter P = 1) ();\n  initial begin #1; $display(\"T cmpw bits=%0d hex=%h\", $bits(P), P); end\nendinterface\nmodule top;\n  ifc #(.P(8'd3 + (128'h1_0000_0000_0000_0000 > 128'd1))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T cmpw bits=8 hex=04"));
    assert!(run("module su #(parameter P = 1) ();\n  localparam W = $bits(P);\n  logic [W-1:0] v = '1;\n  initial begin #1; $display(\"T shcnt w=%0d v=%h\", W, v); end\nendmodule\nmodule top;\n  su #(.P(8'd1 << 128'd2)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T shcnt w=8 v=ff"));
    assert!(run("module su #(parameter P = 1) ();\n  localparam W = $bits(P);\n  logic [W-1:0] v = '1;\n  initial begin #1; $display(\"T not1 w=%0d v=%h\", W, v); end\nendmodule\nmodule top;\n  su #(.P(~(128'd1 != 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T not1 w=1 v=1"));
    assert!(run("module su #(parameter P = 1) ();\n  localparam W = $bits(P);\n  logic [W-1:0] v = '1;\n  initial begin #1; $display(\"T ashr w=%0d v=%h\", W, v); end\nendmodule\nmodule top;\n  su #(.P((-8'sd8) >>> 128'sd1)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ashr w=8 v=ff"));
    assert!(run("module su #(parameter P = 1) ();\n  localparam W = $bits(P);\n  logic [W-1:0] v = '1;\n  initial begin #1; $display(\"T powe w=%0d v=%h\", W, v); end\nendmodule\nmodule top;\n  su #(.P(8'd2 ** 128'd3)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T powe w=8 v=ff"));
    assert!(run("module su #(parameter P = 1) ();\n  localparam W = $bits(P);\n  logic [W-1:0] v = '1;\n  initial begin #1; $display(\"T tern w=%0d v=%h\", W, v); end\nendmodule\nmodule top;\n  su #(.P(128'd0 ? 8'd1 : 8'd2)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T tern w=8 v=ff"));
    assert!(run("module su #(parameter P = 1) ();\n  localparam W = $bits(P);\n  logic [W-1:0] v = '1;\n  initial begin #1; $display(\"T cmpw w=%0d v=%h\", W, v); end\nendmodule\nmodule top;\n  su #(.P(8'd3 + (128'h1_0000_0000_0000_0000 > 128'd1))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T cmpw w=8 v=ff"));
}

/// Typed `logic [15:0]` cells: every PRE value kept (the four `+`-over-a-comparison cells are row 16's split, iverilog's side); `cmpw*` were E3009 and both oracles agree on them.
#[test]
fn a_typed_target_is_unchanged_and_its_comparison_cells_fold() {
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T shcnt bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(8'd1 << 128'd2)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T shcnt bits=16 hex=0004"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T not1 bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(~(128'd1 != 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T not1 bits=16 hex=fffe"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T ashr bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P((-8'sd8) >>> 128'sd1)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ashr bits=16 hex=fffc"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T powe bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(8'd2 ** 128'd3)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T powe bits=16 hex=0008"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T tern bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(128'd0 ? 8'd1 : 8'd2)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T tern bits=16 hex=0002"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T ternw bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(128'h1_0000_0000_0000_0000 ? 8'd1 : 8'd2)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ternw bits=16 hex=0001"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T wname bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(8'd1 << W)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T wname bits=16 hex=0004"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T addcmp bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(8'hFF + (128'd1 > 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T addcmp bits=16 hex=0100"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T k2 bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P((8'hFF + 8'd1) + 16'd0 + (128'd1 > 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T k2 bits=16 hex=0101"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T mixed bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(16'd5 + (128'd7 > 128'd6) + 8'd1)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T mixed bits=16 hex=0007"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T nest bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P((8'd1 << (128'd1 + 128'd1)) + 8'd0)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T nest bits=16 hex=0004"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T red bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(8'hF0 + (|128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T red bits=16 hex=00f0"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T redw bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(8'hF0 + (|128'h1_0000_0000_0000_0000))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T redw bits=16 hex=00f1"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T wcmp bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(8'hFF + (W > 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T wcmp bits=16 hex=0100"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T dbl bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P((128'd1 > 128'd0) + (128'd2 > 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T dbl bits=16 hex=0002"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T clog bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P($clog2(128'd8) + 8'd0)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T clog bits=16 hex=0003"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T ctrl bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(8'd1 << 2)) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T ctrl bits=16 hex=0004"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T cmpw bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(8'd3 + (128'h1_0000_0000_0000_0000 > 128'd1))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T cmpw bits=16 hex=0004"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T cmpw2 bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(8'd3 + (128'hFFFF_FFFF_FFFF_FFFF_0 == 128'd0))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T cmpw2 bits=16 hex=0003"));
    assert!(run("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T cmpw3 bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(8'd3 + (128'h1_0000_0000_0000_0001 == 128'd1))) u();\n  initial begin #2 $finish; end\nendmodule\n").contains("T cmpw3 bits=16 hex=0003"));
}

/// Not this slice: a shift COUNT past 63 on a non-zero value declines in the width-unlimited
/// lane (`1 << 70` does not fit an i64 — the "correct a value, never create one" accept set
/// of ROADMAP §2 row 30), so `32'd1 << 128'd70` and `8'd1 << 128'h1_0000_0000_0000_0000`
/// stay E3009 as an override on both targets where both oracles bind 0; the `localparam`
/// twins of the same text already fold to 0 through the width-aware walk.
#[test]
fn a_shift_count_past_the_i64_lane_keeps_the_override_loud() {
    loud("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T sh70 bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(32'd1 << 128'd70)) u();\n  initial begin #2 $finish; end\nendmodule\n", "E3009");
    loud("module su #(parameter P = 1) ();\n  initial begin #1; $display(\"T shbig bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  su #(.P(8'd1 << 128'h1_0000_0000_0000_0000)) u();\n  initial begin #2 $finish; end\nendmodule\n", "E3009");
    loud("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T sh70 bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(32'd1 << 128'd70)) u();\n  initial begin #2 $finish; end\nendmodule\n", "E3009");
    loud("module st #(parameter logic [15:0] P = 1) ();\n  initial begin #1; $display(\"T shbig bits=%0d hex=%h\", $bits(P), P); end\nendmodule\nmodule top;\n  parameter logic [127:0] W = 128'd2;\n  st #(.P(8'd1 << 128'h1_0000_0000_0000_0000)) u();\n  initial begin #2 $finish; end\nendmodule\n", "E3009");
    assert!(run("module top;\n  parameter logic [127:0] W = 128'd2;\n  localparam L = 32'd1 << 128'd70;\n  initial begin #1; $display(\"T sh70 bits=%0d hex=%h\", $bits(L), L); $finish; end\nendmodule\n").contains("T sh70 bits=32 hex=00000000"));
    assert!(run("module top;\n  parameter logic [127:0] W = 128'd2;\n  localparam L = 8'd1 << 128'h1_0000_0000_0000_0000;\n  initial begin #1; $display(\"T shbig bits=%0d hex=%h\", $bits(L), L); $finish; end\nendmodule\n").contains("T shbig bits=8 hex=00"));
}
