//! §2 「다음 착수 순서」 new #1 — bound/count/index lanes fold at self width.
//!
//! Two spellings of one defect, plus the legality gate their fix exposed:
//!
//!   1. `const_bound_u32` DECLINED every width-inexact shape (its strong domain
//!      is width-unlimited), and the consumers' silent `unwrap_or(1)`/`unwrap_or(0)`
//!      defaults turned a `**` in a replication count (`{(8'd2 ** 8'd3){1'b1}}` →
//!      empty) or an indexed part-select width (`v[0 +: (8'd2 ** 8'd3)]` → 1 bit)
//!      into wrong values. A bound/count is its own context (§11.6.1), so a
//!      self-determined tier now folds what the unlimited tier must decline —
//!      gated on every foldable node having a KNOWN self width (the walk degrades
//!      to the unlimited domain where one is unknown, and an unlimited value in a
//!      wrap-sensitive bound would trade one silent default for another).
//!   2. The lowered tree's SHALLOW reduction is width-blind: `Const 4'd9 +
//!      Const 4'd8` reduced to 17, so `v[(4'd9+4'd8):0]` selected 18 bits where
//!      iverilog selects `v[1:0]`. `lower_index_expr` — the one funnel every
//!      index, bound, offset and width site shares (read AND write) — now hands
//!      the consumer the funnel's constant wherever the two DISAGREE.
//!   3. With counts folding width-honestly, `{(4'd15+4'd1){1'b1}}` becomes a
//!      ZERO-count replication — which §11.4.12.1 allows only as a direct
//!      concatenation operand. Bare zero replication is now loud (iverilog
//!      rejects it too); the in-concat drop and the string-replicate empty
//!      result are unchanged.
//!
//! Oracle: iverilog 13.0 for every cell here (they are unsigned/wrap cells where
//! it agrees); the write twins were measured against it as well.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (String, bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_sdbl_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    (
        s,
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run(src: &str) -> String {
    let (out, ok, err) = run_raw(src);
    assert!(ok, "expected success, stderr:\n{err}");
    out
}

fn loud(src: &str, needle: &str) {
    let (_, ok, err) = run_raw(src);
    assert!(!ok, "expected a loud reject");
    assert!(err.contains(needle), "unexpected diagnostic:\n{err}");
}

