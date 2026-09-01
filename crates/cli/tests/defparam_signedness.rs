//! A `defparam` override carried no signedness, so a NEGATIVE one stopped its sign
//! at bit 63.
//!
//! Overriding a `parameter logic [127:0]`, the value has to be EXTENDED past the i64
//! lane, and the i64 alone cannot say how: `64'hFFFF_FFFF_FFFF_FFFF + 64'd0`,
//! `-(64'sd1)` and `32'd0 - 32'd1` all fold to the same i64 and do not all extend the
//! same way. The other three override channels (`#()` positional, `#()` named, `-G`)
//! record the EXPRESSION's signedness for exactly that reason; the `defparam`
//! collector folded to i64 before the record existed, so its field was `None`, which
//! `bind_one_param` reads as "stay on the route you took before" — zero-extension.
//!
//! `defparam u.K = -32'sd7;` was `0000000000000000fffffffffffffff9` where both
//! oracles give all ones. Fixed by computing the flag in the collector, where the
//! expression still exists, with the SAME helper the `#()` channel uses.
//! ROADMAP §2 row 18.
//!
//! ⚠️ `32'd0 - 32'd1` is deliberately NOT asserted here: ROADMAP §2 row 17 records it
//! as an ORACLE SPLIT (iverilog sign-extends an unsigned 32-bit expression, verilator
//! zero-extends from 32, vita zero-extends from 64) and says not to chase it.
//!
//! Values pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dpsg_{}_{n}", std::process::id()));
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

/// The four shapes whose answer both oracles agree on, on one line so a fix that
/// flips the sign rule wholesale cannot pass: two that must sign-extend and two that
/// must not.
#[test]
fn a_defparam_extends_by_its_expressions_sign() {
    let (o, ok) = run("module leaf #(parameter logic [127:0] K = 128'd0);\n  \
           initial $display(\"K=%h\", K);\n\
         endmodule\n\
         module t;\n  \
           leaf u1(); defparam u1.K = -32'sd7;\n  \
           leaf u2(); defparam u2.K = 32'd7;\n  \
           leaf u3(); defparam u3.K = 64'hFFFF_FFFF_FFFF_FFFF;\n  \
           leaf u4(); defparam u4.K = -(64'sd1);\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    // u1, u4: signed ⇒ sign-extend. u2: small positive. u3: unsigned ⇒ zero-extend.
    assert!(
        o.contains("K=fffffffffffffffffffffffffffffff9"),
        "signed negative must sign-extend:\n{o}"
    );
    assert!(
        o.contains("K=00000000000000000000000000000007"),
        "positive stays positive:\n{o}"
    );
    assert!(
        o.contains("K=0000000000000000ffffffffffffffff"),
        "an UNSIGNED all-ones must NOT sign-extend:\n{o}"
    );
    assert!(
        o.contains("K=ffffffffffffffffffffffffffffffff"),
        "unary minus on a signed literal must sign-extend:\n{o}"
    );
}

/// A defparam that stays inside the i64 lane is unaffected — the flag is consumed
/// ONLY for an extension past it.
#[test]
fn a_narrow_defparam_target_is_unchanged() {
    let (o, ok) = run("module leaf #(parameter signed [31:0] K = 0);\n  \
           initial $display(\"K=%0d\", K);\n\
         endmodule\n\
         module t;\n  leaf u1(); defparam u1.K = -32'sd7;\n  \
           leaf u2(); defparam u2.K = 32'd9;\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("K=-7"), "got:\n{o}");
    assert!(o.contains("K=9"), "got:\n{o}");
}
