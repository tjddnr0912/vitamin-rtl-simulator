//! param-class monomorphization — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

// v5 ⑥ foreach desugar: rename every SINGLE-SEGMENT `Ident` reference to
// `from` into `to`, across a statement tree — exprs, lvalues, nested stmts,
// block-local decl initializers/dims AND event-control sensitivity exprs
// (the last two were review finding 2026-06-11: a missed arm silently binds
// the reference to the OUTER variable). Multi-segment paths are left alone
// (`x.y` never names the loop index).
// ──────────── ⓑ-breadth (§8.25): parameterized-class monomorphization ────────
// Substitute class value parameters (`name → value-expr`) throughout a class's
// declarations so each specialization is a fully-concrete class. Coverage is the
// procedural/declarative subset a class body uses; any un-covered position simply
// keeps the parameter name, which then fails LOUD at elaborate (undeclared) — never
// a silent miscompile.

/// Replace every single-segment `Ident` matching a key in `map` with that key's
/// value-expression (cloned). Recurses through the full `ExprKind` closure.
pub(crate) fn subst_expr(e: &mut Expr, map: &std::collections::BTreeMap<String, Expr>) {
    match &mut e.kind {
        ExprKind::Ident(p) => {
            if p.segments.len() == 1 {
                if let Some(v) = map.get(&p.segments[0].name) {
                    let span = e.span;
                    *e = v.clone();
                    e.span = span;
                }
            }
        }
        ExprKind::Unary { operand, .. } => subst_expr(operand, map),
        ExprKind::Binary { lhs, rhs, .. } => {
            subst_expr(lhs, map);
            subst_expr(rhs, map);
        }
        ExprKind::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            subst_expr(cond, map);
            subst_expr(then_e, map);
            subst_expr(else_e, map);
        }
        ExprKind::BitSelect { base, index } => {
            subst_expr(base, map);
            subst_expr(index, map);
        }
        ExprKind::PartSelect { base, msb, lsb } => {
            subst_expr(base, map);
            subst_expr(msb, map);
            subst_expr(lsb, map);
        }
        ExprKind::IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            subst_expr(base, map);
            subst_expr(offset, map);
            subst_expr(width, map);
        }
        ExprKind::Concat { parts } => {
            for p in parts {
                subst_expr(p, map);
            }
        }
        ExprKind::AssignPattern(elems) => {
            for el in elems {
                subst_expr(el, map);
            }
        }
        // Keys are member NAMES / `default`, never parameter references — only the
        // value side can mention a parameter.
        ExprKind::AssignPatternKeyed(elems) => {
            for (_, v) in elems {
                subst_expr(v, map);
            }
        }
        ExprKind::Replicate { count, value } => {
            subst_expr(count, map);
            for v in value {
                subst_expr(v, map);
            }
        }
        ExprKind::Call { args, .. } | ExprKind::SysCall { args, .. } => {
            for a in args {
                subst_expr(a, map);
            }
        }
        ExprKind::RandomizeWith(b) => {
            for a in b.args.iter_mut().chain(b.constraints.iter_mut()) {
                subst_expr(a, map);
            }
        }
        ExprKind::ArrayMethodWith(b) => subst_expr(&mut b.with_expr, map),
        ExprKind::Paren { inner } => subst_expr(inner, map),
        ExprKind::TimeLit { num, .. } => subst_expr(num, map),
        ExprKind::NamedArg { value, .. } => {
            if let Some(v) = value {
                subst_expr(v, map);
            }
        }
        ExprKind::MethodCall { recv, args, .. } => {
            subst_expr(recv, map);
            for a in args {
                subst_expr(a, map);
            }
        }
        ExprKind::MinTypMax { min, typ, max } => {
            subst_expr(min, map);
            subst_expr(typ, map);
            subst_expr(max, map);
        }
        ExprKind::New { size, src } => {
            subst_expr(size, map);
            if let Some(s) = src {
                subst_expr(s, map);
            }
        }
        ExprKind::ClassNew { args } => {
            for a in args {
                subst_expr(a, map);
            }
        }
        ExprKind::Dist { value, items } => {
            subst_expr(value, map);
            for it in items {
                subst_expr(&mut it.lo, map);
                if let Some(h) = &mut it.hi {
                    subst_expr(h, map);
                }
                subst_expr(&mut it.weight, map);
            }
        }
        ExprKind::Cast { target, expr } => {
            // Substitute the operand AND (for a size cast) the width expression —
            // a param-width cast `WIDTH'(x)` must monomorphize the WIDTH too.
            if let CastTarget::Size(w) = target {
                subst_expr(w, map);
            }
            subst_expr(expr, map);
        }
        ExprKind::IntLit { .. }
        | ExprKind::RealLit { .. }
        | ExprKind::StrLit { .. }
        | ExprKind::PkgScoped { .. }
        | ExprKind::Null
        | ExprKind::Dollar
        | ExprKind::Error => {}
    }
}

