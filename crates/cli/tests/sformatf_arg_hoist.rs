//! `$sformatf(...)` as a top-level VALUE argument to an immediate format task
//! ($display/$write/$fdisplay/$fwrite family) was loud-rejected (E3009 "$sformatf
//! is supported only as the direct rhs of a blocking assignment") — but it is the
//! dominant idiom `$display("%s", $sformatf(...))`. §4.5.127 HOISTS each such arg
//! to a fresh string temp (`tmp = $sformatf(...)` emitted before the task — the
//! immediate task renders exactly once, and $sformatf is pure) and passes the
//! temp Ident. Correct-or-loud boundaries (all still LOUD, exit≠0): the
//! FORMAT-STRING arg (`$display($sformatf(...))`) — a string there needs a
//! runtime-format engine vita lacks (a pre-existing, separate gap); a NESTED
//! `$sformatf` (arg to a concat / another `$sformatf`); and the DEFERRED tasks
//! (`$monitor`/`$strobe`) which re-render (iverilog rejects the arg there too).
//! Values pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sfa_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

#[test]
fn display_sformatf_value_arg() {
    // The dominant idiom: `$display("%s", $sformatf(...))`.
    let (out, c) = run("module t; int x=7; initial begin \
         $display(\"D[%s]\", $sformatf(\"%0d-%0h\", x, x)); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("D[7-7]"), "got:\n{out}");
}

#[test]
fn multiple_sformatf_args() {
    let (out, c) = run("module t; int a=3,b=9; initial begin \
         $display(\"[%s][%s]\", $sformatf(\"a=%0d\",a), $sformatf(\"b=%0d\",b)); \
         #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("[a=3][b=9]"), "got:\n{out}");
}

#[test]
fn mixed_normal_and_sformatf_args() {
    // $sformatf interleaved with ordinary value args (order preserved).
    let (out, c) = run("module t; int x=5; initial begin \
         $display(\"x=%0d s=[%s] y=%0d\", x, $sformatf(\"<%0d>\",x*2), x+1); \
         #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("x=5 s=[<10>] y=6"), "got:\n{out}");
}

#[test]
fn write_and_displayh_families() {
    // $write (no newline) and a radix variant ($displayh) both hoist.
    let (out, c) = run("module t; logic [7:0] x=8'hA5; initial begin \
         $write(\"W[%s]#\", $sformatf(\"v%0d\", x)); $displayh(\" [%s]\", $sformatf(\"%0d\",x)); \
         #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("W[v165]# [165]"), "got:\n{out}");
}

#[test]
fn sformatf_arg_re_renders_in_loop() {
    // The hoist sits inside the loop body, so the temp is re-assigned each
    // iteration (NOT frozen to the first value).
    let (out, c) = run("module t; int i; initial begin \
         for (i=0;i<3;i=i+1) $display(\"[%s]\", $sformatf(\"i=%0d\", i)); \
         #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("[i=0]") && out.contains("[i=1]") && out.contains("[i=2]"),
        "got:\n{out}"
    );
}

#[test]
fn multi_instance_no_value_bleed() {
    // Each instance gets its own module-scoped temp — no cross-instance bleed.
    let (out, c) = run("module sub(input int id); \
         initial #1 $display(\"S%0d[%s]\", id, $sformatf(\"val=%0d\", id*11)); endmodule\n\
         module t; sub a(.id(1)); sub b(.id(2)); initial #2 $finish; endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("S1[val=11]") && out.contains("S2[val=22]"),
        "got:\n{out}"
    );
}

#[test]
fn inline_task_body_hoists() {
    // An inline (non-automatic) task body is spliced into the process stream, so
    // the hoist works there too.
    let (out, c) = run(
        "module t; task show(int v); $display(\"T[%s]\", $sformatf(\"v=%0d\",v)); endtask\n\
         initial begin show(7); #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(out.contains("T[v=7]"), "got:\n{out}");
}

#[test]
fn format_position_sformatf_stays_loud() {
    // Correct-or-loud: `$display($sformatf(...))` — $sformatf in the FORMAT-STRING
    // position — is NOT hoisted (a string variable as the format needs a runtime
    // format engine vita lacks). It stays LOUD (was loud before, stays loud — no
    // silent-wrong). iverilog supports it (runtime format) — a documented follow-on.
    let (out, c) = run(
        "module t; initial begin $display($sformatf(\"hello %0d\", 42)); \
         #1 $finish; end endmodule\n",
    );
    assert_ne!(
        c,
        Some(0),
        "format-position $sformatf must stay loud; got:\n{out}"
    );
}

#[test]
fn fdisplay_format_position_stays_loud() {
    // Same, for $fdisplay (format is arg1 after the fd).
    let (out, c) = run(
        "module t; integer fd; initial begin fd=$fopen(\"o.txt\",\"w\"); \
         $fdisplay(fd, $sformatf(\"z%0d\",1)); $fclose(fd); #1 $finish; end endmodule\n",
    );
    assert_ne!(
        c,
        Some(0),
        "fdisplay format-position must stay loud; got:\n{out}"
    );
}

#[test]
fn deferred_monitor_stays_loud() {
    // $monitor re-renders on change; hoisting would freeze the value. iverilog
    // itself rejects a $sformatf arg there (no oracle). Stays LOUD (base==fix).
    let (out, c) = run("module t; int x=1; initial begin \
         $monitor(\"[%s]\", $sformatf(\"x=%0d\", x)); #1 x=2; #1 $finish; end endmodule\n");
    assert_ne!(
        c,
        Some(0),
        "$monitor $sformatf arg must stay loud; got:\n{out}"
    );
}

#[test]
fn nested_sformatf_stays_loud() {
    // A $sformatf that is an argument to ANOTHER $sformatf (not a bare task arg)
    // is not hoisted → the whole statement stays LOUD (base==fix). iverilog
    // supports it — a follow-on.
    let (out, c) = run("module t; initial begin \
         $display(\"[%s]\", $sformatf(\"outer %s\", $sformatf(\"inner\"))); \
         #1 $finish; end endmodule\n");
    assert_ne!(c, Some(0), "nested $sformatf must stay loud; got:\n{out}");
}

#[test]
fn mutable_string_format_stays_loud() {
    // Correct-or-loud (the R1 differential finding): a mutable `string` VARIABLE in
    // the format position renders WRONG in vita (pre-existing runtime-format gap).
    // Base was LOUD here (the $sformatf value arg's own guard); the hoist must NOT
    // strip that loud and expose the silent mis-render. The hoist is gated on the
    // format arg being a string LITERAL, so a string-variable format is left
    // untouched → the raw $sformatf still hits the loud guard → LOUD (base==fix).
    let (out, c) = run(
        "module t; logic [7:0] x=8'd42; string fmt=\"got %s here\"; initial begin \
         $display(fmt, $sformatf(\"[%0d]\", x)); #1 $finish; end endmodule\n",
    );
    assert_ne!(
        c,
        Some(0),
        "string-variable format must stay loud; got:\n{out}"
    );
}

#[test]
fn fd_position_sformatf_stays_loud() {
    // A $sformatf at the FILE-DESCRIPTOR position of an $f… task (index 0) is never
    // valid (a descriptor must be numeric — iverilog rejects it). Only args strictly
    // after the format (`i > fmt_idx`) are hoisted, so the fd $sformatf stays raw →
    // hits the loud guard → LOUD (not a bogus-fd run).
    let (out, c) = run("module t; int x=1; initial begin \
         $fdisplay($sformatf(\"%0d\",x), \"fmt%0d\", x); #1 $finish; end endmodule\n");
    assert_ne!(
        c,
        Some(0),
        "fd-position $sformatf must stay loud; got:\n{out}"
    );
}

#[test]
fn frame_body_display_sformatf_loud() {
    // Soundness backstop: the hoist runs inside frame (automatic) bodies too, but
    // `validate_frame_body` rejects BOTH the hoisted temp assignment (a write to a
    // net outside the function) AND the $display ($systask) — so a $display with a
    // $sformatf arg in an automatic task/function body is LOUD (base==fix), never a
    // silent-wrong via `run_frame_call`.
    let (out, c) = run("module t; task automatic show(int v); \
         $display(\"A[%s]\", $sformatf(\"v=%0d\",v)); endtask\n\
         initial begin show(8); #1 $finish; end endmodule\n");
    assert_ne!(
        c,
        Some(0),
        "frame-body $display+$sformatf must stay loud; got:\n{out}"
    );
}

#[test]
fn surplus_value_arg_stays_loud() {
    // Correct-or-loud (the R2 differential finding): a literal format with FEWER
    // arg-consuming specifiers than value args leaves a surplus hoisted string temp,
    // which vita renders NUMERICALLY (a pre-existing string-arg gap) while iverilog
    // prints it as text with NO warning (clean valid code: `one=AAABBB`). base was
    // LOUD (the $sformatf guard); the hoist must not strip it and expose the silent
    // mis-render. The count gate (specifiers >= value args) keeps this LOUD.
    let (out, c) = run(
        "module m; initial begin \
         $display(\"one=%s\", $sformatf(\"AAA\"), $sformatf(\"BBB\")); #1 $finish; end endmodule\n",
    );
    assert_ne!(c, Some(0), "surplus value arg must stay loud; got:\n{out}");
}

#[test]
fn empty_format_stays_loud() {
    // An empty literal format has zero specifiers → the value arg is surplus → LOUD.
    let (out, c) = run(
        "module m; initial begin $display(\"\", $sformatf(\"%0d\", 42)); \
         #1 $finish; end endmodule\n",
    );
    assert_ne!(
        c,
        Some(0),
        "empty format + value arg must stay loud; got:\n{out}"
    );
}

#[test]
fn non_consuming_specifiers_not_counted() {
    // `%m` (scope) and `%%` (literal) consume no arg, so `$display("%m %m", $sformatf)`
    // has a surplus arg → LOUD (a counter that miscounted %m as consuming would wrongly
    // hoist it into the numeric-render gap).
    let (out, c) = run(
        "module t; initial begin $display(\"%m %m\", $sformatf(\"x\")); \
         #1 $finish; end endmodule\n",
    );
    assert_ne!(c, Some(0), "%m-only surplus must stay loud; got:\n{out}");
}

#[test]
fn plain_display_unchanged() {
    // Byte-identity guard: a format task with NO $sformatf arg is untouched (the
    // hoist branch's entry gate is a strict no-op).
    let (out, c) = run("module t; int x=42; initial begin \
         $display(\"x=%0d y=%0h z=%b\", x, x, x[3:0]); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("x=42 y=2a z=1010"), "got:\n{out}");
}
