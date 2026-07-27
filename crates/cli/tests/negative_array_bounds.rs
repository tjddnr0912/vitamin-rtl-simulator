//! Unpacked arrays declared with a NEGATIVE low bound (`int a[-1:1]`, IEEE §7.4.2).
//!
//! Regression for a silent-wrong: the per-dim extent clamped `lo` to 0, which shrank the
//! dim itself — `[-1:1]` became `(lo 0, size 2)`. The array lost a word, `foreach` ran
//! two iterations instead of three, and nothing was reported (exit 0). Touching the
//! negative index explicitly did produce an E4002, which is why this was filed as a loud
//! gap; the pure-`foreach` form is what showed it was silent.
//!
//! `lo` is `i64` throughout now (`array_dims`, `flatten_word`, the `net_dims` sidecar) and
//! the declared index space is spoken end to end: access, `foreach`, `$size`/`$left`/…,
//! per-element VCD names.
//!
//! ORACLE: iverilog 13.0 on every design here except the VCD names (iverilog does not
//! dump unpacked arrays), which are pinned to the declared index space by hand.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn dir_for(tag: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_nab_{}_{tag}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_in(d: &std::path::Path, src: &str) -> String {
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn run(src: &str) -> String {
    run_in(&dir_for("r"), src)
}

/// stdout lines only — drops the `timescale` warning and the run trailer.
fn lines(o: &str) -> Vec<String> {
    o.lines()
        .filter(|l| {
            !l.contains("VITA-W1017")
                && !l.starts_with("errors=")
                && !l.starts_with("simulation ended")
        })
        .map(str::to_string)
        .collect()
}

// ── the repro: `foreach` alone, no explicit negative index ──
#[test]
fn negative_low_bound_foreach_visits_every_element() {
    let o = run("module t;\n\
        int a[-1:1]; int n;\n\
        initial begin n = 0; foreach (a[i]) n = n + 1; $display(\"count=%0d\", n); end\n\
        endmodule\n");
    assert_eq!(lines(&o), vec!["count=3"], "{o}");
}

#[test]
fn negative_low_bound_read_write_and_foreach() {
    let o = run("module t;\n\
        int a[-1:1];\n\
        initial begin a[-1]=7; a[0]=8; a[1]=9;\n\
          foreach (a[i]) $display(\"%0d:%0d\", i, a[i]); end\n\
        endmodule\n");
    assert_eq!(lines(&o), vec!["-1:7", "0:8", "1:9"], "{o}");
}

// ── descending declaration whose low bound is negative: the walk is 2 → -2 ──
#[test]
fn negative_low_bound_descending_walks_left_to_right() {
    let o = run("module t;\n\
        int a[2:-2];\n\
        initial begin foreach (a[i]) a[i] = i;\n\
          foreach (a[i]) $display(\"%0d=%0d\", i, a[i]);\n\
          $display(\"size=%0d left=%0d right=%0d low=%0d high=%0d\",\n\
                   $size(a), $left(a), $right(a), $low(a), $high(a)); end\n\
        endmodule\n");
    assert_eq!(
        lines(&o),
        vec![
            "2=2",
            "1=1",
            "0=0",
            "-1=-1",
            "-2=-2",
            "size=5 left=2 right=-2 low=-2 high=2"
        ],
        "{o}"
    );
}

// ── both dims negative: exercises the multi-dim bounds guard on the coordinate ──
#[test]
fn negative_low_bound_multi_dim() {
    let o = run("module t;\n\
        int a[-2:0][-1:1];\n\
        initial begin foreach (a[i,j]) a[i][j] = i*10+j;\n\
          foreach (a[i,j]) $display(\"%0d %0d = %0d\", i, j, a[i][j]); end\n\
        endmodule\n");
    assert_eq!(
        lines(&o),
        vec![
            "-2 -1 = -21",
            "-2 0 = -20",
            "-2 1 = -19",
            "-1 -1 = -11",
            "-1 0 = -10",
            "-1 1 = -9",
            "0 -1 = -1",
            "0 0 = 0",
            "0 1 = 1"
        ],
        "{o}"
    );
}

