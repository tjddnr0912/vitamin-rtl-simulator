//! Round-19: a call that RETURNS A VALUE while writing an `output` actual.
//!
//! Oracle: iverilog 13 rejects BOTH halves of this shape — a function with an
//! output/inout formal ("Function arguments must be input ports") and an explicit
//! `automatic` block-local ("Overriding the default variable lifetime is not yet
//! supported") — so the value pins here are hand-IEEE (§13.5.2 copy-out ordering,
//! §11.4.7 short-circuit evaluation). The one thing iverilog CAN oracle is R19-X1,
//! the default-argument scope, and that pin carries its measured numbers.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("vita_r19_{}_{n}.sv", std::process::id()));
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

/// `nxt` writes a two-member record through an output formal and returns whether the
/// index is still in range — the `.rsp`-walker signature the whole report is about.
const NXT: &str = r#"
    typedef struct { int len; string h; } rec_t;
    function automatic int nxt (input int i, output rec_t r);
        r.len = i*10; r.h = "x";
        return (i < 3);
    endfunction
    function automatic int sc (input int i, output int o); o = i + 100; return 1; endfunction
"#;

// ── §3.1: the write is recognized wherever the value can go ──────────────────

/// The report's minimal repro: the call is the rhs of an assignment, so the DA walk
/// saw only "the rhs mentions `r`" and called it a read. The copy-out happens while
/// the rhs is being evaluated, i.e. strictly before anything downstream.
#[test]
fn an_output_actual_in_a_direct_rhs_is_a_definite_write() {
    let (o, ok) = run(&format!(
        r#"module t;{NXT}
             initial begin
               begin
                 automatic rec_t r; automatic int go;
                 go = nxt(2, r);
                 $display("A go=%0d len=%0d h=%s", go, r.len, r.h);
               end
               $finish;
             end
           endmodule"#
    ));
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("A go=1 len=20 h=x"), "got:\n{o}");
}

/// Buried one level, in an operand of `==`. Both operands of a non-short-circuit
/// operator are evaluated, so the call runs unconditionally.
#[test]
fn an_output_actual_under_a_comparison_is_a_definite_write() {
    let (o, ok) = run(&format!(
        r#"module t;{NXT}
             initial begin
               begin
                 automatic rec_t r;
                 if (nxt(2, r) == 1) $display("B len=%0d", r.len);
               end
               $finish;
             end
           endmodule"#
    ));
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("B len=20"), "got:\n{o}");
}

/// The report's REAL site: `while (n < lim && rsp_next(fd, r) == 1)`. The body runs
/// only when the whole condition is true, and `a && b` is true only when BOTH
/// operands were evaluated — so the call has written `r` by the time the body starts.
#[test]
fn a_short_circuit_operand_writes_before_the_loop_body() {
    let (o, ok) = run(&format!(
        r#"module t;{NXT}
             initial begin
               begin
                 automatic rec_t r; automatic int n;
                 n = 0;
                 while (n < 10 && nxt(n, r) == 1) begin
                   $display("C n=%0d len=%0d", n, r.len);
                   n = n + 1;
                 end
               end
               $finish;
             end
           endmodule"#
    ));
    assert!(ok, "expected acceptance:\n{o}");
    for (n, len) in [(0, 0), (1, 10), (2, 20)] {
        assert!(o.contains(&format!("C n={n} len={len}")), "got:\n{o}");
    }
    assert!(!o.contains("C n=3"), "the loop must stop at i<3:\n{o}");
}

/// The same shape with a plain scalar output — the report measured that the trigger
/// is "the call returns a value", not the formal's type.
#[test]
fn a_scalar_output_actual_behaves_the_same() {
    let (o, ok) = run(&format!(
        r#"module t;{NXT}
             initial begin
               begin
                 automatic int o; automatic int g;
                 g = sc(4, o);
                 $display("D g=%0d o=%0d", g, o);
               end
               $finish;
             end
           endmodule"#
    ));
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("D g=1 o=104"), "got:\n{o}");
}

