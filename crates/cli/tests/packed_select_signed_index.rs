//! A NARROW SIGNED index into a plain `[N:0]` vector carries its sign — the
//! missing container of the rule `const_index_sign.rs` pins for an array WORD.
//! ROADMAP §2 "Index sealing".
//!
//! `norm_offset_for_net` has three geometries. A non-zero declared LSB
//! (`logic [9:2]`) and an ascending or negative-bound net all normalize through
//! `norm_sub_k` / `norm_k_sub`, which seal the index and were correct. The
//! `lsb == 0` arm subtracts nothing, so it returned the index expression RAW —
//! and the engine reads an index with `to_u64`, so a signed value narrower than
//! the 32-bit index domain arrived POSITIVE.
//!
//! On `logic [7:0] pv = 8'b1010_0101`, against iverilog 13.0:
//!
//! | index | vita PRE | iverilog |
//! |---|---|---|
//! | `pv[-2'sd1]` | `0` (bit 3) | `x` |
//! | `pv[3'sd7]` | `1` (bit 7) | `x` |
//! | `pv[s]`, `logic signed [1:0] s = -1` | `0` | `x` |
//! | `pv[3'sd7 -: 2]` | `2` | `x` |
//!
//! ⚠️ The class hides behind an accidental immunity: it is visible only where the
//! UNSIGNED reading also lands inside the net. On an 8-bit net that is widths 2
//! and 3 (`-4'sd1` reads 15, already out of range and already `x`); on a 64-bit
//! net it is widths 2 through 6. Sweeping ONE width would have found nothing.
//!
//! ⚠️ UNSIGNED indices are untouched on purpose. The width-pinning half of the
//! seal is already right in this arm — `pv[~r3]` and `pv[~r5]` agree with
//! iverilog at HEAD — so sealing them would re-emit every `[N:0]` select in every
//! design to change nothing.
//!
//! verilator is not the value oracle here: it has no `x` for an out-of-range
//! select and reads a bit for every cell below. It IS the accept/reject oracle,
//! and it agrees the access is out of range — it refuses the source outright with
//! `%Warning-SELRANGE: Selection index out of range` unless that is waived.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(body: &str) -> (String, Option<i32>) {
    let src =
        format!("`timescale 1ns/1ns\nmodule top;\n{body}\n  initial #20 $finish;\nendmodule\n");
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_psi_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, &src).unwrap();
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

#[test]
fn a_negative_constant_index_into_a_plain_vector_reads_x() {
    // iverilog: every one `x`. PRE: `-2'sd1` read bit 3 and `3'sd7` bit 7, both at
    // exit 0 with no diagnostic. `-8'sd1` and `8'sd255` were correct only because
    // 255 is out of range anyway — they are the accidental-immunity controls.
    let (out, rc) = run("  logic [7:0] pv;\n  logic a, b, c, d, e;\n  \
         initial begin pv = 8'b1010_0101; #1;\n    \
         a = pv[-2'sd1]; b = pv[3'sd7]; c = pv[-8'sd1]; d = pv[8'sd255];\n    \
         e = pv[-2'sd1 + 2'sd0];\n    \
         $display(\"a=%b b=%b c=%b d=%b e=%b\", a, b, c, d, e); end");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("a=x b=x c=x d=x e=x"), "{out}");
}

#[test]
fn the_visible_band_is_where_the_unsigned_reading_lands_in_range() {
    // The census band is its operands: on an 8-bit net only widths 2 and 3 were
    // ever wrong, on a 64-bit net widths 2..6. Both sweeps are `x` throughout in
    // iverilog.
    let mut body = String::from("  logic [7:0] pv;\n  logic [63:0] wv;\n");
    for w in [2u32, 3, 4, 5, 6, 7, 8, 16, 32] {
        body.push_str(&format!("  logic n{w}, m{w};\n"));
    }
    body.push_str("  initial begin pv = 8'b1010_0101; wv = 64'hF0F0_F0F0_F0F0_F0F0; #1;\n");
    for w in [2u32, 3, 4, 5, 6, 7, 8, 16, 32] {
        body.push_str(&format!("    n{w} = pv[-{w}'sd1]; m{w} = wv[-{w}'sd1];\n"));
    }
    body.push_str("    $display(\"n=%b%b%b%b%b%b%b%b%b m=%b%b%b%b%b%b%b%b%b\",");
    body.push_str(" n2,n3,n4,n5,n6,n7,n8,n16,n32, m2,m3,m4,m5,m6,m7,m8,m16,m32); end");
    let (out, rc) = run(&body);
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("n=xxxxxxxxx m=xxxxxxxxx"), "{out}");
}

