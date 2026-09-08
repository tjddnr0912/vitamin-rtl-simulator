//! An array-WORD read whose index is a NET that one constant continuous assign
//! drives: `wire [1:0] k; assign k = 2'd1; assign c = m[k];` read in the same
//! delta as the writer's own write of `m[1]`. ROADMAP §2 🆕 I ⓒ residue.
//!
//! `alias::word_const` folded literal index trees only, so a `Signal` index
//! declined, the copy was refused and the read stayed at the settle's stale value
//! — `22` (or `xx` uninitialised) where iverilog 13.0 and verilator 5.052 both
//! read the word through and print `a5`. `alias::const_driven_nets` now settles
//! which nets hold a compile-time value, and such a net is a leaf of the fold
//! like a literal of its own width and sign.
//!
//! What is NOT admitted is measured too, and each cell has a reason: a
//! PROCEDURAL index, a DELAYED constant driver and a `buf`-driven one are the
//! wider "a computed continuous driver settles one delta late" class
//! (`assign c = r + 8'd0;` with no array reproduces it — ROADMAP §2 🆕 I ⓐ), a
//! `force` target is not a constant, and a multi-driver net is a resolution.
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.052 unless the
//! comment names a split; the lines are the tools' output, copied.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_backend(src: &str, backend: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cwnci_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg(backend)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// `decl` declares the index net; the design writes `m[1]` and reads the copy in
/// the same delta, then again one tick later.
fn cell(decl: &str, idx: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule top;\n  reg [7:0] m[0:1];\n  {decl}\n  \
         wire [7:0] c; assign c = m[{idx}];\n  \
         initial begin m[0] = 8'h5A; m[1] = 8'h22; #1 m[1] = 8'hA5;\n    \
         $display(\"D=%h\", c); #1 $display(\"L=%h\", c); #5 $finish; end\nendmodule\n"
    )
}

fn prints_all(src: &str, want: &[&str]) {
    prints_all_rc(src, want, 0)
}

/// The same, for a design whose read is out of range: the E4002 is an ERROR, so
/// the run ends at exit 1 and still prints every line.
fn prints_all_rc(src: &str, want: &[&str], rc: i32) {
    for b in ["native", "interp", "vm"] {
        let (out, code) = run_backend(src, b);
        assert_eq!(code, Some(rc), "[{b}] exit\n{out}");
        let got: Vec<&str> = out
            .lines()
            .filter(|l| l.starts_with("D=") || l.starts_with("L="))
            .collect();
        assert_eq!(got, want, "[{b}]\n{out}");
    }
}

#[test]
fn a_net_with_one_constant_continuous_driver_indexes_like_the_literal() {
    // both oracles `D=a5 L=a5`; PRE `D=22`. The literal control twin is the same
    // two lines in all three tools and is what this must equal.
    prints_all(
        &cell("wire [1:0] k; assign k = 2'd1;", "k"),
        &["D=a5", "L=a5"],
    );
    prints_all(&cell("", "1"), &["D=a5", "L=a5"]);
    // an index net WIDER than the array's coordinate
    prints_all(
        &cell("wire [7:0] k; assign k = 8'd1;", "k"),
        &["D=a5", "L=a5"],
    );
    // a constant EXPRESSION, and a named constant
    prints_all(
        &cell("wire [1:0] k; assign k = 2'd1 + 2'd0;", "k"),
        &["D=a5", "L=a5"],
    );
    prints_all(
        &cell("parameter [1:0] P = 2'd1; wire [1:0] k; assign k = P;", "k"),
        &["D=a5", "L=a5"],
    );
    // TRANSITIVE: one more name in front of the same constant. Both oracles read
    // it through, and splitting the two spellings would give one read two answers.
    prints_all(
        &cell(
            "wire [1:0] k2; assign k2 = 2'd1; wire [1:0] k; assign k = k2;",
            "k",
        ),
        &["D=a5", "L=a5"],
    );
    // an all-`z` driver beside the constant one is not a second driver (`z` is the
    // identity of every resolution kind — the same account `null_driver` gives
    // `copy_nets`); both oracles `a5`.
    prints_all(
        &cell("wire [1:0] k; assign k = 2'd1; assign k = 2'bzz;", "k"),
        &["D=a5", "L=a5"],
    );
}

#[test]
fn the_select_spelling_of_the_same_word_renames_too() {
    // `assign c = m[k][7:0]` is `assign c = m[k]` under another spelling — the two
    // arms of the fold are opened together or one read has two answers. Both
    // oracles `a5` (PRE `22`).
    let src = "`timescale 1ns/1ns\nmodule top;\n  reg [7:0] m[0:1];\n  \
               wire [1:0] k; assign k = 2'd1;\n  wire [7:0] c; assign c = m[k][7:0];\n  \
               initial begin m[0] = 8'h5A; m[1] = 8'h22; #1 m[1] = 8'hA5;\n    \
               $display(\"D=%h\", c); #5 $finish; end\nendmodule\n";
    prints_all(src, &["D=a5"]);
}

