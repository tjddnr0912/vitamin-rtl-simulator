//! G2 FST waveform breadth — end-to-end: `vita design.sv` with a `.fst`
//! `$dumpfile` produces a real FST whose read-back waveform (via the independent
//! `fst-reader` crate) matches the design's simulated value changes. The pinned
//! expectations are the live iverilog-13.0 waveform for the same design (a `.vcd`
//! run of it prints `b0/1100/10xz` for `v` and `0/1/0` for `a`), so this is a
//! true differential pin, not a self-consistency check.
//!
//! FST is produced by transcoding vita's verified VCD (see
//! `vcd-writer/src/fst.rs`), so a match here confirms the whole path:
//! elaborate → simulate → VCD → FST → read-back.

use fst_reader::{FstFilter, FstHierarchyEntry, FstReader, FstSignalValue};
use std::collections::HashMap;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `vita <file>` in a fresh temp dir; return (stdout, stderr, exit-code) and
/// the dir so the caller can read the produced waveform files.
fn run_vita(src: &str, ext: &str) -> (String, i32, std::path::PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fst_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join(format!("t.{ext}"));
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
        d,
    )
}

/// Read an FST back into sorted (time, var-name, value-string) tuples.
fn read_fst(path: &std::path::Path) -> Vec<(u64, String, String)> {
    let f = std::io::BufReader::new(std::fs::File::open(path).expect("open fst"));
    let mut r = FstReader::open(f).expect("parse fst");
    let mut idx2name: HashMap<usize, String> = HashMap::new();
    r.read_hierarchy(|e| {
        if let FstHierarchyEntry::Var { name, handle, .. } = e {
            idx2name.entry(handle.get_index()).or_insert(name);
        }
    })
    .unwrap();
    let mut out = Vec::new();
    r.read_signals(&FstFilter::all(), |t, h, v| {
        let s = match v {
            FstSignalValue::String(b) => String::from_utf8_lossy(b).into_owned(),
            FstSignalValue::Real(x) => format!("r{x}"),
        };
        out.push((t, idx2name[&h.get_index()].clone(), s));
    })
    .unwrap();
    out.sort();
    out
}

const DESIGN: &str = "\
`timescale 1ns/1ns
module top;
  reg a; reg [3:0] v;
  initial begin
    $dumpfile(\"OUT\");
    $dumpvars(0, top);
    a=0; v=0;
    #10 a=1; v=4'hc;
    #10 a=0; v=4'b10xz;
    #10 $finish;
  end
endmodule
";

/// `$dumpfile("w.fst")` yields a real FST matching the iverilog-pinned waveform.
#[test]
fn dumpfile_fst_matches_iverilog_waveform() {
    let (stderr, code, dir) = run_vita(&DESIGN.replace("OUT", "w.fst"), "sv");
    assert_eq!(code, 0, "vita failed: {stderr}");
    let fst = dir.join("w.fst");
    assert!(fst.exists(), "w.fst was not produced; stderr:\n{stderr}");
    // the sidecar VCD must be cleaned up on success
    assert!(
        !dir.join("w.fst.vcdtmp").exists(),
        "sidecar VCD left behind"
    );

    let got = read_fst(&fst);
    let expected: Vec<(u64, String, String)> = [
        (0u64, "a", "0"),
        (0, "v[3:0]", "0000"),
        (10, "a", "1"),
        (10, "v[3:0]", "1100"),
        (20, "a", "0"),
        (20, "v[3:0]", "10xz"),
    ]
    .into_iter()
    .map(|(t, n, s)| (t, n.to_string(), s.to_string()))
    .collect();
    let mut exp = expected;
    exp.sort();
    assert_eq!(got, exp, "FST waveform must match the iverilog-pinned VCD");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A non-`.fst` `$dumpfile` still produces a plain VCD (regression: the FST path
/// must not hijack ordinary dumps). The VCD is text starting with `$date`.
#[test]
fn dumpfile_vcd_still_vcd() {
    let (stderr, code, dir) = run_vita(&DESIGN.replace("OUT", "w.vcd"), "sv");
    assert_eq!(code, 0, "vita failed: {stderr}");
    let vcd = std::fs::read_to_string(dir.join("w.vcd")).expect("w.vcd");
    assert!(
        vcd.starts_with("$date"),
        "expected a VCD, got:\n{}",
        &vcd[..vcd.len().min(40)]
    );
    assert!(!dir.join("w.vcd.vcdtmp").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
