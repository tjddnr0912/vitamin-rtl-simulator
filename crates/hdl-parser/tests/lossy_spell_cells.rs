//! `hdl_ast::LossySpell` on the measured cells of the row's grounding, plan and premise
//! judges (§5.2 Q-LOSSYSPELL, unit U1), each source copied verbatim from
//! W/lossy/{ground,plan,premise,plan2,premise2}/cells/<id>/t.sv into
//! `testdata/lossy_cells/<stage>_<id>.sv` (ground q4_include is left out: its producer is in
//! an included file, which a parser test cannot preprocess; the cli pins carry it).
//!
//! No oracle: the record is which spellings the token vector holds, not a value of the
//! language. Each expected value was read off the row's rule (W/lossy/plan2/PLAN2.txt
//! section C, option (c) by RULINGS-plan R14) by an independent python implementation over
//! the raw text (W/lossy/build/cells_u1.tsv: 116 accepted cells equal to the parser's bits);
//! `None` = the parser refuses the cell (a parse error), so it carries no claim.
//! Bits: P param_xz, U unsized_xz, S sign_kw_param, L let_any, F fill_xz.

use hdl_ast::{LossySpell, TopItem};

const P: u8 = 1;
const U: u8 = 2;
const S: u8 = 4;
const L: u8 = 8;
const F: u8 = 16;

fn mask(ls: LossySpell) -> u8 {
    [
        (ls.param_xz, P),
        (ls.unsized_xz, U),
        (ls.sign_kw_param, S),
        (ls.let_any, L),
        (ls.fill_xz, F),
    ]
    .iter()
    .filter(|(b, _)| *b)
    .fold(0, |m, (_, k)| m | k)
}

/// The bits of an accepted source, or `None` when the parser reports an error.
fn bits(src: &str) -> Option<u8> {
    let (toks, le) = hdl_lexer::lex(src);
    if !le.is_empty() {
        return None;
    }
    let (su, errs, _) = hdl_parser::parse_with_warnings(&toks, src);
    if !errs.is_empty() {
        return None;
    }
    let found: Vec<LossySpell> = su?
        .items
        .iter()
        .filter_map(|i| match i {
            TopItem::LossySpell(ls) => Some(*ls),
            _ => None,
        })
        .collect();
    assert!(found.len() <= 1);
    Some(found.first().map_or(0, |ls| mask(*ls)))
}

