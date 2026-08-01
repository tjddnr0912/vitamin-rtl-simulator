use super::*;

#[test]
fn add_two_signals_matches_oracle_incl_wrap() {
    // exprs: 0=sig(0), 1=sig(1), 2 = 0 + 1
    let ir = ir_of(
        vec![sig(0), sig(1), bin(BinOp::Add, 0, 1)],
        vec![],
        vec![nv(64, false), nv(64, false)],
    );
    for (a, b) in [(7u64, 35u64), (u64::MAX, 1), (0, 0), (1 << 63, 1 << 63)] {
        assert_matches_oracle(&ir, 2, 64, false, &[v64(a), v64(b)]);
    }
}

#[test]
fn arith_xz_operand_poisons_whole_result() {
    let ir = ir_of(
        vec![sig(0), sig(1), bin(BinOp::Add, 0, 1)],
        vec![],
        vec![nv(64, false), nv(64, false)],
    );
    // one X bit anywhere in an operand ⇒ the oracle poisons the whole add to X.
    assert_matches_oracle(&ir, 2, 64, false, &[v64_xz(5, 0b10), v64(9)]);
    assert_matches_oracle(&ir, 2, 64, false, &[v64(9), v64_xz(0, 1 << 40)]);
}

#[test]
fn sub_and_mul_match_oracle() {
    for op in [BinOp::Sub, BinOp::Mul] {
        let ir = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(32, false), nv(32, false)],
        );
        for (a, b) in [(3u64, 9u64), (0xFFFF_FFFF, 2), (123456, 654321)] {
            assert_matches_oracle(&ir, 2, 32, false, &[v64(a), v64(b)]);
        }
    }
}

#[test]
fn bitwise_4state_matches_oracle() {
    for op in [BinOp::BitAnd, BinOp::BitOr, BinOp::BitXor, BinOp::BitXnor] {
        let ir = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(16, false), nv(16, false)],
        );
        // include X/Z so the 4-state truth tables (not just 2-state) are exercised.
        let a = v64_xz(0b1010_0101, 0b0000_1100);
        let b = v64_xz(0b1100_0011, 0b0011_0000);
        assert_matches_oracle(&ir, 2, 16, false, &[a, b]);
    }
}

// POW-LANE: a small positive const exponent lowers to a native Mul chain;
// for every (w, n) with w*n <= 128 the result is byte-identical to the oracle
// (clean values AND X-poison), so backend selection stays transparent.
#[test]
fn pow_const_small_exponent_matches_oracle() {
    // (ctx width, exponent) pairs: n>=2 and w*n <= 128.
    for (w, n) in [
        (16u32, 2u64),
        (16, 3),
        (16, 4),
        (16, 8),
        (8, 16),
        (64, 2),
        (32, 4),
    ] {
        let ir = ir_of(
            vec![sig(0), Expr::Const { val: 0 }, bin(BinOp::Pow, 0, 1)],
            vec![cnum(8, n)],
            vec![nv(w, false)],
        );
        for a in [0u64, 1, 2, 3, 7, 255, 0x1234, 0xFFFF] {
            assert_matches_oracle(&ir, 2, w, false, &[v64(a)]);
        }
        // X anywhere in the base must X-poison exactly like the oracle's Pow.
        assert_matches_oracle(&ir, 2, w, false, &[v64_xz(0x12, 0x0F)]);
    }
}

