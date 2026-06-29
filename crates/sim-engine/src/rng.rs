//! v7 random-number kernels.
//!
//! `$random` = IEEE 1364-2005 Annex N (`uniform`/`rtl_dist_uniform`
//! specialized to the full-range draw): a 32-bit LCG (`s' = 69069·s + 1`,
//! zero seed substitutes 259341593 first) whose high 23 bits map through an
//! IEEE-754 float mantissa into [1,2), then affine-stretch over the full
//! signed 32-bit range with the Annex's exact floor-style negative cast.
//! Pure double mul/add — bit-exact on every IEEE-754 platform (the 3-OS
//! determinism contract; no libm). iverilog implements the same annex, so
//! the sequence is differential-pinned live.
//!
//! `$urandom`/`$urandom_range` are IMPLEMENTATION-DEFINED by IEEE 1800
//! §18.13 — vitamin pins splitmix64 (top 32 bits) from initial state 0,
//! global (the engine is single-threaded; thread-stability is N/A). The
//! sequence is part of vitamin's reproducibility contract: same design,
//! same seed → byte-identical on every OS.

/// One full-range `$random` draw (Annex N). Updates `seed` in place and
/// returns the signed 32-bit value.
pub(crate) fn annex_n_random(seed: &mut u32) -> i32 {
    if *seed == 0 {
        *seed = 259_341_593;
    }
    *seed = seed.wrapping_mul(69_069).wrapping_add(1);
    // High 23 seed bits → float mantissa with exponent 0 ⇒ c ∈ [1, 2).
    let c = f32::from_bits((*seed >> 9) | 0x3f80_0000) as f64;
    let d = 0.000_000_119_209_289_550_781_25_f64; // 2^-23, Annex constant
    let c = c + c * d;
    // Stretch over the dist_uniform range WITH the `end+1` step folded in
    // (scale = 2^32, exact in doubles), then the Annex floor-style cast:
    // `(long) r` for r ≥ 0, `(long)(r - 1)` otherwise. All four live-pinned
    // iverilog draws (two signs × two seeds) reproduce bit-exactly; the
    // plain (b-a)-scale + trunc reading of the annex text is off by one on
    // both (the t6 probe, 2026-06-12).
    let r = ((c - 1.0) * 4_294_967_296.0) - 2_147_483_648.0;
    if r >= 0.0 {
        r as i64 as i32
    } else {
        (r - 1.0) as i64 as i32
    }
}

/// IEEE 1364-2005 Annex `uniform`: ONE seed advance → a double in `[a, b)`,
/// the same float-mantissa core as [`annex_n_random`] but parameterized over the
/// bounds (`a`/`b` already range-adjusted by the caller). Pure f64 mul/add — no
/// libm, bit-exact on every IEEE-754 platform.
fn uniform(seed: &mut u32, a: f64, b: f64) -> f64 {
    // Annex zero-seed substitution (same as `annex_n_random`): a 0 seed is
    // replaced before the LCG advance — iverilog 13.0 does this too, so
    // `$dist_uniform(s=0, …)` matches live (the LCG's fixed point at 0 would
    // otherwise stick the sequence).
    if *seed == 0 {
        *seed = 259_341_593;
    }
    *seed = seed.wrapping_mul(69_069).wrapping_add(1);
    // high 23 seed bits → IEEE-754 mantissa with exponent 0 ⇒ c ∈ [1, 2).
    let c = f32::from_bits((*seed >> 9) | 0x3f80_0000) as f64;
    let d = 0.000_000_119_209_289_550_781_25_f64; // 2^-23, Annex constant
    let c = c + c * d;
    (b - a) * (c - 1.0) + a
}