#[test]
fn a_signed_net_index_is_sealed_too_and_an_unsigned_one_is_not() {
    // The defect is not constant-only: a signed NET index of the same width was
    // equally raw. `s32` was correct already (its width IS the index domain), and
    // the UNSIGNED net holding the same bits must keep reading bit 3 — all three
    // tools read `pv[3]` there, and moving it would be a fresh silent-wrong.
    let (out, rc) = run(
        "  logic [7:0] pv;\n  logic signed [1:0] s2;\n  logic signed [2:0] s3;\n  \
         logic signed [31:0] s32;\n  logic [1:0] u2;\n  logic a, b, c, d;\n  \
         initial begin pv = 8'b1010_0101; s2 = -1; s3 = -1; s32 = -1; u2 = 2'b11; #1;\n    \
         a = pv[s2]; b = pv[s3]; c = pv[s32]; d = pv[u2];\n    \
         $display(\"a=%b b=%b c=%b d=%b\", a, b, c, d); end",
    );
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("a=x b=x c=x d=0"), "{out}");
}

#[test]
fn a_part_select_base_shares_the_funnel() {
    // `-:` and `+:` normalize their base through the same arm, so both moved with
    // the bit-select. iverilog prints `x` and `X` for these two respectively.
    let (out, rc) = run("  logic [7:0] pv;\n  logic [1:0] p1, p2;\n  \
         initial begin pv = 8'b1010_0101; #1;\n    \
         p1 = pv[3'sd7 -: 2]; p2 = pv[-2'sd1 +: 2];\n    \
         $display(\"p1=%h p2=%h\", p1, p2); end");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("p1=x p2=X"), "{out}");
}

#[test]
fn the_unsigned_width_pin_and_the_other_geometries_are_unmoved() {
    // Controls that must not move, all already correct at HEAD.
    //
    // `~r3` / `~r5`: the context-determined `~` evaluates at its own width in this
    // arm already (iverilog 1 and `x`), which is why the fix leaves unsigned
    // indices alone instead of sealing them.
    //
    // `qv[…]` on a `logic [9:2]`: a non-zero LSB goes through `norm_sub_k`, which
    // sealed all along — this is the sibling spelling that was right and told us
    // which arm was missing it.
    let (out, rc) = run(
        "  logic [7:0] pv;\n  logic [9:2] qv;\n  logic [2:0] r3;\n  logic [4:0] r5;\n  \
         logic a, b, c, d, e;\n  \
         initial begin pv = 8'b1010_0101; qv = 8'b1010_0101; r3 = 3'd2; r5 = 5'd2; #1;\n    \
         a = pv[~r3]; b = pv[~r5]; c = qv[-1]; d = qv[1]; e = pv[3'd7];\n    \
         $display(\"a=%b b=%b c=%b d=%b e=%b\", a, b, c, d, e); end",
    );
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("a=1 b=x c=x d=x e=1"), "{out}");
    // The WRITE twin was already correct — iverilog drops it and so did vita, so
    // the read-side fix must leave the net untouched.
    let (out, rc) = run("  logic [7:0] wr;\n  \
         initial begin wr = 8'h00; #1; wr[-2'sd1] = 1'b1;\n    \
         $display(\"wr=%b\", wr); end");
    assert_eq!(rc, Some(0), "{out}");
    assert!(out.contains("wr=00000000"), "{out}");
}
