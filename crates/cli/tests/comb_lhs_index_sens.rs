//! §9.2.2.4 implicit sensitivity (`always_comb` / `@(*)`): a variable that
//! appears ONLY as the dynamic INDEX of an LHS bit/part/word-select is a READ,
//! so the block must re-evaluate when it changes. vita previously inferred the
//! comb read-set from RHS + branch conditions only, dropping LHS index reads —
//! a block whose sole read of `idx` was `mask[idx*8 +: 8] = v` never re-fired,
//! leaving a silently stale value (the V10 report silent-wrong, §4.5.166;
//! observed as a wrong SHA-3 pad byte). iverilog 13.0-pinned oracle.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run vita on `src`; return stdout.
fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cls_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn indexed_partselect_lhs_index_is_sensitive() {
    // The V10 core: `mask[idx*8 +: 8]` — idx read only as the LHS offset.
    // iverilog: idx=0→000000ab, idx=1→0000ab00, idx=2→00ab0000.
    let out = run("module t;\n  logic [3:0] idx; logic [31:0] mask;\n\
         always_comb begin mask='0; mask[idx*8 +: 8] = 8'hAB; end\n\
         initial begin\n\
           idx=0; #1; $display(\"%h\", mask);\n\
           idx=1; #1; $display(\"%h\", mask);\n\
           idx=2; #1; $display(\"%h\", mask);\n\
           #1 $finish; end\nendmodule\n");
    assert!(out.contains("000000ab"), "idx=0 stale:\n{out}");
    assert!(out.contains("0000ab00"), "idx=1 did not re-fire:\n{out}");
    assert!(out.contains("00ab0000"), "idx=2 did not re-fire:\n{out}");
}

#[test]
fn bitselect_lhs_index_is_sensitive() {
    // `r[idx] = 1'b1` — idx read only as the LHS bit-select index.
    let out = run("module t;\n  logic [2:0] idx; logic [7:0] r;\n\
         always_comb begin r='0; r[idx] = 1'b1; end\n\
         initial begin\n\
           idx=0; #1; $display(\"%b\", r);\n\
           idx=3; #1; $display(\"%b\", r);\n\
           idx=7; #1; $display(\"%b\", r);\n\
           #1 $finish; end\nendmodule\n");
    assert!(out.contains("00000001"), "idx=0:\n{out}");
    assert!(out.contains("00001000"), "idx=3 did not re-fire:\n{out}");
    assert!(out.contains("10000000"), "idx=7 did not re-fire:\n{out}");
}

#[test]
fn array_word_lhs_index_is_sensitive() {
    // `mem[sel] = 8'hAB` — sel read only as the LHS array WORD index.
    let out = run("module t;\n  logic [1:0] sel; logic [7:0] mem [0:3];\n\
         always_comb begin mem[0]=0; mem[1]=0; mem[2]=0; mem[3]=0; mem[sel]=8'hAB; end\n\
         initial begin\n\
           sel=0; #1; $display(\"%h %h\", mem[0], mem[2]);\n\
           sel=2; #1; $display(\"%h %h\", mem[0], mem[2]);\n\
           #1 $finish; end\nendmodule\n");
    assert!(out.contains("ab 00"), "sel=0:\n{out}");
    assert!(out.contains("00 ab"), "sel=2 did not re-fire:\n{out}");
}

#[test]
fn explicit_star_lhs_index_is_sensitive() {
    // The same read-set inference drives explicit `always @(*)`.
    let out = run("module t;\n  logic [1:0] idx; logic [31:0] mask;\n\
         always @(*) begin mask='0; mask[idx*8 +: 8] = 8'h77; end\n\
         initial begin\n\
           idx=0; #1; $display(\"%h\", mask);\n\
           idx=2; #1; $display(\"%h\", mask);\n\
           #1 $finish; end\nendmodule\n");
    assert!(out.contains("00000077"), "idx=0:\n{out}");
    assert!(out.contains("00770000"), "idx=2 did not re-fire:\n{out}");
}

#[test]
fn latch_lhs_index_is_sensitive() {
    // `always_latch` shares the same inferred read-set path.
    let out = run(
        "module t;\n  logic [1:0] idx; logic en; logic [31:0] mask;\n\
         always_latch if (en) mask[idx*8 +: 8] = 8'h99;\n\
         initial begin\n\
           mask='0; en=1;\n\
           idx=0; #1; $display(\"%h\", mask);\n\
           idx=1; #1; $display(\"%h\", mask);\n\
           #1 $finish; end\nendmodule\n",
    );
    // A latch RETAINS byte0, so idx=1 re-firing yields both bytes set
    // (00009999) — the stale-bug value would be the unchanged 00000099.
    assert!(out.contains("00000099"), "idx=0:\n{out}");
    assert!(out.contains("00009999"), "idx=1 did not re-fire:\n{out}");
}

