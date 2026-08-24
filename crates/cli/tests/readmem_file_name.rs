//! `$readmem*` / `$writemem*` with a file name that is not a string LITERAL.
//!
//! IEEE 1800 §21.4 asks only for a string EXPRESSION, and the canonical SoC testbench keeps
//! the name in a packed reg rather than writing a literal:
//!
//! ```verilog
//! reg [1023:0] firmware_file;
//! if ($value$plusargs("firmware=%s", firmware_file))
//!     $readmemh(firmware_file, mem);
//! ```
//!
//! Both call sites accepted only `Expr::Const` and took a bare `return` on anything else, so
//! such a call loaded nothing and wrote no file — at exit 0, with no diagnostic, leaving the
//! memory at its initial value. A SystemVerilog `string` variable was equally silent. The two
//! sites were the same six lines and now share one helper, because the read side is the one
//! anybody probes and a fix applied there alone would have left `$writemem*` behind.
//!
//! ⚠️ This was found underneath a loud gate. The slice that uncovered it was opening the
//! HIERARCHICAL memory argument (`$readmemh(f, dut.ram.mem)`), which made this reachable on
//! serv's own testbench; that half was reverted for a separate reason recorded in
//! `docs/ROADMAP.md` §3 ④, and this half stands on its own — it needs no hierarchy at all.
//!
//! Every value below is iverilog 13.0's, measured live.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` in a fresh directory holding `fw.hex` = `0A 0B 0C 0D`, so a loaded memory
/// reads 10, 11, 12, 13. Returns (stdout, exit code, stderr, dir) — the dir survives so a
/// test can inspect a file the design wrote.
fn run(src: &str, args: &[&str]) -> (String, Option<i32>, String, std::path::PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_hiermem_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("fw.hex"), "0A\n0B\n0C\n0D\n").unwrap();
    let path = dir.join("t.sv");
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .args(args)
        .current_dir(&dir)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        dir,
    )
}

/// A child module holding an 8-entry memory, for the still-loud hierarchical pin below.
const CHILD: &str = "module ram;\n  reg [7:0] mem [0:7];\n  integer i;\n\
                     initial for (i = 0; i < 8; i = i + 1) mem[i] = 8'd0;\n\
                     endmodule\n";

fn runs(src: &str, args: &[&str], want: &str) -> std::path::PathBuf {
    let (out, code, err, dir) = run(src, args);
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains(want), "want `{want}`; got:\n{out}");
    dir
}

fn loud(src: &str) {
    let (out, code, err, _) = run(src, &[]);
    assert_eq!(code, Some(1), "expected a loud reject, got:\n{out}");
    assert!(err.contains("E3009"), "stderr:\n{err}");
}

// ── the file name is an expression, not just a literal ────────────────────────────

#[test]
fn a_packed_reg_file_name_on_a_local_array() {
    // No hierarchy at all — this was silently loading NOTHING before, at exit 0. Leading
    // NUL bytes in the 1024-bit reg are width padding and must be stripped, exactly as
    // `%0s` renders the same value.
    runs(
        "module t;\n  reg [7:0] mem [0:7]; reg [1023:0] ff; integer i;\n\
         initial begin for (i = 0; i < 8; i = i + 1) mem[i] = 8'd0;\n\
         #1 ff = \"fw.hex\"; $readmemh(ff, mem);\n\
         #1 $display(\"VAL=%0d %0d\", mem[1], mem[3]); #5 $finish; end\n\
         initial #500 $finish;\nendmodule\n",
        &[],
        "VAL=11 13",
    );
}

#[test]
fn a_string_variable_file_name() {
    // The SystemVerilog `string` spelling was equally silent.
    runs(
        "module t;\n  reg [7:0] mem [0:7]; string ff; integer i;\n\
         initial begin for (i = 0; i < 8; i = i + 1) mem[i] = 8'd0;\n\
         #1 ff = \"fw.hex\"; $readmemh(ff, mem);\n\
         #1 $display(\"VAL=%0d %0d\", mem[1], mem[3]); #5 $finish; end\n\
         initial #500 $finish;\nendmodule\n",
        &[],
        "VAL=11 13",
    );
}

#[test]
fn writememh_takes_a_reg_file_name_too() {
    // ⚠️ The two sites had the SAME six lines, and only the read side is the one anybody
    // probes — so they now share one helper rather than being fixed one at a time.
    let dir = runs(
        "module t;\n  reg [7:0] mem [0:3]; reg [1023:0] ff;\n\
         initial begin mem[0] = 8'hAA; mem[1] = 8'hBB; mem[2] = 8'hCC; mem[3] = 8'hDD;\n\
         #1 ff = \"o.hex\"; $writememh(ff, mem); $display(\"VAL=ok\"); #5 $finish; end\n\
         initial #500 $finish;\nendmodule\n",
        &[],
        "VAL=ok",
    );
    let got = std::fs::read_to_string(dir.join("o.hex")).expect("no file written");
    assert_eq!(got, "// 0x00000000\naa\nbb\ncc\ndd\n", "got:\n{got}");
}