/// `$dist_uniform(seed, start, end)` = IEEE 1364-2005 Annex `rtl_dist_uniform`:
/// a uniform integer in `[start, end]` inclusive, advancing `seed` ONCE. Pure
/// f64 mul/add/floor — bit-exact on every IEEE-754 platform (the 3-OS contract;
/// no libm), and iverilog implements the same Annex so the sequence is
/// differential-pinned live.
///
/// The `$dist_normal`/`exponential`/`poisson`/`chi_square`/`t`/`erlang` siblings
/// LANDED in v19 (below) on the vendored libm: their SEED stream is still Annex
/// pure-integer (iverilog-byte-identical), but the result int is the 3-OS
/// deterministic vitamin pin rather than an iverilog byte-match (D3).
pub(crate) fn dist_uniform(seed: &mut u32, start: i32, end: i32) -> i32 {
    if start >= end {
        return start;
    }
    if end != i32::MAX {
        // common branch: float over `[start, end+1)`, floor-cast, clamp.
        let r = uniform(seed, start as f64, end as f64 + 1.0);
        let mut i = if r >= 0.0 { r as i64 } else { (r - 1.0) as i64 };
        if i < start as i64 {
            i = start as i64;
        }
        if i > end as i64 {
            i = end as i64;
        }
        i as i32
    } else if start != i32::MIN {
        // end == MAX: float over `[start-1, end)`, +1.0, floor-cast, clamp.
        let r = uniform(seed, start as f64 - 1.0, end as f64) + 1.0;
        let mut i = if r >= 0.0 { r as i64 } else { (r - 1.0) as i64 };
        if i < start as i64 {
            i = start as i64;
        }
        if i > end as i64 {
            i = end as i64;
        }
        i as i32
    } else {
        // full range [MIN, MAX] (Annex's third branch).
        let r = (uniform(seed, start as f64, end as f64) + 2_147_483_648.0) / 4_294_967_295.0;
        let r = r * 4_294_967_296.0 - 2_147_483_648.0;
        let i = if r >= 0.0 { r as i64 } else { (r - 1.0) as i64 };
        i.clamp(start as i64, end as i64) as i32
    }
}

// ── v19: non-uniform `$dist_*` transcendentals (IEEE 1364-2005 Annex). ──
//
// Each is a faithful port of the Annex distribution algorithm. The seed advances
// through the SAME integer LCG as `uniform` (pure integer math), so the SEED
// stream is byte-identical to iverilog — the algorithms here issue the exact
// same number/order of `uniform` draws as the Annex reference (verified live
// against iverilog 13.0 in the tests below). The returned int is computed with
// the vendored pure-Rust libm (`log`/`sqrt`/`exp`), so it is 3-OS byte-identical
// but MAY differ from iverilog's platform-libm result by the final rounding/ULP
// (D3: vitamin prioritizes internal 3-OS determinism over platform-byte-identity
// — the `$dist_*` non-uniform results are implementation-defined at this level).

/// One unit-mean exponential variate: `-ln(U)` with `U ∈ (0,1]` (a 0 draw is
/// re-rolled so the log is finite). Advances the seed once per draw.
fn exponential(seed: &mut u32) -> f64 {
    let mut u = 0.0_f64;
    while u == 0.0 {
        u = uniform(seed, 0.0, 1.0);
    }
    -libm::log(u)
}

/// One standard-normal variate via the Marsaglia polar method (Annex `normal`
/// kernel): rejection-sample a point in the unit disc, return one of the two
/// independent normals. Each loop iteration advances the seed twice.
fn standard_normal(seed: &mut u32) -> f64 {
    loop {
        let v1 = 2.0 * uniform(seed, 0.0, 1.0) - 1.0;
        let v2 = 2.0 * uniform(seed, 0.0, 1.0) - 1.0;
        let s = v1 * v1 + v2 * v2;
        if s < 1.0 && s != 0.0 {
            return v1 * libm::sqrt(-2.0 * libm::log(s) / s);
        }
    }
}

