//! The display chain's entries that take the Elaborator (tier E ; §5.2 Q-LEXSCOPE LS2-d): the
//! six display fns and `enter_frame_scope_root`, moved here with their bodies and private ;
//! the outside reaches them only through the typed shims below (RT-a: a shim wraps one call of
//! one private fn into `Shown` / `ShownCtx` / `SavedRoot`) and the two entries cut from
//! call-site fragments (`stmt_loc_instance`, `disp_root_swap_decl_inst`) ; since §5.2 Q-DECLKEY
//! DK3-a also `net_name_table` (net_util.rs's, moved whole, RET `Vec<Shown>` ; its callee
//! `display_of_key_text` is private and LEAVE L4 is closed) and `net_key_shown` (the key of a net
//! id as display text, for the three diagnostics that print it).

use super::RtnKey;
use super::{wrap_ctx, wrap_root, wrap_shown, Mark, SavedRoot, Shown, ShownCtx};
use crate::Elaborator;

impl Elaborator<'_> {
    /// The instance path as `%m` prints it (§4.5.429, ROADMAP §2 🆕 N): a SINGLETON
    /// generate scope (`generate if` / `case` / a bare labelled block) is stored as
    /// `label[0]` so the scope walk treats it like a loop iteration, but it is not an
    /// array — both oracles print `top.g.blk`, not `top.g[0].blk` (§27.5/§27.6). Keyed
    /// POSITIVELY on `gen_singleton_labels` (review B B1: a first draft stripped every
    /// `[0]` not in `gen_loop_labels`, which took an instance-array element `w[0]` to
    /// `w` — two elements of one array printed different spellings). A loop block and
    /// an instance-array element keep their index. Storage keys are untouched.
    fn display_prefix(&self) -> String {
        self.display_of(self.lex.cur_prefix_text())
    }

    /// `display_prefix` of an arbitrary storage path (§4.5.435: the declaring
    /// instance of a subroutine, `inst_prefix`).
    fn display_of(&self, prefix: &str) -> String {
        let mut acc = String::new();
        let mut out: Vec<String> = Vec::new();
        for seg in prefix.split('.') {
            let shown = match seg.strip_suffix("[0]") {
                Some(label) if !label.contains('[') => {
                    let key = if acc.is_empty() {
                        label.to_string()
                    } else {
                        format!("{acc}.{label}")
                    };
                    if self.gen_singleton_labels.contains_text(&key) {
                        label.to_string()
                    } else {
                        seg.to_string()
                    }
                }
                _ => seg.to_string(),
            };
            if !acc.is_empty() {
                acc.push('.');
            }
            acc.push_str(seg);
            out.push(shown);
        }
        out.join(".")
    }

    /// §4.5.435: the `%m` scope of the statement being lowered, ABSOLUTE — the
    /// declaring scope of the enclosing subroutine body (`block_scope_root`) or the
    /// process's own scope, then the named-block chain. Spelled with a leading `.`
    /// so the engine renders it verbatim instead of appending it to the EXECUTING
    /// process's scope (which is the caller's scope inside a task called from a
    /// generate block: `top.gi.t` for both oracles' `top.t`). A chain lowered with
    /// no scope at all (class-method bodies lower before any instance) stays
    /// relative, exactly as before.
    ///
    /// §5.2 row 72 (round 1, differential F3): the process's own scope WITHOUT the
    /// `$blk$<lo>` segments a scoped block-local's block is lowered under. Those are
    /// storage segments, not scopes the design names — the block's label is already in
    /// `block_scope` — so `%m` in `begin : blk` whose local shadows a module variable
    /// printed `t.$blk$40.blk` where all three oracles print `t.blk` (a run-time
    /// diagnostic's scope is this string too). Storage keys, waveform names and an
    /// elaboration diagnostic's `[in …]` frame keep the segment (ROADMAP §2).
    fn scope_chain_abs(&self) -> String {
        let chain = self.lex.disp_chain.join(".");
        let root = match &self.lex.disp_root {
            Some(r) => r.clone(),
            None => {
                let p = self.display_prefix();
                if p.contains("$blk$") {
                    p.split('.')
                        .filter(|seg| !seg.starts_with("$blk$"))
                        .collect::<Vec<_>>()
                        .join(".")
                } else {
                    p
                }
            }
        };
        if root.is_empty() {
            chain
        } else if chain.is_empty() {
            format!(".{root}")
        } else {
            format!(".{root}.{chain}")
        }
    }

    /// The INSTANCE path this diagnostic is being elaborated under, as a
    /// single `Frame`. Empty at the top level (and during any pre-pass that runs
    /// before instantiation), which renders as nothing at all.
    ///
    /// ⚠️ `file:line:col` is NOT a key on its own here. A module instantiated N
    /// times elaborates N times, so one unconnected port in a leaf becomes N
    /// identical warnings; an external report received four of them from three
    /// different instances and could only count them. `cur_prefix` is the
    /// scope-resolution prefix — the same string `fq()` builds symbol keys from
    /// — so it is the instance path by construction rather than a second
    /// spelling of one.
    fn cur_context(&self) -> Vec<diag::Frame> {
        if self.lex.cur_prefix_text().is_empty() {
            return Vec::new();
        }
        // §4.5.429: the spelling `%m` prints — a singleton generate scope without
        // its storage `[0]`.
        vec![diag::Frame {
            // `$pkg$<pkg>` is the RESERVED storage prefix for something the design
            // writes as `package pk;`, so a diagnostic inside it says `[in pk]`.
            // Stripped HERE and not in `display_of`, which also names VCD scopes and
            // OBS paths: the package net scope is `$pkg$p` in the waveform and must
            // stay that way (`pkg_var.rs` pins that a package var is excluded by that
            // very prefix). Every other `$` segment names a construct the reader can
            // point at in the source, so only this one is translated.
            label: {
                let p = self.display_prefix();
                match p.strip_prefix("$pkg$") {
                    Some(rest) => rest.to_string(),
                    None => p,
                }
            },
            location: None,
        }]
    }

    /// §4.5.435: the `%m` of a frame subroutine — its DECLARING instance (display
    /// spelling) and its name, spelled absolute (leading `.`) for the engine's
    /// render. IEEE §21.2.1: `%m` names the scope the subroutine was declared in,
    /// not the process that called it (`top.t` from a generate-block process, not
    /// `top.gi.t`; `top.f` for every level of a recursion, not `top.f.f.f`; `top.a`
    /// for a task called from another task). A class method keeps its bare name
    /// (relative): its declaring instance is not known to a global class table.
    fn frame_path(&self, name: &RtnKey) -> String {
        // §27.3: a GENERATE-scoped routine is declared in the generate BLOCK, so
        // that block is what `%m` names — `t.u.g.show`, not `t.u.g[0]$show` (the
        // storage key) and not `t.u.show` (the enclosing module). Measured against
        // both oracles: iverilog 13 prints `t.u.g.show`; verilator 5.052 prints
        // `t.u.g.g.show` (it repeats the block label). The oracles SPLIT, and
        // iverilog's spelling is the one pinned — it is the scope path IEEE
        // §21.2.1 describes, with `display_of` doing the singleton `[0]` strip a
        // generate-if block already gets everywhere else.
        if let Some(scope) = self.rtn_decl_scope.get(name) {
            let d = self.display_of(self.lex.scope_text(scope));
            let bare = Self::rtn_key_bare(&name.0);
            return if d.is_empty() {
                bare.to_string()
            } else {
                format!(".{d}.{bare}")
            };
        }
        let inst = self.display_of(self.lex.mark_text(Mark::Inst));
        if inst.is_empty() {
            name.0.to_string()
        } else {
            format!(".{inst}.{}", name.0)
        }
    }

    /// `%m` of a frame subroutine, given BOTH its table key and its own declared
    /// name. The two differ for a package-scoped key (`p::f`, whose `%m` is the bare
    /// `f`) and for a §27.3 generate-scoped key (`g[0]$f`, whose `%m` is the scope the
    /// key names). Only the key can answer the second, and only the bare name can
    /// answer the first, so both are passed.
    fn frame_path_of(&self, key: &RtnKey, bare: &str) -> String {
        if self.rtn_decl_scope.contains_key(key) {
            self.frame_path(key)
        } else {
            self.frame_path_bare(bare)
        }
    }

    /// `frame_path` of a DECLARED bare name (C-8's text form, T10): the one birth (b5) at the first meeting, then the keyed form.
    fn frame_path_bare(&self, name: &str) -> String {
        self.frame_path(&super::rtn::rtn_key_unwalked(name))
    }

    /// §4.5.435: a frame body's `%m` chain is rooted at the frame's own declaring
    /// path `<inst>.<name>` (`frame_path`) — the same string `func_names` carries.
    fn enter_frame_scope_root(&mut self, name: &RtnKey) -> Option<String> {
        let fp = self.frame_path(name);
        let root = fp.strip_prefix('.').unwrap_or(&fp).to_string();
        self.lex.disp_root.replace(root)
    }

    /// SHIM: the `%m` path of the current scope (`display_prefix`).
    pub(crate) fn disp_here(&self) -> Shown {
        wrap_shown(self.display_prefix())
    }

    /// SHIM: the `%m` path of the module instance being lowered (`inst_prefix`'s display
    /// spelling, no generate scope).
    pub(crate) fn disp_inst_top(&self) -> Shown {
        wrap_shown(self.disp_inst_top_text())
    }

    /// `display_of` of the instance mark (DELTA11 N-1: the shim's private fn ; it reads the
    /// mark [`Mark::Inst`] inside the module, LS3b).
    fn disp_inst_top_text(&self) -> String {
        self.display_of(self.lex.mark_text(Mark::Inst))
    }

    /// SHIM: the absolute `%m` scope of the statement being lowered (`scope_chain_abs`).
    pub(crate) fn disp_chain_abs(&self) -> Shown {
        wrap_shown(self.scope_chain_abs())
    }

    /// SHIM: the diagnostic context of "here" (`cur_context`).
    pub(crate) fn ctx_here(&self) -> ShownCtx {
        wrap_ctx(self.cur_context())
    }

    /// SHIM: the `%m` of a frame subroutine (`frame_path_of`).
    pub(crate) fn disp_frame_path_of(&self, key: &RtnKey, bare: &str) -> Shown {
        wrap_shown(self.frame_path_of(key, bare))
    }

    /// SHIM: a frame body's display root is entered ; the root that was in effect is saved
    /// (`enter_frame_scope_root`).
    pub(crate) fn disp_enter_frame_root(&mut self, name: &RtnKey) -> SavedRoot {
        wrap_root(self.enter_frame_scope_root(name))
    }

    /// The display spelling of a storage KEY (`net_name_table`'s net names) — key text in, display
    /// text out ; reads no lexscope state. Private since §5.2 Q-DECLKEY DK3-a, which moved its one
    /// caller in and closed LEAVE L4.
    fn display_of_key_text(&self, key: &str) -> String {
        self.display_of(key)
    }

    /// Per-NetId fully-qualified name table for the VCD writer, built by inverting
    /// the FQ-name → NetId `symbols` map (`"top.dut.q"`). A net with no symbol entry
    /// (anonymous/implicit) falls back to `n{id}`. BTreeMap iteration is sorted, so
    /// a net mapped by several aliases keeps the lexicographically smallest FQ name
    /// (its canonical declaration path). Order-independent of arena order → 3-OS
    /// stable. Computed before `finish()` (which moves `self.nets`/`self.symbols`).
    ///
    /// Spelled through `display_of`, so a SINGLETON generate scope (`generate if` /
    /// `case` / a bare labelled block, stored as `label[0]`) prints as `label` — the
    /// spelling both reference tools use and the one every `[in …]` context and `%m`
    /// already print. Before this, one runtime diagnostic carried two spellings of
    /// one scope on one line (`…g_sw[0].g_subword[0]…` for the net, `…g_sw.g_subword…`
    /// for the context; reviewer §3.2, 2026-09-09), and a `--probe` path had to be
    /// typed in the storage spelling. Loop iterations and instance-array elements
    /// keep their index; storage keys (`symbols`) are untouched.
    pub(crate) fn net_name_table(&self) -> Vec<Shown> {
        let n_nets = self.nets.len();
        let mut names = vec![String::new(); n_nets];
        for (fq, &id) in self.symbols.iter_text() {
            if let Some(slot) = names.get_mut(id as usize) {
                if slot.is_empty() {
                    *slot = self.display_of_key_text(fq);
                }
            }
        }
        for (i, n) in names.iter_mut().enumerate() {
            if n.is_empty() {
                *n = format!("n{i}");
            }
        }
        names.into_iter().map(Shown).collect()
    }

    /// An inlined task body's display root: the DECLARING instance (display spelling) ; the
    /// root that was in effect is saved (§4.5.435).
    pub(crate) fn disp_root_swap_decl_inst(&mut self) -> SavedRoot {
        let decl_root = self.display_of(self.lex.mark_text(Mark::Inst));
        SavedRoot(self.lex.disp_root.replace(decl_root))
    }

    /// The KEY of net `net_id` as display text (§5.2 Q-DECLKEY DK3-a (i)): the first key in `symbols`' order whose value is
    /// `net_id`, the storage spelling the three diagnostics that name a net by id have always printed (E3018 cont_var.rs, the
    /// lvalue kind lvalue.rs, the multi-driver netdecl.rs) ; `None` when no key holds it (each caller prints `#<id>`).
    pub(crate) fn net_key_shown(&self, net_id: u32) -> Option<Shown> {
        self.symbols
            .iter_text()
            .find(|(_, &id)| id == net_id)
            .map(|(n, _)| Shown(n.clone()))
    }

    /// The instance a statement's location record names (`record_stmt_loc`): a class
    /// method's `.<class>.<method>` marker, else `%m`'s own string for the statement without
    /// its leading `.` (§4.5.437, §2 🆕 N).
    pub(crate) fn stmt_loc_instance(&self) -> Shown {
        Shown(match &self.lex.disp_method {
            Some(m) => format!(".{m}"),
            None => {
                let s = self.scope_chain_abs();
                s.strip_prefix('.').unwrap_or(&s).to_string()
            }
        })
    }
}
