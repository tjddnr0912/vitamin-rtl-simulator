//! `$realtobits` / `$bitstoreal` with an argument that is not 64 bits wide.
//!
//! ⚠️ THIS TEST EXISTS BECAUSE ITS ABSENCE WAS THE HOLE. §4.5.368 removed a
//! defensive `mask_top()` from `Value::resize`'s equal-width arm and argued the
//! canonical-`Value` invariant from "the whole 5,812-test suite ran with the
//! assert armed and it never fired". The adversarial review showed that was a
//! COVERAGE statement, not a proof: `$realtobits` stamped `width = 64` onto planes
//! that had been sized for its ARGUMENT, so `$realtobits(<128-bit>)` produced a
//! non-canonical value and panicked the debug build — on a design the release
//! build ran correctly. The producer now re-canonicalises, and this pins the shape
//! so the suite actually covers it.
//!
//! ⚠️ iverilog REJECTS a non-64-bit argument outright ("$bitstoreal requires a
//! 64-bit argument"), so these values are vita's own contract — the low 64 bits,
//! which is what it has always returned. That gap (accepting the call at all) is
//! ROADMAP §3's, and it is exactly why this path is reachable.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rtb_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.code(),
    )
}

#[test]
fn realtobits_of_an_argument_wider_than_sixty_four_bits() {
    // Two and four words in — the `Words::Inline` and `Words::Heap` cases.
    let (out, code) = run("module t;\n\
         \x20 logic [127:0] w; logic [199:0] w2; logic [63:0] b, b2;\n\
         \x20 initial begin\n\
         \x20   w  = 128'h0011223344556677_8899aabbccddeeff;\n\
         \x20   w2 = 200'h1122334455667788_99aabbccddeeff00_5566778899aabbcc;\n\
         \x20   b = $realtobits(w); b2 = $realtobits(w2);\n\
         \x20   $display(\"B=%h B2=%h\", b, b2); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "must not panic or reject;\n{out}");
    assert!(
        !out.contains("panicked"),
        "a non-canonical Value reached `resize`;\n{out}"
    );
    assert!(
        out.contains("B=8899aabbccddeeff B2=5566778899aabbcc"),
        "the low 64 bits, unchanged from before the slice;\n{out}"
    );
}

#[test]
fn realtobits_of_a_narrower_argument_and_the_round_trip() {
    // The other direction (fewer words than 64 bits needs) and `$bitstoreal`,
    // whose producer is the `is_real` twin of the one that broke.
    let (out, code) = run("module t;\n\
         \x20 logic [7:0] n; logic [63:0] b; real r; logic [95:0] w;\n\
         \x20 initial begin\n\
         \x20   n = 8'hA5; b = $realtobits(n);\n\
         \x20   w = 96'h0000000000000000_3ff0000000000000; r = $bitstoreal(w);\n\
         \x20   $display(\"B=%h R=%0f\", b, r); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(!out.contains("panicked"), "got:\n{out}");
    assert!(
        out.contains("B=00000000000000a5") && out.contains("R=1.000000"),
        "narrow argument zero-extends; a 96-bit pattern reads its low 64 as 1.0;\n{out}"
    );
}

#[test]
fn a_wide_realtobits_result_survives_being_used() {
    // The panic was in `resize`, which the RESULT hits when it is assigned,
    // compared and part-selected — so exercise those, not just the call.
    let (out, code) = run("module t;\n\
         \x20 logic [127:0] w; logic [63:0] b; logic [31:0] hi;\n\
         \x20 initial begin\n\
         \x20   w = 128'h0011223344556677_8899aabbccddeeff;\n\
         \x20   b = $realtobits(w);\n\
         \x20   hi = b[63:32];\n\
         \x20   $display(\"EQ=%0d HI=%h SUM=%h\", (b == 64'h8899aabbccddeeff), hi, b + 64'd1);\n\
         \x20   $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(!out.contains("panicked"), "got:\n{out}");
    assert!(
        out.contains("EQ=1 HI=8899aabb SUM=8899aabbccddef00"),
        "equality, part-select and arithmetic on the result;\n{out}"
    );
}
