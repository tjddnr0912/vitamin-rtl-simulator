//! procedural block-local hoisting — split out of the original `elaborate` lib.rs.
//!
//! R17 split this file again (it had grown to 1248 lines, past the 1000-line module
//! cap) along the line the code already divided on:
//!   * [`gate`] — may this block-local be flattened? The callee-effect resolver, the
//!     definite-assignment entry point, the scope-leak check, and their diagnostics.
//!   * [`hoist`] — flatten it. Net creation and declaration-initializer collection.
//!
//! What stays here is the block-walking free functions both need, plus the unrelated
//! module-scope `gather_local_decl_names`.

use super::*;

mod gate;
mod hoist;

/// Collect every NESTED `begin…end`/`fork` block (i.e. not the top-level body
/// block) together with the names it declares as locals. `is_top` is true only for
/// the outermost body statement, whose own decls are function/task-scoped (not
/// block-scoped) and so are skipped.
pub(crate) fn gather_nested_block_locals(
    s: &ast::Stmt,
    is_top: bool,
    out: &mut Vec<(ast::Span, Vec<(String, ast::Span)>)>,
) {
    use ast::Stmt::*;
    match s {
        Block {
            stmts, decls, span, ..
        }
        | Fork {
            stmts, decls, span, ..
        } => {
            if !is_top && !decls.is_empty() {
                // R17 §4.1: each name carries its own DECLARATION span, so the
                // scope-leak diagnostic can be anchored at the declaration the user
                // has to rename rather than at nothing at all.
                let names: Vec<(String, ast::Span)> = decls
                    .iter()
                    .flat_map(|d| d.names.iter().map(|n| (n.name.name.clone(), d.span)))
                    .collect();
                if !names.is_empty() {
                    out.push((*span, names));
                }
            }
            for st in stmts {
                gather_nested_block_locals(st, false, out);
            }
        }
        If { then_s, else_s, .. } => {
            gather_nested_block_locals(then_s, false, out);
            if let Some(e) = else_s {
                gather_nested_block_locals(e, false, out);
            }
        }
        Case { items, .. } => {
            for it in items {
                match it {
                    ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                        gather_nested_block_locals(body, false, out)
                    }
                }
            }
        }
        For { body, .. } | While { body, .. } | Repeat { body, .. } | Forever { body, .. } => {
            gather_nested_block_locals(body, false, out)
        }
        Wait { body, .. } | DelayCtrl { body, .. } | EventCtrl { body, .. } => {
            if let Some(b) = body {
                gather_nested_block_locals(b, false, out)
            }
        }
        _ => {}
    }
}

/// Collect every `begin`/`fork` block-local declaration in `s`, in source order
/// (recursing into nested blocks and control-flow bodies). Used to reserve a
/// frame body's block-locals under its `$func$<name>` scope so a local declared
/// inside a `begin … end` resolves (previously unresolved → E3010, for frame
/// functions AND tasks alike).
pub(crate) fn collect_block_local_decls(s: &ast::Stmt, out: &mut Vec<ast::NetVarDecl>) {
    use ast::Stmt::*;
    match s {
        Block { decls, stmts, .. } | Fork { decls, stmts, .. } => {
            out.extend(decls.iter().cloned());
            for st in stmts {
                collect_block_local_decls(st, out);
            }
        }
        If { then_s, else_s, .. } => {
            collect_block_local_decls(then_s, out);
            if let Some(e) = else_s {
                collect_block_local_decls(e, out);
            }
        }
        Case { items, .. } => {
            for it in items {
                match it {
                    ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                        collect_block_local_decls(body, out)
                    }
                }
            }
        }
        For { body, .. } | While { body, .. } | Repeat { body, .. } | Forever { body, .. } => {
            collect_block_local_decls(body, out)
        }
        _ => {}
    }
}

impl Elaborator<'_> {
    /// Param-aware const-eval in a SIGNED 64-bit domain (P0-6, 2026-06-10).
    /// Folds: literals (sign-aware), params/genvars in scope, unary `+ - ~ !`,
    /// the binary operator set with i64 semantics (so a descending genvar
    /// condition `i >= 0` actually terminates), ternary `?:` and `$clog2`
    /// (P0-5 — the `localparam AW = $clog2(DEPTH)` / `W = M ? a : b` idioms).
    /// Overflow and ill-defined folds return None — param-binding callers
    /// escalate None to an ERROR (never a silent 0), width callers clamp
    /// loudly. NOTE: this is a width-less mathematical-integer model; a
    /// logical `>>` of a NEGATIVE value is width-dependent and folds None.
    /// GAP-G shadow guard: the bare names the module declares locally — header
    /// (`#(...)`) params, ports, and top-level body nets/params. A name in this
    /// set SHADOWS a same-named wildcard-imported package array, so a const
    /// element read of it must not fold the imported array. See `local_decl_names`.
    pub(crate) fn gather_local_decl_names(
        &self,
        module: &ast::ModuleDecl,
    ) -> std::collections::BTreeSet<String> {
        let mut names = std::collections::BTreeSet::new();
        for p in &module.params {
            names.insert(p.name.name.clone());
        }
        match &module.ports {
            ast::PortList::Ansi(ports) => {
                for pt in ports {
                    names.insert(pt.name.name.clone());
                }
            }
            ast::PortList::NonAnsi(idents) => {
                for id in idents {
                    names.insert(id.name.clone());
                }
            }
            ast::PortList::None => {}
        }
        for item in &module.body {
            match item {
                ast::ModuleItem::NetVar(d) => {
                    for n in &d.names {
                        names.insert(n.name.name.clone());
                    }
                }
                ast::ModuleItem::Param(p) => {
                    names.insert(p.name.name.clone());
                }
                _ => {}
            }
        }
        names
    }
}
