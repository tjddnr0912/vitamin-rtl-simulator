//! `$size`, `$high`, `$right`, `$low`, `$left`, `$increment`, `$dimensions` and
//! `$unpacked_dimensions` of a DYNAMIC ARRAY or QUEUE are runtime facts.
//!
//! The introspection fold (`try_introspect_fold`) resolves the handle net and
//! asks `net_dims_desc`, which on a handle net sees only the ELEMENT's packed
//! dimension — so `$size(da)` folded to the element WIDTH: `SD=32` for both oracles'
//! `3`, `$high(q)` `31` for `1`, `$dimensions(da)` `1` for `2`. A dyn/queue handle
//! now takes its own arm: `$size` lowers to the `.size()` route (`DynSize`),
//! `$high`/`$right` to one less, `$low`/`$left` to 0, `$increment` to −1 (the
//! unpacked dimension is `[0:size-1]`), `$dimensions` to one plus the element's
//! packed dims and `$unpacked_dimensions` to 1. An associative array declines
//! (loud, where it silently answered 32), and an explicit dimension argument keeps
//! the constant path. A replication count built from `$size(da)` / `$bits(da)` is
//! loud (it was a silent 0 / the element width), and a negative one (`$increment`)
//! is loud too.
//!
//! ORACLES: iverilog 13.0 (-g2012) and verilator 5.052 (--binary --timing). Every
//! value below was measured in both except `$dimensions` / `$unpacked_dimensions`,
//! where iverilog contradicts itself (`$high` 2 but `$dimensions` 0) and verilator +
//! the LRM are pinned. PRE values are from a release binary built at the parent commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dyni_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
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

