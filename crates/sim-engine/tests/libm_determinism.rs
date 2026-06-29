//! The vendored pure-Rust libm (third_party/libm, default-features=false → no
//! `arch` hardware intrinsics) is the 3-OS-deterministic source for the N6
//! real-math system functions. These assertions PIN its bit-exact output so a
//! libm version bump or feature drift that would change a result (and thus the
//! VCD/stdout goldens) is caught here, not silently.
//!
//! Every reference integer is iverilog 13.0's `$realtobits(<fn>(<arg>))` — the
//! raw f64 bit pattern its platform libm produces. The vendored libm matching
//! these bit-for-bit confirms BOTH correctness (agrees with the differential
//! oracle at full f64 precision) AND that our deterministic source tracks the
//! reference math. This is a CONTRACT pin (like rng.rs's splitmix test): the
//! bytes here are vitamin's reproducibility guarantee.

/// π via `acos(-1.0)` — iverilog `$realtobits` = 0x400921FB54442D18.
#[test]
fn libm_acos_pi_bits_match_iverilog() {
    assert_eq!(libm::acos(-1.0).to_bits(), 4_614_256_656_552_045_848);
    assert_eq!(libm::acos(-1.0).to_bits(), std::f64::consts::PI.to_bits());
}

/// A spread of transcendentals, pinned to the VENDORED libm's bit-exact output
/// (the contract — a version bump that changes any value is caught here). Of
/// these, all match iverilog 13.0's `$realtobits` bit-for-bit EXCEPT `$tan(1.0)`
/// (…662 vs …661) and `$exp(1.0)` (…882 vs …881), which the vendored pure-soft
/// libm computes 1 ULP from iverilog's platform libm. That divergence is the D3
/// trade-off made explicit: vitamin prioritizes 3-OS byte-identical determinism
/// (one vendored libm everywhere) over matching a specific platform's last ULP.
/// The gap is far below `%g`/`%f` display precision, so user-visible `$display`
/// output still matches iverilog — only a full-precision `$realtobits` reveals it.
#[test]
fn libm_transcendentals_bit_pinned() {
    // Bit-identical to iverilog $realtobits:
    assert_eq!(libm::sin(1.0).to_bits(), 4_605_754_516_372_524_270);
    assert_eq!(libm::cos(1.0).to_bits(), 4_603_041_830_072_026_764);
    assert_eq!(libm::log(2.0).to_bits(), 4_604_418_534_313_441_775);
    assert_eq!(libm::sqrt(2.0).to_bits(), 4_609_047_870_845_172_685);
    assert_eq!(libm::atan2(1.0, 1.0).to_bits(), 4_605_249_457_297_304_856);
    assert_eq!(libm::asinh(1.0).to_bits(), 4_606_113_927_061_427_239);
    // 1 ULP from iverilog: the documented D3 platform divergence (vendored value).
    assert_eq!(libm::tan(1.0).to_bits(), 4_609_692_760_021_066_662); // iverilog …661
    assert_eq!(libm::exp(1.0).to_bits(), 4_613_303_445_314_885_482); // iverilog …881
                                                                     // Exact (no rounding): integral results.
    assert_eq!(libm::pow(2.0, 10.0), 1024.0);
    assert_eq!(libm::hypot(3.0, 4.0), 5.0);
    assert_eq!(libm::floor(2.7), 2.0);
    assert_eq!(libm::ceil(2.1), 3.0);
}
