//! A procedural read of a whole-net copy, after the reader's own write of the
//! source, sees the source — ROADMAP §2 row 33.
//!
//! `wire [7:0] c; assign c = v;` then `v = 8'hA5; cap = c;` latched `cap = 00` in
//! vita where iverilog and verilator both latch `a5`. The copy is driven by the
//! continuous-assign settle, which runs BETWEEN process batches, so a read one
//! statement after the writer's own write saw the value from before it. iverilog
//! collapses such a copy into its source (one VCD identifier for both); vita now
//! reads the source at exactly the place that is defined — a read that follows the
//! writer's OWN blocking write in program order (`levelize::proc_read_alias`,
//! consumed by all three backends through `WidthTable::read_alias`).
//!
//! ⚠️ A read in ANOTHER process in the same delta is a §5.4.1 race and keeps the
//! settle's value; those cells are pinned below as they were, on both oracles.
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.050 unless a test
//! says otherwise; 91 cells in the slice's census.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_backend(src: &str, backend: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cnrt_{}_{n}", std::process::id()));
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

/// `top` with `decls`, one initial block that at #1 runs `body` and finishes.
fn top(decls: &str, body: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule top;\n{decls}\n  initial begin\n    #1 begin {body} end\n    \
         #1 $finish;\n  end\nendmodule\n"
    )
}

/// The line must appear on ALL THREE backends — the table is one sidecar, and a
/// backend that stopped consulting it would disagree here.
fn prints_all(src: &str, want: &str) {
    for b in ["native", "interp", "vm"] {
        let (out, code) = run_backend(src, b);
        assert_eq!(code, Some(0), "[{b}] exit\n{out}");
        assert!(
            out.lines().any(|l| l == want),
            "[{b}] expected `{want}`\n{out}"
        );
    }
}

fn prints(decls: &str, body: &str, want: &str) {
    prints_all(&top(decls, body), want);
}

/// The headline: widths 1 / 8 / 33 / 128, every source and destination kind.
#[test]
fn a_read_after_the_writers_own_write_sees_the_source() {
    for sk in ["reg", "logic", "bit"] {
        for dk in ["wire", "logic", "tri", "wand", "wor"] {
            prints(
                &format!("  {sk} v = 0; reg cap;\n  {dk} c; assign c = v;"),
                "v = 1'b1; cap = c; $display(\"%h\", cap);",
                "1",
            );
            prints(
                &format!("  {sk} [7:0] v = 0; reg [7:0] cap;\n  {dk} [7:0] c; assign c = v;"),
                "v = 8'hA5; cap = c; $display(\"%h\", cap);",
                "a5",
            );
            prints(
                &format!("  {sk} [32:0] v = 0; reg [32:0] cap;\n  {dk} [32:0] c; assign c = v;"),
                "v = {33{1'b1}}; cap = c; $display(\"%h\", cap);",
                "1ffffffff",
            );
            // 128 bits: iverilog's `%h` of an all-ones 128-bit value; verilator
            // prints the same digits with a line break (harness format).
            prints(
                &format!("  {sk} [127:0] v = 0; reg [127:0] cap;\n  {dk} [127:0] c; assign c = v;"),
                "v = {128{1'b1}}; cap = c; $display(\"%h\", cap);",
                "ffffffffffffffffffffffffffffffff",
            );
        }
    }
}

/// The spellings and shapes the row listed: the declaration-initializer form,
/// parentheses, two- and three-deep chains, signed either side, a partial write of
/// the source, a read used as a condition, as an index, in `$display`, twice.
#[test]
fn every_copy_spelling_and_read_shape() {
    let d = "  reg [7:0] v = 0, cap;\n  wire [7:0] c; assign c = v;";
    prints(
        "  reg [7:0] v = 0, cap;\n  wire [7:0] c = v;",
        "v = 8'hA5; cap = c; $display(\"%h\", cap);",
        "a5",
    );
    prints(
        "  reg [7:0] v = 0, cap;\n  wire [7:0] c; assign c = (v);",
        "v = 8'hA5; cap = c; $display(\"%h\", cap);",
        "a5",
    );
    prints(
        "  reg [7:0] v = 0, cap;\n  wire [7:0] c; assign c = v;\n  wire [7:0] d; assign d = c;\n  \
         wire [7:0] e; assign e = d;",
        "v = 8'hA5; cap = e; $display(\"%h\", cap);",
        "a5",
    );
    // Signed on BOTH sides reads through; a sign MISMATCH does not (see the parity
    // pin below and ROADMAP §2 🆕 I).
    prints(
        "  reg signed [7:0] v = 0; reg [7:0] cap;\n  wire signed [7:0] c; assign c = v;",
        "v = -8'sd91; cap = c; $display(\"%h\", cap);",
        "a5",
    );
    prints(d, "v[3:0] = 4'h5; cap = c; $display(\"%h\", cap);", "05");
    prints(
        d,
        "v = 8'hA5; if (c == 8'hA5) cap = 1; else cap = 0; $display(\"%h\", cap);",
        "01",
    );
    prints(
        "  reg [1:0] v = 0; reg [7:0] m [0:3]; reg [7:0] cap;\n  wire [1:0] c; assign c = v;",
        "m[0]=1; m[1]=2; m[2]=3; m[3]=4; v = 2; cap = m[c]; $display(\"%h\", cap);",
        "03",
    );
    prints(
        "  reg [7:0] v = 0;\n  wire [7:0] c; assign c = v;",
        "v = 8'hA5; $display(\"%h %h\", c, v);",
        "a5 a5",
    );
    prints(
        "  reg [7:0] v = 0, cap, cap2;\n  wire [7:0] c; assign c = v;",
        "v = 8'hA5; cap = c; v = 8'h3C; cap2 = c; $display(\"%h %h\", cap, cap2);",
        "a5 3c",
    );
    // A read BEFORE the write is the old value — and the later read the new one.
    prints(
        d,
        "cap = c; v = 8'hA5; $display(\"%h %h\", cap, c);",
        "00 a5",
    );
    // A write through a task and a read through a function, both in this process.
    prints(
        "  reg [7:0] v = 0, cap;\n  wire [7:0] c; assign c = v;\n  task setv; begin v = 8'hA5; \
         end endtask",
        "setv; cap = c; $display(\"%h\", cap);",
        "a5",
    );
    prints(
        "  reg [7:0] v = 0, cap;\n  wire [7:0] c; assign c = v;\n  function [7:0] getc; input \
         dummy; getc = c; endfunction",
        "v = 8'hA5; cap = getc(0); $display(\"%h\", cap);",
        "a5",
    );
    // x/z bits move like any other bit (iverilog; verilator is 2-state here).
    prints(
        d,
        "v = 8'b1010_zx01; cap = c; $display(\"%b\", cap);",
        "1010zx01",
    );
}

