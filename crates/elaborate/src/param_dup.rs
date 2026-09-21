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
//! declaration sequence answers all four.
//!
//! ⚠️ It runs per DEFINITION, from `driver.rs::run`, beside
//! [`Elaborator::check_decl_name_collisions`]. It used to run from
//! [`Elaborator::bind_params`], i.e. once per INSTANCE, which made it blind to
//! every scope the design never elaborates — measured: a module named only under
//! `generate if (0)` (census p25_a1, `TOP=ok` at exit 0), a module with no
//! instance at all under `--top` (p25_a2 staged, silent), and an INTERFACE with no
//! instance, which is never elaborated at all (p25_a4). Both oracles reject the
//! first; iverilog rejects all three. Declaration legality is not a property of
//! instantiation, so the definition is the right cadence.
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
//! ⚠️ Both shapes that used to stand here are covered now, by SIBLING rules rather
//! than by this walk: the `generate if (0)` module is reached by the
//! per-DEFINITION cadence above, and a duplicate VARIABLE in a block is refused by
//! its own binder (`block_local/hoist.rs`'s per-block flatten-key set, and
//! `inline_task.rs`'s per-inlining one for a static task body). This walk is still
//! `ModuleItem::Param` only.

use super::*;

/// The display name of a parameter declaration, and what to call it.
///
/// A `parameter type T` is lowered by the parser into the carriers `T$w` / `T$s`
/// (plus `T$d…` / `T$p…` extents), so a duplicated type parameter arrives here
/// under a name the user never wrote. Reporting `T$w` would send the reader
/// hunting a declaration that is not in the source, so the stem gets named.
///
/// ⚠️ `stems` is the IDENTITY test, and it is the whole rule: it holds exactly the
/// stems THIS declaration list proves are type parameters (both `<stem>$w` and
/// `<stem>$s` present — what the producer always and only mints for one). A name
/// SHAPE test reported two user `localparam int Q$d0a` as "type parameter `Q`",
/// a declaration and a keyword the file does not contain.
fn display_name<'a>(raw: &'a str, stems: &BTreeSet<&str>) -> (&'a str, &'static str) {
    match decl_collide::carrier_stem(raw) {
        Some(stem) if stems.contains(stem) => (stem, "type parameter"),
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
    ///
    /// Each declaration carries whether it was reached THROUGH a transparent
    /// `generate … endgenerate` region, because that decides which scope rule the
    /// refusal may quote: the §6.20.1 sentence about a parameter port list is
    /// false for a pair the region contributed (`generate localparam int Q = 1;
    /// localparam int Q = 2; endgenerate` has no parameter port list in it at
    /// all).
    pub(crate) fn scope_param_decls(module: &ast::ModuleDecl) -> Vec<(&ast::ParamDecl, bool)> {
        let mut out: Vec<(&ast::ParamDecl, bool)> =
            module.params.iter().map(|p| (p, false)).collect();
        Self::push_item_param_decls(&module.body, false, &mut out);
        out
    }

    /// The parameter declarations of a MODULE-ITEM sequence, in source order —
    /// a package body's declarative region.
    pub(crate) fn item_param_decls(items: &[ast::ModuleItem]) -> Vec<(&ast::ParamDecl, bool)> {
        let mut out = Vec::new();
        Self::push_item_param_decls(items, false, &mut out);
        out
    }

    /// The parameter declarations of ONE generate scope level, in source order.
    /// A nested `for` / `if` / `case` / LABELLED block is its own declarative
    /// region (§27.3), is walked by its own call, and legitimately shadows an
    /// outer name — those are skipped; a TRANSPARENT nested region is flattened
    /// in, exactly as at module scope.
    pub(crate) fn gen_param_decls(items: &[ast::GenItem]) -> Vec<(&ast::ParamDecl, bool)> {
        let mut out = Vec::new();
        Self::push_gen_param_decls(items, false, &mut out);
        out
    }

    /// Append the parameter declarations a MODULE-ITEM sequence contributes to
    /// ITS OWN declarative region, in source order: its own `Param`s, plus every
    /// transparent `generate` region's.
    fn push_item_param_decls<'a>(
        items: &'a [ast::ModuleItem],
        in_region: bool,
        out: &mut Vec<(&'a ast::ParamDecl, bool)>,
    ) {
        for it in items {
            match it {
                ast::ModuleItem::Param(p) => out.push((p, in_region)),
                // Everything below a `generate … endgenerate` is IN a transparent
                // region, however deeply nested — the flag never clears.
                ast::ModuleItem::Generate(g) => Self::push_gen_param_decls(&g.items, true, out),
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
    fn push_gen_param_decls<'a>(
        items: &'a [ast::GenItem],
        in_region: bool,
        out: &mut Vec<(&'a ast::ParamDecl, bool)>,
    ) {
        for it in items {
            match it {
                ast::GenItem::Item(b) => match b.as_ref() {
                    ast::ModuleItem::Param(p) => out.push((p, in_region)),
                    ast::ModuleItem::Generate(g) => Self::push_gen_param_decls(&g.items, true, out),
                    _ => {}
                },
                ast::GenItem::Block {
                    label: None, items, ..
                } => Self::push_gen_param_decls(items, in_region, out),
                _ => {}
            }
        }
    }

    /// Refuse every parameter declaration whose name a previous declaration in
    /// the SAME scope already took. Called once per DEFINITION from
    /// `driver.rs::run` — the report is still keyed on the offending
    /// declaration's NAME span, which is what makes the several carriers of one
    /// duplicated `parameter type` (`T$w`, `T$s`, `T$d0a`, …), all carrying the
    /// name token's span, say it once rather than once each.
    pub(crate) fn check_duplicate_param_decls(&mut self, module: &ast::ModuleDecl, unit: UnitKind) {
        let w = unit.word();
        self.check_duplicate_param_decls_in(
            &Self::scope_param_decls(module),
            &format!(
                "a parameter port list and the {w} body are ONE declarative scope, so a \
                 name is declared there once (IEEE 1800-2017 §6.20.1)"
            ),
            &Self::transparent_region_rule(w),
        );
    }

    /// The scope sentence for a pair a TRANSPARENT `generate … endgenerate`
    /// region contributed. `enclosing` is what the region's names actually belong
    /// to — the module, the interface, or the generate block the region sits in.
    ///
    /// Its own sentence because the §6.20.1 one names a parameter port list, and
    /// `generate localparam int Q = 1; localparam int Q = 2; endgenerate` in a
    /// module with no header at all was told its parameter port list collided
    /// with the module body (measured cell x7_a), which sends the reader to a
    /// construct the file does not contain.
    pub(crate) fn transparent_region_rule(enclosing: &str) -> String {
        format!(
            "a `generate … endgenerate` region with no block label is TRANSPARENT — its \
             declarations belong to the enclosing {enclosing} scope (IEEE 1800-2017 \
             §27.2/§27.3), so a name is declared there once"
        )
    }

    /// The walk itself, over any declaration sequence that forms ONE declarative
    /// region. `scope_rule` completes the diagnostic's sentence with the rule of
    /// the region the caller is walking, because the §6.20.1 sentence about a
    /// parameter port list is false for a generate block and for a package.
    ///
    /// `transparent_rule` is the sentence for a pair a TRANSPARENT
    /// `generate … endgenerate` region contributed — see
    /// [`Elaborator::transparent_region_rule`].
    ///
    /// The span dedupe (`reported_dup_params`) is the shared part and is what
    /// makes the extra call sites free: a generate scope is re-walked once per
    /// GenPhase and once per loop ITERATION, and a package body once per
    /// elaboration, so every one of them reports the same NAME span and only the
    /// first survives.
    pub(crate) fn check_duplicate_param_decls_in(
        &mut self,
        decls: &[(&ast::ParamDecl, bool)],
        scope_rule: &str,
        transparent_rule: &str,
    ) {
        use std::collections::btree_map::Entry;
        // name → the FIRST declaration of it. Source order, and never replaced,
        // so a third declaration of one name points back at the first — which is
        // where both oracles point ("It was declared here as a parameter").
        let mut first: std::collections::BTreeMap<&str, (&ast::ParamDecl, bool)> =
            std::collections::BTreeMap::new();
        let mut reports: Vec<(ast::Span, ast::Span, String)> = Vec::new();
        // The stems this list proves are `parameter type` declarations — see
        // [`display_name`]. Built from the same declarations the walk judges, so a
        // carrier is named by its stem exactly where the parser minted one.
        let stems = decl_collide::carrier_stems(decls.iter().map(|(p, _)| p.name.name.as_str()));
        for (p, in_region) in decls.iter().copied() {
            let (prev, prev_region) = match first.entry(p.name.name.as_str()) {
                Entry::Vacant(v) => {
                    v.insert((p, in_region));
                    continue;
                }
                Entry::Occupied(o) => *o.get(),
            };
            let (disp, what) = display_name(&p.name.name, &stems);
            // EITHER side reached through a transparent region makes the §6.20.1
            // "parameter port list" sentence false for this pair — the flatten is
            // what put the two declarations in one region, so the flatten's own
            // rule is the one that explains the refusal.
            let rule = if in_region || prev_region {
                transparent_rule
            } else {
                scope_rule
            };
            reports.push((
                p.name.span,
                prev.name.span,
                // No oracle wording is quoted here on purpose: iverilog rejects a
                // BODY `parameter type` as a syntax error rather than as a
                // redeclaration, so a quoted "already been declared in this
                // scope" would be a false sentence on the type-parameter cell.
                format!(
                    "duplicate declaration of {what} `{disp}`: this `{}` repeats the `{}` \
                     already declared in this scope — {rule}. Rename one of them",
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
