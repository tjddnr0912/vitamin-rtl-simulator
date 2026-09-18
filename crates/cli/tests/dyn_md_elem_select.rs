//! A SELECT into an element of dynamic storage whose element type carries more
//! than one packed dimension (`typedef logic [1:0][3:0] t_ex; t_ex q [$];` then
//! `q[0][1]`).
//!
//! The handle stores each element flat, so the second index used to lower to a
//! BIT-select of that flat value: vita printed `0` (bit 1) where verilator prints
//! `a` (packed element 1). iverilog is not an oracle for the two-index form — it
//! refuses it outright ("the number of indices (2) is greater than the number of
//! dimensions (1)"), which is itself a third answer. Measured PRE-EXISTING: the
//! explicit-typedef spelling needs no `parameter type` at all and was already
//! wrong; the multi-dim packed `parameter type` slice only routed `T q [$]` onto
//! it.
//!
//! The fix is LOUD, never a value: every whole-element operation — `q[0]`,
//! `q.push_back`, `q.size()`, `d[i] = v`, `a[k]` — keeps its measured value, and a
//! ONE-dimensional element type (`logic [7:0] q1 [$]`, where `q1[0][1]` is a real
//! bit-select) is untouched and still agrees with verilator.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dmes_{}_{n}", std::process::id()));
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
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (all, out.status.success())
}

/// The design's own `$display` lines, in the order the run printed them.
fn said(out: &str, tag: &str) -> Vec<String> {
    out.lines()
        .filter(|l| l.starts_with(tag))
        .map(|l| l.to_string())
        .collect()
}

fn prints(src: &str, tag: &str, want: &[&str]) {
    let (out, ok) = vita(src);
    assert!(ok, "expected exit 0, got:\n{out}");
    assert_eq!(said(&out, tag), want, "{out}");
}

fn loud(src: &str, needle: &str) {
    let (out, ok) = vita(src);
    assert!(!ok, "expected a loud reject, got exit 0:\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in:\n{out}");
}

/// The one sentence every refusal in this file must carry.
const NEEDLE: &str = "more than one packed dimension";

// ───────────────────────── the read (the silent-wrong that was) ─────────────────────────

/// Lens design `p5` verbatim: the explicit-typedef spelling, no `parameter type`.
/// This ran on PRE and printed `p5 q0=a5 e1=0 e0=1 eb1=0 eb0=1 v1=a v0=5` — the
/// `e1=0` is bit 1 of the flat element where verilator answers `e1=a`, and `v1=a`
/// on the same line is the plain variable getting it right.
#[test]
fn a_queue_element_select_is_refused_not_bit_selected() {
    loud(
        "typedef logic [1:0][3:0] t_ex;\n\
         module top;\n\
         \x20 t_ex q [$]; t_ex v;\n\
         \x20 initial begin\n\
         \x20   q.push_back(8'hA5); v = 8'hA5;\n\
         \x20   $display(\"p5 q0=%h e1=%h e0=%h eb1=%b eb0=%b v1=%h v0=%h\", q[0], q[0][1], q[0][0], q[0][1], q[0][0], v[1], v[0]);\n\
         \x20 end\n\
         \x20 initial #100 $finish;\n\
         endmodule\n",
        NEEDLE,
    );
}

/// The dynamic-ARRAY twin. Measured on PRE: `r3 d0=a5 e1=0`; verilator `r3 d0=a5
/// e1=a`; iverilog refuses the two-index form.
#[test]
fn a_dynamic_array_element_select_is_refused() {
    loud(
        "typedef logic [1:0][3:0] t_ex;\n\
         module top;\n\
         \x20 t_ex d [];\n\
         \x20 initial begin d = new[2]; d[0] = 8'hA5; $display(\"r3 d0=%h e1=%h\", d[0], d[0][1]); end\n\
         \x20 initial #5 $finish;\n\
         endmodule\n",
        NEEDLE,
    );
}