/// A real-valued chi-square variate with `df` degrees of freedom (Annex method):
/// `2·Σ exponential` over `df/2` stages, plus one squared standard normal when
/// `df` is odd. Used by both `$dist_chi_square` and `$dist_t`.
fn chi_square_real(seed: &mut u32, df: i32) -> f64 {
    let mut acc = 0.0_f64;
    let pairs = df / 2;
    for _ in 0..pairs {
        acc += 2.0 * exponential(seed);
    }
    if df % 2 != 0 {
        let z = standard_normal(seed);
        acc += z * z;
    }
    acc
}

/// `$dist_normal(seed, mean, standard_deviation)` — Gaussian, rounded to int.
pub(crate) fn dist_normal(seed: &mut u32, mean: i32, std_dev: i32) -> i32 {
    let v = mean as f64 + std_dev as f64 * standard_normal(seed);
    v as i32
}

/// `$dist_exponential(seed, mean)` — `mean·(-ln U)`, truncated. A non-positive
/// `mean` returns 0 with no seed advance (degenerate Annex guard).
pub(crate) fn dist_exponential(seed: &mut u32, mean: i32) -> i32 {
    if mean <= 0 {
        return 0;
    }
    (mean as f64 * exponential(seed)) as i32
}

/// `$dist_poisson(seed, mean)` — Knuth's multiplicative count: the number of unit
/// exponentials whose product stays above `e^-mean`.
pub(crate) fn dist_poisson(seed: &mut u32, mean: i32) -> i32 {
    if mean <= 0 {
        return 0;
    }
    let limit = libm::exp(-(mean as f64));
    let mut n = 0_i32;
    let mut p = uniform(seed, 0.0, 1.0);
    while p > limit {
        p *= uniform(seed, 0.0, 1.0);
        n += 1;
    }
    n
}

/// `$dist_chi_square(seed, degree_of_freedom)`, truncated to int.
pub(crate) fn dist_chi_square(seed: &mut u32, df: i32) -> i32 {
    if df <= 0 {
        return 0;
    }
    chi_square_real(seed, df) as i32
}

/// `$dist_t(seed, degree_of_freedom)` — Student's t = `Z / sqrt(χ²(df)/df)`,
/// truncated to int. The Annex draws the χ² term BEFORE the normal (verified by
/// the iverilog seed stream — reversing the order would desync the seed).
pub(crate) fn dist_t(seed: &mut u32, df: i32) -> i32 {
    if df <= 0 {
        return 0;
    }
    let chi = chi_square_real(seed, df);
    let z = standard_normal(seed);
    let t = z / libm::sqrt(chi / df as f64);
    t as i32
}

/// `$dist_erlang(seed, k_stage, mean)` — Erlang-k = `(mean/k)·Σ exponential`
/// over `k` stages, truncated to int.
pub(crate) fn dist_erlang(seed: &mut u32, k: i32, mean: i32) -> i32 {
    if k <= 0 || mean <= 0 {
        return 0;
    }
    let mut acc = 0.0_f64;
    for _ in 0..k {
        acc += exponential(seed);
    }
    (mean as f64 / k as f64 * acc) as i32
}

