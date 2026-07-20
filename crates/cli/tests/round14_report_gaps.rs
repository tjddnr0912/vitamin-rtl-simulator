//! Round-14 external report — the round-13 ★V10 comb-sensitivity silent-wrong was
//! confirmed FIXED (see `comb_lhs_index_sens.rs`); this file covers the two loud
//! gaps cleared in the round-14 slice, plus the loud→silent-wrong twin that the deep
//! (adversarial) verification pass surfaced and fixed:
//!
//!   V9  64-bit MSB-set literal in a scalar localparam (`64'h8000_0000_0000_0000`)
//!       → folded (was E3009 "not foldable"). The fold reinterprets the full-width
//!       bit pattern into the i64 const domain; a magnitude misuse (huge index)
//!       stays LOUD, so it never becomes a silent-wrong.
//!   V1  function/task/block-local `string` variable (`string s; s = $sformatf(...)`)
//!       → real heap-backed `NetKind::String` frame slot (`frame_local_net_kind`),
//!       procedurally assignable and RETURNable (was E3018 "assignment to net").
//!   V1-twin  a string METHOD (`.len()`/`.substr()`/`.getc()`) on that frame-local
//!       slot — value-use worked but the method read hit the static net and returned
//!       an empty string (0). Fixed by making the engine's `str_bytes` frame-aware
//!       (mirrors `read_net`). Without the twin fix, V1 would have shipped a
//!       loud→silent-wrong regression.
//!
//! Oracles: every V1 case is verified vs iverilog 13.0 (all PASS there). V9 folds
//! vs iverilog PASS. Elaborate value-only + an eval-only engine read change; no
//! sim-ir shape change (format_version stays 22).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn workdir() -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r14_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// (full trimmed stdout, success) for a one-shot vita run of `src`.
fn run(src: &str) -> (String, bool) {
    let d = workdir();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).trim().to_owned(),
        out.status.success(),
    )
}

// ───────────────────────────── V9: 64-bit MSB-set literal fold ──────────────
#[test]
fn v9_msb64_scalar_localparam_folds() {
    // `64'h8000_0000_0000_0000` = i64::MAX + 1 as unsigned; the old `i64::try_from`
    // rejected it as "not foldable" while the bit-pattern-identical `7FFF…` folded.
    // A width-64 literal is a bit container → reinterpret its bits. Both compare equal.
    let src = "module m(output int o);\n\
        localparam logic [63:0] A = 64'h7FFFFFFFFFFFFFFF;\n\
        localparam logic [63:0] B = 64'h8000000000000000;\n\
        initial o = (A == 64'h7FFFFFFFFFFFFFFF && B == 64'h8000000000000000);\n\
        initial $display(\"o=%0d\", o); endmodule";
    let (out, code) = run(src);
    assert!(code, "V9 localparam must fold, got:\n{out}");
    assert!(out.contains("o=1"), "V9 MSB-set fold, got:\n{out}");
}

#[test]
fn v9_msb64_as_array_index_stays_loud() {
    // SOUNDNESS: the fold reinterprets the bit pattern, so a width-64 MSB-set literal
    // becomes i64::MIN. Used as an ARRAY INDEX that is a genuine magnitude misuse —
    // it must NOT silently write element 0; the out-of-range access stays LOUD (E4002,
    // rc≠0). Guards the fold against converting a loud reject into a silent-wrong.
    let src = "module m;\n\
        logic [7:0] a [0:3];\n\
        initial begin a[64'h8000000000000000] = 8'hAA; $display(\"a0=%0h\", a[0]); end endmodule";
    let (out, code) = run(src);
    assert!(
        !code,
        "MSB-set index misuse must be loud (rc≠0), got:\n{out}"
    );
    assert!(
        !out.contains("a0=aa"),
        "must not silently write elem 0, got:\n{out}"
    );
}

// ───────────────────────── V1: frame-local `string` variable ────────────────
#[test]
fn v1_frame_string_local_build_and_return() {
    // The report's V1 (round-10 G4 residual): build a string in a function local and
    // return it. Was E3018 (the local was a Wire slot); now a heap `NetKind::String`.
    let src = "module m(output int o);\n\
        function automatic string f(input int n); string s; s = $sformatf(\"%0d\", n); return s; endfunction\n\
        initial begin o = (f(42) == \"42\"); $display(\"o=%0d\", o); end endmodule";
    let (out, code) = run(src);
    assert!(code, "V1 build-and-return must work, got:\n{out}");
    assert!(out.contains("o=1"), "V1 string return, got:\n{out}");
}

