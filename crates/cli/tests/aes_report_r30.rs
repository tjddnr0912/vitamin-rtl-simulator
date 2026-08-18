//! External report aes_top round-3 (2026-08-18) — the log as data.
//!
//! Zero value defects again across their whole regression (14 targets, GCM/XTS/CMAC/CCM,
//! P7 integration). Every item is about what the log SAYS, and one of them turned out to
//! be a value defect the report could not see from outside.
//!
//! - **§3.2 string escapes.** They reported that `"\r"` reads 0x0D here and the letter
//!   `r` in Xcelium, which cost them two sign-off round trips (a `.vec` parser trimmed
//!   line ends with `== "\r"`, real trailing `r` characters were cut, and 807 CMAC
//!   vectors reported "MAC mismatch"). Grounding that against iverilog AND Verilator
//!   found the larger fact: **five escapes that IEEE 1800-2017 Table 5-1 DOES define
//!   were silently wrong here** — `\ddd`, `\xhh`, `\v`, `\f`, `\a` all produced the
//!   literal character plus a retained backslash, so both the value and the string's
//!   WIDTH were wrong. Their own suggested workarounds (`"\015"`, `"\x0D"`) were two of
//!   them. Those are now correct; `\r` and any other undefined escape keep vita's
//!   established reading and report `W3059`.
//! - **§3.4** `W3056 … default kept` fired on `string` parameter overrides that WERE
//!   applied. A diagnostic stating the opposite of what happened is worse than none.
//! - **§3.8** the only identifier a diagnostic printed (`VITA-W3056`) was rejected by
//!   every flag that takes one; `-Werror=all`, which `--help` documented, did not exist.
//! - **§3.9** `--obs-dir`/`--probe`/`--hier-tree`/`--inst-paths` were absent from
//!   `--help`. They found `--obs-dir` after six days, inside an unrelated error message.
//! - **§3.10** no elaborate or runtime warning carried a position of any kind. Measured
//!   on picorv32: 71 diagnostics, 0 with a `file:line:col`.
//! - **§3.11** three backends within 5% on their design. `wall_s` is one number, so
//!   nothing in the log could say whether the front end or the executor owned the time.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r30_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    (text, out.status.code())
}

fn run(src: &str) -> (String, Option<i32>) {
    run_args(src, &[])
}

// ───────────────────── §3.2 · IEEE 1800 Table 5-1 escapes ─────────────────────

/// The five escapes Table 5-1 defines and vita did not implement, plus the octal
/// and hex boundary rules. **Every value here is iverilog-pinned** (Verilator
/// agrees except that it demands two hex digits; Table 5-1 says "one or two", so
/// the permissive reading is the LRM's).
///
/// `$bits` is asserted next to every value and is not decoration: the old
/// behaviour kept the backslash, so `"\v"` was 16 bits holding `\` and `v`. A
/// value-only check on a one-byte comparison could pass on the low byte alone.
#[test]
fn table_5_1_escapes_have_their_defined_values() {
    let (o, code) = run("module t; initial begin\n\
         $display(\"A %0d %0d\", 8'(\"\\101\"), $bits(\"\\101\"));\n\
         $display(\"B %0d %0d\", 8'(\"\\1\"),   $bits(\"\\1\"));\n\
         $display(\"C %0d %0d\", 8'(\"\\12\"),  $bits(\"\\12\"));\n\
         $display(\"D %0d %0d\", 8'(\"\\377\"), $bits(\"\\377\"));\n\
         $display(\"E %0d %0d\", 16'(\"\\0601\"), $bits(\"\\0601\"));\n\
         $display(\"F %0d %0d\", 8'(\"\\x41\"), $bits(\"\\x41\"));\n\
         $display(\"G %0d %0d\", 8'(\"\\x4\"),  $bits(\"\\x4\"));\n\
         $display(\"H %0d %0d\", 16'(\"\\x412\"), $bits(\"\\x412\"));\n\
         $display(\"V %0d %0d\", 8'(\"\\v\"),   $bits(\"\\v\"));\n\
         $display(\"F2 %0d %0d\", 8'(\"\\f\"),  $bits(\"\\f\"));\n\
         $display(\"A2 %0d %0d\", 8'(\"\\a\"),  $bits(\"\\a\"));\n\
         $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{o}");
    for (label, want) in [
        ("A 65 8", "3-digit octal"),
        ("B 1 8", "1-digit octal"),
        ("C 10 8", "2-digit octal"),
        ("D 255 8", "max octal"),
        (
            "E 12337 16",
            "octal stops at three digits, then literal `1`",
        ),
        ("F 65 8", "2-digit hex"),
        ("G 4 8", "1-digit hex is legal (Table 5-1: one or two)"),
        ("H 16690 16", "hex stops at two digits, then literal `2`"),
        ("V 11 8", "vertical tab"),
        ("F2 12 8", "form feed"),
        ("A2 7 8", "bell"),
    ] {
        assert!(
            o.contains(label),
            "{want}: expected `{label}` (iverilog-pinned):\n{o}"
        );
    }
}

