//! IEEE 1800-2017 §13.4 / §26.3: a package routine's own TEXT — its return and
//! formal ranges, its formals' defaults, its body-local declaration ranges and the
//! constant calls in its body — names what its DECLARING package names, never what
//! the calling module binds under the same spelling.
//!
//! vita folds that text where the call is: the constant interpreter folds a
//! callee's header before it switches to the callee's package (`eval_const_call`),
//! and the frame reserve, the inline lane and the call-typing sites fold it at the
//! caller's prefix with the caller's constant-function table live. A package
//! routine `function automatic logic [f(2):0] h(...)` whose package `f` returns 3
//! was therefore 8 bits wide, not 4 (and printed `v=232`), from a module declaring its
//! own `f` returning 7 — iverilog and verilator print `v=8` (§2 🆕 AD).
//!
//! The WINDOW here is a FIRST PROBE, never a replacement scope: while it is armed
//! and its half of a split runs, a bare constant name first tries the declaring
//! package's binding (`$pkg$<pkg>.<name>`, kept live after `elaborate_package`; a
//! constant the package imports is bound there too, to the same value) and a bare
//! callee a FUNCTION the package itself declares (`pkg_owns(.., RtnKind::Func)`; a
//! function it only imports misses, see `decl_probe_fn`); a miss takes the pre-slice branch verbatim,
//! so a `$unit` copy or the caller's lenient free name still answers where it did. A
//! name the routine declares itself (formal, local, block-local, body enum label, its
//! own name) is never probed.
//!
//! CORRECTIONS ONLY. Each unit of routine text (a range bound, a default, a constant
//! call in a body being lowered) is folded twice: first with the window off and
//! nested arming suppressed — the pre-slice fold, exactly — and only when that
//! answered, again with the window on; the window's value replaces PRE's
//! (`win.or(pre)`). A unit PRE could not fold stays PRE's decline and diagnostic:
//! opening it would hand a value to consumers that inherit known silent defects
//! (🆕 AE's never-assigned 0, 🆕 AC's fallback sinks, the constant domain's OPEN
//! lines; ROADMAP §3.b `pkg-text-open`).
//!
//! Who is armed: only a routine its package DECLARES, as that kind of routine
//! (`pkg_owns`, the positive set: a function's text needs a declared function, a
//! task's a declared task). A module, generate, `$unit`, interface or class routine, and a routine
//! imported into another package, arms `Cleared` — explicitly, so a window an
//! enclosing fold opened never leaks into their text. Bodies run cleared: the
//! interpreter body keeps its own package rule (`const_call_pkg`). The window code
//! itself never writes `const_call_pkg`; a callee the probe answers runs its body
//! under the probe's tag — the package that declares the callee — because
//! `eval_const_call` writes every resolved callee's tag there, as for `p::f`.

use super::*;
use std::cell::OnceCell;
use std::rc::Rc;

/// Which routine table a name is asked in. A package's functions and tasks are
/// separate tables (`pkg_funcs` / `pkg_tasks`) and separate ownership sets
/// (`pkg_own_funcs` / `pkg_own_tasks`); every ownership question names one.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum RtnKind {
    Func,
    Task,
}

/// One owned routine whose text is being folded.
pub(crate) struct DeclWin {
    /// The declaring package.
    pub(crate) pkg: String,
    /// The routine's declared name.
    pub(crate) rtn: String,
    /// Function or task: which table holds its declaration.
    kind: RtnKind,
    /// The names the routine declares itself (`rtn_declared_names`), computed on the
    /// first probe hit. `None` = not found: every name counts as declared, so the
    /// probe never answers (fail-closed).
    declared: OnceCell<Option<Rc<BTreeSet<String>>>>,
}

impl DeclWin {
    fn declares(&self, el: &Elaborator<'_>, name: &str) -> bool {
        self.declared
            .get_or_init(|| {
                el.decl_names_of(&self.pkg, &self.rtn, self.kind)
                    .map(Rc::new)
            })
            .as_ref()
            .is_none_or(|s| s.contains(name))
    }
}

/// Whose text is being folded. Three states, because "no routine text" and "a
/// routine no package is known to declare" differ for the body lane: only the
/// former may arm a body window.
pub(crate) enum DeclArm {
    /// No routine text: an ordinary fold.
    Off,
    /// A non-owned routine's text, an interpreter body, or the PRE half of a split.
    Cleared,
    /// An owned routine's text.
    On(Rc<DeclWin>),
}

