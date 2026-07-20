//! §4.5.178 (SILENT-WRONG → loud): a FRAMED function with an `input` dynamic-array formal
//! (supported for READING since §4.5.177 via a per-activation heap snapshot) that WRITES
//! that formal — `b[0]=9`, `foreach(b[i]) b[i]=…` — used to silently DROP the write. The
//! write runs in the synchronous `&self` frame executor (`run_frame_call` / `run_task`),
//! whose heap store (`write_lvalue` → `dyn_write`) is `&mut`; the attempted write landed in
//! the unused scalar frame slot while READS came from the heap, so `return b[0]` after
//! `b[0]=9` returned the pre-write value (a silent-wrong that §4.5.177's snapshot-soundness
//! argument tacitly assumed away by presuming a read-only body).
//!
//! Fix: a runtime guard in `frame_write_lvalue` fires F4004 the moment a heap dyn-array
//! write reaches an `&self` frame executor. Correct-or-loud, sound by construction (it
//! catches the actual write attempt, covering element/whole/`new[]` writes uniformly). A
//! READ-ONLY dyn-formal body is unaffected — reads never route through this write path.
//! These probes use the DIRECT-rhs (`r = f(a)`) call form, so they exercise the guard
//! independently of the §4.5.179 buried-call hoist.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (combined stdout+stderr, process_success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ffdfw_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (text, out.status.success())
}

/// Loud = the run FAILED (non-zero exit) — a runtime F4004 latches `call_fatal`, which the
/// scheduler converts to `FinishReason::Error` (`errors=1`, exit != 0).
fn is_loud(o: &(String, bool)) -> bool {
    !o.1
}

// ── the write silently vanished before §4.5.178; now loud ──────────────────

#[test]
fn direct_rhs_write_element_then_read_is_loud() {
    // b[0]=9; return b[0] — IEEE pass-by-value would return 9. vita cannot write the heap
    // copy in the `&self` executor, so rather than return the stale 1, it is loud (F4004).
    let o = run("module t;\n\
         function automatic int f(input int b[]); b[0]=9; return b[0]; endfunction\n\
         int r; initial begin int a[]; a=new[3]; a[0]=1; r=f(a); $display(\"r=%0d\", r); end\n\
         endmodule\n");
    assert!(
        is_loud(&o) && o.0.contains("F4004"),
        "a write to a dyn-array input formal must be loud (F4004), never a dropped write:\n{}",
        o.0
    );
}

#[test]
fn direct_rhs_write_element_then_size_is_loud() {
    // b[0]=9; return b.size() — the return does not observe the write, but the write is
    // still an unsupported heap mutation in the `&self` executor → loud (never a partial run).
    let o = run("module t;\n\
         function automatic int f(input byte b[]); b[0]=9; return b.size(); endfunction\n\
         int r; initial begin byte a[]; a=new[3]; r=f(a); $display(\"r=%0d\", r); end\n\
         endmodule\n");
    assert!(
        is_loud(&o),
        "write-to-formal (return size) must be loud:\n{}",
        o.0
    );
}

#[test]
fn direct_rhs_foreach_write_element_is_loud() {
    // foreach(b[i]) b[i]=b[i]+1 — every iteration writes the formal → loud.
    let o = run("module t;\n\
         function automatic int f(input int b[]); foreach(b[i]) b[i]=b[i]+1; return b[0]; endfunction\n\
         int r; initial begin int a[]; a=new[3]; a[0]=4;a[1]=5;a[2]=6; r=f(a); $display(\"r=%0d\", r); end\n\
         endmodule\n");
    assert!(
        is_loud(&o),
        "foreach element-write to formal must be loud:\n{}",
        o.0
    );
}

// ── a READ-ONLY body is UNAFFECTED by the guard (no over-fire) ─────────────

#[test]
fn direct_rhs_read_only_still_supported() {
    // The guard must fire ONLY on writes — a read-only foreach-sum still returns 15.
    let o = run("module t;\n\
         function automatic int f(input int b[]); int s=0; foreach(b[i]) s+=b[i]; return s; endfunction\n\
         int r; initial begin int a[]; a=new[3]; a[0]=4;a[1]=5;a[2]=6; r=f(a); $display(\"r=%0d\", r); end\n\
         endmodule\n");
    assert!(
        !is_loud(&o) && o.0.contains("r=15"),
        "a read-only dyn-formal function must still run (15):\n{}",
        o.0
    );
}