/// The ASSOCIATIVE-array twin. Measured on PRE: `r4 a3=a5 e1=0 n=1`; verilator
/// `r4 a3=a5 e1=a n=1`.
#[test]
fn an_associative_array_element_select_is_refused() {
    loud(
        "typedef logic [1:0][3:0] t_ex;\n\
         module top;\n\
         \x20 t_ex a [int];\n\
         \x20 initial begin a[3] = 8'hA5; $display(\"r4 a3=%h e1=%h n=%0d\", a[3], a[3][1], a.num()); end\n\
         \x20 initial #5 $finish;\n\
         endmodule\n",
        NEEDLE,
    );
}

/// Lens design `b10` verbatim — the same element type reached through
/// `parameter type T`, the spelling the multi-dim packed type-param slice newly
/// routes onto this path. Whole-element `q[0]`/`q[1]`/`q.size()` were right there
/// too (`b10 n=2 q0=a5 q1=5a e=0`); only `e` was wrong, and only `e` is refused.
#[test]
fn the_same_element_select_through_a_type_parameter_is_refused() {
    loud(
        "module child #(parameter type T = logic [1:0][3:0]) ();\n\
         \x20 T q [$];\n\
         \x20 initial begin q.push_back(8'hA5); q.push_back(8'h5A);\n\
         \x20   $display(\"b10 n=%0d q0=%h q1=%h e=%h\", q.size(), q[0], q[1], q[0][1]); end\n\
         endmodule\n\
         module top; child u(); initial #100 $finish; endmodule\n",
        NEEDLE,
    );
}

/// A part-select and an indexed part-select of the same element. vita answered
/// `ps=9 ip=9` (flat bits 5:2 of 0xa5) and verilator `ps=00a5 ip=00a5` — a second
/// divergence on the same base, so both arms carry the guard.
#[test]
fn a_part_select_and_an_indexed_part_select_of_such_an_element_are_refused() {
    let src = "typedef logic [1:0][3:0] t_ex;\n\
         module top;\n\
         \x20 t_ex q [$];\n\
         \x20 initial begin q.push_back(8'hA5); $display(\"r7 ps=%h\", q[0][5:2]); end\n\
         \x20 initial #5 $finish;\n\
         endmodule\n";
    loud(src, NEEDLE);
    let src = "typedef logic [1:0][3:0] t_ex;\n\
         module top;\n\
         \x20 t_ex q [$];\n\
         \x20 initial begin q.push_back(8'hA5); $display(\"r7 ip=%h\", q[0][2+:4]); end\n\
         \x20 initial #5 $finish;\n\
         endmodule\n";
    loud(src, NEEDLE);
}

// ───────────────────────── the write ─────────────────────────

/// The lvalue twin `q[0][1] = 4'h3;` was NOT silent: PRE already refuses every
/// nested lvalue select on a dynamic handle — measured verbatim
/// `error[VITA-E3009] E-ELAB-UNSUPPORTED: nested lvalue select (v1: single-level)`
/// for the queue, the dynamic array and the one-dimensional element alike
/// (verilator writes the element: `r2 q0=35`; iverilog aborts on an assertion).
/// So the write needs no new gate — this pins that it stays loud, and pins WHICH
/// refusal answers it, so a later change cannot turn it into a flat-bit write.
#[test]
fn an_element_write_stays_loud_on_the_existing_nested_lvalue_gate() {
    let write = "typedef logic [1:0][3:0] t_ex;\n\
         module top;\n\
         \x20 t_ex q [$];\n\
         \x20 initial begin q.push_back(8'hA5); q[0][1] = 4'h3; $display(\"r2 q0=%h\", q[0]); end\n\
         \x20 initial #5 $finish;\n\
         endmodule\n";
    loud(write, "nested lvalue select");
    let dyn_write = "typedef logic [1:0][3:0] t_ex;\n\
         module top;\n\
         \x20 t_ex d [];\n\
         \x20 initial begin d = new[2]; d[0] = 8'hA5; d[0][1] = 4'h3; $display(\"r6 d0=%h\", d[0]); end\n\
         \x20 initial #5 $finish;\n\
         endmodule\n";
    loud(dyn_write, "nested lvalue select");
}

