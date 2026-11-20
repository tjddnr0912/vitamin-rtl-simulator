//! `hdl_ast::LossySpell` on the measured cells of the row's grounding, plan and premise
//! judges (§5.2 Q-LOSSYSPELL, unit U1), each source copied verbatim from
//! W/lossy/{ground,plan,premise,plan2,premise2}/cells/<id>/t.sv into
//! `testdata/lossy_cells/<stage>_<id>.sv` (ground q4_include is left out: its producer is in
//! an included file, which a parser test cannot preprocess; the cli pins carry it), and fix
//! N4's (the soundness lens's N4: the token shape alone exempted a relational tail) from
//! W/lossy/fixn4/plan/cells/<id> as `n4plan_<id>` and W/lossy/fixn4/premise/cells{,2}/<id>
//! as `n4prem_<id>`, each "before" bit the frozen printer's (lossy_bits 1228f9dc).
//!
//! No oracle: the record is which spellings the token vector holds, not a value of the
//! language. Each expected value was read off the row's rule (W/lossy/plan2/PLAN2.txt
//! section C, option (c) by RULINGS-plan R14) by an independent python implementation over
//! the raw text (W/lossy/build/cells_u1.tsv: 116 accepted cells equal to the parser's bits);
//! `None` = the parser refuses the cell (a parse error), so it carries no claim.
//! Bits: P param_xz, U unsized_xz, S sign_kw_param, L let_any, F fill_xz.
//! Unit U3: the table holds unedited (the bits of every accepted cell are U2's) ; the last
//! test runs the production entries against the test entries on every cell (premise judge 3
//! L3, L5). Unit U4 (fix N4): a fill is exempt only where the token shape holds AND the
//! parser marked it as the right side it took at one of its 8 assignment / initializer sites
//! (`lossy.rs` `assign_rhs`). Nine rows moved, each 0 -> F or X -> X | F, none the other way:
//! premise2_p9_stmt (the relational tail `b1 = b1 <= 'x;`), premise2_t4_genvar (a genvar
//! initialization), plan_f3_pval, premise_b1_paruntyped, premise_b1_parwide (a parameter
//! value), premise_b2_defparamfill (a defparam value), premise_b3_letctl, premise_b5_letparam,
//! premise2_u4_letifc (a `let` body) — the classes the plan's section 3 and main's K1 name.

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
    ("plan_f3_pval", include_str!("testdata/lossy_cells/plan_f3_pval.sv"), Some(P | F)),
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
    ("premise_b1_paruntyped", include_str!("testdata/lossy_cells/premise_b1_paruntyped.sv"), Some(P | F)),
    ("premise_b1_parwide", include_str!("testdata/lossy_cells/premise_b1_parwide.sv"), Some(P | F)),
    ("premise_b1_pattern", include_str!("testdata/lossy_cells/premise_b1_pattern.sv"), Some(F)),
    ("premise_b1_retdef", include_str!("testdata/lossy_cells/premise_b1_retdef.sv"), Some(F)),
    ("premise_b2_classparam", include_str!("testdata/lossy_cells/premise_b2_classparam.sv"), Some(P | F)),
    ("premise_b2_condlet", include_str!("testdata/lossy_cells/premise_b2_condlet.sv"), Some(L | F)),
    ("premise_b2_defparamfill", include_str!("testdata/lossy_cells/premise_b2_defparamfill.sv"), Some(P | F)),
    ("premise_b2_enum", include_str!("testdata/lossy_cells/premise_b2_enum.sv"), Some(F)),
    ("premise_b2_genif", include_str!("testdata/lossy_cells/premise_b2_genif.sv"), Some(F)),
    ("premise_b2_Gwide", include_str!("testdata/lossy_cells/premise_b2_Gwide.sv"), Some(0)),
    ("premise_b2_twostate", include_str!("testdata/lossy_cells/premise_b2_twostate.sv"), Some(F)),
    ("premise_b2_wideovr", include_str!("testdata/lossy_cells/premise_b2_wideovr.sv"), Some(P | F)),
    ("premise_b2_wideovr1", include_str!("testdata/lossy_cells/premise_b2_wideovr1.sv"), Some(0)),
    ("premise_b3_classctl", include_str!("testdata/lossy_cells/premise_b3_classctl.sv"), Some(P)),
    ("premise_b3_letctl", include_str!("testdata/lossy_cells/premise_b3_letctl.sv"), Some(L | F)),
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
    ("premise_b5_letparam", include_str!("testdata/lossy_cells/premise_b5_letparam.sv"), Some(L | F)),
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
    ("premise2_p9_stmt", include_str!("testdata/lossy_cells/premise2_p9_stmt.sv"), Some(F)),
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
    ("premise2_t4_genvar", include_str!("testdata/lossy_cells/premise2_t4_genvar.sv"), Some(F)),
    ("premise2_u1_svalocal", include_str!("testdata/lossy_cells/premise2_u1_svalocal.sv"), None),
    ("premise2_u2_letclass", include_str!("testdata/lossy_cells/premise2_u2_letclass.sv"), None),
    ("premise2_u3_letunit", include_str!("testdata/lossy_cells/premise2_u3_letunit.sv"), None),
    ("premise2_u4_letifc", include_str!("testdata/lossy_cells/premise2_u4_letifc.sv"), Some(L | F)),
    ("premise2_v1_selw", include_str!("testdata/lossy_cells/premise2_v1_selw.sv"), Some(0)),
    ("premise2_v1b_letw", include_str!("testdata/lossy_cells/premise2_v1b_letw.sv"), Some(L)),
    ("premise2_w1_wide", include_str!("testdata/lossy_cells/premise2_w1_wide.sv"), Some(0)),
    ("n4plan_r1_return", include_str!("testdata/lossy_cells/n4plan_r1_return.sv"), Some(F)),
    ("n4plan_r2_forcond", include_str!("testdata/lossy_cells/n4plan_r2_forcond.sv"), Some(F)),
    ("n4plan_r3_prop", include_str!("testdata/lossy_cells/n4plan_r3_prop.sv"), Some(F)),
    ("n4plan_r4_seq", include_str!("testdata/lossy_cells/n4plan_r4_seq.sv"), Some(F)),
    ("n4plan_r5_constraint", include_str!("testdata/lossy_cells/n4plan_r5_constraint.sv"), Some(F)),
    ("n4plan_r6_cont", include_str!("testdata/lossy_cells/n4plan_r6_cont.sv"), Some(F)),
    ("n4plan_r7_init", include_str!("testdata/lossy_cells/n4plan_r7_init.sv"), Some(F)),
    ("n4plan_r8_nbarel", include_str!("testdata/lossy_cells/n4plan_r8_nbarel.sv"), Some(F)),
    ("n4plan_r9_delay", include_str!("testdata/lossy_cells/n4plan_r9_delay.sv"), Some(F)),
    ("n4plan_r10_paren", include_str!("testdata/lossy_cells/n4plan_r10_paren.sv"), Some(F)),
    ("n4plan_r11_genfor", include_str!("testdata/lossy_cells/n4plan_r11_genfor.sv"), Some(F)),
    ("n4plan_r12_cgopt", include_str!("testdata/lossy_cells/n4plan_r12_cgopt.sv"), Some(F)),
    ("n4plan_r13_ctl", include_str!("testdata/lossy_cells/n4plan_r13_ctl.sv"), Some(0)),
    ("n4plan_r14_pass", include_str!("testdata/lossy_cells/n4plan_r14_pass.sv"), Some(F)),
    ("n4prem_d01_clk", include_str!("testdata/lossy_cells/n4prem_d01_clk.sv"), Some(F)),
    ("n4prem_d02_udp", include_str!("testdata/lossy_cells/n4prem_d02_udp.sv"), None),
    ("n4prem_d02b_udp", include_str!("testdata/lossy_cells/n4prem_d02b_udp.sv"), Some(F)),
    ("n4prem_d03_spec", include_str!("testdata/lossy_cells/n4prem_d03_spec.sv"), Some(P | F)),
    ("n4prem_d04_tdfor", include_str!("testdata/lossy_cells/n4prem_d04_tdfor.sv"), Some(0)),
    ("n4prem_d05_pasg_rel", include_str!("testdata/lossy_cells/n4prem_d05_pasg_rel.sv"), Some(F)),
    ("n4prem_d06_force_rel", include_str!("testdata/lossy_cells/n4prem_d06_force_rel.sv"), Some(F)),
    ("n4prem_d07_tfor_rel", include_str!("testdata/lossy_cells/n4prem_d07_tfor_rel.sv"), Some(F)),
    ("n4prem_d08_ffor_rel", include_str!("testdata/lossy_cells/n4prem_d08_ffor_rel.sv"), Some(F)),
    ("n4prem_d09_rwith", include_str!("testdata/lossy_cells/n4prem_d09_rwith.sv"), None),
    ("n4prem_d09b_rwith", include_str!("testdata/lossy_cells/n4prem_d09b_rwith.sv"), Some(F)),
    ("n4prem_d10_randprop", include_str!("testdata/lossy_cells/n4prem_d10_randprop.sv"), Some(0)),
    ("n4prem_d11_tdfor_rel", include_str!("testdata/lossy_cells/n4prem_d11_tdfor_rel.sv"), Some(F)),
    ("n4prem_d12_pasg", include_str!("testdata/lossy_cells/n4prem_d12_pasg.sv"), Some(0)),
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

