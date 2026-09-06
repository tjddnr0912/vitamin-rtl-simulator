//! A procedural read of a copy of an array word whose constant index is NOT a
//! bare literal offset — a negative-base array (`m[-2:1]`, `m[-4:-1]`), an
//! ascending one (`m[1:-2]`), a signed narrow literal (`4'sd14`, `8'shFE`,
//! `-16'sd2`), a parameter, and arithmetic (`-2+1`, `2-4`, `-2*1`, `4'd14 + 4'd4 -
//! 18`) — after the reader's own write of the source. ROADMAP §2 🆕 I residue
//! (review A S3-P1 of §4.5.433).
//!
//! `alias::copied_source` folded the word index with the width-tree folder, which
//! saturates: `m[-2]` lowers to the coordinate `(-2) + 2`, `0xFFFF_FFFE + 2`
//! saturated to WIDTH_MAX, the copy was declined and the read stayed `xx` where
//! iverilog 13.0 and verilator 5.050 both print `a5`. `alias::word_const` now folds
//! the index tree the way the engine evaluates it (IEEE §11.6 / §11.8.1: the
//! arithmetic at the widest operand's width, a narrow signed operand sign-extended
//! only in an all-signed context, `{1'b0, i}` / `{{n{i[msb]}}, i}` seals as
//! self-determined leaves).
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.050 (32 cells);
//! the lines are the oracles' output, copied.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_backend(src: &str, backend: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cwif_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg(backend)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// `decl` declares `m`; `c` copies `m[idx]`; the block writes `m[widx]` and
/// displays `c` at once — the census cell's exact shape.
fn cell(decl: &str, idx: &str, widx: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule top; {decl} wire [7:0] c; assign c = m[{idx}];\n  \
         initial begin #1 m[{widx}] = 8'hA5; $display(\"D=%h\", c); #5 $finish; end\nendmodule\n"
    )
}

fn prints_all(src: &str, want: &[&str]) {
    for b in ["native", "interp", "vm"] {
        let (out, code) = run_backend(src, b);
        assert_eq!(code, Some(0), "[{b}] exit\n{out}");
        let got: Vec<&str> = out.lines().filter(|l| l.starts_with("D=")).collect();
        assert_eq!(got, want, "[{b}]\n{out}");
    }
}

const NEG: &str = "logic [7:0] m[-2:1];";

#[test]
fn a_negative_base_word_copy_reads_through() {
    // i01 / i11 / i09 · both oracles `a5` (PRE `xx`)
    prints_all(&cell(NEG, "-2", "-2"), &["D=a5"]);
    prints_all(&cell(NEG, "-1", "-1"), &["D=a5"]);
    // i20 / i21 · every word negative
    prints_all(&cell("logic [7:0] m[-4:-1];", "-4", "-4"), &["D=a5"]);
    prints_all(&cell("logic [7:0] m[-4:-1];", "-1", "-1"), &["D=a5"]);
    // i18 / i19 · an ascending declaration
    prints_all(&cell("logic [7:0] m[1:-2];", "-2", "-2"), &["D=a5"]);
    prints_all(&cell("logic [7:0] m[1:-2];", "1", "1"), &["D=a5"]);
    // i12 · the non-negative word of the same array (PRE == POST)
    prints_all(&cell(NEG, "1", "1"), &["D=a5"]);
}

#[test]
fn an_arithmetic_or_named_constant_index_reads_through() {
    // i13 / i15 / i27 / i28 · both oracles `a5`
    prints_all(&cell(NEG, "-2+1", "-1"), &["D=a5"]);
    prints_all(&cell(NEG, "2-4", "-2"), &["D=a5"]);
    prints_all(&cell(NEG, "-(2)", "-2"), &["D=a5"]);
    prints_all(&cell(NEG, "-2 * 1", "-2"), &["D=a5"]);
    // i14 · a parameter
    prints_all(
        &cell("localparam int P = -2; logic [7:0] m[-2:1];", "P", "P"),
        &["D=a5"],
    );
    // i31 · a context-determined sum: `4'd14 + 4'd4` is 18 at the 32-bit width of
    // the `18` beside it (§11.6), not 2 — both oracles `a5` for `m[0]`
    prints_all(&cell(NEG, "4'd14 + 4'd4 - 18", "0"), &["D=a5"]);
}

#[test]
fn a_signed_narrow_literal_index_is_sign_extended() {
    // i17 / i29 / i30 · `4'sd14` = -2, `8'shFE` = -2, `-16'sd2` — both oracles `a5`
    prints_all(&cell(NEG, "4'sd14", "-2"), &["D=a5"]);
    prints_all(&cell(NEG, "8'shFE", "-2"), &["D=a5"]);
    prints_all(&cell(NEG, "-16'sd2", "-2"), &["D=a5"]);
}

#[test]
fn two_copies_of_two_words_read_their_own_word() {
    // i32 · both oracles `a5 5a`
    let src =
        "`timescale 1ns/1ns\nmodule top; logic [7:0] m[-2:1]; wire [7:0] c; assign c = m[-2]; \
               wire [7:0] d; assign d = m[1];\n  initial begin #1 m[-2] = 8'hA5; m[1] = 8'h5A; \
               $display(\"D=%h %h\", c, d); #5 $finish; end endmodule\n";
    prints_all(src, &["D=a5 5a"]);
}

#[test]
fn an_unwritten_word_and_an_out_of_range_index_stay_as_they_were() {
    // i24 · the copy of a word the block did NOT write — both oracles `xx` / `00`
    // (verilator is 2-state); a read-through must not invent a value
    prints_all(&cell(NEG, "-2", "0"), &["D=xx"]);
    // i25 / i26 · an out-of-range constant index is loud (E4002) and reads `xx`
    for idx in ["-3", "2"] {
        let src = cell(NEG, idx, "-2");
        for b in ["native", "interp", "vm"] {
            let (out, _) = run_backend(&src, b);
            assert!(out.contains("VITA-E4002"), "[{b}] {idx}\n{out}");
            assert!(out.contains("D=xx"), "[{b}] {idx}\n{out}");
        }
    }
}