#[rustfmt::skip]
const CELLS: &[(&str, &str, Option<u8>)] = &[
    ("ground_p1_decl_wide_x", include_str!("testdata/lossy_cells/ground_p1_decl_wide_x.sv"), Some(P)),
    ("ground_p1_decl_x", include_str!("testdata/lossy_cells/ground_p1_decl_x.sv"), Some(P)),
    ("ground_p1_defparam_x", include_str!("testdata/lossy_cells/ground_p1_defparam_x.sv"), Some(P)),
    ("ground_p1_G_x", include_str!("testdata/lossy_cells/ground_p1_G_x.sv"), Some(0)),
    ("ground_p1_ovr_bit_x", include_str!("testdata/lossy_cells/ground_p1_ovr_bit_x.sv"), Some(P)),
    ("ground_p1_ovr_k", include_str!("testdata/lossy_cells/ground_p1_ovr_k.sv"), Some(0)),
    ("ground_p1_ovr_untyped_x", include_str!("testdata/lossy_cells/ground_p1_ovr_untyped_x.sv"), Some(P)),
    ("ground_p1_ovr_x", include_str!("testdata/lossy_cells/ground_p1_ovr_x.sv"), Some(P)),
    ("ground_p2_fill_x", include_str!("testdata/lossy_cells/ground_p2_fill_x.sv"), Some(0)),
    ("ground_p2_sized_x", include_str!("testdata/lossy_cells/ground_p2_sized_x.sv"), Some(0)),
    ("ground_p2_unszb_param", include_str!("testdata/lossy_cells/ground_p2_unszb_param.sv"), Some(P | U)),
    ("ground_p2_unszb_x", include_str!("testdata/lossy_cells/ground_p2_unszb_x.sv"), Some(U)),
    ("ground_p3_no_kw", include_str!("testdata/lossy_cells/ground_p3_no_kw.sv"), Some(0)),
    ("ground_p3_sgn_kw", include_str!("testdata/lossy_cells/ground_p3_sgn_kw.sv"), Some(S)),
    ("ground_p3_uns_kw", include_str!("testdata/lossy_cells/ground_p3_uns_kw.sv"), Some(S)),
    ("ground_p4_let_typed", include_str!("testdata/lossy_cells/ground_p4_let_typed.sv"), Some(L)),
    ("ground_p4_let_untyped", include_str!("testdata/lossy_cells/ground_p4_let_untyped.sv"), Some(L)),
    ("ground_p5_noseed", include_str!("testdata/lossy_cells/ground_p5_noseed.sv"), Some(0)),
    ("ground_p5_seed", include_str!("testdata/lossy_cells/ground_p5_seed.sv"), Some(0)),
    ("ground_p6_cfn_lsb", include_str!("testdata/lossy_cells/ground_p6_cfn_lsb.sv"), Some(0)),
    ("ground_p6_cfn_lsb0", include_str!("testdata/lossy_cells/ground_p6_cfn_lsb0.sv"), Some(0)),
    ("ground_p7_f2cmp", include_str!("testdata/lossy_cells/ground_p7_f2cmp.sv"), Some(0)),
    ("ground_p8_errrec", include_str!("testdata/lossy_cells/ground_p8_errrec.sv"), Some(P)),
    ("ground_q4_genif_off", include_str!("testdata/lossy_cells/ground_q4_genif_off.sv"), Some(S)),
    ("ground_q4_pkg_unused", include_str!("testdata/lossy_cells/ground_q4_pkg_unused.sv"), Some(S)),
    ("ground_q4_two_files", include_str!("testdata/lossy_cells/ground_q4_two_files.sv"), Some(P)),
    ("ground_q4_two_inst", include_str!("testdata/lossy_cells/ground_q4_two_inst.sv"), Some(P)),
    ("ground_q4_uninst_mod", include_str!("testdata/lossy_cells/ground_q4_uninst_mod.sv"), Some(S)),
    ("plan_f1_nba", include_str!("testdata/lossy_cells/plan_f1_nba.sv"), Some(0)),
    ("plan_f10_enumx", include_str!("testdata/lossy_cells/plan_f10_enumx.sv"), Some(0)),
    ("plan_f2_tern", include_str!("testdata/lossy_cells/plan_f2_tern.sv"), Some(F)),
    ("plan_f3_pval", include_str!("testdata/lossy_cells/plan_f3_pval.sv"), Some(P)),
    ("plan_f4_ovr", include_str!("testdata/lossy_cells/plan_f4_ovr.sv"), Some(P | F)),
    ("plan_f5_binop", include_str!("testdata/lossy_cells/plan_f5_binop.sv"), Some(F)),
    ("plan_f6_arg", include_str!("testdata/lossy_cells/plan_f6_arg.sv"), Some(F)),
    ("plan_f7_sgnrange", include_str!("testdata/lossy_cells/plan_f7_sgnrange.sv"), Some(0)),
    ("plan_f8_cfnx", include_str!("testdata/lossy_cells/plan_f8_cfnx.sv"), Some(0)),
    ("plan_f9_Gfill", include_str!("testdata/lossy_cells/plan_f9_Gfill.sv"), Some(0)),
    ("premise_b1_bitsnet", include_str!("testdata/lossy_cells/premise_b1_bitsnet.sv"), Some(F)),
    ("premise_b1_case", include_str!("testdata/lossy_cells/premise_b1_case.sv"), Some(F)),
    ("premise_b1_castsign", include_str!("testdata/lossy_cells/premise_b1_castsign.sv"), Some(F)),
    ("premise_b1_cmpshift", include_str!("testdata/lossy_cells/premise_b1_cmpshift.sv"), Some(F)),
    ("premise_b1_concat", include_str!("testdata/lossy_cells/premise_b1_concat.sv"), Some(F)),
    ("premise_b1_constfn", include_str!("testdata/lossy_cells/premise_b1_constfn.sv"), Some(F)),
    ("premise_b1_inside", include_str!("testdata/lossy_cells/premise_b1_inside.sv"), Some(F)),
    ("premise_b1_parexpr", include_str!("testdata/lossy_cells/premise_b1_parexpr.sv"), Some(P | F)),
    ("premise_b1_paruntyped", include_str!("testdata/lossy_cells/premise_b1_paruntyped.sv"), Some(P)),
    ("premise_b1_parwide", include_str!("testdata/lossy_cells/premise_b1_parwide.sv"), Some(P)),
    ("premise_b1_pattern", include_str!("testdata/lossy_cells/premise_b1_pattern.sv"), Some(F)),
    ("premise_b1_retdef", include_str!("testdata/lossy_cells/premise_b1_retdef.sv"), Some(F)),
    ("premise_b2_classparam", include_str!("testdata/lossy_cells/premise_b2_classparam.sv"), Some(P | F)),
    ("premise_b2_condlet", include_str!("testdata/lossy_cells/premise_b2_condlet.sv"), Some(L | F)),
    ("premise_b2_defparamfill", include_str!("testdata/lossy_cells/premise_b2_defparamfill.sv"), Some(P)),
    ("premise_b2_enum", include_str!("testdata/lossy_cells/premise_b2_enum.sv"), Some(F)),
    ("premise_b2_genif", include_str!("testdata/lossy_cells/premise_b2_genif.sv"), Some(F)),
    ("premise_b2_Gwide", include_str!("testdata/lossy_cells/premise_b2_Gwide.sv"), Some(0)),
    ("premise_b2_twostate", include_str!("testdata/lossy_cells/premise_b2_twostate.sv"), Some(F)),
    ("premise_b2_wideovr", include_str!("testdata/lossy_cells/premise_b2_wideovr.sv"), Some(P | F)),
    ("premise_b2_wideovr1", include_str!("testdata/lossy_cells/premise_b2_wideovr1.sv"), Some(0)),
    ("premise_b3_classctl", include_str!("testdata/lossy_cells/premise_b3_classctl.sv"), Some(P)),
    ("premise_b3_letctl", include_str!("testdata/lossy_cells/premise_b3_letctl.sv"), Some(L)),
    ("premise_b3_letfill", include_str!("testdata/lossy_cells/premise_b3_letfill.sv"), Some(L | F)),
    ("premise_b4_arrsigned", include_str!("testdata/lossy_cells/premise_b4_arrsigned.sv"), None),
    ("premise_b4_blocklocal", include_str!("testdata/lossy_cells/premise_b4_blocklocal.sv"), None),
    ("premise_b4_classcollide", include_str!("testdata/lossy_cells/premise_b4_classcollide.sv"), Some(P)),
    ("premise_b4_comment", include_str!("testdata/lossy_cells/premise_b4_comment.sv"), None),
    ("premise_b4_fnlocal", include_str!("testdata/lossy_cells/premise_b4_fnlocal.sv"), None),
    ("premise_b4_letblock", include_str!("testdata/lossy_cells/premise_b4_letblock.sv"), None),
    ("premise_b4_letsites", include_str!("testdata/lossy_cells/premise_b4_letsites.sv"), Some(L)),
    ("premise_b4_sgnrange", include_str!("testdata/lossy_cells/premise_b4_sgnrange.sv"), Some(0)),
    ("premise_b5_fndefparam", include_str!("testdata/lossy_cells/premise_b5_fndefparam.sv"), Some(F)),
    ("premise_b5_G_q", include_str!("testdata/lossy_cells/premise_b5_G_q.sv"), Some(0)),
    ("premise_b5_G_unszb", include_str!("testdata/lossy_cells/premise_b5_G_unszb.sv"), Some(0)),
    ("premise_b5_G_upper", include_str!("testdata/lossy_cells/premise_b5_G_upper.sv"), Some(0)),
    ("premise_b5_letparam", include_str!("testdata/lossy_cells/premise_b5_letparam.sv"), Some(L)),
    ("premise_b5_lex", include_str!("testdata/lossy_cells/premise_b5_lex.sv"), Some(U)),
    ("premise_b7_classbodylp", include_str!("testdata/lossy_cells/premise_b7_classbodylp.sv"), None),
    ("premise_b7_classsign", include_str!("testdata/lossy_cells/premise_b7_classsign.sv"), None),
    ("premise_b8_writes", include_str!("testdata/lossy_cells/premise_b8_writes.sv"), Some(F)),
    ("plan2_k1_unary", include_str!("testdata/lossy_cells/plan2_k1_unary.sv"), Some(F)),
    ("plan2_k10_class", include_str!("testdata/lossy_cells/plan2_k10_class.sv"), Some(F)),
    ("plan2_k10b_class", include_str!("testdata/lossy_cells/plan2_k10b_class.sv"), Some(F)),
    ("plan2_k11_uconcat", include_str!("testdata/lossy_cells/plan2_k11_uconcat.sv"), Some(F)),
    ("plan2_k2_stmt", include_str!("testdata/lossy_cells/plan2_k2_stmt.sv"), Some(F)),
    ("plan2_k3_seqprop", include_str!("testdata/lossy_cells/plan2_k3_seqprop.sv"), Some(F)),
    ("plan2_k3b_prop", include_str!("testdata/lossy_cells/plan2_k3b_prop.sv"), Some(F)),
    ("plan2_k4_port", include_str!("testdata/lossy_cells/plan2_k4_port.sv"), Some(F)),
    ("plan2_k4b_port", include_str!("testdata/lossy_cells/plan2_k4b_port.sv"), Some(F)),
    ("plan2_k5_typedefx", include_str!("testdata/lossy_cells/plan2_k5_typedefx.sv"), None),
    ("plan2_k6_method", include_str!("testdata/lossy_cells/plan2_k6_method.sv"), Some(F)),
    ("plan2_k7_comb", include_str!("testdata/lossy_cells/plan2_k7_comb.sv"), None),
    ("plan2_k7b_comb", include_str!("testdata/lossy_cells/plan2_k7b_comb.sv"), Some(F)),
    ("plan2_k8_q1", include_str!("testdata/lossy_cells/plan2_k8_q1.sv"), Some(0)),
    ("plan2_k9_qxz", include_str!("testdata/lossy_cells/plan2_k9_qxz.sv"), Some(F)),
    ("premise2_p1_decl", include_str!("testdata/lossy_cells/premise2_p1_decl.sv"), Some(0)),
    ("premise2_p10_scopes", include_str!("testdata/lossy_cells/premise2_p10_scopes.sv"), Some(0)),
    ("premise2_p2_fn", include_str!("testdata/lossy_cells/premise2_p2_fn.sv"), Some(0)),
    ("premise2_p2b_frames", include_str!("testdata/lossy_cells/premise2_p2b_frames.sv"), Some(0)),
    ("premise2_p3_class", include_str!("testdata/lossy_cells/premise2_p3_class.sv"), Some(0)),
    ("premise2_p4_hier", include_str!("testdata/lossy_cells/premise2_p4_hier.sv"), Some(0)),
    ("premise2_p5_ifc", include_str!("testdata/lossy_cells/premise2_p5_ifc.sv"), Some(0)),
    ("premise2_p5b_cb", include_str!("testdata/lossy_cells/premise2_p5b_cb.sv"), Some(0)),
    ("premise2_p6_types", include_str!("testdata/lossy_cells/premise2_p6_types.sv"), Some(0)),
    ("premise2_p7_width", include_str!("testdata/lossy_cells/premise2_p7_width.sv"), Some(0)),
    ("premise2_p8_illegal", include_str!("testdata/lossy_cells/premise2_p8_illegal.sv"), Some(0)),
    ("premise2_p8b_enumstr", include_str!("testdata/lossy_cells/premise2_p8b_enumstr.sv"), Some(0)),
    ("premise2_p9_stmt", include_str!("testdata/lossy_cells/premise2_p9_stmt.sv"), Some(0)),
    ("premise2_q1_late", include_str!("testdata/lossy_cells/premise2_q1_late.sv"), Some(0)),
    ("premise2_q2_ovr", include_str!("testdata/lossy_cells/premise2_q2_ovr.sv"), Some(0)),
    ("premise2_q2g_G", include_str!("testdata/lossy_cells/premise2_q2g_G.sv"), Some(0)),
    ("premise2_q3_misc", include_str!("testdata/lossy_cells/premise2_q3_misc.sv"), Some(0)),
    ("premise2_q3b_misc", include_str!("testdata/lossy_cells/premise2_q3b_misc.sv"), Some(0)),
    ("premise2_r1_class2", include_str!("testdata/lossy_cells/premise2_r1_class2.sv"), Some(0)),
    ("premise2_r2_more", include_str!("testdata/lossy_cells/premise2_r2_more.sv"), Some(0)),
    ("premise2_r2b_usdef", include_str!("testdata/lossy_cells/premise2_r2b_usdef.sv"), None),
    ("premise2_r2c_prog", include_str!("testdata/lossy_cells/premise2_r2c_prog.sv"), Some(0)),
    ("premise2_s1_hiersel", include_str!("testdata/lossy_cells/premise2_s1_hiersel.sv"), Some(0)),
    ("premise2_s2_nets", include_str!("testdata/lossy_cells/premise2_s2_nets.sv"), Some(0)),
    ("premise2_s3_typeop", include_str!("testdata/lossy_cells/premise2_s3_typeop.sv"), Some(0)),
    ("premise2_t1_cfsized", include_str!("testdata/lossy_cells/premise2_t1_cfsized.sv"), Some(0)),
    ("premise2_t1b_fndef", include_str!("testdata/lossy_cells/premise2_t1b_fndef.sv"), Some(0)),
    ("premise2_t1c_enum", include_str!("testdata/lossy_cells/premise2_t1c_enum.sv"), Some(0)),
    ("premise2_t2_config", include_str!("testdata/lossy_cells/premise2_t2_config.sv"), None),
    ("premise2_t3_hashnoparen", include_str!("testdata/lossy_cells/premise2_t3_hashnoparen.sv"), None),
    ("premise2_t4_genvar", include_str!("testdata/lossy_cells/premise2_t4_genvar.sv"), Some(0)),
    ("premise2_u1_svalocal", include_str!("testdata/lossy_cells/premise2_u1_svalocal.sv"), None),
    ("premise2_u2_letclass", include_str!("testdata/lossy_cells/premise2_u2_letclass.sv"), None),
    ("premise2_u3_letunit", include_str!("testdata/lossy_cells/premise2_u3_letunit.sv"), None),
    ("premise2_u4_letifc", include_str!("testdata/lossy_cells/premise2_u4_letifc.sv"), Some(L)),
    ("premise2_v1_selw", include_str!("testdata/lossy_cells/premise2_v1_selw.sv"), Some(0)),
    ("premise2_v1b_letw", include_str!("testdata/lossy_cells/premise2_v1b_letw.sv"), Some(L)),
    ("premise2_w1_wide", include_str!("testdata/lossy_cells/premise2_w1_wide.sv"), Some(0)),
];

