//! A3-iii-b ABSOLUTE ANCHOR — a DELEGATED function body writing a CLASS FIELD,
//! on tier-3, iverilog-pinned.
//!
//! A3-iii narrowed its gate row from "names a net outside its own frame" to
//! "WRITES one", and gave the reason: every destination in a delegated body goes
//! through `SimState::frame_write_lvalue`, which is `&self` on that state and
//! cannot reach a caller's arena. That is true of a FLAT destination — the
//! function `debug_assert`s its target is frame-local, so the engine cannot
//! perform one either — and it was measured to be most of the population.
//!
//! It was not true of a CLASS FIELD, which is where this slice lives. That write
//! lands in `SimState::class_heap`, keyed by net id and borrowed by BOTH kernels,
//! so it needs no routing at all. Measured: of the 23 out-of-window write sites
//! the suite reaches, 21 are class fields and 4 are flat module nets.
//!
//! ⚠️⚠️ AND NARROWING THE ROW ALONE WAS A SILENT-WRONG. The heap STORE needs no
//! routing, but the object id it is keyed by lives in a NET — and
//! `frame_or_class_write` read that handle from `SimState`, which a native run
//! leaves at t0. So the design ran, printed a plausible warning, and left every
//! field at 0. The differential below is what caught it, on the first probe; the
//! fix is the handle read taking the caller's store, the same one-line shape
//! A2-i found on the read side and A2-ii found on CRV's receiver.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Every line is a discriminator:
///
///   * `c.v = loc` writes a class field from a body the walk DELEGATES (a
///     function reached through `Expr::Call`), which is the shape the row
///     refused;
///   * `c.w = c.v + 1` READS BACK the field it just wrote, in the same body — so
///     a write that silently went nowhere cannot be hidden by the next
///     statement, and a deferred write would be visible here too;
///   * `bump` is called TWICE with different arguments, so a field left at its
///     initial value cannot pass by coincidence;
///   * the return value rides the ordinary frame-local path, so the two
///     destinations of one body are pinned together.
///
/// ⚠️ ANTI-VACUITY: run.json must say the run was native — before this slice the
/// storage gate refused the design and it fell back to the VM, where every line
/// already passed.
#[test]
fn a_delegated_body_writing_a_class_field_matches_iverilog() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fcw_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        "module top;\n\
           class C;\n\
             int v;\n\
             int w;\n\
           endclass\n\
           C c;\n\
           integer r;\n\
           function automatic integer bump(input integer x);\n\
             integer loc;\n\
             begin\n\
               loc = x * 2;\n\
               c.v = loc;\n\
               c.w = c.v + 1;\n\
               bump = loc + 5;\n\
             end\n\
           endfunction\n\
           initial begin\n\
             c = new();\n\
             r = bump(3);\n\
             $display(\"r=%0d v=%0d w=%0d\", r, c.v, c.w);\n\
             r = bump(10);\n\
             $display(\"r=%0d v=%0d w=%0d\", r, c.v, c.w);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg("native")
        .arg("--obs-dir")
        .arg("obs")
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let txt = String::from_utf8_lossy(&out.stdout).into_owned();
    let rj = std::fs::read_to_string(d.join("obs").join("run.json")).unwrap_or_default();
    assert!(
        rj.contains("\"backend\": \"native\""),
        "the design did not run natively:\n{rj}\n{txt}"
    );
    let mut body = String::new();
    for l in txt.lines().filter(|l| !l.starts_with("simulation ended")) {
        body.push_str(l);
        body.push('\n');
    }
    assert_eq!(
        body,
        "r=11 v=6 w=7\n\
         r=25 v=20 w=21\n",
        "iverilog-pinned: the field write lands and the read-back sees it"
    );
    // …and no null-handle warning, which is what an unrouted handle read
    // produces instead of a wrong number.
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !err.contains("null/X class handle"),
        "the handle read must take the caller's store:\n{err}"
    );
}
