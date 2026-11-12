//! The write contract of the nine FQ parameter maps (ROADMAP §5.2 BIND-1a, ER §5.1).
//!
//! Each map is a field of type [`Bound<T>`]: every reader reads it through `Deref`
//! exactly as it read the bare map, and the one mutable accessor [`Bound::w`] is
//! visible inside this module alone, so a write outside `crate::bound` does not
//! compile. The compiler, not a grep, lists the writers: they are the moved funnel
//! bodies and the one-statement primitives of `write.rs`, the declaration tail
//! [`Elaborator::bind_decl`], and the undo lanes of `undo.rs`. Nothing here changes
//! what is written, when, or in which order: each primitive is the statement its call
//! site spelled and returns what `insert` / `remove` returned.
//!
//! # The store (map, key, what an absent entry means, lifetime)
//!
//! | field | value | absent means | lifetime |
//! |---|---|---|---|
//! | `params` | `i64` | not an integral constant here (a > 64-bit value is never here) | unwound at the scope's exit (U1) |
//! | `param_meta` | `(width, signed)` | value-inferred (32 / 64 / real) | kept (META-STALE: [`Elaborator::bind_decl`] never removes) ; U2, U5-U8 put it back |
//! | `param_range` | [`DeclRange`] | zero-LSB raw offset, or not a declared fact (two meanings) | cleared by the value funnel ; unwound with the value |
//! | `hier_param_range` | [`DeclRange`] | as `param_range` | HIER-IMMORTAL: written and cleared with `param_range`, never unwound |
//! | `hier_params` | `i64` | no hierarchical read | HIER-IMMORTAL ; a real twin publishes here at generate scope only |
//! | `str_param_raw` | raw string literal | not a string constant | kept ; U3, U4, U7 put it back |
//! | `real_param_val` | `f64` | not a real constant | kept ; U5 suspends it, U3, U4, U7 put it back |
//! | `wide_param_bits` | `ir::ConstVal` | not a > 64-bit constant | kept ; U8 writes and never puts back |
//! | `param_type_guessed` | (set) | the recorded meta is a fact | cleared by the value funnel |
//!
//! The package store (`pkg_consts`, `pkg_const_meta`, …) is not guarded here: its maps
//! have one `insert` each, in `elaborate_package` (pinned by `bound_home.rs` W3). Its
//! `$pkg$<pkg>.<name>` twins in the nine maps are FQ keys and go through this module.
//! `pkg_var_aliases` is another store with several writers and is out of W3.
//!
//! # W: the writers (write.rs)
//!
//! - The value funnel, moved verbatim: [`Elaborator::bind_param_value`] (clears both
//!   ranges and the guessed mark, then binds), [`Elaborator::unbind_param`] (clears the
//!   local range only), [`Elaborator::bind_param_range`] (set or clear both ranges),
//!   [`Elaborator::bind_wide_param`].
//! - One-statement primitives, each one map verb: `set_meta` / `clear_meta`,
//!   `set_range_raw` / `clear_range_raw` (no hierarchical twin: the package lane),
//!   `publish_hier`, `set_real` / `clear_real`, `set_str`, `set_wide_raw`
//!   / `clear_wide_raw`, `mark_guessed`, `drop_value_raw` (U6's raw value remove, which
//!   leaves the range: lane RTN-RAW-REMOVE).
//! - Two write orders, kept as two named sequences:
//!   - DECL-ORDER (module body, generate scope, header): hier copy, meta if `Some`,
//!     RANGE FOLD, value, range, guessed test -> [`Elaborator::bind_decl`]. The range
//!     fold runs with the hier copy and the meta written and the value not (the
//!     declaration window, reachable at the generate scope), and the guessed test runs
//!     after the value write cleared the old mark; both are closures run at those
//!     positions.
//!   - PKG-ORDER (`elaborate_package`): value, RANGE FOLD, range raw set-or-clear, meta
//!     set-or-clear, each pushed on its own saved list. No hier copy, no guessed mark.
//!     It stays at its site on primitives (one caller, three out-lists).
//!
//! # U: the undo lanes (undo.rs ; map set per path, none merged)
//!
//! | lane | path | maps |
//! |---|---|---|
//! | U1 | `restore_params` | value via the funnel (re-bind clears range, hier range, guessed ; unbind clears the range only) |
//! | U2 | `restore_param_meta` | meta |
//! | U3 | SIDE8, `side_snapshot` / `side_restore` (instance array prepass) | every map but `params`, whole-map clone (quadratic: filing F-c) |
//! | U4 | `decl_keep_restore` (`$pkg$` keys) | value (raw), meta, real, str, wide, range raw: insert-if-kept, never removes |
//! | U5 | genvar loop (generate.rs) | value + range via the funnel, meta put back, real SUSPENDED and put back under `if let` (no remove), wide not suspended |
//! | U6 | routine genvar replay (frames_reserve.rs) | value (`None` -> `drop_value_raw`: the range is left), meta put back ; no range bound |
//! | U7 | package unwind (package.rs) | value via the funnel, then real, str, meta, range raw, wide put back |
//! | U8 | routine-body package constants (package.rs push, frames_body.rs unwind) | value via the funnel, meta put back ; wide written and never put back (lane "wide not restored", reach unmeasured) |
//!
//! `put_back_{meta,range_raw,real,str,wide}` is the `match prev { Some => insert, None
//! => remove }` each lane spelled. One snapshot type is NOT built: the lanes' map sets
//! differ, and one type would restore a map a lane leaves (cells p07, p13 read the
//! leftovers).
//!
//! # Presence lanes (unchanged)
//!
//! VALUE-ABSENT, META-ABSENT, META-STALE, RANGE-ABSENT (two meanings), HIER-IMMORTAL,
//! REAL-TWIN-NO-HIER (body, header) vs REAL-TWIN-HIER (generate), STR-KEEPS-VALUE,
//! GUESSED. The meta walk (`walk_scopes` over `param_meta` alone) can take a name's
//! value and its type from different keys (cell p16): `name_meta` names it.
//!
//! # R: the read lanes (read.rs, BIND-1b)
//!
//! `name_meta` (META-WALK) and `name_real` (REAL-WALK) are the two one-map walks ;
//! seven `hit_*` rules are the scope walk's hit tests, one per map set, each in the
//! `||` order its sites spelled (`read.rs`'s head note tables the rules, their map
//! sets and their sites, and the sites left with their reason). They read ; they write
//! nothing and change no presence lane. `bound_home.rs` W5 pins them.
//!
//! # What the pins see, and what they do not (REVIEW-ONLY)
//!
//! The compiler refuses a raw `insert` / `remove` / `&mut` through a `Bound` field
//! outside this module. It does NOT refuse a whole-field replacement: `self.x = ..`,
//! `std::mem::swap` / `replace` of two `Bound` fields (also spelled through `use
//! std::mem::swap`), or a destructuring of the struct that moves a field out
//! (`mem::take` is refused: no `Default`). `bound_home.rs` W2 counts the field-place
//! forms it can see (`&mut <recv>.<field>`, `<recv>.<field> =`, a tuple left-hand side
//! naming a field, a `Self {` / `Elaborator {` literal) ; a write through a reference
//! bound earlier, a macro body, and a receiver spelled another way are review-only.
//!
//! Not built here (said plainly): one storage record per name (first reader S1v-V),
//! the (`params`, `param_range`) pair as one value, one snapshot type, an accessor per
//! reader (the direct reads in their own combined orders stay at their sites).