/// #1 headline: `**` in a replication count and an indexed part-select width.
/// The old gate declined both and the consumers silently defaulted (empty
/// replication / 1-bit select).
#[test]
fn pow_in_repl_count_and_idx_width() {
    let out = run("module top;\n\
         logic [15:0] v;\n\
         logic [7:0] r8;\n\
         initial begin\n\
           v = 16'habcd;\n\
           r8 = {(8'd2 ** 8'd3){1'b1}};   $display(\"A=%b\", r8);\n\
           $display(\"B=%h\", v[0 +: (8'd2 ** 8'd3)]);\n\
           $display(\"C=%h\", v[7 -: (4'd2 ** 4'd2)]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("A=11111111"), "got:\n{out}");
    assert!(out.contains("B=cd"), "got:\n{out}");
    assert!(out.contains("C=c"), "got:\n{out}");
}

/// #2 headline: wrapping constant bounds and indices — the shallow width-blind
/// reduction used to win. Read lanes: a range select whose msb wraps (narrow
/// and wide), a bit index, a -: base, an ascending-position lsb.
#[test]
fn wrapping_bounds_and_indices_read() {
    let out = run("module top;\n\
         logic [15:0] v;\n\
         initial begin\n\
           v = 16'habcd;\n\
           $display(\"A=%h\", v[(4'd9 + 4'd8) : 0]);\n\
           $display(\"B=%h\", v[(8'd200 + 8'd100) : 0]);\n\
           $display(\"C=%b\", v[4'd9 + 4'd8]);\n\
           $display(\"D=%h\", v[(4'd9+4'd8) -: 2]);\n\
           $display(\"E=%h\", v[5 : (4'd9+4'd8)]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("A=1"), "got:\n{out}");
    assert!(out.contains("B=xxxxxxxxabcd"), "got:\n{out}");
    assert!(out.contains("C=0"), "got:\n{out}");
    assert!(out.contains("D=1"), "got:\n{out}");
    assert!(out.contains("E=06"), "got:\n{out}");
}

/// The WRITE funnel rides the same `lower_index_expr`, so the write twins moved
/// with the read fix (measured against iverilog): a wrapped-msb range write, a
/// `**` indexed-part width write, and a wrapped bit-index write.
#[test]
fn wrapping_bounds_write_twins() {
    let out = run("module top;\n\
         logic [15:0] v;\n\
         logic [7:0] w8;\n\
         initial begin\n\
           v = 16'h0000;\n\
           v[(4'd9 + 4'd8) : 0] = 2'b11;    $display(\"W1=%h\", v);\n\
           v = 16'hffff;\n\
           v[0 +: (8'd2 ** 8'd3)] = 8'h5a;  $display(\"W2=%h\", v);\n\
           w8 = 8'h00;\n\
           w8[4'd9 + 4'd8] = 1'b1;          $display(\"W3=%h\", w8);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("W1=0003"), "got:\n{out}");
    assert!(out.contains("W2=ff5a"), "got:\n{out}");
    assert!(out.contains("W3=02"), "got:\n{out}");
}

/// `repeat` counts share the funnel: a wrapping count runs zero times in a
/// module context AND unrolls inside a frame (task automatic), where the
/// runtime-counter path used to be the only (loud) option.
#[test]
fn repeat_wrapping_count_module_and_frame() {
    let out = run("module top;\n\
         int cnt;\n\
         task automatic tfr();\n\
           int k;\n\
           k = 0;\n\
           repeat (8'd2 ** 8'd2) k++;\n\
           $display(\"F=%0d\", k);\n\
         endtask\n\
         initial begin\n\
           cnt = 0; repeat (4'd15 + 4'd1) cnt++;  $display(\"M=%0d\", cnt);\n\
           tfr();\n\
         end\n\
         endmodule\n");
    assert!(out.contains("M=0"), "got:\n{out}");
    assert!(out.contains("F=4"), "got:\n{out}");
}

/// §11.4.12.1: a zero replication count is legal only as a DIRECT concatenation
/// operand. Bare (a literal zero, and the folded wrap that now reaches the same
/// position) is loud; in-concat it contributes nothing; nested inside another
/// replication it is NOT a concat operand (the permission must not leak); a
/// string replicate keeps its empty-string result.
#[test]
fn zero_replication_context_rule() {
    loud(
        "module top;\n\
         logic [7:0] r;\n\
         initial begin r = {(0){1'b1}}; $display(\"Z=%b\", r); end\n\
         endmodule\n",
        "a replication count of zero is only legal as a direct operand",
    );
    loud(
        "module top;\n\
         logic [31:0] r;\n\
         initial begin r = {(4'd15 + 4'd1){1'b1}}; $display(\"Z=%b\", r); end\n\
         endmodule\n",
        "a replication count of zero is only legal as a direct operand",
    );
    loud(
        "module top;\n\
         logic [15:0] r;\n\
         initial begin r = {4'hA, {2{ {(0){1'b1}} }}, 4'hB}; $display(\"Z=%h\", r); end\n\
         endmodule\n",
        "a replication count of zero is only legal as a direct operand",
    );
    let out = run("module top;\n\
         logic [15:0] c;\n\
         string s;\n\
         initial begin\n\
           c = {4'hA, {(0){1'b1}}, 4'hB};  $display(\"K=%h\", c);\n\
           s = {\"ab\", {(0){\"x\"}}};      $display(\"S=[%s]\", s);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("K=00ab"), "got:\n{out}");
    assert!(out.contains("S=[ab]"), "got:\n{out}");
}

/// The permission is per-OPERAND, not per-subtree: a zero replication inside an
/// INDEX of a concat operand is not itself a concat operand. Verilator 5.050
/// rejects this with the same clause ("Replication value of 0 is only legal
/// under a concatenation, IEEE 11.4.12.1"); iverilog 13.0 leniently runs it.
#[test]
fn zero_replication_inside_concat_part_index_is_still_loud() {
    loud(
        "module top;\n\
         logic [15:0] v, r;\n\
         initial begin\n\
           v = 16'habcd;\n\
           r = {4'hB, v[{(0){1'b1}}]};\n\
           $display(\"N=%h\", r);\n\
         end\n\
         endmodule\n",
        "a replication count of zero is only legal as a direct operand",
    );
}

/// RESIDUAL MARKER (ROADMAP §2): a wrap-sensitive bound over a WIDTH-UNKNOWN
/// leaf (a const-array element — `const_self_width` has no arm for it) must
/// keep its pre-slice decline: the self-determined walk would DEGRADE to the
/// width-unlimited domain there and fold 300 where SV's 8-bit sum wraps to 44
/// (verilator answers the 44-bit select; iverilog cannot compile unpacked-array
/// parameters). Until the width model answers const-array elements, the silent
/// pre-slice 1-bit read stays — folding the unlimited 300 would trade one
/// silent-wrong for another.
#[test]
fn width_unknown_wrap_bound_keeps_preslice_decline() {
    let out = run("module top;\n\
         localparam bit [7:0] CA [2] = '{8'd200, 8'd100};\n\
         logic [63:0] v;\n\
         initial begin\n\
           v = 64'hffff_ffff_ffff_ffff;\n\
           $display(\"CW=%h\", v[0 +: (CA[0] + CA[1])]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("CW=1"), "got:\n{out}");
}

/// Unchanged neighbours stay byte-for-byte: literal counts/bounds, a plain
/// replication, a param-sized width, and a large repeat.
#[test]
fn non_wrapping_lanes_unchanged() {
    let out = run("module top;\n\
         localparam W = 8;\n\
         logic [15:0] v;\n\
         logic [7:0] r8;\n\
         logic [31:0] r32;\n\
         int cnt;\n\
         initial begin\n\
           v = 16'habcd;\n\
           r8 = {(2){4'b1010}};        $display(\"A=%b\", r8);\n\
           $display(\"B=%h\", v[3:0]);\n\
           $display(\"C=%h\", v[0 +: 8]);\n\
           r32 = {W{1'b1}};            $display(\"D=%h\", r32);\n\
           cnt = 0; repeat (16) cnt++; $display(\"E=%0d\", cnt);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("A=10101010"), "got:\n{out}");
    assert!(out.contains("B=d"), "got:\n{out}");
    assert!(out.contains("C=cd"), "got:\n{out}");
    assert!(out.contains("D=000000ff"), "got:\n{out}");
    assert!(out.contains("E=16"), "got:\n{out}");
}
