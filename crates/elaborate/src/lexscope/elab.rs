//! The lexscope entries that take the Elaborator (tier E): the segment entry `with_scope`
//! and residue R's two routine-key entries (LEAVE L2 / L3, closing at Q-RTNSCOPE RS1), moved
//! here with their bodies (§5.2 Q-LEXSCOPE LS2a) ; and `report_at`, a refusal reported in the
//! carried scope it was lowered in, once per (span, scope) (LS3) ; and the block-local
//! initializer store's five fns (`var_init.rs`'s, moved whole at LS3b-2), which key the two
//! scope-keyed stores `pending_block_local_inits` / `pending_scoped_presize` by the scope's
//! text inside this module ; and (DK1-5b) `names_with_pkg_var_aliases` (X18, `block_local/mod.rs`'s),
//! moved with its body: it reads the current scope's text to sort names by it (`decl_keep_snapshot`
//! stays in `decl_scope.rs` and asks [`super::Scopes::keys_under_cur`]).

use super::{found_text, Mark, ReportedAt, ScopeRef};
use crate::Elaborator;
use diag::MsgCode;
use hdl_ast as ast;

impl Elaborator<'_> {
    /// Run `f` with `cur_prefix` temporarily extended by `seg` (a gen-block
    /// `label[idx]` segment). Restores the prefix on return. Genvar bindings in
    /// `self.params` are NOT touched here (the caller manages those).
    pub(crate) fn with_scope<R>(&mut self, seg: &str, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved = self.lex.scope_enter(seg);
        let r = f(self);
        self.lex.scope_restore(saved);
        r
    }

    /// The routine-table key this generate scope gives `bare`, or `None` when the
    /// current position is not inside a generate scope of this module instance.
    pub(crate) fn cur_gen_rtn_key(&self, bare: &str) -> Option<String> {
        let inst = self.lex.mark_text(Mark::Inst);
        let cur = self.lex.cur_prefix_text();
        let rel = if inst.is_empty() {
            cur
        } else {
            cur.strip_prefix(inst)?.strip_prefix('.')?
        };
        Self::gen_rtn_key(rel, bare)
    }

    /// The routine table key a GENERATE scope gives a bare routine name, or `None`
    /// when `scope` names no generate scope at all (a plain module scope).
    ///
    /// `scope` is a prefix RELATIVE to the module instance (`g[0]`,
    /// `g[0].$func$g[0]$f`, `gl[1].$blk$12_40`). Only the generate-block segments
    /// contribute: a transparent `$func$…`/`$itask$…`/`$blk$…` segment is skipped,
    /// so a call made from inside a generate-scoped function body — or from a named
    /// block nested in the generate block — composes the same key the declaration
    /// registered. Segments are joined with `$`, never `.` (see `rtn_decl_scope`).
    fn gen_rtn_key(scope: &str, bare: &str) -> Option<String> {
        let mut key = String::new();
        for seg in scope.split('.') {
            // A `$…$` segment carries a `[` of its own when the routine it names is
            // itself generate-scoped (`$func$g[0]$f`), so the `$` test comes first.
            if seg.starts_with('$') || !Self::is_gen_scope_segment(seg) {
                continue;
            }
            key.push_str(seg);
            key.push('$');
        }
        if key.is_empty() {
            return None;
        }
        key.push_str(bare);
        Some(key)
    }

    /// Outward walk over the GENERATE scopes enclosing the current position,
    /// innermost first, returning the first composed routine key `hit` accepts.
    ///
    /// This is the routine-table twin of `walk_scopes_key` (lexscope/walk.rs), and it is a
    /// separate walk for one reason: `func_table` is MODULE-local (taken and
    /// restored per instance), so its keys are module-relative while `cur_prefix`
    /// is absolute. The walk therefore runs over `cur_prefix` minus `inst_prefix`
    /// and never crosses an instance boundary — a child instance elaborates with
    /// its own tables, so there is nothing outside this module to reach.
    ///
    /// Innermost-wins is what makes a generate `f` shadow a module `f`, and running
    /// out of relative scope (returning `None`) is what hands the module-level key
    /// back to the caller.
    pub(crate) fn walk_rtn_scopes(&self, bare: &str, hit: impl Fn(&str) -> bool) -> Option<String> {
        let inst = self.lex.mark_text(Mark::Inst);
        let cur = self.lex.cur_prefix_text();
        let mut rel: &str = if inst.is_empty() {
            cur
        } else if cur == inst {
            return None;
        } else {
            // `None` = not under this instance's path at all (a table swapped in
            // for a parent expression — see the `wire_ports` swap). No generate
            // scope of THIS position can be claimed; fall back to the module key.
            cur.strip_prefix(inst).and_then(|r| r.strip_prefix('.'))?
        };
        loop {
            if rel.is_empty() {
                return None;
            }
            if let Some(k) = Self::gen_rtn_key(rel, bare) {
                if hit(&k) {
                    return Some(k);
                }
            }
            rel = match rel.rfind('.') {
                Some(i) => &rel[..i],
                None => "",
            };
        }
    }

    /// The use-before-declaration check's own-binding test (`check_decl_precedes_use`): is
    /// the outward walk's hit for `name` the key `name` forms under the scope `decl_pos`
    /// describes (mark [`Mark::DeclPos`])?
    pub(crate) fn walk_hits_decl_pos(&self, name: &str) -> bool {
        let own_key = self.decl_pos_key(name);
        self.walk_key_net(name).as_ref().map(found_text) == Some(own_key.as_str())
    }

    /// The same check's block-local exemption: is the key `name` forms under the scope
    /// `decl_pos` describes a hoisted block-local (`hoisted_block_local`)?
    pub(crate) fn hoisted_at_decl_pos(&self, name: &str) -> bool {
        self.hoisted_block_local
            .contains_key(&self.decl_pos_key(name))
    }

    /// The key `name` forms under the scope `decl_pos` describes (mark [`Mark::DeclPos`]).
    fn decl_pos_key(&self, name: &str) -> String {
        let decl_pos_scope = self.lex.mark_text(Mark::DeclPos);
        if decl_pos_scope.is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", decl_pos_scope, name)
        }
    }

    pub(crate) fn report_at(
        &mut self,
        span: Option<ast::Span>,
        prefix: &ScopeRef,
        msg: &str,
        reported: &mut ReportedAt,
    ) {
        let (lo, hi) = span.map_or((0, 0), |s| (s.lo, s.hi));
        // One bound lowered twice under one instance (a statement lowered for two routes)
        // is one defect.
        if !reported
            .0
            .insert((lo, hi, self.lex.scope_text(prefix).to_string()))
        {
            return;
        }
        let saved_prefix = self.lex.scope_swap(prefix);
        match span {
            Some(span) => self.error_at(MsgCode::ElabUnsupported, span, msg),
            None => self.error(MsgCode::ElabUnsupported, msg),
        }
        self.lex.scope_restore(saved_prefix);
    }

    /// Move this scope's decl-time PRE-SIZE writes (`pending_scoped_presize[cur_prefix]`)
    /// to the front of `pending_var_inits`, so a routed string array's `new[n]` precedes
    /// the element writes the collectors are about to push. Called once per flush point —
    /// module/interface scope (the `""` key) and each generate scope.
    /// Only the entries this scope OWNS: an unlabeled generate body shares its parent's
    /// prefix, and draining its pre-size here would run `new[n]` in a later process than
    /// the element writes its own flush already emitted — wiping them silently.
    fn drain_scoped_presize(&mut self) {
        let Some(v) = self
            .pending_scoped_presize
            .0
            .remove(self.lex.cur_prefix_text())
        else {
            return;
        };
        let (mine, theirs): (Vec<_>, Vec<_>) = v
            .into_iter()
            .partition(|(owner, ..)| *owner == self.rank_path);
        if !theirs.is_empty() {
            self.pending_scoped_presize
                .0
                .insert(self.lex.cur_prefix_text().to_string(), theirs);
        }
        self.pending_var_inits
            .splice(0..0, mine.into_iter().map(|(_, l, r)| (l, r)));
    }

    /// Record one block-local declaration initializer for the deferred, declaration-ordered
    /// replay in [`Self::flush_block_local_inits`]. `scope` is the `$blk$<lo>` segment when
    /// the declaration earned its own scope; `lo` is the declaring name's source offset.
    pub(crate) fn push_block_local_init(
        &mut self,
        scope: Option<&str>,
        lo: u32,
        lhs: ast::Lvalue,
        rhs: ast::Expr,
    ) {
        let key = self.scoped_init_key(scope);
        let owner = self.rank_path.clone();
        self.pending_block_local_inits
            .0
            .entry(key)
            .or_default()
            .push((lo, owner, lhs, rhs));
    }

    /// Every recorded block-local initializer must have been claimed by exactly one flush
    /// point. One left behind is an initializer the design wrote and the IR does not carry
    /// — a silent-wrong by construction, so say so instead of returning a clean IR. Only
    /// checked on the success path: an already-failed elaboration may bail before a flush.
    pub(crate) fn assert_block_local_inits_drained(&mut self) {
        // The pre-size map is checked with it: a routed string array whose `new[n]` is
        // never emitted stays length 0, and every element write is discarded — the exact
        // silent-wrong shape this guard exists for.
        let orphans = self.pending_block_local_inits.0.len() + self.pending_scoped_presize.0.len();
        if self.had_error || orphans == 0 {
            return;
        }
        let keys: Vec<String> = self
            .pending_block_local_inits
            .0
            .keys()
            .chain(self.pending_scoped_presize.0.keys())
            .cloned()
            .collect();
        self.error(
            MsgCode::ElabUnsupported,
            &format!(
                "internal: block-local declaration initializers recorded under {} were \
                 never emitted — no flush point claimed the scope(s) `{}`; the design's \
                 initializers would be missing from the simulation",
                keys.len(),
                keys.join("`, `")
            ),
        );
    }

    /// Drain `pending_var_inits` into ONE synthesized `initial` process whose body
    /// assigns each non-constant variable initializer in declaration order (§6.8).
    /// Lowered in the current instance scope; a no-op when none were collected (so
    /// a design with no such initializer adds no process — byte-identical IR).
    /// The prefix a scoped block-local's t0 init is recorded under: the current
    /// instance prefix, plus the `$blk$<lo>` segment when the declaration has one.
    fn scoped_init_key(&self, scope: Option<&str>) -> String {
        match scope {
            Some(seg) if self.lex.cur_prefix_text().is_empty() => seg.to_string(),
            Some(seg) => format!("{}.{seg}", self.lex.cur_prefix_text()),
            None => self.lex.cur_prefix_text().to_string(),
        }
    }

    /// §4.5.254: flush this scope's collected module-scope initializers and then EVERY
    /// block-local one belonging to it, in DECLARATION order across blocks.
    ///
    /// Measured order (live iverilog): all module-scope statics first, then the
    /// block-locals by declaration offset — whether or not the declaring block earned a
    /// `$blk$` scope, and whether or not the declaration is a string. A run of consecutive
    /// same-prefix initializers becomes one synthesized `initial`; a t0 `initial` runs in
    /// ProcId order, so emitting the runs in order preserves the global order across them.
    /// A LEADING run in this very scope joins the main sweep's `initial` instead of adding
    /// a process — which is the whole design in every module with no scoped block-local, so
    /// their IR is unchanged.
    pub(crate) fn flush_block_local_inits(&mut self) {
        // At FLUSH time, not when the scope's walk opened. A generate body nested in
        // another shares its prefix, and the outer call's walk begins before the inner
        // one's flush — draining there handed the inner body's pre-size to the OUTER
        // process, so `new[n]` ran after the element writes the inner had already emitted
        // and silently wiped them. Flush order is innermost-first, which is ownership
        // order. Still spliced to the front of this scope's list, so the pre-size keeps
        // preceding the writes that ride it.
        self.drain_scoped_presize();
        let here = self.lex.cur_prefix_text().to_string();
        let dot = if here.is_empty() {
            String::new()
        } else {
            format!("{here}.")
        };
        // This scope owns its own key and every `$blk$` key BELOW it — at any depth, as
        // long as the whole remainder is `$blk$` segments. A generate or instance
        // segment in the remainder means that scope flushes at its own point, and is not
        // claimed here.
        //
        // R16 §3.4: the depth-1 form of this rule (`!rest.contains('.')`) was written
        // when procedural blocks could never nest their scopes, because the classifier
        // dropped any candidate block inside another candidate. With that restriction
        // lifted, a block-local inside a scoped block inside a scoped block records
        // under `t.$blk$<outer>.$blk$<inner>`, which no scope claimed — caught not by a
        // test but by the never-emitted guard, which named both orphaned keys.
        let is_here = |k: &String| {
            *k == here
                || k.strip_prefix(&dot)
                    .is_some_and(|rest| rest.split('.').all(|seg| seg.starts_with("$blk$")))
        };
        let keys: Vec<String> = self
            .pending_block_local_inits
            .0
            .keys()
            .filter(|k| is_here(k))
            .cloned()
            .collect();
        // …and only the entries this scope OWNS, by the same flag the pre-size drain uses.
        // A generate body that vita gives no prefix segment shares the MODULE's key, so a
        // generate walk's flush matched the module's own block-locals and emitted them
        // ahead of the module sweep — which both robbed a module block-local initializer of
        // the module variables it reads and split a routed string array from its `new[n]`
        // (left behind by the flag-partitioned pre-size drain), emptying it at exit 0.
        // Prefix answers WHICH scope; the owner's RANK PATH answers whose. §4.5.265: a
        // bool could not, because two NESTED generate scopes sharing a prefix are both
        // "in a generate" — so a prefix-less nested scope claimed its PARENT's
        // block-locals and emitted them under its own (later) rank.
        let ours = self.rank_path.clone();
        let mut all: Vec<(u32, String, ast::Lvalue, ast::Expr)> = Vec::new();
        for key in keys {
            let Some(v) = self.pending_block_local_inits.0.remove(&key) else {
                continue;
            };
            let (mine, theirs): (Vec<_>, Vec<_>) =
                v.into_iter().partition(|(_, owner, ..)| *owner == ours);
            if !theirs.is_empty() {
                self.pending_block_local_inits.0.insert(key.clone(), theirs);
            }
            all.extend(
                mine.into_iter()
                    .map(|(lo, _, l, r)| (lo, key.clone(), l, r)),
            );
        }
        // Stable: one declaration's several element writes share an offset and keep the
        // order the expansion built them in.
        all.sort_by_key(|(lo, ..)| *lo);
        // Split into consecutive same-prefix runs.
        let mut runs: Vec<(String, Vec<(ast::Lvalue, ast::Expr)>)> = Vec::new();
        for (_, key, lhs, rhs) in all {
            match runs.last_mut() {
                Some((k, v)) if *k == key => v.push((lhs, rhs)),
                _ => runs.push((key, vec![(lhs, rhs)])),
            }
        }
        // A leading run declared in THIS scope needs no process of its own.
        if runs.first().is_some_and(|(k, _)| *k == here) {
            let (_, v) = runs.remove(0);
            self.pending_var_inits.extend(v);
        }
        let own = if self.in_generate_body {
            Self::RANK_GEN_OWN
        } else {
            Self::RANK_MOD_OWN
        };
        self.flush_ranked(own);
        let bl = if self.in_generate_body {
            Self::RANK_GEN_BLOCK_LOCAL
        } else {
            Self::RANK_MOD_BLOCK_LOCAL
        };
        for (key, v) in runs {
            let saved = self.lex.scope_swap_text(key);
            self.pending_var_inits = v;
            // No pre-size drain here: a `$blk$`-scope pre-size is recorded in THIS list
            // (`route_fixed_string_array`), ahead of the element writes it must precede,
            // because it is pushed at the declaration and they are pushed by the collector
            // that runs after it. `pending_scoped_presize` never holds a `$blk$` key — and
            // if one ever appeared, `assert_block_local_inits_drained` reports it loudly
            // rather than letting the array stay length 0.
            self.flush_ranked(bl);
            self.lex.scope_restore(saved);
        }
    }
}