/// The LEFT operand of `&&` is always evaluated, so its write survives the loop —
/// including on the exit path, where the right operand may have been skipped.
#[test]
fn a_left_short_circuit_operand_writes_on_every_path() {
    let (o, ok) = run(&format!(
        r#"module t;{NXT}
             initial begin
               begin
                 automatic rec_t r; automatic int n;
                 n = 9;
                 while (nxt(n, r) == 1 && n < 3) n = n + 1;
                 $display("E len=%0d", r.len);
               end
               $finish;
             end
           endmodule"#
    ));
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("E len=90"), "got:\n{o}");
}

/// `a || f(out r)` is FALSE only when both operands ran — so the `else` branch knows
/// `r` is written even though the join does not.
#[test]
fn a_false_or_condition_writes_before_the_else_branch() {
    let (o, ok) = run(&format!(
        r#"module t;{NXT}
             initial begin
               begin
                 automatic rec_t r; automatic int c;
                 c = 0;
                 if (c || nxt(2, r) == 0) $display("F-then");
                 else $display("F len=%0d", r.len);
               end
               $finish;
             end
           endmodule"#
    ));
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("F len=20"), "got:\n{o}");
}

// ── §3.1 soundness pins: what must stay loud ─────────────────────────────────

/// SOUNDNESS PIN. A `&&` RIGHT operand may be short-circuited away, so the loop EXIT
/// carries no write — only the body does. Reading after the loop must stay loud.
#[test]
fn a_right_short_circuit_operand_does_not_write_on_the_exit_path() {
    let (o, ok) = run(&format!(
        r#"module t;{NXT}
             initial begin
               begin
                 automatic rec_t r; automatic int n;
                 n = 99;
                 while (n < 10 && nxt(n, r) == 1) n = n + 1;
                 $display("G len=%0d", r.len);
               end
               $finish;
             end
           endmodule"#
    ));
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("E3009"), "got:\n{o}");
}

/// SOUNDNESS PIN. A read of the local ELSEWHERE in the same expression may be
/// evaluated before the call (operand order is unspecified for `+`), so the write
/// cannot be claimed.
#[test]
fn a_co_operand_read_still_blocks_the_write_claim() {
    let (o, ok) = run(&format!(
        r#"module t;{NXT}
             initial begin
               begin
                 automatic int o; automatic int g;
                 g = sc(4, o) + o;
                 $display("H %0d", g);
               end
               $finish;
             end
           endmodule"#
    ));
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("E3009"), "got:\n{o}");
}

/// SOUNDNESS PIN. Exactly one `?:` arm runs, so neither arm's write is guaranteed.
#[test]
fn a_ternary_arm_write_is_not_claimed() {
    let (o, ok) = run(&format!(
        r#"module t;{NXT}
             initial begin
               begin
                 automatic int o; automatic int g; automatic int c;
                 c = 0;
                 g = c ? sc(4, o) : 0;
                 $display("I %0d", o);
               end
               $finish;
             end
           endmodule"#
    ));
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("E3009"), "got:\n{o}");
}

/// SOUNDNESS PIN. An `inout` actual COPIES IN before it copies out, so it reads the
/// local first — never a definite write.
#[test]
fn an_inout_actual_is_not_a_definite_write() {
    let (o, ok) = run(r#"module t;
        function automatic int io (input int i, inout int x); x = x + i; return 1; endfunction
        initial begin
          begin automatic int v; automatic int g; g = io(1, v); $display("J %0d", v); end
          $finish;
        end
      endmodule"#);
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("E3009"), "got:\n{o}");
}

// ── §3.2: statement position ─────────────────────────────────────────────────

/// The report's §3.2: throwing the return value away — the natural workaround for
/// §3.1 — was itself rejected, so a TB had no way to express the write at all. Both
/// spellings (`void'(f(…))` and the bare `f(…);`) parse to the same statement.
#[test]
fn a_discarded_out_formal_call_is_a_statement() {
    let (o, ok) = run(&format!(
        r#"module t;{NXT}
             initial begin
               begin automatic rec_t r; void'(nxt(5, r)); $display("K %0d %s", r.len, r.h); end
               begin automatic rec_t r; nxt(6, r);        $display("L %0d %s", r.len, r.h); end
               $finish;
             end
           endmodule"#
    ));
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("K 50 x") && o.contains("L 60 x"), "got:\n{o}");
}

