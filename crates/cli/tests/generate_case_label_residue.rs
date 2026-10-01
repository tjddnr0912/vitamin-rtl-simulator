//! REFUSED-TO-FIX, kept at PRE: a generate-case label compared in the i64 domain (ROADMAP
//! §2 🆕 T, held behind two prerequisites — one current binding per key, and
//! generate-case label resolution that is the same in every elaboration phase). The
//! 4-state compare at full width was built and reverted in review (round 2: a stale
//! same-key narrow binding beside a current wide one, and a forward label resolving to
//! a different object in the Nets phase than in later phases). Each pin asserts the
//! value vita prints today with both oracles' text beside it, so the admission is a
//! value change plus a text move.
//!
//! Oracles: iverilog 13.0 (`-g2012`, `vvp -n`), sv2v 0.0.13 → iverilog 13.0, verilator
//! 5.052 (`--binary --timing`; it refuses x/z/? generate-case labels).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_gencase_res_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// One generate-case per `(id, scrutinee, label)`: `<id> item` or `<id> default`.
fn cases(decls: &str, cells: &[(&str, &str, &str)]) -> String {
    let mut s = format!("`timescale 1ns/1ns\nmodule t;\n{decls}\n");
    for (id, scrut, lab) in cells {
        s += &format!(
            "  case ({scrut})\n    {lab}: begin : i_{id} initial $display(\"{id} item\"); end\n    \
             default: begin : d_{id} initial $display(\"{id} default\"); end\n  endcase\n"
        );
    }
    s + "endmodule\n"
}

/// - T1, a 65-bit parameter label (`LPA = {64'd0, P1}`, value 1) against 1: vita
///   `default`; iverilog, sv2v → iverilog and verilator `item`.
/// - T2, `$isunknown(4'bx100 ==? 4'b1?00)` against 1: vita `default`; iverilog and
///   sv2v → iverilog `item`.
/// - S06, `-1` against `32'hFFFFFFFF` (§12.5: unsigned at 32 bits): vita `default`;
///   all three `item`.
/// - X13, `4'b1100 inside {4'b1?00}` against 1: `item` in vita since the i64 constant
///   `==?` (§2 🆕 S, its i64 half) and in sv2v → iverilog (iverilog's own `==?` twin 1;
///   PRE `default`).
#[test]
fn generate_case_labels_are_compared_in_the_i64_domain() {
    let (out, err, code) = run(&cases(
        "  localparam P1 = (4'b1100 ==? 4'b1?00);\n  localparam [64:0] LPA = {64'd0, P1};",
        &[
            ("T1", "1", "LPA"),
            ("T2", "1", "$isunknown(4'bx100 ==? 4'b1?00)"),
            ("S06", "-1", "32'hFFFFFFFF"),
            ("X13", "1", "4'b1100 inside {4'b1?00}"),
        ],
    ));
    assert_eq!(code, 0, "{err}");
    let mut got: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .collect();
    got.sort_unstable();
    assert_eq!(
        got,
        ["S06 default", "T1 default", "T2 default", "X13 item"],
        "{out}"
    );
}
