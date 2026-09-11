//! Two same-named block-locals assigned only by a DECLARATION INITIALIZER
//! (`begin int s = 44; … end` beside `begin int s = 55; … end`) were refused by the
//! read-before-assign E3009 guard. Both oracles run them and print `o1=44 o2=55`.
//!
//! ## Mechanism
//!
//! `frames_reserve.rs::gather_auto_block_locals` admitted a declaring span for three
//! reasons — `automatic`, dynamic storage, shadowing a module name — so a plain
//! static `int s = 44;` was none of them and both declarations flattened onto ONE
//! module net; the name landed in `coalesced_block_locals` and `block_local/hoist.rs`
//! asked `block_local_definitely_assigned(stmts, …)`, a walk over STATEMENTS only,
//! which cannot see a declaration's initializer, so the write reads as absent.
//! The fix is the STORAGE classifier, not the guard: a fourth admission rule (a
//! static declarator carrying an initializer, decl-ANY) plus the same term in the
//! decl-ANY twin in `hoist.rs`, so each declaration earns its own `$blk$` net and
//! there is no sharing left for the guard to protect against.
//!
//! Counting the initializer inside the GUARD instead would have been a
//! loud→silent-wrong: a static initializer runs ONCE at t0, not on block entry
//! (`c92_static_initializer_runs_once_at_t0` below measures the `6,7,8,9` ladder), so
//! on one shared net the LAST initializer of the name is the only one that runs and
//! both readers would print `44`/`55` from the same cell.
//!
//! `AdmitReason` carries the new reason as a third field, and the two candidacy
//! filters in `block_local_class.rs` (review S3's enclosing-widened drop and the
//! §4.5.259 nesting drop) exempt a name whose EVERY declaring span is such a static
//! initialised declaration — homogeneous, so a name with any `automatic`,
//! dynamic-storage or shadow span keeps the pre-existing path byte for byte.
//!
//! ## Oracles
//!
//! Every value pinned here was measured 3-way: vita, iverilog 13 (`-g2012`) and
//! verilator 5.052 (`--binary --timing`), all three identical. The exceptions are
//! noted at their test: `c86` is a vita-only residue (both oracles disagree with it),
//! and `c40`'s loud is vita's own representation, not an oracle verdict.
//!
//! ## What stays LOUD, and why
//!
//! An INITIALIZER-FREE pair where one block READS the name before assigning it
//! (`c40_initializer_free_read_before_assign_stays_loud`). Those declarations carry
//! no initializer, so the new rule does not admit them, they still share one
//! flattened net, and the reader would observe the other block's leftover value.
//! Both oracles give that block its own storage and print `o2=0`; vita's flatten
//! cannot, so correct-or-loud is the E3009. That is the guard's whole protected
//! class and it is unchanged.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_blsi_{}_{n}", std::process::id()));
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
fn lines(src: &str, want: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
}

// ── (1) sibling static-initialised pairs and triples, by body kind and type ──

/// c01 — the base shape: two LABELLED `initial` blocks, `int s = 44/55`.
#[test]
fn c01_two_labelled_initial_siblings_of_one_name() {
    lines(
        r#"module t;
  int o1, o2;
  initial begin : b1
    int s = 44;
    o1 = s;
  end
  initial begin : b2
    int s = 55;
    o2 = s;
  end
  initial begin #2 $display("o1=%0d o2=%0d", o1, o2); $finish; end
endmodule
"#,
        &["o1=44 o2=55"],
    );
}

/// c02 — the same pair with UNLABELLED blocks: the scope segment is minted from the
/// span, not from a label.
#[test]
fn c02_two_unlabelled_initial_siblings_of_one_name() {
    lines(
        r#"module t;
  int o1, o2;
  initial begin
    int s = 44;
    o1 = s;
  end
  initial begin
    int s = 55;
    o2 = s;
  end
  initial begin #2 $display("o1=%0d o2=%0d", o1, o2); $finish; end
endmodule
"#,
        &["o1=44 o2=55"],
    );
}

/// c03 — a packed 4-state type rather than `int`.
#[test]
fn c03_two_siblings_of_one_name_packed_logic() {
    lines(
        r#"module t;
  logic [7:0] o1, o2;
  initial begin : b1
    logic [7:0] s = 8'h2c;
    o1 = s;
  end
  initial begin : b2
    logic [7:0] s = 8'h37;
    o2 = s;
  end
  initial begin #2 $display("o1=%0h o2=%0h", o1, o2); $finish; end
endmodule
"#,
        &["o1=2c o2=37"],
    );
}