#[test]
fn v1_frame_string_local_method_len() {
    // V1-TWIN: `.len()` on a frame-local string. Value-use (return/concat) worked but
    // the method read hit the static net (empty) → silent 0. The engine `str_bytes`
    // frame fix routes it to the live frame slot. "123".len() == 3.
    let src = "module m(output int o);\n\
        function automatic int f(input int n); string s; s = $sformatf(\"%0d\", n); return s.len(); endfunction\n\
        initial begin o = f(123); $display(\"o=%0d\", o); end endmodule";
    let (out, code) = run(src);
    assert!(code, "V1 .len() must work, got:\n{out}");
    assert!(out.contains("o=3"), "V1 frame-local .len(), got:\n{out}");
}

#[test]
fn v1_frame_string_local_substr() {
    // V1-TWIN breadth: `.substr()` on a frame-local string. "hello".substr(1,3)=="ell".
    let src = "module m(output int o);\n\
        function automatic string f(); string s; s = \"hello\"; return s.substr(1, 3); endfunction\n\
        initial begin o = (f() == \"ell\"); $display(\"o=%0d\", o); end endmodule";
    let (out, code) = run(src);
    assert!(code, "V1 .substr() must work, got:\n{out}");
    assert!(out.contains("o=1"), "V1 frame-local .substr(), got:\n{out}");
}

#[test]
fn v1_task_local_string_method() {
    // A TASK-local string written then read via `.len()` (TASKSTR). Same frame slot
    // machinery as functions; the twin fix covers it. "123".len() == 3.
    let src = "module m(output int o);\n\
        task automatic g(input int n, output int r); string s; s = $sformatf(\"%0d\", n); r = s.len(); endtask\n\
        initial begin int r; g(123, r); o = r; $display(\"o=%0d\", o); end endmodule";
    let (out, code) = run(src);
    assert!(code, "V1 task-local string method must work, got:\n{out}");
    assert!(out.contains("o=3"), "V1 task-local .len(), got:\n{out}");
}

#[test]
fn v1_block_local_string_method() {
    // A BLOCK-local string (`begin string s; … end`) written then `.len()` (BLKSTR).
    // Exercises the `reserve_frame_block_locals` `frame_local_net_kind` site. len==3.
    let src = "module m(output int o);\n\
        function automatic int f(input int n); begin string s; s = $sformatf(\"x%0d\", n); return s.len(); end endfunction\n\
        initial begin o = f(42); $display(\"o=%0d\", o); end endmodule";
    let (out, code) = run(src);
    assert!(code, "V1 block-local string method must work, got:\n{out}");
    assert!(out.contains("o=3"), "V1 block-local .len(), got:\n{out}");
}

#[test]
fn v1_string_input_formal_unchanged() {
    // GUARD: a string INPUT formal keeps its Wire-slot + str_params materialization
    // (NOT re-routed to a heap slot) — the frame fix is scoped to frame-local decls.
    let src = "module m(output int o);\n\
        function automatic int len(input string s); return s.len(); endfunction\n\
        initial begin o = len(\"abcd\"); $display(\"o=%0d\", o); end endmodule";
    let (out, code) = run(src);
    assert!(code, "string input formal must still work, got:\n{out}");
    assert!(
        out.contains("o=4"),
        "input-formal .len() unchanged, got:\n{out}"
    );
}

#[test]
fn v1_module_string_method_unchanged() {
    // GUARD: a MODULE-scope string is NOT frame-local, so `str_bytes` keeps its
    // authoritative dyn_heap path (embedded-NUL preserving). "world".len() == 5.
    let src = "module m(output int o);\n\
        string w;\n\
        initial begin w = \"world\"; o = w.len(); $display(\"o=%0d\", o); end endmodule";
    let (out, code) = run(src);
    assert!(code, "module string method must still work, got:\n{out}");
    assert!(out.contains("o=5"), "module .len() unchanged, got:\n{out}");
}
