//! casez/casex don't-care precision (v7 BinOp::CasezEq/CasexEq): casez treats
//! ONLY z (either side) as don't-care — an x bit compares 4-state exact, so
//! `casez(1x10)` vs `1010` is a strict MISS (the v1 redor(xor) formula matched
//! it). casex keeps x AND z as don't-care. All expectations pinned LIVE
//! against iverilog 13.0 (2026-06-12).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_czp_{}_{n}", std::process::id()));
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
        out.status.code(),
    )
}

#[test]
fn casez_x_scrutinee_is_strict_miss() {
    // s=1x10: label 1010 must MISS (x is not a casez wildcard); label 1z10
    // MATCHES via the label's z. iverilog: B_match1z10.
    let (out, err, code) = run("module top;\n\
         reg [3:0] s;\n\
         initial begin\n\
           s = 4'b1x10;\n\
           casez (s)\n\
             4'b1010: $display(\"A_match1010\");\n\
             4'b1z10: $display(\"B_match1z10\");\n\
             default: $display(\"C_default\");\n\
           endcase\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("B_match1z10"), "got:\n{out}");
    assert!(!out.contains("A_match1010"), "got:\n{out}");
}

#[test]
fn casez_z_scrutinee_is_dont_care() {
    // s=1z10 matches label 1010 (scrutinee z is don't-care). iverilog: D.
    let (out, err, code) = run("module top;\n\
         reg [3:0] s;\n\
         initial begin\n\
           s = 4'b1z10;\n\
           casez (s)\n\
             4'b1010: $display(\"D_match1010\");\n\
             default: $display(\"E_default\");\n\
           endcase\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("D_match1010"), "got:\n{out}");
}

#[test]
fn casez_explicit_x_label_compares_exact() {
    // Label 0x00: against s=0000 it MISSES (x vs 0 differs, x is not a casez
    // wildcard); against s=0x00 it MATCHES (x === x). iverilog: G then H.
    let (out, err, code) = run("module top;\n\
         reg [3:0] s;\n\
         initial begin\n\
           s = 4'b0000;\n\
           casez (s)\n\
             4'b0x00: $display(\"F_xlabel\");\n\
             default: $display(\"G_default\");\n\
           endcase\n\
           s = 4'b0x00;\n\
           casez (s)\n\
             4'b0x00: $display(\"H_xlabel_xscrut\");\n\
             default: $display(\"I_default\");\n\
           endcase\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("G_default"), "got:\n{out}");
    assert!(out.contains("H_xlabel_xscrut"), "got:\n{out}");
    assert!(!out.contains("F_xlabel"), "got:\n{out}");
}

#[test]
fn casex_x_scrutinee_still_matches() {
    // casex keeps x/z don't-care on either side (behavior unchanged from v1).
    // iverilog: J_casex_match.
    let (out, err, code) = run("module top;\n\
         reg [3:0] s;\n\
         initial begin\n\
           s = 4'b1x10;\n\
           casex (s)\n\
             4'b1010: $display(\"J_casex_match\");\n\
             default: $display(\"K_default\");\n\
           endcase\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("J_casex_match"), "got:\n{out}");
}

#[test]
fn casez_question_label_and_plain_case_regression() {
    // `?` in a casez label = z (don't-care); plain `case` stays 4-state exact.
    // iverilog: Q_match then P_exact.
    let (out, err, code) = run("module top;\n\
         reg [3:0] s;\n\
         initial begin\n\
           s = 4'b1110;\n\
           casez (s)\n\
             4'b1?10: $display(\"Q_match\");\n\
             default: $display(\"R_default\");\n\
           endcase\n\
           s = 4'b1x10;\n\
           case (s)\n\
             4'b1x10: $display(\"P_exact\");\n\
             default: $display(\"S_default\");\n\
           endcase\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("Q_match"), "got:\n{out}");
    assert!(out.contains("P_exact"), "got:\n{out}");
}
