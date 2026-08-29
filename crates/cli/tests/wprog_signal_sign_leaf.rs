//! A `Signal` leaf declined the compiled backend whenever the context's signedness
//! differed from the net's declared signedness — at EQUAL width, where the bits are the
//! same either way.
//!
//! ## Why this is the narrow half of a reverted change
//!
//! ⚠️⚠️ `wprog`'s module header records that dropping the sign half of its admission gate
//! for EVERY node was built, measured **sound**, measured **1.00×**, and reverted. That
//! measurement was right; its conclusion was scoped to **picorv32 and keccak**, which is
//! what the header says. A fresh execution-weighted census puts
//! **6,600,872 requests from exactly TWO expressions** on this gate in `darkriscv` — a
//! design that was not in the original pair, and 92% of its declines.
//!
//! ⭐ So only the LEAF exemption is taken, and its argument is stronger than the `Const`
//! one already in the code: a `Signal` at equal width compiles to `Load { vi }`, two word
//! reads at a compile-time index, and that arm does not consult `signed` at all — not even
//! for a mask, because it requires `slot.width == w` and the arena's slot invariant
//! already keeps bits above the width zero. A `Const` at least has to mask.
//!
//! ## What it bought, and what it did not
//!
//! ```text
//!   picorv32 -3.4%   darkriscv -1.6%   biriscv -1.6%   serv -1.2%
//!   sha256 -0.9%     keccak -0.4%      aes / keccak-arr flat
//! ```
//!
//! every pinned corpus digest unchanged.
//!
//! ⚠️ Re-censusing after the change shows the gate did not disappear — it MOVED UP. On
//! darkriscv the 6.6M leaf declines became **6,425,888 requests from four `Ternary` nodes
//! failing the same `sw.signed != signed` test**, because the gate is per-node and a
//! parent inherits nothing from an admitted child. The remaining question is which
//! non-leaf kinds are also sign-inert at equal width, and that is the reverted set-wide
//! relaxation, which needs re-measuring on this corpus rather than on the old pair — and
//! needs the `AShr` guard rewritten from `signed` to `sw.signed && signed` first, because
//! dropping the entry gate is exactly what stops those two being the same question.
//!
//! ## What these cells pin
//!
//! The compiled and generic paths must agree, and the VM and interpreter never enter
//! `wprog` at all — so a mis-compiled leaf shows up as a backend split even without an
//! external tool. Every cell is also checked against iverilog.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn agrees_across_backends(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wsl_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let mut first: Option<String> = None;
    for be in ["native", "vm", "interp"] {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .args(["t.sv", "--backend", be])
            .current_dir(&d)
            .output()
            .expect("run vita");
        let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&out.stderr));
        match &first {
            None => first = Some(s),
            Some(f) => assert_eq!(f, &s, "backend {be} diverged"),
        }
    }
    let _ = std::fs::remove_dir_all(&d);
    first.unwrap()
}

/// ⭐ THE SHAPE THE CENSUS FOUND: a signed net read in an unsigned context and an unsigned
/// net read in a signed one, at equal width, under the operators that now compile the
/// leaf. If the leaf's bits were NOT sign-inert, these would move.
///
/// iverilog: `A=4294967295 B=4294967295 C=0 D=4294967295 E=1 F=15`.
#[test]
fn a_leaf_read_at_equal_width_is_sign_inert() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         reg signed [31:0] s; reg [31:0] u; reg [31:0] a, b, c, d, e, f;\n  \
         always @(*) begin\n    \
         a = s & 32'hFFFF_FFFF;\n    b = u | 32'd0;\n    c = s ^ u;\n    \
         d = ~(~s);\n    e = (s == u);\n    f = s[3:0];\n  end\n  \
         initial begin s = -32'sd1; u = 32'hFFFF_FFFF; #1\n    \
         $display(\"A=%0d B=%0d C=%0d D=%0d E=%0d F=%0d\", a, b, c, d, e, f);\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(
        out.contains("A=4294967295 B=4294967295 C=0 D=4294967295 E=1 F=15"),
        "{out}"
    );
}

/// ⚠️ THE OPERATORS THAT DO READ A SIGN must be unaffected. `>>>` fills from the sign bit
/// only when the result type is signed, and a comparison sizes its operands by their pair
/// signedness — neither takes its answer from the leaf's declared sign, and both have
/// their own guards.
///
/// `G` a signed value arithmetically shifted in a signed context: sign-fills.
/// `H` the same shift with an unsigned sibling: §11.4.10 makes it logical.
/// `I`/`J` ordered comparison, signed pair vs mixed.
/// `K` division, which reads the sign of both operands.
///
/// iverilog: `G=-2 H=2147483646 I=1 J=0 K=-2`. ⚠️ `H` is 2147483646, not ...47 — I
/// guessed and the oracle corrected me; `-4 >>> 1` is `-2`, whose low 31 bits read
/// unsigned are `7FFFFFFE`.
#[test]
fn the_sign_reading_operators_still_decide_for_themselves() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         reg signed [31:0] s; reg [31:0] u;\n  \
         reg signed [31:0] g, i2, k; reg [31:0] h, j;\n  \
         always @(*) begin\n    \
         g = s >>> 1;\n    h = (s >>> 1) + 32'd0;\n    \
         i2 = (s < 32'sd0);\n    j = (s < u);\n    k = s / 32'sd2;\n  end\n  \
         initial begin s = -32'sd4; u = 32'd1; #1\n    \
         $display(\"G=%0d H=%0d I=%0d J=%0d K=%0d\", g, h, i2, j, k);\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(
        out.contains("G=-2 H=2147483646 I=1 J=0 K=-2"),
        "the shift, the comparisons and the division keep their own sign rules:\n{out}"
    );
}

