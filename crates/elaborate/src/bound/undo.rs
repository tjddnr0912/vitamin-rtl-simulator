//! The undo lanes U1-U8 (table in the head note of `bound/mod.rs`): the restores moved
//! verbatim from `params.rs` and `decl_scope.rs`, the instance-array prepass's
//! whole-map snapshot (U3, SIDE8), and the `match prev` put-backs the lanes spelled.

use crate::decl_scope::DeclKept;
use crate::*;

/// U3 (SIDE8): every map but `params`, cloned whole before the instance-array prepass
/// binds a child's header at the parent's prefix, and put back after it.
pub(crate) type SideSnap = (
    BTreeMap<String, ir::ConstVal>,
    BTreeMap<String, DeclRange>,
    BTreeMap<String, DeclRange>,
    BTreeMap<String, String>,
    BTreeMap<String, f64>,
    BTreeMap<String, (u32, bool)>,
    BTreeMap<String, i64>,
    BTreeSet<String>,
);

impl Elaborator<'_> {
    /// Restore the param map to the snapshot taken before this instance bound its
    /// params (so sibling instances of the same module re-bind cleanly).
    pub(crate) fn restore_params(&mut self, saved: Vec<(String, Option<i64>)>) {
        for (k, prev) in saved.into_iter().rev() {
            // Through the funnel in both directions: the scope being unwound is dead,
            // and a range left behind for one of its keys would outlive the value it
            // describes.
            match prev {
                Some(v) => {
                    self.bind_param_value(k, v);
                }
                None => {
                    self.unbind_param(&k);
                }
            }
        }
    }

    /// §4.5.158: twin of `restore_params` for the `param_meta` side-map — unwinds a
    /// scoped body-local enum-label width/sign registration so it does not pollute
    /// later scopes (`param_meta` is otherwise persistent).
    pub(crate) fn restore_param_meta(&mut self, saved: Vec<(String, Option<(u32, bool)>)>) {
        for (k, prev) in saved.into_iter().rev() {
            match prev {
                Some(v) => {
                    self.param_meta.w().insert(k, v);
                }
                None => {
                    self.param_meta.w().remove(&k);
                }
            }
        }
    }

    /// Re-install the kept bindings after the unwind. Only `decl_probe_key` reads
    /// them: a module or generate prefix never starts with `$pkg$`, the outward walk
    /// stops at an instance boundary, and the one whole-map use of these tables (the
    /// instance-array prepass's clone and restore) puts them back verbatim.
    pub(crate) fn decl_keep_restore(&mut self, kept: Vec<DeclKept>) {
        for k in kept {
            if let Some(v) = k.value {
                self.params.w().insert(k.key.clone(), v);
            }
            if let Some(m) = k.meta {
                self.param_meta.w().insert(k.key.clone(), m);
            }
            if let Some(r) = k.real {
                self.real_param_val.w().insert(k.key.clone(), r);
            }
            if let Some(s) = k.string {
                self.str_param_raw.w().insert(k.key.clone(), s);
            }
            if let Some(w) = k.wide {
                self.wide_param_bits.w().insert(k.key.clone(), w);
            }
            if let Some(r) = k.range {
                self.param_range.w().insert(k.key, r);
            }
        }
    }

    /// U3: the eight maps the prepass puts back, in SIDE8's order.
    pub(crate) fn side_snapshot(&self) -> SideSnap {
        (
            self.wide_param_bits.clone(),
            self.param_range.clone(),
            self.hier_param_range.clone(),
            self.str_param_raw.clone(),
            self.real_param_val.clone(),
            self.param_meta.clone(),
            self.hier_params.clone(),
            self.param_type_guessed.clone(),
        )
    }

    /// U3: whole-map put-back of [`Self::side_snapshot`] (assigned in the same order).
    pub(crate) fn side_restore(&mut self, side: SideSnap) {
        (
            *self.wide_param_bits.w(),
            *self.param_range.w(),
            *self.hier_param_range.w(),
            *self.str_param_raw.w(),
            *self.real_param_val.w(),
            *self.param_meta.w(),
            *self.hier_params.w(),
            *self.param_type_guessed.w(),
        ) = side;
    }

    /// `param_meta` put back: `Some` re-inserts, `None` removes (U5-U8).
    pub(crate) fn put_back_meta(&mut self, k: String, prev: Option<(u32, bool)>) {
        match prev {
            Some(m) => {
                self.param_meta.w().insert(k, m);
            }
            None => {
                self.param_meta.w().remove(&k);
            }
        }
    }

    /// `param_range` put back without the hierarchical twin (U7).
    pub(crate) fn put_back_range_raw(&mut self, k: String, prev: Option<DeclRange>) {
        match prev {
            Some(r) => {
                self.param_range.w().insert(k, r);
            }
            None => {
                self.param_range.w().remove(&k);
            }
        }
    }

    /// `real_param_val` put back (U7).
    pub(crate) fn put_back_real(&mut self, k: String, prev: Option<f64>) {
        match prev {
            Some(v) => {
                self.real_param_val.w().insert(k, v);
            }
            None => {
                self.real_param_val.w().remove(&k);
            }
        }
    }

    /// `str_param_raw` put back (U7).
    pub(crate) fn put_back_str(&mut self, k: String, prev: Option<String>) {
        match prev {
            Some(v) => {
                self.str_param_raw.w().insert(k, v);
            }
            None => {
                self.str_param_raw.w().remove(&k);
            }
        }
    }

    /// `wide_param_bits` put back (U7).
    pub(crate) fn put_back_wide(&mut self, k: String, prev: Option<ir::ConstVal>) {
        match prev {
            Some(cv) => {
                self.wide_param_bits.w().insert(k, cv);
            }
            None => {
                self.wide_param_bits.w().remove(&k);
            }
        }
    }
}
