//! IEEE 1800-2017 §26.3: a package routine's BODY resolves its bare names in the
//! PACKAGE's scope, and only a name the package does not declare falls back to
//! the caller.
//!
//! vita lowers every routine body with the CALLER module's flat tables live —
//! an imported package function is injected into the module's `func_table`
//! (`apply_import_routines`), a scoped `pk::g()` call into the same table under
//! its scoped key (`inject_pkg_callees`) — so a bare name inside the body used to
//! bind wherever the MODULE binds it: `package pk; logic [15:0] x = 16'h0123;
//! function [31:0] g; g = x; endfunction` called from a module that declares its
//! own `logic [7:0] x = 8'hEE` printed `Z=ee` where both oracles (iverilog 13,
//! verilator 5.052) print `Z=123`, and with no module `x` it was E3010
//! (ROADMAP §2 "Scoping / imports / block-locals", entry 1). The frame lane
//! already bound the package's CONSTANTS under the body's own `$func$` scope
//! (`push_pkg_consts_scoped`); the inline lane bound nothing (`gs = C + 1;` was
//! E3010 there), and no lane bound a package VARIABLE anywhere.
//!
//! The scope is a stack entry (`RtnPkgScope`) pushed around the body by every
//! lane that lowers one — the frame function and task bodies, the inline
//! function fold and the inline task expansion — and consulted by the ONE bare
//! name decision (`bare_ident_route`) and the ONE net resolver every reader
//! shares (`lookup_net_scoped`, hence `resolve_net`, the select chains and the
//! classifiers), so a whole-name read, a `[i]` / `[m:l]` select, an lvalue and
//! the width/sign walks all answer the same object. Precedence inside the body:
//! the routine's OWN declarations (formals, body locals, block-locals, its return
//! name — `rtn_declared_names`, the same set the scoped-call gate and the
//! constant binder skip) win; then the package's constants and variables; then
//! the caller's scope, verbatim as before (a free name is a pre-existing
//! lenience of the import lane and a refusal of the scoped lane; neither moves).

use super::*;

/// The scope a package routine's body is being lowered in.
pub(crate) struct RtnPkgScope {
    /// The declaring package.
    pub(crate) pkg: String,
    /// Every name the routine declares itself — bound innermost, so a same-named
    /// package item must not shadow it.
    pub(crate) declared: BTreeSet<String>,
}

/// Every name a routine declares: its formals, its top-level body locals, every
/// `begin`/`fork` block-local anywhere in the body, the LABELS of its body-local
/// `typedef enum`s (`push_body_enum_labels` binds those innermost — round-1
/// soundness: without them a same-named package constant or variable shadowed
/// the routine's own label, `G=7` / `G=f7` where the module twin and verilator
/// print 3), and (a function's) own name, which its body may assign as the
/// return value.
///
/// ONE construction. `pkg_func_self_contained` (the scoped-call gate's write set),
/// `push_pkg_consts_scoped` (the constant binder's skip set) and this module's
/// scope entry each used to build it by hand; a further copy is how one of them
/// would drift. `frames_classify.rs::body_reads_only_locals` still gathers its
/// own (formals + locals, no labels): it is a ROUTING conjunct that can only
/// decline a frame route, so an omission there over-reports and is fail-closed
/// (measured in review: a label-reading body and its literal control print the
/// same value); it must not become a semantics decision without joining this.
pub(crate) fn rtn_declared_names(
    ports: &[ast::TfPort],
    body_decls: &[ast::NetVarDecl],
    body_enums: &[ast::TypedefDecl],
    body: &ast::Stmt,
    self_name: Option<&str>,
) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for p in ports {
        names.insert(p.name.name.clone());
    }
    for td in body_enums {
        #[allow(irrefutable_let_patterns)]
        if let ast::TypedefKind::Enum { labels, .. } = &td.kind {
            for lab in labels {
                names.insert(lab.name.name.clone());
            }
        }
    }
    let mut decls = body_decls.to_vec();
    collect_block_local_decls(body, &mut decls);
    for d in &decls {
        for n in &d.names {
            names.insert(n.name.name.clone());
        }
    }
    if let Some(n) = self_name {
        names.insert(n.to_string());
    }
    names
}

