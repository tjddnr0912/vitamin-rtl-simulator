//! A5-dumpall ABSOLUTE ANCHOR — `$dumpoff` / `$dumpon` / `$dumpall` on tier-3.
//!
//! These two tasks were the LAST members of `systask_refusal`, and their row was
//! right and one function wide: "they re-snapshot through `full_snapshot`, and
//! only the `$dumpvars` call site threads the arena reader so far". Everything
//! else about them — `dumping`, the VCD id tables, the writer — is `SimState`'s
//! and shared, which is why threading the other two call sites was the whole
//! slice. The set is now EMPTY (see `native::kernel::systask_refusal`).
//!
//! ⚠️ The failure mode an unthreaded snapshot has is not a crash: it re-reads
//! the ENGINE's store, which a native run leaves at t0, so `$dumpon` and
//! `$dumpall` would write a snapshot of the DECLARATION INITIALISERS into a
//! waveform whose other records are right — at exit 0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Every record is a discriminator:
///
///   * `$dumpoff` writes the all-X snapshot (iverilog-checked), which proves the
///     off path still runs;
///   * `$dumpon` writes a FULL snapshot of the values as of that instant — and
///     `a` was assigned WHILE dumping was off, so the value it carries exists
///     nowhere else in the file. An unthreaded snapshot writes the t0 value here;
///   * `$dumpall` does the same at a later time with a different value, so a
///     single stale snapshot cannot satisfy both;
///   * `b` never changes after t0, so it appears ONLY in the three snapshots —
///     which is what shows a snapshot is a whole-design walk rather than a replay
///     of the change stream.
///
/// ⚠️ ANTI-VACUITY: run.json must say the run was native. Before this slice the
/// design was refused by the EXECUTOR layer and fell back to the VM.
///
/// ⚠️ The pin is on vita's own VCD text, not on iverilog's bytes: iverilog
/// strips leading zeros from vector values (`b100010` where vita writes
/// `b00100010`), emits an extra parameter block before `#0`, and closes with a
/// bare `#6` time marker vita does not write. All three are pre-existing
/// formatting differences, not this slice's. The SEMANTICS
/// were checked against it — off ⇒ all-X, on/all ⇒ the current values — and the
/// two backends are byte-identical to each other.
#[test]
fn dump_on_and_dump_all_snapshot_the_running_store_on_tier_3() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dmp_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        "module top;\n\
           reg [7:0] a = 0;\n\
           reg [3:0] b = 4'h5;\n\
           initial begin\n\
             $dumpfile(\"w.vcd\");\n\
             $dumpvars(0, top);\n\
             #1 a = 8'h11; b = 4'h6;\n\
             #1 $dumpoff;\n\
             a = 8'h22;\n\
             #1 $dumpon;\n\
             #1 a = 8'h33;\n\
             #1 $dumpall;\n\
             #1 $finish;\n\
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
    let rj = std::fs::read_to_string(d.join("obs").join("run.json")).unwrap_or_default();
    assert!(
        rj.contains("\"backend\": \"native\""),
        "the design did not run natively:\n{rj}\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let vcd = std::fs::read_to_string(d.join("w.vcd")).expect("w.vcd");
    let body = vcd
        .split_once("$enddefinitions $end\n")
        .expect("a definitions section")
        .1
        .to_string();
    assert_eq!(
        body,
        "$dumpvars\n\
         b00000000 !\n\
         b0101 \"\n\
         $end\n\
         #0\n\
         #1\n\
         b00010001 !\n\
         b0110 \"\n\
         #2\n\
         $dumpoff\n\
         bxxxxxxxx !\n\
         bxxxx \"\n\
         $end\n\
         #3\n\
         $dumpon\n\
         b00100010 !\n\
         b0110 \"\n\
         $end\n\
         #4\n\
         b00110011 !\n\
         #5\n\
         $dumpall\n\
         b00110011 !\n\
         b0110 \"\n\
         $end\n",
        "the snapshots must carry the RUNNING values, not the engine store's t0"
    );
}