// POW-LANE bail boundary: exponent 0 / over POW_MAX / w*n>128 / non-const must
// NOT compile natively (they stay oracle-bound), or values would diverge from
// the oracle's u128-`checked_pow().unwrap_or(0)` overflow quirk.
#[test]
fn pow_uncompilable_cases_bail_to_oracle() {
    let wt_none = crate::FuncTable::new();
    let compiles = |consts: Vec<sim_ir::ConstVal>, rhs: Expr, w: u32, signed: bool| {
        // two nets so a non-const `rhs = sig(1)` resolves (net 1 unused otherwise).
        let ir = ir_of(
            vec![sig(0), rhs, bin(BinOp::Pow, 0, 1)],
            consts,
            vec![nv(w, signed), nv(w, false)],
        );
        let wt = WidthTable::build(&ir, &wt_none);
        try_compile(&ir, &wt, &ineligible_nets(&ir), 2, w, signed).is_some()
    };
    // a ** 0 : oracle X-poisons an X base, but a Const-1 would say 1.
    assert!(
        !compiles(vec![cnum(8, 0)], Expr::Const { val: 0 }, 16, false),
        "a**0 must bail"
    );
    // a ** 1 : native passthrough keeps an X base, but the oracle X-poisons it.
    assert!(
        !compiles(vec![cnum(8, 1)], Expr::Const { val: 0 }, 16, false),
        "a**1 must bail"
    );
    // a ** 17 : over POW_MAX.
    assert!(
        !compiles(vec![cnum(8, 17)], Expr::Const { val: 0 }, 16, false),
        "a**17 must bail"
    );
    // a ** 4 at w=64 : w*n = 256 > 128 (oracle overflow-to-0 quirk reachable).
    assert!(
        !compiles(vec![cnum(8, 4)], Expr::Const { val: 0 }, 64, false),
        "wide w*n>128 must bail"
    );
    // a ** b (non-const exponent).
    assert!(
        !compiles(vec![], sig(1), 16, false),
        "a**b (non-const) must bail"
    );
    // signed Pow uses ipow_signed — bail.
    assert!(
        !compiles(vec![cnum(8, 2)], Expr::Const { val: 0 }, 16, true),
        "signed a**2 must bail"
    );
}

#[test]
fn bitnot_and_negate_match_oracle() {
    // ~sig0
    let ir_not = ir_of(
        vec![
            sig(0),
            Expr::Unary {
                op: UnOp::BitNot,
                operand: 0,
            },
        ],
        vec![],
        vec![nv(8, false)],
    );
    assert_matches_oracle(&ir_not, 1, 8, false, &[v64(0b1011_0010)]);
    assert_matches_oracle(&ir_not, 1, 8, false, &[v64_xz(0b1011_0010, 0b0000_1111)]);

    // -sig0 (two's complement); X/Z poisons.
    let ir_neg = ir_of(
        vec![
            sig(0),
            Expr::Unary {
                op: UnOp::Minus,
                operand: 0,
            },
        ],
        vec![],
        vec![nv(8, true)],
    );
    assert_matches_oracle(&ir_neg, 1, 8, true, &[v64(5)]);
    assert_matches_oracle(&ir_neg, 1, 8, true, &[v64_xz(5, 0b10)]);
}

#[test]
fn chained_adds_match_oracle() {
    // (((s0 + s1) + s2) + s3) — the EXPR_HEAVY shape, all 64-bit.
    let ir = ir_of(
        vec![
            sig(0),
            sig(1),
            bin(BinOp::Add, 0, 1),
            sig(2),
            bin(BinOp::Add, 2, 3),
            sig(3),
            bin(BinOp::Add, 4, 5),
        ],
        vec![],
        vec![nv(64, false), nv(64, false), nv(64, false), nv(64, false)],
    );
    assert_matches_oracle(&ir, 6, 64, false, &[v64(11), v64(22), v64(33), v64(44)]);
}

#[test]
fn over_128_bits_is_not_native_compilable() {
    // beyond the two-word wide lane (>128) the whole tree must bail (None)
    // → the VM keeps interpreting it. (65..=128 now compiles — C6 lane.)
    let ir = ir_of(
        vec![sig(0), sig(1), bin(BinOp::Add, 0, 1)],
        vec![],
        vec![nv(200, false), nv(200, false)],
    );
    let wt = WidthTable::build(&ir, &crate::FuncTable::new());
    assert!(try_compile(&ir, &wt, &ineligible_nets(&ir), 2, 200, false).is_none());
}

