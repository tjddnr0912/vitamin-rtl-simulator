//! Generate-for STEP with increment/decrement/compound operators (IEEE 1800 §27.4
//! genvar_iteration): `for (g = 0; g < 3; g++)`, `++g`, `g--`, `--g`, `g += e`,
//! `g *= e`, etc. vita previously accepted only `g = expr` for the step and
//! parse-rejected the operator forms (E2002 "expected '=' in generate-for"). The
//! fix desugars each to `g = g <op> operand` (a literal `1` for `++`/`--`), reusing
//! the procedural for-step operators — byte-identical to the explicit `g = g + …`
//! form. The INIT stays `g = expr` only (`for (g++; …)` is invalid SV → stays loud).
//! Pure parser (reuses `GenAssign`/`Binary`/`Ident`), so the AST/`.vu` schema hash
//! and format_version are unchanged. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_genfor_{}_{n}", std::process::id()));
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
fn postfix_incr() {
    let (out, code) = run("module m; genvar g;\n\
         generate for (g = 0; g < 3; g++) begin: b initial $display(\"g=%0d\", g); end endgenerate\n\
         initial #1 $finish; endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("g=0") && out.contains("g=1") && out.contains("g=2"),
        "{out}"
    );
}

#[test]
fn prefix_incr() {
    // `++g` (prefix) — inc_or_dec_operator genvar_identifier.
    let (out, code) = run("module m; genvar g;\n\
         generate for (g = 0; g < 3; ++g) begin: b initial $display(\"g=%0d\", g); end endgenerate\n\
         initial #1 $finish; endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("g=0") && out.contains("g=1") && out.contains("g=2"),
        "{out}"
    );
}

#[test]
fn postfix_decr() {
    let (out, code) = run("module m; genvar g;\n\
         generate for (g = 3; g > 0; g--) begin: b initial $display(\"g=%0d\", g); end endgenerate\n\
         initial #1 $finish; endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("g=3") && out.contains("g=2") && out.contains("g=1"),
        "{out}"
    );
}

#[test]
fn compound_add() {
    let (out, code) = run("module m; genvar g;\n\
         generate for (g = 0; g < 4; g += 2) begin: b initial $display(\"g=%0d\", g); end endgenerate\n\
         initial #1 $finish; endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("g=0") && out.contains("g=2") && !out.contains("g=1"),
        "{out}"
    );
}

#[test]
fn compound_mul() {
    // `g *= 2` — geometric step. iverilog: 1,2,4,8.
    let (out, code) = run("module m; genvar g;\n\
         generate for (g = 1; g < 9; g *= 2) begin: b initial $display(\"g=%0d\", g); end endgenerate\n\
         initial #1 $finish; endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("g=1") && out.contains("g=8"), "{out}");
}

#[test]
fn compound_add_param_rhs() {
    // `g += S` with a param rhs (the operand is a general genvar_expression).
    let (out, code) = run("module m; genvar g; localparam S = 3;\n\
         generate for (g = 0; g < 9; g += S) begin: b initial $display(\"g=%0d\", g); end endgenerate\n\
         initial #1 $finish; endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("g=0") && out.contains("g=3") && out.contains("g=6"),
        "{out}"
    );
}

#[test]
fn nested_genfor_incr() {
    let (out, code) = run("module m; genvar i, j;\n\
         generate for (i = 0; i < 2; i++) begin: oi\n\
         for (j = 0; j < 2; j++) begin: ij initial $display(\"i=%0d j=%0d\", i, j); end\n\
         end endgenerate\n\
         initial #1 $finish; endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("i=0 j=0") && out.contains("i=1 j=1"), "{out}");
}

#[test]
fn genfor_incr_generates_hardware() {
    // The `g++` unroll must produce the same hardware as `g = g + 1`. iverilog: 1010.
    let (out, code) = run("module m; genvar g; wire [3:0] w;\n\
         generate for (g = 0; g < 4; g++) begin: b assign w[g] = g[0]; end endgenerate\n\
         initial begin #1 $display(\"w=%b\", w); $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("w=1010"), "{out}");
}

#[test]
fn incr_byte_identical_to_explicit() {
    // `g++` must produce byte-identical stdout to the explicit `g = g + 1` step.
    let pp = run("module m; genvar g;\n\
         generate for (g = 0; g < 5; g++) begin: b initial $display(\"g=%0d\", g); end endgenerate\n\
         initial #1 $finish; endmodule\n");
    let ex = run("module m; genvar g;\n\
         generate for (g = 0; g < 5; g = g + 1) begin: b initial $display(\"g=%0d\", g); end endgenerate\n\
         initial #1 $finish; endmodule\n");
    assert_eq!(pp.1, Some(0));
    assert_eq!(
        pp.0, ex.0,
        "g++ must be byte-identical to g=g+1:\n{}\n---\n{}",
        pp.0, ex.0
    );
}

#[test]
fn init_incr_is_loud() {
    // `for (g++; …)` — an inc/dec form is NOT valid in the INIT (only `g = expr`);
    // it must stay a loud error, not be silently accepted.
    let (out, code) = run("module m; genvar g;\n\
         generate for (g++; g < 3; g = g + 1) begin: b initial $display(\"g=%0d\", g); end endgenerate\n\
         initial #1 $finish; endmodule\n");
    assert_ne!(code, Some(0), "init `g++` must be loud:\n{out}");
}