// ── §4.5.166 HIER twin: a hierarchical indexed read/write in `always_comb`
// (`y=dut.mem[idx]` / `dut.mem[idx]=v`) lowers behind a deferred sentinel, so
// its index was invisible to the read-set inference at lowering time. The
// post-hier-resolution recompute pass restores it.

#[test]
fn hier_indexed_write_from_comb_is_sensitive() {
    let out = run("module sub; logic [7:0] mem [0:3]; endmodule\n\
         module t;\n  sub u(); logic [1:0] idx;\n\
         always_comb begin u.mem[0]=0; u.mem[1]=0; u.mem[2]=0; u.mem[3]=0; u.mem[idx]=8'hAB; end\n\
         initial begin\n\
           idx=0; #1; $display(\"%h %h\", u.mem[0], u.mem[2]);\n\
           idx=2; #1; $display(\"%h %h\", u.mem[0], u.mem[2]);\n\
           #1 $finish; end\nendmodule\n");
    assert!(out.contains("ab 00"), "idx=0:\n{out}");
    assert!(
        out.contains("00 ab"),
        "idx=2 did not re-fire (hier write stale):\n{out}"
    );
}

#[test]
fn hier_indexed_read_from_comb_is_sensitive() {
    let out = run("module sub; logic [7:0] mem [0:3]; endmodule\n\
         module t;\n  sub u(); logic [1:0] idx; logic [7:0] y;\n\
         always_comb y = u.mem[idx];\n\
         initial begin\n\
           u.mem[0]=8'h10; u.mem[1]=8'h11; u.mem[2]=8'h12; u.mem[3]=8'h13;\n\
           idx=0; #1; $display(\"y=%h\", y);\n\
           idx=3; #1; $display(\"y=%h\", y);\n\
           #1 $finish; end\nendmodule\n");
    assert!(out.contains("y=10"), "idx=0:\n{out}");
    assert!(
        out.contains("y=13"),
        "idx=3 did not re-fire (hier read stale):\n{out}"
    );
}

#[test]
fn self_timed_clock_survives_hier_recompute() {
    // The recompute pass (triggered by the hier-sel write) must NOT widen a bare
    // self-timed `always #5 clk=~clk` into a data-sensitive block — clocks stay
    // clocks. 12 posedges over ~123 ns; `tap` reads the hier mem element.
    let out = run("module sub; logic [7:0] mem [0:3]; endmodule\n\
         module t;\n  sub u(); logic clk; logic [1:0] idx; logic [7:0] tap; int ticks;\n\
         always #5 clk = ~clk;\n\
         always_comb u.mem[idx] = 8'hAB;\n\
         always_comb tap = u.mem[2];\n\
         always @(posedge clk) ticks++;\n\
         initial begin clk=0; ticks=0; idx=0;\n\
           #23 idx=2;\n\
           #100 $display(\"ticks=%0d tap=%h\", ticks, tap); $finish; end\nendmodule\n");
    assert!(
        out.contains("ticks=12 tap=ab"),
        "clock widened or hier tap wrong:\n{out}"
    );
}

#[test]
fn whole_net_hier_read_from_comb_is_sensitive() {
    // §4.5.166 whole-net twin: `always_comb y = dut.q` — the sole read is a
    // whole-net hierarchical ref (no index). It lowers to a POISON placeholder
    // patched only after elaboration, so ALL FOUR hier deferral lanes must arm
    // the recompute; otherwise it stays stale and its correctness would depend
    // on the coincidental presence of an unrelated indexed hier ref elsewhere.
    let out = run("module sub; logic [7:0] q; endmodule\n\
         module t;\n  sub u(); logic [7:0] y;\n\
         always_comb y = u.q;\n\
         initial begin\n\
           u.q=8'h10; #1; $display(\"y=%h\", y);\n\
           u.q=8'h20; #1; $display(\"y=%h\", y);\n\
           #1 $finish; end\nendmodule\n");
    assert!(out.contains("y=10"), "q=10:\n{out}");
    assert!(
        out.contains("y=20"),
        "q change did not re-fire (whole-net hier read stale):\n{out}"
    );
}