#[test]
fn unsupported_operator_bails() {
    // SysFunc stays outside the subset → None (Concat/Select/Replicate
    // joined in the structural increment).
    let ir = ir_of(
        vec![
            sig(0),
            Expr::SysFunc {
                which: sim_ir::SysFuncId::Time,
                args: vec![],
            },
            bin(BinOp::Add, 0, 1),
        ],
        vec![],
        vec![nv(32, false)],
    );
    let wt = WidthTable::build(&ir, &crate::FuncTable::new());
    assert!(try_compile(&ir, &wt, &ineligible_nets(&ir), 2, 64, false).is_none());
}

#[test]
fn relational_signed_pair_matches_oracle() {
    for op in [BinOp::Lt, BinOp::Le, BinOp::Gt, BinOp::Ge] {
        let ir = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(8, true), nv(8, true)],
        );
        // -8 vs 3, 3 vs -8, equal, MIN vs MAX — signed ordering.
        for (a, b) in [(0xF8u64, 3u64), (3, 0xF8), (5, 5), (0x80, 0x7F)] {
            assert_matches_oracle(&ir, 2, 32, false, &[vws(8, a), vws(8, b)]);
        }
        // any X/Z → 1-bit X (zero-extended into the context).
        assert_matches_oracle(&ir, 2, 32, false, &[vw_xz(8, 5, 0b100), vws(8, 9)]);
    }
}

#[test]
fn relational_mixed_sign_is_unsigned_compare() {
    // one unsigned operand ⇒ pair compares UNSIGNED (0xF8 > 3, not -8 < 3).
    let ir = ir_of(
        vec![sig(0), sig(1), bin(BinOp::Lt, 0, 1)],
        vec![],
        vec![nv(8, true), nv(8, false)],
    );
    assert_matches_oracle(&ir, 2, 8, false, &[vws(8, 0xF8), vw(8, 3)]);
    assert_matches_oracle(&ir, 2, 8, false, &[vws(8, 3), vw(8, 0xF8)]);
}

#[test]
fn equality_and_case_equality_match_oracle() {
    for op in [BinOp::Eq, BinOp::Ne] {
        let ir = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(8, false), nv(8, false)],
        );
        assert_matches_oracle(&ir, 2, 8, false, &[vw(8, 0xAB), vw(8, 0xAB)]);
        assert_matches_oracle(&ir, 2, 8, false, &[vw(8, 0xAB), vw(8, 0xAC)]);
        // any X in either ⇒ X result for ==/!=.
        assert_matches_oracle(&ir, 2, 8, false, &[vw_xz(8, 0xAB, 1), vw(8, 0xAB)]);
    }
    for op in [BinOp::CaseEq, BinOp::CaseNe] {
        let ir = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(8, false), nv(8, false)],
        );
        // === compares X positions literally: matching X ⇒ equal, never X.
        assert_matches_oracle(
            &ir,
            2,
            8,
            false,
            &[vw_xz(8, 0xA0, 0xF), vw_xz(8, 0xA0, 0xF)],
        );
        assert_matches_oracle(&ir, 2, 8, false, &[vw_xz(8, 0xA0, 0xF), vw(8, 0xA0)]);
    }
}

#[test]
fn shifts_match_oracle() {
    for op in [BinOp::Shl, BinOp::Shr, BinOp::AShr] {
        let ir = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(16, true), nv(8, false)],
        );
        for (x, amt) in [
            (0x8001u64, 0u64),
            (0x8001, 3),
            (0x8001, 15),
            (0x8001, 16), // == width: everything out / full sign fill
            (0x8001, 200),
            (0x7001, 4),
        ] {
            assert_matches_oracle(&ir, 2, 16, false, &[vws(16, x), vw(8, amt)]);
            // signed enclosing context too (AShr fill follows lhs OWN sign).
            assert_matches_oracle(&ir, 2, 16, true, &[vws(16, x), vw(8, amt)]);
        }
        // X/Z amount poisons; X MSB on AShr fills X.
        assert_matches_oracle(&ir, 2, 16, false, &[vws(16, 0x8001), vw_xz(8, 2, 1)]);
        assert_matches_oracle(&ir, 2, 16, false, &[vw_xz(16, 1, 1 << 15), vw(8, 3)]);
    }
}

