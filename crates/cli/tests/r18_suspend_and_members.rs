//! Round-18: a callee that advances time, struct member writes, and the silent-wrong
//! the first of those uncovered.
//!
//! Oracle: iverilog 13 live where the shape is expressible there (it rejects an
//! explicit `automatic` lifetime override, so the sharing pins use plain static
//! block-locals, which it accepts and which is the shape that actually shares a net).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("vita_r18_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .arg("--timeout")
        .arg("400")
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

// ── §3.1: a callee that suspends is still analysable ─────────────────────────

/// The report's §3.1, verbatim in shape: a clocked driver task called before the
/// caller's later block-local is assigned. `stmt_no_ref_deep` had no arm for
/// `@(posedge clk)`, and its `_ => false` means "may reference ANY name", so one
/// timing control in a callee made every later local in the caller unusable — 11 of
/// the report's 12 diagnostics.
#[test]
fn a_callee_that_suspends_does_not_end_the_walk() {
    let (o, ok) = run(r#"module t;
        logic clk = 0; always #5 clk = ~clk;
        task automatic preload (input int addr, input byte m []);
          @(posedge clk);
          for (int i = 0; i < m.size(); i++) @(posedge clk);
        endtask
        initial begin
          begin
            automatic byte msg   [] = '{8'h61};
            automatic byte ports [];
            preload(1, msg);
            ports = new[2];
            if (ports.size() == 2) $display("PASS");
          end
          $finish;
        end
      endmodule"#);
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("PASS"), "got:\n{o}");
}

/// Every suspend form a driver task actually uses, one call each.
#[test]
fn each_suspend_form_in_a_callee_is_vetted() {
    for body in [
        "@(posedge clk);",
        "#1;",
        "wait (clk == 1);",
        "@(posedge clk) x <= 1;",
        "begin @(posedge clk); #1; end",
    ] {
        let (o, ok) = run(&format!(
            r#"module t;
                 logic clk = 0; int x; always #5 clk = ~clk;
                 task automatic tick(); {body} endtask
                 initial begin
                   begin
                     automatic byte ports [];
                     tick();
                     ports = new[2];
                     if (ports.size() == 2) $display("PASS");
                   end
                   $finish;
                 end
               endmodule"#
        ));
        assert!(ok && o.contains("PASS"), "body `{body}` got:\n{o}");
    }
}

// ── R18-X1: the silent-wrong the §3.1 fix would have widened ─────────────────

/// SOUNDNESS PIN — a pre-existing silent-wrong, measured identical at `c8ad2b4` and
/// `46b9816`: vita printed `A v=99`, iverilog prints `A v=1`, at exit 0.
///
/// Two same-named STATIC block-locals share one flattened net. Block A writes 1,
/// calls a task that suspends, and reads back; while A is parked, B writes 99 to the
/// one net. Real `automatic`/per-scope storage keeps A's 1.
///
/// It survived R17's shared-net rule twice over: the rule read only SYNTACTIC timing
/// (so a one-line `tick()` wrapper hid the suspend), and the top-level walk returned
/// early once the local was assigned (so the rule was never even consulted). Both are
/// closed; this must stay loud.
#[test]
fn a_suspending_call_on_a_shared_net_is_loud() {
    let (o, ok) = run(r#"module t;
        logic clk = 0; always #5 clk = ~clk;
        task automatic tick(); @(posedge clk); endtask
        initial begin
          begin int v; v = 1; tick(); $display("A v=%0d", v); end
        end
        initial begin #2; begin int v; v = 99; end end
        initial #100 $finish;
      endmodule"#);
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("E3009"), "got:\n{o}");
    assert!(o.contains("time can advance here"), "got:\n{o}");
    assert!(
        !o.contains("A v=99"),
        "the silent-wrong value escaped:\n{o}"
    );
}