impl Elaborator<'_> {
    /// `names` ∪ the bare names bound into THIS scope by a package VARIABLE import
    /// (`import pkg::*` / `import pkg::name`). For the block-local SCOPING
    /// computations only — see the two traps below.
    ///
    /// Three-lifetime census (§2 Scoping row 3, measured against iverilog 13 and
    /// verilator 5.052, which agree on every cell):
    /// - DECLARATION: `package.rs:1288-1291` (wildcard) and `:1406-1408` (explicit)
    ///   insert the `$pkg$<pkg>` net id VERBATIM into `symbols`, so an imported bare
    ///   name is an ALIAS to the package's single shared net (`lib.rs:1180`), not a
    ///   per-module copy.
    /// - SHADOW: `gather_local_decl_names` above returns the module's OWN
    ///   declarations only, so `shadows_module` (`block_local_class.rs:398`,
    ///   `frames_reserve.rs:180`, `hoist.rs`'s twin) was false for such a name. On the
    ///   module-PROCESS feed that term is the SOLE one a plain, initializer-free,
    ///   non-`automatic` block-local can satisfy (`admit_static_plain` is false there,
    ///   `block_local_class.rs:284-286`), so the declaration took the bare flatten —
    ///   onto `symbols[<scope>.<name>]`, i.e. the package's own storage.
    /// - EXPORT: measured, both spellings name ONE net, so the block-local's write was
    ///   visible to `pkg::name` INSIDE the block, to a continuous assign, to a sibling
    ///   instance, and to a DIFFERENT module importing the same package.
    ///
    /// An imported name occupies this scope's bare-name namespace exactly as a
    /// declared net does — the precondition `shadows_module` exists to detect ("the net
    /// it would flatten onto is the shadowed one", `frames_reserve.rs:171-177`).
    /// Mechanism proof: adding a module-level declaration of the same name (which
    /// touches no package storage) already makes the whole shape correct, and the only
    /// thing it changes is this set.
    ///
    /// Two traps this signature is shaped around:
    /// 1. It returns a SEPARATE set; callers must not augment `names` in place. That
    ///    same set is `apply_import_consts`'s `local_names` (`package.rs:1258`), where
    ///    membership SUPPRESSES the import binding, so feeding it back its own product
    ///    would unbind every wildcard-imported variable. It is also read by
    ///    `compute_coalesced_block_locals` and `const_array.rs:125`.
    /// 2. It must NOT reach `compute_per_entry_block_locals`
    ///    (`block_local_class.rs:532`), which reads `module_names` with the OPPOSITE
    ///    polarity (`contains` -> `continue`): widening it there would remove per-entry
    ///    re-initialisation from an `automatic`-with-init block-local shadowing an
    ///    import.
    ///
    /// Keyed on `pkg_var_aliases` (`lib.rs:1187`), never on `symbols`: an
    /// interface-port alias also lives in `symbols` and is not a package variable
    /// (`package.rs:1284-1287` keeps the two apart for exactly this reason).
    pub(crate) fn names_with_pkg_var_aliases(
        &self,
        names: &std::collections::BTreeSet<String>,
    ) -> std::collections::BTreeSet<String> {
        if self.pkg_var_aliases.is_empty() {
            return names.clone();
        }
        let mut out = names.clone();
        for key in self.pkg_var_aliases.keys() {
            // Keys are `self.fq(name)`, so only the aliases bound at the CURRENT scope
            // are this scope's bare names (a nested instance's alias is not visible
            // here by bare name).
            let bare = match key.rsplit_once('.') {
                Some((scope, bare)) if scope == self.lex.cur_prefix_text() => bare,
                None if self.lex.cur_prefix_text().is_empty() => key.as_str(),
                _ => continue,
            };
            out.insert(bare.to_string());
        }
        out
    }
}
