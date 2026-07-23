use super::*;

#[test]
fn indexed_load_in_arith_matches_oracle() {
    // mem[i] + 1 — indexed read composing with the arith lane.
    let ir = ir_of(
        vec![
            sig(1),
            Expr::Signal {
                net: 0,
                word: Some(0),
            },
            Expr::Const { val: 0 },
            bin(BinOp::Add, 1, 2),
        ],
        vec![cnum(32, 1)],
        vec![nv(16, false), nv(4, false)],
    );
    let mem = FakeMem(vec![
        FakeNet::Array(vec![vw(16, 0xFFFF), vw(16, 7)], 16),
        FakeNet::Scalar(vw(4, 0)),
    ]);
    assert_matches_oracle_on(&ir, 3, 32, false, &mem);
}

#[test]
fn wide_indexed_load_matches_oracle() {
    // a 100-bit element array read lands on the WIDE stack.
    let ir = ir_of(
        vec![
            sig(1),
            Expr::Signal {
                net: 0,
                word: Some(0),
            },
        ],
        vec![],
        vec![nv(100, false), nv(8, false)],
    );
    let mem = |idx: Value| {
        FakeMem(vec![
            FakeNet::Array(
                vec![vwide(100, u64::MAX, 0xABC), vwide(100, 5, 1 << 35)],
                100,
            ),
            FakeNet::Scalar(idx),
        ])
    };
    assert_matches_oracle_on(&ir, 1, 100, false, &mem(vw(8, 1)));
    assert_matches_oracle_on(&ir, 1, 100, false, &mem(vw(8, 99))); // OOR
    assert_matches_oracle_on(&ir, 1, 100, false, &mem(vw_xz(8, 0, 1))); // X idx
}

#[test]
fn wide_arith_matches_oracle() {
    for op in [BinOp::Add, BinOp::Sub, BinOp::Mul] {
        let ir = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(100, false), nv(100, false)],
        );
        for (a, b) in [
            (vwide(100, u64::MAX, 0), vwide(100, 1, 0)), // carry crosses word 0→1
            (vwide(100, 0, 1), vwide(100, 1, 0)),        // borrow crosses back
            (vwide(100, u64::MAX, 0xF_FFFF_FFFF), vwide(100, 3, 7)), // wrap at 100
            (vwide(100, 1 << 60, 0), vwide(100, 1 << 60, 0)), // mul carries past 63
        ] {
            assert_matches_oracle(&ir, 2, 100, false, &[a.clone(), b.clone()]);
        }
        // X anywhere (here: only in word 1) poisons the whole result.
        assert_matches_oracle(
            &ir,
            2,
            100,
            false,
            &[vwide_xz(100, 5, 0, 0, 1 << 10), vwide(100, 9, 0)],
        );
    }
}

#[test]
fn wide_bitwise_not_neg_match_oracle() {
    for op in [BinOp::BitAnd, BinOp::BitOr, BinOp::BitXor, BinOp::BitXnor] {
        let ir = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(96, false), nv(96, false)],
        );
        let a = vwide_xz(96, 0xA5A5, 0xFF00, 0x0F, 0x3);
        let b = vwide_xz(96, 0x5A5A, 0x00FF, 0xF0, 0xC);
        assert_matches_oracle(&ir, 2, 96, false, &[a, b]);
    }
    let ir_not = ir_of(
        vec![sig(0), un(UnOp::BitNot, 0)],
        vec![],
        vec![nv(96, false)],
    );
    assert_matches_oracle(&ir_not, 1, 96, false, &[vwide_xz(96, 0xA5, 0x10, 0xF, 0x1)]);
    let ir_neg = ir_of(
        vec![sig(0), un(UnOp::Minus, 0)],
        vec![],
        vec![nv(100, false)],
    );
    assert_matches_oracle(&ir_neg, 1, 100, false, &[vwide(100, 0, 1)]); // borrow chain
    assert_matches_oracle(&ir_neg, 1, 100, false, &[vwide(100, 5, 7)]);
    assert_matches_oracle(&ir_neg, 1, 100, false, &[vwide_xz(100, 5, 0, 1, 0)]);
    // X poison
}