/// ① THE HEADLINE: every query on a dynamic array and a queue, beside
/// `$bits(da.size())`, which was already right.
#[test]
fn every_query_on_a_dynamic_array_or_queue_is_its_runtime_geometry() {
    let (o, e, code) = run(r#"module t;
  int da[]; int q[$];
  initial begin
    da = new[3]; q.push_back(1); q.push_back(2);
    $display("SD=%0d SQ=%0d BD=%0d", $size(da), $size(q), $bits(da.size()));
    $display("HD=%0d LD=%0d DD=%0d HQ=%0d UD=%0d RD=%0d LFD=%0d", $high(da), $low(da), $dimensions(da), $high(q), $unpacked_dimensions(da), $right(da), $left(da));
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    // PRE: `SD=32 SQ=32 BD=32` and `HD=31 LD=0 DD=1 HQ=31 UD=0 RD=0 LFD=31`.
    assert_eq!(o, "SD=3 SQ=2 BD=32\nHD=2 LD=0 DD=2 HQ=1 UD=1 RD=2 LFD=0");
}

/// ② The size is LIVE (a loop bound over `$size`, a signed comparison against it,
/// an empty queue's `$high` = −1), `$increment` and a 4-bit element's
/// `$dimensions`, and the static array / explicit-dimension controls that keep the
/// constant path.
#[test]
fn the_size_is_live_and_the_static_controls_are_unchanged() {
    let (o, e, code) = run(r#"module t;
  int da[]; int q[$]; logic [3:0] w[]; int st[0:4]; logic [7:0] pk [3][2];
  initial begin
    da = new[4]; w = new[5];
    $display("E=%0d I=%0d DW=%0d SW=%0d HW=%0d LT=%0d", $high(q), $increment(da), $dimensions(w), $size(w), $high(w), $size(da) - 4 < 0);
    for (int i = 0; i < $size(da); i++) da[i] = i * 3;
    $display("D=%0d %0d SS=%0d P1=%0d P2=%0d", da[3], $size(da) * 2 - 9, $size(st), $size(pk, 1), $size(pk, 2));
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    // PRE: `E=31 I=1 DW=1 SW=4 HW=3 LT=0`, the loop wrote nothing (`W4020`), and
    // `D=9 55` (a 32-iteration bound read past the array); `LT` is `4 - 4 < 0`.
    assert_eq!(o, "E=-1 I=-1 DW=2 SW=5 HW=4 LT=0\nD=9 -1 SS=5 P1=3 P2=2");
}

/// ③ An associative array's `$size` is LOUD now (PRE printed the element width,
/// 32; iverilog refuses the declaration and verilator answers 0 for a non-empty
/// one, so there is no oracle to pin).
#[test]
fn an_associative_arrays_size_declines() {
    let (_, e, code) = run(r#"module t;
  int aa[int];
  initial begin
    aa[5] = 1;
    $display("SA=%0d", $size(aa));
    #1 $finish;
  end
endmodule
"#);
    assert_ne!(code, Some(0));
    assert!(e.contains("unsupported system function"), "{e}");
}

/// ④ THE FILED RESIDUE, NOW CLOSED. This cell used to pin the STALE `N1=100`: an
/// inferred-sensitivity block reading a dynamic-storage handle was not woken by a
/// handle mutation, because the wake runs off the dirty sweep and no heap write
/// reached `note_change`. Round 1 measured `$size(w)` joining the class once it
/// became a runtime read (PRE's element-width constant had coincided with
/// `new[4]`; `new[3]` printed 104 for 103 in PRE too), and the three refusal
/// attempts that followed each refused designs PRE and both oracles run — which is
/// why the axis was reverted and the class filed rather than gated.
///
/// It is a WAKE now, not a refusal: `SimState::note_dyn_change` stages every
/// module-net heap mutation and both schedulers drain it into their dirty channel.
/// So the cell is CONVERTED to the oracle value rather than deleted — `N1=104`,
/// which is what iverilog 13.0 and verilator 5.052 both print (raw:
/// `N1=104` / `N2=1004` from each). The class and its measurements live in
/// `crates/cli/tests/dyn_handle_mutation_wakes_comb.rs`.
#[test]
fn a_dyn_handle_read_in_an_inferred_sensitivity_block_is_woken() {
    let (o, e, code) = run(
        "module t;\n  logic [3:0] w[]; int n; logic t = 0;\n  always_comb n = $size(w) + (t ? 1000 : 100);\n  initial begin w = new[4]; #1 $display(\"N1=%0d\", n); t = 1; #1 $display(\"N2=%0d\", n); #1 $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "N1=104\nN2=1004");
    // A declaration-initialised handle is final before the block's t0 fire, and the
    // initializer must NOT be an event: correct in all four tools, PRE-identical,
    // and the cell that says the wake did not swallow the t0 rollback (the round-3
    // correct→loud cell, now the round-4 correct→correct one).
    let (o, e, code) = run(
        "module t;\n  int w[] = new[3]; string s = \"hello\"; int n, m;\n  always_comb n = w.size() + 100;\n  always_comb m = s.len();\n  initial begin w[0] = 1; #1 $display(\"H=%0d L=%0d\", n, m); #1 $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "H=103 L=5");
}

/// ⑤ ROUND-1 DIFFERENTIAL FINDING (closed loud): a REPLICATION COUNT built from
/// `$size(da)` folded to a silent 0 (`{{$size(da){1'b1}}}` = `00000000`) where both
/// oracles refuse the design — the count's runtime-net detector treated every
/// type query as a constant. A type query over a dyn/queue handle is a runtime
/// read there now, so the count is refused like any other runtime count; the
/// `localparam` / range-bound spellings were loud already.
#[test]
fn a_replication_count_from_a_dyn_size_is_loud() {
    let (_, e, code) = run(r#"module t;
  int da[]; logic [31:0] z;
  initial begin
    da = new[3];
    z = {{$size(da){1'b1}}};
    $display("Z=%h", z);
    #1 $finish;
  end
endmodule
"#);
    assert_ne!(code, Some(0));
    assert!(
        e.contains("a replication count must be a constant expression"),
        "{e}"
    );
    // The queries that fold to a constant over a handle stay a constant count
    // (round-2 soundness residue): `$dimensions(da)` is 2 (verilator `{2{1'b1}}`).
    let (o, e, code) = run(r#"module t;
  logic [3:0] da[]; logic [31:0] d1, d2;
  initial begin
    da = new[3];
    d1 = {{$dimensions(da){1'b1}}}; d2 = {{$unpacked_dimensions(da){1'b1}}};
    $display("D=%h %h", d1, d2);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "D=00000003 00000001");
    // `$increment(da)` folds to `32'sd-1`; a NEGATIVE count is loud (both oracles
    // refuse; PRE replicated the two's-complement pattern, `55` for `f5`).
    let (_, e, code) = run(r#"module t;
  int da[]; logic [7:0] z1;
  initial begin
    da = new[3];
    z1 = {4'ha, {$increment(da){1'b1}}, 4'h5};
    $display("B=%h", z1);
    #1 $finish;
  end
endmodule
"#);
    assert_ne!(code, Some(0));
    assert!(e.contains("a replication count may not be negative"), "{e}");
}
