//! A block-local that SHADOWS a module net and sits in a same-NAME NESTING —
//! `begin int s; begin int s; … end; s = …; end` over a module `int s` — put the
//! OUTER block's write on the MODULE net. `MOD=41` where both oracles print `MOD=0`.
//! 12 measured silent-wrong shapes; every one of them is pinned below.
//!
//! ## Root
//!
//! `frames_reserve.rs::gather_auto_block_locals` admits a declaring span for THREE
//! independent reasons — `automatic`, dynamic storage (§4.5.249), and shadowing a
//! module-scope name — and used to record all three under ONE bool,
//! `widened = d.lifetime != Some(true)`. That bool is true for a static shadow and
//! for a dynamic-storage widening alike.
//!
//! `block_local_class.rs`'s filter A then drops any *widened* span that ENCLOSES
//! another declaring span of the same name (review S3: a widening must not withdraw
//! scoping an `automatic` pair already had). Dropping a widened dynamic-storage span
//! is harmless — its flatten target is a fresh net of its own. Dropping a SHADOW span
//! is not: `block_local/hoist.rs` records that a shadow's flatten target IS the
//! shadowed module net. So the outer declaration fell through to the flatten and its
//! writes landed on the parent's state, observable by a hierarchical reference and by
//! `$dumpvars`.
//!
//! The span now carries an `AdmitReason { widened, shadows_module }` instead of the
//! bool, and a name whose declaring spans are ALL static shadows is exempt from filter
//! A *and* from the §4.5.259 same-name nesting filter. Both exemptions are needed: the
//! nesting filter drops BOTH members of a nesting pair, so fixing only filter A would
//! have lost the INNER scope as well and leaked the inner write too (c24 below is the
//! cell that measures this). Nesting is safe because the Nets-phase hoist nests its
//! `$blk$` segments exactly as the Logic-phase lowering does (R16 §3.4), so the two
//! levels become `…$blk$<outer>.s` and `…$blk$<outer>.$blk$<inner>.s`.
//!
//! A name with an `automatic` or dynamic-storage span keeps the pre-existing filters:
//! those spans carry a per-entry lifetime requirement, which is what the filters were
//! built to protect and which the loud E3009 gate is the authority on. `c04`/`c12`/
//! `c13` (the `automatic` shapes) are still loud, unchanged.
//!
//! ## Oracles
//!
//! Every expectation below is iverilog 13's, measured; verilator 5.052 agrees on all
//! of them except `c25`, where the two disagree only about the INITIAL value of an
//! unwritten net (iverilog `xxxx` = 4-state, verilator `0` = 2-state). vita is
//! 4-state, so `xxxx` is iverilog's answer and the agreed fact is that the module net
//! is never written.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_blsn_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// A clean run whose output contains every `want` line, verbatim as observed.
fn expect(src: &str, want: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
}

/// The tail every `probe` design shares: sample the module net one step after the
/// writes, then stop.
const TAIL: &str = r#"  initial begin
    #1 $display("MOD=%0h", s);
    #1 $finish;
  end
endmodule
"#;

/// A design with a module-scope `int s` and `body`, followed by [`TAIL`].
fn probe(body: &str) -> String {
    format!("module top;\n  int s;\n{body}{TAIL}")
}

// ── the 12 silent-wrong cells: the OUTER write must not reach the module net ──

/// c02 — the base shape. Two nested labelled blocks, both `int s`, over a module
/// `int s`. PRE `MOD=41` (the outer write on the module net); both oracles `MOD=0`.
#[test]
fn c02_two_nested_static_ints_leave_the_module_net_alone() {
    expect(
        &probe(
            r#"  initial begin
    begin : b0
      int s;
      begin : b1
        int s;
        s = 8'h42;
      end
      s = 8'h41;
    end
  end
"#,
        ),
        &["MOD=0"],
    );
}