pub(crate) fn subst_opt_expr(e: &mut Option<Expr>, map: &std::collections::BTreeMap<String, Expr>) {
    if let Some(x) = e {
        subst_expr(x, map);
    }
}

pub(crate) fn subst_range(r: &mut Option<Range>, map: &std::collections::BTreeMap<String, Expr>) {
    if let Some(rng) = r {
        subst_expr(&mut rng.msb, map);
        subst_expr(&mut rng.lsb, map);
    }
}

pub(crate) fn subst_netvar(d: &mut NetVarDecl, map: &std::collections::BTreeMap<String, Expr>) {
    subst_range(&mut d.range, map);
    for p in &mut d.packed {
        subst_expr(&mut p.msb, map);
        subst_expr(&mut p.lsb, map);
    }
    for n in &mut d.names {
        for dim in &mut n.unpacked {
            match dim {
                Dim::Range(rg) => {
                    subst_expr(&mut rg.msb, map);
                    subst_expr(&mut rg.lsb, map);
                }
                Dim::Size(e) => subst_expr(e, map),
                Dim::Dyn | Dim::Queue(_) | Dim::Assoc(_) => {}
            }
        }
        subst_opt_expr(&mut n.init, map);
    }
}

/// Substitute params through the common procedural statement forms a class method
/// body uses. Un-covered forms keep the parameter (→ loud at elaborate).
pub(crate) fn subst_stmt(s: &mut Stmt, map: &std::collections::BTreeMap<String, Expr>) {
    match s {
        Stmt::Blocking { rhs, .. } | Stmt::NonBlocking { rhs, .. } => subst_expr(rhs, map),
        Stmt::If {
            cond,
            then_s,
            else_s,
            ..
        } => {
            subst_expr(cond, map);
            subst_stmt(then_s, map);
            if let Some(e) = else_s {
                subst_stmt(e, map);
            }
        }
        Stmt::Return { value, .. } => subst_opt_expr(value, map),
        Stmt::Case {
            scrutinee, items, ..
        } => {
            subst_expr(scrutinee, map);
            for it in items {
                match it {
                    CaseItem::Match { labels, body, .. } => {
                        for l in labels {
                            subst_expr(l, map);
                        }
                        subst_stmt(body, map);
                    }
                    CaseItem::Default { body, .. } => subst_stmt(body, map),
                }
            }
        }
        Stmt::For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            subst_stmt(init, map);
            subst_expr(cond, map);
            subst_stmt(step, map);
            subst_stmt(body, map);
        }
        Stmt::While { cond, body, .. }
        | Stmt::Repeat {
            count: cond, body, ..
        } => {
            subst_expr(cond, map);
            subst_stmt(body, map);
        }
        Stmt::Forever { body, .. } => subst_stmt(body, map),
        Stmt::Block { decls, stmts, .. } | Stmt::Fork { decls, stmts, .. } => {
            // A block-local declared with a parameter's name SHADOWS it: its decl
            // ranges/inits still use the outer params, but inside the block that
            // name must NOT be substituted (else a local read silently becomes the
            // parameter value — a silent-wrong).
            for d in decls.iter_mut() {
                subst_netvar(d, map);
            }
            let inner = map_without(
                map,
                decls
                    .iter()
                    .flat_map(|d| d.names.iter().map(|n| &n.name.name)),
            );
            let m = inner.as_ref().unwrap_or(map);
            for st in stmts.iter_mut() {
                subst_stmt(st, m);
            }
        }
        Stmt::SysTaskCall { args, .. } => {
            for a in args {
                subst_expr(a, map);
            }
        }
        Stmt::UserTaskCall { args, .. } => {
            for a in args {
                subst_expr(a, map);
            }
        }
        Stmt::RandomizeWith {
            args, constraints, ..
        } => {
            for a in args.iter_mut().chain(constraints.iter_mut()) {
                subst_expr(a, map);
            }
        }
        _ => {}
    }
}