#[test]
fn div_mod_match_oracle() {
    for op in [BinOp::Div, BinOp::Mod] {
        // UNSIGNED
        let ir = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(16, false), nv(16, false)],
        );
        for (a, b) in [(100u64, 7u64), (7, 100), (0xFFFF, 3), (5, 0)] {
            assert_matches_oracle(&ir, 2, 16, false, &[vw(16, a), vw(16, b)]);
        }
        assert_matches_oracle(&ir, 2, 16, false, &[vw_xz(16, 9, 1), vw(16, 3)]);
        // SIGNED: truncating toward zero (-7/2 = -3, -7%2 = -1).
        let irs = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(16, true), nv(16, true)],
        );
        for (a, b) in [(0xFFF9u64, 2u64), (7, 0xFFFE), (0xFFF9, 0xFFFE), (5, 0)] {
            assert_matches_oracle(&irs, 2, 16, true, &[vws(16, a), vws(16, b)]);
        }
    }
}

#[test]
fn ternary_matches_oracle() {
    // exprs: 0=cond sig2, 1=sig0, 2=sig1, 3 = cond ? sig0 : sig1
    let ir = ir_of(
        vec![
            sig(2),
            sig(0),
            sig(1),
            Expr::Ternary {
                cond: 0,
                then_e: 1,
                else_e: 2,
            },
        ],
        vec![],
        vec![nv(8, false), nv(8, false), nv(1, false)],
    );
    let (t, e) = (vw(8, 0xAA), vw(8, 0xAC));
    assert_matches_oracle(&ir, 3, 8, false, &[t.clone(), e.clone(), vw(1, 1)]);
    assert_matches_oracle(&ir, 3, 8, false, &[t.clone(), e.clone(), vw(1, 0)]);
    // X cond ⇒ bitwise merge: agreeing bits pass, differing become X.
    assert_matches_oracle(&ir, 3, 8, false, &[t.clone(), e.clone(), vw_xz(1, 0, 1)]);
    // merge where branches carry X themselves.
    assert_matches_oracle(
        &ir,
        3,
        8,
        false,
        &[vw_xz(8, 0xA0, 3), vw_xz(8, 0xA0, 3), vw_xz(1, 0, 1)],
    );
}

#[test]
fn reductions_and_lognot_match_oracle() {
    for op in [
        UnOp::RedAnd,
        UnOp::RedNand,
        UnOp::RedOr,
        UnOp::RedNor,
        UnOp::RedXor,
        UnOp::RedXnor,
        UnOp::LogNot,
    ] {
        let ir = ir_of(vec![sig(0), un(op, 0)], vec![], vec![nv(8, false)]);
        for v in [
            vw(8, 0xFF),
            vw(8, 0x00),
            vw(8, 0b1010_0110),
            vw_xz(8, 0xFF, 0b1),    // X with otherwise-all-1 (AND → X, OR → 1)
            vw_xz(8, 0x00, 0b1000), // X with otherwise-all-0 (OR → X, AND → 0)
        ] {
            assert_matches_oracle(&ir, 1, 8, false, &[v.clone()]);
        }
    }
}

#[test]
fn logical_and_or_match_oracle() {
    for op in [BinOp::LogAnd, BinOp::LogOr] {
        let ir = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(8, false), nv(8, false)],
        );
        for (a, b) in [
            (vw(8, 5), vw(8, 9)),             // T,T
            (vw(8, 0), vw(8, 9)),             // F,T
            (vw(8, 5), vw(8, 0)),             // T,F
            (vw(8, 0), vw(8, 0)),             // F,F
            (vw_xz(8, 0, 1), vw(8, 9)),       // X,T
            (vw_xz(8, 0, 1), vw(8, 0)),       // X,F
            (vw_xz(8, 0, 1), vw_xz(8, 0, 2)), // X,X
            (vw_xz(8, 2, 1), vw(8, 0)),       // definite-1 + X bit = TRUE, F
        ] {
            assert_matches_oracle(&ir, 2, 8, false, &[a.clone(), b.clone()]);
        }
    }
}

