//! The generate-case arm choice (IEEE 1800-2017 §27.5): the first item whose label
//! matches the case expression, else `default`, else no arm.
//!
//! §12.5 compares the case expression with each item expression by CASE EQUALITY — every
//! bit in all four states, so an x-valued label is a defined non-match against a known
//! case expression, never an unknown — after making "the length of all the case item
//! expressions, as well as the case expression, … equal to the length of the longest",
//! unsigned "if any of these expressions is unsigned". The label is therefore read in the
//! 4-state bit domain at its full width (`const_wide`), not as an i64: the i64 fold has no
//! x/z and no width, so a label only the bit domain can read (a 65-bit parameter, a
//! string-literal sum `"a" + 1`, `$isunknown(…)`) used to be skipped as a non-match and
//! the design took `default` at exit 0, and `-1` never matched `32'hFFFFFFFF`.
//!
//! ⚠️⚠️ The SIZING of that comparison is an oracle split, measured on generate-case
//! census cells W01–W20 (`case (4'd15 + 4'd1) 0: … 16: …`, `case (4'sb1111) -1: … 8'd0:
//! …`). The WHOLE reading is §12.5's — the case expression and every item evaluated at
//! the longest width, unsigned if any is unsigned — and it is what every tool's
//! PROCEDURAL `case` and vita's runtime do. Each oracle deviates from it in a generate
//! region on part of the axis: iverilog 13.0 (and sv2v → iverilog, which hands it the
//! same generate-case) evaluates each expression at its OWN width and compares one item
//! at a time with the pair's sign, contradicting its own procedural `case` (`4'd15 +
//! 4'd1` against `0` / `16` takes `16` procedurally and `0` in a generate-case; `-1`
//! equals `64'hFFFF_FFFF_FFFF_FFFF` procedurally, where `-1` negates at 64 bits, and not
//! in a generate-case); verilator 5.052 follows the whole width but sign-extends a signed
//! item inside a wider unsigned case (`case (4'sb1111) 4'sb1111: … 8'd0: …` takes
//! `default` — its procedural case does too there — and a `$unit` `int` `-1` item under
//! `64'hFFFF_FFFF_FFFF_FFFF` hits while its own `===` of the pair is 0). So vita
//! computes the PAIR reading (iverilog's) and the WHOLE reading (§12.5), and a label
//! decides only where they agree; where they disagree, and wherever the bit domain
//! cannot read the case, the label keeps exactly the answer it had before this lane
//! existed: its i64 value compared with the case expression's, else a non-match. Nothing
//! here refuses (a label the bit domain cannot fold is a residue to close by making it
//! foldable).

use super::*;
use crate::const_wide::{self, WideBits};

/// §12.5 case equality of two folded constants at width `w`: each is extended to `w` —
/// sign-extended only when `sg`, zero-extended otherwise — and every bit compared in all
/// four states, so x matches only x and z only z.
fn case_equal(a: &WideBits, b: &WideBits, w: u32, sg: bool) -> bool {
    let ea = const_wide::extend_bits(&a.0, a.1, w, sg);
    let eb = const_wide::extend_bits(&b.0, b.1, w, sg);
    (0..w as usize).all(|i| const_wide::bp_get(&ea, i) == const_wide::bp_get(&eb, i))
}

impl Elaborator<'_> {
    /// Choose the arm of a generate-case whose case expression folded to `scrut_i` in
    /// the i64 domain (the caller's gate, unchanged: a case expression the i64 fold
    /// cannot read is refused there).
    ///
    /// Every expression of the case is folded once in the bit domain at its own width
    /// (`fold_selfdet_operand`, so a lone fill is one bit there). When the case
    /// expression folds to known bits and every label of every item folds (x/z bits
    /// allowed), each label is compared under both sizings — its own width beside the
    /// case expression's with the pair's sign, and the whole case's width and sign with
    /// both sides refolded into it (`fold_in_region`, the comparison region's own
    /// operand rule) — and decides where the two agree. Where they disagree, and in a
    /// case the bit domain cannot read whole (a constant-function call, a string
    /// parameter, a real, or a label no domain folds somewhere in it), a label is
    /// compared exactly as before this lane: its i64 value against `scrut_i`, and a
    /// label with no i64 value is a non-match.
    ///
    /// `use_region` false forces that previous comparison for every label: the caller
    /// passes it when the region was not available on the construct's first
    /// elaboration, so that a region a later phase can build (a label that refers
    /// forward to a generate-scope localparam the Nets walk binds) cannot change the
    /// arm between phases.
    ///
    /// Returns the body to elaborate — the first matching item in source order, else
    /// `default`, else none — and whether the region was available.
    pub(crate) fn gen_case_choose<'a>(
        &self,
        scrutinee: &ast::Expr,
        scrut_i: i64,
        items: &'a [ast::GenCaseItem],
        use_region: bool,
    ) -> (Option<&'a [ast::GenItem]>, bool) {
        let name = |n: &ast::Expr, _: bool| self.param_leaf_bits(n);
        let wide = |e: &ast::Expr| const_wide::fold_selfdet_operand(e, &name);
        // The i64 reading the lane decided with before: a numeric fold, or a string
        // label's bytes (§5.9) — `case (P) "ab": …` over `P = 16'h6162`.
        let narrow = |e: &ast::Expr| {
            self.const_eval_in_scope(e).or_else(|| {
                self.const_str_in_scope(e)
                    .and_then(|t| const_wide::str_raw_i64(&t))
            })
        };
        let s0 = wide(scrutinee).filter(|v| !const_wide::bp_any_unknown(&v.0, v.1));
        let folded: Vec<Option<WideBits>> = items
            .iter()
            .flat_map(|ci| match ci {
                ast::GenCaseItem::Match { labels, .. } => labels.as_slice(),
                ast::GenCaseItem::Default { .. } => &[],
            })
            .map(wide)
            .collect();
        // The whole case's width and sign, and the case expression refolded into them.
        let region = s0.as_ref().and_then(|s0| {
            let mut w = s0.1;
            let mut sg = s0.2;
            for v in &folded {
                let v = v.as_ref()?;
                w = w.max(v.1);
                sg &= v.2;
            }
            let s = const_wide::fold_in_region(scrutinee, s0.clone(), w, sg, &name)?;
            Some((w, sg, s))
        });
        let region_ok = region.is_some();
        let region = region.filter(|_| use_region);
        let mut default: Option<&'a [ast::GenItem]> = None;
        let mut folded = folded.into_iter();
        for ci in items {
            let (labels, body) = match ci {
                ast::GenCaseItem::Match { labels, body, .. } => (labels, body),
                ast::GenCaseItem::Default { body, .. } => {
                    default = Some(body);
                    continue;
                }
            };
            for lab in labels {
                let l0 = folded.next().flatten();
                // The answer the i64 lane gave: its value compared, else a non-match.
                let pre = || narrow(lab) == Some(scrut_i);
                let hit = match (&s0, &region, l0) {
                    (Some(s0), Some((w, sg, s)), Some(l0)) => {
                        let pair = case_equal(s0, &l0, s0.1.max(l0.1), s0.2 && l0.2);
                        let whole = const_wide::fold_in_region(lab, l0, *w, *sg, &name)
                            .map(|l| case_equal(s, &l, *w, *sg));
                        if whole == Some(pair) {
                            pair
                        } else {
                            pre()
                        }
                    }
                    _ => pre(),
                };
                if hit {
                    return (Some(body), region_ok);
                }
            }
        }
        (default, region_ok)
    }
}
