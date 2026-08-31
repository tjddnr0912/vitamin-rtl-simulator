//! Two gaps that stood between vita and `verilog-axi`'s crossbar, found by
//! censusing the design rather than by reading the queue line — which said the
//! blocker was "54 errors" and "a wide accumulator" and was wrong on both counts
//! (4 errors, one expression, and the return value is exactly 64 bits).
//!
//! **1. A const function could not write a part-select of its own return value.**
//! `exec_const_stmt`'s assignment arm took `Lvalue::Ident` and declined everything
//! else, so `calcBaseAddrs[i*ADDR_WIDTH +: ADDR_WIDTH] = base` — how the crossbar
//! builds its whole base-address vector — had no fold. Neither the width nor the
//! runtime index was the obstacle; a CONSTANT offset failed too.
//!
//! **2. An untyped `localparam` initialised by a CONDITIONAL took its width from
//! the folded value.** §11.4.11 makes the result as wide as the wider arm, so
//! `Z ? Z : 64'h0100000000000000` is 64 bits — vita recorded 58 (its magnitude),
//! and the parameter's top bits then read `x`. That one is a pre-existing
//! silent-wrong and reproduces with no function call anywhere.
//!
//! Every value here is iverilog 13.0's.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cfsel_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.v");
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

fn expect(src: &str, want: &str) {
    let (o, ok) = run(src);
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains(want), "want `{want}` in:\n{o}");
}

/// A CONSTANT-offset part-select write to the return variable. This is the rung
/// that actually failed first — before the loop, before the runtime index.
#[test]
fn a_constant_offset_part_select_write_folds() {
    expect(
        "module m;\n\
           function [63:0] f(input [31:0] d);\n\
             reg [31:0] b;\n\
             begin f = 0; b = 5; f[0 +: 32] = b; end\n\
           endfunction\n\
           localparam P = f(0);\n\
           initial $display(\"P=%0d\", P);\n\
         endmodule\n",
        "P=5",
    );
}

/// A RUNTIME offset built from the loop variable — the accumulator shape.
#[test]
fn a_loop_indexed_part_select_write_folds() {
    expect(
        "module m;\n\
           function [63:0] f(input [31:0] d);\n\
             integer i; reg [31:0] b;\n\
             begin f = 0; b = 5;\n\
               for (i = 0; i < 2; i = i + 1) f[i*32 +: 32] = b + i;\n\
             end\n\
           endfunction\n\
           localparam P = f(0);\n\
           initial $display(\"P=%0h\", P);\n\
         endmodule\n",
        "P=600000005",
    );
}

/// A parameter READ whose index mentions a function local. The environment had
/// to reach the select fold for this; before, the value walk had no select arm
/// at all and fell through to a module-scope delegate that fires only on an
/// EMPTY environment — i.e. never inside a function body.
#[test]
fn a_parameter_select_indexed_by_a_local_folds() {
    expect(
        "module m;\n\
           parameter [63:0] W = {32'd24, 32'd25};\n\
           function [63:0] f(input [31:0] d);\n\
             integer i; reg [31:0] b;\n\
             begin f = 0; i = 1; b = W[i*32 +: 32]; f = b; end\n\
           endfunction\n\
           localparam P = f(0);\n\
           initial $display(\"P=%0d\", P);\n\
         endmodule\n",
        "P=24",
    );
}

/// `calcBaseAddrs` itself, at the crossbar's own parameters — read, write, loop
/// and shift together.
#[test]
fn the_crossbar_base_address_function_folds_to_the_oracle_value() {
    expect(
        "module m;\n\
           localparam M_COUNT = 2, M_REGIONS = 1, ADDR_WIDTH = 32;\n\
           localparam M_ADDR_WIDTH = {M_COUNT{{M_REGIONS{32'd24}}}};\n\
           function [M_COUNT*M_REGIONS*ADDR_WIDTH-1:0] cba(input [31:0] d);\n\
             integer i; reg [ADDR_WIDTH-1:0] base, width, size, mask;\n\
             begin cba = 0; base = 0;\n\
               for (i = 0; i < M_COUNT*M_REGIONS; i = i + 1) begin\n\
                 width = M_ADDR_WIDTH[i*32 +: 32];\n\
                 mask = {ADDR_WIDTH{1'b1}} >> (ADDR_WIDTH - width);\n\
                 size = mask + 1;\n\
                 if (width > 0) begin\n\
                   if ((base & mask) != 0) base = base + size - (base & mask);\n\
                   cba[i*ADDR_WIDTH +: ADDR_WIDTH] = base;\n\
                   base = base + size;\n\
                 end\n\
               end\n\
             end\n\
           endfunction\n\
           localparam [63:0] R = cba(0);\n\
           initial $display(\"R=%h\", R);\n\
         endmodule\n",
        "R=0100000000000000",
    );
}

/// ⭐ THE PRE-EXISTING SILENT-WRONG, isolated: no function call anywhere. An
/// untyped parameter initialised by a conditional is as wide as its wider arm
/// (§11.4.11), not as wide as the value's magnitude.
#[test]
fn an_untyped_param_from_a_conditional_is_as_wide_as_its_wider_arm() {
    expect(
        "module m;\n\
           localparam Z = 0;\n\
           localparam T = Z ? Z : 64'h0100000000000000;\n\
           initial $display(\"b=%0d T=%h\", $bits(T), T);\n\
         endmodule\n",
        "b=64 T=0100000000000000",
    );
}

/// And the shape the crossbar actually writes — the conditional's else arm is
/// the const-function call, and a select of the result must reach the top bits.
#[test]
fn a_select_of_a_conditional_initialised_param_reaches_the_top_bits() {
    expect(
        "module m;\n\
           localparam Z = 0;\n\
           function [63:0] f(input [31:0] d); begin f = 64'h0100000000000000; end endfunction\n\
           localparam T = Z ? Z : f(0);\n\
           initial $display(\"lo=%h hi=%h\", T[0 +: 32], T[32 +: 32]);\n\
         endmodule\n",
        "lo=00000000 hi=01000000",
    );
}

/// A `[msb:lsb]` lvalue stays LOUD, deliberately: §11.5.1 reads that pair in the
/// base's declared DIRECTION, and the const-function width table records width
/// and signedness but not direction. Wording pin — there is no value to assert.
#[test]
fn a_directed_range_lvalue_stays_loud() {
    let (o, ok) = run("module m;\n\
           function [63:0] f(input [31:0] d);\n\
             reg [31:0] b;\n\
             begin f = 0; b = 5; f[31:0] = b; end\n\
           endfunction\n\
           localparam P = f(0);\n\
           initial $display(\"P=%0d\", P);\n\
         endmodule\n");
    assert!(!ok, "expected a refusal, got:\n{o}");
}
