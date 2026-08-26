//! R2 (round-36) — `--obs-procs` / `--obs-procs-time`: the per-BUILTIN table in
//! `run.json`'s `builtins` object.
//!
//! SPEC = `docs/preview/19-ai-agent-observability.md` §4.9.
//!
//! WHY THE COUNTS ARE ASSERTED EXACTLY, and not merely "present and plausible".
//! The external report that asked for this reads one number and decides what to
//! rewrite. A `$sscanf` row that is off by the EOF iteration, or a `.size()`
//! swallowed into the `$display` that called it, sends them to the wrong line —
//! which is the failure the whole rail exists to prevent (doc-19 §3: a wrong log
//! is a silent-wrong). So every count below is derived by hand from the design
//! and the input file, and the derivation is written beside it. If a change
//! moves one of these numbers, redo the derivation; do not relax the assertion.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run vita on `src` with extra `args`, in a per-test directory that also holds
/// `files` (name, contents) — the vector file a `$fopen` in the design reads.
///
/// The unique directory is `obs_procs.rs`'s recipe and load-bearing for the same
/// reason: `n` restarts at 0 in every test PROCESS and the OS recycles PIDs, so
/// two runs can otherwise land on one directory.
fn run_with(src: &str, files: &[(&str, &str)], args: &[&str]) -> (String, i32, PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_obsbi_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for (name, body) in files {
        std::fs::write(d.join(name), body).unwrap();
    }
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let obs = d.join("obsout");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .arg("--obs-dir")
        .arg(obs.to_str().unwrap())
        .args(args)
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code().unwrap_or(-1),
        obs,
    )
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