/// A Table 5-1 escape must NOT warn. Without this row the whole diagnostic could
/// be "warn on every backslash" and the test above would still pass.
#[test]
fn a_defined_escape_is_silent() {
    let (o, _) = run("module t; initial begin\n\
         $display(\"%0d\", 8'(\"\\101\") + 8'(\"\\x41\") + 8'(\"\\v\") + 8'(\"\\f\")\n\
                        + 8'(\"\\a\") + 8'(\"\\n\") + 8'(\"\\t\") + 8'(\"\\\\\") + 8'(\"\\\"\"));\n\
         $finish; end endmodule\n");
    assert!(
        !o.contains("W3059"),
        "no Table 5-1 escape may report W3059:\n{o}"
    );
}

/// The report's own case. The warning must name BOTH readings — "this is
/// non-standard" alone does not tell a reader which tool will disagree or how,
/// which is the entire information a single-simulator box cannot obtain.
#[test]
fn an_undefined_escape_names_both_readings() {
    let (o, code) = run("module t; initial begin\n\
         $display(\"%0d %0d\", (8'd13 == \"\\r\"), (8'd114 == \"\\r\"));\n\
         $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{o}");
    // The VALUE is deliberately unchanged: IEEE does not define `\r`, the two
    // oracles genuinely split (Verilator reads 0x0D like vita, iverilog reads
    // the letter), so there is no majority to follow — only a fact to report.
    assert!(o.contains("1 0"), "`\\r` still reads 0x0D:\n{o}");
    let line = o
        .lines()
        .find(|l| l.contains("W3059"))
        .unwrap_or_else(|| panic!("expected W3059:\n{o}"));
    assert!(line.contains("Table 5-1"), "cites the clause: {line}");
    assert!(line.contains("0x0D"), "says what vita read: {line}");
    assert!(
        line.contains("iverilog") && line.contains("Xcelium"),
        "names who disagrees, which is the part a dev box cannot find out: {line}"
    );
}

/// An escape outside Table 5-1 that vita does NOT give a C meaning keeps both
/// characters, so the string is a byte WIDER too — the message has to say that,
/// because a width difference propagates into every comparison the string is in.
#[test]
fn an_unknown_escape_reports_the_width_difference() {
    let (o, _) = run("module t; initial begin\n\
         $display(\"%0d\", $bits(\"\\q\")); $finish; end endmodule\n");
    assert!(o.contains("16"), "vita keeps both bytes:\n{o}");
    let line = o
        .lines()
        .find(|l| l.contains("W3059"))
        .unwrap_or_else(|| panic!("expected W3059:\n{o}"));
    assert!(line.contains("WIDER"), "names the width difference: {line}");
}

/// One line per DISTINCT escape per literal, and two literals on one statement
/// are two locations. Anchoring at `cur_span` would print the same
/// `file:line:col` twice, which is the defect §3.10 is about — reproduced inside
/// the diagnostic this slice adds.
#[test]
fn the_escape_warning_is_per_literal_and_deduped() {
    let (o, _) = run("module t; initial begin\n\
         $display(\"%0d %0d\", 8'(\"\\r\"), 8'(\"\\q\"));\n\
         $display(\"%0d\", $bits(\"\\r\\r\\r\"));\n\
         $finish; end endmodule\n");
    let lines: Vec<&str> = o.lines().filter(|l| l.contains("W3059")).collect();
    assert_eq!(lines.len(), 3, "2 on line 2 + 1 deduped on line 3:\n{o}");
    let cols: Vec<&str> = lines
        .iter()
        .map(|l| l.split(": ").next().unwrap_or(""))
        .collect();
    assert_ne!(
        cols[0], cols[1],
        "two escapes on one statement, one column:\n{o}"
    );
}

