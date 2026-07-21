//! §4.5.179: a call to a FRAMED function with an `input` dynamic-array formal (the set
//! §4.5.177 supports on the direct-rhs `r = f(arr)` path) BURIED in a larger expression —
//! `$display(f(a))`, `r = f(a)+1`, `if (f(a) > 0)` — is now supported by HOISTING the call
//! to a fresh temp `__t = f(a)` (a direct-rhs blocking assign that re-triggers §4.5.177's
//! snapshot marker) and reading the temp in place. Reuses the R5-B inout hoist skeleton.
//!
//! Correct-or-loud is preserved BY CONSTRUCTION: a framed function is pure (no output
//! formals), so hoisting its evaluation earlier never changes another operand — no
//! eval-order guard is needed. Only UNCONDITIONALLY-evaluated positions are hoisted; a
//! call in a `&&`/`||` short-circuit RHS, a `?:` arm, a `while`/`for`/`case` scrutinee, or
//! inside another subroutine body (no `&mut` executor to run the snapshot marker) is left
//! in place → it reaches `emit_frame_call` and is loud (E3009). Each hoist emits its own
//! snapshot marker, so pass-by-value freshness across a mutation between calls holds.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ffdfn_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn is_loud(o: &str) -> bool {
    o.contains("E3009") || o.contains("F4004") || o.contains("ended (Error)")
}

const FSUM: &str = "function automatic int fsum(input int c[]); \
     int s=0; foreach(c[i]) s+=c[i]; return s; endfunction\n";
const FMAX: &str = "function automatic int fmax(input int c[]); \
     int m=c[0]; foreach(c[i]) if(c[i]>m) m=c[i]; return m; endfunction\n";

fn prog(funcs: &str, body: &str) -> String {
    format!(
        "module top;\n{funcs}  int a[]; int b[]; int r; int x;\n\
         initial begin\n  a=new[3]; a[0]=4;a[1]=5;a[2]=6;\n  b=new[2]; b[0]=10;b[1]=20;\n\
         {body}\n  end\nendmodule\n"
    )
}

// ── SUPPORTED: a dyn-formal call buried in a bigger expression ──────────────

#[test]
fn display_single_call() {
    // $display("%0d", fsum(a)) — iverilog: 15
    let o = run(&prog(FSUM, "  $display(\"R=%0d\", fsum(a));"));
    assert!(
        !is_loud(&o) && o.contains("R=15"),
        "display(fsum(a))=15:\n{o}"
    );
}

#[test]
fn arith_on_call_rhs() {
    // r = fsum(a)*2 + 1 — iverilog: 31
    let o = run(&prog(FSUM, "  r = fsum(a)*2 + 1; $display(\"R=%0d\", r);"));
    assert!(!is_loud(&o) && o.contains("R=31"), "fsum(a)*2+1=31:\n{o}");
}

#[test]
fn if_condition_call() {
    // if (fsum(a) > 10) — iverilog: big
    let o = run(&prog(
        FSUM,
        "  if (fsum(a) > 10) $display(\"R=big\"); else $display(\"R=small\");",
    ));
    assert!(
        !is_loud(&o) && o.contains("R=big"),
        "if(fsum(a)>10)=big:\n{o}"
    );
}

#[test]
fn two_calls_one_expr() {
    // fsum(a) + fmax(b) = 15 + 20 = 35
    let o = run(&prog(
        &format!("{FSUM}{FMAX}"),
        "  $display(\"R=%0d\", fsum(a)+fmax(b));",
    ));
    assert!(
        !is_loud(&o) && o.contains("R=35"),
        "fsum(a)+fmax(b)=35:\n{o}"
    );
}

#[test]
fn two_args_same_display() {
    // $display("%0d %0d", fsum(a), fmax(b)) — 15 20 (both hoisted, order preserved)
    let o = run(&prog(
        &format!("{FSUM}{FMAX}"),
        "  $display(\"R=%0d %0d\", fsum(a), fmax(b));",
    ));
    assert!(
        !is_loud(&o) && o.contains("R=15 20"),
        "two args hoisted = 15 20:\n{o}"
    );
}

#[test]
fn paren_and_unary() {
    // r = -(fsum(a)) — iverilog: -15
    let o = run(&prog(FSUM, "  r = -(fsum(a)); $display(\"R=%0d\", r);"));
    assert!(!is_loud(&o) && o.contains("R=-15"), "-(fsum(a))=-15:\n{o}");
}

