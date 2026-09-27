//! A parameter WIDER than the i64 constant domain, installed with the declared range a
//! HIERARCHICAL select of it reads.
//!
//! `wide_param_bits` holds such a parameter's value as `[w-1:0]` bits and nothing about
//! its declaration, so `u.P[79:72]` on a child's `logic [79:8] P` read the stored bits
//! positionally (`xx` and `68` for `u.P[15:8]`, both oracles `61` / `69`). The
//! hierarchical select already normalizes a narrow parameter against
//! `hier_param_range`; [`Elaborator::bind_wide_param`] records the wide one there too,
//! with its value, so the two cannot describe different declarations.
//!
//! ⚠️ Deliberately NOT `param_range`, the table the name-walk select resolvers
//! (`param_sel_range`, the constant select) read. Letting those see a >64-bit binding
//! was built in §4.5.560 and reverted after three review rounds: every lane that folds
//! one scope's code at another scope's prefix — a module function at a generate call
//! site, a `$unit` or package routine, a typedef or formal/return range, a default —
//! then read the calling scope's same-named >64-bit parameter where the pre-slice walk,
//! blind to it, had declined or read the declaring scope's. The prerequisite is a
//! declaring-scope fold (REMAINING_WORK §D). A hierarchical path names its object
//! explicitly, so the value and the range come from one resolved key.

use super::*;

impl Elaborator<'_> {
    /// The range a >64-bit parameter's hierarchical select reads: the declared one
    /// ([`Self::param_decl_range_opt`]), or `[w-1:0]` for an untyped, unranged
    /// declaration, whose range is its value's (§6.20.2). `None` when the declaration
    /// states a width other than the value's — a range that does not describe the
    /// stored bits must not normalize a select of them.
    pub(crate) fn wide_param_decl_range(
        &self,
        p: &ast::ParamDecl,
        default_binds: bool,
        width: u32,
    ) -> Option<DeclRange> {
        let r = self.param_decl_range_opt(p, default_binds).or_else(|| {
            (matches!(p.ty, ast::ParamType::Implicit) && p.range.is_none())
                .then_some((0, width, false))
        })?;
        (r.1 == width).then_some(r)
    }

    /// Install a >64-bit parameter value at `key` and set — or CLEAR — its
    /// hierarchical select range. Descending ranges only: the hierarchical select
    /// refuses an ascending declaration (`build_hier_param_select`), so recording one
    /// would turn `u.A[12]` on `logic [12:83] A` from its pre-existing positional
    /// reading into an error (ROADMAP §2).
    pub(crate) fn bind_wide_param(&mut self, key: String, cv: ir::ConstVal, r: Option<DeclRange>) {
        match r {
            Some(r) if !r.2 => {
                self.hier_param_range.insert(key.clone(), r);
            }
            _ => {
                self.hier_param_range.remove(&key);
            }
        }
        self.wide_param_bits.insert(key, cv);
    }

    /// [`Self::bind_wide_param`] for a declaration: the range from
    /// [`Self::wide_param_decl_range`].
    pub(crate) fn bind_wide_param_decl(
        &mut self,
        p: &ast::ParamDecl,
        default_binds: bool,
        cv: ir::ConstVal,
    ) {
        let r = self.wide_param_decl_range(p, default_binds, cv.width);
        let key = self.fq(&p.name.name);
        self.bind_wide_param(key, cv, r);
    }
}