/// One `$urandom` draw (vitamin-pinned splitmix64, top 32 bits).
pub(crate) fn splitmix_urandom(state: &mut u64) -> u32 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 32) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annex_n_matches_iverilog_seeded() {
        // iverilog 13.0 live (2026-06-12): s=5 → -2147138048 (seed 345346),
        // then 230383387 (seed -1917100901 as i32 = 2377866395 as u32).
        let mut s: u32 = 5;
        assert_eq!(annex_n_random(&mut s), -2147138048);
        assert_eq!(s, 345346);
        assert_eq!(annex_n_random(&mut s), 230383387);
        assert_eq!(s as i32, -1917100901);
    }

    #[test]
    fn annex_n_matches_iverilog_default_seed() {
        // iverilog live: first three no-arg draws from the zero seed.
        let mut s: u32 = 0;
        assert_eq!(annex_n_random(&mut s), 303379748);
        assert_eq!(annex_n_random(&mut s), -1064739199);
        assert_eq!(annex_n_random(&mut s), -2071669239);
    }

    #[test]
    fn dist_uniform_matches_iverilog() {
        // iverilog 13.0 live (2026-06-19): $dist_uniform(seed, 0, 99) with seed=1
        // → 0 (seed becomes 69070), then → 11 (seed becomes 475628535).
        let mut s: u32 = 1;
        assert_eq!(dist_uniform(&mut s, 0, 99), 0);
        assert_eq!(s, 69070);
        assert_eq!(dist_uniform(&mut s, 0, 99), 11);
        assert_eq!(s, 475_628_535);
        // zero-seed substitution (iverilog live): s=0 → 259341593 before the LCG.
        let mut z: u32 = 0;
        assert_eq!(dist_uniform(&mut z, 0, 99), 57);
        assert_eq!(z as i32, -1_844_104_698);
    }

    #[test]
    fn dist_uniform_degenerate_and_negative_range() {
        // start >= end returns start with NO seed advance (Annex guard).
        let mut s: u32 = 42;
        assert_eq!(dist_uniform(&mut s, 7, 7), 7);
        assert_eq!(s, 42);
        assert_eq!(dist_uniform(&mut s, 9, 3), 9);
        assert_eq!(s, 42);
    }

    // v19: the non-uniform $dist_* SEED advances must be byte-identical to
    // iverilog 13.0 (pure-integer LCG — structural fidelity to the Annex). The
    // RESULT ints are vitamin's deterministic contract (libm-derived; may differ
    // from iverilog by final rounding — D3). Both are pinned here.
    #[test]
    fn dist_normal_seed_matches_iverilog() {
        let mut s: u32 = 1;
        let r = dist_normal(&mut s, 100, 10);
        assert_eq!(s, 772_999_773, "normal seed advance must match iverilog");
        assert_eq!(r, 105); // vitamin contract (iverilog rounds to 106 — D3 gap)
    }
    #[test]
    fn dist_exponential_seed_matches_iverilog() {
        let mut s: u32 = 1;
        let r = dist_exponential(&mut s, 20);
        assert_eq!(s, 69070, "exponential advances the seed once");
        assert_eq!(r, 220); // vitamin contract (iverilog rounds to 221 — D3 gap)
    }
    #[test]
    fn dist_poisson_seed_matches_iverilog() {
        let mut s: u32 = 1;
        let r = dist_poisson(&mut s, 5);
        assert_eq!(s, 69070, "poisson breaks after one draw here");
        assert_eq!(r, 0); // iverilog: 0
    }
    #[test]
    fn dist_chi_square_seed_matches_iverilog() {
        let mut s: u32 = 1;
        let r = dist_chi_square(&mut s, 4);
        assert_eq!(s, 475_628_535, "chi_square(4) = two exponentials");
        assert_eq!(r, 26); // iverilog: 26
    }
    #[test]
    fn dist_t_seed_matches_iverilog() {
        let mut s: u32 = 1;
        let r = dist_t(&mut s, 4);
        assert_eq!(
            s, 772_999_773,
            "t(4) = normal(2 draws) + chi_square(2 draws)"
        );
        assert_eq!(r, 0); // iverilog: 0
    }
    #[test]
    fn dist_erlang_seed_matches_iverilog() {
        let mut s: u32 = 1;
        let r = dist_erlang(&mut s, 3, 20);
        assert_eq!(s as i32, -1_017_563_188, "erlang(k=3) = three exponentials");
        assert_eq!(r, 90); // iverilog: 90
    }

    #[test]
    fn splitmix_is_deterministic_from_zero() {
        // vitamin contract pin (NOT an oracle): the documented $urandom
        // sequence from the initial state. A change here is a contract break.
        let mut st: u64 = 0;
        let a = splitmix_urandom(&mut st);
        let b = splitmix_urandom(&mut st);
        let mut st2: u64 = 0;
        assert_eq!(a, splitmix_urandom(&mut st2));
        assert_eq!(b, splitmix_urandom(&mut st2));
        assert_ne!(a, b);
    }
}
