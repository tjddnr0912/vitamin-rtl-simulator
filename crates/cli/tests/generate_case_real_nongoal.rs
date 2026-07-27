//! A `generate case` whose scrutinee is a REAL is rejected by BOTH vita and
//! iverilog 13.0 ("Cannot evaluate genvar case expression: R"), so it is a
//! non-goal rather than a gap — recorded here because §4.5.242 left it on the
//! remaining list and this settles it.
//!
//! §4.5.241/242 routed generate-scope declarations and generate if/for CONDITIONS
//! through the real domain, which raised the fair question of whether the case
//! scrutinee should follow. It should not: the oracle refuses the same source, so
//! there is nothing to converge on, and vita's loud is honest. The integer
//! scrutinee — which does work and is what real code uses — is pinned alongside
//! so the loud on the real form can never quietly widen to cover it.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_gcn_{}_{n}", std::process::id()));
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

const CASE: &str = "module m;\n  localparam {ty} S = {val};\n  generate case (S)\n\
       {l1}: begin : a initial $display(\"S=a\"); end\n\
       {l2}: begin : b initial $display(\"S=b\"); end\n\
       default: begin : c initial $display(\"S=c\"); end\n\
     endcase endgenerate\n  initial #1 $finish;\nendmodule\n";

/// The integer form works and picks the matching arm.
#[test]
fn an_integer_generate_case_selects_its_arm() {
    let src = CASE
        .replace("{ty}", "int")
        .replace("{val}", "2")
        .replace("{l1}", "2")
        .replace("{l2}", "3");
    let (out, c) = run(&src);
    assert_eq!(c, Some(0), "integer generate-case; got:\n{out}");
    assert!(out.contains("S=a"), "matching arm; got:\n{out}");
}

/// The REAL form is loud, matching iverilog, which refuses it as well.
#[test]
fn a_real_generate_case_scrutinee_stays_loud() {
    let src = CASE
        .replace("{ty}", "real")
        .replace("{val}", "2.5")
        .replace("{l1}", "2.5")
        .replace("{l2}", "3.5");
    let (out, c) = run(&src);
    assert_ne!(
        c,
        Some(0),
        "iverilog refuses it too — loud is correct; got:\n{out}"
    );
}