/// An escape is a property of the SOURCE, so one literal is one report however
/// many times its module is instantiated. `lower_expr` runs per elaboration, so
/// without a per-span latch a leaf instantiated N times scales a portability
/// note by N — found by reviewing this slice's own output, not by the report.
#[test]
fn the_escape_warning_does_not_scale_with_instance_count() {
    let (o, _) = run("`timescale 1ns/1ps\n\
         module leaf(input logic i, output logic o);\n\
           assign o = i;\n\
           initial if (8'd13 == \"\\r\") $display(\"x\");\n\
         endmodule\n\
         module t; logic i=0; logic [3:0] o;\n\
           leaf u0(.i(i), .o(o[0]));\n\
           leaf u1(.i(i), .o(o[1]));\n\
           leaf u2(.i(i), .o(o[2]));\n\
           leaf u3(.i(i), .o(o[3]));\n\
           initial begin #1 $finish; end\n\
         endmodule\n");
    assert_eq!(
        o.matches("W3059").count(),
        1,
        "one source literal is one report, not one per instance:\n{o}"
    );
    // Anti-vacuity: the latch is keyed by SPAN, so a second literal elsewhere
    // must still report — a latch keyed by the escape alone would hide it.
    let (o2, _) = run("`timescale 1ns/1ps\n\
         module leaf(input logic i, output logic o);\n\
           assign o = i;\n\
           initial if (8'd13 == \"\\r\") $display(\"x\");\n\
           initial if (8'd13 == \"\\r\") $display(\"y\");\n\
         endmodule\n\
         module t; logic i=0, o0, o1;\n\
           leaf u0(.i(i), .o(o0));\n\
           leaf u1(.i(i), .o(o1));\n\
           initial begin #1 $finish; end\n\
         endmodule\n");
    assert_eq!(
        o2.matches("W3059").count(),
        2,
        "two distinct literals are two reports:\n{o2}"
    );
}

// ───────────────── §3.4 · a warning that said the opposite ─────────────────