#[test]
fn every_measured_cell_has_its_bits() {
    let mut wrong = Vec::new();
    for (id, src, want) in CELLS {
        let got = bits(src);
        if got != *want {
            wrong.push(format!("{id}: got {got:?}, want {want:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_fill_rule_cells() {
    // PLAN2 section A's (c) row: plain fills false, the counted positions true, the let
    // fill held by let_any, the EH1 shape all-false.
    let get = |k: &str| CELLS.iter().find(|c| c.0 == k).map(|c| bits(c.1)).unwrap();
    for k in ["plan_f1_nba", "ground_p2_fill_x"] {
        assert_eq!(get(k).map(|b| b & F), Some(0), "{k}");
    }
    // b8_writes's plain forms (`s.a = 'x;`, `mem[2] <= 'z;`, `pa[1] = 'z;`) are not counted;
    // its task argument `tk('x)` is (a call argument, not the plain shape), so the cell is
    // true; k2_stmt likewise (`c ? '1 : 'z`, `<= #1 'z`, `|= 'z`).
    for k in ["premise_b8_writes", "plan2_k2_stmt"] {
        assert_eq!(get(k).map(|b| b & F), Some(F), "{k}");
    }
    for k in [
        "plan_f2_tern",
        "premise_b1_case",
        "premise_b1_inside",
        "plan2_k9_qxz",
        "plan2_k10b_class",
    ] {
        assert_eq!(get(k).map(|b| b & F), Some(F), "{k}");
    }
    assert_eq!(get("premise_b3_letfill").map(|b| b & L), Some(L));
    assert_eq!(get("premise_b4_classcollide").map(|b| b & P), Some(P));
}
