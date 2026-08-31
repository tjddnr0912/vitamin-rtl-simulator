//! `always @*` has NO implicit execution at time zero (IEEE 1800 §9.2.2.2).
//!
//! That paragraph gives the implicit time-zero run to `always_comb` and
//! `always_latch`. A plain `always @*` waits for its inferred read set to
//! change, exactly as `always @(a or b)` does — so if nothing it reads ever
//! changes, it never runs and its targets keep their initial `x`.
//!
//! vita ran it at time zero, which turned an `x` into a definite value. Found
//! by censusing why `verilog-axi`'s digest differed from iverilog's: the
//! crossbar's register slices compute `m_axi_awvalid_next` in an `always @*`,
//! and vita's time-zero run made the whole write path definite where the oracle
//! has `x` for the first cycles after reset (ROADMAP §2-N).
//!
//! Fixed by mapping `always @*` to `SensKind::Level` rather than `Comb`. The
//! scheduler already registers Level, Comb and Latch with the same level waiter
//! over the same inferred read set, so the ONLY thing that changes is the
//! time-zero arm — no IR shape moves and `format_version` stays 29.
//!
//! Values pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_star_{}_{n}", std::process::id()));
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
        out.status.success(),
    )
}

/// THE THREE-LINE REPRO. Nothing ever changes `a`, so the `@*` block must never
/// run and `star_out` stays `x`; `always_comb` runs once at time zero and gives
/// `0`. Both answers on one line so neither can be fixed by breaking the other.
#[test]
fn always_star_does_not_self_start_but_always_comb_does() {
    let (o, ok) = run("module m;\n\
           reg a = 1'b0;\n\
           reg star_out;\n\
           reg comb_out;\n\
           always @*   star_out = a;\n\
           always_comb comb_out = a;\n\
           initial begin #1 $display(\"star=%b comb=%b\", star_out, comb_out); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("star=x comb=0"), "got:\n{o}");
}

/// The shape it actually broke: a register slice whose `always @*` feeds a
/// sequential block. The `x` must reach the output and stay, cycle for cycle.
#[test]
fn an_uncomputed_next_value_propagates_x_through_the_register() {
    let (o, ok) = run("module m;\n\
           reg clk = 0, rst = 1; always #5 clk = ~clk;\n\
           reg vreg = 1'b0, rreg = 1'b0;\n\
           reg vnext;\n\
           wire early = !vnext;\n\
           always @* begin vnext = vreg; if (rreg) vnext = 1'b1; end\n\
           always @(posedge clk) begin\n\
             if (rst) begin rreg <= 1'b0; vreg <= 1'b0; end\n\
             else begin rreg <= early; vreg <= vnext; end\n\
           end\n\
           integer c = 0;\n\
           always @(posedge clk) begin c = c + 1;\n\
             if (c < 6) $display(\"Q c=%0d vreg=%b\", c, vreg);\n\
             if (c == 2) rst = 0; if (c > 6) $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    for want in [
        "Q c=1 vreg=0",
        "Q c=3 vreg=0",
        "Q c=4 vreg=x",
        "Q c=5 vreg=x",
    ] {
        assert!(o.contains(want), "missing `{want}` in:\n{o}");
    }
}

/// A self-timed `always` with in-body timing is NOT `@*` and must keep starting
/// at time zero — it is every design's clock generator. Guard against a future
/// widening of the change.
#[test]
fn a_self_timed_always_still_starts_at_time_zero() {
    let (o, ok) = run("module m;\n\
           reg clk = 0; integer n = 0;\n\
           always #5 clk = ~clk;\n\
           always @(posedge clk) n = n + 1;\n\
           initial begin #52 $display(\"n=%0d\", n); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("n=5"), "the clock must run:\n{o}");
}

/// A combinational UDP is a PRIMITIVE, so it has an output from time zero like a
/// gate — its desugar uses `always_comb`, not `always @*`. This is the shape the
/// fix broke first.
#[test]
fn a_combinational_udp_still_drives_from_time_zero() {
    let (o, ok) = run("primitive p_and (o, a, b);\n\
           output o; input a, b;\n\
           table 0 ? : 0; ? 0 : 0; 1 1 : 1; endtable\n\
         endprimitive\n\
         module m;\n\
           reg a = 1'b0, b = 1'b1; wire o;\n\
           p_and u (o, a, b);\n\
           initial begin #1 $display(\"udp=%b\", o); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("udp=0"), "got:\n{o}");
}
