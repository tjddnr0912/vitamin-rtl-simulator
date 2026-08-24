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

// ── the hierarchical memory argument (§3 ④) ───────────────────────────────────────

#[test]
fn a_hierarchical_memory_argument_loads_the_child_s_memory() {
    // `$readmemh(f, dut.mem)` — the firmware-loading idiom serv, picorv32 and ibex all use.
    //
    // ⚠️ This test previously asserted the OPPOSITE, and the reason it gave was measured
    // wrong. §4.5.375 built this, matched forty-odd shapes on both oracles, then reverted
    // it on the claim that "vita runs a PARENT's `initial` before its child's while BOTH
    // ORACLES run the child's first", which would make a RAM that loads its own memory
    // overwrite the testbench's load. Re-measured on the exact design that claim cites:
    // iverilog prints `aa bb cc dd`, and **verilator prints `01 02 03 04`** — vita's
    // answer. Verified not to be a dropped write: with the child's competing load removed,
    // verilator honours the parent's hierarchical `$readmemh` (`aa bb cc dd`). So the
    // write-vs-write case is an ORACLE SPLIT, not a two-oracle silent-wrong, and vita
    // lands on verilator's side of it.
    //
    // The competition it feared is also absent from every testbench that motivated the
    // feature: serv passes `.memfile(...)` and never sets `+firmware=`, so its hierarchical
    // load never fires; picorv32's `wb_ram` is instantiated without `.memfile`; picorv32's
    // `axi4_memory` has no load of its own. Not one has a child that loads the same array.
    //
    // What IS a two-oracle defect is a parent `initial` READING a child net at t0 (both
    // oracles give the value, vita gives X) — a separate, pre-existing ordering row that
    // no corpus design exercises. Recorded in `docs/ROADMAP.md` §2 row 7; it does not
    // gate this construct.
    runs(
        &format!(
            "{CHILD}module t;\n  ram dut();\n\
             initial begin #1 $readmemh(\"fw.hex\", dut.mem);\n\
             #1 $display(\"VAL=%0d\", dut.mem[1]); #5 $finish; end\n\
             initial #500 $finish;\nendmodule\n"
        ),
        &[],
        "VAL=11",
    );
}

#[test]
fn a_hierarchical_memory_argument_reaches_through_two_levels() {
    // `expr_array_view` already joins dotted segments, so depth is not a separate case —
    // pinned because the fix records an EID, and an eid-keyed exemption that only worked
    // at depth 1 would be indistinguishable from one that works at every depth until a
    // grandchild is asked for.
    runs(
        &format!(
            "{CHILD}module mid;\n  ram r();\nendmodule\n\
             module t;\n  mid dut();\n\
             initial begin #1 $readmemh(\"fw.hex\", dut.r.mem);\n\
             #1 $display(\"VAL=%0d\", dut.r.mem[2]); #5 $finish; end\n\
             initial #500 $finish;\nendmodule\n"
        ),
        &[],
        "VAL=12",
    );
}

#[test]
fn a_parenthesised_hierarchical_memory_argument_is_the_same_reference() {
    // `(dut.mem)` is `dut.mem`. The shape predicate must peel parens — asking the question
    // one node too high answered "no" here, which is the soundness NIT §4.5.375 recorded
    // against the first attempt's twin predicate.
    runs(
        &format!(
            "{CHILD}module t;\n  ram dut();\n\
             initial begin #1 $readmemh(\"fw.hex\", (dut.mem));\n\
             #1 $display(\"VAL=%0d\", dut.mem[3]); #5 $finish; end\n\
             initial #500 $finish;\nendmodule\n"
        ),
        &[],
        "VAL=13",
    );
}

