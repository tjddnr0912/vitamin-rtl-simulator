//! Real → integer assignment coercion into a destination WIDER than 128 bits.
//!
//! `Value::from_i128` built the destination by shifting the i128's bit image
//! word by word; for a width ≥129 that shifted a `u128` by ≥128, which is a
//! PANIC in debug and in release wraps to `>> 0` — replicating word 0 into every
//! word above 128. `reg [191:0] p; p = 3.0;` therefore printed
//! `…0003_0000000000000000_0000000000000003` with `errors=0`: a silent wrong
//! value, the one class this project treats as worse than any refusal.
//!
//! Reachable only through `real_to_int_round` (the write funnel's real arm and
//! `$rtoi`) — every other `from_i128` call site passes a literal 32-bit width,
//! which is why the whole existing suite was green over it.
//!
//! Every expected value below is what iverilog 13 prints for the same source.
//! The boundary is pinned from both sides: 128 bits was already correct, 129 was
//! not, so a regression that "fixes" only the high words would still fail here.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r2wi_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.v"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.v")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    (text, out.status.code())
}

#[test]
fn a_real_assigned_to_a_net_wider_than_128_bits_sign_extends() {
    let (o, ok) = run("module t;\n\
           reg [191:0] p192; reg [129:0] p130; reg [128:0] p129;\n\
           reg [127:0] p128; reg [191:0] n192; reg [129:0] n130;\n\
           initial begin\n\
             p192 = 3.0; p130 = 3.0; p129 = 3.0; p128 = 3.0;\n\
             n192 = -3.0; n130 = -3.0;\n\
             $display(\"p192=%h\", p192);\n\
             $display(\"p130=%h\", p130);\n\
             $display(\"p129=%h\", p129);\n\
             $display(\"p128=%h\", p128);\n\
             $display(\"n192=%h\", n192);\n\
             $display(\"n130=%h\", n130);\n\
             #1 $finish;\n\
           end\n\
         endmodule\n");
    assert_eq!(ok, Some(0), "must not panic or error:\n{o}");
    // Positive: zero fill above the i128 image.
    assert!(
        o.contains("p192=000000000000000000000000000000000000000000000003"),
        "192-bit positive:\n{o}"
    );
    assert!(
        o.contains("p130=000000000000000000000000000000003"),
        "130-bit positive:\n{o}"
    );
    assert!(
        o.contains("p129=000000000000000000000000000000003"),
        "129-bit positive (the first wrong width):\n{o}"
    );
    // 128 bits was always correct — pinned so a fix cannot regress it.
    assert!(
        o.contains("p128=00000000000000000000000000000003"),
        "128-bit positive:\n{o}"
    );
    // Negative: ONES fill above the image (two's-complement sign extension).
    assert!(
        o.contains("n192=fffffffffffffffffffffffffffffffffffffffffffffffd"),
        "192-bit negative:\n{o}"
    );
    assert!(
        o.contains("n130=3fffffffffffffffffffffffffffffffd"),
        "130-bit negative (top word masked to the declared width):\n{o}"
    );
}

/// The same coercion through `$rtoi`, the other `real_to_int_round` caller.
#[test]
fn rtoi_into_a_wide_net_sign_extends_too() {
    let (o, ok) = run("module t;\n\
           reg [191:0] w; reg [191:0] n;\n\
           initial begin\n\
             w = $rtoi(5.9); n = $rtoi(-5.9);\n\
             $display(\"w=%h\", w); $display(\"n=%h\", n);\n\
             #1 $finish;\n\
           end\n\
         endmodule\n");
    assert_eq!(ok, Some(0), "{o}");
    assert!(
        o.contains("w=000000000000000000000000000000000000000000000005"),
        "$rtoi truncates toward zero:\n{o}"
    );
    assert!(
        o.contains("n=fffffffffffffffffffffffffffffffffffffffffffffffb"),
        "and sign-extends the negative:\n{o}"
    );
}
