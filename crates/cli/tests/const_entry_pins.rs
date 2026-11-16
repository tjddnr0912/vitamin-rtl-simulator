//! The whole stdout, stderr (diagnostic code, text, line:col) and exit code of the cells that show
//! what the parser's constant-table entry is (ROADMAP §5.2 PARSE-1: `ConstEntry { v, w, signed, lane }`
//! in hdl-parser, written at one place, `param_item_to_module_item`, and compared at one place, the
//! wildcard-import ambiguity in `imports.rs`, by `val()` = the three fields it compared before the lane
//! existed). Every expected text was measured on PRE (W/batch5/pre/bin/vita-post, md5 4d1d84bb, the tree
//! before PARSE-1) with `t.sv` as the argument. The oracle text in each doc is iverilog 13.0 (`iverilog
//! -g2012`) and verilator 5.054 (`--binary --timing`) verbatim (W/parse1b/ground/cells,
//! W/parse1b/premise2/cells2) ; where vita's output differs from an oracle's, the pin holds vita's as PRE
//! prints it (structure only: no fix here). A named mutant killer here is a claim until its mutant runs.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// (stdout, stderr, exit code) of `vita t.sv` in a temp dir of its own.
fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("vita_const_entry_pins_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&dir)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&dir);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

fn check(id: &str, src: &str, want_stdout: &str, want_stderr: &str, want_rc: i32) {
    let (so, se, rc) = run(src);
    assert_eq!(so, want_stdout, "{id}: stdout");
    assert_eq!(se, want_stderr, "{id}: stderr");
    assert_eq!(rc, Some(want_rc), "{id}: exit code");
}

/// Two wildcard imports of `N`, both `localparam int N = 4`: equal entries, the name stays and the
/// struct member's range folds (cell `w_wild_same_typed`).
/// Oracles: iverilog 13.0: compile rc=1, `t.sv:10: error: Ambiguous use of 'N'. It is exported by both 'p' and by 'q'.` ;
/// verilator 5.054: compile rc=0, run `A b=4`. (vita keeps the name where iverilog refuses it: PRE's behaviour, pinned as is.)
#[test]
fn w_wild_same_typed() {
    check(
        "w_wild_same_typed",
        r##"package p;
  localparam int N = 4;
endpackage
package q;
  localparam int N = 4;
endpackage
module top;
  import p::*;
  import q::*;
  typedef struct packed { logic [N-1:0] a; } s_t;
  s_t v;
  initial begin #1 $display("A b=%0d", $bits(v)); #1 $finish; end
  initial #1000 $finish;
endmodule
"##,
        r##"A b=4
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// The same with `q`'s `N` untyped (`localparam N = 4`): p's entry is `Converted`, q's `NoTypeNoRange`,
/// their `val()` is equal (4, 32 bits, signed), so the first package's entry and its lane are kept and the
/// output is `w_wild_same_typed`'s (cell `w_wild_same_mixed`). Named killer of mutant n3 (the compare
/// includes the lane: the name would be dropped and the member range would not fold).
/// Oracles: iverilog 13.0: compile rc=1, `t.sv:10: error: Ambiguous use of 'N'. It is exported by both 'p' and by 'q'.` ;
/// verilator 5.054: compile rc=0, run `A b=4`.
#[test]
fn w_wild_same_mixed() {
    check(
        "w_wild_same_mixed",
        r##"package p;
  localparam int N = 4;
endpackage
package q;
  localparam N = 4;
endpackage
module top;
  import p::*;
  import q::*;
  typedef struct packed { logic [N-1:0] a; } s_t;
  s_t v;
  initial begin #1 $display("A b=%0d", $bits(v)); #1 $finish; end
  initial #1000 $finish;
endmodule
"##,
        r##"A b=4
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// The control: `q`'s `N` is 5, the values differ, the name is dropped and the struct member's range
/// does not fold (cell `w_wild_diff_ctl`).
/// Oracles: iverilog 13.0: compile rc=1, `t.sv:10: error: Ambiguous use of 'N'. It is exported by both 'p' and by 'q'.` ;
/// verilator 5.054: compile rc=0, run `A b=4`.
#[test]
fn w_wild_diff_ctl() {
    check(
        "w_wild_diff_ctl",
        r##"package p;
  localparam int N = 4;
endpackage
package q;
  localparam int N = 5;
endpackage
module top;
  import p::*;
  import q::*;
  typedef struct packed { logic [N-1:0] a; } s_t;
  s_t v;
  initial begin #1 $display("A b=%0d", $bits(v)); #1 $finish; end
  initial #1000 $finish;
endmodule
"##,
        r##""##,
        r##"t.sv:10:27: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or a constant-literal range in v1 (a `[E:0]` range over a header parameter is laid out per instance), found identifier 's_t'
errors=1 warnings=0 notes=0
"##,
        1,
    );
}

/// `localparam string S = 5`: the parser consumes the `string` keyword into `ParamType::Implicit`, so W1
/// records the entry as `NoTypeNoRange` with the decimal's value and the range `[S:0]` folds (cell
/// `k8_lp_string_int`, premise judge 2 line 11). The record of this behaviour is the existing
/// docs/PROBE_CATALOG.md:41 row (a `string`-typed parameter is its literal in an integral context) ; no new
/// entry.
/// Oracles: iverilog 13.0: compile rc=1, `t.sv:2: error: The expression ''sd5' cannot be implicitly cast to the target type.` /
/// `1 error(s) during elaboration.` ; verilator 5.054: compile rc=1,
/// `%Error: Internal Error: ../V3Number.h:249: \`num\` member accessed when data type is STRING`.
#[test]
fn k8_lp_string_int() {
    check(
        "k8_lp_string_int",
        r##"module top;
  localparam string S = 5;
  typedef struct packed { logic [S:0] a; } s_t;
  s_t s;
  initial begin $display("A b=%0d", $bits(s)); end
  initial #50 $finish;
endmodule
"##,
        r##"A b=6
simulation ended (Finish) at time 50
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}