#[test]
fn a_child_loading_its_own_memory_at_t0_is_an_oracle_split() {
    // The design §4.5.375 reverted on: the child loads its own memory at t0 and the parent
    // loads the same array hierarchically at t0. Measured on all three:
    //
    //     iverilog  aa bb cc dd   (child's initial runs first, parent's load wins)
    //     verilator 01 02 03 04   (parent's runs first, child's load wins)
    //     vita      01 02 03 04   (== verilator)
    //
    // IEEE 1800 §4.7 makes the order of execution of `initial` procedures explicitly
    // nondeterministic, and the two oracles use that freedom in opposite directions, so
    // there is no answer to be wrong about here — the §4.5.372 precedent (a cont-assign
    // order where verilator sided with vita) ruled the same way.
    //
    // Pinned as a SPLIT, not as a correct answer: if a future slice reorders t0 processes
    // to match iverilog (§2 row 7), this value flips, and it should flip deliberately with
    // this comment read, not silently.
    runs(
        "module ram;\n  reg [7:0] mem [0:7];\n\
         initial for (int i = 0; i < 8; i = i + 1) mem[i] = 8'd200 + i[7:0];\n\
         endmodule\n\
         module t;\n  ram dut();\n\
         initial $readmemh(\"fw.hex\", dut.mem);\n\
         initial begin #1 $display(\"VAL=%0d\", dut.mem[1]); #5 $finish; end\n\
         initial #500 $finish;\nendmodule\n",
        &[],
        "VAL=201",
    );
}

#[test]
fn a_whole_hierarchical_array_is_still_not_a_value() {
    // The guard the fix relaxes still has its real job: `x = dut.mem;` asks for a VALUE,
    // and a whole unpacked array has none. The exemption is scoped to the `$readmem*`
    // MEMORY POSITION, so this must stay loud — otherwise the slice traded a correct
    // rejection for a silent word-0 read.
    loud(&format!(
        "{CHILD}module t;\n  ram dut(); reg [7:0] x;\n\
         initial begin #1 x = dut.mem; $display(\"VAL=%0d\", x); #5 $finish; end\n\
         initial #500 $finish;\nendmodule\n"
    ));
}

#[test]
fn an_event_in_the_memory_position_is_still_loud() {
    // Only the WHOLE-ARRAY arm of the guard is exempted. A named event has no array to
    // hand over either, so the memory position does not rescue it — the exemption answers
    // "is an array the operand here", not "skip this guard".
    loud(
        "module ram;\n  event ev;\nendmodule\n\
         module t;\n  ram dut();\n\
         initial begin #1 $readmemh(\"fw.hex\", dut.ev); #5 $finish; end\n\
         initial #500 $finish;\nendmodule\n",
    );
}

#[test]
fn a_dynamic_handle_in_the_memory_position_is_still_loud() {
    // The other non-array arm, for the same reason.
    loud(
        "module ram;\n  int dq[];\nendmodule\n\
         module t;\n  ram dut();\n\
         initial begin #1 $readmemh(\"fw.hex\", dut.dq); #5 $finish; end\n\
         initial #500 $finish;\nendmodule\n",
    );
}

#[test]
fn readmem_into_a_hierarchical_const_array_parameter_is_loud() {
    // The local arm denies `$readmem` into a desugared array parameter; the hierarchical
    // arm has to deny it at RESOLVE time, because the net is not known at lowering. Before
    // the fix this was loud for the wrong reason ("no plain readable value"); it must stay
    // loud for the right one.
    let (out, code, err, _d) = run(
        "module ram;\n  localparam int P[0:3] = '{1,2,3,4};\nendmodule\n\
         module t;\n  ram dut();\n\
         initial begin #1 $readmemh(\"fw.hex\", dut.P); #5 $finish; end\n\
         initial #500 $finish;\nendmodule\n",
        &[],
    );
    let all = format!("{out}{err}");
    assert_ne!(code, Some(0), "must reject: {all}");
    assert!(
        all.contains("$readmem into parameter"),
        "must name the parameter, not the readable-value guard: {all}"
    );
}

#[test]
fn writemem_of_a_hierarchical_const_array_parameter_is_allowed() {
    // The write-side twin of the check above, and the reason it is not applied to the whole
    // family: `$writemem*` only READS the memory, so a parameter is a legitimate source.
    // §4.5.375's soundness lens raised exactly this over-application as a NIT.
    let dir = runs(
        "module ram;\n  localparam int P[0:3] = '{1,2,3,4};\nendmodule\n\
         module t;\n  ram dut();\n\
         initial begin #1 $writememh(\"p.hex\", dut.P); $display(\"VAL=ok\"); #5 $finish; end\n\
         initial #500 $finish;\nendmodule\n",
        &[],
        "VAL=ok",
    );
    let got = std::fs::read_to_string(dir.join("p.hex")).expect("p.hex");
    assert!(
        got.contains("00000001") && got.contains("00000004"),
        "the parameter's elements must reach the file: {got}"
    );
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
