//! READ sub-selecting a packed-struct member — `s.f[i]` / `s.f[a:b]` /
//! `s.f[base+:w]` / `s.f[base-:w]`. The parser desugars `s.f` to the field
//! part-select `pv = s[off+w-1:off]`; a trailing sub-select becomes one
//! `IndexedPart` on `pv`, so elaborate's `IndexedPart`-on-`PartSelect` fold keeps
//! it FIELD-bounded (out-of-field bits read X — they never leak into an adjacent
//! member).
//!
//! For a DESCENDING member (`logic [7:0] f`) field index i = `pv[i]` (identity);
//! oracle = iverilog directly. For an ASCENDING member (`logic [0:7] f`) field
//! index i = `pv[w-1-i]`, so `+:`/`-:` flip and the offset mirrors — vita
//! previously read the field as descending (silent-wrong: `a[0+:4]`=5 vs IEEE
//! `a`). iverilog is itself BUGGY on ascending struct fields, so the oracle is
//! the equivalent ascending NET `logic [0:7] a` (which iverilog handles
//! correctly — a struct field must match it).
//!
//! WRITES to a sub-field (`s.f[…] = …`) mirror the READ-side field-bounded
//! normalization to a FLAT part-select on the struct net: a CONSTANT in-direction
//! range `[a:b]` and a CONSTANT bit-select `[i]` fold (the only forms iverilog
//! 13.0 supports for a struct-member write); an OOB bit-select drops (no-op). An
//! indexed `[i±:w]`, a runtime/non-constant index, or a reversed range stays loud
//! (iverilog refuses the indexed/runtime forms; the reversed range matches the
//! loud READ side).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita(src: &str) -> std::process::Output {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_sfs_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    out
}

fn run(src: &str) -> String {
    let out = vita(src);
    assert!(
        out.status.success(),
        "vita failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| {
        !l.starts_with("simulation ended") && !l.contains("VITA-W1017") && !l.trim().is_empty()
    }) {
        s.push_str(l.trim());
        s.push('\n');
    }
    s
}

