//! Shared by `always_comb_t0_after_settle.rs` and `always_comb_t0_splits.rs` (§4.5.584):
//! run one design on native, interp and vm, byte-identical, and the staged chain.
#![allow(dead_code)]
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A fresh scratch directory (removed first: process ids recur across test processes).
pub fn scratch() -> PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_s584_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// One run's observable result.
#[derive(Debug, PartialEq)]
pub struct Run {
    /// stdout, whole (the `$display` lines and `simulation ended …`).
    pub out: String,
    /// Each diagnostic as `<line:col> <CODE> <[in …] [at time …]>`, the report's identity
    /// without its fixed message text. Two lines are dropped: the default-timescale
    /// warning (W1017) and the unique-overlap info (I2021). Both are once per parse,
    /// printed by the parse stage, so the staged chain prints them at `vcmp` and its
    /// `vrun` never does; keeping them would break `staged_matches`. I2021 is pinned in
    /// `unique_overlap_note.rs`.
    pub diags: Vec<String>,
    pub code: i32,
}

pub fn diags(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|l| {
            l.contains("[VITA-")
                && !l.contains("W-PP-TIMESCALE-DEFAULT")
                && !l.contains("I-PARSE-UNIQUE-OVERLAP-UNCHECKED")
        })
        .map(|l| {
            let code = l
                .split("[VITA-")
                .nth(1)
                .and_then(|r| r.split(']').next())
                .unwrap_or("?");
            let loc = l.split(": ").next().unwrap_or("");
            let loc = loc.strip_prefix("t.sv:").unwrap_or(loc);
            let tail = l.find(" [in ").map(|i| &l[i + 1..]).unwrap_or("");
            format!("{loc} {code} {tail}")
        })
        .collect()
}

pub fn vita_in(dir: &Path, args: &[&str]) -> Run {
    let o = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run vita");
    Run {
        out: String::from_utf8_lossy(&o.stdout).into_owned(),
        diags: diags(&String::from_utf8_lossy(&o.stderr)),
        code: o.status.code().unwrap_or(-1),
    }
}

/// The design on native, interp and vm, each in its own directory; asserts the three
/// byte-identical, and the `d.vcd` a design dumps too (`None` when it dumps nothing).
pub fn sim_vcd(src: &str) -> (Run, Option<Vec<u8>>) {
    let mut runs: Vec<(Run, Option<Vec<u8>>)> = Vec::new();
    for be in ["native", "interp", "vm"] {
        let dir = scratch();
        std::fs::write(dir.join("t.sv"), src).unwrap();
        let r = vita_in(&dir, &["--backend", be, "t.sv"]);
        runs.push((r, std::fs::read(dir.join("d.vcd")).ok()));
        let _ = std::fs::remove_dir_all(&dir);
    }
    assert_eq!(runs[0], runs[1], "native vs interp, design:\n{src}");
    assert_eq!(runs[0], runs[2], "native vs vm, design:\n{src}");
    runs.swap_remove(0)
}

/// `sim_vcd` for a design that dumps nothing.
pub fn sim(src: &str) -> Run {
    let (r, vcd) = sim_vcd(src);
    assert!(vcd.is_none(), "unexpected d.vcd:\n{src}");
    r
}

/// The staged chain on the same design; asserts it matches the one-shot run.
pub fn staged_matches(src: &str, oneshot: &Run) {
    let dir = scratch();
    std::fs::write(dir.join("t.sv"), src).unwrap();
    let c = vita_in(&dir, &["vcmp", "-o", "t.vu", "t.sv"]);
    assert_eq!(c.code, 0, "vcmp: {c:?}");
    let e = vita_in(&dir, &["velab", "-o", "t.velab", "t.vu"]);
    assert_eq!(e.code, 0, "velab: {e:?}");
    let r = vita_in(&dir, &["vrun", "-o", "s.vcd", "t.velab"]);
    assert_eq!(
        &r, oneshot,
        "staged vrun differs from the one-shot run:\n{src}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

pub fn ok(out: &str) -> String {
    format!("{out}simulation ended (Finish) at time ")
}

/// `out` must be `lines` followed by `simulation ended (Finish) at time <t>`.
pub fn assert_out(r: &Run, lines: &str, t: u32) {
    assert_eq!(r.out, format!("{}{t}\n", ok(lines)), "{r:?}");
}
