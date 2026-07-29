//! R16 §3.1 / §3.2: the definite-assignment walker's two false-loud modes, and the
//! hazards that must survive fixing them.
//!
//! v1 flattens an `automatic` block-local to ONE static module net. That flatten is
//! byte-identical to a real per-activation `automatic` only when the local is
//! DEFINITELY ASSIGNED before every read on every path, so `da_stmt` walks the block
//! and stays loud whenever it cannot prove that. Two things it could not prove were
//! not actually unprovable:
//!
//!   §3.1 A `break`/`continue` placed BEFORE the first write. The walker carried only
//!        the assigned bool, which cannot distinguish "runs on to the next statement"
//!        from "jumps away", so the jump read as a live path arriving at the later
//!        read with the local still unwritten. Every path that really reaches that
//!        read has already executed the write. 49 of the 84 diagnostics in the
//!        round-16 report were this one conflation.
//!
//!   §3.2 A statement-position user task / void-function call. It was left unvetted
//!        DELIBERATELY (round-19 review F5): a callee body can name the flattened bare
//!        net without the call head or any argument mentioning it. The hazard is real
//!        — both `task peek; $display(a); endtask` and `task poke; t.a = 99; endtask`
//!        reach a block-local `a` — so the fix proves the callee cannot touch the name
//!        rather than assuming it.
//!
//! ORACLE. iverilog 13.0 rejects an explicit `automatic` lifetime override outright
//! ("sorry: Overriding the default variable lifetime"), so these shapes have no live
//! differential oracle. The reference is instead the ALREADY-WORKING boundary the
//! report measured: moving the jump AFTER the first write, or the write BEFORE the
//! call, is accepted today, and the accepted form's printed values are what the fixed
//! form must reproduce. Every `loud` case below is a hazard that must NOT become an
//! accept — those are the soundness pins.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dacf_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (text, out.status.success())
}

/// Accepted AND produced exactly `want` (one line per element, in order).
fn runs(src: &str, want: &[&str]) {
    let (o, ok) = run(src);
    assert!(ok, "expected acceptance, got:\n{o}");
    let got: Vec<&str> = o
        .lines()
        .filter(|l| l.starts_with("R ") || *l == "PASS")
        .collect();
    assert_eq!(got, want, "output mismatch:\n{o}");
}

/// Rejected with E3009 naming `who` — a soundness pin, not a wish.
fn loud(src: &str, who: &str) {
    let (o, ok) = run(src);
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("E3009"), "expected E3009, got:\n{o}");
    assert!(
        o.contains(&format!("`{who}`")),
        "expected the diagnostic to name `{who}`, got:\n{o}"
    );
}

// ---------------------------------------------------------------------------
// §3.1 — a loop jump before the first write
// ---------------------------------------------------------------------------

/// The report's own reproducer. `continue` sits before the first write of both
/// locals; the read is reached only on iterations that skipped the `continue`.
#[test]
fn continue_before_first_write_is_supported() {
    runs(
        r#"module t; string f[3] = '{"a","skipme","c"};
             initial begin
               foreach (f[i]) begin
                 automatic int  L;
                 automatic byte md [];
                 if (f[i] == "skipme") continue;
                 L = i; md = new[2];
                 $display("R %0d %0d %0d", i, L, md.size());
               end
               $display("PASS");
             end
           endmodule"#,
        &["R 0 0 2", "R 2 2 2", "PASS"],
    );
}

/// The report's PASS boundary: the identical program with the jump moved after the
/// write was already accepted. Pinning it keeps the fix from being a no-op check.
#[test]
fn continue_after_first_write_still_supported() {
    runs(
        r#"module t;
             initial begin
               for (int i = 0; i < 3; i++) begin
                 automatic int L;
                 L = i;
                 if (i == 1) continue;
                 $display("R %0d %0d", i, L);
               end
               $display("PASS");
             end
           endmodule"#,
        &["R 0 0", "R 2 2", "PASS"],
    );
}

/// A jump in ONE arm of an `if`/`else`: the jumping arm never reaches the join, so
/// the else-arm's write alone makes the local definitely assigned there. Plain
/// passthrough (treating the jump as falling through) would still be loud here.
#[test]
fn jump_in_one_arm_drops_out_of_the_join() {
    runs(
        r#"module t;
             initial begin
               for (int i = 0; i < 3; i++) begin
                 automatic int L;
                 if (i == 1) continue; else L = i * 10;
                 $display("R %0d %0d", i, L);
               end
               $display("PASS");
             end
           endmodule"#,
        &["R 0 0", "R 2 20", "PASS"],
    );
}