/// The sources U3's pins add (W/lossy/premise3/cells, W/lossy/plan3/cells ; testdata/lossy_u3).
const U3_SOURCES: &[(&str, &str)] = &[
    ("a1_forms2", include_str!("testdata/lossy_u3/a1_forms2.sv")),
    ("a2_iparse", include_str!("testdata/lossy_u3/a2_iparse.sv")),
    ("a3_ielab", include_str!("testdata/lossy_u3/a3_ielab.sv")),
    ("b1_attr", include_str!("testdata/lossy_u3/b1_attr.sv")),
    (
        "k1_fnlocal",
        include_str!("testdata/lossy_u3/k1_fnlocal.sv"),
    ),
    ("k2_port", include_str!("testdata/lossy_u3/k2_port.sv")),
    ("k3_block", include_str!("testdata/lossy_u3/k3_block.sv")),
];

#[test]
fn production_entries_equal_the_test_entries() {
    // Premise judge 3 L3 and L5 (§5.2 Q-LOSSYSPELL U3): over the 132 cells and U3's
    // sources, `lex` is `lex_facts` with the fact dropped, and the production pair
    // (`lex_facts` then `parse_lexed` with the lexer's fact, cli `frontend.rs`) gives what
    // `parse_with_warnings` (the fact from the tokens) gives: the unit, every error and
    // warning, so the record too. The lexer's fact is a superset of the tokens' (it differs
    // only where every x / z literal sits in a stripped attribute instance: b1_attr).
    let all: Vec<(&str, &str)> = CELLS
        .iter()
        .map(|(id, src, _)| (*id, *src))
        .chain(U3_SOURCES.iter().copied())
        .collect();
    assert_eq!(all.len(), 167);
    let (mut with_fact, mut fact_only) = (0, Vec::new());
    for (id, src) in all {
        let (toks, errs, facts) = hdl_lexer::lex_facts(src);
        assert_eq!(hdl_lexer::lex(src), (toks.clone(), errs), "{id}: lex");
        let of_tokens = hdl_lexer::LexFacts::of_tokens(&toks).xz_literal();
        assert!(
            facts.xz_literal() || !of_tokens,
            "{id}: the fact under the tokens'"
        );
        with_fact += usize::from(facts.xz_literal());
        if facts.xz_literal() != of_tokens {
            fact_only.push(id);
        }
        assert_eq!(
            hdl_parser::parse_lexed(&toks, src, facts),
            hdl_parser::parse_with_warnings(&toks, src),
            "{id}: parse"
        );
    }
    assert_eq!(fact_only, ["b1_attr"]);
    // measured at U3: 106 of the 139 run the pass, 33 skip it ; fix N4's 28 cells each spell one.
    assert_eq!(with_fact, 134, "units whose lexer saw an x / z literal");
}