/// …including the `inout` copy-in / copy-out pair, which is the part a plain
/// discarded call must not lose.
#[test]
fn a_discarded_call_still_copies_an_inout_both_ways() {
    let (o, ok) = run(r#"module t;
        function automatic int s2 (input int i, output int o, inout int x);
          o = i + 1; x = x * 2; return 7;
        endfunction
        initial begin
          begin automatic int o; int x; x = 3; void'(s2(10, o, x)); $display("M o=%0d x=%0d", o, x); end
          $finish;
        end
      endmodule"#);
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("M o=11 x=6"), "got:\n{o}");
}

// ── §3.3: named arguments ────────────────────────────────────────────────────

/// A named argument does not make a call unanalysable — the positional-then-named
/// mapping was simply never written, so the walk answered `Unknown` and blamed a
/// local several arguments to the left of the one it could not map.
#[test]
fn a_named_argument_call_still_resolves_its_output_actual() {
    let (o, ok) = run(r#"module t;
        typedef struct { int c; } rec_t;
        task automatic scen (input string nm, input int alg, output rec_t e, input int inj = 0);
          e.c = alg + inj;
        endtask
        initial begin
          begin automatic rec_t e; scen("D11", 7, e, .inj(5)); $display("N %0d", e.c); end
          begin automatic rec_t e; scen(.nm("D12"), .alg(2), .e(e), .inj(1)); $display("O %0d", e.c); end
          begin automatic rec_t e; scen("D13", 4, e);          $display("P %0d", e.c); end
          $finish;
        end
      endmodule"#);
    assert!(ok, "expected acceptance:\n{o}");
    assert!(
        o.contains("N 12") && o.contains("O 3") && o.contains("P 4"),
        "got:\n{o}"
    );
}

/// The same for a value-returning function — this path never reordered named args at
/// all, and produced two diagnostics naming neither cause.
#[test]
fn a_named_argument_out_formal_function_lowers() {
    let (o, ok) = run(r#"module t;
        function automatic int f (input int a, output int o, input int b = 3);
          o = a + b; return 1;
        endfunction
        initial begin
          begin automatic int o; automatic int g; g = f(.a(1), .o(o), .b(2)); $display("Q %0d %0d", g, o); end
          begin automatic int o; if (f(.a(5), .o(o)) == 1) $display("R %0d", o); end
          $finish;
        end
      endmodule"#);
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("Q 1 3") && o.contains("R 8"), "got:\n{o}");
}

/// SOUNDNESS PIN. An OMITTED formal's default is lowered in the CALLER's scope, so it
/// can read the flattened local even though no written-out argument mentions it. The
/// mapping must count that as a read, not skip it.
#[test]
fn an_omitted_default_that_reads_the_local_blocks_the_accept() {
    let (o, ok) = run(r#"module t;
        task automatic tw (output int x, input int y = a);
          x = y + 1;
        endtask
        initial begin
          begin int a; tw(a); $display("S %0d", a); end
          begin int a; tw(a); $display("T %0d", a); end
          $finish;
        end
      endmodule"#);
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("E3009"), "got:\n{o}");
}

// ── R19-X1: the default-argument scope ───────────────────────────────────────

/// SOUNDNESS PIN — a pre-existing silent-wrong, measured against iverilog 13:
/// vita printed `91`, iverilog prints `6`, at exit 0 with no diagnostic.
///
/// vita lowers a filled DEFAULT argument value in the CALLER's scope; IEEE 1800
/// §13.5.4 evaluates it where the subroutine is DECLARED. A caller that declares its
/// own `g` therefore hijacked the callee's default. (The identical hazard for a CLASS
/// method's default was already closed by `default_is_scope_safe`; the plain
/// function/task twin was not.)
#[test]
fn a_caller_shadowed_default_argument_is_loud() {
    let (o, ok) = run(r#"module t;
        int g = 5;
        task automatic tw (output int x, input int y = g); x = y + 1; endtask
        task automatic outer(); int g; int a; g = 90; tw(a); $display("U a=%0d", a); endtask
        initial begin outer(); $finish; end
      endmodule"#);
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("13.5.4"), "got:\n{o}");
    assert!(
        !o.contains("U a=91"),
        "the silent-wrong value escaped:\n{o}"
    );
}

/// …and the correct cases must keep working: a default naming a module net resolves
/// outward to the SAME net from a module process and from a generate block. Both
/// values verified against iverilog 13.
#[test]
fn an_unshadowed_default_argument_still_resolves() {
    let (o, ok) = run(r#"module t;
        int g = 5;
        task automatic tw (output int x, input int y = g); x = y + 1; endtask
        genvar i;
        generate for (i = 0; i < 1; i = i + 1) begin : gb
          initial begin int a; tw(a); $display("GEN a=%0d", a); end
        end endgenerate
        initial begin
          #1;
          begin int a; tw(a); $display("MOD a=%0d", a); end
          $finish;
        end
      endmodule"#);
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("GEN a=6") && o.contains("MOD a=6"), "got:\n{o}");
}

// ── R19-X2: file reads inside a framed subroutine body ───────────────────────

/// Write a two-line vector file and return its absolute path (the simulator's cwd is
/// the crate root, so the design must name the file absolutely).
fn vector_file(tag: &str) -> String {
    let p = std::env::temp_dir().join(format!("vita_r19_vec_{}_{tag}.txt", std::process::id()));
    std::fs::write(&p, "Len = 8\nMsg = 61\n").unwrap();
    p.to_string_lossy().replace('\\', "/")
}

/// SOUNDNESS PIN — a pre-existing silent-wrong, measured against iverilog 13:
/// vita printed `rc=0` with an EMPTY string, iverilog reads the line, at exit 0.
///
/// `$fgets` (and `$fscanf`/`$sscanf`/`$fread`/`$fgetc`/`$ungetc`) write their
/// destination as a statement-level effect that only the PROCESS executor performs.
/// A frame body evaluates the same `SysFunc` through the pure evaluator, whose arm
/// for these ids returns X and touches nothing — which is exactly the `.rsp` walker
/// shape §3.1 has just made reachable.
#[test]
fn a_file_read_inside_a_framed_body_is_fatal_not_a_quiet_zero() {
    let f = vector_file("framed");
    let (o, ok) = run(&format!(
        r#"module t;
             function automatic int rd (input int fd, output string s);
               int rc;
               rc = $fgets(s, fd);
               return rc;
             endfunction
             int fd; int r; string s;
             initial begin
               fd = $fopen("{f}", "r");
               r = rd(fd, s);
               $display("X rc=%0d s=[%s]", r, s);
               $fclose(fd);
               $finish;
             end
           endmodule"#
    ));
    let _ = std::fs::remove_file(&f);
    assert!(!ok, "expected a diagnostic, got a clean run:\n{o}");
    assert!(o.contains("F4004") && o.contains("$fgets"), "got:\n{o}");
}

/// REGRESSION PIN for how that fatal is placed. An elaborate-time gate on the framed
/// BODY was tried first and measured wrong: a `task automatic` with no output formals
/// is lowered BOTH framed and inline, and the inline copy — the one its callers run —
/// reads the file correctly. Gating the body loud-rejected this working design; the
/// runtime fatal fires only where the frame copy actually executes.
#[test]
fn an_inlined_task_still_reads_the_file() {
    let f = vector_file("inline");
    let (o, ok) = run(&format!(
        r#"module t;
             int fd; string s; int rc;
             task automatic pull (); rc = $fgets(s, fd); endtask
             initial begin
               fd = $fopen("{f}", "r");
               pull();
               $display("Y rc=%0d s=%s", rc, s.substr(0, 2));
               $fclose(fd);
               $finish;
             end
           endmodule"#
    ));
    let _ = std::fs::remove_file(&f);
    assert!(ok, "expected acceptance:\n{o}");
    assert!(o.contains("Y rc=8 s=Len"), "got:\n{o}");
}
