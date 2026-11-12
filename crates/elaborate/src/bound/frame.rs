//! The constant-function frame (ROADMAP §5.2 BIND-2a, ER §5.1): one call's locals and
//! formals, `env` (name -> value) and `envw` (name -> declared width and signedness),
//! as one type. `eval_const_call_at` makes it ; every interpreter fn takes it as one
//! `&ConstFrame` (`&mut` in `bind_const_decl`, `exec_const_select_write`,
//! `exec_const_stmt`) where it took the two maps. Both fields are private to this file,
//! so the four writer methods are the only writers (a raw write elsewhere does not
//! compile) ; each is one map verb, the PRE statement at the same place
//! (`tests/bound_home.rs` F0-F4 pin the writers' calls, their order and the frames made).
//!
//! The two maps do not move together: a name can be declared and unbound (`envw` only:
//! an initializer that did not fold, a select-written local) or bound and undeclared
//! (`env` only: a package constant seeded per call) ; a merged per-local record is not
//! built here (BIND-2 plan (r), first reader Q-CONSTSELLSB). `const_unwritten` stays an
//! `Elaborator` field: it is written on reads through `&` frames.

use std::collections::BTreeMap;

use crate::const_fn_width::ConstWidths;

pub(crate) struct ConstFrame {
    env: BTreeMap<String, i64>,
    envw: ConstWidths,
}

impl ConstFrame {
    #[inline]
    pub(crate) const fn new() -> Self {
        ConstFrame {
            env: BTreeMap::new(),
            envw: BTreeMap::new(),
        }
    }

    /// A declared local's, a formal's or the return variable's shape (W1 W6 W9) ; a width
    /// that did not fold is recorded as width 0 = unknown (the one place that reading is
    /// spelled for a frame), so a reader propagates "no masking" instead of guessing.
    #[inline]
    pub(crate) fn bind_shape(&mut self, name: String, shape: Option<(u32, bool)>) {
        self.envw.insert(name, shape.unwrap_or((0, false)));
    }

    /// A value bound to a name (W2 W3 W7 W11 W12): a declaration's 0 or initializer, a
    /// formal's argument, a select write's merge, a blocking assignment.
    #[inline]
    pub(crate) fn bind_value(&mut self, name: String, v: i64) {
        self.env.insert(name, v);
    }

    /// A declared local left unbound (W4 W5: its initializer did not fold, or its width
    /// did not), so a read of it is loud.
    #[inline]
    pub(crate) fn unbind_value(&mut self, name: &str) {
        self.env.remove(name);
    }

    /// A binding that keeps an existing one (W8 a package constant, W10 the return
    /// variable's 0) ; the key is built unconditionally, as PRE.
    #[inline]
    pub(crate) fn seed_value(&mut self, name: String, v: i64) {
        self.env.entry(name).or_insert(v);
    }

    /// The widths, for the 8 fns that read widths alone (they also serve the certified
    /// producers' maps, which never meet a frame).
    #[inline]
    pub(crate) fn widths(&self) -> &ConstWidths {
        &self.envw
    }

    /// The value map, for the reads still spelled at their sites (BIND-2b names the read
    /// lanes ; this stays while any site keeps PRE's verb).
    #[inline]
    pub(crate) fn env(&self) -> &BTreeMap<String, i64> {
        &self.env
    }
}
