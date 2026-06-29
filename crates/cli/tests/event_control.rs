//! Bit-select event control (`always @(posedge clk[k])`).
//!
//! The engine's edge model detects posedge/negedge on BIT 0 of a net only
//! (IEEE: a vector's `posedge` tracks its LSB). So a CONSTANT bit-select whose
//! selected source bit IS the net's LSB (packed bit 0) is EXACTLY representable
//! by arming on the underlying net — and `sens_event_net` returns the same net
//! id as the bare-ident path, leaving existing goldens byte-identical.
//!
//! Packed bit 0 corresponds to source index `nv.lsb` in BOTH range directions
//! (descending `[hi:lo]` → index lo; ascending `[lo:hi]` stored msb<lsb → index
//! = the larger bound). Any other bit (non-LSB const), part-select, variable /
//! non-const index, array-element, multi-dim-packed, or computed base needs
//! per-bit edge tracking the engine lacks → LOUD reject (VITA-E3009), never the
//! old silent `warn` + POISON_NET that indexed `net_to_edge[u32::MAX]` and
//! panicked the scheduler.
//!
//! Oracle: iverilog 13.0 (probes A/B/C/D/E/F, stimulus 00→01→00→11→10).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_evtctl_{}_{n}", std::process::id()));
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
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// A clean elaboration error is exit 1 with the E3009 text; a panic would be
/// exit 101 (and no E3009), so `Some(1)` precisely rules out the old crash.
fn assert_rejected(err: &str, code: Option<i32>) {
    assert_eq!(
        code,
        Some(1),
        "expected clean E3009 (exit 1), not a panic; stderr:\n{err}"
    );
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
}

// ── supported: selected bit IS the LSB ───────────────────────────────────

