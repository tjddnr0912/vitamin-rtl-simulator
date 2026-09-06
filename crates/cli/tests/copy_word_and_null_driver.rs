//! A procedural read of a copy of a CONSTANT ARRAY WORD, and of a copy beside an
//! all-`z` constant driver, after the reader's own write of the source — ROADMAP
//! §2 🆕 I ⓒ / ⓑ (row 33's residue).
//!
//! `wire [7:0] c; assign c = m[1];` then `m[1] = 8'hA5; $display(c)` printed `xx`
//! in vita where iverilog 13.0 and verilator 5.050 both print `a5` — `copy_nets`
//! refused an array base as a source, so the row-33 read-through never applied.
//! `assign c = v; assign c = 8'hzz;` was refused as a multi-driver group, although
//! `z` is the identity of every resolution and the net is a copy of `v`. Both are
//! copies now (`alias::copied_source` admits a constant, in-range word — the alias
//! carries the word's index — and `alias::null_driver` drops the `z` driver from
//! both counts).
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.050 (35 cells);
//! the lines are the oracles' output, copied.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_backend(src: &str, backend: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cwnd_{}_{n}", std::process::id()));
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

/// `top` with `decls`; one initial block runs `writes`, displays `disp` at once
/// and again one tick later.
fn top(decls: &str, writes: &str, disp: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule top;\n{decls}\n  initial begin\n    {writes}\n    \
         $display(\"D=%h\", {disp});\n    #1 $display(\"D=%h\", {disp});\n  end\n  \
         initial #5 $finish;\nendmodule\n"
    )
}

/// The `D=` lines must be exactly `want` on ALL THREE backends — the table is one
/// sidecar, and a backend that stopped consulting it would disagree here.
fn prints_all(src: &str, want: &[&str]) {
    for b in ["native", "interp", "vm"] {
        let (out, code) = run_backend(src, b);
        assert_eq!(code, Some(0), "[{b}] exit\n{out}");
        let got: Vec<&str> = out.lines().filter(|l| l.starts_with("D=")).collect();
        assert_eq!(got, want, "[{b}]\n{out}");
    }
}

#[test]
fn a_constant_word_copy_reads_the_word_after_the_writers_own_write() {
    // i01 · both oracles `a5 a5` (PRE `xx a5`)
    prints_all(
        &top(
            "  reg [7:0] m [0:3]; wire [7:0] c; assign c = m[1];",
            "m[1] = 8'hA5;",
            "c",
        ),
        &["D=a5", "D=a5"],
    );
    // i10 / i11 · a non-zero-base and a descending declaration — the word index is
    // the declaration's, both oracles `a5 a5`
    prints_all(
        &top(
            "  reg [7:0] m [2:5]; wire [7:0] c; assign c = m[3];",
            "m[3] = 8'hA5;",
            "c",
        ),
        &["D=a5", "D=a5"],
    );
    prints_all(
        &top(
            "  reg [7:0] m [3:0]; wire [7:0] c; assign c = m[1];",
            "m[1] = 8'hA5;",
            "c",
        ),
        &["D=a5", "D=a5"],
    );
    // i08 · a 2-state source array, both oracles `a5 a5` (PRE `00 a5`)
    prints_all(
        &top(
            "  bit [7:0] m [0:3]; wire [7:0] c; assign c = m[1];",
            "m[1] = 8'hA5;",
            "c",
        ),
        &["D=a5", "D=a5"],
    );
    // i29 · the process writes other words too — the LAST write of the word
    prints_all(
        &top(
            "  reg [7:0] m [0:3]; wire [7:0] c; assign c = m[1];",
            "m[1] = 8'h11; m[0] = 8'hA5; m[1] = 8'hA5;",
            "c",
        ),
        &["D=a5", "D=a5"],
    );
}

