//! The ONE refusal for a parameter name declared twice in one module or
//! interface scope (ROADMAP §2 🆕 L ⓢ).
//!
//! IEEE 1800-2017 §6.20.1 / §23.2.3: the parameter port list and the body
//! `parameter`/`localparam` declarations of a module share one declarative
//! scope, so a name may be declared there once. vita bound the second
//! declaration and answered it at exit 0 — measured on `module top
//! #(parameter int P = 3); parameter int P = 7;`, where vita printed `P=7`
//! while iverilog 13.0 says "'P' has already been declared in this scope" and
//! verilator 5.052 says "Duplicate declaration of signal: 'P'".
//!
//! The rule lives here, not at the binders, because the same duplicate reaches
//! FOUR shapes that would each have needed their own site: the ANSI header vs a
//! body declaration, two declarations inside one ANSI header, two body
//! declarations of a module with no ANSI header (IEEE 1364-2005 §12.2 makes the
//! FIRST of those the parameter port list — see [`Elaborator::param_ports`]),
//! and every one of those again in an `interface`. One pure walk over the
//! declaration sequence answers all four, and [`Elaborator::bind_params`] is
//! the single call site both the module lane (`instance.rs`) and the interface
//! window (`iface_inst.rs`) already share.
//!
//! ⚠️ Scope, measured — this refuses a SECOND DECLARATION, never a shadow and
//! never an override:
//! * a generate-block `localparam` of a header parameter's name is a nested
//!   scope (§27.3) and both oracles ACCEPT it (`gP=7` then `P=3`); it is not a
//!   `ModuleItem::Param` of the module body, so this walk never sees it;
//! * `#(.P(9))` / `defparam` / `-G` are overrides of one declaration, and both
//!   oracles accept them;
//! * a package or `$unit` constant of the same name is an import shadow, and
//!   both oracles accept it (`import pk::*` + `localparam int P = 7` gives 7);
//! * a body `localparam` whose name is a header parameter of a DIFFERENT
//!   module is unrelated — this walk is per-`ModuleDecl`.
//!
//! ⚠️ NOT covered, measured, pre-existing: a PACKAGE body that declares one
//! parameter name twice (`package pk; parameter int P = 3; parameter int P = 7;`
//! gives vita 7 at exit 0, and both oracles reject it). A package has no
//! parameter port list, so `bind_params` never runs for one; `package.rs`'s own
//! loop is that lane, and the duplicate guard it already carries covers
//! parameter-vs-variable only.

use super::*;

/// The display name of a parameter declaration, and what to call it.
///
/// A `parameter type T` is lowered by the parser into the synthesized carriers
/// `T$w` / `T$s` (plus `T$d…` / `T$p…` extents), so a duplicated type parameter
/// arrives here under a name the user never wrote. Reporting `T$w` would send
/// the reader hunting a declaration that is not in the source, so the text
/// before the first `$` is what gets named.
fn display_name(raw: &str) -> (&str, &str) {
    match raw.split_once('$') {
        Some((base, _)) if !base.is_empty() => (base, "type parameter"),
        _ => (raw, "parameter"),
    }
}

fn keyword_of(p: &ast::ParamDecl) -> &'static str {
    match p.kind {
        ast::ParamKind::Parameter => "parameter",
        ast::ParamKind::Localparam => "localparam",
    }
}

impl Elaborator<'_> {
    /// Every parameter DECLARATION of one module/interface scope in source
    /// order: the ANSI `#(...)` header list first, then the body
    /// `ModuleItem::Param`s in declaration order.
    ///
    /// Deliberately NOT [`Elaborator::param_ports`], which answers a different
    /// question (which declarations an override may target) and therefore
    /// returns the header *or* the body, never both — the pair this gate exists
    /// to find would be invisible to it. A header ARRAY parameter's body twin is
    /// a `ModuleItem::NetVar` (`ParamItem::ConstArrayVar`), not a `Param`, so the
    /// parser's own desugar never looks like a user duplicate here.
    pub(crate) fn scope_param_decls(module: &ast::ModuleDecl) -> Vec<&ast::ParamDecl> {
        module
            .params
            .iter()
            .chain(module.body.iter().filter_map(|it| match it {
                ast::ModuleItem::Param(p) => Some(p),
                _ => None,
            }))
            .collect()
    }

    /// Refuse every parameter declaration whose name a previous declaration in
    /// the SAME scope already took. Called once from `bind_params`, i.e. once
    /// per instance — the report is keyed on the offending declaration's NAME
    /// span so a module instantiated N times says it once, and so the several
    /// carriers of one duplicated `parameter type` (`T$w`, `T$s`, `T$d0a`, …),
    /// which all carry the name token's span, say it once as well.
    pub(crate) fn check_duplicate_param_decls(&mut self, module: &ast::ModuleDecl) {
        use std::collections::btree_map::Entry;
        let decls = Self::scope_param_decls(module);
        // name → the FIRST declaration of it. Source order, and never replaced,
        // so a third declaration of one name points back at the first — which is
        // where both oracles point ("It was declared here as a parameter").
        let mut first: std::collections::BTreeMap<&str, &ast::ParamDecl> =
            std::collections::BTreeMap::new();
        let mut reports: Vec<(ast::Span, ast::Span, String)> = Vec::new();
        for p in decls.iter().copied() {
            let prev = match first.entry(p.name.name.as_str()) {
                Entry::Vacant(v) => {
                    v.insert(p);
                    continue;
                }
                Entry::Occupied(o) => *o.get(),
            };
            let (disp, what) = display_name(&p.name.name);
            reports.push((
                p.name.span,
                prev.name.span,
                // No oracle wording is quoted here on purpose: iverilog rejects a
                // BODY `parameter type` as a syntax error rather than as a
                // redeclaration, so a quoted "already been declared in this
                // scope" would be a false sentence on the type-parameter cell.
                format!(
                    "duplicate declaration of {what} `{disp}`: this `{}` repeats the `{}` \
                     already declared in this scope — a parameter port list and the module \
                     body are ONE declarative scope, so a name is declared there once \
                     (IEEE 1800-2017 §6.20.1). Rename one of them",
                    keyword_of(p),
                    keyword_of(prev)
                ),
            ));
        }
        for (dup_span, first_span, msg) in reports {
            // Keyed on the NAME token: the `T$w` / `T$s` carriers of one
            // duplicated `parameter type T` share it, and so does every instance
            // of the module that declares it.
            if !self.reported_dup_params.insert((dup_span.lo, dup_span.hi)) {
                continue;
            }
            self.error_at(MsgCode::ElabUnsupported, dup_span, &msg);
            self.note_at(
                MsgCode::ElabUnsupported,
                first_span,
                "the first declaration of that name is here",
            );
        }
    }
}