#[test]
fn descending_lsb_bitselect_arms_on_net() {
    // reg[1:0], clk[0] == nv.lsb 0 → packed bit 0. LSB seq 0,1,0,1,1 = 2 posedges.
    // iverilog probe A: cnt=2.
    let (out, err, code) = run("module t;\n\
         reg [1:0] clk; integer cnt=0;\n\
         always @(posedge clk[0]) cnt=cnt+1;\n\
         initial begin clk=0; #1 clk=2'b01; #1 clk=2'b00; #1 clk=2'b11; #1 clk=2'b10;\n\
           #1 $display(\"A cnt=%0d\",cnt); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("A cnt=2"), "got:\n{out}");
}

#[test]
fn ascending_lsb_bitselect_arms_on_net() {
    // reg[0:1] ascending → nv.lsb=1, clk[1] is the LSB. iverilog probe C: cnt=2.
    let (out, err, code) = run("module t;\n\
         reg [0:1] clk; integer cnt=0;\n\
         always @(posedge clk[1]) cnt=cnt+1;\n\
         initial begin clk=0; #1 clk=2'b01; #1 clk=2'b00; #1 clk=2'b11; #1 clk=2'b10;\n\
           #1 $display(\"C cnt=%0d\",cnt); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("C cnt=2"), "got:\n{out}");
}

#[test]
fn scalar_1bit_bitselect_supported() {
    // A 1-bit `reg clk` is [0:0]; clk[0] == lsb 0. Equals the bare `@(posedge clk)`.
    let (out, err, code) = run("module t;\n\
         reg clk; integer cnt=0;\n\
         always @(posedge clk[0]) cnt=cnt+1;\n\
         initial begin clk=0; #1 clk=1; #1 clk=0; #1 clk=1;\n\
           #1 $display(\"S cnt=%0d\",cnt); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("S cnt=2"), "got:\n{out}");
}

#[test]
fn negedge_lsb_bitselect_supported() {
    // negedge on the LSB bit-select goes through the same arm-on-net path as
    // `@(negedge clk)`; both equal iverilog (probe: cnt=2 for this stimulus).
    let (out, err, code) = run("module t;\n\
         reg [1:0] clk; integer cnt=0;\n\
         always @(negedge clk[0]) cnt=cnt+1;\n\
         initial begin clk=0; #1 clk=2'b01; #1 clk=2'b00; #1 clk=2'b11;\n\
           #1 $display(\"N cnt=%0d\",cnt); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("N cnt=2"), "got:\n{out}");
}

// ── regression: bare-ident path must stay byte-identical ──────────────────

#[test]
fn bare_vector_posedge_unchanged() {
    // `@(posedge clk)` on a vector tracks the LSB (bit0). iverilog probe D: cnt=2.
    let (out, err, code) = run("module t;\n\
         reg [1:0] clk; integer cnt=0;\n\
         always @(posedge clk) cnt=cnt+1;\n\
         initial begin clk=0; #1 clk=2'b01; #1 clk=2'b00; #1 clk=2'b11; #1 clk=2'b10;\n\
           #1 $display(\"D cnt=%0d\",cnt); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("D cnt=2"), "got:\n{out}");
}

// ── loud reject: selected bit is NOT the LSB ──────────────────────────────

#[test]
fn descending_non_lsb_bitselect_rejected() {
    // reg[1:0], clk[1] is the MSB (packed bit 1) ≠ bit0. Mapping to bit0 would be
    // silent-wrong → reject.
    let (_, err, code) = run("module t;\n\
         reg [1:0] clk; integer cnt=0;\n\
         always @(posedge clk[1]) cnt=cnt+1;\n\
         initial begin clk=0; #1 clk=2'b11; #1 $finish; end\n\
         endmodule\n");
    assert_rejected(&err, code);
}

#[test]
fn ascending_non_lsb_bitselect_rejected() {
    // reg[0:1] ascending → clk[0] is the MSB (iverilog probe B: cnt=1). Mapping to
    // bit0 would give cnt=2 (silent-wrong) → reject.
    let (_, err, code) = run("module t;\n\
         reg [0:1] clk; integer cnt=0;\n\
         always @(posedge clk[0]) cnt=cnt+1;\n\
         initial begin clk=0; #1 clk=2'b01; #1 clk=2'b00; #1 clk=2'b11; #1 clk=2'b10;\n\
           #1 $finish; end\n\
         endmodule\n");
    assert_rejected(&err, code);
}

// ── loud reject: forms the engine cannot represent ────────────────────────

#[test]
fn variable_index_event_control_rejected() {
    // `clk[i]` (non-const index) — iverilog probe F accepts it (runtime bit), the
    // engine has no per-index edge tracking.
    let (_, err, code) = run("module t;\n\
         reg [1:0] clk; integer i; integer cnt=0;\n\
         initial i=0;\n\
         always @(posedge clk[i]) cnt=cnt+1;\n\
         initial begin clk=0; #1 clk=2'b01; #1 $finish; end\n\
         endmodule\n");
    assert_rejected(&err, code);
}

#[test]
fn part_select_event_control_rejected() {
    // `clk[1:0]` part-select — iverilog probe E fires on slice redor; no slice-edge
    // model here.
    let (_, err, code) = run("module t;\n\
         reg [3:0] clk; integer cnt=0;\n\
         always @(posedge clk[1:0]) cnt=cnt+1;\n\
         initial begin clk=0; #1 clk=4'b0001; #1 $finish; end\n\
         endmodule\n");
    assert_rejected(&err, code);
}

#[test]
fn array_element_event_control_rejected() {
    // `mem[1]` is an 8-bit array-element word, not a scalar net bit.
    let (_, err, code) = run("module t;\n\
         reg [7:0] mem [0:3]; integer cnt=0;\n\
         always @(posedge mem[1]) cnt=cnt+1;\n\
         initial begin mem[1]=0; #1 mem[1]=8'h01; #1 $finish; end\n\
         endmodule\n");
    assert_rejected(&err, code);
}

#[test]
fn multidim_packed_select_event_control_rejected() {
    // `m[1]` on `reg [1:0][7:0] m` selects an 8-bit word, not a scalar bit.
    let (_, err, code) = run("module t;\n\
         reg [1:0][7:0] m; integer cnt=0;\n\
         always @(posedge m[1]) cnt=cnt+1;\n\
         initial begin m=0; #1 m[1]=8'h01; #1 $finish; end\n\
         endmodule\n");
    assert_rejected(&err, code);
}

#[test]
fn computed_base_event_control_rejected() {
    // Non-Ident base `(a&b)[0]` — was previously warn+poison+panic.
    let (_, err, code) = run("module t;\n\
         reg [1:0] a,b; integer cnt=0;\n\
         always @(posedge (a&b)[0]) cnt=cnt+1;\n\
         initial begin a=0;b=0; #1 a=2'b11; b=2'b11; #1 $finish; end\n\
         endmodule\n");
    assert_rejected(&err, code);
}

#[test]
fn level_bitselect_rejected() {
    // LEVEL sensitivity `@(clk[0])` (no posedge/negedge): the engine's LEVEL
    // detection is WHOLE-NET (fires on any bit change), so mapping the bit-select
    // to the net would over-trigger — iverilog fires only on clk[0] changes
    // (discriminating stimulus: bit0 constant, bit1 toggles → iverilog cnt=1,
    // whole-net would give cnt=4). Single-bit level sensitivity is unrepresentable
    // → reject loud rather than silent-wrong.
    let (_, err, code) = run("module t;\n\
         reg [1:0] clk; integer cnt=0;\n\
         always @(clk[0]) cnt=cnt+1;\n\
         initial begin clk=0; #1 clk=2'b10; #1 clk=2'b00; #1 clk=2'b10; #1 $finish; end\n\
         endmodule\n");
    assert_rejected(&err, code);
}

#[test]
fn inbody_edge_lsb_bitselect_supported() {
    // In-body `@(posedge clk[0])` (WaitCause::Edge) also uses bit0 detection, so
    // the LSB bit-select is exact there too. iverilog: cnt=2.
    let (out, err, code) = run("module t;\n\
         reg [1:0] clk; integer cnt=0;\n\
         initial begin clk=0; #1 clk=2'b01; #1 clk=2'b00; #1 clk=2'b11; #2 $finish; end\n\
         initial begin\n\
           @(posedge clk[0]); cnt=cnt+1;\n\
           @(posedge clk[0]); cnt=cnt+1;\n\
           $display(\"IB cnt=%0d\",cnt);\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("IB cnt=2"), "got:\n{out}");
}