#[test]
fn snapshot_fresh_across_mutation() {
    // Each hoist re-snapshots: 15, then (after a[0]=99) 110 — pass-by-value freshness.
    let o = run(&prog(
        FSUM,
        "  $display(\"R=%0d\", fsum(a)); a[0]=99; $display(\"R=%0d\", fsum(a));",
    ));
    assert!(
        !is_loud(&o) && o.contains("R=15") && o.contains("R=110"),
        "fresh snapshot each call = 15 then 110:\n{o}"
    );
}

#[test]
fn signed_byte_dedicated() {
    let o = run("module top;\n\
         function automatic int f(input byte c[]); int s=0; foreach(c[i]) s+=c[i]; return s; endfunction\n\
         byte a[]; initial begin a=new[3]; a[0]=-5;a[1]=10;a[2]=-2; $display(\"R=%0d\", f(a)); end\n\
         endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("R=3"),
        "signed byte buried = 3:\n{o}"
    );
}

// ── LOUD boundaries (correct-or-loud; never silently made unconditional) ────

#[test]
fn shortcircuit_rhs_loud() {
    // `x && fsum(a)>0` — fsum in the `&&` RHS is only conditionally evaluated → loud.
    let o = run(&prog(
        FSUM,
        "  if (x!=0 && fsum(a) > 0) $display(\"R=y\"); else $display(\"R=n\");",
    ));
    assert!(
        is_loud(&o),
        "short-circuit RHS dyn-formal call = loud:\n{o}"
    );
}

#[test]
fn ternary_arm_loud() {
    // `(x) ? fsum(a) : 0` — conditional arm → loud.
    let o = run(&prog(
        FSUM,
        "  r = (x!=0) ? fsum(a) : 0; $display(\"R=%0d\", r);",
    ));
    assert!(is_loud(&o), "ternary-arm dyn-formal call = loud:\n{o}");
}

#[test]
fn while_condition_loud() {
    // re-evaluated per iteration → a one-shot hoist would be wrong → loud.
    let o = run(&prog(
        FSUM,
        "  while (fsum(a) < 0) x=1; $display(\"R=%0d\", x);",
    ));
    assert!(is_loud(&o), "while-cond dyn-formal call = loud:\n{o}");
}

#[test]
fn case_scrutinee_loud() {
    let o = run(&prog(
        FSUM,
        "  case (fsum(a)) 15: $display(\"R=hit\"); default: $display(\"R=miss\"); endcase",
    ));
    assert!(is_loud(&o), "case-scrutinee dyn-formal call = loud:\n{o}");
}

#[test]
fn nested_in_frame_body_loud() {
    // Family C (r17): the snapshot marker now RUNS inside a frame body (§4.5.194
    // RefCell dyn_heap), so a dyn-formal call inside a TASK body, or inside a
    // function with a LOCAL dyn-array arg, is supported. What stays loud here is a
    // narrower residual: a FUNCTION re-forwarding ITS OWN dyn-array formal `c` to
    // another dyn-formal function (`fsum(c)` where `c` is wrap's formal) — the framed
    // function's own formal is heap-resident and `dyn_array_actual_net` cannot resolve
    // it as a re-forward source (the pre-existing "array-formal re-forwarding" gap).
    // Correct-or-loud: loud, never silently wrong.
    let o = run("module top;\n\
         function automatic int fsum(input int c[]); int s=0; foreach(c[i]) s+=c[i]; return s; endfunction\n\
         function automatic int wrap(input int c[]); $display(\"R=%0d\", fsum(c)); return 0; endfunction\n\
         int a[]; int z; initial begin a=new[3]; a[0]=4;a[1]=5;a[2]=6; z=wrap(a); end\n\
         endmodule\n");
    assert!(
        is_loud(&o),
        "function re-forwarding its own dyn-formal = loud:\n{o}"
    );
}

// ── the DIRECT-rhs path (§4.5.177) is untouched — still supported ──────────

#[test]
fn direct_rhs_still_works() {
    let o = run(&prog(FSUM, "  r = fsum(a); $display(\"R=%0d\", r);"));
    assert!(
        !is_loud(&o) && o.contains("R=15"),
        "direct rhs still 15:\n{o}"
    );
}