/// A `string` parameter override has no i64 value BY CONSTRUCTION, so `value:
/// None` never meant "dropped" for it — it rides the `str` channel and
/// `bind_params` applies it. The values prove the override landed; the absence
/// of the warning is the fix.
#[test]
fn an_applied_string_override_says_nothing() {
    let (o, code) = run("`timescale 1ns/1ps\n\
         module leaf #(parameter string MODE=\"X\", parameter logic [3:0] F=4'h0)\n\
                     (output logic [7:0] o);\n\
           assign o = (MODE==\"Y\") ? {4'hA, F} : {4'h5, F};\n\
         endmodule\n\
         module t;\n\
           logic [7:0] a, b, c, d;\n\
           leaf                       u0 (.o(a));\n\
           leaf #(.MODE(\"Y\"))         u1 (.o(b));\n\
           leaf #(.MODE(\"Y\"), .F('1)) u2 (.o(c));\n\
           leaf #(\"Y\")                u3 (.o(d));\n\
           initial begin #1 $display(\"%02h %02h %02h %02h\", a, b, c, d); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "{o}");
    assert!(o.contains("50 a0 af a0"), "every override applied:\n{o}");
    assert!(
        !o.contains("default kept"),
        "the override was applied; saying it was not is worse than silence:\n{o}"
    );
}

/// …and the warning is still there for the case it describes. Without this row
/// the fix above is indistinguishable from deleting the diagnostic.
#[test]
fn an_override_that_really_is_dropped_still_warns() {
    let (o, _) = run("`timescale 1ns/1ps\n\
         module leaf #(parameter int W=4) (output logic [7:0] o); assign o = W; endmodule\n\
         module t; logic [7:0] a; logic [3:0] sig = 4'h7;\n\
           leaf #(.W(sig)) u (.o(a));\n\
           initial begin #1 $display(\"a=%02h\", a); $finish; end\n\
         endmodule\n");
    assert!(
        o.contains("default kept"),
        "a non-constant override on NO channel must still warn:\n{o}"
    );
}

// ──────────────── §3.8 · the printed identifier must be the key ────────────────

/// Every diagnostic prints its mnemonic next to its number. doc-15 is the
/// reference a reader is sent to and 42 of its 55 worked examples print this
/// form; the product printed none of them.
#[test]
fn a_diagnostic_prints_its_mnemonic() {
    let (o, _) = run("module t; initial begin #1 $finish; end endmodule\n");
    assert!(
        o.contains("[VITA-W1017] W-PP-TIMESCALE-DEFAULT:"),
        "number and mnemonic on the same line:\n{o}"
    );
}

/// Every form a diagnostic prints must work in every flag that takes a code.
/// This is one table on purpose: the defect was that `explain` accepted the
/// number and the gate flags did not, which is two spellings of one question.
#[test]
fn a_code_flag_accepts_every_printed_form() {
    const SRC: &str = "`timescale 1ns/1ps\n\
                       module leaf #(parameter int W=4) (output logic [7:0] o);\n\
                         assign o = W; endmodule\n\
                       module t; logic [7:0] a; logic [3:0] sig = 4'h7;\n\
                         leaf #(.W(sig)) u (.o(a));\n\
                         initial begin #1 $display(\"a=%02h\", a); $finish; end\n\
                       endmodule\n";
    // Baseline: exactly one W3056 to act on (E3009 also fires and is untouched).
    let (base, _) = run(SRC);
    assert_eq!(base.matches("VITA-W3056").count(), 1, "{base}");

    for form in ["W3056", "VITA-W3056", "W-ELAB-FEATURE-LIMIT", "w3056"] {
        let (o, _) = run_args(SRC, &[&format!("-Wno-{form}")]);
        assert!(
            !o.contains("VITA-W3056"),
            "-Wno-{form} must suppress it:\n{o}"
        );
        let (o, _) = run_args(SRC, &[&format!("-Werror={form}")]);
        assert!(
            o.contains("error[VITA-W3056]"),
            "-Werror={form} must promote it:\n{o}"
        );
    }
    // `--help` has always documented "all, or one code"; `=all` was the half of
    // that sentence which did not exist.
    let (o, _) = run_args(SRC, &["-Werror=all"]);
    assert!(
        o.contains("error[VITA-W3056]"),
        "-Werror=all promotes:\n{o}"
    );
}

/// A typo is still loud, and now says what a code looks like instead of only
/// that this one is not one.
#[test]
fn an_unknown_code_says_which_forms_exist() {
    let (o, code) = run_args(
        "module t; initial $finish; endmodule\n",
        &["-Wno-NOT-A-CODE"],
    );
    assert_ne!(code, Some(0), "a typo stays a usage error:\n{o}");
    assert!(o.contains("VITA-E0001"), "{o}");
    assert!(
        o.contains("mnemonic") && o.contains("VITA-W3056") && o.contains("W3056"),
        "the rejection lists the accepted forms:\n{o}"
    );
}

/// `explain` takes the same forms. It already took two of the three; the point
/// is that ONE resolver now answers for all three consumers.
#[test]
fn explain_takes_the_same_forms() {
    for form in ["E3009", "VITA-E3009", "E-ELAB-UNSUPPORTED", "e3009"] {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .args(["explain", form])
            .output()
            .expect("run vita explain");
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(
            text.contains("VITA-E3009"),
            "explain {form} must resolve:\n{text}"
        );
    }
}

// ──────────────────── §3.9 · a flag absent from --help ────────────────────

/// Each observability flag is named in `--help` AND is really accepted. Listing
/// a flag that does not exist would be a worse failure than not listing one.
#[test]
fn help_lists_the_observability_flags_and_they_work() {
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--help")
        .output()
        .expect("run vita --help");
    let help = String::from_utf8_lossy(&out.stdout).into_owned();
    for f in ["--obs-dir", "--probe", "--hier-tree", "--inst-paths"] {
        assert!(help.contains(f), "`{f}` missing from --help:\n{help}");
    }

    let d = std::env::temp_dir().join(format!("vita_r30_help_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("t.sv"),
        "`timescale 1ns/1ps\nmodule t; logic o = 0; initial begin #1 o = 1; #1 $finish; end endmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args([
            "t.sv",
            "--obs-dir",
            "obs",
            "--probe",
            "t.o",
            "--hier-tree",
            "h.txt",
            "--inst-paths",
            "p.txt",
        ])
        .current_dir(&d)
        .output()
        .expect("run vita");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for f in ["obs/run.json", "obs/trace.jsonl", "h.txt", "p.txt"] {
        assert!(d.join(f).exists(), "`{f}` not written");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ─────────────────── §3.10 · a diagnostic says where it is ───────────────────

/// The report's minimal repro: one leaf, three instantiations, four warnings that
/// were byte-identical. The property is that they are now PAIRWISE DISTINCT —
/// counting them is what the reporter was reduced to.
#[test]
fn an_elaborate_warning_names_its_file_line_and_instance() {
    let (o, _) = run("`timescale 1ns/1ps\n\
         module leaf(input logic i, output logic o1, output logic o2);\n\
           assign o1=i; assign o2=~i; endmodule\n\
         module mid(input logic i, output logic o); leaf u_a(.i(i), .o1(o), .o2()); endmodule\n\
         module t; logic i=0, o1, o2;\n\
           mid  u_m1(.i(i), .o(o1));\n\
           mid  u_m2(.i(i), .o(o2));\n\
           leaf u_l (.i(i), .o1(), .o2());\n\
           initial begin #1 $display(\"o=%0b%0b\", o1, o2); $finish; end\n\
         endmodule\n");
    let lines: Vec<&str> = o.lines().filter(|l| l.contains("VITA-W3056")).collect();
    assert_eq!(lines.len(), 4, "four unconnected outputs:\n{o}");
    for l in &lines {
        assert!(l.starts_with("t.sv:"), "no file:line:col: {l}");
        assert!(l.contains("[in t."), "no instance path: {l}");
    }
    let mut sorted = lines.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        4,
        "two warnings a reader cannot tell apart:\n{o}"
    );
    // The two `u_a` rows share a source line and differ ONLY by instance path —
    // the case `file:line:col` alone cannot separate, and the reason the
    // instance path is not decoration.
    assert!(
        lines.iter().any(|l| l.contains("[in t.u_m1.u_a]"))
            && lines.iter().any(|l| l.contains("[in t.u_m2.u_a]")),
        "the two instantiations of one leaf must be distinguishable:\n{o}"
    );
}

/// A runtime range diagnostic names the ARRAY. Two arrays and both kinds
/// (unknown index → W4029, known-out-of-range → E4002) so a fixed string cannot
/// pass, and all three backends must agree — the message crosses the arena, the
/// engine store and the deferred wprog lane, which are three different places
/// that had to learn the net.
#[test]
fn a_runtime_range_diagnostic_names_the_array() {
    const SRC: &str = "`timescale 1ns/1ps\n\
                       module t;\n\
                         logic [7:0] rom [0:3];\n\
                         logic [7:0] ram [0:7];\n\
                         logic [1:0] xi;\n\
                         logic [7:0] a, b, c;\n\
                         initial begin\n\
                           a = rom[xi];\n\
                           b = ram[xi];\n\
                           ram[xi] = 8'h11;\n\
                           c = rom[3'd6];\n\
                           $display(\"%02h %02h %02h\", a, b, c);\n\
                           $finish;\n\
                         end\n\
                       endmodule\n";
    for backend in ["native", "vm", "interp"] {
        let (o, _) = run_args(SRC, &["--backend", backend]);
        assert_eq!(
            o.matches("of `t.rom`").count(),
            2,
            "{backend}: one read + one out-of-range on rom:\n{o}"
        );
        assert_eq!(
            o.matches("of `t.ram`").count(),
            2,
            "{backend}: one read + one write on ram:\n{o}"
        );
        assert!(
            o.contains("VITA-W4029") && o.contains("VITA-E4002"),
            "{backend}: both kinds present:\n{o}"
        );
        assert!(
            o.contains("[at time 0]"),
            "{backend}: the round-29 timestamp is still there:\n{o}"
        );
    }
}

// ─────────────── §3.11 · which half of the wall clock was it ───────────────

/// `run.json` splits the front end from `simulate`. One number cannot attribute
/// a run: comparing `--backend` values compares runs whose front end is
/// identical, so a front-end-dominated design shows every backend within noise —
/// which is the measurement the report drew "the backend does not help" from.
#[test]
fn run_json_splits_elaborate_from_simulate() {
    let d = std::env::temp_dir().join(format!("vita_r30_phase_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("t.sv"),
        "`timescale 1ns/1ps\nmodule t; int i; initial begin for (i=0;i<20000;i=i+1) ; $finish; end endmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["t.sv", "--obs-dir", "obs"])
        .current_dir(&d)
        .output()
        .expect("run vita");
    assert_eq!(out.status.code(), Some(0));
    let j = std::fs::read_to_string(d.join("obs/run.json")).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    assert!(j.contains("\"elab_s\""), "no front-end time:\n{j}");
    assert!(j.contains("\"sim_s\""), "no simulate time:\n{j}");
    // Anti-vacuity: the two must be REAL and must add up. A design that spends
    // its time in a loop has to show it under `sim_s`, or the split is a pair of
    // constants.
    let num = |k: &str| -> f64 {
        j.split(&format!("\"{k}\": "))
            .nth(1)
            .and_then(|t| t.split([',', '\n']).next())
            .and_then(|t| t.trim().parse::<f64>().ok())
            .unwrap_or_else(|| panic!("no numeric `{k}`:\n{j}"))
    };
    let (wall, elab, sim) = (num("wall_s"), num("elab_s"), num("sim_s"));
    assert!(sim > elab, "a 20k-iteration loop is simulate time:\n{j}");
    assert!(
        elab + sim <= wall + 1e-6,
        "the parts cannot exceed the whole:\n{j}"
    );
}
