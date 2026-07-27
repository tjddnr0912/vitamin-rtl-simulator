//! Packed ranges declared with a NEGATIVE low bound (`logic [3:-2] x`, IEEE §7.4.1).
//!
//! Two different failures, both closed here:
//!
//! - a PLAIN net was warn-and-clamped to width 1 (whole-value damage — loud, but the
//!   value was destroyed);
//! - a MULTI-PACKED inner dim (`logic [1:0][3:-2]`) was clamped SILENTLY: the inner dim
//!   came out 4 bits instead of 6, so the vector was 8 bits instead of 12, at exit 0 with
//!   no diagnostic. The clamp warning that made this look loud came from a sibling
//!   declaration, never from the packed path.
//!
//! `NetVar.msb`/`lsb` are frozen `u32`, so a negative-bound net is stored NORMALIZED as
//! `[w-1:0]` and the declared bound rides `net_decl_neg_lsb` (plain net) /
//! `packed_dims` (multi-packed, whose low bound is `i64` now). `norm_offset_for_net` and
//! `norm_offset_for_range` consult those, so a bit select addresses the DECLARED
//! numbering. No format bump: nothing serialized changed shape.
//!
//! ORACLE: iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("vita_npb_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.success(),
    )
}

fn ok(src: &str) -> String {
    let (o, s) = run(src);
    assert!(s, "expected success:\n{o}");
    assert!(!o.contains("W3056"), "clamp warning survived:\n{o}");
    o
}

#[test]
fn a_plain_net_is_sized_and_bit_selectable() {
    let o = ok("module t; logic [3:-2] x;\n\
        initial begin x = 6'b101010;\n\
          $display(\"%b w=%0d b3=%b b2=%b b1=%b b0=%b bm1=%b bm2=%b\",\n\
                   x, $bits(x), x[3], x[2], x[1], x[0], x[-1], x[-2]);\n\
        end endmodule\n");
    // iverilog: 101010 w=6 b3=1 b2=0 b1=1 b0=0 bm1=1 bm2=0
    assert!(
        o.contains("101010 w=6 b3=1 b2=0 b1=1 b0=0 bm1=1 bm2=0"),
        "{o}"
    );
}

#[test]
fn array_query_functions_report_the_declared_bounds() {
    let o = ok("module t; logic [3:-2] x;\n\
        initial $display(\"L=%0d R=%0d H=%0d LO=%0d\",\n\
                         $left(x), $right(x), $high(x), $low(x));\n\
        endmodule\n");
    assert!(o.contains("L=3 R=-2 H=3 LO=-2"), "{o}");
}

// ── the SILENT one: a multi-packed inner negative dim ──
#[test]
fn a_multi_packed_inner_negative_dim_is_sized_correctly() {
    let o = ok("module t; logic [1:0][3:-2] mp;\n\
        initial begin mp = 12'b101010110011;\n\
          $display(\"%b w=%0d e1=%b e0=%b e1b3=%b e0bm2=%b\",\n\
                   mp, $bits(mp), mp[1], mp[0], mp[1][3], mp[0][-2]);\n\
        end endmodule\n");
    // iverilog: 101010110011 w=12 e1=101010 e0=110011 e1b3=1 e0bm2=1
    // (was a silent w=8 / e1=1011 / e0bm2=x)
    assert!(
        o.contains("101010110011 w=12 e1=101010 e0=110011 e1b3=1 e0bm2=1"),
        "{o}"
    );
}

#[test]
fn an_array_of_negative_bound_words() {
    let o = ok("module t; logic [3:-2] m[2];\n\
        initial begin m[0] = 6'b111000; m[1] = 6'b000111;\n\
          $display(\"m0=%b m1=%b m0b3=%b m1bm2=%b\", m[0], m[1], m[0][3], m[1][-2]);\n\
        end endmodule\n");
    assert!(o.contains("m0=111000 m1=000111 m0b3=1 m1bm2=1"), "{o}");
}

#[test]
fn concat_and_arithmetic_use_the_full_width() {
    let o = ok("module t; logic [3:-2] a, b; logic [11:0] c;\n\
        initial begin a = 6'b111000; b = 6'b000111; c = {a, b};\n\
          $display(\"c=%b sum=%0d and=%b\", c, a+b, a & b);\n\
        end endmodule\n");
    assert!(o.contains("c=111000000111 sum=63 and=000000"), "{o}");
}