/// c03 — three levels where the MIDDLE one changes type (`int` / `string` / `int t`).
/// The nesting is what matters, not the types agreeing. PRE `MOD=41`.
#[test]
fn c03_three_levels_with_a_string_in_the_middle() {
    expect(
        &probe(
            r#"  initial begin
    begin : b0
      int s;
      begin : b1
        string s;
        begin : b2
          int t;
          t = 1;
        end
        s = "x";
      end
      s = 8'h41;
    end
  end
"#,
        ),
        &["MOD=0"],
    );
}

/// c08 — `string s` at both levels. The string kind is admitted by the §4.5.249
/// dynamic-storage rule AS WELL as by the shadow rule, which is why an exemption
/// keyed on "shadow is the ONLY reason" would have missed this cell. PRE `MOD=78`
/// (the leaked `"x"`).
#[test]
fn c08_two_nested_strings() {
    expect(
        &probe(
            r#"  initial begin
    begin : b0
      string s;
      begin : b1
        string s;
        s = "y";
      end
      s = "x";
    end
  end
"#,
        ),
        &["MOD=0"],
    );
}

/// c09 — `logic [7:0]` locals over an `int` net: a narrower local, so the leak also
/// carried the wrong width. PRE `MOD=41`.
#[test]
fn c09_two_nested_packed_vectors() {
    expect(
        &probe(
            r#"  initial begin
    begin : b0
      logic [7:0] s;
      begin : b1
        logic [7:0] s;
        s = 8'h42;
      end
      s = 8'h41;
    end
  end
"#,
        ),
        &["MOD=0"],
    );
}

/// c11 — UNLABELLED blocks. The scope segment is keyed on `span.lo`, not on a label,
/// so neither level needs a name to be scoped. PRE `MOD=41`.
#[test]
fn c11_unlabelled_nested_blocks() {
    expect(
        &probe(
            r#"  initial begin
    begin
      int s;
      begin
        int s;
        s = 8'h42;
      end
      s = 8'h41;
    end
  end
"#,
        ),
        &["MOD=0"],
    );
}

/// c14 — the same nesting inside an `always @(posedge clk)` rather than an `initial`.
/// PRE `MOD=41`.
#[test]
fn c14_inside_an_always_block() {
    expect(
        r#"module top;
  int s;
  reg clk = 0;
  always @(posedge clk) begin : b0
    int s;
    begin : b1
      int s;
      s = 8'h42;
    end
    s = 8'h41;
  end
  initial begin
    #1 clk = 1;
    #1 $display("MOD=%0h", s);
    #1 $finish;
  end
endmodule
"#,
        &["MOD=0"],
    );
}

/// c17 — the nesting under a `fork…join` arm. `gather_auto_block_locals` recurses
/// through `Fork`, so the arm's blocks are seen. PRE `MOD=41`.
#[test]
fn c17_inside_a_fork_arm() {
    expect(
        &probe(
            r#"  initial begin
    fork
      begin : b0
        int s;
        begin : b1
          int s;
          s = 8'h42;
        end
        s = 8'h41;
      end
    join
  end
"#,
        ),
        &["MOD=0"],
    );
}

/// c18 — the nesting as a `for` loop BODY, so the outer block runs twice. PRE `MOD=41`.
#[test]
fn c18_as_a_for_loop_body() {
    expect(
        &probe(
            r#"  initial begin
    for (int i = 0; i < 2; i++) begin : b0
      int s;
      begin : b1
        int s;
        s = 8'h42;
      end
      s = 8'h41;
    end
  end
"#,
        ),
        &["MOD=0"],
    );
}

/// c20 — the shadowed module-scope name is an output PORT. This is the cell that
/// shows the leak ESCAPING the module: the parent reads the port. PRE `MOD=41`.
#[test]
fn c20_the_shadowed_name_is_an_output_port() {
    expect(
        r#"module dut(output int s);
  initial begin : b0
    int s;
    begin : b1
      int s;
      s = 8'h42;
    end
    s = 8'h41;
  end
endmodule
module top;
  int s;
  dut u(.s(s));
  initial begin
    #1 $display("MOD=%0h", s);
    #1 $finish;
  end
endmodule
"#,
        &["MOD=0"],
    );
}