/// `break` as the jump, and the loop it leaves is the one whose body declares the
/// local.
#[test]
fn break_before_first_write_is_supported() {
    runs(
        r#"module t;
             initial begin
               for (int i = 0; i < 4; i++) begin
                 automatic int L;
                 if (i == 2) break;
                 L = i;
                 $display("R %0d %0d", i, L);
               end
               $display("PASS");
             end
           endmodule"#,
        &["R 0 0", "R 1 1", "PASS"],
    );
}

/// The report's decisive evidence: a `break` belonging to an INNER loop cannot skip
/// the outer block's write at all, yet it was still loud.
#[test]
fn inner_loop_break_cannot_skip_the_outer_write() {
    runs(
        r#"module t;
             initial begin
               begin
                 automatic int x;
                 for (int j = 0; j < 3; j++) begin if (j == 1) break; end
                 x = 5;
                 $display("R %0d", x);
               end
               $display("PASS");
             end
           endmodule"#,
        &["R 5", "PASS"],
    );
}

/// A `case` arm that jumps drops out of the arm merge exactly as an `if` arm does.
#[test]
fn case_arm_jump_drops_out_of_the_merge() {
    runs(
        r#"module t;
             initial begin
               for (int i = 0; i < 3; i++) begin
                 automatic int L;
                 case (i) 1: continue; default: L = i + 100; endcase
                 $display("R %0d %0d", i, L);
               end
               $display("PASS");
             end
           endmodule"#,
        &["R 0 100", "R 2 102", "PASS"],
    );
}

/// Every loop form the report measured behaves the same.
#[test]
fn while_and_repeat_jumps_behave_the_same() {
    runs(
        r#"module t;
             int k = 0;
             initial begin
               while (k < 3) begin
                 automatic int L;
                 k++;
                 if (k == 2) continue;
                 L = k;
                 $display("R %0d", L);
               end
               $display("PASS");
             end
           endmodule"#,
        &["R 1", "R 3", "PASS"],
    );
    runs(
        r#"module t;
             int k = 0;
             initial begin
               repeat (3) begin
                 automatic int L;
                 k++;
                 if (k == 2) continue;
                 L = k;
                 $display("R %0d", L);
               end
               $display("PASS");
             end
           endmodule"#,
        &["R 1", "R 3", "PASS"],
    );
}

/// The jump may be nested arbitrarily deep inside `if`/`begin`.
#[test]
fn deeply_nested_jump_is_supported() {
    runs(
        r#"module t;
             initial begin
               for (int i = 0; i < 3; i++) begin
                 automatic int L;
                 if (i > 0) begin if (i == 1) begin continue; end end
                 L = i;
                 $display("R %0d %0d", i, L);
               end
               $display("PASS");
             end
           endmodule"#,
        &["R 0 0", "R 2 2", "PASS"],
    );
}

/// Dynamic-storage locals take the same path as scalars.
#[test]
fn jump_before_write_of_string_and_dynarray() {
    runs(
        r#"module t;
             initial begin
               for (int i = 0; i < 3; i++) begin
                 automatic string s;
                 automatic byte   b [];
                 if (i == 1) continue;
                 s = "x"; b = new[3];
                 $display("R %s %0d", s, b.size());
               end
               $display("PASS");
             end
           endmodule"#,
        &["R x 3", "R x 3", "PASS"],
    );
}

/// SOUNDNESS PIN. Verilog escaped identifiers may contain `$`, so a user CAN write a
/// block literally named `\$break$77` — the same spelling the parser synthesizes for a
/// `break`. If the two collided, the `disable` below would be read as a loop jump, its
/// arm would drop out of the merge, and the read of the unwritten `x` would be silently
/// accepted. The discriminator distinguishes them (this stays loud, while the genuine
/// `continue` in `jump_in_one_arm_drops_out_of_the_join` is accepted).
#[test]
fn an_escaped_identifier_cannot_spoof_a_loop_jump() {
    loud(
        r#"module t;
             int c = 1;
             initial begin
               begin
                 automatic int x;
                 begin : \$break$77
                   if (c) disable \$break$77 ;
                   else x = 1;
                 end
                 $display("R %0d", x);
               end
             end
           endmodule"#,
        "x",
    );
}

/// SOUNDNESS PIN. A real `disable` of some OTHER block is not a loop jump: it kills
/// that block and lets this one run on, so its arm still reaches the join with the
/// local unwritten. Treating every `disable` as a jump would silently accept this
/// read of the previous entry's leftover value.
#[test]
fn plain_disable_is_not_treated_as_a_loop_jump() {
    loud(
        r#"module t;
             initial begin : other
               #10 $display("other");
             end
             initial begin
               for (int i = 0; i < 3; i++) begin
                 automatic int L;
                 if (i == 1) disable other; else L = i;
                 $display("R %0d %0d", i, L);
               end
             end
           endmodule"#,
        "L",
    );
}

