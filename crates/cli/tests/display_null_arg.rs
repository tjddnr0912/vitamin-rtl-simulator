//! §3.b `display-null-arg` (§4.5.575): a NULL argument of a display-family task — nothing
//! between two commas, or between a comma and a parenthesis — as in darkriscv
//! `darkram.v:72`, `$display("dpram: RMW cycle enabled.",);` (reached with upstream's
//! `__RMW_CYCLE__`). IEEE 1364-2005 §17.1.1 permits one and says it displays a single
//! space; it was `E2002 expected expression`.
//!
//! Measured on iverilog 13.0 (`-g2012`), sv2v 0.0.13 → iverilog 13.0 and verilator 5.050
//! (`--binary --timing`): a null argument behaves as the string literal `" "` under every
//! format specifier (`%d` of it is ` 32`, `%h` `20`, `%b` `00100000`, `%c` a space), and
//! vita now parses it as that literal. Each value cell is pinned to the line all three
//! print unless its comment says otherwise. A null file descriptor, and a null argument
//! of any other task, keep the parse error.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dna_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (all, out.status.code())
}

/// `body` inside `initial begin … end` of a module with `reg [7:0] v = 8'h5a`.
fn design(body: &str) -> String {
    format!("module t;\n  reg [7:0] v = 8'h5a;\n  initial begin\n    {body}\n  end\nendmodule\n")
}

/// The design's own output lines (a clean run, exit 0): vita's diagnostics and status
/// lines dropped. Trailing spaces are kept — they are the value under test.
fn runs(body: &str) -> Vec<String> {
    let src = design(body);
    let (out, rc) = run(&src);
    assert_eq!(rc, Some(0), "expected a clean run\n{src}\n{out}");
    out.lines()
        .filter(|l| {
            !l.contains("[VITA-") && !l.starts_with("simulation ended") && !l.starts_with("errors=")
        })
        .map(str::to_string)
        .collect()
}

fn is_parse_error(body: &str) {
    let src = design(body);
    let (out, rc) = run(&src);
    assert_eq!(rc, Some(1), "expected a refusal\n{src}\n{out}");
    assert!(out.contains("VITA-E2002"), "expected E2002\n{out}");
}

#[test]
fn darkram_line_72() {
    assert_eq!(
        runs(r#"$display("dpram: RMW cycle enabled.",);"#),
        ["dpram: RMW cycle enabled. "]
    );
}

#[test]
fn null_between_and_around_values() {
    assert_eq!(runs(r#"$display("A",,"B");"#), ["A B"]);
    assert_eq!(runs("$display(v,,v);"), [" 90  90"]);
    assert_eq!(runs("$display(v,);"), [" 90 "]);
    assert_eq!(runs(r#"$display("",,"");"#), [" "]);
    // verilator 5.050 prints `A` (no space); iverilog and sv2v → iverilog print ` A`.
    assert_eq!(runs(r#"$display(,"A");"#), [" A"]);
    // verilator 5.050 rejects `(,)` as a syntax error; iverilog and sv2v → iverilog.
    assert_eq!(runs("$display(,);"), ["  "]);
    // `()` is still no arguments: an empty line.
    assert_eq!(runs("$display();"), [""]);
}

#[test]
fn null_consumed_by_a_format_specifier() {
    assert_eq!(runs(r#"$display("%d",v,);"#), [" 90 "]);
    assert_eq!(runs(r#"$display("%d%d",v,,v);"#), [" 90 32 90"]);
    assert_eq!(runs(r#"$display("%d",,v);"#), [" 32 90"]);
    assert_eq!(runs(r#"$display("x%0dy",,);"#), ["x32y "]);
    assert_eq!(runs(r#"$display("%s",,"q");"#), [" q"]);
    assert_eq!(runs(r#"$display("%c",);"#), [" "]);
    assert_eq!(runs(r#"$display("%h",,);"#), ["20 "]);
    assert_eq!(runs(r#"$display("%b",);"#), ["00100000"]);
    assert_eq!(runs(r#"$display("%0t",);"#), ["32"]);
    assert_eq!(runs(r#"$display("%x|",,);"#), ["20| "]);
}

#[test]
fn radix_and_family_variants() {
    assert_eq!(runs("$displayh(v,,v);"), ["5a 5a"]);
    assert_eq!(runs("$displayb(v,,);"), ["01011010  "]);
    assert_eq!(runs(r#"$write("W",); $display;"#), ["W "]);
    assert_eq!(runs(r#"$strobe("S",);"#), ["S "]);
    assert_eq!(
        runs(r#"$monitor("M",v,); #1 v = 1; #1 $finish;"#),
        ["M 90 ", "M  1 "]
    );
    assert_eq!(runs(r#"$fdisplay(32'h8000_0001,"F",);"#), ["F "]);
    // verilator 5.050 rejects a null right after the descriptor; iverilog and sv2v → iverilog.
    assert_eq!(runs(r#"$fwrite(32'h8000_0001,,"F"); $display;"#), [" F"]);
}

#[test]
fn null_descriptor_and_other_tasks_stay_refused() {
    // iverilog: "$fdisplay's file descriptor/MCD must be numeric".
    is_parse_error(r#"$fdisplay(,"x");"#);
    // Severity tasks are not display-family here: all three oracles accept these, but the
    // message line vita prints for them is its own, so this slice leaves them refused.
    is_parse_error(r#"$error("E",);"#);
    is_parse_error(r#"$info("I",);"#);
    is_parse_error(r#"$finish(,);"#);
}