// ───────────────────────── what must NOT move ─────────────────────────

/// Every whole-element operation across all three container kinds, plus a plain
/// variable of the same type. Verilator prints this line verbatim (iverilog does
/// not take a type name as a `$bits`/declaration argument here).
#[test]
fn whole_element_reads_and_writes_keep_their_values() {
    prints(
        "typedef logic [1:0][3:0] t_ex;\n\
         module top;\n\
         \x20 t_ex q [$]; t_ex d []; t_ex a [int]; t_ex v;\n\
         \x20 initial begin\n\
         \x20   q.push_back(8'hA5); q.push_back(8'h5A);\n\
         \x20   d = new[2]; d[0] = 8'hC3; d[1] = q[1];\n\
         \x20   a[7] = 8'h3C;\n\
         \x20   v = q[0];\n\
         \x20   $display(\"r14 q0=%h q1=%h n=%0d d0=%h d1=%h a7=%h na=%0d v=%h v1=%h\",\n\
         \x20            q[0], q[1], q.size(), d[0], d[1], a[7], a.num(), v, v[1]);\n\
         \x20 end\n\
         \x20 initial #5 $finish;\n\
         endmodule\n",
        "r14 ",
        &["r14 q0=a5 q1=5a n=2 d0=c3 d1=5a a7=3c na=1 v=a5 v1=a"],
    );
}

/// The `parameter type` spelling of the same whole-element traffic — 3-way
/// identical (vita, iverilog 13.0 and verilator 5.052 all print this line).
#[test]
fn whole_element_traffic_through_a_type_parameter_keeps_its_values() {
    prints(
        "module child #(parameter type T = logic [1:0][3:0]) ();\n\
         \x20 T q [$];\n\
         \x20 initial begin q.push_back(8'hA5); q.push_back(8'h5A);\n\
         \x20   $display(\"r15 n=%0d q0=%h q1=%h\", q.size(), q[0], q[1]); end\n\
         endmodule\n\
         module top; child u(); initial #5 $finish; endmodule\n",
        "r15 ",
        &["r15 n=2 q0=a5 q1=5a"],
    );
}

/// The exclusion that makes the gate a gate: a ONE-dimensional element type. Here
/// `q1[0][1]` IS a bit-select, and vita's answer matches verilator's verbatim
/// (`r1 q0=a5 b1=0 b0=1 b7=1`). iverilog refuses the two-index form, so this cell
/// is verilator-pinned.
#[test]
fn a_one_dimensional_element_type_still_bit_selects() {
    prints(
        "module top;\n\
         \x20 logic [7:0] q1 [$];\n\
         \x20 initial begin\n\
         \x20   q1.push_back(8'hA5);\n\
         \x20   $display(\"r1 q0=%h b1=%b b0=%b b7=%b\", q1[0], q1[0][1], q1[0][0], q1[0][7]);\n\
         \x20 end\n\
         \x20 initial #5 $finish;\n\
         endmodule\n",
        "r1 ",
        &["r1 q0=a5 b1=0 b0=1 b7=1"],
    );
}

/// A STATIC unpacked array of the same element type is a different storage class
/// and was always correct — `a[0][1]` is the packed element. Pinned so the gate
/// cannot widen onto it. 3-way: vita = iverilog = verilator.
#[test]
fn a_static_unpacked_array_of_the_same_element_type_is_untouched() {
    prints(
        "typedef logic [1:0][3:0] t_ex;\n\
         module top;\n\
         \x20 t_ex s [0:1]; t_ex w;\n\
         \x20 initial begin s[0] = 8'hA5; w = 8'hA5; $display(\"r16 s0=%h e=%h w1=%h\", s[0], s[0][1], w[1]); end\n\
         \x20 initial #5 $finish;\n\
         endmodule\n",
        "r16 ",
        &["r16 s0=a5 e=a w1=a"],
    );
}
