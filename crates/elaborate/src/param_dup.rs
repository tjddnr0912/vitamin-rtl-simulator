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
//! The walk is over a DECLARATION SEQUENCE, not over a module, so the two other
//! declarative regions that hold parameter declarations share it verbatim
//! ([`Elaborator::check_duplicate_param_decls_in`]), each supplying the sentence
//! that names its own scope rule:
//! * a GENERATE block (`generate.rs`, one call per scope level, IEEE §27.3) —
//!   `for (…) begin : g localparam Q = i; localparam Q = i + 1; end` answered
//!   `g Q=1 / g Q=2` at exit 0 where iverilog says "'Q' has already been declared
//!   in this scope" and verilator says "Duplicate declaration of signal: 'Q'";
//! * a PACKAGE body (`package.rs`, IEEE §26.2) — `package pk; parameter P = 1;
//!   parameter P = 2;` answered `P=2` at exit 0 against the same two refusals. A
//!   package has no parameter port list, so `bind_params` never runs for one; its
//!   own loop is the lane, and the duplicate guard it already carried covers
//!   parameter-vs-variable only.
//!
//! A TRANSPARENT `generate … endgenerate` REGION is the fourth place a
//! `ParamDecl` can sit and is NOT a fourth call site: IEEE §27.2 gives it no
//! scope, so [`Elaborator::scope_param_decls`] FLATTENS it into the module's own
//! sequence — which is also what makes the cross-region pairs (header + region,
//! body + region) refusable, as both oracles say they must be.
//!
//! ⚠️ Still NOT covered, measured, recorded: a module instantiated ONLY under
//! `generate if (0)` is never elaborated, so nothing walks its declarations
//! (`module dead #(parameter int P = 3); parameter int P = 7;` under a false
//! generate leaves vita at exit 0 while both oracles reject the file); and a
//! duplicate VARIABLE in a named block is a different name space with its own
//! binder (`integer x; integer x;` in one `begin : blk` runs, both oracles
//! reject).

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
    ///
    /// ⚠️ A TRANSPARENT `generate … endgenerate` REGION contributes its parameter
    /// declarations HERE, at the position of the `generate` item, because IEEE
    /// §27.2 gives an unlabelled region no scope of its own — its names ARE the
    /// module's. Filtering `module.body` for `ModuleItem::Param` alone missed
    /// them (they sit inside `ModuleItem::Generate`) while the §27.3 per-scope
    /// walk skips the transparent path, so `generate localparam int Q = 1;
    /// localparam int Q = 2; endgenerate` answered `Q=2` at exit 0 — iverilog
    /// "'Q' has already been declared in this scope. : It was declared here as a
    /// parameter.", verilator "Duplicate declaration of signal: 'Q'". Flattening
    /// (rather than a fourth call site) is also what makes the CROSS-region pairs
    /// refusable, and both oracles reject both of them: a header parameter plus a
    /// region `localparam` of that name, and a body `localparam` plus a region
    /// one.
    pub(crate) fn scope_param_decls(module: &ast::ModuleDecl) -> Vec<&ast::ParamDecl> {
        let mut out: Vec<&ast::ParamDecl> = module.params.iter().collect();
        Self::push_item_param_decls(&module.body, &mut out);
        out
    }

    /// The parameter declarations of a MODULE-ITEM sequence, in source order —
    /// a package body's declarative region.
    pub(crate) fn item_param_decls(items: &[ast::ModuleItem]) -> Vec<&ast::ParamDecl> {
        let mut out = Vec::new();
        Self::push_item_param_decls(items, &mut out);
        out
    }

    /// The parameter declarations of ONE generate scope level, in source order.
    /// A nested `for` / `if` / `case` / LABELLED block is its own declarative
    /// region (§27.3), is walked by its own call, and legitimately shadows an
    /// outer name — those are skipped; a TRANSPARENT nested region is flattened
    /// in, exactly as at module scope.
    pub(crate) fn gen_param_decls(items: &[ast::GenItem]) -> Vec<&ast::ParamDecl> {
        let mut out = Vec::new();
        Self::push_gen_param_decls(items, &mut out);
        out
    }

    /// Append the parameter declarations a MODULE-ITEM sequence contributes to
    /// ITS OWN declarative region, in source order: its own `Param`s, plus every
    /// transparent `generate` region's.
    fn push_item_param_decls<'a>(items: &'a [ast::ModuleItem], out: &mut Vec<&'a ast::ParamDecl>) {
        for it in items {
            match it {
                ast::ModuleItem::Param(p) => out.push(p),
                ast::ModuleItem::Generate(g) => Self::push_gen_param_decls(&g.items, out),
                _ => {}
            }
        }
    }

    /// The GEN-ITEM twin of [`Self::push_item_param_decls`]. TRANSPARENCY is the
    /// whole rule, and it is read off `elaborate_gen_item`: a `GenItem::Item` is a
    /// plain module item of the enclosing scope, a nested `generate` region is
    /// another transparent region, and an UNLABELLED `begin…end` in a gen-item
    /// list is the anachronistic surround (`elaborate_gen_scoped(None, …)`, no
    /// scope minted). Everything else — `for`, `if`, `case`, a LABELLED block —
    /// mints a scope and is judged by its own walk.
    fn push_gen_param_decls<'a>(items: &'a [ast::GenItem], out: &mut Vec<&'a ast::ParamDecl>) {
        for it in items {
            match it {
                ast::GenItem::Item(b) => match b.as_ref() {
                    ast::ModuleItem::Param(p) => out.push(p),
                    ast::ModuleItem::Generate(g) => Self::push_gen_param_decls(&g.items, out),
                    _ => {}
                },
                ast::GenItem::Block {
                    label: None, items, ..
                } => Self::push_gen_param_decls(items, out),
                _ => {}
            }
        }
    }

    /// Refuse every parameter declaration whose name a previous declaration in
    /// the SAME scope already took. Called once from `bind_params`, i.e. once
    /// per instance — the report is keyed on the offending declaration's NAME
    /// span so a module instantiated N times says it once, and so the several
    /// carriers of one duplicated `parameter type` (`T$w`, `T$s`, `T$d0a`, …),
    /// which all carry the name token's span, say it once as well.
    pub(crate) fn check_duplicate_param_decls(&mut self, module: &ast::ModuleDecl) {
        self.check_duplicate_param_decls_in(
            &Self::scope_param_decls(module),
            "a parameter port list and the module body are ONE declarative scope, so a \
             name is declared there once (IEEE 1800-2017 §6.20.1)",
        );
    }

    /// The walk itself, over any declaration sequence that forms ONE declarative
    /// region. `scope_rule` completes the diagnostic's sentence with the rule of
    /// the region the caller is walking, because the §6.20.1 sentence about a
    /// parameter port list is false for a generate block and for a package.
    ///
    /// The span dedupe (`reported_dup_params`) is the shared part and is what
    /// makes the extra call sites free: a generate scope is re-walked once per
    /// GenPhase and once per loop ITERATION, and a package body once per
    /// elaboration, so every one of them reports the same NAME span and only the
    /// first survives.
    pub(crate) fn check_duplicate_param_decls_in(
        &mut self,
        decls: &[&ast::ParamDecl],
        scope_rule: &str,
    ) {
        use std::collections::btree_map::Entry;
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
                     already declared in this scope — {scope_rule}. Rename one of them",
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
