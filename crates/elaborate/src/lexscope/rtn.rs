//! The routine key (§5.2 Q-RTNSCOPE RS1, S11-A ROUND 0 on a copy): `RtnKey`, the text a routine is filed under in its module's
//! routine tables, and its named entries — births (b1 b3 b5), compares, cuts and leaves (DELTA20B K2.6 as DELTA21B M3 R0-2 /
//! DELTA21C C1a I-2 restate it) — with the two fns moved in whole (I-3: `resolve_rtn_key` with its ruled body, `rtn_key_bare`).

use std::collections::BTreeSet;

use super::{wrap_shown, Shown};
use crate::Elaborator;

/// A routine-table key. Ordered and hashed as its text ; the text is read or built only inside lexscope (the field).
pub(crate) type RtnKey = String;

/// b1: the key a module scope BINDS a bare name under (a declaration, an import, a modport / import probe).
pub(crate) fn rtn_key_bound(name: &str) -> RtnKey {
    name.to_string()
}

/// b3: the key of `bare` declared in package `pkg` (`pkg::bare`).
pub(crate) fn rtn_key_scoped(pkg: &str, bare: &str) -> RtnKey {
    format!("{pkg}::{bare}")
}

/// b5: a call SPELLING taken as a module key WITHOUT the scope walk.
pub(crate) fn rtn_key_unwalked(spelling: &str) -> RtnKey {
    spelling.to_string()
}

/// b5, owned: the spelling's String becomes the key (no copy).
pub(crate) fn rtn_key_unwalked_owned(spelling: String) -> RtnKey {
    spelling
}

/// Compare: does key `k` spell `s` (text == text).
pub(crate) fn rtn_spells(k: &RtnKey, s: &str) -> bool {
    (*k) == s
}

/// Compare: is key `k`'s text one of the spellings in `set`.
pub(crate) fn rtn_spelled_in(k: &RtnKey, set: &BTreeSet<String>) -> bool {
    set.contains(k)
}

/// Values filed by routine key, read by a call SPELLING taken unwalked (b5's question, asked of a fn-local index).
pub(crate) struct RtnSpelledMap<V>(std::collections::HashMap<String, V>);

impl<V> RtnSpelledMap<V> {
    /// The empty index.
    pub(crate) fn new() -> RtnSpelledMap<V> {
        RtnSpelledMap(std::collections::HashMap::new())
    }

    /// `v` is filed under key `k`: the key's text copied once.
    pub(crate) fn put_key(&mut self, k: &RtnKey, v: V) {
        self.0.insert(k.clone(), v);
    }

    /// b5 asked of the index: the value filed under the key `spelling` names, taken unwalked.
    pub(crate) fn get_unwalked(&self, spelling: &str) -> Option<&V> {
        self.0.get(spelling)
    }
}

/// Cut: is `k` a package-scoped key (its text holds `::`).
pub(crate) fn is_pkg_scoped(k: &RtnKey) -> bool {
    (*k).contains("::")
}

/// Cut: is `k` a generate-scoped key (its text holds `$`).
pub(crate) fn is_gen_scoped(k: &RtnKey) -> bool {
    (*k).contains('$')
}

/// Cut: the package of a scoped key (`rtn_key_pkg`'s `split_once("::")`).
pub(crate) fn rtn_scoped_pkg(k: &RtnKey) -> Option<&str> {
    (*k).split_once("::").map(|(pkg, _)| pkg)
}

/// Cut: the bare declared name of a key — `rsplit("::")` then the tail after the last `$`, in today's order
/// (pkg_body_scope.rs:257-258).
pub(crate) fn rtn_bare_name(k: &RtnKey) -> &str {
    let bare = (*k).rsplit("::").next().unwrap_or(k);
    Elaborator::rtn_key_bare(bare)
}

/// D4: the IR net name of a static task's argument local (`__taskarg_{key}_{port}_{local}`).
pub(crate) fn taskarg_net_name(k: &RtnKey, port: &str, local: u32) -> String {
    format!("__taskarg_{}_{}_{}", (*k), port, local)
}

/// D7: the OBS name of a routine (the key's bytes).
pub(crate) fn rtn_obs_name(k: &RtnKey) -> String {
    (*k).clone()
}

/// D8: a routine key as message text.
pub(crate) fn rtn_show(k: &RtnKey) -> Shown {
    wrap_shown((*k).clone())
}

impl Elaborator<'_> {
    /// Bind one import's FUNCTION/TASK symbols (local definitions win —
    /// skip-if-present, called after the module's own (3.5) collection).
    /// The `func_table`/`task_table` key a BARE callee name resolves to.
    ///
    /// Normally the name itself. Inside the body of a routine declared in package
    /// `p`, `p::name` wins when it exists — that is the whole point of the scoped
    /// injection in `apply_import_routines`: a package routine's body must see its
    /// OWN siblings, not whatever the importing module gave the same name. Without
    /// this, `import p::f2;` in a module that also declares its own `helper` made
    /// `f2`'s body call the module's `helper` at exit 0 (measured: 1002 where
    /// iverilog says 4 — an earlier note said 2002, which the design as written does
    /// not produce).
    ///
    /// IEEE 1800-2017 §27.3: a routine declared inside a GENERATE block lives in that
    /// block's scope, so a bare callee resolves innermost-first — the enclosing
    /// generate scopes outward, then the module. That is what makes a generate `f`
    /// shadow a module `f`, and what keeps the two branches of a generate-if (or two
    /// iterations of a generate-for) from ever seeing each other's `f`. The walk sits
    /// AFTER the package check (a package routine's body is not inside any generate
    /// scope of the instantiating module) and BEFORE the bare fallback.
    pub(crate) fn resolve_rtn_key(&self, bare: &str) -> RtnKey {
        if let Some(pkg) = self.cur_rtn_pkg.last().map(|s| s.pkg.as_str()) {
            let scoped = rtn_key_scoped(pkg, bare);
            if self.func_table.contains_key(&scoped) || self.task_table.contains_key(&scoped) {
                return scoped;
            }
        }
        // Nothing to walk when this module declares no generate-scoped routine at
        // all, which is every design that predates §27.3 support.
        if !self.rtn_decl_scope.is_empty() {
            if let Some(k) = self.walk_rtn_scopes(bare, |k| {
                self.func_table.contains_key(k) || self.task_table.contains_key(k)
            }) {
                return k;
            }
        }
        rtn_key_bound(bare)
    }

    /// The bare routine name inside a table key: the tail after the last `$`.
    /// Byte-identical to the input for a module key (no `$`), so callers that hold
    /// either kind can use it unconditionally.
    pub(super) fn rtn_key_bare(key: &str) -> &str {
        match key.rfind('$') {
            Some(i) => &key[i + 1..],
            None => key,
        }
    }

    /// D1: run `f` inside the `$func${key}` scope segment of a frame body.
    pub(crate) fn with_func_scope<R>(&mut self, key: &RtnKey, f: impl FnOnce(&mut Self) -> R) -> R {
        self.with_scope(&format!("$func${}", (*key)), f)
    }

    /// D2: run `f` inside the `$itask${key}$L` scope segment of an inlined task body's locals.
    pub(crate) fn with_itask_scope<R>(
        &mut self,
        key: &RtnKey,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.with_scope(&format!("$itask${}$L", (*key)), f)
    }
}