mod frame;
mod read;
mod undo;
mod write;

pub(crate) use frame::ConstFrame;

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;

use crate::{ir, DeclRange};

/// A map written only inside `crate::bound`. Reads go through `Deref` ; there is no
/// `Clone`, `Default` or `DerefMut`, so a write elsewhere does not compile.
#[repr(transparent)]
pub(crate) struct Bound<T>(T);

impl<T> Deref for Bound<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> Bound<T> {
    /// The constructor's empty map (`driver.rs`, nine times: `bound_home.rs` W1).
    pub(crate) fn new_empty() -> Self
    where
        T: Default,
    {
        Bound(T::default())
    }

    /// The only mutable accessor, visible in this module alone.
    #[inline]
    pub(in crate::bound) fn w(&mut self) -> &mut T {
        &mut self.0
    }
}

// `repr(transparent)`: a guarded field is the size of its map.
const _: () = {
    use std::mem::size_of;
    assert!(size_of::<Bound<BTreeMap<String, i64>>>() == size_of::<BTreeMap<String, i64>>());
    assert!(
        size_of::<Bound<BTreeMap<String, (u32, bool)>>>()
            == size_of::<BTreeMap<String, (u32, bool)>>()
    );
    assert!(
        size_of::<Bound<BTreeMap<String, DeclRange>>>() == size_of::<BTreeMap<String, DeclRange>>()
    );
    assert!(size_of::<Bound<BTreeMap<String, String>>>() == size_of::<BTreeMap<String, String>>());
    assert!(size_of::<Bound<BTreeMap<String, f64>>>() == size_of::<BTreeMap<String, f64>>());
    assert!(
        size_of::<Bound<BTreeMap<String, ir::ConstVal>>>()
            == size_of::<BTreeMap<String, ir::ConstVal>>()
    );
    assert!(size_of::<Bound<BTreeSet<String>>>() == size_of::<BTreeSet<String>>());
};