/// c05 — `real`, a separate storage kind from the packed lanes.
#[test]
fn c05_two_siblings_of_one_name_real() {
    lines(
        r#"module t;
  real o1, o2;
  initial begin : b1
    real s = 4.5;
    o1 = s;
  end
  initial begin : b2
    real s = 5.5;
    o2 = s;
  end
  initial begin #2 $display("o1=%0.2f o2=%0.2f", o1, o2); $finish; end
endmodule
"#,
        &["o1=4.50 o2=5.50"],
    );
}

/// c06 — THREE siblings, so the candidacy set is larger than the two-span bar.
#[test]
fn c06_three_siblings_of_one_name() {
    lines(
        r#"module t;
  int o1, o2, o3;
  initial begin : b1
    int s = 44;
    o1 = s;
  end
  initial begin : b2
    int s = 55;
    o2 = s;
  end
  initial begin : b3
    int s = 66;
    o3 = s;
  end
  initial begin #2 $display("o1=%0d o2=%0d o3=%0d", o1, o2, o3); $finish; end
endmodule
"#,
        &["o1=44 o2=55 o3=66"],
    );
}

/// c07 — two `always @(posedge clk)` bodies: the initializer still runs once at t0,
/// and each block reads its own net on every edge.
#[test]
fn c07_two_edge_triggered_siblings_of_one_name() {
    lines(
        r#"module t;
  int o1, o2; reg clk = 0;
  always #1 clk = ~clk;
  always @(posedge clk) begin : b1
    int s = 44;
    o1 = s;
  end
  always @(posedge clk) begin : b2
    int s = 55;
    o2 = s;
  end
  initial begin #5 $display("o1=%0d o2=%0d", o1, o2); $finish; end
endmodule
"#,
        &["o1=44 o2=55"],
    );
}

/// c08 — two `always_comb` bodies.
#[test]
fn c08_two_always_comb_siblings_of_one_name() {
    lines(
        r#"module t;
  int o1, o2; logic g = 1;
  always_comb begin : b1
    int s = 44;
    o1 = g ? s : 0;
  end
  always_comb begin : b2
    int s = 55;
    o2 = g ? s : 0;
  end
  initial begin #2 $display("o1=%0d o2=%0d", o1, o2); $finish; end
endmodule
"#,
        &["o1=44 o2=55"],
    );
}

/// c09 — two `for` BODIES, each iterated twice, accumulating: `88`/`110` is two
/// iterations of `44`/`55`, so the initializer is not re-run per iteration either.
#[test]
fn c09_two_for_body_siblings_of_one_name_accumulate() {
    lines(
        r#"module t;
  int o1, o2;
  initial begin
    for (int i = 0; i < 2; i++) begin : b1
      int s = 44;
      o1 = o1 + s;
    end
  end
  initial begin
    for (int i = 0; i < 2; i++) begin : b2
      int s = 55;
      o2 = o2 + s;
    end
  end
  initial begin #2 $display("o1=%0d o2=%0d", o1, o2); $finish; end
endmodule
"#,
        &["o1=88 o2=110"],
    );
}

// ── (2) if/else arms — the ROADMAP row's claim ⓑ, 2-oracle ──

/// c20 — mutually exclusive UNLABELLED arms, the `if` arm taken.
#[test]
fn c20_if_else_arms_unlabelled_then_arm() {
    lines(
        r#"module t;
  int o1; logic c = 1;
  initial begin
    if (c) begin
      int s = 44;
      o1 = s;
    end else begin
      int s = 55;
      o1 = s;
    end
  end
  initial begin #2 $display("o1=%0d", o1); $finish; end
endmodule
"#,
        &["o1=44"],
    );
}

/// c21 — the same arms LABELLED.
#[test]
fn c21_if_else_arms_labelled_then_arm() {
    lines(
        r#"module t;
  int o1; logic c = 1;
  initial begin
    if (c) begin : a1
      int s = 44;
      o1 = s;
    end else begin : a2
      int s = 55;
      o1 = s;
    end
  end
  initial begin #2 $display("o1=%0d", o1); $finish; end
endmodule
"#,
        &["o1=44"],
    );
}

/// c22 — the ELSE arm taken, packed type: the other arm's initializer must not be
/// what the taken arm reads.
#[test]
fn c22_if_else_arms_else_arm_taken() {
    lines(
        r#"module t;
  logic [7:0] o1; logic c = 0;
  initial begin
    if (c) begin
      logic [7:0] s = 8'h2c;
      o1 = s;
    end else begin
      logic [7:0] s = 8'h37;
      o1 = s;
    end
  end
  initial begin #2 $display("o1=%0h", o1); $finish; end
endmodule
"#,
        &["o1=37"],
    );
}

// ── (3) widened nesting — the filters' exemption is what makes these run ──