#[test]
fn an_empty_file_name_warns_instead_of_vanishing() {
    // An all-NUL name decodes to "", which used to be indistinguishable from "the argument
    // was not a literal" — both took the same silent `return`.
    let (out, code, err, _) = run(
        "module t;\n  reg [7:0] mem [0:3]; reg [1023:0] ff; integer i;\n\
         initial begin for (i = 0; i < 4; i = i + 1) mem[i] = 8'd0;\n\
         #1 ff = 1024'd0; $readmemh(ff, mem);\n\
         #1 $display(\"VAL=%0d\", mem[0]); #5 $finish; end\n\
         initial #500 $finish;\nendmodule\n",
        &[],
    );
    assert_eq!(code, Some(0), "a bad name is a warning, not an error");
    assert!(out.contains("VAL=0"), "got:\n{out}");
    assert!(
        err.contains("W-RUN-READMEM") && err.contains("file-name argument is empty"),
        "stderr:\n{err}"
    );
}

// ── what already worked and must be unchanged ─────────────────────────────────────

#[test]
fn a_local_array_with_a_literal_name_is_unchanged() {
    runs(
        "module t;\n  reg [7:0] mem [0:7]; integer i;\n\
         initial begin for (i = 0; i < 8; i = i + 1) mem[i] = 8'd0;\n\
         #1 $readmemh(\"fw.hex\", mem);\n\
         #1 $display(\"VAL=%0d %0d\", mem[1], mem[3]); #5 $finish; end\n\
         initial #500 $finish;\nendmodule\n",
        &[],
        "VAL=11 13",
    );
}

#[test]
fn a_file_name_with_a_control_byte_keeps_stderr_textual() {
    // ⚠️ Letting the name be a VALUE lets it hold any byte — a trailing NUL from a
    // concatenation, or the raw f64 bytes of a `real`. Printed verbatim, one such byte
    // makes vita's whole stderr a BINARY stream, and `grep` then suppresses EVERY line in
    // it, so a CI log filter silently loses all diagnostics from the run. PRE never hit
    // this only because it returned without saying anything at all.
    //
    // The bytes still reach the filesystem call unescaped; only what a person reads is
    // escaped, so the message still shows what the design actually asked for.
    let (_, code, err, _) = run(
        "module t;\n  reg [7:0] mem [0:3]; integer i;\n\
         initial begin for (i = 0; i < 4; i = i + 1) mem[i] = 8'd0;\n\
         #1 $readmemh({\"fw.hex\", 8'd0}, mem);\n\
         #1 $display(\"VAL=%0d\", mem[0]); #5 $finish; end\n\
         initial #500 $finish;\nendmodule\n",
        &[],
    );
    assert_eq!(code, Some(0));
    assert!(!err.contains('\0'), "stderr must stay textual:\n{err:?}");
    assert!(
        err.contains(r"unable to open 'fw.hex\x00'"),
        "stderr:\n{err}"
    );
}

// ── the half that did NOT ship ────────────────────────────────────────────────────

#[test]
fn a_hierarchical_memory_argument_is_still_loud() {
    // `$readmemh(f, dut.mem)` — the firmware-loading idiom serv, picorv32 and ibex all use.
    // It was built, measured correct across forty-odd shapes, and then reverted, because
    // vita runs a PARENT's `initial` before its child's while both oracles run the child's
    // first. A RAM that loads its own memory therefore overwrote the testbench's load, at
    // exit 0 — so opening this construct traded a loud reject for a silent wrong answer.
    //
    // The ordering is pre-existing and independent (a plain `u1.s = 8'hAA` hierarchical
    // write already loses to the child's `initial`, with no `$readmem` anywhere), and it
    // lives in a frozen IR type, so fixing it needs an out-of-band process-rank sidecar of
    // its own. Recorded as the prerequisite in `docs/ROADMAP.md` §3 ④.
    //
    // Pinned so the next attempt starts from a measured statement rather than the queue's.
    loud(&format!(
        "{CHILD}module t;\n  ram dut();\n\
         initial begin #1 $readmemh(\"fw.hex\", dut.mem);\n\
         #1 $display(\"VAL=%0d\", dut.mem[1]); #5 $finish; end\n\
         initial #500 $finish;\nendmodule\n"
    ));
}

#[test]
fn hierarchical_element_select_works_and_always_did() {
    // ⚠️ The queue line named THIS as the gap. It has been wrong for about two months —
    // three slices in June (N3.1, its multi-dim follow-on, and HIER-REST) closed the read,
    // the write, and the element/bit/part-select forms. A census of eleven element shapes
    // found no failure. Pinned so the rewritten queue line stays honest.
    runs(
        &format!(
            "{CHILD}module t;\n  ram dut(); integer x;\n\
             initial begin #1 dut.mem[2] = 8'd77; dut.mem[1][3:0] = 4'hA;\n\
             #1 x = dut.mem[2]; $display(\"VAL=%0d %0d\", x, dut.mem[1]); #5 $finish; end\n\
             initial #500 $finish;\nendmodule\n"
        ),
        &[],
        "VAL=77 10",
    );
}