impl Elaborator<'_> {
    /// Enter a package routine's body scope. Paired with [`Self::pop_rtn_pkg_scope`].
    pub(crate) fn push_rtn_pkg_scope(&mut self, pkg: String, declared: BTreeSet<String>) {
        self.cur_rtn_pkg.push(RtnPkgScope { pkg, declared });
    }

    pub(crate) fn pop_rtn_pkg_scope(&mut self) {
        self.cur_rtn_pkg.pop();
    }

    /// The package whose routine body is being lowered, when `name` is NOT one of
    /// that routine's own declarations. `None` outside a package routine body, and
    /// for a name the routine binds itself (the frame slot / inline substitution
    /// wins, exactly as before).
    fn pkg_body_scope_for(&self, name: &str) -> Option<&str> {
        let sc = self.cur_rtn_pkg.last()?;
        (!sc.declared.contains(name)).then_some(sc.pkg.as_str())
    }

    /// A bare `name` read or written inside a package routine's body that names one
    /// of that package's VARIABLES: the package-level net. Consulted first by
    /// `lookup_net_scoped` (every net reader) and `resolve_net` (the lowering's
    /// tail, ahead of its declaration-order and import-alias checks, which are
    /// about the MODULE's binding of the name).
    pub(crate) fn pkg_body_var(&self, name: &str) -> Option<u32> {
        let pkg = self.pkg_body_scope_for(name)?;
        self.pkg_vars.get(pkg)?.get(name).copied()
    }

    /// A bare `name` inside a package routine's body that names one of that
    /// package's CONSTANTS: the route the `pkg::name` spelling takes, in the same
    /// order (`lower_expr`'s `PkgScoped` arm — real, string, wide, numeric), so the
    /// two spellings of one constant build one IR. The frame lane also binds these
    /// under the body's `$func$` scope; the answer is the same there, and this is
    /// what the inline lane, which has no scope of its own, was missing.
    pub(crate) fn pkg_body_const_route(&self, name: &str) -> Option<BareIdentRoute> {
        let pkg = self.pkg_body_scope_for(name)?;
        if let Some(&v) = self.pkg_real_val.get(pkg).and_then(|m| m.get(name)) {
            return Some(BareIdentRoute::Real(v));
        }
        if let Some(raw) = self.pkg_str_raw.get(pkg).and_then(|m| m.get(name)) {
            return Some(BareIdentRoute::Str(raw.clone()));
        }
        if let Some(cv) = self.pkg_wide_bits.get(pkg).and_then(|m| m.get(name)) {
            return Some(BareIdentRoute::Wide(cv.clone()));
        }
        let v = *self.pkg_consts.get(pkg)?.get(name)?;
        let meta = self
            .pkg_const_meta
            .get(pkg)
            .and_then(|m| m.get(name))
            .copied();
        Some(BareIdentRoute::Param {
            v,
            meta,
            guessed: false,
        })
    }

    /// IEEE 1800 §13.5.4: a formal's DEFAULT value is evaluated in the scope where
    /// the subroutine is DECLARED. Every lane fills an omitted actual with the
    /// declaration's default expression and lowers it beside the user's actuals in
    /// the CALLER's scope (`fill_default_args` / `resolve_named_args`), which for a
    /// PACKAGE routine bound a default naming a package variable, constant or sibling
    /// routine to the caller module's same-named object (`gd()` with
    /// `input [15:0] a = x` read the module's `x`: `D=ef` for both oracles' `124`;
    /// `a = C` the module's `C`; `a = h()` the module's `h`). Run `f` with the
    /// package's scope pushed when `a` IS the declared default of `p` (the same AST
    /// node: `resolve_named_args` clones it, span included, so identity is the span)
    /// and the routine is a package routine; a user-written actual, a module routine
    /// and every other call are `f` verbatim. The declared set is empty: a default
    /// that names another formal is refused before this point.
    pub(crate) fn with_default_arg_scope<T>(
        &mut self,
        rtn_name: &str,
        p: &ast::TfPort,
        a: &ast::Expr,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let is_default = p
            .default
            .as_ref()
            .is_some_and(|d| d.span == a.span && (d.span.lo, d.span.hi) != (0, 0));
        let pkg = if is_default {
            self.rtn_key_pkg(rtn_name)
        } else {
            None
        };
        match pkg {
            Some(pkg) => {
                self.push_rtn_pkg_scope(pkg, BTreeSet::new());
                let out = f(self);
                self.pop_rtn_pkg_scope();
                out
            }
            None => f(self),
        }
    }

    /// The `frame_idx` KEY of frame function `fid` — `pk::name` for a scoped call, the
    /// bare name otherwise — so a call site that only holds the FuncId can ask
    /// `rtn_key_pkg` the same question the body lowering asks.
    pub(crate) fn frame_key_of(&self, fid: u32) -> Option<&str> {
        self.frame_idx
            .iter()
            .find(|(_, &v)| v == fid)
            .map(|(k, _)| k.as_str())
    }

    /// The names a package's routine bodies may read and write without leaving the
    /// package: `(constants, variables)`. The scoped-call gate's admission sets.
    pub(crate) fn pkg_scope_names(&self, pkg: &str) -> (BTreeSet<String>, BTreeSet<String>) {
        let consts = self
            .pkg_consts
            .get(pkg)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        let vars = self
            .pkg_vars
            .get(pkg)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        (consts, vars)
    }
}
