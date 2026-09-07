//! A bare name whose INNERMOST binding is a constant — ROADMAP §2 🆕 O.
//!
//! `generate if (1) begin : g localparam int ROTA = 99; … $display(ROTA, ROTA[1])`
//! over an outer `logic [31:0] ROTA[0:3]` printed `whole=99 sel=20`: the whole-name
//! read was right (the lowering routes it through `bare_ident_route`) and the SELECT
//! read the outer array's element, because the index chains resolve their base with
//! `lookup_net_scoped`, a `symbols`-only walk that a `params` binding does not stop.
//! Both oracles print `whole=99 sel=1` — bit 1 of the inner constant.
//!
//! Five readers shared that walk: the unpacked read chain, its packed twin, both
//! WRITE chains — where the assignment silently stored into the outer array at exit
//! 0 while both oracles reject the program — and the `$size` introspection lane.
//!
//! Values are pinned to iverilog 13.0 (`-g2012`) and verilator 5.052
//! (`--binary --timing`); each test says which cells are refusals in both.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_shcr_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut all = String::from_utf8_lossy(&out.stdout).into_owned();
    all.push_str(&String::from_utf8_lossy(&out.stderr));
    let mut s = String::new();
    for l in all.lines().filter(|l| {
        !l.starts_with("simulation ended")
            && !l.starts_with("errors=")
            && !l.contains("W-PP-TIMESCALE-DEFAULT")
    }) {
        s.push_str(l);
        s.push('\n');
    }
    (s, out.status.success())
}

fn run(src: &str) -> String {
    let (s, ok) = vita(src);
    assert!(ok, "expected exit 0, got:\n{s}");
    s
}

fn loud(src: &str, needle: &str) {
    let (s, ok) = vita(src);
    assert!(!ok, "expected a loud reject, got exit 0:\n{s}");
    assert!(s.contains(needle), "expected `{needle}` in:\n{s}");
}

/// The headline: one `$display`, two reads of one name, and they must agree.
#[test]
fn a_select_reads_the_same_object_the_whole_name_does() {
    let out = run("module top;\n\
           logic [31:0] ROTA[0:3];\n\
           generate if (1) begin : g\n\
             localparam int ROTA = 99;\n\
             initial begin\n\
               top.ROTA[1] = 32'd20;\n\
               #1;\n\
               $display(\"whole=%0d sel=%0d\", ROTA, ROTA[1]);\n\
               $finish;\n\
             end\n\
           end endgenerate\n\
           initial #100 $finish;\n\
         endmodule\n");
    // Both oracles: whole=99 sel=1 (bit 1 of 99 = 0b1100011).
    assert_eq!(out, "whole=99 sel=1\n");
}

/// The same shape with a PACKED outer array — the second chain.
#[test]
fn the_packed_chain_takes_the_same_answer() {
    let out = run("module top;\n\
           logic [3:0][31:0] PK;\n\
           generate if (1) begin : g\n\
             localparam int PK = 99;\n\
             initial begin\n\
               top.PK[1] = 32'd20;\n\
               #1;\n\
               $display(\"whole=%0d sel=%0d\", PK, PK[1]);\n\
               $finish;\n\
             end\n\
           end endgenerate\n\
           initial #100 $finish;\n\
         endmodule\n");
    // Both oracles: whole=99 sel=1.
    assert_eq!(out, "whole=99 sel=1\n");
}

/// The controls: no shadow at all, and a shadow that is a NET rather than a
/// constant. Both were already right and must not move — the class is exactly
/// "the innermost binding is in `params`".
#[test]
fn an_unshadowed_read_and_a_net_shadow_are_unchanged() {
    let none = run("module top;\n\
           logic [31:0] ROTA[0:3];\n\
           generate if (1) begin : g\n\
             initial begin top.ROTA[1] = 32'd20; #1; $display(\"sel=%0d\", ROTA[1]); $finish; end\n\
           end endgenerate\n\
           initial #100 $finish;\n\
         endmodule\n");
    assert_eq!(none, "sel=20\n");

    let net_shadow = run("module top;\n\
           logic [31:0] ROTA[0:3];\n\
           generate if (1) begin : g\n\
             initial begin\n\
               logic [31:0] ROTA;\n\
               top.ROTA[1] = 32'd20;\n\
               ROTA = 99;\n\
               #1;\n\
               $display(\"sel=%0d\", ROTA[1]);\n\
               $finish;\n\
             end\n\
           end endgenerate\n\
           initial #100 $finish;\n\
         endmodule\n");
    // Both oracles: sel=1 — a block-local NET shadow was already resolved correctly,
    // which is what makes the guard's predicate "binds to a constant", not "a name
    // that also exists outside".
    assert_eq!(net_shadow, "sel=1\n");
}

/// A WRITE through the shadowed name. It stored into the OUTER object at exit 0;
/// both oracles reject the program (iverilog "Could not find variable ``ROTA['sd1]''
/// in ``top.g''", verilator "Storing to parameter variable 'ROTA'").
#[test]
fn a_write_through_a_constant_binding_is_loud() {
    let needle = "resolves to a constant";
    // element write
    loud(
        "module top;\n\
           logic [31:0] ROTA[0:3];\n\
           generate if (1) begin : g\n\
             localparam int ROTA = 99;\n\
             initial begin ROTA[1] = 32'd77; $display(\"x\"); $finish; end\n\
           end endgenerate\n\
           initial #100 $finish;\n\
         endmodule\n",
        needle,
    );
    // bit write on a shadowed SCALAR net — the same funnel, and the shape that shows
    // the class is not about arrays.
    loud(
        "module top;\n\
           logic [31:0] SC;\n\
           generate if (1) begin : g\n\
             localparam int SC = 99;\n\
             initial begin SC[1] = 1'b1; $display(\"x\"); $finish; end\n\
           end endgenerate\n\
           initial #100 $finish;\n\
         endmodule\n",
        needle,
    );
    // whole-name write
    loud(
        "module top;\n\
           logic [31:0] SC;\n\
           generate if (1) begin : g\n\
             localparam int SC = 99;\n\
             initial begin SC = 32'd5; $display(\"x\"); $finish; end\n\
           end endgenerate\n\
           initial #100 $finish;\n\
         endmodule\n",
        needle,
    );
}

/// The introspection lane. `$size` answered the OUTER array's 4 where both oracles
/// answer the inner constant's 32; it now declines, which lands on the pre-existing
/// (separately tracked) "unsupported system function" the UNSHADOWED spelling of the
/// same query already gives — silent-wrong → loud, and the five readers agree again.
#[test]
fn the_size_query_no_longer_answers_the_outer_array() {
    loud(
        "module top;\n\
           logic [31:0] ROTA[0:3];\n\
           generate if (1) begin : g\n\
             localparam int ROTA = 99;\n\
             initial begin #1; $display(\"size=%0d\", $size(ROTA)); $finish; end\n\
           end endgenerate\n\
           initial #100 $finish;\n\
         endmodule\n",
        "unsupported system function",
    );
}
