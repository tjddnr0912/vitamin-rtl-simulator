//! 2-state integer-type variables (`bit`/`byte`/`shortint`/`int`/`longint`).
//! Two IEEE rules that the formal-local rework surfaced:
//!  (1) §6.8: a VARIABLE declaration initializer (`int x = 5;`) is a one-time
//!      value applied at creation — NOT an implicit continuous assign. The prior
//!      `is_var` list omitted the 2-state kinds, so `int x = 5` was wired as a
//!      continuous driver that re-applied 5 every settle, discarding later
//!      procedural writes (a silent-wrong; iverilog keeps the last write).
//!  (2) §6.11.3: a 2-state variable can never hold X/Z; X/Z bits convert to 0 on
//!      assignment. (Covered for task formals in task_output_copyout.rs.)
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_2state_{}_{n}", std::process::id()));
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

// ── §6.11.1: an explicit `unsigned` on a 2-state atom type makes it UNSIGNED ──
#[test]
fn unsigned_atom_compares_and_prints_unsigned() {
    let o = run("module top;\n\
         int unsigned a; int unsigned b;\n\
         initial begin\n\
           a = 32'hFFFF_FFF0; b = 32'd16;\n\
           $display(\"lt=%0d a=%0d\", (a < b), a);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    // unsigned: a (4294967280) > b (16) → lt=0, and a prints unsigned.
    assert!(
        o.contains("lt=0") && o.contains("a=4294967280"),
        "`int unsigned` must compare and print as unsigned, not signed:\n{o}"
    );
}

#[test]
fn signed_atom_default_and_explicit_signed_unchanged() {
    // bare `int` (default signed) and `int signed` both stay signed.
    let o = run("module top;\n\
         int a; int signed b;\n\
         initial begin\n\
           a = -16; b = -16;\n\
           $display(\"a=%0d b=%0d lt=%0d\", a, b, (a < 0));\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        o.contains("a=-16 b=-16 lt=1"),
        "default `int` and `int signed` stay signed:\n{o}"
    );
}

// ── §6.8: an `int` initializer is a one-time value, not a continuous driver ──
#[test]
fn int_initializer_is_not_continuous_driver() {
    let o = run("module top;\n\
         int x = 5;\n\
         initial begin #1 x = 7; #1 $display(\"x=%0d\", x); $finish; end\n\
         endmodule\n");
    assert!(
        o.contains("x=7"),
        "a later procedural write must stick; the init must not re-apply:\n{o}"
    );
}

// ── a non-literal CONSTANT initializer (param / expression) is folded into the
//    one-time init value (was X/0 — the prior fold only handled literals) ──
#[test]
fn param_and_expr_initializers_fold() {
    // 2-state and 4-state, param and expression initializers.
    for (decl, want) in [
        ("parameter P=42; int x=P;", "x=42"),
        ("int x=3+4;", "x=7"),
        ("parameter P=42; logic [31:0] x=P;", "x=42"),
        ("parameter P=42; reg [31:0] x=P;", "x=42"),
        ("localparam Q=8'hA5; byte x=Q;", "x=-91"), // 0xA5 as signed byte = -91
    ] {
        let src = format!(
            "module top;\n{decl}\ninitial begin $display(\"x=%0d\", x); $finish; end\nendmodule\n"
        );
        let o = run(&src);
        assert!(o.contains(want), "init `{decl}` must fold to {want}:\n{o}");
    }
}

#[test]
fn byte_shortint_longint_initializers_are_one_time() {
    for (ty, init, set, want) in [
        ("byte", "8'sd5", "8'sd9", "x=9"),
        ("shortint", "16'sd5", "16'sd9", "x=9"),
        ("longint", "64'sd5", "64'sd9", "x=9"),
        ("bit [7:0]", "8'd5", "8'd9", "x=9"),
    ] {
        let src = format!(
            "module top;\n\
             {ty} x = {init};\n\
             initial begin #1 x = {set}; #1 $display(\"x=%0d\", x); $finish; end\n\
             endmodule\n"
        );
        let o = run(&src);
        assert!(
            o.contains(want),
            "{ty} init must be one-time (want {want}):\n{o}"
        );
    }
}