/// c30 — depth 3, every level declaring the name with an initializer. Without the
/// exemption in BOTH filters the nesting loses every scope and the name falls below
/// the two-span bar.
#[test]
fn c30_depth_three_nesting_of_one_name() {
    lines(
        r#"module t;
  int o1, o2, o3;
  initial begin : L1
    int s = 11;
    o1 = s;
    begin : L2
      int s = 22;
      o2 = s;
      begin : L3
        int s = 33;
        o3 = s;
      end
    end
  end
  initial begin #2 $display("o1=%0d o2=%0d o3=%0d", o1, o2, o3); $finish; end
endmodule
"#,
        &["o1=11 o2=22 o3=33"],
    );
}

/// c31 — depth 2, the same shape.
#[test]
fn c31_depth_two_nesting_of_one_name() {
    lines(
        r#"module t;
  int o1, o2;
  initial begin : L1
    int s = 11;
    o1 = s;
    begin : L2
      int s = 22;
      o2 = s;
    end
  end
  initial begin #2 $display("o1=%0d o2=%0d", o1, o2); $finish; end
endmodule
"#,
        &["o1=11 o2=22"],
    );
}

// ── (4) the class that stays LOUD ──

/// c40 — the guard's protected class: an INITIALIZER-FREE pair where `b2` reads `s`
/// before assigning it. No initializer, so the new admission rule does not reach
/// these declarations; they still share one flattened net and `b2` would read `b1`'s
/// leftover `44`. Both oracles give `b2` its own storage and print `o1=44 o2=0`;
/// vita's flatten cannot, so it stays loud. The wording is pinned because it is the
/// user's only account of what to do (assign before use, or rename).
#[test]
fn c40_initializer_free_read_before_assign_stays_loud() {
    let (o, code) = run(r#"module t;
  int o1, o2;
  initial begin : b1
    int s;
    s = 44;
    o1 = s;
  end
  initial begin : b2
    int s;
    o2 = s;
  end
  initial begin #2 $display("o1=%0d o2=%0d", o1, o2); $finish; end
endmodule
"#);
    assert_eq!(code, Some(1), "expected a loud refusal:\n{o}");
    assert!(o.contains("VITA-E3009"), "expected E3009 in:\n{o}");
    assert!(
        o.contains("is READ before it is assigned here"),
        "expected the read-before-assign wording in:\n{o}"
    );
    assert!(
        o.contains("assign it before use, or rename one"),
        "expected the remedy wording in:\n{o}"
    );
}

// ── (5) the four dynamic-storage kinds beside a static-initialised same-name
//        sibling: already scoped by the §4.5.249/§4.5.251 rule, must not move ──

/// c60 — `string`, one side initialised.
#[test]
fn c60_string_pair_unchanged() {
    lines(
        r#"module t;
  string o1, o2;
  initial begin : b1
    string s;
    s = "aa";
    o1 = s;
  end
  initial begin : b2
    string s = "bb";
    o2 = s;
  end
  initial begin #2 $display("o1=%s o2=%s", o1, o2); $finish; end
endmodule
"#,
        &["o1=aa o2=bb"],
    );
}

/// c61 — dynamic array.
#[test]
fn c61_dynamic_array_pair_unchanged() {
    lines(
        r#"module t;
  int o1, o2;
  initial begin : b1
    int s[];
    s = new[2];
    s[0] = 7;
    o1 = s[0];
  end
  initial begin : b2
    int s[] = '{9, 9};
    o2 = s[1];
  end
  initial begin #2 $display("o1=%0d o2=%0d", o1, o2); $finish; end
endmodule
"#,
        &["o1=7 o2=9"],
    );
}

/// c62 — queue.
#[test]
fn c62_queue_pair_unchanged() {
    lines(
        r#"module t;
  int o1, o2;
  initial begin : b1
    int s[$];
    s.push_back(7);
    o1 = s[0];
  end
  initial begin : b2
    int s[$] = '{9, 9};
    o2 = s[1];
  end
  initial begin #2 $display("o1=%0d o2=%0d", o1, o2); $finish; end
endmodule
"#,
        &["o1=7 o2=9"],
    );
}

// ── (6) shadow nesting: a module net of the same name must stay untouched ──

/// c70 — module `int s`, outer block initialised, inner block written. `MOD=0`.
#[test]
fn c70_shadow_nesting_outer_initialised() {
    lines(
        r#"module t;
  int s;
  int oi;
  initial begin : L1
    int s = 8'h41;
    begin : L2
      int s;
      s = 9;
      oi = s;
    end
  end
  initial begin #2 $display("MOD=%0d INNER=%0d", s, oi); $finish; end
endmodule
"#,
        &["MOD=0 INNER=9"],
    );
}

/// c71 — the same without any initializer: a name with a shadow span keeps the
/// shadow path, which the new rule must not disturb.
#[test]
fn c71_shadow_nesting_no_initialiser() {
    lines(
        r#"module t;
  int s;
  int oi;
  initial begin : L1
    int s;
    s = 8'h41;
    begin : L2
      int s;
      s = 9;
      oi = s;
    end
  end
  initial begin #2 $display("MOD=%0d INNER=%0d", s, oi); $finish; end
endmodule
"#,
        &["MOD=0 INNER=9"],
    );
}

/// c72 — both levels initialised and the outer read AFTER the inner block: the outer
/// value survives the inner block and the module net is still untouched.
#[test]
fn c72_shadow_nesting_both_levels_initialised() {
    lines(
        r#"module t;
  int s;
  int oi, oo;
  initial begin : L1
    int s = 8'h41;
    begin : L2
      int s = 9;
      oi = s;
    end
    oo = s;
  end
  initial begin #2 $display("MOD=%0d INNER=%0d OUTER=%0d", s, oi, oo); $finish; end
endmodule
"#,
        &["MOD=0 INNER=9 OUTER=65"],
    );
}

/// c73 — the innermost declaration RENAMED, so only the outer shadows the module net.
#[test]
fn c73_shadow_nesting_inner_renamed() {
    lines(
        r#"module t;
  int s;
  int oi, oo;
  initial begin : L1
    int s = 8'h41;
    begin : L2
      int u = 9;
      oi = u;
    end
    oo = s;
  end
  initial begin #2 $display("MOD=%0d INNER=%0d OUTER=%0d", s, oi, oo); $finish; end
endmodule
"#,
        &["MOD=0 INNER=9 OUTER=65"],
    );
}

/// c73d — the one-token discriminator: make the innermost span DOUBLE-admitted
/// (a `string` with an initializer is both the dynamic-storage rule and the new
/// static-initialiser rule) so no single admission rule owns it alone. The module net
/// must still be untouched.
#[test]
fn c73d_double_admitted_innermost_span_leaves_the_module_net_alone() {
    lines(
        r#"module t;
  int s;
  int oi, oo;
  initial begin : L1
    int s = 8'h41;
    begin : L2
      string u = "9";
      oi = u.len();
    end
    oo = s;
  end
  initial begin #2 $display("MOD=%0d INNER=%0d OUTER=%0d", s, oi, oo); $finish; end
endmodule
"#,
        &["MOD=0 INNER=1 OUTER=65"],
    );
}

/// c72d — the same discriminator with the inner span keeping the SHADOWED name, so
/// the innermost span is admitted by three rules at once.
#[test]
fn c72d_double_admitted_inner_shadow_leaves_the_module_net_alone() {
    lines(
        r#"module t;
  int s;
  int oi, oo;
  initial begin : L1
    int s = 8'h41;
    begin : L2
      string s = "9";
      oi = s.len();
    end
    oo = s;
  end
  initial begin #2 $display("MOD=%0d INNER=%0d OUTER=%0d", s, oi, oo); $finish; end
endmodule
"#,
        &["MOD=0 INNER=1 OUTER=65"],
    );
}

// ── (7) the t0-once ladder and the frame-path residue ──

/// c92 — a static initializer runs ONCE at t0, not on block entry: four posedges
/// print `6,7,8,9`, not `6,6,6,6`. This is the measurement that refutes fixing the
/// symptom inside the read-before-assign guard, and all three tools agree on it.
#[test]
fn c92_static_initializer_runs_once_at_t0() {
    lines(
        r#"module t;
  reg clk = 0;
  always #1 clk = ~clk;
  always @(posedge clk) begin : b
    int s = 5;
    s = s + 1;
    $display("s=%0d", s);
  end
  initial begin #9 $finish; end
endmodule
"#,
        &["s=6", "s=7", "s=8", "s=9"],
    );
}

/// c86 — NOT a pass. Two sibling labelled blocks inside ONE STATIC task body still
/// coalesce onto one net, so both readers see the LAST initializer: vita prints
/// `o1=55 o2=55` where both oracles print `o1=44 o2=55`. That is a recorded ROADMAP
/// §2 residue on a different storage path (`frames_reserve.rs::
/// reserve_frame_block_locals`, the frame path), which this slice does not touch. It
/// is pinned at its measured value so the day the frame path is fixed this test fails
/// and is updated rather than the regression going unnoticed. Two separate static
/// tasks, a `task automatic`, and a re-entry ladder are all correct today.
#[test]
fn c86_sibling_blocks_in_one_static_task_body_are_a_recorded_residue() {
    lines(
        r#"module t;
  int o1, o2;
  task tt;
    begin : b1
      int s = 44;
      o1 = s;
    end
    begin : b2
      int s = 55;
      o2 = s;
    end
  endtask
  initial begin tt; #2 $display("o1=%0d o2=%0d", o1, o2); $finish; end
endmodule
"#,
        &["o1=55 o2=55"],
    );
}
