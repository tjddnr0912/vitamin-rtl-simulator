//! A heap mutation of a dynamic array / queue / associative array / string WAKES
//! the inferred-sensitivity blocks that read the handle.
//!
//! An `always_comb`/`always_latch`/`@*` block carries its elaborate-inferred read
//! set in `sensitivity.edges`, and a handle net IS in it: `always_comb n =
//! w.size()` arms `Level { nets: [t.w] }` (measured by dumping the armed
//! sensitivity). The wake then runs off the DIRTY sweep, whose only producer was
//! `SimState::note_change` — and no heap mutation reaches that, because a handle
//! net's WORD never moves. So `w = new[3]`, `w[0] = 10`, `q.push_back(1)`,
//! `s = "abc"` and every other heap write were invisible to the sweep and the
//! reader kept its stale value at exit 0.
//!
//! `SimState::note_dyn_change` is the funnel that closes it: every module-net heap
//! mutation stages `(net, author)` and the two schedulers drain it into their own
//! dirty channel (`SimState::mark_heap_dirty` for the engine, `drain_heap_marks`
//! for tier-3). It carries no VCD and no probe bytes, because a handle net has
//! neither channel — `$dumpvars` skips every DynArray/Queue/Assoc/AssocStr/String
//! net and `--probe` on one is a loud CLI refusal (E0001).
//!
//! ORACLES: iverilog 13.0 (`-g2012` + `vvp -n`) and verilator 5.052 (`--binary
//! --timing`). Every value below is a RAW oracle line, not a hand computation.
//!
//! ⚠️ THE ORACLES SPLIT on ONE class of mutation, and the split is recorded
//! rather than averaged. For an INDEXED ELEMENT STORE and for a QUEUE METHOD on an
//! already-live handle, verilator re-fires the reader and iverilog does not:
//!
//! ```text
//! ② `w[0] = 10` on an allocated array   iverilog A=6  B=6  C=115 D=0
//!                                        verilator A=6  B=15 C=115 D=0
//! ⑪ push_front / insert / delete(i) /   iverilog Q1=1 Q2=1 Q3=1 Q4=1 Q5=1 Q6=0
//!    pop_front on a live queue           verilator Q1=1 Q2=2 Q3=3 Q4=2 Q5=1 Q6=0
//! ⑬ a second `q.push_back` (`Y` row)    iverilog X=3 1 4 / Y=5 1 1 / Z=0 0 0
//!                                        verilator X=3 1 4 / Y=5 2 1 / Z=0 0 0
//! ```
//!
//! iverilog is DISQUALIFIED here by self-contradiction, not outvoted: in the very
//! same designs it re-fires the block for `w = new[3]`, for `w = new[4](w)`, for
//! `w.delete()` and for `q.delete()` — so `w`/`q` IS in its read set — while
//! leaving `w[0] = 10` and `q.push_front(8)` unobserved, and in ② it prints its
//! reason at compile time ("A for statement must have a constant initial value to
//! be synthesized in an always_comb process"). IEEE 1800 §9.2.2.2 makes `w` a
//! variable the block reads, and a write to `w[0]` or a push onto `q` changes that
//! variable. The pinned value is verilator's, and ⑫ (`int a[int]`, which iverilog
//! cannot elaborate) and ⑩ (`assign n = w.size();`, which aborts its code
//! generator) say where it is the only oracle at all. EVERY OTHER cell below is a
//! value BOTH oracles printed.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dhmw_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let so = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    (
        so,
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// ① `$size` of a dynamic array read by an `always_comb`, against a second input
/// so the two wake sources are told apart cell by cell.
///
/// PRE: `100 / 1003 / 1003 / 108` — every cell whose only mover is the ARRAY is
/// stale, and only the plain-net cells (`t`) are right. Both oracles: `103 / 1003
/// / 1008 / 108`.
#[test]
fn a_dyn_new_wakes_a_size_reader() {
    let (o, e, code) = run(r#"module a8b;
  int da[]; int n; logic t = 0;
  always_comb n = $size(da) + (t ? 1000 : 100);
  initial begin
    da = new[3]; #1 $display("N1=%0d", n);
    t = 1;       #1 $display("N2=%0d", n);
    da = new[8]; #1 $display("N3=%0d", n);
    t = 0;       #1 $display("N4=%0d", n);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "N1=103\nN2=1003\nN3=1008\nN4=108");
}

/// ② A `foreach` accumulator over the array, in an `always_comb` declared BEFORE
/// the `initial` that fills it. PRE: `0 / 0 / 0 / 0` — the block ran once at t0
/// over an empty array and never again.
///
/// Cell B is the oracle split recorded in this file's header — verilator `B=15`,
/// iverilog `B=6`, and verilator's is the pin; A / C / D are values BOTH printed.
#[test]
fn an_element_write_and_a_resize_and_a_delete_all_wake_a_foreach_sum() {
    let (o, e, code) = run(r#"module t; int w[]; int s;
  always_comb begin s = 0; foreach (w[i]) s = s + w[i]; end
  initial begin w = new[3]; w[0]=1; w[1]=2; w[2]=3; #1 $display("A=%0d", s);
    w[0] = 10; #1 $display("B=%0d", s); w = new[4](w); w[3]=100; #1 $display("C=%0d", s); w.delete(); #1 $display("D=%0d", s); $finish; end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "A=6\nB=15\nC=115\nD=0");
}

/// ③ THE THREE STORAGE KINDS AT ONCE — `.size()` of a dynamic array, `.size()` of
/// a queue, `.len()` of a string, each in its own `always_comb`. PRE: `A=0 0 0` /
/// `B=0 0 0`; both oracles: `A=0 0 0` / `B=3 1 3`.
///
/// The A row is the control that makes the B row mean something: before the
/// mutation every reader legitimately answers 0, so a fix that simply made the
/// blocks run more often could not produce this pair.
#[test]
fn dyn_and_queue_and_string_mutations_each_wake_their_own_reader() {
    let (o, e, code) = run(
        r#"module t; int w[]; int n; int q[$]; int m; string s; int l;
  always_comb n = w.size();
  always_comb m = q.size();
  always_comb l = s.len();
  initial begin #1 $display("A=%0d %0d %0d", n, m, l); w = new[3]; q.push_back(1); s = "abc"; #1 $display("B=%0d %0d %0d", n, m, l); $finish; end
endmodule
"#,
    );
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "A=0 0 0\nB=3 1 3");
}

/// ④ A whole-handle string assignment (`s = "abcd"`) beside a plain-net input, the
/// string twin of ①. PRE: `102 / 102 / 1004` — only the `t` cell moved. Both
/// oracles: `102 / 104 / 1004`.
#[test]
fn a_string_assignment_wakes_a_len_reader() {
    let (o, e, code) = run(r#"module f1g;
  string s = "ab"; int n; logic t = 0;
  always_comb n = s.len() + (t ? 1000 : 100);
  initial begin
    #1 $display("N1=%0d", n);
    s = "abcd";
    #1 $display("N2=%0d", n);
    t = 1;
    #1 $display("N3=%0d", n);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "N1=102\nN2=104\nN3=1004");
}

/// ⑤ A self-written accumulator (`foreach` sum) and a block-local + loop-variable
/// twin (`for (int i = 0; i < w.size(); i++)`), both re-allocated mid-run. PRE:
/// `0 / 0` for both. Both oracles: `6 / 16`.
///
/// The pair is deliberate: ⑤a's sensitivity contains the block's OWN write target
/// (`s`, because `s = s + w[i]` reads it) and ⑤b's contains a block-local plus a
/// loop variable, so the SELF-RETRIG author tag has to survive the staged mark in
/// both shapes or the block re-fires on itself.
#[test]
fn a_reallocation_wakes_a_self_accumulating_block_and_a_loop_var_block() {
    let (o, e, code) = run(r#"module g1a;
  int w[];
  int s;
  always_comb begin
    s = 0;
    foreach (w[i]) s = s + w[i];
  end
  initial begin
    w = new[3]; w[0]=1; w[1]=2; w[2]=3;
    #1 $display("A1=%0d", s);
    w = new[4]; w[0]=1; w[1]=2; w[2]=3; w[3]=10;
    #1 $display("A2=%0d", s);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "A1=6\nA2=16");

    let (o, e, code) = run(r#"module g1b;
  int w[];
  int n;
  always_comb begin
    int acc;
    acc = 0;
    for (int i = 0; i < w.size(); i++) acc = acc + w[i];
    n = acc;
  end
  initial begin
    w = new[3]; w[0]=1; w[1]=2; w[2]=3;
    #1 $display("B1=%0d", n);
    w = new[4]; w[0]=1; w[1]=2; w[2]=3; w[3]=10;
    #1 $display("B2=%0d", n);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "B1=6\nB2=16");
}

/// ⑥ CONTROL — a DECLARATION-INITIALISED array. `int w[] = new[3];` runs before
/// any process is armed (IEEE 1800 §6.21), so it must NOT hand `always_comb n =
/// w.size() + 100` an event; the block simply reads the already-allocated array on
/// its own t0 run. Right in PRE (103) and right in both oracles (103) — the cell
/// that would move if the initializer's heap marks escaped the t0 rollback.
#[test]
fn a_declaration_initialised_array_is_not_an_event() {
    let (o, e, code) = run(r#"module g1h;
  int w[] = new[3];
  int n;
  always_comb n = w.size() + 100;
  initial begin
    w[0]=1;
    #1 $display("H1=%0d", n);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "H1=103");
}

/// ⑦ CONTROL — an element READ mixed with a plain net (`n = w[0] + e`), where the
/// array stops moving after t0 and only `e` changes afterwards. Right in PRE (42)
/// and in both oracles (42): the funnel must not change a design whose handle is
/// quiet, and must not re-order the t0 answer.
#[test]
fn an_element_read_beside_a_plain_net_is_unchanged() {
    let (o, e, code) = run(r#"module t;
  int w [];
  int e;
  int n;
  initial begin
    w = new[3]; w[0] = 40;
    e = 1;
    #1 e = 2;
    #1 $display("POL=%0d", n);
    $finish;
  end
  always_comb n = w[0] + e;
  initial #100 $finish;
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "POL=42");
}

/// ⑧ CONTROL — the block WRITES the array it reads (`always_comb begin w[0] = e;
/// n = w[0]; end`). Both oracles print `5 5` / `9 9`, and so did PRE.
///
/// This is the convergence cell. The funnel stages a mark for `w[0] = e`, so
/// without the SELF-RETRIG author tag AND the same-value suppression the block
/// would be queued by its own write every delta. Both halves are measured here:
/// the author tag comes from `blocking_writer` (set by `Scheduler::run_body` and,
/// since this slice, by tier-3's batch loop on the SHARED state), and the
/// suppression is `elems[i] != coerced` inside the element store.
#[test]
fn a_block_that_writes_the_array_it_reads_still_converges() {
    let (o, e, code) = run(r#"module h2self;
  int w[]; int e; int n;
  always_comb begin
    w[0] = e;
    n = w[0];
  end
  initial begin
    w = new[2];
    e = 5;
    #1 $display("S1=%0d %0d", n, w[0]);
    e = 9;
    #1 $display("S2=%0d %0d", n, w[0]);
    $finish;
  end
  initial #100 $finish;
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "S1=5 5\nS2=9 9");
}

/// ⑨ CONTROL — the EXPLICIT-sensitivity twin. `always_ff @(posedge clk) n <=
/// w.size();` names only `clk`, so a heap mutation must not fire it; it picks the
/// new size up at the next clock edge and not before. `0 / 4` in PRE and in both
/// oracles.
#[test]
fn an_explicit_edge_sensitivity_is_not_woken_by_a_heap_mutation() {
    let (o, e, code) = run(r#"module h3ff;
  int w[]; int n; logic clk = 0;
  always #1 clk = ~clk;
  always_ff @(posedge clk) n <= w.size();
  initial begin
    #3 $display("F1=%0d", n);
    w = new[4];
    #4 $display("F2=%0d", n);
    $finish;
  end
  initial #100 $finish;
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "F1=0\nF2=4");
}

/// ⑩ CONTROL — a CONTINUOUS ASSIGN over the same read. `levelize::ca_deps` refuses
/// to certify any assign with a heap-handle dependency, so it is already in
/// `ca_always` and every settle pass re-evaluates it; the funnel deliberately does
/// NOT mark `ca_dirty`, and this cell is the measurement that says the omission is
/// not a gap. `0 / 3 / 7` in PRE and in verilator.
///
/// (iverilog is NOT an oracle for this design: `assign n = w.size();` aborts its
/// code generator — "Assertion failed: (0), function draw_type_string_of_nex" — so
/// the pin is verilator's line, said out loud rather than left implied.)
#[test]
fn a_continuous_assign_over_a_handle_is_unchanged() {
    let (o, e, code) = run(r#"module h1ca;
  int w[]; logic [31:0] n;
  assign n = w.size();
  initial begin
    #1 $display("C1=%0d", n);
    w = new[3];
    #1 $display("C2=%0d", n);
    w = new[7];
    #1 $display("C3=%0d", n);
    $finish;
  end
  initial #100 $finish;
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "C1=0\nC2=3\nC3=7");
}

/// ⑪ The QUEUE mutators one at a time — `push_back`, `push_front`, `insert`,
/// `delete(i)`, `pop_front`, and a whole-queue `delete()`. Each is a separate
/// `note_dyn_change` site, and the cell after it is the only observer that can
/// tell whether that site stages a mark.
///
/// PRE: `0 0 0 0 0 0`. verilator prints `1 2 3 2 1 0` and is the pin; iverilog
/// prints `1 1 1 1 1 0` — the split recorded in this file's header, in the form
/// that makes it a class rather than one cell: FIVE consecutive mutators leave its
/// reader untouched while `q.delete()` on the last line moves it.
#[test]
fn every_queue_mutator_wakes_a_size_reader() {
    let (o, e, code) = run(r#"module t;
  int q[$]; int m;
  always_comb m = q.size();
  initial begin
    q.push_back(7);   #1 $display("Q1=%0d", m);
    q.push_front(8);  #1 $display("Q2=%0d", m);
    q.insert(1, 9);   #1 $display("Q3=%0d", m);
    q.delete(0);      #1 $display("Q4=%0d", m);
    void'(q.pop_front()); #1 $display("Q5=%0d", m);
    q.delete();       #1 $display("Q6=%0d", m);
    $finish;
  end
  initial #100 $finish;
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "Q1=1\nQ2=2\nQ3=3\nQ4=2\nQ5=1\nQ6=0");
}

/// ⑫ The ASSOCIATIVE-array mutators — an element write that CREATES a key, one
/// that REPLACES a key with a different value, one that rewrites a key with the
/// SAME value (which must not be an event, and is observable only through the
/// count), and `delete(k)`.
///
/// verilator prints `1 1 1 0`. iverilog is NOT an oracle for this design — it
/// cannot elaborate `int a[int]` at all ("Type names are not valid expressions
/// here", "Object t.a has no method \"num(...)\"") — so the pin is verilator's
/// line, said out loud rather than left implied. PRE: `0 0 0 0`.
#[test]
fn assoc_writes_and_deletes_wake_a_num_reader() {
    let (o, e, code) = run(r#"module t;
  int a[int]; int m;
  always_comb m = a.num();
  initial begin
    a[3] = 1;    #1 $display("K1=%0d", m);
    a[3] = 2;    #1 $display("K2=%0d", m);
    a[3] = 2;    #1 $display("K3=%0d", m);
    a.delete(3); #1 $display("K4=%0d", m);
    $finish;
  end
  initial #100 $finish;
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "K1=1\nK2=1\nK3=1\nK4=0");
}

/// ⑬ The BACKEND DIFFERENTIAL. The wake is applied twice — `SimState::
/// mark_heap_dirty` for the engine's `st.dirty`, `native::run::drain_heap_marks`
/// for tier-3's `arena.ch.dirty` — off ONE producer. Two applies is exactly the
/// shape that drifts, so the same design is run on all three executors and the
/// bytes are compared, not just the values.
#[test]
fn the_three_backends_agree_on_a_woken_heap_reader() {
    let src = r#"module t;
  int w[]; int q[$]; string s; int n, m, l;
  always_comb n = w.size();
  always_comb m = q.size();
  always_comb l = s.len();
  initial begin
    w = new[3]; q.push_back(1); s = "abcd";
    #1 $display("X=%0d %0d %0d", n, m, l);
    w = new[5]; q.push_back(2); s = "z";
    #1 $display("Y=%0d %0d %0d", n, m, l);
    w.delete(); q.delete(); s = "";
    #1 $display("Z=%0d %0d %0d", n, m, l);
    $finish;
  end
  initial #100 $finish;
endmodule
"#;
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dhmw_be_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let mut seen: Vec<(String, String)> = Vec::new();
    for b in ["native", "interp", "vm"] {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .args(["--backend", b, "t.sv"])
            .current_dir(&d)
            .output()
            .expect("run vita");
        assert_eq!(
            out.status.code(),
            Some(0),
            "{b}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        seen.push((
            b.to_string(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        ));
    }
    let _ = std::fs::remove_dir_all(&d);
    // verilator: `X=3 1 4` / `Y=5 2 1` / `Z=0 0 0` (the pin). iverilog agrees on
    // every cell except the second `q.push_back` — `Y=5 1 1` — which is the split
    // this file's header records. PRE: three rows of `0 0 0`.
    assert!(
        seen[0].1.starts_with("X=3 1 4\nY=5 2 1\nZ=0 0 0\n"),
        "{:?}",
        seen[0]
    );
    for w in seen.windows(2) {
        assert_eq!(w[0].1, w[1].1, "{} vs {} differ", w[0].0, w[1].0);
    }
}

/// ⑭ THE NEGATIVE CHANNELS. A handle net carries no VCD variable and no probe
/// record, and the funnel emits neither — so the wake must not add bytes to either
/// artifact.
///
/// `$dumpvars` skips every DynArray/Queue/Assoc/AssocStr/String net (variable
/// length has no `$var` form), and `--probe` on one is a LOUD CLI refusal. Both
/// halves are asserted, because "no bytes appeared" and "the feature is off" look
/// identical from the outside otherwise.
#[test]
fn a_woken_handle_adds_no_vcd_and_no_probe_bytes() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dhmw_vcd_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("t.sv"),
        r#"module t; int w[]; int n;
  always_comb n = w.size();
  initial begin $dumpfile("d.vcd"); $dumpvars(0, t);
    w = new[3]; #1 $display("V=%0d", n); w = new[9]; #1 $display("W=%0d", n); $finish; end
  initial #100 $finish;
endmodule
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let so = String::from_utf8_lossy(&out.stdout);
    // Both oracles: `V=3` / `W=9` — the wake is live in a dumping run too. PRE: `0` / `0`.
    assert!(so.starts_with("V=3\nW=9\n"), "{so}");
    let vcd = std::fs::read_to_string(d.join("d.vcd")).expect("the VCD exists");
    // The design declares TWO nets and the dump declares ONE: `w`, the handle, has
    // no `$var` line, and `n` does.
    let vars: Vec<&str> = vcd.lines().filter(|l| l.starts_with("$var")).collect();
    assert_eq!(vars, vec!["$var reg 32 ! n [31:0] $end"], "{vcd}");
    // …and `n` carries BOTH of its transitions — the positive marker that makes the
    // absence above a fact rather than an empty-file artefact (§7.1: count a
    // positive marker in any probe that asserts absence).
    let w32 = |v: u32| format!("b{v:032b} !");
    assert!(vcd.contains(&w32(3)), "no n=3 record: {vcd}");
    assert!(vcd.contains(&w32(9)), "no n=9 record: {vcd}");

    // `--probe` on the handle is refused, loudly, at the CLI.
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--probe", "t.w", "--obs-dir", "obs", "t.sv"])
        .current_dir(&d)
        .output()
        .expect("run vita");
    let e = String::from_utf8_lossy(&out.stderr);
    assert_ne!(out.status.code(), Some(0), "{e}");
    assert!(e.contains("is a dynamic-array/queue/string handle"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

/// ⑯ THE OTHER TWO SENSITIVITY KINDS. `arm_sensitivity` treats `Level | Comb |
/// Latch` identically — all three carry their read set in `sensitivity.edges` — so
/// a fix measured only on `always_comb` would leave two thirds of the class open.
/// `always @*` (which lowers to `Level`) and `always_latch` are measured here.
///
/// PRE: `0 0` / `0 0` / `0 0`. BOTH oracles: `0 0` / `5 5` / `2 2`.
///
/// And the negative half: a handle in an EXPLICIT event control (`always @(w)`) is
/// a pre-existing loud refusal (E3009) that this slice does not touch, so the
/// wake cannot be mistaken for having opened that shape.
#[test]
fn always_star_and_always_latch_are_woken_too() {
    let (o, e, code) = run(r#"module t;
  int w[]; int a, b;
  always @* a = w.size();
  always_latch if (w.size() > 0) b <= w.size();
  initial begin
    #1 $display("P=%0d %0d", a, b);
    w = new[5];
    #1 $display("Q=%0d %0d", a, b);
    w = new[2];
    #1 $display("R=%0d %0d", a, b);
    $finish;
  end
  initial #100 $finish;
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "P=0 0\nQ=5 5\nR=2 2");

    let (_, e, code) = run(r#"module t;
  int w[]; int c;
  always @(w) c = w.size();
  initial begin #1 $display("C=%0d", c); #1 $finish; end
endmodule
"#);
    assert_ne!(code, Some(0), "{e}");
    assert!(
        e.contains("a dynamic-storage handle cannot appear in an event control"),
        "{e}"
    );
}
