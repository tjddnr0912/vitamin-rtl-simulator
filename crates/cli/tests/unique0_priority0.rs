//! `unique0` / `priority0` case·if qualifiers (IEEE 1800 §12.4.2) —
//! hand-IEEE pinned: the `0` variants keep the qualifier's intent but
//! SUPPRESS the no-match violation report, so they parse as the plain
//! if/case with no synthetic `$warning` default/else injected. (Icarus is
//! not an oracle here: it rejects `unique0 if` outright and its runtime
//! reports "unique/unique0 qualities are ignored" on case — it cannot
//! distinguish the two.) Plain `unique`/`priority` keep the W4007 no-match
//! warning, pinned as a regression.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_u0_{}_{n}", std::process::id()));
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
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn unique0_suppresses_no_match_warning() {
    // No-match on unique0 case / if: silent fall-through, value untouched.
    let (out, err, code) = run("module t; logic [3:0] op; logic [7:0] y; initial begin\n\
           op = 4'hF; y = 0;\n\
           unique0 case (op) 4'hA: y=1; 4'hB: y=2; endcase\n\
           $display(\"case y=%0d\", y);\n\
           unique0 if (op==4'hB) y=4; else if (op==4'hC) y=5;\n\
           $display(\"if y=%0d\", y);\n\
           priority0 case (op) 4'hA: y=6; endcase\n\
           priority0 if (op==4'hA) y=7;\n\
           $display(\"p0 y=%0d\", y);\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(
        out.contains("case y=0"),
        "unique0 case no-match silent:\n{out}"
    );
    assert!(out.contains("if y=0"), "unique0 if no-match silent:\n{out}");
    assert!(out.contains("p0 y=0"), "priority0 no-match silent:\n{out}");
    assert!(
        !err.contains("W4007"),
        "no W4007 from the 0-variants:\n{err}"
    );
}

#[test]
fn unique0_matching_behaves_like_plain() {
    let (out, _, code) = run("module t; logic [3:0] op; logic [7:0] y; initial begin\n\
           op = 4'hA; y = 0;\n\
           unique0 case (op) 4'hA: y=1; 4'hB: y=2; endcase\n\
           $display(\"m=%0d\", y);\n\
           unique0 if (op==4'hB) y=4; else if (op==4'hA) y=5;\n\
           $display(\"i=%0d\", y);\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("m=1"), "unique0 case match:\n{out}");
    assert!(out.contains("i=5"), "unique0 if chain match:\n{out}");
}

#[test]
fn plain_unique_priority_still_warn_regression() {
    // The non-0 qualifiers keep their W4007 no-match warning — exactly one
    // per unhandled statement here (the two 0-variants add none).
    let (out, err, code) = run("module t; logic [3:0] op; logic [7:0] y; initial begin\n\
           op = 4'hF;\n\
           unique case (op) 4'hA: y=1; endcase\n\
           unique0 case (op) 4'hA: y=1; endcase\n\
           priority if (op==4'hA) y=2;\n\
           priority0 if (op==4'hA) y=3;\n\
           $display(\"done\");\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("done"), "{out}");
    assert_eq!(
        err.matches("W4007").count(),
        2,
        "exactly unique+priority warn (not the 0-variants):\n{err}"
    );
}

#[test]
fn explicit_default_and_else_are_untouched() {
    // A user-written default/else is never displaced by any qualifier.
    let (out, err, code) = run("module t; logic [3:0] op; logic [7:0] y; initial begin\n\
           op = 4'hF;\n\
           unique0 case (op) 4'hA: y=1; default: y=9; endcase\n\
           $display(\"d=%0d\", y);\n\
           unique case (op) 4'hA: y=1; default: y=8; endcase\n\
           $display(\"u=%0d\", y);\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("d=9"), "unique0 explicit default runs:\n{out}");
    assert!(out.contains("u=8"), "unique explicit default runs:\n{out}");
    assert!(!err.contains("W4007"), "explicit default = handled:\n{err}");
}

#[test]
fn casez_casex_variants_accept_the_qualifiers() {
    let (out, _, code) = run("module t; logic [3:0] op; logic [7:0] y; initial begin\n\
           op = 4'b1010; y = 0;\n\
           unique0 casez (op) 4'b1?1?: y=1; endcase\n\
           $display(\"z=%0d\", y);\n\
           priority0 casex (op) 4'b10x0: y=2; endcase\n\
           $display(\"x=%0d\", y);\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("z=1"), "unique0 casez:\n{out}");
    assert!(out.contains("x=2"), "priority0 casex:\n{out}");
}

#[test]
fn stray_qualifier_is_loud() {
    // A 0-variant not followed by if/case is a loud parse error (same as the
    // existing unique/priority behavior).
    let (_, err, code) = run("module t; initial begin\n\
           unique0 ;\n\
           end endmodule\n");
    assert_ne!(code, 0);
    assert!(
        err.contains("'if' or 'case'"),
        "stray qualifier loud:\n{err}"
    );
}
