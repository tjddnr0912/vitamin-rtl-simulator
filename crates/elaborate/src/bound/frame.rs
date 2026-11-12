//! The constant-function frame (ROADMAP §5.2 BIND-2a, BIND-2b ; ER §5.1): one call's
//! locals, formals and return variable. `eval_const_call_at` makes it ; every interpreter
//! fn takes it as one `&ConstFrame` (`&mut` in `bind_const_decl`, `exec_const_select_write`,
//! `exec_const_stmt`) where it took the two maps. Both fields are private to this file:
//! the four writer methods are the only writers and the read lanes below the only raw
//! reads (a raw verb elsewhere does not compile) ; `tests/bound_home.rs` F0-F5 pin the
//! writers' calls, their order, the frames made and the lanes' calls per (file, fn).
//!
//! WHAT IT IS: two stores that share a key and not a lifetime. `env` (name -> value) and
//! `envw` (name -> declared width and signedness, width 0 = unknown) are written by
//! different statements, so a name can be in one and not the other:
//!   L1 declared + bound: a formal (W6 `bind_shape`, W7 `bind_value`), a declaration with
//!      no initializer (W1, W2 the 0) or a folded one (W1, W3), a written local (W12).
//!   L2 declared, unbound (`envw` only): W1 then W4 `unbind_value` (the initializer did
//!      not fold) or W5 (the width did not) ; a read is loud (cells c15 c29 c33 g1a) and a
//!      select write merges into 0 (W11).
//!   L3 bound, undeclared (`env` only): W8 `seed_value`, a package function's constants
//!      seeded per call ; W12 to a seeded name (c18 `A P=13`) ; its width comes from the
//!      package meta, not from here (`const_placement_wide` falls to `lookup_scoped`: g3b).
//!   L4 width 0 = unknown: `bind_shape(_, None)`, the one place that reading is spelled
//!      (W1 W6 ; j4r, c02, c03, g1b, c30-c34) ; readers propagate "no masking".
//!   L5 flat: one slot per name, a block declaration overwrites (g2b g4 g5).
//!   L6 empty frame = a module-scope ask (26 `ConstFrame::new()`, F3) ; a FIRST formal's
//!      default runs on an empty frame inside a call (g6a folds, g6b is E3009).
//!   L7 `[N:0]` numbering: `const_range_numbered` (c11 c12 c13).
//!   L8 a local's range folds at module / declaring-package scope, never on the frame
//!      (c20a E3009, c20b `A P=511`) ; L9 the shape is recorded before the initializer
//!      runs, which sees the name's OLD value (g5, c10).
//!   (cells: W/homes/lanes/bind2_lane ; pins: crates/cli/tests/frame_pins.rs)
//!
//! WRITERS AND THEIR ORDER (const_fn.rs ; F1 counts the calls, F2 the order):
//!   `bind_const_decl` (DECL-ORDER): `const_redeclared`, W1 `bind_shape`, then one arm:
//!     W2 `bind_value(0)` + `const_seed` if 4-state | W3 `bind_value` + `const_written` |
//!     W4 `unbind_value` + `const_seed` | W5 `unbind_value`.
//!   `eval_const_call_at` (CALL-ORDER): per formal W6 `bind_shape` then W7 `bind_value` ;
//!     the package seed W8 after the formals ; the body's declarations ; the return
//!     variable W9 `bind_shape`, the `has_value` test (`const_seed` if unbound and 4-state),
//!     W10 `seed_value(0)` (keeps a formal of the function's name: k5).
//!   `exec_const_select_write` W11 `bind_value` (+ `const_clear` if live, numbered, not
//!     shadowed) ; `exec_const_stmt` W12 `bind_value` + `const_written`.
//!   `const_unwritten` stays an `Elaborator` field (it is written on reads through `&`
//!   frames, `const_seed_read`) ; its one observer is cli/tests/const_seed_mark.rs.
//!
//! READ LANES (BIND-2b ; each one map verb, PRE's verb and key type at each site):
//!   `is_empty`     env.is_empty() && envw.is_empty() -- const_clog2_selfdet, eval_const_env
//!                  (the select arm's fallback and the default arm), fold_ctx's compare.
//!   `is_local`     env || envw -- fold_ctx's reduction, value/arms.rs selfdet_truth_arm.
//!   `is_local_w`   envw || env -- const_placement_wide's select root and count position.
//!                  Two lanes, not one: each keeps its sites' `||` order (same answer, the
//!                  order is cost alone ; it differs only on a one-sided name, L2 or L3).
//!   `is_declared`  envw.contains_key -- eval_const_env's leaf, the declared-unbound arm.
//!   `has_value`    env.contains_key -- eval_const_call_at's return-variable seed test.
//!   `value`        env.get -- the leaf, const_placement_wide's local, the return value of
//!                  a body that falls off its end, the select write's merge. It returns the
//!                  map's reference: the leaf reads it after `const_seed_read` ; a copy
//!                  taken first put 2 more instructions on the leaf's hit path
//!                  (eval_const_env 425 -> 427, W/bind2/build_b/asm/cmp2.txt).
//!   `shape`        envw.get().copied() -- const_placement_wide's local, the select write's
//!                  target width, the blocking assignment's target context.
//!   Generic over the borrowed key (`String: Borrow<Q>`): a site that asked with a
//!   `&String` still instantiates the map's `get::<String>` ; `&str` keys moved 18 symbols
//!   and added 2 (asm/cmp1.txt). LEFT: none of the 17 read sites. `widths()` stays: it
//!   hands `envw` to the 8 width-only fns (F4b), which also read the certified widths.
//!
//! OTHER PRODUCERS OF `ConstWidths` (not a frame): `declared_override_widths`
//! (param_query.rs), `declared_env_for` (params.rs), x_certified's merge (fold_x.rs). Other
//! stores of the value map's Rust type: `FoldScope::Env` (expr_size_hier.rs, a child
//! module's parameter env), `elaborate_package`'s `consts` (package.rs).
//!
//! MEASURED (W = ~/vita-work): BIND-2b's release text is byte-equal to BIND-2a's (masked
//! disassembly, every symbol: 6876 / 6876, 0 changed ; W/bind2/build_b/asm/cmp3.txt), so
//! each of the 17 sites re-points by its host fn's asm, including the 6 no cost cell
//! reaches (the 3 `is_empty` outside fold_ctx, the 2 `is_local_w`, arms.rs's `is_local`).
//! Per-site uses on the cost cells: W/bind2/p0/count/COUNT.tsv (leaf hits 624.8 M, fold_ctx
//! `is_empty` 130.2 M, `shape` at the blocking assignment 256.3 M, fold_ctx `is_local` 7.5 M).
//! BIND-2a's thread: 19 cells, 0 gate flags, z_pkg_call_loop +0.477 % (W/bind2/p0/
//! P0_SUMMARY.txt) ; `drop_in_place<ConstFrame>` out of line, 73 instructions ; cost30's
//! r_lsb_self_ok +1.92 % = recursion depth, the frame 16 B smaller per level
//! (W/bind2/build_a/DELTA1_SUMMARY.txt). UNMEASURED: what a merged per-local record
//! {value, width, sign} would cost or save -- P0-c's same-key pair counts (2.0 per
//! iteration on 16 cells, W/bind2/p0/count/P0C.txt) are a loose upper bound, not a saving.
//!
//! NOT BUILT: the merged per-local record (BIND-2 plan (r): the width half is a type the
//! certified producers share, and both halves are one-sided, L2 and L3) ; its first reader
//! is Q-CONSTSELLSB (a local's lsb and direction). REVIEW-ONLY (no token rule sees it): a
//! frame moved by value or swapped (`mem::swap` / `replace`), a writer or lane reached
//! through a fn pointer, a macro body.

