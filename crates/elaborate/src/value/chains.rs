//! value/chains -- a consumer's sequence of lanes, spelled once (ROADMAP §5.2 VAL-2, owner 2026-11-09). See value/mod.rs's head
//! note for the lane table and for the chains that stay at their consumer.

use crate::*;

impl Elaborator<'_> {
    /// The value of a parameter DECLARATION's initializer, as the four binders take it: the
    /// module body (`elaborate_instance`), a package (`elaborate_package`), a generate scope
    /// (`lower_gen_module_item`) and a module / interface header after its three override
    /// channels (`bind_one_param`). PRE's five steps, each asked only when every step before it
    /// declined (`or_else` closures), in PRE's order:
    ///
    /// 1. `untyped_fill_init` -- an untyped fill (`parameter P = '1`). Asked only when `fill`:
    ///    the header passes `default_binds`, so an override that did not fold never reaches it
    ///    (the escalation has already made that loud) ; the other three binders pass `true`.
    /// 2. `param_init_at_declared_width` -- the wide read-back of a name or an operand wider than
    ///    64 bits at the declared width.
    /// 3. `eval_param_init` -- the evaluator every binder takes (§4.5.420 review B BLOCKING-1:
    ///    the generate twin was left on `const_eval_in_scope` and answered `00ffffffff` one
    ///    `generate` deeper than the module scope's `ffffffffff`) ; a fill inside a sized
    ///    initializer folds at the declared width there.
    /// 4. `param_value_via_real` -- branch parity of all four binders: a declared-integral
    ///    parameter whose initializer mentions a real converts at the declaration (§6.24.1 ;
    ///    the same text was `04` in a module body and E3009 in a header or an interface, and
    ///    6 at module scope but loud one `generate` deeper).
    /// 5. `param_i64_at_declared` at `param_decl_width_declared` -- the WIDE bit domain read
    ///    back as an i64 because the declaration fits one. Last: it fires only where every
    ///    integer step declined, so a reduction (`^A`), a select (`A[7:4]`) or a wide comparison
    ///    becomes a value instead of an error, and nothing that folded before changes route.
    ///
    /// `meta` is the caller's (`param_decl_width_unoverridden` in the three default-only
    /// binders, the override-aware width in the header), computed where PRE computed it ;
    /// step 5's `dm` is computed inside its closure. The tail -- the wide bind
    /// (`wide_disagreeing_value`), each site's own E3009 text and recovery value -- stays at
    /// each site.
    pub(crate) fn param_decl_fold(
        &self,
        p: &ast::ParamDecl,
        meta: Option<(u32, bool)>,
        fill: bool,
    ) -> Option<i64> {
        fill.then(|| self.untyped_fill_init(p))
            .flatten()
            .map(|(v, _)| v)
            .or_else(|| self.param_init_at_declared_width(p))
            .or_else(|| self.eval_param_init(&p.value, meta))
            .or_else(|| self.param_value_via_real(p, meta))
            .or_else(|| {
                let dm = self.param_decl_width_declared(p);
                self.param_i64_at_declared(&p.value, dm)
            })
    }

    /// A declared range's two bounds, folded by the range router (`const_range_bound_fold`)
    /// then checked (`check_const_range_bound`), in PRE's order: both folds, then both checks,
    /// msb first. The four sites that declare a range loudly take it: `packed_extents` (each
    /// packed dim), `check_param_decl_range`, `array_dim_extents` (an unpacked `[m:l]`) and
    /// `range_to_dims_opt`. The pair returned is the router's, unchanged (PRE's WRAP lane, see
    /// value/mod.rs) ; each site keeps its own default and narrowing.
    pub(crate) fn range_bounds_checked(&mut self, r: &ast::Range) -> (Option<i64>, Option<i64>) {
        let m = self.const_range_bound_fold(&r.msb);
        let l = self.const_range_bound_fold(&r.lsb);
        self.check_const_range_bound(&r.msb, m);
        self.check_const_range_bound(&r.lsb, l);
        (m, l)
    }
}