#[test]
fn wide_cmp_and_equality_match_oracle() {
    // signed 100-bit: sign bit is bit 99 (word-1 bit 35).
    let neg = vwide_s(100, 5, (1 << 35) | 3); // negative (bit 99 set)
    let pos = vwide_s(100, u64::MAX, 0x3_FFFF_FFFF); // large positive
    for op in [BinOp::Lt, BinOp::Le, BinOp::Gt, BinOp::Ge] {
        let irs = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(100, true), nv(100, true)],
        );
        assert_matches_oracle(&irs, 2, 8, false, &[neg.clone(), pos.clone()]);
        assert_matches_oracle(&irs, 2, 8, false, &[pos.clone(), neg.clone()]);
        assert_matches_oracle(&irs, 2, 8, false, &[neg.clone(), neg.clone()]);
        // unsigned pair: same bits compare the other way around.
        let iru = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(100, false), nv(100, false)],
        );
        let (a, b) = (
            vwide(100, 5, (1 << 35) | 3),
            vwide(100, u64::MAX, 0x3_FFFF_FFFF),
        );
        assert_matches_oracle(&iru, 2, 8, false, &[a.clone(), b.clone()]);
        // X in word 1 only → 1-bit X.
        assert_matches_oracle(&iru, 2, 8, false, &[vwide_xz(100, 0, 0, 0, 1), b]);
    }
    for op in [BinOp::Eq, BinOp::Ne, BinOp::CaseEq, BinOp::CaseNe] {
        let ir = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(128, false), nv(128, false)],
        );
        let same = vwide(128, 0xDEAD_BEEF, 0xFEED_F00D);
        let diff_hi = vwide(128, 0xDEAD_BEEF, 0xFEED_F00E); // differs only in word 1
        assert_matches_oracle(&ir, 2, 8, false, &[same.clone(), same.clone()]);
        assert_matches_oracle(&ir, 2, 8, false, &[same.clone(), diff_hi]);
        // matching X positions: ==/!= → X, ===/!== → equal.
        let x1 = vwide_xz(128, 0xA0, 0xB0, 0xF, 0xF0);
        assert_matches_oracle(&ir, 2, 8, false, &[x1.clone(), x1.clone()]);
        assert_matches_oracle(&ir, 2, 8, false, &[x1, same]);
    }
}

#[test]
fn wide_shifts_match_oracle() {
    for op in [BinOp::Shl, BinOp::Shr, BinOp::AShr] {
        let ir = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(100, true), nv(8, false)],
        );
        let x = vwide_s(100, 0xDEAD_BEEF_CAFE_F00D, (1 << 35) | 0x123); // bit 99 set
        for amt in [0u64, 1, 37, 63, 64, 65, 99, 100, 127, 200] {
            assert_matches_oracle(&ir, 2, 100, false, &[x.clone(), vw(8, amt)]);
            assert_matches_oracle(&ir, 2, 100, true, &[x.clone(), vw(8, amt)]);
        }
        // X amount → all-X; X MSB on >>> fills X.
        assert_matches_oracle(&ir, 2, 100, false, &[x.clone(), vw_xz(8, 2, 1)]);
        assert_matches_oracle(
            &ir,
            2,
            100,
            false,
            &[vwide_xz(100, 1, 0, 0, 1 << 35), vw(8, 3)],
        );
    }
}

#[test]
fn wide_divmod_match_oracle() {
    for op in [BinOp::Div, BinOp::Mod] {
        let ir = ir_of(
            vec![sig(0), sig(1), bin(op, 0, 1)],
            vec![],
            vec![nv(128, false), nv(128, false)],
        );
        let big = vwide(128, 0x1234_5678_9ABC_DEF0, 0xFFFF_0000_1111_2222);
        assert_matches_oracle(&ir, 2, 128, false, &[big.clone(), vwide(128, 7, 0)]);
        assert_matches_oracle(&ir, 2, 128, false, &[big.clone(), vwide(128, 0, 3)]);
        assert_matches_oracle(&ir, 2, 128, false, &[vwide(128, 7, 0), big.clone()]);
        assert_matches_oracle(&ir, 2, 128, false, &[big, vwide(128, 0, 0)]);
        // /0 → X
    }
}

#[test]
fn wide_ternary_matches_oracle() {
    // 1-bit cond steering 100-bit branches (clean, X-cond merge).
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
        vec![nv(100, false), nv(100, false), nv(1, false)],
    );
    let (t, e) = (vwide(100, 0xAAAA, 0xA), vwide(100, 0xAAAC, 0xC));
    assert_matches_oracle(&ir, 3, 100, false, &[t.clone(), e.clone(), vw(1, 1)]);
    assert_matches_oracle(&ir, 3, 100, false, &[t.clone(), e.clone(), vw(1, 0)]);
    assert_matches_oracle(&ir, 3, 100, false, &[t, e, vw_xz(1, 0, 1)]);
    // WIDE cond whose only definite-1 lives in word 1.
    let ir_wc = ir_of(
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
        vec![nv(100, false), nv(100, false), nv(100, false)],
    );
    for cond in [
        vwide(100, 0, 1 << 20),         // true via word 1
        vwide(100, 0, 0),               // false
        vwide_xz(100, 0, 0, 0, 1 << 5), // unknown via word 1
    ] {
        assert_matches_oracle(
            &ir_wc,
            3,
            100,
            false,
            &[vwide(100, 1, 2), vwide(100, 3, 4), cond],
        );
    }
}

