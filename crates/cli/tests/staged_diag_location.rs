//! §3.10/#9 — STAGED elaborate diagnostics carry the SAME `file:line:col` as one-shot.
//!
//! `vcmp` serialized only the `SourceUnit`; the preprocessor's `SourceMap` died with
//! the process, so `velab` elaborated with `resolver: None` and every elaborate-time
//! diagnostic printed location-less while the identical one-shot run printed
//! `file:line:col` (the machinery existed since §4.5.249 — the staged path just could
//! not feed it). Since format v28 the `.vu` carries a source-map tail and `velab`
//! installs the same `MapResolver` the one-shot driver uses.
//!
//! Every test here compares the DIAGNOSTIC LINE between the two paths and pins the
//! location absolutely — parity alone would pass two location-less lines, which is
//! exactly the defect this file exists to keep closed.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn fresh_dir() -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_stagloc_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_in(d: &std::path::Path, args: &[&str]) -> (String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .current_dir(d)
        .output()
        .expect("spawn vita");
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.code().unwrap_or(-1),
    )
}

/// The first line containing `code` — the unit both paths must agree on.
fn diag_line(transcript: &str, code: &str) -> String {
    transcript
        .lines()
        .find(|l| l.contains(code))
        .unwrap_or_else(|| panic!("no `{code}` line in:\n{transcript}"))
        .to_string()
}

/// One-shot vs vcmp→velab over the same files; returns the two `code` lines.
/// `entries` are written into one fresh dir; ALL of them are passed to the
/// compile step (the include closure rides the preprocessor, not argv).
fn both_paths(files: &[(&str, &str)], argv: &[&str], code: &str) -> (String, String) {
    let d = fresh_dir();
    for (name, src) in files {
        std::fs::write(d.join(name), src).unwrap();
    }
    let mut one = vec![];
    one.extend_from_slice(argv);
    let (oneshot, _) = run_in(&d, &one);
    let mut vc = vec!["vcmp", "-o", "t.vu"];
    vc.extend_from_slice(argv);
    let (vcmp_out, vcmp_code) = run_in(&d, &vc);
    assert_eq!(vcmp_code, 0, "vcmp must succeed:\n{vcmp_out}");
    let (staged, _) = run_in(&d, &["velab", "t.vu", "-o", "t.velab"]);
    let a = diag_line(&oneshot, code);
    let b = diag_line(&staged, code);
    let _ = std::fs::remove_dir_all(&d);
    (a, b)
}

// The staged diagnostic equals the one-shot one BYTE-FOR-BYTE, and both carry a
// location — the anti-vacuity half: before v28 this test's equality would have
// failed (one side located, one not), and a regression that strips BOTH sides
// must not sneak past as "still equal".
#[test]
fn a_staged_elab_diagnostic_matches_one_shot_byte_for_byte() {
    let src =
        "`timescale 1ns/1ns\nmodule top;\n  wire w;\n  assign w = undeclared_name;\nendmodule\n";
    let (one, staged) = both_paths(&[("e.sv", src)], &["e.sv"], "VITA-E3010");
    assert_eq!(one, staged);
    assert!(
        one.starts_with("e.sv:2:8: error[VITA-E3010]"),
        "one-shot must locate (file:line:col prefix), got: {one}"
    );
}

// A module DECLARED in an `include`d file resolves to THAT file's name and its
// LOCAL line — the verbatim cross-file segment. A map that lost the per-file
// split would name the entry file with a global line.
#[test]
fn an_included_files_diagnostic_names_that_file_with_its_local_line() {
    let inc =
        "// pad1\n// pad2\n// pad3\nmodule top;\n  wire h;\n  assign h = ghost_net;\nendmodule\n";
    let top = "`timescale 1ns/1ns\n`include \"inc4.svh\"\n";
    let (one, staged) = both_paths(&[("inc4.svh", inc), ("t.sv", top)], &["t.sv"], "VITA-E3010");
    assert_eq!(one, staged);
    assert!(
        one.starts_with("inc4.svh:4:8:"),
        "must name the INCLUDED file at its local line, got: {one}"
    );
}

// A declaration-precise span INSIDE a macro body resolves to the macro USE site
// (the collapsed segment's pinned origin). This is the only case in this file
// whose bytes flow through a `collapsed` segment — a wire form that dropped the
// flag would resolve into the middle of the `\`define` line instead.
#[test]
fn a_macro_expanded_construct_resolves_to_the_use_site_on_the_staged_path() {
    let src = "`timescale 1ns/1ns\n`define DECL automatic int x;\nmodule t;\ninitial begin\n  begin\n    `DECL\n    if (x == 0) $display(\"q\");\n    x = 1;\n  end\n  $finish;\nend\nendmodule\n";
    let (one, staged) = both_paths(&[("m.sv", src)], &["m.sv"], "VITA-E3009");
    assert_eq!(one, staged);
    assert!(
        one.starts_with("m.sv:6:5:"),
        "must point at the `DECL use site, got: {one}"
    );
}

