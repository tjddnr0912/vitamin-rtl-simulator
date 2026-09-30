//! §3.b `oob-read-exit` (§4.5.576): a KNOWN array word index past the end is the
//! access IEEE 1364-2005 §5.2.1 defines — read x, write ignored — and vita reports it
//! as `E-RUN-RANGE` (`VITA-E4002`) at WARNING severity, exit 0 (owner ruling: a
//! warning and the value x). It was an Error, so the corpus row `aes`
//! (`aes_key_mem.v:182`) printed the right digest and exited 1 where both oracles
//! exit 0 and say nothing.
//!
//! Values are iverilog 13.0's (`-g2012`), which prints every line below and exits 0;
//! it warns at COMPILE time for the constant `m[-1]` l-value only. verilator 5.050 gets
//! no vote: it has no x and reads out-of-range words as live data (`R 11`, `N 13`,
//! `C 77`), outside §5.2.1. The code keeps its number and mnemonic (doc-15: fixed by
//! meaning), so `-Werror=E-RUN-RANGE` restores exit 1 and `-Wno-E-RUN-RANGE` drops it.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str, args: &[&str]) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_oob_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.v");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .args(args)
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

const SRC: &str = "module t;\n\
  reg [7:0] m [0:3]; integer i, k; reg [1:0] x;\n\
  initial begin\n\
    for (k = 0; k < 4; k = k + 1) m[k] = k + 8'h10;\n\
    i = 9;  $display(\"R %h\", m[i]);\n\
    m[i] = 8'hEE; $display(\"W %h %h %h %h\", m[0], m[1], m[2], m[3]);\n\
    i = -1; $display(\"N %h\", m[i]);\n\
    m[-1] = 8'h77; $display(\"C %h\", m[3]);\n\
    x = 2'bx1; $display(\"X %h\", m[x]);\n\
    for (k = 4; k < 16; k = k + 1) m[k] = 1;\n\
    $display(\"done\");\n\
    $finish;\n\
  end\n\
endmodule\n";

/// The design's own lines, as iverilog 13.0 prints them.
const IVERILOG: [&str; 6] = ["R xx", "W 10 11 12 13", "N xx", "C 13", "X xx", "done"];

fn design_lines(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| !l.starts_with("simulation ended") && !l.starts_with("errors="))
        .collect()
}

#[test]
fn a_known_out_of_range_word_is_a_warning_on_every_backend() {
    for be in ["interp", "vm", "native"] {
        let (out, err, rc) = run(SRC, &["--backend", be]);
        assert_eq!(rc, Some(0), "[{be}] a warning, not an error:\n{err}");
        assert_eq!(design_lines(&out), IVERILOG, "[{be}] values:\n{out}");
        // Four known accesses (read, write, negative read, constant write) and
        // four from the loop fill the cap of eight reports; the ninth access
        // prints the suppression line and later ones nothing. The x index is the
        // separate W4029 budget.
        let e4002: Vec<&str> = err.lines().filter(|l| l.contains("[VITA-E4002]")).collect();
        assert_eq!(e4002.len(), 9, "[{be}]\n{err}");
        assert!(
            e4002.iter().all(|l| l.contains("warning[VITA-E4002]")),
            "[{be}]\n{err}"
        );
        assert!(
            e4002[8].contains("further out-of-range diagnostics suppressed"),
            "[{be}]\n{err}"
        );
        assert_eq!(
            err.matches("warning[VITA-W4029]").count(),
            1,
            "[{be}]\n{err}"
        );
        assert!(!err.contains("error["), "[{be}]\n{err}");
        assert!(
            err.contains("errors=0 ") || out.contains("errors=0 "),
            "[{be}]\n{out}{err}"
        );
    }
}

#[test]
fn werror_restores_exit_1_and_wno_silences_it() {
    for spelling in ["E-RUN-RANGE", "VITA-E4002", "E4002"] {
        let (out, err, rc) = run(SRC, &[&format!("-Werror={spelling}")]);
        assert_eq!(rc, Some(1), "-Werror={spelling}:\n{err}");
        assert!(
            err.contains("error[VITA-E4002]"),
            "-Werror={spelling}:\n{err}"
        );
        assert_eq!(design_lines(&out), IVERILOG, "-Werror={spelling}: values");
    }
    let (out, err, rc) = run(SRC, &["-Wno-E-RUN-RANGE"]);
    assert_eq!(rc, Some(0), "{err}");
    assert!(!err.contains("VITA-E4002"), "{err}");
    // The unknown-index twin is a different code and is not suppressed with it.
    assert!(err.contains("warning[VITA-W4029]"), "{err}");
    assert_eq!(design_lines(&out), IVERILOG);
}