/// `calls` for one builtin NAME, or `None` when the table has no such row.
///
/// A hand parser and not a JSON crate, for `obs_procs.rs`'s reason: the CLI
/// hand-serializes this file with no JSON dependency, and asserting through a
/// lenient reader would let a formatting bug hide.
fn calls(json: &str, name: &str) -> Option<u64> {
    let pat = format!("{{\"name\": \"{name}\", \"calls\": ");
    let i = json.find(&pat)? + pat.len();
    let rest = &json[i..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// The `builtins` object's text, so field-level assertions cannot accidentally
/// match the `processes` object above it.
fn builtins_obj(json: &str) -> String {
    let i = json.find("\"builtins\": ").expect("no builtins key");
    let rest = &json[i..];
    let end = rest.find("\"utc_unix_s\"").unwrap_or(rest.len());
    rest[..end].to_string()
}

/// THE hand-checkable design: the external report's vector-driver shape in
/// miniature — a `$fopen`/`$fgets`/`$sscanf` loop with a queue push, a nested
/// task, one failing record, and a `.size()` evaluated INSIDE a `$display`
/// argument (which is what makes the nesting assertion possible).
const VECDRV: &str = r#"module tb;
  int    fd, a, b, n, got, checked;
  string line;
  int    q[$];

  task automatic check_rec(input int x, input int y);
    if (x + 1 != y) $warning("bad record %0d %0d", x, y);
    checked++;
  endtask

  initial begin
    fd = $fopen("vec.txt", "r");
    got = $fgets(line, fd);
    while (got != 0) begin
      n = $sscanf(line, "%d %d", a, b);
      if (n == 2) begin
        q.push_back(a);
        check_rec(a, b);
      end
      got = $fgets(line, fd);
    end
    $fclose(fd);
    $display("checked=%0d size=%0d", checked, q.size());
  end
endmodule
"#;

/// Four records; the THIRD is deliberately inconsistent so exactly one
/// `$warning` fires. Four lines ⇒ five `$fgets` (the fifth returns 0 at EOF).
const VEC: &str = "0 1\n1 2\n2 9\n3 4\n";

/// SHAPE + EXACT COUNTS, each derived by hand from `VECDRV` + `VEC`.
#[test]
fn builtin_counts_are_hand_checkable() {
    let (stdout, code, obs) = run_with(VECDRV, &[("vec.txt", VEC)], &["--obs-procs"]);
    assert_eq!(code, 0, "$warning does not change the exit class");
    assert!(stdout.contains("checked=4 size=4"), "stdout: {stdout}");
    let j = read(&obs.join("run.json"));

    // ── the derivation ───────────────────────────────────────────────────
    // $fopen      : 1  — one open.
    // $fgets      : 5  — one per line (4) plus the one that hits EOF and
    //                    returns 0, which is what ends the loop. An assertion
    //                    of 4 here would be the classic off-by-the-EOF-read.
    // $sscanf     : 4  — one per line read successfully.
    // .push_back(): 4  — every line parses two fields, so every one pushes.
    // $warning    : 1  — record 3 ("2 9") is the only inconsistent one. This is
    //                    the row that proves the label un-folds `SysTaskId::
    //                    Display`: a `$warning` is a Display plus a severities
    //                    sidecar entry, and naming it "$display" would hide it.
    // $fclose     : 1
    // $display    : 1
    // .size()     : 1  — evaluated as an ARGUMENT of that $display, i.e. NESTED
    //                    inside another builtin. Its own row is what proves a
    //                    nested builtin is charged to itself rather than
    //                    absorbed by its caller.
    assert_eq!(calls(&j, "$fopen"), Some(1), "{j}");
    assert_eq!(calls(&j, "$fgets"), Some(5), "the EOF read counts too: {j}");
    assert_eq!(calls(&j, "$sscanf"), Some(4), "{j}");
    assert_eq!(calls(&j, ".push_back()"), Some(4), "{j}");
    assert_eq!(calls(&j, "$warning"), Some(1), "severity un-folded: {j}");
    assert_eq!(calls(&j, "$fclose"), Some(1), "{j}");
    assert_eq!(calls(&j, "$display"), Some(1), "{j}");
    assert_eq!(calls(&j, ".size()"), Some(1), "nested, not absorbed: {j}");
    // A `$warning` must NOT also appear as a `$display`, or the two rows would
    // double-count the one call.
    assert_eq!(calls(&j, "$display"), Some(1));

    let b = builtins_obj(&j);
    // The two fields that answer "may I add these up?". They are the contract a
    // reader needs and are asserted so they cannot quietly change meaning.
    assert!(b.contains("\"attribution\": \"self\""), "{b}");
    assert!(b.contains("\"included_in_processes\": true"), "{b}");
    assert!(b.contains("\"timed\": false"), "counts-only run: {b}");
    assert!(
        !b.contains("time_s"),
        "an untimed run must not emit a 0.0 that reads as `costs nothing`: {b}"
    );
    // distinct = the 8 names above; total_calls = 1+5+4+4+1+1+1+1 = 18.
    assert!(b.contains("\"distinct\": 8"), "{b}");
    assert!(b.contains("\"total_calls\": 18"), "{b}");
}

/// Rows are sorted most-called-first, with a TOTAL tiebreak on the name — the
/// property that makes two runs of one design byte-diffable.
#[test]
fn builtin_rows_are_sorted_and_totally_ordered() {
    let (_, _, obs) = run_with(VECDRV, &[("vec.txt", VEC)], &["--obs-procs"]);
    let b = builtins_obj(&read(&obs.join("run.json")));
    let names: Vec<&str> = b
        .lines()
        .filter_map(|l| l.trim().strip_prefix("{\"name\": \""))
        .map(|r| &r[..r.find('"').unwrap()])
        .collect();
    let counts: Vec<u64> = names
        .iter()
        .map(|n| calls(&b, n).expect("row parses"))
        .collect();
    assert!(
        counts.windows(2).all(|w| w[0] >= w[1]),
        "descending by calls: {names:?} {counts:?}"
    );
    // The five singletons ($display, $fclose, $fopen, $warning, .size()) all
    // have `calls == 1`, so their order is decided entirely by the name
    // tiebreak — which is exactly the case a non-total order would leave free.
    let ones: Vec<&&str> = names
        .iter()
        .zip(&counts)
        .filter(|(_, &c)| c == 1)
        .map(|(n, _)| n)
        .collect();
    let mut sorted = ones.clone();
    sorted.sort();
    assert_eq!(ones, sorted, "equal-count rows sort by name: {ones:?}");
}

/// Two runs of the same input produce a byte-identical `builtins` object. This
/// is the determinism claim the counts half of the rail rests on.
#[test]
fn builtin_table_is_byte_identical_across_runs() {
    let a = {
        let (_, _, o) = run_with(VECDRV, &[("vec.txt", VEC)], &["--obs-procs"]);
        builtins_obj(&read(&o.join("run.json")))
    };
    let b = {
        let (_, _, o) = run_with(VECDRV, &[("vec.txt", VEC)], &["--obs-procs"]);
        builtins_obj(&read(&o.join("run.json")))
    };
    assert_eq!(a, b);
}

/// Every backend charges the SAME accumulators. The four instrumented seams are
/// spread across the interpreter, the VM and the tier-3 walk, so a table that
/// changed with `--backend` would mean one of them was missed — the "routing
/// lives in several places" trap this codebase keeps hitting.
#[test]
fn builtin_counts_are_backend_invariant() {
    let get = |backend: &str| {
        let (_, _, o) = run_with(
            VECDRV,
            &[("vec.txt", VEC)],
            &["--obs-procs", "--backend", backend],
        );
        builtins_obj(&read(&o.join("run.json")))
    };
    let n = get("native");
    let v = get("vm");
    let i = get("interp");
    assert_eq!(n, v, "native vs vm");
    assert_eq!(n, i, "native vs interp");
}

/// `--obs-procs-time` adds `time_s` and flips `timed` WITHOUT moving a count.
/// The VALUE is wall clock and is never asserted — only its presence and the
/// invariance of the deterministic half.
#[test]
fn timing_adds_time_s_without_moving_the_counts() {
    let (_, _, o) = run_with(VECDRV, &[("vec.txt", VEC)], &["--obs-procs-time"]);
    let timed = builtins_obj(&read(&o.join("run.json")));
    assert!(timed.contains("\"timed\": true"), "{timed}");
    assert!(timed.contains("\"time_s\": "), "{timed}");
    for (name, n) in [("$fgets", 5), ("$sscanf", 4), (".push_back()", 4)] {
        assert_eq!(calls(&timed, name), Some(n), "{name} moved under timing");
    }
}

/// Without `--obs-procs` the key is `null`, NOT an empty object: a consumer must
/// be able to tell "not measured" from "measured, no builtin ran". Same
/// convention `processes` uses.
#[test]
fn without_the_flag_the_table_is_null() {
    let (_, _, o) = run_with(VECDRV, &[("vec.txt", VEC)], &[]);
    let j = read(&o.join("run.json"));
    assert!(j.contains("\"builtins\": null"), "{j}");
    assert!(j.contains("\"processes\": null"), "{j}");
}

/// A `$display` inside a subroutine BODY is counted.
///
/// ⚠️ This is the seam that does not go through `builtins::dispatch`: vita has a
/// synchronous `&self` frame executor with its own three system-task arms, and a
/// hook only in the shared dispatch would report `$display: 1` here. The
/// assertion is 3 regardless of which executor vita picks for `f`, which is the
/// point — the profile is a property of the run, not of the routing.
#[test]
fn a_display_inside_a_function_body_is_counted() {
    let src = "module tb;\n\
        \x20 int r;\n\
        \x20 function automatic int f(input int x);\n\
        \x20   $display(\"f %0d\", x);\n\
        \x20   return x + 1;\n\
        \x20 endfunction\n\
        \x20 initial begin\n\
        \x20   r = f(1);\n\
        \x20   r = f(2);\n\
        \x20   $display(\"r=%0d\", r);\n\
        \x20 end\n\
        endmodule\n";
    let (stdout, code, o) = run_with(src, &[], &["--obs-procs"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("r=3"), "stdout: {stdout}");
    let j = read(&o.join("run.json"));
    assert_eq!(
        calls(&j, "$display"),
        Some(3),
        "two prints inside `f` plus one outside: {j}"
    );
}