// Two command-line sources: the second file's diagnostic keeps ITS name and its
// FILE-LOCAL line (G12 per-file map entries survive the wire).
#[test]
fn a_second_command_line_file_keeps_its_own_name_and_local_line() {
    let f1 = "`timescale 1ns/1ns\nmodule a; wire x; assign x = 1'b0; endmodule\n";
    let f2 = "module top;\n  a u();\n  wire y;\n  assign y = nope_here;\nendmodule\n";
    let (one, staged) = both_paths(
        &[("f1.sv", f1), ("f2.sv", f2)],
        &["f1.sv", "f2.sv"],
        "VITA-E3010",
    );
    assert_eq!(one, staged);
    assert!(
        one.starts_with("f2.sv:1:8:"),
        "must name the SECOND file at its local line, got: {one}"
    );
}

// Column is a CHAR count (IEEE has nothing to say; vita's own one-shot contract):
// a multibyte comment before the module keyword pushes the byte column to 21 but
// the char column stays 17. The wire tail carries the ORIGINAL text, so the
// staged column must match; a tail that carried a line TABLE instead of text
// would print 21 here.
#[test]
fn a_multibyte_comment_before_the_span_keeps_the_char_column() {
    let src = "`timescale 1ns/1ns\n/* 안녕 */ module top; wire q; assign q = miss42; endmodule\n";
    let (one, staged) = both_paths(&[("mb.sv", src)], &["mb.sv"], "VITA-E3010");
    assert_eq!(one, staged);
    assert!(
        one.starts_with("mb.sv:2:17:"),
        "char column (17), not byte column (21), got: {one}"
    );
}

// A `.vu` whose source-map tail is truncated is LOUD (E-ART-FORMAT-MISMATCH,
// exit 2 "rebuild upstream") — never a silent fall-back to unlocated
// diagnostics, which would quietly resurrect the very gap v28 closes.
#[test]
fn a_corrupt_source_map_trailer_is_loud() {
    let d = fresh_dir();
    std::fs::write(
        d.join("c.sv"),
        "`timescale 1ns/1ns\nmodule top; wire w; assign w = 1'b0; endmodule\n",
    )
    .unwrap();
    let (out, code) = run_in(&d, &["vcmp", "c.sv", "-o", "c.vu"]);
    assert_eq!(code, 0, "vcmp must succeed:\n{out}");
    let bytes = std::fs::read(d.join("c.vu")).unwrap();
    std::fs::write(d.join("c.vu"), &bytes[..bytes.len() - 1]).unwrap();
    let (out, code) = run_in(&d, &["velab", "c.vu", "-o", "c.velab"]);
    assert!(
        out.contains("undecodable .vu source-map trailer"),
        "must name the trailer:\n{out}"
    );
    assert_ne!(code, 0, "corrupt artifact must not exit 0");
    let _ = std::fs::remove_dir_all(&d);
}

// ABSENCE of the tail (body truncated exactly at the map-tail boundary — the
// SourceUnit and timescale frames both intact) is as loud as corruption. A
// tolerant `None` here would silently print location-less diagnostics again,
// i.e. resurrect the exact gap the tail exists to end.
#[test]
fn an_absent_source_map_trailer_is_loud_not_silently_unlocated() {
    let d = fresh_dir();
    std::fs::write(
        d.join("c.sv"),
        "`timescale 1ns/1ns\nmodule top; wire w; assign w = 1'b0; endmodule\n",
    )
    .unwrap();
    let (out, code) = run_in(&d, &["vcmp", "c.sv", "-o", "c.vu"]);
    assert_eq!(code, 0, "vcmp must succeed:\n{out}");
    // Cut the body at the exact end of the v22 timescale frame, dropping the
    // whole v28 map tail: header ++ SourceUnit ++ TsEnv survive undamaged.
    let bytes = std::fs::read(d.join("c.vu")).unwrap();
    let (_h, body) = vita_artifact::read_vu(&bytes).expect("read_vu");
    let (_unit, rest1): (hdl_ast::SourceUnit, &[u8]) =
        postcard::take_from_bytes(body).expect("SourceUnit frame");
    type TsEnv = (
        std::collections::BTreeMap<String, i8>,
        i8,
        std::collections::BTreeMap<String, i8>,
    );
    let (_ts, rest2): (TsEnv, &[u8]) = postcard::take_from_bytes(rest1).expect("timescale frame");
    assert!(
        !rest2.is_empty(),
        "a v28 vcmp must have written the map tail"
    );
    let cut = bytes.len() - rest2.len();
    std::fs::write(d.join("c.vu"), &bytes[..cut]).unwrap();
    let (out, code) = run_in(&d, &["velab", "c.vu", "-o", "c.velab"]);
    assert!(
        out.contains("undecodable .vu source-map trailer"),
        "absence must be loud, not tolerated:\n{out}"
    );
    assert_ne!(code, 0);
    let _ = std::fs::remove_dir_all(&d);
}