#[test]
fn a_chain_through_a_word_copy_carries_the_word() {
    // i05 · `d = c`, `c = m[1]` — both oracles `a5 a5` (PRE `xx a5`)
    prints_all(
        &top(
            "  reg [7:0] m [0:3]; wire [7:0] c, d; assign c = m[1]; assign d = c;",
            "m[1] = 8'hA5;",
            "d",
        ),
        &["D=a5", "D=a5"],
    );
    // i30 · a `z` driver on each link of the chain
    prints_all(
        &top(
            "  reg [7:0] m [0:3]; wire [7:0] c, d; assign c = m[1]; assign c = 'z; \
             assign d = c; assign d = 8'hzz;",
            "m[1] = 8'hA5;",
            "d",
        ),
        &["D=a5", "D=a5"],
    );
    // i31 · a signed word into a signed copy, read at 32 bits — both oracles
    prints_all(
        &top(
            "  reg signed [7:0] m [0:3]; wire signed [7:0] c; assign c = m[1]; integer r;",
            "m[1] = 8'hA5; r = c;",
            "r",
        ),
        &["D=ffffffa5", "D=ffffffa5"],
    );
}

#[test]
fn an_all_z_constant_driver_leaves_the_net_a_copy() {
    // i13 / i14 / i20 · both oracles `a5 a5` (PRE `xx a5`), whichever is written first
    for decl in [
        "  reg [7:0] v; wire [7:0] c; assign c = v; assign c = 8'hzz;",
        "  reg [7:0] v; wire [7:0] c; assign c = v; assign c = 'z;",
        "  reg [7:0] v; wire [7:0] c; assign c = 8'hzz; assign c = v;",
        // i18 · iverilog only (verilator: unsupported tristate) — `z` is the identity
        // of `wand` too (§6.6.1)
        "  reg [7:0] v; wand [7:0] c; assign c = v; assign c = 8'hzz;",
    ] {
        prints_all(&top(decl, "v = 8'hA5;", "c"), &["D=a5", "D=a5"]);
    }
    // i22 · the source was `z` itself before the write — both oracles `a5 a5`
    // (PRE `zz a5`: the read took the settle's resolution of two `z`s)
    prints_all(
        &top(
            "  reg [7:0] v; wire [7:0] c; assign c = v; assign c = 8'hzz;",
            "v = 8'hzz; #1 v = 8'hA5;",
            "c",
        ),
        &["D=a5", "D=a5"],
    );
}

#[test]
fn what_is_still_computed_stays_computed() {
    // i15 / i16 · a partial `8'hzx` conflicts, a narrow `4'hz` zero-extends and its
    // high half drives 0 — iverilog `ax ax` / `X5 X5`; vita keeps the settle's value
    // at t0 (`xx`) and agrees one tick later (PRE == POST, the t0 cell is a
    // recorded residue).
    prints_all(
        &top(
            "  reg [7:0] v; wire [7:0] c; assign c = v; assign c = 8'hzx;",
            "v = 8'hA5;",
            "c",
        ),
        &["D=xx", "D=ax"],
    );
    prints_all(
        &top(
            "  reg [7:0] v; wire [7:0] c; assign c = v; assign c = 4'hz;",
            "v = 8'hA5;",
            "c",
        ),
        &["D=xx", "D=X5"],
    );
    // i17 · two real drivers beside the `z` are a resolution — PRE == POST `xx a5`
    prints_all(
        &top(
            "  reg [7:0] v, u; wire [7:0] c; assign c = v; assign c = u; assign c = 8'hzz;",
            "v = 8'hA5; u = 8'hA5;",
            "c",
        ),
        &["D=xx", "D=a5"],
    );
    // i12 · a slice of a word is not read through — iverilog `x 5` too
    prints_all(
        &top(
            "  reg [7:0] m [0:3]; wire [3:0] c; assign c = m[1][3:0];",
            "m[1] = 8'hA5;",
            "c",
        ),
        &["D=x", "D=5"],
    );
    // i27 · the writer is ANOTHER process in the same delta — a §5.4.1 race, kept on
    // the settle's value (row 33's rule; both oracles happen to run the writer first)
    prints_all(
        &top(
            "  reg [7:0] m [0:3]; wire [7:0] c; assign c = m[1]; initial #0 m[1] = 8'hA5;",
            "#0;",
            "c",
        ),
        &["D=xx", "D=a5"],
    );
}