// ── a PARAMETER whose low bound folds negative gets the same treatment ──
#[test]
fn a_param_folded_negative_low_bound() {
    let o = ok("module t; parameter W = 1; logic [7:W-3] y;\n\
        initial begin y = '1; $display(\"%b w=%0d\", y, $bits(y)); end\n\
        endmodule\n");
    assert!(o.contains("1111111111 w=10"), "{o}"); // iverilog: 10 bits
}

// ── PRESERVED: the degenerate `[W-1:0]` with W == 0 is a parameter UNDERFLOW, not a
// declared negative low bound. It keeps its own message and its graceful width-1.
#[test]
fn the_param_zero_width_underflow_is_untouched() {
    let (o, _) = run("module t; parameter W=0; logic [W-1:0] x;\n\
        initial $display(\"w=%0d\", $bits(x)); endmodule\n");
    assert!(o.contains("parameterized range underflowed"), "{o}");
    assert!(o.contains("w=1"), "{o}");
}

// ── STILL LOUD, with the real reason: a PART select folds its own bounds through the
// unsigned const path, where `-2` reads as 0xFFFFFFFE.
#[test]
fn a_part_select_is_loud_naming_the_real_reason() {
    let (o, s) = run("module t; logic [3:-2] x;\n\
        initial begin x = 6'b101010; $display(\"%b\", x[1:-2]); end endmodule\n");
    assert!(!s, "must stay loud:\n{o}");
    assert!(
        o.contains("PART select of a net declared with a negative low bound"),
        "must name the real reason, not a direction mismatch:\n{o}"
    );
}

// ── STILL LOUD (documented asymmetry): only a plain net DECLARATION opts in. A PORT
// with a negative bound keeps the warn-and-clamp, because the width and the select
// normalization have to be enabled together and the port path records no side map.
#[test]
fn a_port_with_a_negative_bound_stays_clamped_and_loud() {
    let (o, _) = run("module sub(input logic [3:-2] p, output logic [3:-2] q);\n\
          assign q = ~p;\n\
        endmodule\n\
        module t; logic [3:-2] a, y; sub u(.p(a), .q(y));\n\
          initial begin a = 6'b101010; #1 $display(\"y=%b\", y); end\n\
        endmodule\n");
    assert!(o.contains("W3056"), "port path must stay loud:\n{o}");
}

// ── a pathological declaration PANICKED (`max - min` overflowed i64) where iverilog
// asserts. A crash is below loud on the ladder; checked + capped now.
#[test]
fn a_pathological_span_is_loud_not_a_panic() {
    let (o, s) = run("module t;\n\
        string s[9223372036854775807:-9223372036854775807];\n\
        initial $display(\"hi\"); endmodule\n");
    assert!(!s, "must be loud:\n{o}");
    assert!(!o.contains("panicked"), "must not panic:\n{o}");
    assert!(o.contains("element cap"), "{o}");
}

// ── the VCD `$var` line must carry the DECLARED range, not the normalized storage one:
// same bits either way, but a waveform viewer labels every bit from this. Rides the
// `net_decl_ranges` sidecar so the STAGED path agrees. iverilog: `x [3:-2]`.
#[test]
fn the_vcd_var_line_uses_the_declared_range() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_npbv_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        "module t;\n\
           logic [3:-2] x;\n\
           logic [1:0][3:-2] mp;\n\
           initial begin $dumpfile(\"w.vcd\"); $dumpvars(0, t);\n\
             x = 6'b101010; mp = 12'b111000000111; #1 $finish; end\n\
         endmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let vcd = std::fs::read_to_string(d.join("w.vcd")).unwrap_or_default();
    let vars: Vec<&str> = vcd.lines().filter(|l| l.starts_with("$var")).collect();
    assert!(
        vars.iter().any(|l| l.contains("x [3:-2]")),
        "declared range missing (run: {}{})\n{vars:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // A multi-packed vector still flattens to `[11:0]`, exactly as iverilog dumps it.
    assert!(vars.iter().any(|l| l.contains("mp [11:0]")), "{vars:?}");
}