#[test]
fn wide_reductions_and_lognot_match_oracle() {
    for op in [
        UnOp::RedAnd,
        UnOp::RedNand,
        UnOp::RedOr,
        UnOp::RedNor,
        UnOp::RedXor,
        UnOp::RedXnor,
        UnOp::LogNot,
    ] {
        let ir = ir_of(vec![sig(0), un(op, 0)], vec![], vec![nv(128, false)]);
        for v in [
            vwide(128, u64::MAX, u64::MAX),     // all ones
            vwide(128, u64::MAX, u64::MAX - 1), // single 0 in word 1
            vwide(128, 0, 0),                   // all zeros
            vwide(128, 1, 1 << 40),             // parity across words
            vwide_xz(128, u64::MAX, 0, 0, 1),   // X only in word 1
            vwide_xz(128, 0, 0, 0, 1 << 63),    // X with otherwise-all-0
        ] {
            assert_matches_oracle(&ir, 1, 8, false, &[v.clone()]);
        }
    }
}

#[test]
fn wide_const_matches_oracle() {
    // 128-bit const + 128-bit signal — WConst materialization.
    let ir = ir_of(
        vec![Expr::Const { val: 0 }, sig(0), bin(BinOp::Add, 0, 1)],
        vec![cnum2(128, u64::MAX, 0x7)],
        vec![nv(128, false)],
    );
    assert_matches_oracle(&ir, 2, 128, false, &[vwide(128, 1, 0)]);
}

#[test]
fn wide_compare_feeds_narrow_context() {
    // (a & b) != 0 over 100 bits → 1-bit result zero-extended into 8-bit ctx.
    let ir = ir_of(
        vec![
            sig(0),
            sig(1),
            bin(BinOp::BitAnd, 0, 1),
            Expr::Const { val: 0 },
            bin(BinOp::Ne, 2, 3),
        ],
        vec![cnum2(100, 0, 0)],
        vec![nv(100, false), nv(100, false)],
    );
    assert_matches_oracle(
        &ir,
        4,
        8,
        false,
        &[vwide(100, 0, 1 << 30), vwide(100, 0, 1 << 30)],
    );
    assert_matches_oracle(
        &ir,
        4,
        8,
        false,
        &[vwide(100, 0, 1 << 30), vwide(100, 1, 0)],
    );
}

#[test]
fn wide_lane_bails_outside_subset() {
    let wt_of = |ir: &SimIr| WidthTable::build(ir, &crate::FuncTable::new());
    // SIGNED >64-bit arith: the oracle X-poisons via a different route —
    // conservatively out of the native subset.
    let ir = ir_of(
        vec![sig(0), sig(1), bin(BinOp::Add, 0, 1)],
        vec![],
        vec![nv(100, true), nv(100, true)],
    );
    assert!(try_compile(&ir, &wt_of(&ir), 2, 100, true).is_none());
    // shift AMOUNT wider than 64 bits.
    let ir = ir_of(
        vec![sig(0), sig(1), bin(BinOp::Shl, 0, 1)],
        vec![],
        vec![nv(32, false), nv(100, false)],
    );
    assert!(try_compile(&ir, &wt_of(&ir), 2, 32, false).is_none());
    // select over a >128-bit source stays oracle-bound (the v6 ④ wide
    // structural trio runs to 128; beyond it the whole tree bails).
    let ir = ir_of(
        vec![
            sig(0),
            Expr::Const { val: 0 },
            Expr::Const { val: 1 },
            Expr::Select {
                base: 0,
                offset: 1,
                width: 2,
                kind: SelKind::PartConst,
            },
        ],
        vec![cnum(32, 4), cnum(32, 8)],
        vec![nv(200, false)],
    );
    assert!(try_compile(&ir, &wt_of(&ir), 3, 8, false).is_none());
    // select OFFSET wider than 64 bits (base narrow).
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
        vec![nv(16, false), nv(100, false)],
    );
    assert!(try_compile(&ir, &wt_of(&ir), 3, 16, false).is_none());
    // logical &&/|| over a wide operand.
    let ir = ir_of(
        vec![sig(0), sig(1), bin(BinOp::LogAnd, 0, 1)],
        vec![],
        vec![nv(100, false), nv(8, false)],
    );
    assert!(try_compile(&ir, &wt_of(&ir), 2, 8, false).is_none());
    // narrow-result ternary steered by a WIDE cond.
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
        vec![nv(8, false), nv(8, false), nv(100, false)],
    );
    assert!(try_compile(&ir, &wt_of(&ir), 3, 8, false).is_none());
    // array index expr wider than 64 bits.
    let ir = ir_of(
        vec![
            sig(1),
            Expr::Signal {
                net: 0,
                word: Some(0),
            },
        ],
        vec![],
        vec![nv(8, false), nv(100, false)],
    );
    assert!(try_compile(&ir, &wt_of(&ir), 1, 8, false).is_none());
}