// ---------------------------------------------------------------------------
// §3.2 — a statement-position subroutine call
// ---------------------------------------------------------------------------

/// The report's reproducer: the call sits before the write, and the scan used to end
/// there so the later write was invisible.
#[test]
fn statement_call_does_not_end_the_scan() {
    runs(
        r#"module t;
             task automatic show (input int v); $display("R v=%0d", v); endtask
             initial begin
               begin
                 automatic int a;
                 show(0);
                 a = 1;
                 $display("R %0d", a);
               end
               $display("PASS");
             end
           endmodule"#,
        &["R v=0", "R 1", "PASS"],
    );
}

/// A statement-position void FUNCTION call is the same shape (both parse to
/// `UserTaskCall`).
#[test]
fn statement_void_function_call_does_not_end_the_scan() {
    runs(
        r#"module t;
             function automatic void vf (input int q); $display("R vf %0d", q); endfunction
             initial begin
               begin
                 automatic int a;
                 vf(3);
                 a = 1;
                 $display("R %0d", a);
               end
               $display("PASS");
             end
           endmodule"#,
        &["R vf 3", "R 1", "PASS"],
    );
}

/// A callee that calls a clean callee is still clean — the proof is transitive.
#[test]
fn transitively_clean_callee_is_inert() {
    runs(
        r#"module t;
             task automatic inner; $display("R inner"); endtask
             task automatic outer; inner(); endtask
             initial begin
               begin
                 automatic int a;
                 outer();
                 a = 1;
                 $display("R %0d", a);
               end
               $display("PASS");
             end
           endmodule"#,
        &["R inner", "R 1", "PASS"],
    );
}

/// SOUNDNESS PIN (review F5, measured). The callee names the flattened BARE net, so
/// the call really does read `a` before its first write.
#[test]
fn callee_reading_the_flattened_bare_name_stays_loud() {
    loud(
        r#"module t;
             task automatic peek; $display("peek a=%0d", a); endtask
             initial begin
               begin
                 automatic int a;
                 peek();
                 a = 1;
                 $display("R %0d", a);
               end
             end
           endmodule"#,
        "a",
    );
}

/// SOUNDNESS PIN. The same reach through a HIERARCHICAL self-path. This is why the
/// callee-body walk uses an all-segments path rule: the head-segment rule that is
/// correct for the caller's own statements would call `t.a = 99;` ref-free.
#[test]
fn callee_hierarchical_self_write_stays_loud() {
    loud(
        r#"module t;
             task automatic poke; t.a = 99; endtask
             initial begin
               begin
                 automatic int a;
                 poke();
                 a = 1;
                 $display("R %0d", a);
               end
             end
           endmodule"#,
        "a",
    );
}

/// SOUNDNESS PIN. The reach is transitive too — `outer` looks clean on its own.
#[test]
fn transitively_reaching_callee_stays_loud() {
    loud(
        r#"module t;
             task automatic inner; $display("a=%0d", a); endtask
             task automatic outer; inner(); endtask
             initial begin
               begin
                 automatic int a;
                 outer();
                 a = 1;
                 $display("R %0d", a);
               end
             end
           endmodule"#,
        "a",
    );
}

/// SOUNDNESS PIN. `name` at an ordinary INPUT actual is a genuine read-before-write;
/// widening statement calls must not have widened this.
#[test]
fn call_reading_the_local_as_an_argument_stays_loud() {
    loud(
        r#"module t;
             task automatic show (input int v); $display("v=%0d", v); endtask
             initial begin
               begin
                 automatic int a;
                 show(a);
                 a = 1;
                 $display("R %0d", a);
               end
             end
           endmodule"#,
        "a",
    );
}

/// A mutually recursive pair cannot be walked to a fixpoint by the depth budget, so
/// it answers "may touch" and the local stays loud — precision lost, never soundness.
#[test]
fn mutually_recursive_callees_stay_loud() {
    loud(
        r#"module t;
             int n;
             task automatic ping; if (n > 0) begin n--; pong(); end endtask
             task automatic pong; if (n > 0) begin n--; ping(); end endtask
             initial begin
               begin
                 automatic int a;
                 n = 4;
                 ping();
                 a = 1;
                 $display("R %0d", a);
               end
             end
           endmodule"#,
        "a",
    );
}
