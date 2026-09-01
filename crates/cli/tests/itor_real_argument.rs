//! `$itor` of a REAL argument converted the IEEE-754 bit pattern, not the number.
//!
//! IEEE 1800 §20.5 defines `$itor` as integral→real, and the engine's arm reads its
//! argument with `to_i128_signed` — which for a real argument is the double's bit
//! pattern. `$itor(3.9)` came out as `4.615964438073390e18` at exit 0 where iverilog
//! gives `4.0`: the argument is coerced to an integer first (§6.24.1, round half away
//! from zero) and then converted. ROADMAP §2 row 4.
//!
//! ⚠️ The CAST spelling lowers to the SAME `SysFuncId::Itor` and was already right —
//! `lower_prim_cast` returns a real operand unchanged and only emits `Itor` for an
//! integral one — so the fix had to go at the `$itor` lowering, not in the evaluator,
//! or `real'(3.9)` would have started rounding to 4.0. Both spellings are asserted on
//! the same line here so neither can be fixed by breaking the other.
//!
//! ⚠️ The intermediate is 64 bits, not `integer`'s 32: iverilog keeps `$itor(1e10)` as
//! 10000000000 and a 32-bit intermediate wrapped it to 1410065408.
//!
//! Values pinned to iverilog 13.0. verilator is not an oracle here — it ICEs on the
//! whole family.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_itor_{}_{n}", std::process::id()));
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

/// THE REPRO, with the three spellings that must NOT move beside it: `$itor` of an
/// integral, and both `real'()` casts.
#[test]
fn itor_of_a_real_rounds_it_and_the_cast_spelling_does_not() {
    let (o, ok) = run("module t;\n  \
           real a, b, c, d, e;\n  \
           integer i = 7;\n  \
           real rr = 3.9;\n  \
           initial begin\n    \
             a = $itor(3.9); b = $itor(rr); c = $itor(i);\n    \
             d = real'(3.9); e = real'(i);\n    \
             $display(\"a=%0f b=%0f c=%0f d=%0f e=%0f\", a, b, c, d, e);\n    \
             #1 $finish;\n  \
           end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(
        o.contains("a=4.000000 b=4.000000 c=7.000000 d=3.900000 e=7.000000"),
        "got:\n{o}"
    );
}

/// The rounding is §6.24.1 — half AWAY FROM ZERO, both signs — and the magnitude
/// survives past 32 bits. Every value here is iverilog's.
#[test]
fn the_rounding_is_half_away_from_zero_and_does_not_wrap_at_32_bits() {
    let (o, ok) = run("module t;\n  \
           real r[0:8];\n  integer k;\n  \
           initial begin\n    \
             r[0]=$itor(-3.9); r[1]=$itor(2.5); r[2]=$itor(-2.5); r[3]=$itor(0.5);\n    \
             r[4]=$itor(-0.5); r[5]=$itor(0.0); r[6]=$itor(3.0);  r[7]=$itor(1e10);\n    \
             r[8]=$itor(-1e10);\n    \
             for (k=0;k<9;k=k+1) $write(\"%0.1f \", r[k]);\n    \
             $display(\"\");\n    #1 $finish;\n  \
           end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(
        o.contains("-4.0 3.0 -3.0 1.0 -1.0 0.0 3.0 10000000000.0 -10000000000.0"),
        "got:\n{o}"
    );
}