/// ⚠️ The exemption is for a `Signal` at EQUAL width only. A narrower net in a wider
/// context still goes through the widening admission, which extends by the VALUE's own
/// sign — so a signed 8-bit net in a signed 32-bit context sign-extends and in an
/// unsigned one zero-extends, exactly as before.
///
/// iverilog: `L=-1 M=255 N=255`.
#[test]
fn a_narrower_leaf_still_extends_by_its_own_sign() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         reg signed [7:0] s8; reg [7:0] u8;\n  \
         reg signed [31:0] l; reg [31:0] m, n;\n  \
         always @(*) begin\n    l = s8 + 32'sd0;\n    \
         m = s8 & 32'hFFFF_FFFF;\n    n = u8 + 32'd0;\n  end\n  \
         initial begin s8 = -8'sd1; u8 = 8'hFF; #1\n    \
         $display(\"L=%0d M=%0d N=%0d\", l, m, n);\n    $finish;\n  end\nendmodule\n",
    );
    assert!(out.contains("L=-1 M=255 N=255"), "{out}");
}

/// ⚠️ x and z must survive the leaf load unchanged — the compiled `Load` reads both plane
/// words and the exemption touches neither.
///
/// ⚠️ The `z` becomes `x`: `z | 0` is `x` per the bitwise table, so the pinned value is
/// `…1x0x`, not `…1x0z`. The cell still discriminates — a leaf that dropped the unknown
/// plane would read `1000`.
///
/// iverilog: `X=00000000000000000000000000001x0x`.
#[test]
fn an_unknown_leaf_loads_both_planes() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         reg signed [3:0] s; reg [31:0] r;\n  \
         always @(*) r = s | 32'd0;\n  \
         initial begin s = 4'b1x0z; #1 $display(\"X=%b\", r); $finish; end\nendmodule\n",
    );
    assert!(out.contains("X=00000000000000000000000000001x0x"), "{out}");
}

/// ⭐ THE RUNTIME-INDEX HALF, which the first version of this file did not cover and the
/// adversarial review caught. The `Signal` arm has two sub-paths: a constant index gives
/// `Load { vi }`, and a RUNTIME index compiles the index at its own width and sign and
/// emits `LoadIdx`. The exemption admits both, and the second is the one that also moves a
/// DIAGNOSTIC — `LoadIdx` is the module's only caller of `note_bad_index`.
///
/// It is genuinely newly reached: the same design with an UNSIGNED memory is equally fast
/// in both binaries, while the signed one measured **2.3×** — the sign gate was the sole
/// blocker, so the tree PRE never compiled contains `LoadIdx`, not `Load`.
///
/// `A` an all-ones signed element in an unsigned `^` context. `B` the sign bit set, read
/// three ways (unsigned `^`, unsigned `+`, and a signed `>>>` that must still sign-fill).
/// `C` an unwritten element, so both planes come back unknown through the compiled load.
///
/// iverilog: `A=f0f0f0f0 4294967295 -1`, `B=8f0f0f0f 2147483648 -1073741824`,
/// `C=xxxxxxxx x x`.
#[test]
fn a_runtime_indexed_signed_element_is_sign_inert_too() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         logic signed [31:0] mem [0:7];\n  logic [31:0] b; logic [2:0] i;\n  \
         logic [31:0] r1, r2; logic signed [31:0] r3;\n  \
         always @(*) begin r1 = mem[i] ^ b; r2 = mem[i] + 32'd0; r3 = mem[i] >>> 1; end\n  \
         initial begin\n    mem[0] = -32'sd1; mem[3] = 32'sh8000_0000; b = 32'h0F0F_0F0F;\n    \
         i = 3'd0; #1 $display(\"A=%h %0d %0d\", r1, r2, r3);\n    \
         i = 3'd3; #1 $display(\"B=%h %0d %0d\", r1, r2, r3);\n    \
         i = 3'd5; #1 $display(\"C=%h %0d %0d\", r1, r2, r3);\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(out.contains("A=f0f0f0f0 4294967295 -1"), "{out}");
    assert!(out.contains("B=8f0f0f0f 2147483648 -1073741824"), "{out}");
    assert!(out.contains("C=xxxxxxxx x x"), "{out}");
}
