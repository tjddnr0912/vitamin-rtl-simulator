//! `W-PARSE-SELECT-BASE` — a bit/part select on something IEEE 1800-2017 §11.5.1
//! does not allow one on.
//!
//! External aes_top round-3 §3.1. The reporter's most expensive class: vita accepts
//! these, so a design is green here and fails at sign-off. §11.5.1 allows a select
//! only on a VARIABLE REFERENCE — a name, possibly narrowed by array indexing or
//! member access — and this parser attached one to any primary.
//!
//! WARNING, NOT REFUSAL, and the oracles are why. Measured:
//!
//!   ((a^b)>>8)[7:0]   vita ok   iverilog REJECT   verilator REJECT
//!   16'hABCD[7:0]     vita ok   iverilog REJECT   verilator REJECT
//!   f(a)[7:0]         vita ok   iverilog REJECT   verilator ok, same value
//!   {a,b}[7:0]        vita ok   iverilog REJECT   verilator ok, same value
//!
//! Refusing the last two would descend the ladder for code that is portable to
//! verilator today, so the value is left alone and the log says the spelling is not
//! portable — the policy §3.2 reached for `\r`.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sbp_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
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

fn design(expr: &str) -> String {
    format!(
        "module tb;\n\
         \x20 logic [15:0] a = 16'h1234, b = 16'h00FF; logic [7:0] m[0:3]; logic [7:0] o;\n\
         \x20 typedef struct packed {{ logic [7:0] hi, lo; }} P; P p;\n\
         \x20 function [15:0] f(input [15:0] x); f = x + 1; endfunction\n\
         \x20 initial begin m[1] = 8'h5A; p = 16'hAABB; o = {expr}; $display(\"o=%02h\", o); end\n\
         endmodule\n"
    )
}

fn warns(out: &str) -> usize {
    out.matches("VITA-W2004").count()
}

#[test]
fn a_select_on_a_non_variable_warns_once_and_keeps_its_value() {
    // The value column is the point: every one of these still produces exactly what
    // it produced before, so nothing that runs today stops running.
    for (expr, val) in [
        ("((a^b)>>8)[7:0]", "o=12"),
        ("f(a)[7:0]", "o=35"),
        ("{a,b}[7:0]", "o=ff"),
        ("16'hABCD[7:0]", "o=cd"),
        ("$signed(a)[7:0]", "o=34"),
    ] {
        let out = run(&design(expr));
        assert_eq!(warns(&out), 1, "`{expr}` must warn exactly once:\n{out}");
        assert!(
            out.contains(val),
            "`{expr}` must still produce {val}:\n{out}"
        );
        assert!(!out.contains("error["), "…and must not be an error:\n{out}");
    }
}

#[test]
fn a_select_on_a_variable_reference_is_silent() {
    // OVER-WARNING is the failure mode that would make this useless, and the middle
    // case is why the check cannot read the AST: a packed-struct member access is
    // DESUGARED to a part-select by the same parser loop, so `p.hi[3:0]` and
    // `a[7:0][3:0]` are the same shape by the time an AST exists. Only in the parser
    // is it still known that the chain began at a name.
    for (expr, val) in [
        ("a[7:0]", "o=34"),    // a plain name
        ("p.hi[3:0]", "o=0a"), // a packed-struct member (desugars to a part-select)
        ("m[1][3:0]", "o=0a"), // an unpacked-array element
    ] {
        let out = run(&design(expr));
        assert_eq!(
            warns(&out),
            0,
            "`{expr}` is a variable reference — no warning:\n{out}"
        );
        assert!(out.contains(val), "…and its value is unchanged:\n{out}");
    }
}

#[test]
fn the_warning_is_suppressible_and_promotable_like_any_other() {
    let src = design("((a^b)>>8)[7:0]");
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sbp_g_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, &src).unwrap();
    let go = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .arg(f.to_str().unwrap())
            .args(args)
            .current_dir(&d)
            .output()
            .expect("run vita");
        (
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
            out.status.code().unwrap_or(-1),
        )
    };
    let (plain, plain_rc) = go(&[]);
    assert_eq!(warns(&plain), 1);
    assert_eq!(
        plain_rc, 0,
        "a warning does not change the exit class:\n{plain}"
    );
    // Both spellings of the key work — the printed number and the mnemonic (§3.8).
    for flag in ["-Wno-W-PARSE-SELECT-BASE", "-Wno-VITA-W2004", "-Wno-W2004"] {
        let (q, _) = go(&[flag]);
        assert_eq!(warns(&q), 0, "`{flag}` must suppress it:\n{q}");
    }
    let (p, rc) = go(&["-Werror=W-PARSE-SELECT-BASE"]);
    assert!(p.contains("VITA-W2004"), "promoted, same code number:\n{p}");
    assert_ne!(rc, 0, "…and now it fails the run:\n{p}");
}

#[test]
fn the_warning_names_the_file_and_line() {
    // §3.10: a portability warning the reader cannot locate is a worse version of the
    // problem it reports.
    let out = run(&design("((a^b)>>8)[7:0]"));
    assert!(
        out.contains("t.sv:5:") || out.contains("t.sv:4:"),
        "the warning must carry file:line:col:\n{out}"
    );
}