/// The same hazard written inline was already loud and must not regress — it is the
/// pin that proved the R17 rule was load-bearing.
#[test]
fn an_inline_suspend_on_a_shared_net_stays_loud() {
    let (o, ok) = run(r#"module t;
        logic clk = 0; always #5 clk = ~clk;
        initial begin begin int v; v = 1; @(posedge clk); $display("A v=%0d", v); end end
        initial begin #2; begin int v; v = 99; end end
        initial #100 $finish;
      endmodule"#);
    assert!(!ok, "expected a diagnostic:\n{o}");
    assert!(o.contains("E3009"), "got:\n{o}");
}

/// …but a suspend on a FRESH net changes nothing, so it must stay accepted: there is
/// no other writer to hand the scheduler to.
#[test]
fn a_suspending_call_on_a_fresh_net_is_accepted() {
    let (o, ok) = run(r#"module t;
        logic clk = 0; always #5 clk = ~clk;
        task automatic tick(); @(posedge clk); endtask
        initial begin
          begin int v; v = 1; tick(); $display("A v=%0d", v); end
          $finish;
        end
      endmodule"#);
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("A v=1"), "got:\n{o}");
}

// ── §3.2: a member write that covers the whole variable ──────────────────────

/// The report's §3.2: `rm.c = 5;` on a single-member struct writes ALL of `rm`, yet
/// the walk called it "only PART (a select)". A struct member is a constant
/// part-select after the parser's desugar, so the rule is bit coverage.
#[test]
fn a_member_write_covering_every_bit_is_a_whole_write() {
    let (o, ok) = run(r#"module t;
        typedef struct { int c; } rec_t;
        initial begin
          begin int pad; rec_t rm; rm.c = 5; if (rm.c == 5) $display("PASS"); pad = 0; end
          begin rec_t rm; rm.c = 6; if (rm.c == 6) $display("ok"); end
          $finish;
        end
      endmodule"#);
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("PASS") && o.contains("ok"), "got:\n{o}");
}

/// Field by field reaches full coverage the same way — the general rule, not a
/// single-member special case.
#[test]
fn two_member_writes_together_cover_the_variable() {
    let (o, ok) = run(r#"module t;
        typedef struct { int c; int d; } rec_t;
        initial begin
          begin rec_t rm; rm.c = 5; rm.d = 6; if (rm.c == 5) $display("PASS2"); end
          begin rec_t rm; rm.c = 7; rm.d = 8; if (rm.c == 7) $display("ok2"); end
          $finish;
        end
      endmodule"#);
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("PASS2") && o.contains("ok2"), "got:\n{o}");
}

/// SOUNDNESS PIN. PARTIAL coverage must stay loud — writing one member of a
/// two-member struct leaves the other holding the sibling block's leftover.
#[test]
fn partial_member_coverage_stays_loud() {
    let (o, ok) = run(r#"module t;
        typedef struct { int c; int d; } rec_t;
        initial begin
          begin rec_t rm; rm.c = 5; $display("A %0d", rm.d); end
          begin rec_t rm; rm.c = 7; rm.d = 8; end
          $finish;
        end
      endmodule"#);
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("E3009"), "got:\n{o}");
}

// ── §3.3 root: `automatic` in front of an unpacked-struct type ───────────────

/// `automatic rec_t r;` was silently downgraded to STATIC: the automatic-lifetime
/// parse helper cannot resolve an unpacked-struct type name, and the member fan-out
/// that then parsed the declaration stamped no lifetime. So two same-named struct
/// locals in disjoint blocks shared one flattened net, while the identical
/// `automatic int` / enum / typedef-alias pair each got its own `$blk$` scope.
///
/// With the lifetime preserved they are two variables, and block A reads the fresh
/// default exactly as iverilog does for per-scope storage.
#[test]
fn an_automatic_unpacked_struct_local_keeps_its_lifetime() {
    let (o, ok) = run(r#"module t;
        typedef struct { int c; } rec_t;
        initial begin
          begin automatic rec_t r; $display("A %0d", r.c); end
          begin automatic rec_t r; r.c = 1; $display("B %0d", r.c); end
        end
      endmodule"#);
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("A 0"), "block A must see a FRESH default:\n{o}");
    assert!(o.contains("B 1"), "got:\n{o}");
}

/// The scalar / enum / alias forms this was measured against — they already worked
/// and must keep working, since the fix is about making the struct match them.
#[test]
fn the_scalar_enum_and_alias_forms_agree_with_it() {
    for (ty, def) in [
        ("int", ""),
        ("e_t", "typedef enum { R, G } e_t;"),
        ("my_t", "typedef int my_t;"),
    ] {
        let (o, ok) = run(&format!(
            r#"module t;
                 {def}
                 initial begin
                   begin automatic {ty} v; $display("A %0d", v); end
                   begin automatic {ty} v; v = 1; $display("B %0d", v); end
                 end
               endmodule"#
        ));
        assert!(
            ok && o.contains("A 0") && o.contains("B 1"),
            "{ty} got:\n{o}"
        );
    }
}