// ── a RUNTIME index in the negative domain resolves; a genuine OOB is still loud ──
#[test]
fn negative_low_bound_runtime_index_and_oob_stays_loud() {
    let o = run("module t;\n\
        int a[-1:1]; int k;\n\
        initial begin\n\
          for (k=-1; k<=1; k=k+1) a[k] = k*100;\n\
          for (k=-1; k<=1; k=k+1) $display(\"%0d=%0d\", k, a[k]);\n\
          $display(\"oob=%0d\", a[2]); end\n\
        endmodule\n");
    assert!(
        o.contains("-1=-100") && o.contains("0=0") && o.contains("1=100"),
        "declared domain:\n{o}"
    );
    // iverilog warns and reads x for `a[2]`; vita stays loud with an X read.
    assert!(o.contains("E4002") && o.contains("oob=x"), "oob:\n{o}");
}

// ── the routed string-array form inherits the same map (its decline is lifted) ──
#[test]
fn negative_low_bound_string_array_runtime_index() {
    let o = run("module t;\n\
        string s[-1:1]; int k;\n\
        initial begin k=-1; s[k]=\"aa\"; k=1; s[k]=\"cc\";\n\
          foreach (s[i]) $display(\"%0d=[%s]\", i, s[i]); end\n\
        endmodule\n");
    assert_eq!(lines(&o), vec!["-1=[aa]", "0=[]", "1=[cc]"], "{o}");
}

// ── pre-existing, found while making `array_dims` signed: the map stores (lo, SIZE)
// but `$bits` of a partially-indexed view read the second field as an upper bound.
#[test]
fn bits_of_a_partially_indexed_array_view() {
    let o = run("module t;\n\
        int a[2][3];\n\
        initial $display(\"a0=%0d a=%0d\", $bits(a[0]), $bits(a));\n\
        endmodule\n");
    assert_eq!(lines(&o), vec!["a0=96 a=192"], "{o}"); // iverilog: 96 / 192
}

// ── the same (lo, SIZE) misread in the engine's file-I/O base: `$readmem` addresses
// live in the DECLARED domain, and `lo.min(size)` put `reg m[10:11]`'s base at 2. With
// no explicit address the base cancels out, so only the ranged form ever showed it.
#[test]
fn readmem_with_an_explicit_range_on_a_non_zero_base_memory() {
    let d = dir_for("rdm");
    std::fs::write(d.join("m.hex"), "AA\nBB\n").unwrap();
    let o = run_in(
        &d,
        "module t;\n\
         reg [7:0] m[10:11];\n\
         initial begin $readmemh(\"m.hex\", m, 10, 11);\n\
           $display(\"%h %h\", m[10], m[11]); end\n\
         endmodule\n",
    );
    assert_eq!(lines(&o), vec!["aa bb"], "{o}"); // iverilog: aa bb (was: warn + xx xx)
}

// ── per-element VCD names use the DECLARED index, so the middle element is `a[0]`
// and the first is `a[-1]` — not a renumbered `a[0] a[1] a[2]`.
#[test]
fn negative_low_bound_vcd_element_names() {
    let d = dir_for("vcd");
    let o = run_in(
        &d,
        "module t;\n\
         int a[-1:1];\n\
         initial begin $dumpfile(\"w.vcd\"); $dumpvars(0, t);\n\
           a[-1]=7; a[0]=8; a[1]=9; #1 $finish; end\n\
         endmodule\n",
    );
    let vcd = std::fs::read_to_string(d.join("w.vcd")).unwrap_or_default();
    let names: Vec<&str> = vcd.lines().filter(|l| l.starts_with("$var")).collect();
    assert_eq!(names.len(), 3, "element count (run:\n{o}\nvcd:\n{vcd})");
    for want in ["a[-1]", "a[0]", "a[1]"] {
        assert!(
            names.iter().any(|l| l.contains(want)),
            "missing `{want}` in {names:?}"
        );
    }
}