/// Which half of a split is running (`decl_split`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DeclPhase {
    /// No split is running.
    Idle,
    /// The PRE half: no window, nested arming suppressed.
    Pre,
    /// The window half: the probes answer.
    Win,
}

/// One package binding kept live after `elaborate_package` for the window's probe:
/// the six tables a bare constant walk reads, at one `$pkg$<pkg>.<name>` key, as the
/// package's own fold left them.
pub(crate) struct DeclKept {
    key: String,
    value: Option<i64>,
    meta: Option<(u32, bool)>,
    real: Option<f64>,
    string: Option<String>,
    wide: Option<ir::ConstVal>,
    range: Option<DeclRange>,
}

/// What `decl_enter` displaced, for `decl_exit`. `prev: None` = the arming was
/// suppressed (PRE half), nothing to restore.
pub(crate) struct DeclSaved {
    prev: Option<DeclArm>,
    pushed: bool,
    reentered: bool,
}

impl Elaborator<'_> {
    /// Does package `pkg` itself DECLARE a `kind` routine named `rtn`? The one
    /// ownership predicate: the window, the callee probe, the body lane and
    /// `pkg_fn_own` all ask it.
    pub(crate) fn pkg_owns(&self, pkg: &str, rtn: &str, kind: RtnKind) -> bool {
        let own = match kind {
            RtnKind::Func => &self.pkg_own_funcs,
            RtnKind::Task => &self.pkg_own_tasks,
        };
        own.get(pkg).is_some_and(|s| s.contains(rtn))
    }

    /// The window for `kind` routine `rtn` filed under `pkg`, when `pkg` DECLARES it as
    /// that kind (`pkg_owns`). `None` for every other routine — module, generate,
    /// `$unit`, interface, class, and one imported into another package (filed under
    /// the importer, which does not declare it).
    pub(crate) fn decl_win(
        &self,
        pkg: Option<&str>,
        rtn: &str,
        kind: RtnKind,
    ) -> Option<Rc<DeclWin>> {
        let p = pkg?;
        if !self.pkg_owns(p, rtn, kind) {
            return None;
        }
        let key = (p.to_string(), rtn.to_string(), kind);
        if let Some(w) = self.decl_wins.borrow().get(&key) {
            return Some(w.clone());
        }
        let w = Rc::new(DeclWin {
            pkg: key.0.clone(),
            rtn: key.1.clone(),
            kind,
            declared: OnceCell::new(),
        });
        self.decl_wins.borrow_mut().insert(key, w.clone());
        Some(w)
    }

    /// The window of the function the constant interpreter is running, when its
    /// package declares it (`pkg_fn_own`) — the body-local declaration ranges' owner.
    pub(crate) fn decl_local_win(&self) -> Option<Rc<DeclWin>> {
        let pkg = self.pkg_fn_own()?;
        let f = self.const_call_fn.borrow().clone()?;
        self.decl_win(Some(&pkg), &f, RtnKind::Func)
    }

    fn decl_names_of(&self, pkg: &str, rtn: &str, kind: RtnKind) -> Option<BTreeSet<String>> {
        if kind == RtnKind::Func {
            let f = self.pkg_funcs.get(pkg)?.get(rtn)?;
            return Some(pkg_body_scope::rtn_declared_names(
                &f.ports,
                &f.body_decls,
                &f.body_enums,
                &f.body,
                Some(&f.name.name),
            ));
        }
        let t = self.pkg_tasks.get(pkg)?.get(rtn)?;
        Some(pkg_body_scope::rtn_declared_names(
            &t.ports,
            &t.body_decls,
            &t.body_enums,
            &t.body,
            None,
        ))
    }

    /// Arm `win` (`None` clears) around a unit of routine text; paired with
    /// [`Self::decl_exit`]. `hdr` marks the routine's HEADER (return range, formal
    /// ranges, defaults, and the reserve / inline twins of those): a header armed
    /// again while its own header is being folded in the window half is a cycle
    /// the window opened (`function [f(1):0] f` reached through the package's `f`),
    /// so the arming clears instead and flags the enclosing split to keep PRE.
    /// Suppressed in the PRE half, which must be the pre-slice fold exactly.
    pub(crate) fn decl_enter(&self, win: &Option<Rc<DeclWin>>, hdr: bool) -> DeclSaved {
        let phase = self.decl_phase.get();
        if phase == DeclPhase::Pre {
            return DeclSaved {
                prev: None,
                pushed: false,
                reentered: false,
            };
        }
        let mut reentered = false;
        let mut pushed = false;
        let new = match win {
            None => DeclArm::Cleared,
            Some(w) => {
                if hdr
                    && phase == DeclPhase::Win
                    && self
                        .decl_hdr
                        .borrow()
                        .iter()
                        .any(|h| h.pkg == w.pkg && h.rtn == w.rtn && h.kind == w.kind)
                {
                    self.decl_reentered.set(true);
                    reentered = true;
                    DeclArm::Cleared
                } else {
                    if hdr {
                        self.decl_hdr.borrow_mut().push(w.clone());
                        pushed = true;
                    }
                    DeclArm::On(w.clone())
                }
            }
        };
        let prev = std::mem::replace(&mut *self.decl_arm.borrow_mut(), new);
        DeclSaved {
            prev: Some(prev),
            pushed,
            reentered,
        }
    }

    pub(crate) fn decl_exit(&self, saved: DeclSaved) {
        if let Some(prev) = saved.prev {
            *self.decl_arm.borrow_mut() = prev;
            if saved.pushed {
                self.decl_hdr.borrow_mut().pop();
            }
        }
    }

    /// [`Self::decl_enter`] around `f` for an owned routine's header. `None` on a
    /// re-entry: the caller declines (the enclosing split then keeps PRE).
    pub(crate) fn with_decl_hdr<T>(
        &self,
        win: &Option<Rc<DeclWin>>,
        f: impl FnOnce(&Self) -> T,
    ) -> Option<T> {
        let saved = self.decl_enter(win, true);
        if saved.reentered {
            self.decl_exit(saved);
            return None;
        }
        let r = f(self);
        self.decl_exit(saved);
        Some(r)
    }

    /// Fold one unit of routine text twice — PRE, then (only if PRE answered) the
    /// window — and keep the window's answer where it has one (`win.or(pre)`).
    /// Unarmed, or inside a PRE half, it is `f()` once.
    pub(crate) fn decl_split<T>(&self, f: impl Fn() -> Option<T>) -> Option<T> {
        let phase = self.decl_phase.get();
        if phase == DeclPhase::Pre || !matches!(*self.decl_arm.borrow(), DeclArm::On(_)) {
            return f();
        }
        let win = std::mem::replace(&mut *self.decl_arm.borrow_mut(), DeclArm::Cleared);
        self.decl_phase.set(DeclPhase::Pre);
        let pre = f();
        *self.decl_arm.borrow_mut() = win;
        let Some(pre) = pre else {
            self.decl_phase.set(phase);
            return None;
        };
        self.decl_phase.set(DeclPhase::Win);
        let w = f();
        self.decl_phase.set(phase);
        let reentered = self.decl_reentered.get();
        if phase == DeclPhase::Idle {
            self.decl_reentered.set(false);
        }
        match w {
            Some(v) if !reentered => Some(v),
            _ => Some(pre),
        }
    }

    /// The declaring package's key for bare constant `name`, written into `key`,
    /// while the window half runs and the routine does not declare `name` itself.
    /// `walk_scopes_key_inner` asks it before its outward walk.
    pub(crate) fn decl_probe_key(
        &self,
        name: &str,
        key: &mut String,
        hit: &impl Fn(&str) -> bool,
    ) -> bool {
        use std::fmt::Write;
        if self.decl_phase.get() != DeclPhase::Win {
            return false;
        }
        let arm = self.decl_arm.borrow();
        let DeclArm::On(w) = &*arm else {
            return false;
        };
        key.clear();
        let _ = write!(key, "$pkg${}.{name}", w.pkg);
        hit(key) && !w.declares(self, name)
    }

    /// The window package's OWN function for bare callee `f`, tagged with that package,
    /// while the window half runs; `const_fn_def` asks it before its pre-slice branches.
    ///
    /// Only a FUNCTION the package DECLARES answers (`pkg_owns(.., RtnKind::Func)`; a
    /// task of that name does not make an imported function its own). `pkg_funcs[pkg]`
    /// also holds every function the package imports (`elaborate_package`'s import arm
    /// files them there), and the tag returned here is what `eval_const_call` writes
    /// into `const_call_pkg` for the callee's body — so an imported `r::g` answered
    /// from here ran its body under the IMPORTER's constants (§4.5.589 review round 1:
    /// `K` read the importer's 3 where both oracles read `r`'s 8). An imported callee
    /// misses and keeps the pre-slice resolution; binding it to its origin package
    /// needs that origin recorded at the import (ROADMAP §2 "Scoping").
    pub(crate) fn decl_probe_fn(&self, f: &str) -> Option<(&ast::FunctionDef, Option<String>)> {
        if self.decl_phase.get() != DeclPhase::Win {
            return None;
        }
        let arm = self.decl_arm.borrow();
        let DeclArm::On(w) = &*arm else {
            return None;
        };
        if !self.pkg_owns(&w.pkg, f, RtnKind::Func) {
            return None;
        }
        let def = self.pkg_funcs.get(&w.pkg)?.get(f)?;
        Some((def, Some(w.pkg.clone())))
    }

    /// The body lane: a top-level constant call (not inside the interpreter, no
    /// routine text armed, no split running) made while an owned package routine's
    /// BODY is being lowered folds as a unit of that routine's text.
    pub(crate) fn decl_body_win(&self) -> Option<Rc<DeclWin>> {
        if self.decl_phase.get() != DeclPhase::Idle
            || !matches!(*self.decl_arm.borrow(), DeclArm::Off)
            || self.const_call_fn.borrow().is_some()
        {
            return None;
        }
        let sc = self.cur_rtn_pkg.last().filter(|s| s.owned)?;
        Some(Rc::new(DeclWin {
            pkg: sc.pkg.clone(),
            rtn: sc.rtn.clone(),
            kind: sc.kind,
            declared: OnceCell::from(Some(sc.declared.clone())),
        }))
    }

    /// Snapshot, before `elaborate_package` unwinds them, the bindings its fold made
    /// under `prefix` (`$pkg$<pkg>`): every key the fold saved for restore.
    pub(crate) fn decl_keep_snapshot<'k>(
        &self,
        prefix: &str,
        keys: impl Iterator<Item = &'k String>,
    ) -> Vec<DeclKept> {
        let mine = format!("{prefix}.");
        let keys: BTreeSet<&String> = keys.collect();
        debug_assert!(
            keys.iter().all(|k| k.starts_with(&mine)),
            "a package fold saved a key outside its own prefix {prefix}"
        );
        keys.into_iter()
            .filter(|k| k.starts_with(&mine))
            .map(|k| DeclKept {
                key: k.clone(),
                value: self.params.get(k).copied(),
                meta: self.param_meta.get(k).copied(),
                real: self.real_param_val.get(k).copied(),
                string: self.str_param_raw.get(k).cloned(),
                wide: self.wide_param_bits.get(k).cloned(),
                range: self.param_range.get(k).copied(),
            })
            .collect()
    }

    /// Re-install the kept bindings after the unwind. Only `decl_probe_key` reads
    /// them: a module or generate prefix never starts with `$pkg$`, the outward walk
    /// stops at an instance boundary, and the one whole-map use of these tables (the
    /// instance-array prepass's clone and restore) puts them back verbatim.
    pub(crate) fn decl_keep_restore(&mut self, kept: Vec<DeclKept>) {
        for k in kept {
            if let Some(v) = k.value {
                self.params.insert(k.key.clone(), v);
            }
            if let Some(m) = k.meta {
                self.param_meta.insert(k.key.clone(), m);
            }
            if let Some(r) = k.real {
                self.real_param_val.insert(k.key.clone(), r);
            }
            if let Some(s) = k.string {
                self.str_param_raw.insert(k.key.clone(), s);
            }
            if let Some(w) = k.wide {
                self.wide_param_bits.insert(k.key.clone(), w);
            }
            if let Some(r) = k.range {
                self.param_range.insert(k.key, r);
            }
        }
    }

    /// A call's declared return `(width, signed)`, its return range folded as the
    /// callee's header text (`pkg` = the package `const_fn_def` filed it under).
    pub(crate) fn const_fn_ret_wsign_in(
        &self,
        f: &ast::FunctionDef,
        pkg: Option<&str>,
    ) -> Option<(u32, bool)> {
        let win = self.decl_win(pkg, &f.name.name, RtnKind::Func);
        let saved = self.decl_enter(&win, true);
        let r = self.const_fn_ret_wsign(f);
        self.decl_exit(saved);
        r
    }
}
