//! Pins for system tasks that had ZERO test coverage. Found by the strategy
//! §4.5.236 recorded: `grep -rl '<spec>' crates/cli/tests/` coming back empty
//! marks a place where nothing would notice a regression. Unlike `%p` — which
//! that scan found actually broken — these all match iverilog 13.0 today; the
//! risk was purely that nothing pinned them.
//!
//! Covered: `$sformat` into a `string` and into a packed `reg` vector, `$swrite`,
//! `$test$plusargs` (hit and miss), and `$value$plusargs` with `%d` and `%s`
//! conversions, driven by real `+arg` values on the command line.
//!
//! Recorded restriction (ROADMAP §3, not a defect): `$value$plusargs` is
//! supported only as the direct right-hand side of a blocking assignment, so
//! `$display("%0d", $value$plusargs(...))` is loud where iverilog allows it.
//! `$test$plusargs` has no such restriction.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_spa_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_vita"));
    c.arg(f.to_str().unwrap());
    for a in args {
        c.arg(a);
    }
    let out = c.current_dir(&d).output().expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

fn run(src: &str) -> (String, Option<i32>) {
    run_args(src, &[])
}

/// `$sformat` / `$swrite` render into a destination instead of stdout — same
/// conversions, including into a packed `reg` vector rather than a `string`.
#[test]
fn sformat_and_swrite_render_into_their_destination() {
    let (out, c) = run("module m;\n  string s; reg [8*20:1] r;\n  initial begin\n\
           $sformat(s, \"a=%0d b=%s c=%h\", 42, \"hi\", 8'hAF);\n\
           $display(\"S=[%s]\", s);\n\
           $sformat(r, \"x=%0d\", 7);\n\
           $display(\"R=[%0s]\", r);\n\
           $swrite(s, \"w=%0d\", 99);\n\
           $display(\"W=[%s]\", s);\n\
           #1 $finish; end\nendmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("S=[a=42 b=hi c=af]"),
        "sformat to string; got:\n{out}"
    );
    assert!(
        out.contains("R=[x=7]"),
        "sformat to a packed vector; got:\n{out}"
    );
    assert!(out.contains("W=[w=99]"), "swrite; got:\n{out}");
}

/// Plusargs, with and without the argument actually present.
#[test]
fn plusargs_match_and_convert() {
    let src = "module m;\n  int v; string sv; int ok1, ok2;\n  initial begin\n\
           $display(\"T=%0d %0d\", $test$plusargs(\"foo\"), $test$plusargs(\"nope\"));\n\
           ok1 = $value$plusargs(\"num=%d\", v);\n\
           ok2 = $value$plusargs(\"str=%s\", sv);\n\
           $display(\"V=%0d %0d | %0d %s\", ok1, ok2, v, sv);\n\
           #1 $finish; end\nendmodule\n";

    let (out, c) = run_args(src, &["+foo", "+num=42", "+str=hey"]);
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("T=1 0"),
        "$test$plusargs hit/miss; got:\n{out}"
    );
    assert!(
        out.contains("V=1 1 | 42 hey"),
        "$value$plusargs; got:\n{out}"
    );

    // With nothing supplied every query misses and the targets keep their
    // pre-call values (IEEE: the destination is untouched on a miss).
    let (out, c) = run_args(src, &[]);
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(out.contains("T=0 0"), "no plusargs; got:\n{out}");
    assert!(
        out.contains("V=0 0 | 0 "),
        "misses leave targets alone; got:\n{out}"
    );
}

/// The documented restriction, pinned so it is visible rather than surprising:
/// `$value$plusargs` must be the direct RHS of a blocking assignment.
#[test]
fn value_plusargs_in_an_expression_is_loud() {
    let (out, c) = run("module m;\n  int v;\n  initial begin\n\
           $display(\"%0d\", $value$plusargs(\"num=%d\", v));\n\
           #1 $finish; end\nendmodule\n");
    assert_ne!(c, Some(0), "must be loud, not silently wrong; got:\n{out}");
}