/// A copy of `map` with `names` removed, or `None` when nothing was removed (so
/// the caller can keep using the original by reference — no allocation in the
/// common no-shadow case).
pub(crate) fn map_without<'a>(
    map: &std::collections::BTreeMap<String, Expr>,
    names: impl Iterator<Item = &'a String>,
) -> Option<std::collections::BTreeMap<String, Expr>> {
    let drop: Vec<&String> = names.filter(|n| map.contains_key(*n)).collect();
    if drop.is_empty() {
        return None;
    }
    let mut m = map.clone();
    for n in drop {
        m.remove(n);
    }
    Some(m)
}

pub(crate) fn subst_class_item(it: &mut ClassItem, map: &std::collections::BTreeMap<String, Expr>) {
    match it {
        ClassItem::Property(_, d) => subst_netvar(d, map),
        ClassItem::RandProperty { decl, .. } => subst_netvar(decl, map),
        ClassItem::Constraint(cd) => {
            for e in &mut cd.exprs {
                subst_expr(e, map);
            }
        }
        ClassItem::Func { def, .. } => {
            subst_range(&mut def.range, map);
            for p in &mut def.ports {
                subst_range(&mut p.range, map);
            }
            for d in &mut def.body_decls {
                subst_netvar(d, map);
            }
            // method args / locals shadow class params inside the body.
            let shadow = def.ports.iter().map(|p| &p.name.name).chain(
                def.body_decls
                    .iter()
                    .flat_map(|d| d.names.iter().map(|n| &n.name.name)),
            );
            let inner = map_without(map, shadow);
            subst_stmt(&mut def.body, inner.as_ref().unwrap_or(map));
        }
        ClassItem::Task { def, .. } => {
            for p in &mut def.ports {
                subst_range(&mut p.range, map);
            }
            for d in &mut def.body_decls {
                subst_netvar(d, map);
            }
            let shadow = def.ports.iter().map(|p| &p.name.name).chain(
                def.body_decls
                    .iter()
                    .flat_map(|d| d.names.iter().map(|n| &n.name.name)),
            );
            let inner = map_without(map, shadow);
            subst_stmt(&mut def.body, inner.as_ref().unwrap_or(map));
        }
        ClassItem::Error(_) => {}
    }
}

/// Build a fully-concrete specialization of a parameterized class: clone the
/// template, substitute the parameter values, clear the param list, and rename.
/// Render a parameterized-class specialization argument to an identifier-safe
/// string for the mangled class name. v1 accepts integer literals (and a leading
/// `-`/parens); anything else returns `None` (→ a loud reject upstream).
pub(crate) fn arg_render(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::IntLit { raw, .. } => {
            let s: String = raw.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        ExprKind::Unary {
            op: UnOp::Minus,
            operand,
        } => arg_render(operand).map(|s| format!("n{s}")),
        ExprKind::Paren { inner } => arg_render(inner),
        _ => None,
    }
}

