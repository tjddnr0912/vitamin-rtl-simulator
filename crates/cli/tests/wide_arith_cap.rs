//! WIDE-ARITH-CAP (2026-06-22 §5.1 #5): multi-word `*`/`/`/`%`/`**` have no
//! per-eval resource cap. A declaration-legal net (≤ MAX_NET_WIDTH = 2^20) fed
//! through a replication concat (`{2{a}}`) reaches > 2^20 bits, and the O(n²)
//! multiply / O(bits·n) division then stall for tens-to-hundreds of seconds with
//! NO diagnostic. Above the width cap the result must poison to X (the
//! div-by-zero degrade precedent) and the run must warn loud — both at once.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_widearith_{}_{n}", std::process::id()));
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

/// A multiply whose operand width exceeds the cap (524289 × 2 = 1,048,578 bits
/// > 2^20) poisons to X and warns loud — instead of stalling on the O(n²) kernel.
#[test]
fn over_cap_multiply_poisons_to_x_and_warns() {
    let src = "module top;\n\
         wire [524288:0] a;\n\
         assign a = 5;\n\
         wire [7:0] lo;\n\
         assign lo = {2{a}} * {2{a}};\n\
         initial begin #1 $display(\"lo=%h\", lo); $finish; end\n\
         endmodule\n";
    let (out, err, code) = run(src);
    assert_eq!(
        code,
        Some(0),
        "a width-capped arith warns (not fatal). stderr:\n{err}"
    );
    // The low byte must be X (poisoned), not the pre-cap product 0x19.
    let lo = out
        .lines()
        .find_map(|l| l.strip_prefix("lo="))
        .unwrap_or_else(|| panic!("no lo= line; out:\n{out}"));
    assert!(
        lo.contains('x') || lo.contains('X'),
        "over-cap multiply must poison to X; got lo={lo}"
    );
    let low = format!("{out}{err}").to_lowercase();
    assert!(
        low.contains("wide") || low.contains("cap") || low.contains("exceed"),
        "over-cap arith must warn loud; out:\n{out}\nerr:\n{err}"
    );
}

/// A normal-width arithmetic stays bit-exact and silent — the cap never trips.
#[test]
fn normal_width_arith_stays_exact_and_quiet() {
    let src = "module top;\n\
         wire [31:0] x, y;\n\
         assign x = 6; assign y = 7;\n\
         wire [63:0] p;\n\
         assign p = x * y;\n\
         initial begin #1 $display(\"p=%0d\", p); $finish; end\n\
         endmodule\n";
    let (out, err, code) = run(src);
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("p=42"),
        "32-bit multiply must be exact (42); out:\n{out}"
    );
    let low = format!("{out}{err}").to_lowercase();
    assert!(
        !low.contains("wide") && !low.contains("arith"),
        "the cap must not warn on a 32-bit multiply; err:\n{err}"
    );
}
