//! §4.5.194 (was §4.5.178 loud): a FRAMED function with an `input` dynamic-array formal
//! (supported for READING since §4.5.177 via a per-activation heap snapshot) that WRITES
//! that formal — `b[0]=9`, `foreach(b[i]) b[i]=…`. §4.5.178 made this a loud F4004 because
//! the synchronous `&self` frame executor could not mutate the `&mut` heap. Now that
//! `dyn_heap` is interior-mutable (§4.5.194 Phase 0), the write is a real heap element store
//! on the function's per-activation SNAPSHOT — IEEE §13.5.1 pass-by-value: the local copy is
//! modified and observed by later reads, while the CALLER's array is unchanged.
//!
//! The RETURN value has an iverilog oracle (matched below). The caller-side isolation is
//! hand-IEEE: iverilog implements a dyn `input` formal as pass-by-REFERENCE (the caller's
//! array shows the write), which violates §13.5.1 — vita's snapshot is the correct
//! pass-by-value, so caller isolation is verified by construction, not against iverilog.
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

// ── the write is now a real pass-by-value store (was loud in §4.5.178) ─────

#[test]
fn direct_rhs_write_element_then_read_now_supported() {
    // b[0]=9; return b[0] — IEEE pass-by-value: the local copy write returns 9. iverilog: 9.
    let o = run("module t;\n\
         function automatic int f(input int b[]); b[0]=9; return b[0]; endfunction\n\
         int r; initial begin int a[]; a=new[3]; a[0]=1; r=f(a); $display(\"r=%0d\", r); end\n\
         endmodule\n");
    assert!(
        o.1 && o.0.contains("r=9"),
        "pass-by-value local write returns 9:\n{}",
        o.0
    );
}

#[test]
fn direct_rhs_write_element_then_size() {
    // b[0]=9; return b.size() — the write does not affect the returned size. iverilog: 3.
    let o = run("module t;\n\
         function automatic int f(input byte b[]); b[0]=9; return b.size(); endfunction\n\
         int r; initial begin byte a[]; a=new[3]; r=f(a); $display(\"r=%0d\", r); end\n\
         endmodule\n");
    assert!(
        o.1 && o.0.contains("r=3"),
        "write + return size = 3:\n{}",
        o.0
    );
}

#[test]
fn direct_rhs_foreach_write_element() {
    // foreach(b[i]) b[i]=b[i]+1; return b[0] — 4+1 = 5 on the local copy. iverilog: 5.
    let o = run("module t;\n\
         function automatic int f(input int b[]); foreach(b[i]) b[i]=b[i]+1; return b[0]; endfunction\n\
         int r; initial begin int a[]; a=new[3]; a[0]=4;a[1]=5;a[2]=6; r=f(a); $display(\"r=%0d\", r); end\n\
         endmodule\n");
    assert!(
        o.1 && o.0.contains("r=5"),
        "foreach local write, b[0]=5:\n{}",
        o.0
    );
}

#[test]
fn write_is_pass_by_value_isolated_from_caller() {
    // The write lands in the local snapshot only — the caller's array a[0] stays 1
    // (IEEE §13.5.1). iverilog aliases (a0=9) and is non-compliant here; vita is correct.
    let o = run("module t;\n\
         function automatic int f(input int b[]); b[0]=9; return b[0]; endfunction\n\
         int r; initial begin int a[]; a=new[2]; a[0]=1; a[1]=2; r=f(a); $display(\"r=%0d a0=%0d\", r, a[0]); end\n\
         endmodule\n");
    assert!(
        o.1 && o.0.contains("r=9 a0=1"),
        "caller array unchanged (pass-by-value):\n{}",
        o.0
    );
}

// ── a READ-ONLY body is UNAFFECTED (no regression) ─────────────────────────

#[test]
fn direct_rhs_read_only_still_supported() {
    let o = run("module t;\n\
         function automatic int f(input int b[]); int s=0; foreach(b[i]) s+=b[i]; return s; endfunction\n\
         int r; initial begin int a[]; a=new[3]; a[0]=4;a[1]=5;a[2]=6; r=f(a); $display(\"r=%0d\", r); end\n\
         endmodule\n");
    assert!(
        o.1 && o.0.contains("r=15"),
        "a read-only dyn-formal function must still run (15):\n{}",
        o.0
    );
}
