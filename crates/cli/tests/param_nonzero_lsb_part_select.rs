//! A localparam/parameter declared with a NON-zero LSB (`localparam [15:8] P`)
//! read a RAW offset on a bit/part/indexed READ (`P[15:12]`, `P[9]`, `P[8+:4]`),
//! because the base folds to a Const (not a net) and `norm_offset_if_net`'s
//! `lookup_net_scoped` returned `None` → the declared LSB (8) was never subtracted
//! → silent X. The declared range is recorded in `param_range` (resolved by the
//! SAME `walk_scopes` as the param value/meta) and normalized via
//! `norm_offset_for_range` — the param twin of a net's `norm_offset_for_net`. A
//! zero-LSB param (`[N:0]`) and the whole-param read are byte-identical (no entry).
//!
//! Every value is pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pnz_{}_{n}", std::process::id()));
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
fn localparam_nonzero_lsb_read_sub_selects() {
    // P = 5A on [15:8]: [15:12]=5, [11:8]=A, bit9=1, [8+:4]=A.
    let (out, c) = run("module m;\n\
         localparam [15:8] P = 8'h5A;\n\
         initial begin $display(\"%h %h %b %h\", P[15:12], P[11:8], P[9], P[8+:4]); #1 $finish; end\n\
       endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("5 a 1 a"),
        "localparam non-zero-LSB read:\n{out}"
    );
}

#[test]
fn parameter_nonzero_lsb_read() {
    let (out, c) = run("module m #(parameter [15:8] Q = 8'h3C) ();\n\
         initial begin $display(\"%h %h\", Q[15:12], Q[11:8]); #1 $finish; end\n\
       endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("3 c"), "parameter non-zero-LSB read:\n{out}");
}

#[test]
fn localparam_lsb4_indices_below_width() {
    // `[11:4]` (lo=4, width 8): the select indices 7..4 are BELOW the width, so a
    // width-proxy guard would miss it — the declared LSB must be subtracted.
    let (out, c) = run("module m;\n\
         localparam [11:4] R = 8'hE7;\n\
         initial begin $display(\"%h %h\", R[7:4], R[11:8]); #1 $finish; end\n\
       endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("7 e"), "localparam [11:4] read:\n{out}");
}

#[test]
fn localparam_ascending_read() {
    // ascending `[8:15]`: A[8:11] = high nibble = 5, A[8+:4] = 5.
    let (out, c) = run("module m;\n\
         localparam [8:15] A = 8'h5A;\n\
         initial begin $display(\"%h %h\", A[8:11], A[8+:4]); #1 $finish; end\n\
       endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("5 5"), "localparam ascending read:\n{out}");
}

#[test]
fn localparam_runtime_index() {
    let (out, c) = run("module m;\n\
         localparam [15:8] P = 8'h5A; integer k;\n\
         initial begin k = 9; $display(\"%b\", P[k]); #1 $finish; end\n\
       endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("1"), "localparam runtime index:\n{out}");
}

#[test]
fn zero_lsb_param_and_whole_read_unchanged() {
    // Zero-LSB param: no `param_range` entry → raw offset (already correct). Whole
    // read unaffected.
    let (out, c) = run("module m;\n\
         localparam [7:0] Z = 8'hE7; localparam [15:8] P = 8'h5A;\n\
         initial begin $display(\"%h %h %h\", Z[7:4], Z[3:0], P); #1 $finish; end\n\
       endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("e 7 5a"),
        "zero-LSB param + whole read:\n{out}"
    );
}

#[test]
fn inline_function_local_shadowing_param_not_drifted() {
    // R2: an INLINE function local shadowing a module non-zero-LSB param must read
    // the LOCAL (value AND offset), not subtract the param's LSB. Was: base 3 → fix
    // x (net-agreement drift). `param_sel_range` re-derives the innermost binding.
    let (out, c) = run("module t;\n\
         localparam [15:8] P = 8'hA5;\n\
         function [3:0] f; reg [7:0] P; begin P = 8'h3C; f = P[7:4]; end endfunction\n\
         initial begin $display(\"f=%h\", f()); #1 $finish; end\n\
       endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("f=3"),
        "inline-local shadow must not drift:\n{out}"
    );
}

#[test]
fn generate_local_shadow_and_real_param_both_correct() {
    // The real param read is correct AND a same-named zero-LSB generate/block local
    // is not mis-normalized (R2 t5_mixed / t_drift_A class).
    let (out, c) = run("module t;\n\
         localparam [15:8] P = 8'hA5;\n\
         function [3:0] real_; real_ = P[15:12]; endfunction\n\
         function [3:0] shad; reg [7:0] P; begin P = 8'h3C; shad = P[7:4]; end endfunction\n\
         initial begin $display(\"r=%h s=%h\", real_(), shad()); #1 $finish; end\n\
       endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("r=a s=3"),
        "real param + shadowed local:\n{out}"
    );
}