#[test]
fn what_is_not_a_constant_driver_stays_computed() {
    // ⚠️ These four are DELIBERATE, and each is a divergence from both oracles that
    // this slice does not close: they are the wider computed-driver class
    // (ROADMAP §2 🆕 I ⓐ), whose fix is the store-side forward that reordered
    // picorv32 / UDP / keccak. Pinned so a later widening of `const_driven_nets`
    // has to answer for them rather than reach them by accident.
    //
    // a PROCEDURAL index: the value changes mid-run, so the driver's rhs is only
    // part of the account
    prints_all(
        &cell("reg [1:0] k; initial k = 2'd1;", "k"),
        &["D=22", "L=a5"],
    );
    // a DELAYED constant driver has its own inertial register
    prints_all(
        &cell("wire [1:0] k; assign #1 k = 2'd1;", "k"),
        &["D=22", "L=a5"],
    );
    // two whole-net drivers are a resolution, not one constant
    prints_all(
        &cell("wire [1:0] k; assign k = 2'd1; assign k = 2'd1;", "k"),
        &["D=22", "L=a5"],
    );
    // a `buf` is the §7.3 `z`→`x` coercion and COMPUTES — `oracle_split_rulings`
    // pins that ruling, and its `~~in` desugar declines here for the same reason
    // any operator does
    prints_all(
        &cell("wire [1:0] k; buf b0(k[0], 1'b1); buf b1(k[1], 1'b0);", "k"),
        &["D=22", "L=a5"],
    );
}

#[test]
fn a_forced_index_is_not_a_constant_and_still_redirects_the_word() {
    // The `force` must keep CHOOSING the word: all three tools read `m[0]` = `5a`
    // while `k` is held at 0, and `m[1]` = `a5` again after the `release`. The net
    // is excluded from `const_driven_nets` for exactly that reason, so this design
    // keeps byte-identically the behaviour it had before the slice.
    // (iverilog's `T1` is `a5` — the same same-delta read the class above owns.)
    let src = "`timescale 1ns/1ns\nmodule top;\n  reg [7:0] m[0:1];\n  \
               wire [1:0] k; assign k = 2'd1;\n  wire [7:0] c; assign c = m[k];\n  \
               initial begin m[0] = 8'h5A; m[1] = 8'h22; #1 m[1] = 8'hA5;\n    \
               $display(\"T1=%h\", c);\n    \
               #1 force k = 2'd0; $display(\"T2=%h\", c); #1 $display(\"T3=%h\", c);\n    \
               #1 release k; #1 $display(\"T4=%h\", c); #5 $finish; end\nendmodule\n";
    for b in ["native", "interp", "vm"] {
        let (out, code) = run_backend(src, b);
        assert_eq!(code, Some(0), "[{b}] exit\n{out}");
        let got: Vec<&str> = out.lines().filter(|l| l.contains("T")).collect();
        assert_eq!(
            got,
            ["T1=22", "T2=a5", "T3=5a", "T4=a5"],
            "[{b}] the force must still pick the word\n{out}"
        );
    }
}

#[test]
fn an_out_of_range_or_unknown_constant_index_stays_where_it_was() {
    // `2'bx1` — an x/z constant is not a constant here (iverilog `xx`; verilator
    // truncates the index to one bit and its own control twin is an internal
    // error, so it gets no vote).
    prints_all(
        &cell("wire [1:0] k; assign k = 2'bx1;", "k"),
        &["D=xx", "L=xx"],
    );
    // Out of range on `m[0:1]` — refused by the range test the literal index
    // already faces, so the read stays computed and its E4002 STREAM does not
    // grow. The two spellings do not emit the same NUMBER (4 against the
    // literal's 3): that gap is the extra driver's own settle visit and is
    // PRE-identical, measured on a `git archive HEAD` binary. What the copy-set
    // must never do is add one — `native/dirty.rs` records the picorv32 6 → 9
    // regression from re-visiting an out-of-range read on every repair.
    for b in ["native", "interp", "vm"] {
        let (out, _) = run_backend(&cell("wire [1:0] k; assign k = 2'd3;", "k"), b);
        let (lit, _) = run_backend(&cell("", "3"), b);
        assert_eq!(
            (
                out.matches("VITA-E4002").count(),
                lit.matches("VITA-E4002").count()
            ),
            (4, 3),
            "[{b}] the out-of-range diagnostic stream moved\n{out}\n--\n{lit}"
        );
    }
    // a NEGATIVE constant index is out of range on `m[0:1]`: iverilog `xx`,
    // verilator reads `m[1]` (a live split — vita is on iverilog's side, PRE and
    // POST alike).
    prints_all_rc(
        &cell("wire signed [3:0] k; assign k = -4'sd1;", "k"),
        &["D=xx", "L=xx"],
        1,
    );
}

#[test]
fn the_time_zero_event_matches_the_literal_spelling() {
    // A copy net carries no event its source chain did not have, and that has to
    // be true of BOTH spellings of the same read. PRE woke `always @(c)` at time 0
    // for the wire-index spelling and not for the literal one; iverilog wakes
    // neither. (verilator wakes both at 0 and gets no vote — it has no `z`.)
    let src = "`timescale 1ns/1ns\nmodule top;\n  reg [7:0] m[0:1];\n  \
               wire [1:0] k; assign k = 2'd1;\n  \
               wire [7:0] c; assign c = m[k];\n  wire [7:0] cl; assign cl = m[1];\n  \
               always @(c) $display(\"EVT_C at %0t\", $time);\n  \
               always @(cl) $display(\"EVT_L at %0t\", $time);\n  \
               initial begin #1 m[1] = 8'hA5; #5 $display(\"END\"); $finish; end\nendmodule\n";
    for b in ["native", "interp", "vm"] {
        let (out, code) = run_backend(src, b);
        assert_eq!(code, Some(0), "[{b}] exit\n{out}");
        let c: Vec<&str> = out.lines().filter(|l| l.starts_with("EVT_C")).collect();
        let l: Vec<&str> = out.lines().filter(|l| l.starts_with("EVT_L")).collect();
        assert_eq!(
            c,
            ["EVT_C at 1"],
            "[{b}] no time-zero event on the copy\n{out}"
        );
        assert_eq!(
            l,
            ["EVT_L at 1"],
            "[{b}] the literal twin is the control\n{out}"
        );
    }
}
