//! GEN-NET-CAP (2026-06-22 hostile-input hardening): `GENERATE_UNROLL_CAP` bounds
//! each generate loop individually, but nested generates MULTIPLY — a 33×4096
//! pair declares >2^17 nets with every loop still within its own cap. `add_net`
//! needs an AGGREGATE budget (`MAX_TOTAL_NETS`): above it, no-op + one loud
//! diagnostic, so the arena cannot OOM the process.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_gennet_{}_{n}", std::process::id()));
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

/// A nested generate that declares > MAX_TOTAL_NETS (2^17) nets must reject
/// loud (exit 1) instead of OOMing — each inner+outer loop is within its own
/// UNROLL_CAP, so only the aggregate budget catches it.
#[test]
fn nested_generate_net_explosion_is_loud_not_oom() {
    // 33 * 4096 = 135,168 `wire` nets > 131,072 cap.
    let src = "module top;\n\
         genvar i, j;\n\
         generate\n\
         for (i = 0; i < 33; i = i + 1) begin: a\n\
           for (j = 0; j < 4096; j = j + 1) begin: b\n\
             wire w;\n\
           end\n\
         end\n\
         endgenerate\n\
         endmodule\n";
    let (_out, err, code) = run(src);
    assert_eq!(
        code,
        Some(1),
        "net explosion must be a loud reject. stderr:\n{err}"
    );
    assert!(
        err.contains("exceeds the v1 cap") && err.to_lowercase().contains("net"),
        "must carry the net-budget diagnostic; stderr:\n{err}"
    );
}

/// A normal small design stays clean — the cap never perturbs realistic net
/// counts.
#[test]
fn small_design_under_net_cap_is_clean() {
    let src = "module top;\n\
         genvar i;\n\
         generate\n\
         for (i = 0; i < 64; i = i + 1) begin: a\n\
           wire w;\n\
         end\n\
         endgenerate\n\
         initial #1 $finish;\n\
         endmodule\n";
    let (_out, err, code) = run(src);
    assert_eq!(code, Some(0), "small design must run clean. stderr:\n{err}");
    assert!(
        !err.contains("v1 cap"),
        "cap must not false-trip on 64 nets; stderr:\n{err}"
    );
}