// ── v6 ④ wide structural trio (select/concat/replicate to 128 bits) ──

#[test]
fn wide_select_from_wide_base_matches_oracle() {
    // 100-bit base; windows crossing the word-0/1 boundary, hanging off the
    // top (OOR→X), and an X/Z offset — narrow result (sel_w 16).
    for off in [0u64, 56, 60, 90, 96, 200] {
        let ir = ir_of(
            vec![
                sig(0),
                Expr::Const { val: 0 },
                Expr::Const { val: 1 },
                Expr::Select {
                    base: 0,
                    offset: 1,
                    width: 2,
                    kind: SelKind::PartConst,
                },
            ],
            vec![cnum(32, off), cnum(32, 16)],
            vec![nv(100, false)],
        );
        assert_matches_oracle(
            &ir,
            3,
            32,
            true,
            &[vwide_xz(
                100,
                0xA5C3_1234_DEAD_BEEF,
                0x9_ABCD,
                1 << 62,
                0b1010,
            )],
        );
    }
}

#[test]
fn wide_select_wide_result_matches_oracle() {
    // sel_w 100 from a 120-bit base (wide → wide) AND from a 32-bit base
    // (narrow base, wide result: everything beyond bit 31 reads X).
    for (base_w, src) in [
        (120u32, vwide_xz(120, 77, 0xFFFF_0000_0000_0001, 0, 1 << 50)),
        (32u32, vw_xz(32, 0xA5C3_0F0F, 0x10)),
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
                    kind: SelKind::PartConst,
                },
            ],
            vec![cnum(32, 4), cnum(32, 100)],
            vec![nv(base_w, false)],
        );
        assert_matches_oracle(&ir, 3, 100, false, &[src]);
    }
}

#[test]
fn wide_select_idxdown_negative_lsb_matches_oracle() {
    // s[3 -: 8] on a 100-bit base ⇒ lsb = −4 ⇒ low half OOR→X.
    let ir = ir_of(
        vec![
            sig(0),
            Expr::Const { val: 0 },
            Expr::Const { val: 1 },
            Expr::Select {
                base: 0,
                offset: 1,
                width: 2,
                kind: SelKind::PartIdxDown,
            },
        ],
        vec![cnum(32, 3), cnum(32, 8)],
        vec![nv(100, false)],
    );
    assert_matches_oracle(&ir, 3, 8, false, &[vwide(100, 0xCAFE, 0x3)]);
}

#[test]
fn wide_concat_folds_match_oracle() {
    // {a(64), b(36)} = 100: the fold CROSSES 64 on the second part
    // (acc narrow + part narrow → wide).
    let ir = ir_of(
        vec![sig(0), sig(1), Expr::Concat { parts: vec![0, 1] }],
        vec![],
        vec![nv(64, false), nv(36, false)],
    );
    assert_matches_oracle(
        &ir,
        2,
        100,
        true,
        &[
            vw_xz(64, 0xDEAD_BEEF_A5C3_1234, 1 << 40),
            vw(36, 0xF_F00F_F00F),
        ],
    );
    // {w(100), n(20)} = 120: acc already wide + narrow part.
    let ir2 = ir_of(
        vec![sig(0), sig(1), Expr::Concat { parts: vec![0, 1] }],
        vec![],
        vec![nv(100, false), nv(20, false)],
    );
    assert_matches_oracle(
        &ir2,
        2,
        120,
        false,
        &[vwide_xz(100, 1, 0xF_0000_0001, 0, 1 << 35), vw(20, 0xABCDE)],
    );
    // {n(20), w(100)} = 120: narrow acc + WIDE part.
    let ir3 = ir_of(
        vec![sig(0), sig(1), Expr::Concat { parts: vec![1, 0] }],
        vec![],
        vec![nv(100, false), nv(20, false)],
    );
    assert_matches_oracle(
        &ir3,
        2,
        120,
        false,
        &[vwide(100, u64::MAX, 0x9_9999), vw_xz(20, 0x12345, 0b100)],
    );
}

#[test]
fn wide_replicate_matches_oracle() {
    // {3{s(40)}} = 120 natural bits — X bits repeat with the pattern.
    let ir = ir_of(
        vec![sig(0), Expr::Replicate { count: 1, value: 0 }],
        vec![],
        vec![nv(40, false)],
    );
    // count edge is a const-expr edge: build via cnum like the width edges.
    let ir = {
        let mut ir = ir;
        ir.consts.push(cnum(32, 3));
        ir.exprs[1] = Expr::Replicate { count: 2, value: 0 };
        ir.exprs.push(Expr::Const { val: 0 });
        ir
    };
    assert_matches_oracle(&ir, 1, 128, false, &[vw_xz(40, 0xAB_CD12_3456, 0xF0)]);
}