/// c21 — c02 plus a READ of `s` in the outer block after its write. Both lines are
/// pinned: the outer block must see its OWN `41`, and the module net must stay `0`.
/// PRE printed `INNERSCOPE=41` (right) with `MOD=41` (wrong); iverilog prints exactly
/// the two lines below.
#[test]
fn c21_the_outer_block_reads_back_its_own_local() {
    expect(
        r#"module top;
  int s;
  initial begin : b0
    int s;
    begin : b1
      int s;
      s = 8'h42;
    end
    s = 8'h41;
    $display("INNERSCOPE=%0h", s);
  end
  initial begin
    #1 $display("MOD=%0h", s);
    #1 $finish;
  end
endmodule
"#,
        &["INNERSCOPE=41", "MOD=0"],
    );
}

/// c23 — one outer `s` with TWO DISJOINT inner blocks each declaring `s`. Three spans
/// and two nesting relations, so the outer span encloses more than one sibling.
/// PRE `MOD=41`.
#[test]
fn c23_one_outer_over_two_sibling_inners() {
    expect(
        r#"module top;
  int s;
  initial begin : b0
    int s;
    begin : b1
      int s;
      s = 8'h42;
    end
    begin : b2
      int s;
      s = 8'h43;
    end
    s = 8'h41;
  end
  initial begin
    #1 $display("MOD=%0h", s);
    #1 $finish;
  end
endmodule
"#,
        &["MOD=0"],
    );
}

/// c25 — a NARROWER local (`logic [3:0]`) over a WIDER module net (`logic [15:0]`).
/// PRE printed `MOD=d`, i.e. the leaked write read back at the module net's width.
///
/// The pin is `MOD=xxxx`, which is what vita prints POST and what iverilog prints: the
/// module net is never written, so it holds its 4-state initial value. verilator
/// prints `0` for the same net because it is 2-state. The two oracles AGREE on the
/// fact under test — the net is unwritten — and differ only in how an unwritten
/// `logic [15:0]` reads out; vita is 4-state, so iverilog is the oracle here.
#[test]
fn c25_a_narrower_local_over_a_wider_net_leaves_it_unwritten() {
    expect(
        r#"module top;
  logic [15:0] s;
  initial begin : b0
    logic [3:0] s;
    begin : b1
      logic [3:0] s;
      s = 4'hE;
    end
    s = 4'hD;
  end
  initial begin
    #1 $display("MOD=%0h", s);
    #1 $finish;
  end
endmodule
"#,
        &["MOD=xxxx"],
    );
}

// ── cells that were already correct and must not move ──────────────────────

/// c05 — only the OUTER block shadows (`s` outside, `q` inside). `s` has ONE declaring
/// span, so no nesting relation exists and neither filter ever applied. `MOD=0` before
/// and after. The axis is same-NAME nesting, not nesting.
#[test]
fn c05_only_the_outer_block_shadows() {
    expect(
        &probe(
            r#"  initial begin
    begin : b0
      int s;
      begin : b1
        int q;
        q = 8'h42;
      end
      s = 8'h41;
    end
  end
"#,
        ),
        &["MOD=0"],
    );
}

/// c06 — only the INNER block shadows (`q` outside, `s` inside). The mirror of c05.
/// `MOD=0` before and after.
#[test]
fn c06_only_the_inner_block_shadows() {
    expect(
        &probe(
            r#"  initial begin
    begin : b0
      int q;
      begin : b1
        int s;
        s = 8'h41;
      end
      q = 8'h42;
    end
  end
"#,
        ),
        &["MOD=0"],
    );
}

/// c22 — the same nesting inside a `generate if (1)` body. `for_each_proc` walks
/// generate contents, and the branch-PATH machinery (§4.5.259) that decides whether
/// two spans can coexist is untouched by the exemption. `MOD=0` before and after.
#[test]
fn c22_inside_a_generate_if_body() {
    expect(
        r#"module top;
  int s;
  generate
    if (1) begin : g
      initial begin : b0
        int s;
        begin : b1
          int s;
          s = 8'h42;
        end
        s = 8'h41;
      end
    end
  endgenerate
  initial begin
    #1 $display("MOD=%0h", s);
    #1 $finish;
  end
endmodule
"#,
        &["MOD=0"],
    );
}

