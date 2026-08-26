//! V33-8 — the two runtime warnings that named WHAT but never WHERE.
//!
//! `W4029` (`W-RUN-RANGE-UNKNOWN` / its `E4002` twin) named the ARRAY and
//! `W4023` (`W-RUN-READMEM`) named the FILE, both with `location: None`. An
//! external report elaborated its RTL, got 11 warnings, NINE of them `W4029` on
//! ONE package table read from three different places, and had to bisect by
//! commenting reads out. Both now resolve through the same elaborate-time
//! `stmt_locs` record the severity diagnostics have used since #10.
//!
//! Neither oracle helps here: iverilog and verilator say NOTHING for an
//! out-of-range or unknown array index (that is why W4029 is a warning and not
//! an error), and neither warns about a failed `$readmem` open the way vita
//! does. So the CONTENT is vita's own and is pinned absolutely — the value
//! being pinned is that three reads of one array produce three DIFFERENT lines,
//! which is the reported defect stated as an assertion.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// One array, read at three different places and written at a fourth — the
/// reporter's shape. `pick` puts one of the reads inside a SUBROUTINE, which is
/// the case with a deliberate residue (see `a_read_inside_a_subroutine_is_named
/// _by_its_call_statement`).
const THREE_SITES: &str = "module t;
  reg [7:0] tab [0:3];
  integer i;
  reg [7:0] a, b, c;
  function automatic [7:0] pick(input integer k);
    pick = tab[k];
  endfunction
  initial begin
    i = 'x;
    a = tab[i];
    b = tab[i];
    c = pick(i);
    tab[i] = 8'h55;
    $display(\"%0h %0h %0h\", a, b, c);
    $finish;
  end
endmodule
";

/// The E4002 half of the same emitter: a KNOWN index past the end.
const KNOWN_OOR: &str = "module t;
  reg [7:0] mem [0:3];
  reg [7:0] v;
  initial begin
    v = mem[9];
    mem[7] = 8'h11;
    $display(\"%0h\", v);
  end
endmodule
";

/// Both accesses sit in a TERMINATOR condition, not in a statement.
const TERMINATOR_COND: &str = "module t;
  reg [7:0] mem [0:3];
  integer i;
  reg [7:0] q;
  initial begin
    i = 'x;
    q = 8'h01;
    if (mem[i]) q = 8'h02;
    while (mem[i] === 8'h00) q = 8'h03;
    $display(\"%0h\", q);
    $finish;
  end
endmodule
";

/// A declaration initializer, whose span is the initializer itself.
const DECL_INIT: &str = "module t;
  reg [7:0] mem [0:3];
  integer bad = 9;
  reg [7:0] v = mem[9];
  initial begin
    $display(\"%0h\", v);
    $finish;
  end
endmodule
";

const MEMFILES: &str = "module t;
  reg [7:0] mem [0:3];
  initial begin
    $readmemh(\"no_such_file.hex\", mem);
    $writememh(\"no_such_dir_zz/out.hex\", mem);
    $finish;
  end
endmodule
";

fn fresh_dir() -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rangeloc_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_in(d: &std::path::Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .current_dir(d)
        .output()
        .expect("spawn vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn one_shot_with(src: &str, extra: &[&str]) -> String {
    let d = fresh_dir();
    std::fs::write(d.join("d.sv"), src).unwrap();
    let mut args: Vec<&str> = vec!["d.sv"];
    args.extend_from_slice(extra);
    let r = run_in(&d, &args);
    let _ = std::fs::remove_dir_all(&d);
    r
}

fn one_shot(src: &str) -> String {
    one_shot_with(src, &[])
}

/// The `file:line:col` prefixes of the W4029 lines, in order.
fn range_sites(t: &str) -> Vec<String> {
    t.lines()
        .filter(|l| l.contains("VITA-W4029"))
        .map(|l| l.split(' ').next().unwrap_or("").to_string())
        .collect()
}

// THE REPORTED DEFECT, as an assertion: one array, four accesses, four lines.
// PRE printed four byte-identical lines that named only `t.tab`.
#[test]
fn every_access_to_one_array_reports_its_own_line() {
    let out = one_shot(THREE_SITES);
    assert_eq!(
        range_sites(&out),
        vec![
            "d.sv:10:5:".to_string(), // a = tab[i]     (read)
            "d.sv:11:5:".to_string(), // b = tab[i]     (read)
            "d.sv:12:5:".to_string(), // c = pick(i)    (read, inside the callee)
            "d.sv:13:5:".to_string(), // tab[i] = 8'h55 (write)
        ],
        "four accesses must produce four distinct anchors:\n{out}"
    );
    // The array name is NOT dropped in exchange — the reader wants both.
    assert!(
        out.contains("array word index of `t.tab` is unknown (x/z)"),
        "the array name must survive:\n{out}"
    );
}

// The backend is a DEBUG KNOB, so the same source must anchor identically under
// all three executors. This is the reason a read inside a subroutine is
// attributed to its CALL STATEMENT: the tier-3 arena can only record an
// out-of-range access and is drained at the caller's statement boundary, so the
// callee's own line is unavailable to it. Measured, not assumed — publishing
// the callee's StmtId made `interp` say d.sv:6 while `native` said nothing.
#[test]
fn all_three_backends_anchor_at_the_same_lines() {
    let interp = range_sites(&one_shot_with(THREE_SITES, &["--backend", "interp"]));
    let vm = range_sites(&one_shot_with(THREE_SITES, &["--backend", "vm"]));
    let native = range_sites(&one_shot_with(THREE_SITES, &["--backend", "native"]));
    assert_eq!(interp, vm, "interp vs vm");
    assert_eq!(interp, native, "interp vs native");
    assert_eq!(interp.len(), 4);
}

// The residue, pinned so it cannot drift silently: `pick`'s `tab[k]` is on line
// 6, and the report says line 12 — the call. Named by a test rather than left
// to a comment, because the next person to touch this will want line 6 and
// should find the reason before the code.
#[test]
fn a_read_inside_a_subroutine_is_named_by_its_call_statement() {
    let out = one_shot(THREE_SITES);
    assert!(
        out.contains("d.sv:12:5:") && !out.contains("d.sv:6:"),
        "the callee's access reports the CALL line, not the subscript line:\n{out}"
    );
}

// FAIL TO NONE, NEVER TO STALE. A branch/loop condition is evaluated AFTER the
// last statement of its basic block, so the executing-statement cursor is
// cleared before every terminator. The residue is an unanchored line; the bug
// this prevents is a confidently WRONG one (the previous statement's).
#[test]
fn a_terminator_condition_reports_no_location_rather_than_a_wrong_one() {
    let out = one_shot(TERMINATOR_COND);
    let w: Vec<&str> = out.lines().filter(|l| l.contains("VITA-W4029")).collect();
    assert_eq!(w.len(), 2, "both conditions must report:\n{out}");
    assert!(
        w.iter().all(|l| l.starts_with("warning[")),
        "a condition's access must carry NO anchor, not a neighbouring one:\n{out}"
    );
}

// A declaration initializer is a statement with a span of its own, so it anchors
// at the initializer rather than at the module header (`cur_span` during module
// elaboration IS the header — the trap `Elaborator::note_at` warns about).
#[test]
fn a_declaration_initializer_anchors_at_itself() {
    let out = one_shot(DECL_INIT);
    assert!(
        out.contains("d.sv:4:17: error[VITA-E4002]"),
        "decl-init anchor wrong:\n{out}"
    );
}

// The KNOWN-out-of-range twin. `W4029` and `E4002` are one emitter with two
// severities (a reset window's x/z index is not an error; walking past the end
// almost always is), so the location has to arrive for both or the split would
// have quietly made only half the family locatable.
#[test]
fn the_known_out_of_range_error_locates_too() {
    let out = one_shot(KNOWN_OOR);
    assert!(
        out.contains(
            "d.sv:5:5: error[VITA-E4002] E-RUN-RANGE: \
             array word index of `t.mem` (out of range; read X / write ignored) \
             [in t] [at time 0]"
        ),
        "read side:\n{out}"
    );
    assert!(
        out.contains("d.sv:6:5: error[VITA-E4002]"),
        "write side:\n{out}"
    );
}

// W4023, both directions. PRE named the file and nothing else, so a design that
// loads several memories emitted N interchangeable lines.
#[test]
fn readmem_and_writemem_open_failures_report_their_call_site() {
    let out = one_shot(MEMFILES);
    assert!(
        out.contains(
            "d.sv:4:5: warning[VITA-W4023] W-RUN-READMEM: \
             $readmem: unable to open 'no_such_file.hex' for reading [in t] [at time 0]"
        ),
        "$readmem line missing or unanchored:\n{out}"
    );
    assert!(
        out.contains("d.sv:5:5: warning[VITA-W4023] W-RUN-READMEM: $writemem: unable to open"),
        "$writemem line missing or unanchored:\n{out}"
    );
}

// The STAGED-DROP hazard class: the record rides the `.velab` extra-sidecars
// trailer, so a vcmp -> velab -> vrun run must reproduce the one-shot anchors.
// (This is why the location is resolved at ELABORATE and not at run time --
// vrun has no source map of its own beyond that trailer.)
#[test]
fn staged_vrun_anchors_identically_to_one_shot() {
    for src in [THREE_SITES, MEMFILES] {
        let d = fresh_dir();
        std::fs::write(d.join("d.sv"), src).unwrap();
        let one = run_in(&d, &["d.sv"]);
        run_in(&d, &["vcmp", "d.sv", "-o", "d.vu"]);
        run_in(&d, &["velab", "d.vu", "-o", "d.velab"]);
        let staged = run_in(&d, &["vrun", "d.velab"]);
        // BOTH codes, in one comparison: THREE_SITES produces only W4029 and
        // MEMFILES only W4023, so filtering to one of them would let the other
        // design pass on an empty list.
        let lines = |t: &str| -> Vec<String> {
            t.lines()
                .filter(|l| l.contains("VITA-W4029") || l.contains("VITA-W4023"))
                .map(str::to_string)
                .collect()
        };
        let a = lines(&one);
        assert!(
            !a.is_empty(),
            "probe must produce a located warning:\n{one}"
        );
        assert!(
            a.iter().all(|l| l.starts_with("d.sv:")),
            "every one-shot line must be anchored:\n{one}"
        );
        assert_eq!(a, lines(&staged), "staged vs one-shot anchors");
        let _ = std::fs::remove_dir_all(&d);
    }
}
