//! The display chain's entries that take the Elaborator (tier E ; §5.2 Q-LEXSCOPE LS2-d): the
//! six display fns and `enter_frame_scope_root`, moved here with their bodies and private ;
//! the outside reaches them only through the typed shims below (RT-a: a shim wraps one call of
//! one private fn into `Shown` / `ShownCtx` / `SavedRoot`), the one LEAVE row
//! `display_of_key_text` (L4, text in and out, closing at DK4) and the two entries cut from
//! call-site fragments (`stmt_loc_instance`, `disp_root_swap_decl_inst`).

use super::{wrap_ctx, wrap_root, wrap_shown, SavedRoot, Shown, ShownCtx};
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
                    if self.gen_singleton_labels.contains(&key) {
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
    fn frame_path(&self, name: &str) -> String {
        // §27.3: a GENERATE-scoped routine is declared in the generate BLOCK, so
        // that block is what `%m` names — `t.u.g.show`, not `t.u.g[0]$show` (the
        // storage key) and not `t.u.show` (the enclosing module). Measured against
        // both oracles: iverilog 13 prints `t.u.g.show`; verilator 5.052 prints
        // `t.u.g.g.show` (it repeats the block label). The oracles SPLIT, and
        // iverilog's spelling is the one pinned — it is the scope path IEEE
        // §21.2.1 describes, with `display_of` doing the singleton `[0]` strip a
        // generate-if block already gets everywhere else.
        if let Some(scope) = self.rtn_decl_scope.get(name) {
            let d = self.display_of(scope);
            let bare = Self::rtn_key_bare(name);
            return if d.is_empty() {
                bare.to_string()
            } else {
                format!(".{d}.{bare}")
            };
        }
        let inst = self.display_of(&self.inst_prefix);
        if inst.is_empty() {
            name.to_string()
        } else {
            format!(".{inst}.{name}")
        }
    }

    /// `%m` of a frame subroutine, given BOTH its table key and its own declared
    /// name. The two differ for a package-scoped key (`p::f`, whose `%m` is the bare
    /// `f`) and for a §27.3 generate-scoped key (`g[0]$f`, whose `%m` is the scope the
    /// key names). Only the key can answer the second, and only the bare name can
    /// answer the first, so both are passed.
    fn frame_path_of(&self, key: &str, bare: &str) -> String {
        if self.rtn_decl_scope.contains_key(key) {
            self.frame_path(key)
        } else {
            self.frame_path(bare)
        }
    }

    /// §4.5.435: a frame body's `%m` chain is rooted at the frame's own declaring
    /// path `<inst>.<name>` (`frame_path`) — the same string `func_names` carries.
    fn enter_frame_scope_root(&mut self, name: &str) -> Option<String> {
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
    /// mark `inst_prefix` until LS3 takes the mark in).
    fn disp_inst_top_text(&self) -> String {
        self.display_of(&self.inst_prefix)
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
    pub(crate) fn disp_frame_path_of(&self, key: &str, bare: &str) -> Shown {
        wrap_shown(self.frame_path_of(key, bare))
    }

    /// SHIM: a frame body's display root is entered ; the root that was in effect is saved
    /// (`enter_frame_scope_root`).
    pub(crate) fn disp_enter_frame_root(&mut self, name: &str) -> SavedRoot {
        wrap_root(self.enter_frame_scope_root(name))
    }

    /// LEAVE L4 (closes at DK4): the display spelling of a storage KEY (`net_name_table`'s
    /// net names) — key text in, display text out ; reads no lexscope state.
    pub(crate) fn display_of_key_text(&self, key: &str) -> String {
        self.display_of(key)
    }

    /// An inlined task body's display root: the DECLARING instance (display spelling) ; the
    /// root that was in effect is saved (§4.5.435).
    pub(crate) fn disp_root_swap_decl_inst(&mut self) -> SavedRoot {
        let decl_root = self.display_of(&self.inst_prefix);
        SavedRoot(self.lex.disp_root.replace(decl_root))
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