pub(crate) fn monomorphize_class(
    template: &ClassDecl,
    new_name: &str,
    map: &std::collections::BTreeMap<String, Expr>,
) -> ClassDecl {
    let mut c = template.clone();
    c.name.name = new_name.to_string();
    c.params = Vec::new();
    // A class MEMBER (field/method) named like a parameter shadows it (a degenerate
    // §13.3 collision); the member wins. Excluding such names keeps the field/method
    // working — a width that then references the (shadowed) param is a non-constant
    // ref → loud at elaborate, never a silent miscompile.
    let members = c.items.iter().flat_map(|it| -> Vec<&String> {
        match it {
            ClassItem::Property(_, d) => d.names.iter().map(|n| &n.name.name).collect(),
            ClassItem::RandProperty { decl, .. } => {
                decl.names.iter().map(|n| &n.name.name).collect()
            }
            ClassItem::Func { def, .. } => vec![&def.name.name],
            ClassItem::Task { def, .. } => vec![&def.name.name],
            _ => Vec::new(),
        }
    });
    let eff = map_without(map, members);
    let m = eff.as_ref().unwrap_or(map);
    for it in &mut c.items {
        subst_class_item(it, m);
    }
    c
}

pub(crate) fn rename_ident_in_stmt(s: &mut Stmt, from: &str, to: &str) {
    let fix_path = |p: &mut HierPath| {
        if p.segments.len() == 1 && p.segments[0].name == from {
            p.segments[0].name = to.to_string();
        }
    };
    // SVA sequence antecedent: recurse into every boolean leaf so a loop-index
    // rename reaches sequence terms too (same outer-capture lesson as the
    // EventCtrl/foreach rename arms).
    fn fix_sequence(seq: &mut Sequence, from: &str, to: &str) {
        match seq {
            Sequence::Boolean(e) => fix_expr(e, from, to),
            Sequence::Delay { lhs, rhs, .. } => {
                fix_sequence(lhs, from, to);
                fix_sequence(rhs, from, to);
            }
            Sequence::Repeat { seq, .. } => fix_sequence(seq, from, to),
            Sequence::Throughout { cond, seq } => {
                fix_expr(cond, from, to);
                fix_sequence(seq, from, to);
            }
            Sequence::Within { seq1, seq2 } => {
                fix_sequence(seq1, from, to);
                fix_sequence(seq2, from, to);
            }
            // A re-clocking boundary: recurse into the inner sequence. The clock is a
            // module-level signal (never a loop index — you cannot clock on a genvar),
            // so its sensitivity is not renamed.
            Sequence::Clocked { seq, .. } => fix_sequence(seq, from, to),
            // A named instance: the `name` is a sequence/property identifier (not a
            // loop index), so it is never renamed; only the (reserved) actual-arg
            // expressions are.
            Sequence::Instance { args, .. } => {
                for a in args.iter_mut() {
                    fix_expr(a, from, to);
                }
            }
            // A match-item capture `(b, x = e)`: recurse into the boolean term and the
            // captured value expressions (a generate-loop index may appear in `e`); the
            // local-variable NAMES are not loop indices, so they are not renamed.
            Sequence::MatchItem { seq, assigns } => {
                fix_sequence(seq, from, to);
                for (_, val) in assigns.iter_mut() {
                    fix_expr(val, from, to);
                }
            }
        }
    }
    /// Rename a loop index inside an N2d property-expression tree (foreach desugar
    /// completeness — the antecedent sequences and nested consequents must all be
    /// renamed, mirroring `fix_sequence`). Property/recursion names are
    /// identifiers, not loop indices, so they are not renamed (they parse as bare
    /// `Seq(Boolean(Ident))` leaves and resolve at elaborate).
    fn fix_prop_expr(pe: &mut PropExpr, from: &str, to: &str) {
        match pe {
            PropExpr::Seq(s) => fix_sequence(s, from, to),
            PropExpr::Impl { ante, cons, .. } => {
                fix_sequence(ante, from, to);
                fix_prop_expr(cons, from, to);
            }
            PropExpr::And(l, r) | PropExpr::Or(l, r) => {
                fix_prop_expr(l, from, to);
                fix_prop_expr(r, from, to);
            }
            PropExpr::Not(p) => fix_prop_expr(p, from, to),
            PropExpr::Until { lhs, rhs, .. } => {
                fix_prop_expr(lhs, from, to);
                fix_prop_expr(rhs, from, to);
            }
            PropExpr::Eventually { prop, .. } => fix_prop_expr(prop, from, to),
            PropExpr::Always(p) => fix_prop_expr(p, from, to),
        }
    }
    fn fix_expr(e: &mut Expr, from: &str, to: &str) {
        match &mut e.kind {
            ExprKind::Ident(p) => {
                if p.segments.len() == 1 && p.segments[0].name == from {
                    p.segments[0].name = to.to_string();
                }
            }
            // v7: a package-scoped name can never be the loop index.
            ExprKind::PkgScoped { .. } => {}
            ExprKind::Unary { operand, .. } => fix_expr(operand, from, to),
            ExprKind::Binary { lhs, rhs, .. } => {
                fix_expr(lhs, from, to);
                fix_expr(rhs, from, to);
            }
            ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                fix_expr(cond, from, to);
                fix_expr(then_e, from, to);
                fix_expr(else_e, from, to);
            }
            ExprKind::BitSelect { base, index } => {
                fix_expr(base, from, to);
                fix_expr(index, from, to);
            }
            ExprKind::PartSelect { base, msb, lsb } => {
                fix_expr(base, from, to);
                fix_expr(msb, from, to);
                fix_expr(lsb, from, to);
            }
            ExprKind::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                fix_expr(base, from, to);
                fix_expr(offset, from, to);
                fix_expr(width, from, to);
            }
            ExprKind::Concat { parts } | ExprKind::Replicate { value: parts, .. } => {
                for p in parts {
                    fix_expr(p, from, to);
                }
            }
            ExprKind::AssignPattern(elems) => {
                for el in elems {
                    fix_expr(el, from, to);
                }
            }
            // Keys are member NAMES / `default`; only the value side holds an expr.
            ExprKind::AssignPatternKeyed(elems) => {
                for (_, v) in elems {
                    fix_expr(v, from, to);
                }
            }
            ExprKind::Call { args, .. } | ExprKind::SysCall { args, .. } => {
                for a in args {
                    fix_expr(a, from, to);
                }
            }
            ExprKind::RandomizeWith(b) => {
                for a in b.args.iter_mut().chain(b.constraints.iter_mut()) {
                    fix_expr(a, from, to);
                }
            }
            ExprKind::ArrayMethodWith(b) => fix_expr(&mut b.with_expr, from, to),
            ExprKind::Paren { inner } => fix_expr(inner, from, to),
            ExprKind::TimeLit { num, .. } => fix_expr(num, from, to),
            ExprKind::NamedArg { value, .. } => {
                if let Some(v) = value {
                    fix_expr(v, from, to);
                }
            }
            ExprKind::MethodCall { recv, args, .. } => {
                fix_expr(recv, from, to);
                for a in args {
                    fix_expr(a, from, to);
                }
            }
            ExprKind::MinTypMax { min, typ, max } => {
                fix_expr(min, from, to);
                fix_expr(typ, from, to);
                fix_expr(max, from, to);
            }
            ExprKind::New { size, src } => {
                fix_expr(size, from, to);
                if let Some(s) = src {
                    fix_expr(s, from, to);
                }
            }
            ExprKind::ClassNew { args } => {
                for a in args {
                    fix_expr(a, from, to);
                }
            }
            ExprKind::Dist { value, items } => {
                fix_expr(value, from, to);
                for it in items {
                    fix_expr(&mut it.lo, from, to);
                    if let Some(h) = &mut it.hi {
                        fix_expr(h, from, to);
                    }
                    fix_expr(&mut it.weight, from, to);
                }
            }
            ExprKind::Cast { target, expr } => {
                if let CastTarget::Size(w) = target {
                    fix_expr(w, from, to);
                }
                fix_expr(expr, from, to);
            }
            ExprKind::IntLit { .. }
            | ExprKind::RealLit { .. }
            | ExprKind::StrLit { .. }
            | ExprKind::Null
            | ExprKind::Dollar
            | ExprKind::Error => {}
        }
        // Replicate.count rides outside the parts vec.
        if let ExprKind::Replicate { count, .. } = &mut e.kind {
            fix_expr(count, from, to);
        }
    }
    fn fix_lv(lv: &mut Lvalue, from: &str, to: &str) {
        match lv {
            Lvalue::Ident(p) => {
                if p.segments.len() == 1 && p.segments[0].name == from {
                    p.segments[0].name = to.to_string();
                }
            }
            Lvalue::BitSelect { base, index, .. } => {
                fix_lv(base, from, to);
                fix_expr(index, from, to);
            }
            Lvalue::PartSelect { base, msb, lsb, .. } => {
                fix_lv(base, from, to);
                fix_expr(msb, from, to);
                fix_expr(lsb, from, to);
            }
            Lvalue::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                fix_lv(base, from, to);
                fix_expr(offset, from, to);
                fix_expr(width, from, to);
            }
            Lvalue::Concat { parts, .. } => {
                for p in parts {
                    fix_lv(p, from, to);
                }
            }
            Lvalue::Error(_) => {}
        }
    }
    let fix_delay = |d: &mut Delay, from: &str, to: &str| {
        for e in &mut d.values {
            fix_expr(e, from, to);
        }
    };
    match s {
        Stmt::Blocking {
            lhs, delay, rhs, ..
        }
        | Stmt::NonBlocking {
            lhs, delay, rhs, ..
        } => {
            fix_lv(lhs, from, to);
            if let Some(d) = delay {
                fix_delay(d, from, to);
            }
            fix_expr(rhs, from, to);
        }
        Stmt::If {
            cond,
            then_s,
            else_s,
            ..
        } => {
            fix_expr(cond, from, to);
            rename_ident_in_stmt(then_s, from, to);
            if let Some(e) = else_s {
                rename_ident_in_stmt(e, from, to);
            }
        }
        Stmt::Case {
            scrutinee, items, ..
        } => {
            fix_expr(scrutinee, from, to);
            for it in items {
                match it {
                    CaseItem::Match { labels, body, .. } => {
                        for l in labels {
                            fix_expr(l, from, to);
                        }
                        rename_ident_in_stmt(body, from, to);
                    }
                    CaseItem::Default { body, .. } => rename_ident_in_stmt(body, from, to),
                }
            }
        }
        Stmt::For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            rename_ident_in_stmt(init, from, to);
            fix_expr(cond, from, to);
            rename_ident_in_stmt(step, from, to);
            rename_ident_in_stmt(body, from, to);
        }
        Stmt::While { cond, body, .. } => {
            fix_expr(cond, from, to);
            rename_ident_in_stmt(body, from, to);
        }
        Stmt::Repeat { count, body, .. } => {
            fix_expr(count, from, to);
            rename_ident_in_stmt(body, from, to);
        }
        Stmt::Forever { body, .. } => rename_ident_in_stmt(body, from, to),
        Stmt::Block { decls, stmts, .. } | Stmt::Fork { decls, stmts, .. } => {
            // a nested redeclaration of the SAME name shadows — stop renaming
            // inside (its own occurrences already bind to the inner decl).
            if decls
                .iter()
                .any(|d| d.names.iter().any(|n| n.name.name == from))
            {
                return;
            }
            // decl INITIALIZERS and dimension exprs reference outer names too
            // (review finding 2026-06-11 — they live outside `stmts`).
            for d in decls.iter_mut() {
                if let Some(r) = &mut d.range {
                    fix_expr(&mut r.msb, from, to);
                    fix_expr(&mut r.lsb, from, to);
                }
                for r in &mut d.packed {
                    fix_expr(&mut r.msb, from, to);
                    fix_expr(&mut r.lsb, from, to);
                }
                for n in d.names.iter_mut() {
                    if let Some(e) = &mut n.init {
                        fix_expr(e, from, to);
                    }
                    for dim in &mut n.unpacked {
                        match dim {
                            Dim::Size(e) => fix_expr(e, from, to),
                            Dim::Range(r) => {
                                fix_expr(&mut r.msb, from, to);
                                fix_expr(&mut r.lsb, from, to);
                            }
                            Dim::Queue(Some(b)) => fix_expr(b, from, to),
                            Dim::Queue(None) | Dim::Dyn | Dim::Assoc(_) => {}
                        }
                    }
                }
            }
            for st in stmts {
                rename_ident_in_stmt(st, from, to);
            }
        }
        Stmt::SysTaskCall { args, .. } | Stmt::UserTaskCall { args, .. } => {
            for a in args {
                fix_expr(a, from, to);
            }
        }
        Stmt::RandomizeWith {
            args, constraints, ..
        } => {
            for a in args.iter_mut().chain(constraints.iter_mut()) {
                fix_expr(a, from, to);
            }
        }
        Stmt::DelayCtrl { delay, body, .. } => {
            fix_delay(delay, from, to);
            if let Some(b) = body {
                rename_ident_in_stmt(b, from, to);
            }
        }
        Stmt::EventCtrl { ctrl, body, .. } => {
            // the sensitivity exprs reference names too (review finding
            // 2026-06-11 — `@(arr[i])` inside a foreach body).
            if let Sensitivity::List(evs) = ctrl {
                for ev in evs {
                    fix_expr(&mut ev.expr, from, to);
                }
            }
            if let Some(b) = body {
                rename_ident_in_stmt(b, from, to);
            }
        }
        Stmt::Wait { cond, body, .. } => {
            fix_expr(cond, from, to);
            if let Some(b) = body {
                rename_ident_in_stmt(b, from, to);
            }
        }
        Stmt::Assign { lhs, rhs, .. } | Stmt::Force { lhs, rhs, .. } => {
            fix_lv(lhs, from, to);
            fix_expr(rhs, from, to);
        }
        Stmt::Deassign { lhs, .. } | Stmt::Release { lhs, .. } => fix_lv(lhs, from, to),
        Stmt::EventTrigger { name, .. } => fix_path(name),
        Stmt::ConcurrentAssert {
            clock,
            disable_iff,
            antecedent,
            consequent,
            pass,
            fail,
            prop_expr,
            ..
        } => {
            // Rename every operand (clock sensitivity exprs + disable iff +
            // antecedent + consequent + action-block statements + the N2d
            // property-expression tree) — same completeness lesson as EventCtrl
            // above (an unrenamed operand would silently capture the outer signal).
            if let Sensitivity::List(evs) = clock {
                for ev in evs {
                    fix_expr(&mut ev.expr, from, to);
                }
            }
            if let Some(e) = disable_iff {
                fix_expr(e, from, to);
            }
            fix_sequence(antecedent, from, to);
            fix_sequence(consequent, from, to);
            if let Some(pe) = prop_expr {
                fix_prop_expr(pe, from, to);
            }
            if let Some(s) = pass {
                rename_ident_in_stmt(s, from, to);
            }
            if let Some(s) = fail {
                rename_ident_in_stmt(s, from, to);
            }
        }
        Stmt::DeferredAssert {
            cond,
            then_s,
            else_s,
            ..
        } => {
            fix_expr(cond, from, to);
            rename_ident_in_stmt(then_s, from, to);
            rename_ident_in_stmt(else_s, from, to);
        }
        Stmt::Return { value, .. } => {
            if let Some(e) = value {
                fix_expr(e, from, to);
            }
        }
        Stmt::CoverProperty {
            clock,
            disable_iff,
            seq,
            ..
        } => {
            if let Sensitivity::List(evs) = clock {
                for ev in evs {
                    fix_expr(&mut ev.expr, from, to);
                }
            }
            if let Some(e) = disable_iff {
                fix_expr(e, from, to);
            }
            fix_sequence(seq, from, to);
        }
        Stmt::WaitFork { .. } | Stmt::Disable { .. } | Stmt::Null(_) | Stmt::Error(_) => {}
    }
}
