//! RULEV-MTIME (2026-06-23, ROADMAP §5 option A): the vrun RULE-V auto-gate
//! grows an mtime+size fast-path. A 15th `.velab` trailer (`WorkStamps`) records,
//! per consumed entry, the (mtime, size) that velab observed WHILE verifying the
//! entry's content still hashed to the recorded digest. At vrun, RULE-V stats the
//! path: a matching (mtime, size) lets it trust the recorded hash and skip the
//! read+blake3; any mismatch falls back to the authoritative rehash.
//!
//! These tests pin the OBSERVABLE consequences of that fast-path:
//!   1. A frozen (mtime,size) makes vrun trust a stale file (the documented,
//!      industry-standard mtime hole) — this is the only externally visible
//!      proof the read was skipped, and it is RED before the optimization.
//!   2. A normal edit (size and/or mtime advance) is still caught — the gate's
//!      soundness for the realistic case is preserved.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn tdir() -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rulevmtime_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn vita(args: &[&str]) -> (String, String, Option<i32>) {
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .output()
        .expect("failed to run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

fn write(p: &std::path::Path, s: &str) {
    std::fs::write(p, s).unwrap();
}

fn mtime_of(p: &std::path::Path) -> std::time::SystemTime {
    std::fs::metadata(p).unwrap().modified().unwrap()
}

fn set_mtime(p: &std::path::Path, t: std::time::SystemTime) {
    std::fs::File::options()
        .write(true)
        .open(p)
        .unwrap()
        .set_modified(t)
        .unwrap();
}

/// Build a fresh worklib `.velab` from `src_text`; returns (lib dir, .velab path).
fn build(d: &std::path::Path, src: &std::path::Path, src_text: &str) -> std::path::PathBuf {
    write(src, src_text);
    let lib = d.join("w");
    let work = format!("work={}", lib.display());
    assert_eq!(
        vita(&["vcmp", src.to_str().unwrap(), "--work", &work]).2,
        Some(0),
        "vcmp --work"
    );
    let ve = d.join("t.velab");
    assert_eq!(
        vita(&[
            "velab",
            "-L",
            &work,
            "--top",
            "top",
            "-o",
            ve.to_str().unwrap()
        ])
        .2,
        Some(0),
        "velab -L"
    );
    ve
}

// Same byte length so only the content (and thus the hash) differs — isolates
// the (mtime,size) fast-path from any size-based shortcut.
const SRC_A: &str = "module top; initial begin $display(\"AAAA\"); $finish; end endmodule\n";
const SRC_B: &str = "module top; initial begin $display(\"BBBB\"); $finish; end endmodule\n";

#[test]
fn mtime_fastpath_trusts_frozen_stamp_and_skips_rehash() {
    let d = tdir();
    let src = d.join("top.sv");
    let ve = build(&d, &src, SRC_A);

    // Baseline: fresh snapshot runs and prints A.
    let (out, err, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(0), "fresh worklib runs: {err}");
    assert!(out.contains("AAAA"), "snapshot prints A; got:\n{out}");

    // Freeze (mtime,size): overwrite with same-length DIFFERENT content, then
    // restore the original mtime. Content now mismatches the recorded hash, but
    // the stat fingerprint is identical to what velab stamped.
    let frozen = mtime_of(&src);
    assert_eq!(SRC_A.len(), SRC_B.len(), "fixtures must be equal length");
    write(&src, SRC_B);
    set_mtime(&src, frozen);

    // The fast-path trusts the stamp → no rehash → snapshot still runs (exit 0),
    // and it runs the SNAPSHOT IR (A), never the on-disk B. Before the
    // optimization this rehashed B, mismatched, and refused with exit class 2.
    let (out, err, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(
        code,
        Some(0),
        "frozen (mtime,size) is trusted → runs without rehash; stderr:\n{err}"
    );
    assert!(
        out.contains("AAAA"),
        "ran the snapshot, not the on-disk edit; got:\n{out}"
    );
}

#[test]
fn normal_edit_changing_size_is_still_caught() {
    let d = tdir();
    let src = d.join("top.sv");
    let ve = build(&d, &src, SRC_A);
    assert_eq!(
        vita(&["vrun", ve.to_str().unwrap()]).2,
        Some(0),
        "fresh runs"
    );

    // A realistic edit changes the byte length even if mtime granularity is
    // coarse → the stat fingerprint diverges → authoritative rehash → stale.
    write(
        &src,
        "module top; initial begin $display(\"CCCCCCCC\"); $finish; end endmodule\n",
    );
    let (_, err, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(2), "edited source = stale exit class 2:\n{err}");
    assert!(err.contains("VITA-E9003"), "E9003 expected:\n{err}");
}

#[test]
fn frozen_mtime_but_changed_size_is_still_caught() {
    // Pin the AND in the fast-path: a matching mtime is NOT enough when the size
    // moved — velab stamps both, and RULE-V requires both to skip the rehash.
    let d = tdir();
    let src = d.join("top.sv");
    let ve = build(&d, &src, SRC_A);
    assert_eq!(
        vita(&["vrun", ve.to_str().unwrap()]).2,
        Some(0),
        "fresh runs"
    );

    let frozen = mtime_of(&src);
    write(
        &src,
        "module top; initial begin $display(\"DD\"); $finish; end endmodule\n", // shorter
    );
    set_mtime(&src, frozen); // mtime matches the stamp, size does not
    let (_, err, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(
        code,
        Some(2),
        "size mismatch defeats the mtime fast-path → stale:\n{err}"
    );
    assert!(err.contains("VITA-E9003"), "E9003 expected:\n{err}");
}
