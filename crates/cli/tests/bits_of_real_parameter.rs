//! §2 🆕 L ⓐ: `$bits(P)` where `P` is a `real` / `realtime` PARAMETER.
//!
//! IEEE 1800 §6.12.1 makes `real` the IEEE-754 double and `realtime` the same
//! type, so both are 64 bits. A real parameter has no `param_meta` width — its
//! value lives in `real_param_val`, not the i64 domain — so `bits_of_view`'s
//! constant arm fell through to the untyped tail and answered the i64 domain's
//! 32.
//!
//! The oracle here is not a preference between tools. THREE of vita's own answers
//! for the same object already read 64 — a real VARIABLE (`real r; $bits(r)`),
//! the package-SCOPED spelling `$bits(p::P)`, and `$realtobits` — so one source
//! object had two answers, and verilator reads 64 for every one of them. iverilog
//! 13.0 answers 1 for a real VARIABLE too, which disqualifies it on this axis by
//! its own neighbouring answer rather than by disagreement.
//!
//! Fixed at `bits_of_view`, the ONE table both `$bits` arms funnel through — the
//! lowering arm (`lower_bits_fold`) and the constant-domain arm (`const_fn.rs`'s
//! `$bits` SysCall) — so a range bound, a `localparam` initializer, a generate
//! condition and a constant-function body all moved with the runtime spelling.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_brp_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
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
        out.status.code().unwrap_or(-1),
    )
}

fn shows(src: &str, want: &str) {
    let (out, code) = run(src);
    assert_eq!(code, 0, "expected exit 0:\n{out}");
    assert!(out.contains(want), "wanted `{want}`:\n{out}");
}

/// Both spellings of the declaration, and `realtime` — §6.12.1 gives all three the
/// same 64-bit double.
#[test]
fn a_real_or_realtime_parameter_is_sixty_four_bits() {
    for decl in [
        "localparam real P = 3;",
        "parameter real P = 3;",
        "localparam realtime P = 3;",
    ] {
        shows(
            &format!(
                "module top; {decl}\n\
                   initial begin $display(\"R=%0d\", $bits(P)); $finish; end\n\
                 endmodule\n"
            ),
            "R=64",
        );
    }
}

/// The CONSTANT-domain arm, which is a second reader of the same table: a range
/// bound, a `localparam` initializer, a generate condition and a constant-function
/// body each folded 32 before. The bound is the one that mattered — a 32-bit net
/// where both the LRM and verilator size 64 truncates silently at exit 0.
#[test]
fn every_constant_consumer_moved_with_it() {
    shows(
        "module top; localparam real P = 3; wire [$bits(P)-1:0] w;\n\
           initial begin $display(\"R=%0d\", $bits(w)); $finish; end endmodule\n",
        "R=64",
    );
    shows(
        "module top; localparam real P = 3; localparam Q = $bits(P);\n\
           initial begin $display(\"R=%0d\", Q); $finish; end endmodule\n",
        "R=64",
    );
    shows(
        "module top; localparam real P = 3;\n\
           generate if ($bits(P) == 64) begin : g\n\
             initial begin $display(\"R=%0d\", 64); $finish; end\n\
           end endgenerate endmodule\n",
        "R=64",
    );
    shows(
        "module top; localparam real P = 3;\n\
           function automatic int f(); return $bits(P); endfunction\n\
           initial begin $display(\"R=%0d\", f()); $finish; end endmodule\n",
        "R=64",
    );
}

/// The answers that must NOT move, each a different tail of the same arm: a real
/// VARIABLE and the package-SCOPED parameter already read 64 (they are the reason
/// this was a self-contradiction and not a judgement call), a TYPED integral param
/// reports its declared width, an `int` / untyped one stays 32, and a string
/// parameter stays 16 (§6.16, vita = iverilog).
#[test]
fn the_neighbouring_answers_are_unchanged() {
    shows(
        "module top; real r; initial begin $display(\"R=%0d\", $bits(r)); $finish; end endmodule\n",
        "R=64",
    );
    shows(
        "package p; localparam real P = 3; endpackage\n\
         module top; initial begin $display(\"R=%0d\", $bits(p::P)); $finish; end endmodule\n",
        "R=64",
    );
    for (decl, want) in [
        ("localparam logic [11:0] P = 3;", "R=12"),
        ("localparam int P = 3;", "R=32"),
        ("localparam P = 3;", "R=32"),
        ("localparam string P = \"ab\";", "R=16"),
    ] {
        shows(
            &format!(
                "module top; {decl}\n\
                   initial begin $display(\"R=%0d\", $bits(P)); $finish; end endmodule\n"
            ),
            want,
        );
    }
}

/// A block-local that SHADOWS the real parameter still answers for the object it
/// names — the new arm sits inside the `!local_shadows` branch, so it inherits the
/// §4.5.430 shadow test the value read uses rather than adding a second rule.
#[test]
fn a_block_local_shadow_still_wins() {
    shows(
        "module top; localparam real P = 3;\n\
           initial begin : b logic [11:0] P; P = 12'h5a;\n\
             $display(\"R=%0d\", $bits(P)); $finish; end\n\
         endmodule\n",
        "R=12",
    );
}

/// Recorded residue, pinned so it cannot move silently: the same parameter read
/// through `import p::*` still answers 32 (verilator 64). That is the §2 🆕 L ⓕ
/// name-lookup family — a wildcard-imported real does not bind at a module key
/// this walk reaches — and not the width rule this slice fixed, which is why the
/// SCOPED spelling above already answered 64.
#[test]
fn the_wildcard_imported_spelling_is_recorded_residue() {
    shows(
        "package p; localparam real P = 3; endpackage\n\
         module top; import p::*;\n\
           initial begin $display(\"R=%0d\", $bits(P)); $finish; end endmodule\n",
        "R=32",
    );
}
