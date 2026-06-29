//! FORK-TIE-CAP (2026-06-22 hostile-input hardening): a fork child's
//! deterministic ordering key packs `(parent_tie + 1)` into the high 16 bits and
//! the child declaration index into the low 16 (`compose_child_tie`). Past ~65535
//! top-level processes the high half overflows the u32; past ~65536 fork arms the
//! low half aliases (`child_idx & 0xFFFF`). Either silently breaks deterministic
//! sibling ordering — the cap (a comment-only claim before) must become a loud
//! graceful fatal at the spawn site.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_forktie_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// A fork with > 65536 arms would alias the low-16 child index → reject loud
/// (graceful fatal) instead of silently corrupting deterministic ordering.
#[test]
fn fork_arm_explosion_is_loud_not_alias() {
    let arms = "#0;".repeat(65_537); // > 0x1_0000
    let src = format!("module top;\ninitial fork {arms} join\nendmodule\n");
    let (_out, err, code) = run(&src);
    assert_ne!(
        code,
        Some(0),
        "a fork that overflows the tie encoding must NOT exit clean. stderr:\n{err}"
    );
    let low = err.to_lowercase();
    assert!(
        low.contains("fork")
            && (low.contains("tie") || low.contains("too many") || low.contains("limit")),
        "must carry the fork-tie diagnostic; stderr:\n{err}"
    );
}

/// A realistic fork (a handful of arms) still runs clean — the cap never trips.
#[test]
fn small_fork_is_clean() {
    let src = "module top;\n\
         reg a=0, b=0, c=0;\n\
         initial fork a=1; b=1; c=1; join\n\
         initial #1 $finish;\n\
         endmodule\n";
    let (_out, err, code) = run(src);
    assert_eq!(code, Some(0), "a 3-arm fork must run clean. stderr:\n{err}");
    assert!(
        !err.to_lowercase().contains("fork") || !err.to_lowercase().contains("tie"),
        "cap must not false-trip on a 3-arm fork; stderr:\n{err}"
    );
}
