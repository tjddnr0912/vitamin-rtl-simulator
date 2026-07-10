//! HIER-REST-PS (read): a hierarchical PART-select / INDEXED-part READ of a
//! NON-zero-LSB net — scalar `dut.v[m:l]` / `dut.v[b+:w]` and array-element
//! `dut.mem[i][m:l]`. The READ previously wrapped the deferred hierarchical Signal
//! with a RAW offset (the net's declared LSB is unknown until pass 8), so a
//! non-zero-LSB net (`logic [15:8]`) read out-of-range bits → silent X. Now the
//! part-select is DEFERRED with a `HierPart` (mirroring the write side) and the
//! offset is normalized against the element/net LSB at resolution. Bit-selects and
//! zero-LSB nets already worked and stay byte-identical. Pure IR-0 (elaborate only).
//!
//! Every value is pinned to LIVE iverilog 13.0. iverilog asserts on a struct-field
//! `-:`, but a hierarchical NET `-:` is fine, so those are pinned directly.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita(src: &str) -> std::process::Output {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hpr_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita")
}

fn run(src: &str) -> String {
    let out = vita(src);
    assert!(
        out.status.success(),
        "vita failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn run_loud(src: &str) {
    let out = vita(src);
    assert!(
        !out.status.success(),
        "expected a loud refusal, got:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

// ── scalar / vector non-zero-LSB net (`logic [15:8]`) ────────────────────────

#[test]
fn scalar_nonzero_lsb_part_select() {
    // sc = 5A on [15:8]: sc[11:8] = low nibble = A, sc[15:12] = 5.
    let out = run("module sub; logic [15:8] sc; initial sc=8'h5A; endmodule\n\
         module top; sub d();\n\
           initial begin #1; $display(\"R %h %h\", d.sc[11:8], d.sc[15:12]); end\n\
         endmodule\n");
    assert!(
        out.contains("R a 5"),
        "scalar non-zero-LSB part-select:\n{out}"
    );
}

#[test]
fn scalar_nonzero_lsb_indexed_part() {
    let out = run("module sub; logic [15:8] sc; initial sc=8'h5A; endmodule\n\
         module top; sub d();\n\
           initial begin #1; $display(\"R %h %h\", d.sc[8+:4], d.sc[15-:4]); end\n\
         endmodule\n");
    assert!(
        out.contains("R a 5"),
        "scalar non-zero-LSB indexed part:\n{out}"
    );
}

// ── array element with a non-zero-LSB element range ──────────────────────────

#[test]
fn array_elem_nonzero_lsb_part_select() {
    let out = run("module sub; logic [15:8] mem[0:1];\n\
           initial begin mem[0]=8'h5A; mem[1]=8'h3C; end\n\
         endmodule\n\
         module top; sub d();\n\
           initial begin #1; $display(\"R %h %h\", d.mem[0][11:8], d.mem[0][15:12]); end\n\
         endmodule\n");
    assert!(
        out.contains("R a 5"),
        "array-elem non-zero-LSB part-select:\n{out}"
    );
}

#[test]
fn array_elem_nonzero_lsb_indexed_part() {
    let out = run("module sub; logic [15:8] mem[0:1];\n\
           initial begin mem[0]=8'h5A; mem[1]=8'h3C; end\n\
         endmodule\n\
         module top; sub d();\n\
           initial begin #1; $display(\"R %h\", d.mem[0][8+:4]); end\n\
         endmodule\n");
    assert!(
        out.contains("R a"),
        "array-elem non-zero-LSB indexed part:\n{out}"
    );
}

#[test]
fn three_level_hier_part_select() {
    let out = run("module sub; logic [15:8] mem[0:1];\n\
           initial begin mem[0]=8'h5A; mem[1]=8'h3C; end\n\
         endmodule\n\
         module mid; sub s(); endmodule\n\
         module top; mid m();\n\
           initial begin #1; $display(\"R %h %h\", m.s.mem[0][11:8], m.s.mem[1][15-:4]); end\n\
         endmodule\n");
    assert!(out.contains("R a 3"), "3-level hier part-select:\n{out}");
}

// ── byte-identity: bit-select and zero-LSB net already worked, stay correct ──

#[test]
fn bit_select_still_correct() {
    // Trailing bit-select on a non-zero-LSB hier element: was already correct.
    let out = run(
        "module sub; logic [15:8] mem[0:1]; initial mem[0]=8'h5A; endmodule\n\
         module top; sub d();\n\
           initial begin #1; $display(\"R %b %b\", d.mem[0][9], d.mem[0][8]); end\n\
         endmodule\n",
    );
    assert!(out.contains("R 1 0"), "hier bit-select:\n{out}");
}

#[test]
fn zero_lsb_part_select_unchanged() {
    // Zero-LSB net / element: offset normalization is a no-op (lsb 0) — unchanged.
    let out = run("module sub; logic [7:0] z; logic [7:0] bs[0:1];\n\
           initial begin z=8'hE7; bs[0]=8'h3C; end\n\
         endmodule\n\
         module top; sub d();\n\
           initial begin #1; $display(\"R %h %h\", d.z[7:4], d.bs[0][3:0]); end\n\
         endmodule\n");
    assert!(out.contains("R e c"), "zero-LSB hier part-select:\n{out}");
}

#[test]
fn whole_element_read_unchanged() {
    let out = run(
        "module sub; logic [15:8] mem[0:1]; initial mem[0]=8'h5A; endmodule\n\
         module top; sub d();\n\
           initial begin #1; $display(\"R %h\", d.mem[0]); end\n\
         endmodule\n",
    );
    assert!(out.contains("R 5a"), "whole hier element read:\n{out}");
}

// ── correct-or-loud: multi-dim packed element sub-part-select is a follow-on ──

#[test]
fn multidim_packed_elem_part_select_is_loud() {
    run_loud(
        "module sub; logic [3:0][7:0] qm[0:1]; initial qm[0]=32'h11223344; endmodule\n\
         module top; sub d();\n\
           initial begin #1; $display(\"%h\", d.qm[0][1][3:0]); end\n\
         endmodule\n",
    );
}

// ── correct-or-loud: ascending-net indexed part-select is loud (follow-on) ────
// The select KIND is baked descending at lowering (net direction unknown then),
// so an ascending `[b+:w]`/`[b-:w]` would walk the wrong way → loud-reject rather
// than silently mis-select. The ascending `[m:l]` twin is already loud (width
// check). (R2 finding: was silent `x10`.)

#[test]
fn ascending_net_indexed_part_is_loud() {
    run_loud(
        "module sub; logic [8:15] v; initial v=8'b1010_0110; endmodule\n\
         module top; sub d();\n\
           initial begin #1; $display(\"%b\", d.v[9+:3]); end\n\
         endmodule\n",
    );
}

#[test]
fn ascending_net_minus_indexed_part_is_loud() {
    run_loud(
        "module sub; logic [8:15] v; initial v=8'b1010_0110; endmodule\n\
         module top; sub d();\n\
           initial begin #1; $display(\"%b\", d.v[14-:3]); end\n\
         endmodule\n",
    );
}

#[test]
fn ascending_array_elem_indexed_part_is_loud() {
    run_loud(
        "module sub; logic [8:15] mem[0:1]; initial mem[1]=8'b1010_0110; endmodule\n\
         module top; sub d();\n\
           initial begin #1; $display(\"%b\", d.mem[1][9+:3]); end\n\
         endmodule\n",
    );
}
