//! Part-select WRITES into a frame-local slot and into a dynamic-array element
//! (IEEE 1800 §11.5.1 — out-of-range bits of a part-select are DROPPED).
//!
//! Both sites deposited the value one bit at a time, and that loop is also what
//! implements the out-of-range drop. §4.5.367 added a word-parallel fast arm for
//! the case where the window lies wholly inside the net, keeping the per-bit loop
//! verbatim for every other case. These tests pin BOTH arms, because the gate
//! between them is the whole correctness argument.
//!
//! ⚠️ The word-parallel primitive is `replace_bits`, NOT `copy_bits`: the latter
//! OR-merges into a destination it requires to be zero, and a part-select write's
//! destination is the slot's CURRENT value. Writing `8'h0F` over `8'hF0` with an
//! OR would read `8'hFF` — which is why several of the cases below deliberately
//! write ZERO bits and x/z bits over ONE bits.
//!
//! Every expected value was measured live on iverilog 13.0 (and verilator where it
//! runs the shape).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_psw_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

/// Run `body` inside BOTH an automatic function and an automatic task — the two
/// frame spellings — and return their `%h` results. A fast arm that fired in one
/// and not the other would show here.
fn frame_both(decl: &str, body: &str) -> (String, Option<i32>) {
    run(&format!(
        "module top;\n\
         \x20 function automatic [255:0] f();\n    {decl}\n    {body}\n    f = t;\n  endfunction\n\
         \x20 task automatic tt(output logic [255:0] o);\n    {decl}\n    {body}\n    o = t;\n  endtask\n\
         \x20 logic [255:0] a, b;\n\
         \x20 initial begin a = f(); tt(b); $display(\"FN=%h TK=%h\", a, b); $finish; end\n\
         endmodule\n"
    ))
}

fn assert_frame(decl: &str, body: &str, want: &str) {
    let (out, code) = frame_both(decl, body);
    assert_eq!(code, Some(0), "`{body}`: nonzero exit;\n{out}");
    assert!(
        out.contains(&format!("FN={want} TK={want}")),
        "`{body}`: want {want} from BOTH frame spellings;\n{out}"
    );
}

#[test]
fn a_fully_in_range_window_is_replaced_not_merged() {
    // ⚠️ THE OR-MERGE HAZARD, pinned first: every one of these writes ZERO bits
    // over ONE bits, so an OR-merge would leave the old bits standing.
    assert_frame(
        "logic [31:0] t;",
        "t = 32'hFFFFFFFF; t[15:8] = 8'h00;",
        "00000000000000000000000000000000000000000000000000000000ffff00ff",
    );
    assert_frame(
        "logic [31:0] t;",
        "t = 32'hAAAA5555; t[15:8] = 8'hF0;",
        "00000000000000000000000000000000000000000000000000000000aaaaf055",
    );
}

#[test]
fn the_window_may_be_unaligned_and_span_words() {
    // 64-bit word boundaries are where a hand-rolled clear/copy goes wrong.
    assert_frame(
        "logic [95:0] t;",
        "t = {96{1'b1}}; t[70:58] = 13'h0;",
        "0000000000000000000000000000000000000000ffffff8003ffffffffffffff",
    );
    assert_frame(
        "logic [127:0] t;",
        "t = {128{1'b1}}; t[100:50] = 51'h0;",
        "00000000000000000000000000000000ffffffe0000000000003ffffffffffff",
    );
    // `lsb + width == net_w` exactly — the fast arm's upper edge.
    assert_frame(
        "logic [31:0] t;",
        "t = 32'hAAAA5555; t[24 +: 8] = 8'h3C;",
        "000000000000000000000000000000000000000000000000000000003caa5555",
    );
}

#[test]
fn both_planes_are_replaced_not_merged() {
    // x/z over defined, and defined over x/z — the `unk` plane has to be CLEARED,
    // not just OR-ed, or an x can never be written away.
    let (out, code) = run("module top;\n\
         \x20 function automatic [31:0] f(input int mode);\n\
         \x20   logic [31:0] t; logic [7:0] p;\n\
         \x20   if (mode == 0) begin t = 32'hFFFFFFFF; p = 8'bxxxx_xxxx; end\n\
         \x20   else            begin t = 32'hxxxxxxxx; p = 8'h5A;      end\n\
         \x20   t[15:8] = p; f = t;\n  endfunction\n\
         \x20 initial begin $display(\"A=%h B=%h\", f(0), f(1)); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("A=ffffxxff B=xxxx5axx"),
        "x written over 1, and a defined value written over x;\n{out}"
    );
}

#[test]
fn an_out_of_range_window_still_drops_its_bits() {
    // ⚠️ THE ELSE ARM — the per-bit loop, kept verbatim, is what implements
    // §11.5.1. The gate exists to keep these OFF the fast arm.
    // Partially below the net: only the in-range bits land.
    assert_frame(
        "logic [31:0] t; int i;",
        "t = 32'hAAAA5555; i = -2; t[i +: 8] = 8'hFF;",
        "00000000000000000000000000000000000000000000000000000000aaaa557f",
    );
    // Partially above.
    assert_frame(
        "logic [31:0] t; int i;",
        "t = 32'hAAAA5555; i = 28; t[i +: 8] = 8'hFF;",
        "00000000000000000000000000000000000000000000000000000000faaa5555",
    );
    // Wholly outside, both directions — the net is untouched.
    for lsb in ["64", "-9"] {
        assert_frame(
            "logic [31:0] t; int i;",
            &format!("t = 32'hAAAA5555; i = {lsb}; t[i +: 8] = 8'hFF;"),
            "00000000000000000000000000000000000000000000000000000000aaaa5555",
        );
    }
    // A width WIDER than the net drops everything above it.
    assert_frame(
        "logic [31:0] t; int i;",
        "t = 32'hAAAA5555; i = 0; t[i +: 40] = 40'h0;",
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
}

#[test]
fn the_dynamic_array_element_twin_follows_the_same_rule() {
    // The second deposit site. `w` there is the ELEMENT width, and the gate must
    // use that one.
    let (out, code) = run("module top;\n  logic [31:0] d[];\n\
         \x20 initial begin d = new[2]; d[0] = 32'hFFFFFFFF; d[1] = 32'hAAAA5555;\n\
         \x20   d[0][15:8] = 8'h0F;\n\
         \x20   d[1][28 +: 8] = 8'hFF;\n\
         \x20   $display(\"A=%h B=%h\", d[0], d[1]); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("A=ffff0fff B=faaa5555"),
        "in-range replace, and an out-of-range window that still drops;\n{out}"
    );
}

#[test]
fn the_two_arms_agree_on_a_single_bit_and_on_the_whole_net() {
    // The gate's other two edges, both oracle-backed: a 1-bit window (the
    // narrowest fast-arm case) and `width == net_w` (the widest).
    assert_frame(
        "logic [31:0] t;",
        "t = 32'hAAAA5555; t[13] = 1'b0;",
        "00000000000000000000000000000000000000000000000000000000aaaa5555",
    );
    assert_frame(
        "logic [31:0] t;",
        "t = 32'hAAAA5555; t[31:0] = 32'h12345678;",
        "0000000000000000000000000000000000000000000000000000000012345678",
    );
    // ⚠️ A truly ZERO-width indexed part-select is not pinned here: iverilog
    // REJECTS it ("Indexed part select width must be an integral constant greater
    // than zero"), so there is no oracle for the value. vita accepts it silently —
    // a pre-existing loud gap recorded in ROADMAP §2, not something this slice
    // changed. The `width > 0` half of the gate is what keeps both arms doing
    // nothing there.
}