use std::borrow::Borrow;
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

    /// L-empty: no local, formal or seed at all (a module-scope ask).
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.env.is_empty() && self.envw.is_empty()
    }

    /// L-local: bound or declared here, the value map asked first.
    #[inline]
    pub(crate) fn is_local<Q: Ord + ?Sized>(&self, n: &Q) -> bool
    where
        String: Borrow<Q>,
    {
        self.env.contains_key(n) || self.envw.contains_key(n)
    }

    /// L-local-w: bound or declared here, the width map asked first (the other `||`
    /// order ; same answer, kept as its sites spelled it).
    #[inline]
    pub(crate) fn is_local_w<Q: Ord + ?Sized>(&self, n: &Q) -> bool
    where
        String: Borrow<Q>,
    {
        self.envw.contains_key(n) || self.env.contains_key(n)
    }

    /// L-declared: declared here (a shape recorded), bound or not.
    #[inline]
    pub(crate) fn is_declared<Q: Ord + ?Sized>(&self, n: &Q) -> bool
    where
        String: Borrow<Q>,
    {
        self.envw.contains_key(n)
    }

    /// L-bound: a value is bound here.
    #[inline]
    pub(crate) fn has_value<Q: Ord + ?Sized>(&self, n: &Q) -> bool
    where
        String: Borrow<Q>,
    {
        self.env.contains_key(n)
    }

    /// L-value: the bound value, as the map's `get` hands it (a reference: a site that
    /// reads it after a side effect keeps that order).
    #[inline]
    pub(crate) fn value<Q: Ord + ?Sized>(&self, n: &Q) -> Option<&i64>
    where
        String: Borrow<Q>,
    {
        self.env.get(n)
    }

    /// L-shape: the recorded shape (width 0 = unknown).
    #[inline]
    pub(crate) fn shape<Q: Ord + ?Sized>(&self, n: &Q) -> Option<(u32, bool)>
    where
        String: Borrow<Q>,
    {
        self.envw.get(n).copied()
    }
}