#[test]
fn comparison_of_arith_results_matches_oracle() {
    // (s0 + s1) < (s2 * s3) — comparison over native sub-trees.
    let ir = ir_of(
        vec![
            sig(0),
            sig(1),
            bin(BinOp::Add, 0, 1),
            sig(2),
            sig(3),
            bin(BinOp::Mul, 3, 4),
            bin(BinOp::Lt, 2, 5),
        ],
        vec![],
        vec![nv(16, false); 4],
    );
    assert_matches_oracle(
        &ir,
        6,
        8,
        false,
        &[vw(16, 100), vw(16, 200), vw(16, 20), vw(16, 14)],
    );
    assert_matches_oracle(
        &ir,
        6,
        8,
        false,
        &[vw(16, 1000), vw(16, 2000), vw(16, 2), vw(16, 3)],
    );
}

#[test]
fn bit_select_matches_oracle() {
    // exprs: 0=sig0(16b), 1=Const#0 (offset), 2=Const#1 (width edge, =1),
    // 3 = sig0[off]. Sweep in-range bits (0/1/X at the picked bit) + OOR.
    for off in [0u64, 5, 15, 16, 200] {
        let ir = ir_of(
            vec![
                sig(0),
                Expr::Const { val: 0 },
                Expr::Const { val: 1 },
                Expr::Select {
                    base: 0,
                    offset: 1,
                    width: 2,
                    kind: SelKind::Bit,
                },
            ],
            vec![cnum(32, off), cnum(32, 1)],
            vec![nv(16, false)],
        );
        assert_matches_oracle(
            &ir,
            3,
            8,
            false,
            &[vw_xz(16, 0b1010_0101_1100_0011, 0b10_0000)],
        );
    }
}

#[test]
fn part_selects_match_oracle_incl_oor_and_xz_src() {
    // [11:4] as PartConst(off=4,w=8); s[4 +: 8]; s[11 -: 8]; plus a select
    // whose window hangs off the top (off=12 ⇒ upper bits OOR→X) and one off
    // the bottom (IdxDown off=3 ⇒ lsb=-4 ⇒ low bits OOR→X).
    for (kind, off) in [
        (SelKind::PartConst, 4u64),
        (SelKind::PartIdxUp, 4),
        (SelKind::PartIdxDown, 11),
        (SelKind::PartConst, 12),
        (SelKind::PartIdxDown, 3),
    ] {
        let ir = ir_of(
            vec![
                sig(0),
                Expr::Const { val: 0 },
                Expr::Const { val: 1 },
                Expr::Select {
                    base: 0,
                    offset: 1,
                    width: 2,
                    kind,
                },
            ],
            vec![cnum(32, off), cnum(32, 8)],
            vec![nv(16, false)],
        );
        // ctx wider than the select (32) — proves the unsigned zero-extend;
        // ctx_signed=true proves a select stays unsigned in a signed context.
        assert_matches_oracle(&ir, 3, 32, true, &[vw_xz(16, 0xA5C3, 0x0420)]);
    }
}

