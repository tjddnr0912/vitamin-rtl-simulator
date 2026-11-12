//! The writers of the nine maps: the value funnel moved verbatim from `params.rs` and
//! `wide_param_range.rs`, the one-statement primitives, and DECL-ORDER's one body
//! [`Elaborator::bind_decl`]. See the head note of `bound/mod.rs` for the store table.

use crate::*;

impl Elaborator<'_> {
    /// Bind a constant VALUE at `key`, and CLEAR any declared range recorded for the
    /// binding it replaces. **Every binder of `self.params` goes through here** — the
    /// header, body, generate, enum-label, genvar, real-twin, package and import
    /// binders alike — so the side map cannot be left describing a declaration that is
    /// no longer bound. A binder that HAS a range calls [`Self::bind_param_range`]
    /// immediately after; the debug assert there is what keeps that order.
    ///
    /// ⚠️ The class this closes was created by this slice and measured by the review.
    /// `param_range` is keyed exactly like `params`, and before imports bound a range
    /// no OTHER writer could rebind a ranged key — the FQ key made every scope
    /// disjoint. A wildcard `import pk::*` breaks that: it binds a package
    /// declaration at the module's OWN key, and then a local enum label, a genvar, a
    /// real parameter's integer twin or a body parameter rebinds the same key one
    /// phase later. Two of those were live correct→silent-wrong:
    /// `import pk::*` over `parameter [39:8] W` plus a local
    /// `enum { W = 32'hDEADBEEF }` answered `W[15:8]` = 239 where all three tools had
    /// said 190, and the genvar spelling answered 8 where all three had said 0.
    /// Clearing at 3 of ~13 binders was the wrong shape: the fix is that there is only
    /// one binder.
    pub(crate) fn bind_param_value(&mut self, key: String, v: i64) -> Option<i64> {
        self.param_range.w().remove(&key);
        self.hier_param_range.w().remove(&key);
        self.param_type_guessed.w().remove(&key);
        self.params.w().insert(key, v)
    }

    /// Unbind `key` — the value and the range together, for the same reason.
    pub(crate) fn unbind_param(&mut self, key: &str) -> Option<i64> {
        self.param_range.w().remove(key);
        self.params.w().remove(key)
    }

    /// Bind (or CLEAR) the declared range of the param now bound at `key`.
    ///
    /// ⚠️ The clear is the point. `param_range` answers *"is this param's width a
    /// declared fact?"*, and that question is about the declaration currently bound
    /// at the key — so every writer of `self.params` that could REBIND a key another
    /// writer already ranged has to say which it is. Imports made that reachable: a
    /// wildcard `import pk::*` binds `pk`'s `parameter [31:0] W` at `top.W`, and a
    /// module-body `localparam W = ~8'hCB;` then rebinds the VALUE two phases later
    /// (3a.5 imports, 3b body params). Leaving the package's range behind would let
    /// `W[15:8]` extract bits of a 32-bit declaration out of an 8-bit value — the
    /// §4.5.363 "263-bit net" regression, arriving through the import door.
    pub(crate) fn bind_param_range(&mut self, key: &str, r: Option<DeclRange>) {
        // The range describes the binding that is THERE. Writing it before the value
        // would be silently undone by `bind_param_value`'s clear, so the order is part
        // of the contract and this is what enforces it across every call site.
        debug_assert!(
            self.params.contains_key(key),
            "bind_param_range({key}) before the value was bound"
        );
        match r {
            Some(r) => {
                self.param_range.w().insert(key.to_string(), r);
                self.hier_param_range.w().insert(key.to_string(), r);
            }
            None => {
                self.param_range.w().remove(key);
                self.hier_param_range.w().remove(key);
            }
        }
    }

    /// Install a >64-bit parameter value at `key` and set — or CLEAR — its
    /// hierarchical select range. Descending ranges only: the hierarchical select
    /// refuses an ascending declaration (`build_hier_param_select`), so recording one
    /// would turn `u.A[12]` on `logic [12:83] A` from its pre-existing positional
    /// reading into an error (ROADMAP §2).
    pub(crate) fn bind_wide_param(&mut self, key: String, cv: ir::ConstVal, r: Option<DeclRange>) {
        match r {
            Some(r) if !r.2 => {
                self.hier_param_range.w().insert(key.clone(), r);
            }
            _ => {
                self.hier_param_range.w().remove(&key);
            }
        }
        self.wide_param_bits.w().insert(key, cv);
    }

    // ---- primitives: one map verb each, returning what the map returned ----

    /// `param_meta` set.
    pub(crate) fn set_meta(&mut self, key: String, m: (u32, bool)) -> Option<(u32, bool)> {
        self.param_meta.w().insert(key, m)
    }

    /// `param_meta` remove.
    pub(crate) fn clear_meta(&mut self, key: &str) -> Option<(u32, bool)> {
        self.param_meta.w().remove(key)
    }

    /// `param_range` set WITHOUT the hierarchical twin (the package lane, PKG-ORDER).
    pub(crate) fn set_range_raw(&mut self, key: String, r: DeclRange) -> Option<DeclRange> {
        self.param_range.w().insert(key, r)
    }

    /// `param_range` remove WITHOUT the hierarchical twin (the package lane, PKG-ORDER).
    pub(crate) fn clear_range_raw(&mut self, key: &str) -> Option<DeclRange> {
        self.param_range.w().remove(key)
    }

    /// `hier_params` set: the persistent copy a hierarchical read (`dut.P`) folds.
    pub(crate) fn publish_hier(&mut self, key: String, v: i64) -> Option<i64> {
        self.hier_params.w().insert(key, v)
    }

    /// `real_param_val` set.
    pub(crate) fn set_real(&mut self, key: String, v: f64) -> Option<f64> {
        self.real_param_val.w().insert(key, v)
    }

    /// `real_param_val` remove.
    pub(crate) fn clear_real(&mut self, key: &str) -> Option<f64> {
        self.real_param_val.w().remove(key)
    }

    /// `str_param_raw` set.
    pub(crate) fn set_str(&mut self, key: String, raw: String) -> Option<String> {
        self.str_param_raw.w().insert(key, raw)
    }

    /// `wide_param_bits` set WITHOUT the hierarchical range ([`Self::bind_wide_param`]
    /// is the declaration's binder).
    pub(crate) fn set_wide_raw(&mut self, key: String, cv: ir::ConstVal) -> Option<ir::ConstVal> {
        self.wide_param_bits.w().insert(key, cv)
    }

    /// `wide_param_bits` remove.
    pub(crate) fn clear_wide_raw(&mut self, key: &str) -> Option<ir::ConstVal> {
        self.wide_param_bits.w().remove(key)
    }

    /// `param_type_guessed` insert: the recorded meta is a guess.
    pub(crate) fn mark_guessed(&mut self, key: String) -> bool {
        self.param_type_guessed.w().insert(key)
    }

    /// `params` remove WITHOUT the range (U6's replay unwind, lane RTN-RAW-REMOVE:
    /// [`Self::unbind_param`] would also drop the range).
    pub(crate) fn drop_value_raw(&mut self, key: &str) -> Option<i64> {
        self.params.w().remove(key)
    }

    // ---- DECL-ORDER ----

    /// The declaration tail the module body, the generate scope and the header binder
    /// spell alike (DECL-ORDER): the persistent hier copy, the declared meta if any, the
    /// range fold, the value through the funnel, its range, and the guessed mark.
    ///
    /// ⚠️ The two closures run where the tails ran them, and that is the contract:
    /// `range` runs with the hier copy and the meta written and the value NOT (a
    /// generate-scope declaration that names an outer one of its own name reads the
    /// meta just written: cells p18, j09, j10) ; `guessed` runs after the value write
    /// cleared the previous mark (cells p02, g12b). `meta = None` never removes an older
    /// entry (META-STALE). Returns what the value funnel returned.
    pub(crate) fn bind_decl(
        &mut self,
        key: &str,
        v: i64,
        meta: Option<(u32, bool)>,
        range: impl FnOnce(&Self) -> Option<DeclRange>,
        guessed: impl FnOnce(&Self) -> bool,
    ) -> Option<i64> {
        self.publish_hier(key.to_string(), v);
        if let Some(m) = meta {
            self.set_meta(key.to_string(), m);
        }
        let r = range(self);
        let prev = self.bind_param_value(key.to_string(), v);
        self.bind_param_range(key, r);
        if guessed(self) {
            self.mark_guessed(key.to_string());
        }
        prev
    }
}