/// Across a module boundary: the parent writes the reg bound to the child's input,
/// then reads the child's output (a copy of that input) and the child's internal
/// copy by hierarchical name — both `a5` on both oracles, both `00` before.
#[test]
fn a_port_copy_reads_through_across_the_module_boundary() {
    prints_all(
        "`timescale 1ns/1ns\nmodule ch(input [7:0] p, output [7:0] q); assign q = p; endmodule\n\
         module top;\n  reg [7:0] v = 0, cap;\n  wire [7:0] q;\n  ch u(.p(v), .q(q));\n  \
         initial begin #1 v = 8'hA5; cap = q; $display(\"%h\", cap); #1 $finish; end\nendmodule\n",
        "a5",
    );
    prints_all(
        "`timescale 1ns/1ns\nmodule ch(input [7:0] p); wire [7:0] w; assign w = p; endmodule\n\
         module top;\n  reg [7:0] v = 0, cap;\n  ch u(.p(v));\n  \
         initial begin #1 v = 8'hA5; cap = u.w; $display(\"%h\", cap); #1 $finish; end\nendmodule\n",
        "a5",
    );
}

/// ⚠️ Review finding: a copy whose declared SIGN differs from its source's does not
/// read through — the substituted net would lend its sign to the extension, and the
/// interpreter/VM (slot sign) and the native compiled path (node sign) disagreed
/// (255 vs 4294967295; both oracles 4294967295). Excluded in `copy_alias`, so the
/// three backends agree again; the value is the pre-slice one (stale), pinned here
/// as PARITY only — ROADMAP §2 🆕 I records the cell.
#[test]
fn a_sign_mismatched_copy_keeps_backend_parity() {
    let src = top(
        "  reg [7:0] v = 0; reg [31:0] r;\n  wire signed [7:0] c; assign c = v;",
        "v = 8'hFF; r = c; $display(\"r=%0d sra=%h\", r, c >>> 1);",
    );
    let mut seen = std::collections::BTreeSet::new();
    for b in ["native", "interp", "vm"] {
        let (out, code) = run_backend(&src, b);
        assert_eq!(code, Some(0), "[{b}]\n{out}");
        let line = out
            .lines()
            .find(|l| l.starts_with("r="))
            .map(str::to_string);
        assert!(line.is_some(), "[{b}]\n{out}");
        seen.insert(line.unwrap());
    }
    assert_eq!(seen.len(), 1, "the three backends disagree: {seen:?}");
    // The same-sign twins DO read through, on every backend.
    prints(
        "  reg [7:0] v = 0; reg [31:0] r;\n  wire [7:0] c; assign c = v;",
        "v = 8'hFF; r = c; $display(\"r=%0d\", r);",
        "r=255",
    );
    prints(
        "  reg signed [7:0] v = 0; reg [31:0] r;\n  wire signed [7:0] c; assign c = v;",
        "v = 8'hFF; r = c; $display(\"r=%0d\", r);",
        "r=4294967295",
    );
}

/// ⚠️ What does NOT read through, pinned on both oracles so the scope cannot creep
/// silently: a read in ANOTHER process of the same delta keeps the settle's value
/// when the reader runs first (`00`, both oracles); a level-sensitive reader wakes
/// after the settle and sees `a5` either way; a FORCED copy holds the forced value
/// over a fresh source; `$monitor` prints the settled pair.
#[test]
fn the_race_cells_and_the_forced_copy_keep_their_value() {
    prints_all(
        "`timescale 1ns/1ns\nmodule top;\n  reg [7:0] v = 0;\n  wire [7:0] c; assign c = v;\n  \
         initial begin #1 $display(\"%h\", c); #1 $finish; end\n  initial begin #1 v = 8'hA5; end\n\
         endmodule\n",
        "00",
    );
    prints_all(
        "`timescale 1ns/1ns\nmodule top;\n  reg [7:0] v = 0, cap;\n  wire [7:0] c; assign c = v;\n  \
         always @* cap = c;\n  initial begin #1 v = 8'hA5; #1 $display(\"%h\", cap); #1 $finish; \
         end\nendmodule\n",
        "a5",
    );
    prints(
        "  reg [7:0] v = 0, cap;\n  wire [7:0] c; assign c = v;",
        "force c = 8'h11; v = 8'hA5; cap = c; $display(\"%h\", cap); release c;",
        "11",
    );
    prints_all(
        "`timescale 1ns/1ns\nmodule top;\n  reg [7:0] v = 0;\n  wire [7:0] c; assign c = v;\n  \
         initial $monitor(\"m %h %h\", c, v);\n  initial begin #1 v = 8'hA5; #1 $finish; end\n\
         endmodule\n",
        "m a5 a5",
    );
}