#[test]
fn dynamic_select_offset_from_net_matches_oracle() {
    // offset comes from a NET (exprs: 1=sig1) — in-range, OOR, and X-offset.
    let ir = ir_of(
        vec![
            sig(0),
            sig(1),
            Expr::Const { val: 0 },
            Expr::Select {
                base: 0,
                offset: 1,
                width: 2,
                kind: SelKind::PartIdxUp,
            },
        ],
        vec![cnum(32, 4)],
        vec![nv(16, false), nv(8, false)],
    );
    let src = vw_xz(16, 0xA5C3, 0x0420);
    assert_matches_oracle(&ir, 3, 16, false, &[src.clone(), vw(8, 6)]);
    assert_matches_oracle(&ir, 3, 16, false, &[src.clone(), vw(8, 14)]); // window OOR top
                                                                         // X/Z offset ⇒ the whole select reads X (zero-extended into ctx).
    assert_matches_oracle(&ir, 3, 16, false, &[src, vw_xz(8, 6, 0b1)]);
}

#[test]
fn concat_matches_oracle() {
    // {sig0(8b), sig1(4b), sig2(4b)} = 16 natural bits; parts[0] is MSB-most.
    let ir = ir_of(
        vec![
            sig(0),
            sig(1),
            sig(2),
            Expr::Concat {
                parts: vec![0, 1, 2],
            },
        ],
        vec![],
        vec![nv(8, false), nv(4, false), nv(4, false)],
    );
    // clean + X/Z-bearing parts; ctx 32 (zero-extend) and ctx_signed=true
    // (concat is unsigned regardless of context).
    assert_matches_oracle(&ir, 3, 32, true, &[vw(8, 0xAB), vw(4, 0x5), vw(4, 0xC)]);
    assert_matches_oracle(
        &ir,
        3,
        32,
        true,
        &[vw_xz(8, 0xAB, 0x0F), vw_xz(4, 0x5, 0b1000), vw(4, 0xC)],
    );
}

#[test]
fn replicate_matches_oracle() {
    // {3{sig0(5b)}} = 15 natural bits, X bits repeat with the pattern.
    let ir = ir_of(
        vec![
            sig(0),
            Expr::Const { val: 0 },
            Expr::Replicate { count: 1, value: 0 },
        ],
        vec![cnum(32, 3)],
        vec![nv(5, false)],
    );
    assert_matches_oracle(&ir, 2, 16, false, &[vw(5, 0b10110)]);
    assert_matches_oracle(&ir, 2, 16, false, &[vw_xz(5, 0b10110, 0b00100)]);
}

#[test]
fn select_of_concat_composes() {
    // {sig0, sig1}[6 +: 4] — structural ops compose inside one program.
    let ir = ir_of(
        vec![
            sig(0),
            sig(1),
            Expr::Concat { parts: vec![0, 1] },
            Expr::Const { val: 0 },
            Expr::Const { val: 1 },
            Expr::Select {
                base: 2,
                offset: 3,
                width: 4,
                kind: SelKind::PartIdxUp,
            },
        ],
        vec![cnum(32, 6), cnum(32, 4)],
        vec![nv(8, false), nv(8, false)],
    );
    assert_matches_oracle(&ir, 5, 8, false, &[vw(8, 0x3C), vw_xz(8, 0xF0, 0x0F)]);
}

#[test]
fn indexed_load_matches_oracle() {
    // mem[idx] where idx comes from a net: exprs 0=sig(1) (index), 1=mem read.
    let ir = ir_of(
        vec![
            sig(1),
            Expr::Signal {
                net: 0,
                word: Some(0),
            },
        ],
        vec![],
        vec![nv(8, false), nv(8, false)],
    );
    let mem = |idx: Value| {
        FakeMem(vec![
            FakeNet::Array(vec![vw(8, 0x11), vw(8, 0x22), vw_xz(8, 0x30, 0xF)], 8),
            FakeNet::Scalar(idx),
        ])
    };
    assert_matches_oracle_on(&ir, 1, 16, false, &mem(vw(8, 0))); // first
    assert_matches_oracle_on(&ir, 1, 16, false, &mem(vw(8, 2))); // X-bearing element
    assert_matches_oracle_on(&ir, 1, 16, false, &mem(vw(8, 200))); // OOR → all-X
    assert_matches_oracle_on(&ir, 1, 16, false, &mem(vw_xz(8, 1, 0b1))); // X idx → all-X
}