/// c24 — the nesting with NO outer write, so nothing leaked even PRE (the inner block
/// was already the only scoped span).
///
/// ⭐ This is the cell that measures the SECOND exemption. The §4.5.259 nesting filter
/// drops BOTH members of a nesting pair, so exempting only filter A would have kept
/// the outer span, put the two in a nesting relation, dropped both, and taken the
/// INNER scope away — regressing this cell from `MOD=0` to `MOD=42`. A fix for the
/// outer leak that leaks the inner instead is not a fix.
#[test]
fn c24_inner_write_only_stays_scoped() {
    expect(
        r#"module top;
  int s;
  initial begin : b0
    int s;
    begin : b1
      int s;
      s = 8'h42;
    end
  end
  initial begin
    #1 $display("MOD=%0h", s);
    #1 $finish;
  end
endmodule
"#,
        &["MOD=0"],
    );
}

// ── two cells this slice did NOT target, which the fix nonetheless opened ───

/// c07 — three levels ALL declaring `s`. This was loud (E3009) and is now a value.
///
/// It is a loud → correct-value move, not a widening chased on purpose: c07 is c02 at
/// one more depth, and every rule that scopes c02's two static shadows scopes c07's
/// three — there is no non-arbitrary predicate that separates them. Both oracles print
/// `MOD=0` and vita now agrees, so the new answer is measured, not assumed. Pinned so
/// the move stays visible.
#[test]
fn c07_three_levels_all_shadowing_was_loud_and_is_now_correct() {
    expect(
        &probe(
            r#"  initial begin
    begin : b0
      int s;
      begin : b1
        int s;
        begin : b2
          int s;
          s = 8'h43;
        end
        s = 8'h42;
      end
      s = 8'h41;
    end
  end
"#,
        ),
        &["MOD=0"],
    );
}

/// c26 — two nested DYNAMIC (`int s[]`) shadows. Also loud (E3009) before and a value
/// now, for the same reason as c07: both spans are static, so both are shadow-admitted
/// and both are exempt. Both oracles print `MOD=0`. The dynamic-storage family proper
/// is a separate slice; this pin only records the state the fix left this shape in.
#[test]
fn c26_two_nested_dynamic_shadows_was_loud_and_is_now_correct() {
    expect(
        r#"module top;
  int s;
  initial begin : b0
    int s[];
    begin : b1
      int s[];
      s = new[1];
    end
    s = new[2];
  end
  initial begin
    #1 $display("MOD=%0h", s);
    #1 $finish;
  end
endmodule
"#,
        &["MOD=0"],
    );
}

// ── the `automatic` shapes stay LOUD ───────────────────────────────────────

/// c04 — both levels `automatic int s = 0`. A span admitted by `automatic` carries a
/// per-entry lifetime requirement, which is exactly what the two nesting filters were
/// built to protect, so the exemption deliberately does NOT apply and the pre-existing
/// loud stands. iverilog rejects the lifetime override outright ("sorry: Overriding
/// the default variable lifetime is not yet supported"), so verilator is the only
/// oracle with a value here — one oracle is not enough to move a loud.
#[test]
fn c04_both_levels_automatic_stays_loud() {
    let (o, _) = run(r#"module top;
  int s;
  initial begin
    begin : b0
      automatic int s = 0;
      begin : b1
        automatic int s = 0;
        s = 8'h42;
      end
      s = 8'h41;
    end
  end
  initial begin
    #1 $display("MOD=%0h", s);
    #1 $finish;
  end
endmodule
"#);
    assert!(o.contains("VITA-E3009"), "E3009 expected:\n{o}");
    assert!(!o.contains("MOD="), "must not reach simulation:\n{o}");
}