/// Assert vita loudly refuses (nonzero exit) — a field-bounded write isn't v1.
fn run_loud(src: &str) {
    let out = vita(src);
    assert!(
        !out.status.success(),
        "expected a loud refusal, but vita succeeded:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

// ── ASCENDING member `logic [0:7] a` (oracle: equivalent ascending net) ──────

#[test]
fn asc_read_all_forms_off0() {
    // a = 8'hA5; idx0 = MSB. a[0+:4]=hi nibble=a, a[4+:4]=lo=5, a[2]=1,
    // a[1:3]=010=2, a[5-:2]=a[4:5]=01=1. (matches `logic [0:7]` net under iverilog)
    let out = run("module top;\n\
        typedef struct packed { logic [0:7] a; logic [7:0] b; } st_t;\n\
        st_t s;\n\
        initial begin s.a = 8'hA5;\n\
          $display(\"%h %h %b %h %h\", s.a[0+:4], s.a[4+:4], s.a[2], s.a[1:3], s.a[5-:2]);\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "a 5 1 2 1\n");
}

#[test]
fn asc_read_all_forms_nonzero_offset() {
    // `a` now occupies flat bits [11:4] (off=4). The field VALUE is unchanged,
    // so the sub-select results must be identical to off=0.
    let out = run("module top;\n\
        typedef struct packed { logic [0:7] a; logic [3:0] tl; } st_t;\n\
        st_t s;\n\
        initial begin s.a = 8'hA5; s.tl = 4'h0;\n\
          $display(\"%h %h %b %h %h\", s.a[0+:4], s.a[4+:4], s.a[2], s.a[1:3], s.a[5-:2]);\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "a 5 1 2 1\n");
}

#[test]
fn asc_read_runtime_index() {
    // Runtime bit-select / indexed offset on an ascending field.
    let out = run("module top;\n\
        typedef struct packed { logic [0:7] a; logic [7:0] b; } st_t;\n\
        st_t s; integer k;\n\
        initial begin s.a = 8'hA5; k = 2;\n\
          $display(\"%b %h\", s.a[k], s.a[k+:2]);\n\
        end\n\
      endmodule\n");
    // ascending net oracle: a[2]=1, a[2+:2]=a[2:3]=10=2.
    assert_eq!(out, "1 2\n");
}

// ── DESCENDING member `logic [7:0] a` (oracle: iverilog directly) ────────────

#[test]
fn desc_read_all_forms() {
    let out = run("module top;\n\
        typedef struct packed { logic [7:0] a; logic [7:0] b; } st_t;\n\
        st_t s;\n\
        initial begin s.a = 8'hA5;\n\
          $display(\"%h %h %b %h %h\", s.a[0+:4], s.a[4+:4], s.a[2], s.a[3:1], s.a[5-:2]);\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "5 a 1 2 2\n");
}

#[test]
fn desc_read_runtime_index() {
    let out = run("module top;\n\
        typedef struct packed { logic [7:0] a; logic [7:0] b; } st_t;\n\
        st_t s; integer k;\n\
        initial begin s.a = 8'hA5; k = 2;\n\
          $display(\"%b %h\", s.a[k], s.a[k+:2]);\n\
        end\n\
      endmodule\n");
    // descending net oracle: a[2]=1, a[2+:2]=a[3:2]=01=1.
    assert_eq!(out, "1 1\n");
}

// ── FIELD-bounded: an out-of-field read is X, never a neighbouring member ────

#[test]
fn oob_read_is_x_not_leak_low_field() {
    // `a` is the LOW member (off=0) with `pad`=FF directly above it. A partial-OOB
    // select must read X for the out-of-field bits, NOT pad's bits (no leak).
    let out = run("module top;\n\
        typedef struct packed { logic [7:0] pad; logic [7:0] a; } st_t;\n\
        st_t s;\n\
        initial begin s.a = 8'hA5; s.pad = 8'hFF;\n\
          $display(\"%b %b\", s.a[6+:4], s.a[9]);\n\
        end\n\
      endmodule\n");
    // descending net oracle: a[6+:4]=bits 6,7,8(oob),9(oob)=xx10 ; a[9]=x.
    assert_eq!(out, "xx10 x\n");
}

#[test]
fn oob_read_is_x_not_leak_ascending() {
    let out = run("module top;\n\
        typedef struct packed { logic [7:0] pad; logic [0:7] a; } st_t;\n\
        st_t s;\n\
        initial begin s.a = 8'hA5; s.pad = 8'hFF;\n\
          $display(\"%b %b\", s.a[6+:4], s.a[9]);\n\
        end\n\
      endmodule\n");
    // ascending net oracle.
    assert_eq!(out, "01xx x\n");
}

#[test]
fn oob_regular_range_x_extends_correct_end() {
    // An out-of-field regular range must X-extend on the IEEE-correct end: for an
    // ascending field `a[0:9]` the OOB high indices (8,9 = LSB end) read X →
    // `10100101xx`; for a descending field `a[9:0]` the OOB high bits (MSB end) →
    // `xx10100101`. (Normalizing `[a:b]` to the validated indexed path; a naive
    // offset clamp put the X bits on the wrong end — a silent-wrong.)
    let asc = run("module top;\n\
        typedef struct packed { logic [7:0] pad; logic [0:7] a; } st_t;\n\
        st_t s;\n\
        initial begin s.pad = 8'hFF; s.a = 8'hA5; $display(\"%b\", s.a[0:9]); end\n\
      endmodule\n");
    assert_eq!(asc, "10100101xx\n");
    let desc = run("module top;\n\
        typedef struct packed { logic [7:0] pad; logic [7:0] a; } st_t;\n\
        st_t s;\n\
        initial begin s.pad = 8'hFF; s.a = 8'hA5; $display(\"%b\", s.a[9:0]); end\n\
      endmodule\n");
    assert_eq!(desc, "xx10100101\n");
}

// ── A reversed regular range (against the member's direction) is loud ────────

#[test]
fn asc_reversed_range_is_loud() {
    // `logic [0:7]` is ascending; `s.a[3:0]` runs the wrong way → loud (it was a
    // silent-wrong: vita read it as a flat descending slice).
    run_loud(
        "module top;\n\
        typedef struct packed { logic [0:7] a; logic [7:0] b; } st_t;\n\
        st_t s;\n\
        initial begin s.a = 8'hA5; $display(\"%h\", s.a[3:0]); end\n\
      endmodule\n",
    );
}

#[test]
fn desc_reversed_range_is_loud() {
    run_loud(
        "module top;\n\
        typedef struct packed { logic [7:0] a; logic [7:0] b; } st_t;\n\
        st_t s;\n\
        initial begin s.a = 8'hA5; $display(\"%h\", s.a[0:3]); end\n\
      endmodule\n",
    );
}

// ── Whole-field access unchanged (regression guard) ──────────────────────────

#[test]
fn whole_field_read_unchanged() {
    let out = run("module top;\n\
        typedef struct packed { logic [0:7] a; logic [7:0] b; } st_t;\n\
        st_t s;\n\
        initial begin s.a = 8'hA5; s.b = 8'h3C; $display(\"%h %h\", s.a, s.b); end\n\
      endmodule\n");
    assert_eq!(out, "a5 3c\n");
}

// ── Indexed sub-field WRITE stays loud (iverilog refuses `[i±:w]` writes) ─────

#[test]
fn asc_indexed_subfield_write_is_loud() {
    run_loud(
        "module top;\n\
        typedef struct packed { logic [0:7] a; logic [7:0] b; } st_t;\n\
        st_t s;\n\
        initial begin s.a = 8'hA5; s.a[0+:4] = 4'hF; $display(\"%h\", s.a); end\n\
      endmodule\n",
    );
}

#[test]
fn desc_indexed_subfield_write_is_loud() {
    run_loud(
        "module top;\n\
        typedef struct packed { logic [7:0] a; logic [7:0] b; } st_t;\n\
        st_t s;\n\
        initial begin s.a = 8'hA5; s.a[0+:4] = 4'hF; $display(\"%h\", s.a); end\n\
      endmodule\n",
    );
}

#[test]
fn reversed_range_subfield_write_is_loud() {
    // A range running AGAINST the member's declared direction stays loud (matches
    // the loud READ side); iverilog accepts it with quirky semantics — no oracle.
    run_loud(
        "module top;\n\
        typedef struct packed { logic [0:7] a; logic [7:0] b; } st_t;\n\
        st_t s;\n\
        initial begin s.a[3:2] = 2'b11; $display(\"%b\", s.a); end\n\
      endmodule\n",
    );
}

// ── Field-bounded sub-field WRITE (constant range / bit-select), iverilog-pinned

#[test]
fn asc_subfield_range_write() {
    // `logic [0:7] f`: `f[2:3]=11` sets MSB-first bits 2,3 → 00110000 (iverilog).
    let out = run("module top;\n\
        typedef struct packed { logic [0:7] f; logic [7:0] g; } p;\n\
        p s;\n\
        initial begin s = 0; s.f[2:3] = 2'b11; $display(\"%b %b\", s.f, s.g); end\n\
      endmodule\n");
    assert_eq!(out, "00110000 00000000\n");
}

#[test]
fn desc_subfield_range_write() {
    // `logic [7:0] g`: `g[3:2]=11` sets bits 3,2 → 00001100 (iverilog).
    let out = run("module top;\n\
        typedef struct packed { logic [0:7] f; logic [7:0] g; } p;\n\
        p s;\n\
        initial begin s = 0; s.g[3:2] = 2'b11; $display(\"%b %b\", s.f, s.g); end\n\
      endmodule\n");
    assert_eq!(out, "00000000 00001100\n");
}

#[test]
fn asc_subfield_bit_write() {
    let out = run("module top;\n\
        typedef struct packed { logic [0:7] f; logic [7:0] g; } p;\n\
        p s;\n\
        initial begin s = 0; s.f[2] = 1'b1; $display(\"%b %b\", s.f, s.g); end\n\
      endmodule\n");
    assert_eq!(out, "00100000 00000000\n");
}

#[test]
fn desc_subfield_bit_write() {
    let out = run("module top;\n\
        typedef struct packed { logic [0:7] f; logic [7:0] g; } p;\n\
        p s;\n\
        initial begin s = 0; s.g[2] = 1'b1; $display(\"%b %b\", s.f, s.g); end\n\
      endmodule\n");
    assert_eq!(out, "00000000 00000100\n");
}

#[test]
fn subfield_write_no_neighbour_leak() {
    // Preload the whole struct, write into ONE field; the other field and the rest
    // of the written field keep their bits (iverilog: 16'hABCD with f[2:3]:=00).
    let out = run("module top;\n\
        typedef struct packed { logic [0:7] f; logic [7:0] g; } p;\n\
        p s;\n\
        initial begin s = 16'hABCD; s.f[2:3] = 2'b00;\n\
          $display(\"%b %b %b\", s.f, s.g, s); end\n\
      endmodule\n");
    assert_eq!(out, "10001011 11001101 1000101111001101\n");
}

#[test]
fn oob_bit_write_drops_no_op() {
    // `f` is 8 bits (indices 0..7); `s.f[8]=1` is out of bounds and drops (no-op)
    // — never leaking into the neighbour `g`. (iverilog 13.0 has no defined
    // behaviour here — it aborts at compile — so this is vita's correct-or-loud
    // safe choice, not an iverilog match.)
    let out = run("module top;\n\
        typedef struct packed { logic [0:7] f; logic [7:0] g; } p;\n\
        p s;\n\
        initial begin s = 0; s.f[8] = 1'b1; $display(\"%b %b\", s.f, s.g); end\n\
      endmodule\n");
    assert_eq!(out, "00000000 00000000\n");
}

#[test]
fn asc_subfield_range_write_spanning() {
    // A wider in-direction range `f[1:6]=6'b111111` over the ascending member.
    let out = run("module top;\n\
        typedef struct packed { logic [0:7] f; logic [7:0] g; } p;\n\
        p s;\n\
        initial begin s = 0; s.f[1:6] = 6'b111111; $display(\"%b %b\", s.f, s.g); end\n\
      endmodule\n");
    assert_eq!(out, "01111110 00000000\n");
}

// ── Whole-field WRITE still works (regression guard) ─────────────────────────

#[test]
fn whole_field_write_unchanged() {
    let out = run("module top;\n\
        typedef struct packed { logic [0:7] a; logic [7:0] b; } st_t;\n\
        st_t s;\n\
        initial begin s.a = 8'h00; s.a = 8'hA5; $display(\"%h\", s.a); end\n\
      endmodule\n");
    assert_eq!(out, "a5\n");
}

// ── NON-zero-LSB member — declared base must be removed from the sub-select ──
//
// A member with a non-zero declared LSB (`logic [15:8] a`) or non-zero ascending
// base (`logic [4:11] c`) previously read/wrote raw / out-of-range bits (silent
// X or a wrong value), because the sub-select source index was used verbatim
// against the field part-select `pv` (normalized to `[w-1:0]`). The declared base
// index `min(msb,lsb)` is now subtracted. Oracle = iverilog directly for `:`/`+:`
// / bit forms; iverilog 13.0 asserts on a struct-field `-:`, so that form is
// pinned by vita-internal equivalence to its `:` twin.

#[test]
fn nonzero_lsb_desc_read_all_forms() {
    // field `a` = `logic [15:8]` (declared base 8), value 5A.
    let out = run("module top;\n\
        typedef struct packed { logic [15:8] a; logic [7:0] b; } s_t;\n\
        s_t s;\n\
        initial begin s.a = 8'h5A; s.b = 8'h3C;\n\
          $display(\"%h %h %b %h\", s.a[11:8], s.a[15:12], s.a[9], s.a[8+:4]);\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "a 5 1 a\n");
}

#[test]
fn nonzero_lsb_desc_read_pos_and_base_differ() {
    // field `q` = `logic [11:4]` (declared base 4) sits at flat pos [7:0] — the
    // flat offset (0) and the declared base (4) differ, so a naive path reads the
    // wrong nibble (`e` instead of `7`).
    let out = run("module top;\n\
        typedef struct packed { logic [3:0] p; logic [11:4] q; } t_t;\n\
        t_t t;\n\
        initial begin t.p = 4'h9; t.q = 8'hE7;\n\
          $display(\"%h %h %h\", t.q[7:4], t.q[11:8], t.q[4+:4]);\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "7 e 7\n");
}

#[test]
fn nonzero_base_asc_read_all_forms() {
    // ascending field `c` = `logic [4:11]` (declared base 4), value E7. Oracle =
    // equivalent ascending net (iverilog handles the net; a struct field matches).
    let out = run("module top;\n\
        typedef struct packed { logic [4:11] c; logic [3:0] d; } u_t;\n\
        u_t u;\n\
        initial begin u.c = 8'hE7; u.d = 4'h5;\n\
          $display(\"%h %h %b\", u.c[4:11], u.c[4:7], u.c[4]);\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "e7 e 1\n");
}

#[test]
fn nonzero_lsb_minus_colon_equiv_to_colon() {
    // iverilog 13.0 asserts on a struct-field `-:`; pin `s.a[hi-:w]` to its `:`
    // twin `s.a[hi:hi-w+1]` (identical bits) — a vita-internal equivalence.
    let out = run("module top;\n\
        typedef struct packed { logic [15:8] a; logic [7:0] b; } s_t;\n\
        s_t s;\n\
        initial begin s.a = 8'h5A;\n\
          $display(\"%h %h %h %h\", s.a[15-:4], s.a[15:12], s.a[11-:4], s.a[11:8]);\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "5 5 a a\n");
}

#[test]
fn nonzero_lsb_out_of_field_reads_x() {
    // A select touching outside the member's declared range reads X (both the
    // pre-existing OOB-HIGH and the newly-correct OOB-LOW — previously `s.a[7:4]`
    // silently read `5`).
    let out = run("module top;\n\
        typedef struct packed { logic [15:8] a; logic [7:0] b; } s_t;\n\
        s_t s;\n\
        initial begin s.a = 8'h5A; s.b = 8'h3C;\n\
          $display(\"%h %h %h %b\", s.a[7:4], s.a[19:16], s.a[10:7], s.a[7]);\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "x x x x\n");
}

#[test]
fn nonzero_lsb_desc_write() {
    // WRITE to a non-zero-LSB member sub-field (was loud E2002, now supported):
    // s.a[11:8]=F, s.a[15]=1 → a = 1000_1111 = 8F.
    let out = run("module top;\n\
        typedef struct packed { logic [15:8] a; logic [7:0] b; } s_t;\n\
        s_t s;\n\
        initial begin s.a = 8'h00; s.b = 8'h00;\n\
          s.a[11:8] = 4'hF; s.a[15] = 1'b1;\n\
          $display(\"%h\", s);\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "8f00\n");
}

#[test]
fn nonzero_base_write_desc_and_asc() {
    // Non-zero declared base WRITE on a descending (`q` base 4) and an ascending
    // (`c` base 4) member. q[7:4]=A,q[11]=1 → q=8A; c[4]=1(MSB),c[11]=1(LSB) → c=81.
    let out = run("module top;\n\
        typedef struct packed { logic [11:4] q; logic [4:11] c; } x_t;\n\
        x_t x;\n\
        initial begin x.q = 8'h00; x.c = 8'h00;\n\
          x.q[7:4] = 4'hA; x.q[11] = 1'b1;\n\
          x.c[4] = 1'b1;   x.c[11] = 1'b1;\n\
          $display(\"%h %h\", x.q, x.c);\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "8a 81\n");
}

#[test]
fn nonzero_lsb_write_out_of_field_range_loud() {
    // A write RANGE straying outside the member's declared range is loud (matches
    // the pre-fix in-bounds guard, now field-relative) — never a neighbour leak.
    run_loud(
        "module top;\n\
        typedef struct packed { logic [15:8] a; logic [7:0] b; } s_t;\n\
        s_t s;\n\
        initial begin s.a[7:4] = 4'hF; end\n\
      endmodule\n",
    );
}

// ── NEGATIVE-LSB member — sub-select is loud (whole read/write stays correct) ─
//
// A member declared with a negative LSB (`logic [7:-4]`, declared base −4) would
// need SIGNED field-relative offsets across every sub-select form (read, write,
// `+:`/`-:`, runtime); v1 does not do that, so a sub-select is loud-rejected
// rather than silently mis-mapped (it previously read the wrong bits because the
// declared base was clamped to 0). The whole-field read/write is unaffected.

#[test]
fn neg_lsb_read_subselect_is_loud() {
    run_loud(
        "module top;\n\
        typedef struct packed { logic [7:-4] a; logic [7:0] b; } s_t;\n\
        s_t s;\n\
        initial begin s.a = 12'h5A3; $display(\"%b\", s.a[7:4]); end\n\
      endmodule\n",
    );
}

#[test]
fn neg_lsb_bit_select_is_loud() {
    run_loud(
        "module top;\n\
        typedef struct packed { logic [7:-4] a; logic [7:0] b; } s_t;\n\
        s_t s;\n\
        initial begin s.a = 12'h5A3; $display(\"%b\", s.a[0]); end\n\
      endmodule\n",
    );
}

#[test]
fn neg_lsb_write_subselect_is_loud() {
    run_loud(
        "module top;\n\
        typedef struct packed { logic [7:-4] a; logic [7:0] b; } s_t;\n\
        s_t s;\n\
        initial begin s.a[7:4] = 4'hF; end\n\
      endmodule\n",
    );
}

#[test]
fn neg_lsb_whole_read_write_unaffected() {
    // Whole-field read and write of a negative-LSB member use the flat offset/width
    // (not the declared base), so they stay correct (oracle = iverilog).
    let out = run("module top;\n\
        typedef struct packed { logic [7:-4] a; logic [7:0] b; } s_t;\n\
        s_t s;\n\
        initial begin s.a = 12'h5A3; s.b = s.a; $display(\"%h %h\", s.a, s.b); end\n\
      endmodule\n");
    assert_eq!(out, "5a3 a3\n");
}
