//! constrained-random — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// N7-REST: the full draw range of an integral rand field. Width ≥ 63 falls back to
/// i64 extremes (the engine's full-width seeded path handles those).
pub(crate) fn rand_type_range(width: u32, signed: bool) -> (i64, i64) {
    if width == 0 {
        return (0, 0);
    }
    if width >= 63 {
        return if signed {
            (i64::MIN, i64::MAX)
        } else {
            (0, i64::MAX)
        };
    }
    if signed {
        (-(1i64 << (width - 1)), (1i64 << (width - 1)) - 1)
    } else {
        (0, (1i64 << width) - 1)
    }
}

/// Map an AST binary operator to a B2 constraint-predicate op (`None` ⇒ outside
/// the subset, e.g. shifts / bitwise — loud-rejected by the caller).
pub(crate) fn map_cbinop(op: ast::BinOp) -> Option<sim_ir::CBinOp> {
    use ast::BinOp as B;
    use sim_ir::CBinOp as C;
    Some(match op {
        B::Add => C::Add,
        B::Sub => C::Sub,
        B::Mul => C::Mul,
        B::Div => C::Div,
        B::Mod => C::Mod,
        B::Lt => C::Lt,
        B::Le => C::Le,
        B::Gt => C::Gt,
        B::Ge => C::Ge,
        B::Eq => C::Eq,
        B::Ne => C::Ne,
        B::LogAnd => C::And,
        B::LogOr => C::Or,
        _ => return None,
    })
}

/// Whether the constraint expression `e` references the field named `name`
/// anywhere (used to loud-reject a constraint on a `randc` field).
pub(crate) fn expr_mentions_field(e: &ast::Expr, name: &str) -> bool {
    use ast::ExprKind as K;
    match &e.kind {
        K::Ident(p) => p.segments.len() == 1 && p.segments[0].name == name,
        K::Unary { operand, .. } => expr_mentions_field(operand, name),
        K::Binary { lhs, rhs, .. } => {
            expr_mentions_field(lhs, name) || expr_mentions_field(rhs, name)
        }
        K::Paren { inner } => expr_mentions_field(inner, name),
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            expr_mentions_field(cond, name)
                || expr_mentions_field(then_e, name)
                || expr_mentions_field(else_e, name)
        }
        K::Dist { value, items } => {
            expr_mentions_field(value, name)
                || items.iter().any(|it| {
                    expr_mentions_field(&it.lo, name)
                        || it.hi.as_ref().is_some_and(|h| expr_mentions_field(h, name))
                        || expr_mentions_field(&it.weight, name)
                })
        }
        _ => false,
    }
}

/// Names of the rand fields a constraint list draws via a `dist` weighted sampler
/// (`field dist { … }`). Their `[lo,hi]` domain is IGNORED by the weighted draw,
/// so a single-field range on a dist field must stay a PREDICATE (checked after
/// the draw), not be folded into the domain.
pub(crate) fn dist_field_names(
    constraints: &[ast::ConstraintDecl],
) -> std::collections::BTreeSet<String> {
    let mut s = std::collections::BTreeSet::new();
    for c in constraints {
        for e in &c.exprs {
            collect_dist_fields(e, &mut s);
        }
    }
    s
}

pub(crate) fn collect_dist_fields(e: &ast::Expr, s: &mut std::collections::BTreeSet<String>) {
    match &e.kind {
        ast::ExprKind::Dist { value, .. } => {
            if let Some(n) = rand_field_ident(value) {
                s.insert(n);
            }
        }
        ast::ExprKind::Paren { inner } => collect_dist_fields(inner, s),
        ast::ExprKind::Binary {
            op: ast::BinOp::LogAnd,
            lhs,
            rhs,
        } => {
            collect_dist_fields(lhs, s);
            collect_dist_fields(rhs, s);
        }
        _ => {}
    }
}

/// A bare single-segment ident (a class field reference inside a constraint).
pub(crate) fn rand_field_ident(e: &ast::Expr) -> Option<String> {
    if let ast::ExprKind::Ident(p) = &e.kind {
        if p.segments.len() == 1 {
            return Some(p.segments[0].name.clone());
        }
    }
    None
}

/// Flip a comparison for a `const OP field` constraint into `field FLIP(OP) const`.
pub(crate) fn flip_cmp(op: ast::BinOp) -> ast::BinOp {
    use ast::BinOp::*;
    match op {
        Lt => Gt,
        Le => Ge,
        Gt => Lt,
        Ge => Le,
        other => other, // Eq is symmetric
    }
}

/// Narrow `[lo, hi]` by `field OP const`. Saturating so an extreme constant cannot
/// overflow the i64 bound.
pub(crate) fn apply_cmp_bound(op: ast::BinOp, c: i64, lo: &mut i64, hi: &mut i64) {
    use ast::BinOp::*;
    match op {
        Lt => *hi = (*hi).min(c.saturating_sub(1)),
        Le => *hi = (*hi).min(c),
        Gt => *lo = (*lo).max(c.saturating_add(1)),
        Ge => *lo = (*lo).max(c),
        Eq => {
            *lo = (*lo).max(c);
            *hi = (*hi).min(c);
        }
        _ => {}
    }
}

impl Elaborator<'_> {
    /// N7-REST: build the per-class `rand`-field bounds sidecar (`[class_id]` →
    /// `[(field_id, width, signed, lo, hi, ranged)]`). For each class, walk the
    /// base chain to gather inherited rand-field names + constraints, then fold each
    /// constraint to per-field `[lo, hi]` bounds (IEEE 1800 §18). A `rand` field with
    /// no constraint draws over its full type range.
    pub(crate) fn class_rand_table(&mut self) -> Vec<Vec<RandBound>> {
        let names = self.class_order.clone();
        names.iter().map(|n| self.class_rand_for(n)).collect()
    }

    pub(crate) fn class_rand_for(&mut self, name: &str) -> Vec<RandBound> {
        // Gather inherited rand-field names + constraints (self→base→…→root).
        let mut rand_names: std::collections::BTreeSet<String> = Default::default();
        let mut constraints: Vec<ast::ConstraintDecl> = Vec::new();
        let mut cur = Some(name.to_string());
        let mut guard = 0;
        while let Some(n) = cur {
            let Some(ci) = self.class_table.get(&n) else {
                break;
            };
            rand_names.extend(ci.rand_fields.iter().cloned());
            constraints.extend(ci.constraints.iter().cloned());
            cur = ci.base.clone();
            guard += 1;
            if guard > 256 {
                break;
            }
        }
        if rand_names.is_empty() {
            return Vec::new();
        }
        let Some(fields) = self.class_table.get(name).map(|ci| ci.fields.clone()) else {
            return Vec::new();
        };
        // Per-rand-field bounds (name, field_id, width, signed, lo, hi), full range.
        let mut bounds: Vec<(String, u32, u32, bool, i64, i64)> = Vec::new();
        for (idx, f) in fields.iter().enumerate() {
            if !rand_names.contains(&f.name) {
                continue;
            }
            if f.class_type.is_some() {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a `rand` class-handle member is outside Phase B1 (integral only)",
                );
                continue;
            }
            let (lo, hi) = rand_type_range(f.width, f.signed);
            bounds.push((f.name.clone(), idx as u32, f.width, f.signed, lo, hi));
        }
        for c in &constraints.clone() {
            for (i, e) in c.exprs.iter().enumerate() {
                // SOFT constraints must NOT narrow the [lo,hi] domain — they have to
                // stay droppable predicates (a hard domain can't be dropped).
                if c.soft.get(i).copied().unwrap_or(false) {
                    continue;
                }
                self.apply_constraint_expr(e, &mut bounds);
            }
        }
        let mut tab = Vec::new();
        for (fname, idx, width, signed, lo, hi) in bounds {
            if lo > hi {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "contradictory constraint on rand field `{fname}` (empty solution set)"
                    ),
                );
                continue;
            }
            // `constrained` ⇒ at least one constraint narrowed the field below its
            // full type range, so the engine MUST draw within [lo, hi] (regardless of
            // width — a wide field with small bounds, or large i64 bounds, both honor
            // the range). An unconstrained field keeps the full range and is drawn
            // full-width. (Earlier this gated on `fits-i32`, which SILENTLY dropped
            // the constraint for wide/large-bound fields — a silent-wrong.)
            let constrained = (lo, hi) != rand_type_range(width, signed);
            tab.push((idx, width, signed, lo, hi, constrained));
        }
        tab
    }

    /// Best-effort: narrow a rand field's `[lo,hi]` SAMPLING DOMAIN for the
    /// single-field range forms `FIELD </<=/>/>=/== CONST` (and top-level `&&`).
    /// Any OTHER form (inter-variable `x<y`, inside, implication, arithmetic, …)
    /// is a NO-OP here — it is enforced EXACTLY by the compiled predicate
    /// (`class_constraints_for`), which is the loud authority. So this only ever
    /// shrinks the rejection-sampling domain; it never errors.
    pub(crate) fn apply_constraint_expr(
        &mut self,
        e: &ast::Expr,
        bounds: &mut [(String, u32, u32, bool, i64, i64)],
    ) {
        use ast::{BinOp, ExprKind};
        match &e.kind {
            ExprKind::Binary {
                op: BinOp::LogAnd,
                lhs,
                rhs,
            } => {
                self.apply_constraint_expr(lhs, bounds);
                self.apply_constraint_expr(rhs, bounds);
            }
            ExprKind::Binary { op, lhs, rhs }
                if matches!(
                    op,
                    BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq
                ) =>
            {
                let narrow = if let Some(fname) = rand_field_ident(lhs) {
                    self.const_eval_in_scope(rhs).map(|cv| (fname, cv, *op))
                } else if let Some(fname) = rand_field_ident(rhs) {
                    self.const_eval_in_scope(lhs)
                        .map(|cv| (fname, cv, flip_cmp(*op)))
                } else {
                    None
                };
                if let Some((fname, cval, op)) = narrow {
                    if let Some(b) = bounds.iter_mut().find(|(n, ..)| *n == fname) {
                        apply_cmp_bound(op, cval, &mut b.4, &mut b.5);
                    }
                }
            }
            ExprKind::Binary {
                op: BinOp::LogOr, ..
            } => {
                // A top-level OR over a SINGLE field (e.g. a desugared `inside
                // {1,3,[10:15]}`) narrows the field's domain to the UNION bounding
                // interval, so rejection sampling can actually reach the set (a
                // full-width random would almost never land in it). The predicate
                // still filters to the EXACT membership.
                if let Some((f, lo, hi)) = self.expr_field_interval(e) {
                    if let Some(b) = bounds.iter_mut().find(|(n, ..)| *n == f) {
                        b.4 = b.4.max(lo);
                        b.5 = b.5.min(hi);
                    }
                }
            }
            ExprKind::Paren { inner } => self.apply_constraint_expr(inner, bounds),
            _ => {}
        }
    }

    /// Phase B2: per-class constraint PREDICATES (parallel to `class_rand_table`).
    pub(crate) fn class_constraints_table(&mut self) -> Vec<Vec<Vec<sim_ir::COp>>> {
        let names = self.class_order.clone();
        names
            .iter()
            .map(|n| self.class_constraints_for(n))
            .collect()
    }

    /// Phase B2: per-class `dist` weighted distributions (parallel to the rand/
    /// constraint tables).
    pub(crate) fn class_dist_table(&mut self) -> Vec<Vec<DistField>> {
        let names = self.class_order.clone();
        names.iter().map(|n| self.class_dist_for(n)).collect()
    }

    /// Phase B2: per-class `randc` (cyclic) fields. B2 v1 cycles each over its FULL
    /// type range with NO constraints (a constraint on a randc field, or a range
    /// wider than 16 bits, is loud-rejected — the cyclic permutation must be
    /// finite/unconstrained in this subset).
    pub(crate) fn class_randc_table(&mut self) -> Vec<Vec<RandcField>> {
        let names = self.class_order.clone();
        names.iter().map(|n| self.class_randc_for(n)).collect()
    }

    pub(crate) fn class_randc_for(&mut self, name: &str) -> Vec<RandcField> {
        let mut randc_names: std::collections::BTreeSet<String> = Default::default();
        let mut constraints: Vec<ast::ConstraintDecl> = Vec::new();
        let mut cur = Some(name.to_string());
        let mut guard = 0;
        while let Some(n) = cur {
            let Some(ci) = self.class_table.get(&n) else {
                break;
            };
            randc_names.extend(ci.randc_fields.iter().cloned());
            constraints.extend(ci.constraints.iter().cloned());
            cur = ci.base.clone();
            guard += 1;
            if guard > 256 {
                break;
            }
        }
        if randc_names.is_empty() {
            return Vec::new();
        }
        let Some(fields) = self.class_table.get(name).map(|ci| ci.fields.clone()) else {
            return Vec::new();
        };
        let mut out: Vec<RandcField> = Vec::new();
        for (idx, f) in fields.iter().enumerate() {
            if !randc_names.contains(&f.name) {
                continue;
            }
            let (lo, hi) = rand_type_range(f.width, f.signed);
            if (hi as i128 - lo as i128 + 1) > 65536 {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a `randc` field wider than 16 bits is unsupported (cyclic \
                     permutation cap)",
                );
                continue;
            }
            if constraints
                .iter()
                .any(|c| c.exprs.iter().any(|e| expr_mentions_field(e, &f.name)))
            {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a constraint on a `randc` field is unsupported in B2 (a randc \
                     field cycles over its full range)",
                );
                continue;
            }
            out.push((idx as u32, lo, hi));
        }
        out
    }

    pub(crate) fn class_dist_for(&mut self, name: &str) -> Vec<DistField> {
        let mut rand_names: std::collections::BTreeSet<String> = Default::default();
        let mut constraints: Vec<ast::ConstraintDecl> = Vec::new();
        let mut cur = Some(name.to_string());
        let mut guard = 0;
        while let Some(n) = cur {
            let Some(ci) = self.class_table.get(&n) else {
                break;
            };
            rand_names.extend(ci.rand_fields.iter().cloned());
            constraints.extend(ci.constraints.iter().cloned());
            cur = ci.base.clone();
            guard += 1;
            if guard > 256 {
                break;
            }
        }
        if rand_names.is_empty() {
            return Vec::new();
        }
        let Some(fields) = self.class_table.get(name).map(|ci| ci.fields.clone()) else {
            return Vec::new();
        };
        let mut map: std::collections::HashMap<String, u32> = Default::default();
        for (idx, f) in fields.iter().enumerate() {
            if rand_names.contains(&f.name) {
                map.insert(f.name.clone(), idx as u32);
            }
        }
        let mut out: Vec<DistField> = Vec::new();
        for c in &constraints.clone() {
            for e in &c.exprs {
                if let ast::ExprKind::Dist { value, items } = &e.kind {
                    if let Some(df) = self.fold_dist(value, items, &map) {
                        out.push(df);
                    }
                }
            }
        }
        out
    }

    /// Compile every constraint expression of class `name` (inherited included) to
    /// a postfix predicate over candidate rand-field values. A top-level `&&` is
    /// split into independent predicates. Each predicate references rand fields by
    /// their field-id (matching `class_rand`); only rand fields + constants are in
    /// the B2 subset (a non-rand / unsupported reference is loud).
    pub(crate) fn class_constraints_for(&mut self, name: &str) -> Vec<Vec<sim_ir::COp>> {
        let mut rand_names: std::collections::BTreeSet<String> = Default::default();
        let mut randc_names: std::collections::BTreeSet<String> = Default::default();
        let mut constraints: Vec<ast::ConstraintDecl> = Vec::new();
        let mut cur = Some(name.to_string());
        let mut guard = 0;
        while let Some(n) = cur {
            let Some(ci) = self.class_table.get(&n) else {
                break;
            };
            rand_names.extend(ci.rand_fields.iter().cloned());
            randc_names.extend(ci.randc_fields.iter().cloned());
            constraints.extend(ci.constraints.iter().cloned());
            cur = ci.base.clone();
            guard += 1;
            if guard > 256 {
                break;
            }
        }
        if rand_names.is_empty() {
            return Vec::new();
        }
        let Some(fields) = self.class_table.get(name).map(|ci| ci.fields.clone()) else {
            return Vec::new();
        };
        // rand field name → (field-id, width, signed). The field-id matches
        // `class_rand`'s `RandBound.0`; width/signed gate the i64 predicate lane.
        let mut map: std::collections::HashMap<String, (u32, u32, bool)> = Default::default();
        for (idx, f) in fields.iter().enumerate() {
            if rand_names.contains(&f.name) {
                map.insert(f.name.clone(), (idx as u32, f.width, f.signed));
            }
        }
        // PLAIN rand fields (range constraints on these ARE captured by [lo,hi]):
        // exclude `randc` (cyclic draw ignores the domain) and `dist` (weighted draw
        // ignores it). A single-field range on a non-plain or unknown field stays a
        // predicate (→ enforced after the draw, or loud-rejected if the name is bad).
        let dist_names = dist_field_names(&constraints);
        let plain_rand: std::collections::BTreeSet<String> = rand_names
            .iter()
            .filter(|n| !randc_names.contains(*n) && !dist_names.contains(*n))
            .cloned()
            .collect();
        let mut preds: Vec<Vec<sim_ir::COp>> = Vec::new();
        for c in &constraints.clone() {
            for (i, e) in c.exprs.iter().enumerate() {
                let is_soft = c.soft.get(i).copied().unwrap_or(false);
                self.collect_constraint_preds(e, &map, &plain_rand, is_soft, &mut preds);
            }
        }
        preds
    }

    /// Split top-level `&&` into independent predicates; compile each leaf. A pure
    /// single-field range/equality (`x </<=/>/>=/== const`) is SKIPPED — it is
    /// captured exactly by the `[lo,hi]` sampling domain (`apply_constraint_expr`),
    /// so it needs no predicate (and skipping it keeps a WIDE-field range constraint
    /// off the i64 predicate lane, which only handles ≤64-bit signed / ≤63-bit
    /// unsigned).
    pub(crate) fn collect_constraint_preds(
        &mut self,
        e: &ast::Expr,
        map: &std::collections::HashMap<String, (u32, u32, bool)>,
        plain_rand: &std::collections::BTreeSet<String>,
        is_soft: bool,
        preds: &mut Vec<Vec<sim_ir::COp>>,
    ) {
        // A `dist` constraint is a weighted SAMPLER, not a boolean predicate — it
        // rides the `class_dist` sidecar, so it contributes no predicate here.
        if matches!(e.kind, ast::ExprKind::Dist { .. }) {
            return;
        }
        if let ast::ExprKind::Binary {
            op: ast::BinOp::LogAnd,
            lhs,
            rhs,
        } = &e.kind
        {
            self.collect_constraint_preds(lhs, map, plain_rand, is_soft, preds);
            self.collect_constraint_preds(rhs, map, plain_rand, is_soft, preds);
            return;
        }
        if let ast::ExprKind::Paren { inner } = &e.kind {
            self.collect_constraint_preds(inner, map, plain_rand, is_soft, preds);
            return;
        }
        // A HARD single-field range is captured by the [lo,hi] sampling domain — but
        // ONLY when the field is a PLAIN rand field (the domain is the authority).
        // If the field is UNKNOWN (typo / not a member), `dist` (domain ignored by
        // the weighted draw), or `randc` (cyclic, domain ignored), the range MUST
        // stay a predicate so `compile_constraint_pred` either loud-rejects the
        // unknown name (E3009, symmetric with the `!=` path) or enforces the range
        // after the draw. A SOFT range also stays a predicate (so it can be dropped).
        if !is_soft {
            if let Some(f) = self.single_range_field(e) {
                if plain_rand.contains(&f) {
                    return;
                }
            }
        }
        let mut prog: Vec<sim_ir::COp> = Vec::new();
        if is_soft {
            prog.push(sim_ir::COp::SoftMarker);
        }
        if self.compile_constraint_pred(e, map, &mut prog) {
            preds.push(prog);
        }
    }

    /// Post-order compile a constraint expression to a postfix predicate. Returns
    /// false (after a loud diagnostic) on a form outside the B2 subset.
    pub(crate) fn compile_constraint_pred(
        &mut self,
        e: &ast::Expr,
        map: &std::collections::HashMap<String, (u32, u32, bool)>,
        out: &mut Vec<sim_ir::COp>,
    ) -> bool {
        use ast::{ExprKind, UnOp};
        match &e.kind {
            ExprKind::Paren { inner } => self.compile_constraint_pred(inner, map, out),
            ExprKind::Ident(path) if path.segments.len() == 1 => {
                if let Some(&(idx, width, signed)) = map.get(&path.segments[0].name) {
                    // The predicate lane evaluates in i64: a value fits iff the field
                    // is ≤63 bits (any sign), or exactly 64-bit SIGNED. A >64-bit field
                    // or a ≥64-bit UNSIGNED field cannot be faithfully compared in i64
                    // (it would truncate / mis-sign), so reject it loudly here rather
                    // than silently accept out-of-constraint draws. (Pure single-field
                    // range constraints on such fields are still honored via [lo,hi].)
                    if width > 64 || (width >= 64 && !signed) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "a general (non-range) constraint on rand field `{}` is \
                                 unsupported for its width/signedness (B2 predicates \
                                 evaluate in i64: ≤63-bit, or 64-bit signed)",
                                path.segments[0].name
                            ),
                        );
                        return false;
                    }
                    out.push(sim_ir::COp::Field(idx));
                    return true;
                }
                if let Some(v) = self.const_eval_in_scope(e) {
                    out.push(sim_ir::COp::Const(v));
                    return true;
                }
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "constraint references `{}` — only rand fields and constants are \
                         supported (B2; a non-rand class field is a follow-on)",
                        path.segments[0].name
                    ),
                );
                false
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let Some(cop) = map_cbinop(*op) else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "unsupported operator in a constraint expression (B2)",
                    );
                    return false;
                };
                if !self.compile_constraint_pred(lhs, map, out) {
                    return false;
                }
                if !self.compile_constraint_pred(rhs, map, out) {
                    return false;
                }
                out.push(sim_ir::COp::Bin(cop));
                true
            }
            ExprKind::Unary {
                op: UnOp::LogNot,
                operand,
            } => {
                if !self.compile_constraint_pred(operand, map, out) {
                    return false;
                }
                out.push(sim_ir::COp::Not);
                true
            }
            ExprKind::Unary {
                op: UnOp::Minus,
                operand,
            } => {
                out.push(sim_ir::COp::Const(0));
                if !self.compile_constraint_pred(operand, map, out) {
                    return false;
                }
                out.push(sim_ir::COp::Bin(sim_ir::CBinOp::Sub));
                true
            }
            _ => {
                // A whole-subexpr constant (e.g. `P*2`) folds to one Const.
                if let Some(v) = self.const_eval_in_scope(e) {
                    out.push(sim_ir::COp::Const(v));
                    return true;
                }
                self.error(
                    MsgCode::ElabUnsupported,
                    "unsupported constraint expression form (B2: relational/arithmetic/\
                     logical over rand fields and constants)",
                );
                false
            }
        }
    }

    /// N7-REST B-CRV final: fold one inline `randomize() with {…}` block into a
    /// per-call sidecar entry and return its with-id (the index the engine reads
    /// from a Const arg). Mirrors the class-constraint fold: single-field ranges
    /// narrow per-field `[lo,hi]` DOMAINS (`apply_constraint_expr`), everything
    /// else compiles to PREDICATES (`collect_constraint_preds`). The field map is
    /// the RECEIVER's flattened layout (base fields first), so the indices match
    /// the heap object's `class_rand`/`class_constraints`. The engine INTERSECTS
    /// these domains with the class domains and ANDs the predicates (§18.7).
    pub(crate) fn fold_inline_with(&mut self, class_name: &str, constraints: &[ast::Expr]) -> u32 {
        let id = self.randomize_with.len() as u32;
        // Inherited rand-field names + class constraints + randc names (self→base→…
        // →root), like `class_rand_for`/`class_constraints_for`.
        let mut rand_names: std::collections::BTreeSet<String> = Default::default();
        let mut randc_names: std::collections::BTreeSet<String> = Default::default();
        let mut class_constraints: Vec<ast::ConstraintDecl> = Vec::new();
        let mut cur = Some(class_name.to_string());
        let mut guard = 0;
        while let Some(n) = cur {
            let Some(ci) = self.class_table.get(&n) else {
                break;
            };
            rand_names.extend(ci.rand_fields.iter().cloned());
            randc_names.extend(ci.randc_fields.iter().cloned());
            class_constraints.extend(ci.constraints.iter().cloned());
            cur = ci.base.clone();
            guard += 1;
            if guard > 256 {
                break;
            }
        }
        // §18.7 + B2: an inline constraint that references a `randc` field is
        // unsupported — same loud-reject as a CLASS constraint on a randc field
        // (`class_randc_for`). A randc field is drawn cyclically OUTSIDE the solver,
        // so an inline predicate/range on it would be silently ignored (the field
        // would draw its full range / be read as a stale 0). Loud, never silent.
        let mentions_randc = constraints
            .iter()
            .any(|e| randc_names.iter().any(|rn| expr_mentions_field(e, rn)));
        if mentions_randc {
            self.error(
                MsgCode::ElabUnsupported,
                "an inline `with` constraint referencing a `randc` field is unsupported \
                 in B2 (a randc field cycles over its full range, outside the solver)",
            );
        }
        let fields = self
            .class_table
            .get(class_name)
            .map(|ci| ci.fields.clone())
            .unwrap_or_default();
        let mut map: std::collections::HashMap<String, (u32, u32, bool)> = Default::default();
        let mut bounds: Vec<(String, u32, u32, bool, i64, i64)> = Vec::new();
        for (idx, f) in fields.iter().enumerate() {
            if rand_names.contains(&f.name) {
                map.insert(f.name.clone(), (idx as u32, f.width, f.signed));
                let (lo, hi) = rand_type_range(f.width, f.signed);
                bounds.push((f.name.clone(), idx as u32, f.width, f.signed, lo, hi));
            }
        }
        // PLAIN rand fields (range constraints captured by [lo,hi]): exclude randc +
        // dist (the dist may come from a CLASS constraint, e.g. `x dist {…}` in the
        // class with an inline `x < 100` — that inline range must stay a predicate so
        // it actually excludes the out-of-range dist value).
        let dist_names = dist_field_names(&class_constraints);
        let plain_rand: std::collections::BTreeSet<String> = rand_names
            .iter()
            .filter(|n| !randc_names.contains(*n) && !dist_names.contains(*n))
            .cloned()
            .collect();
        // Single-field ranges on PLAIN fields → per-field domain narrowing (engine
        // intersects with the class domain; a contradictory `[lo>hi]` is caught as
        // infeasible BEFORE drawing). A range on a dist field does NOT narrow the
        // (ignored) domain — it rides the predicate path below.
        for e in constraints {
            self.apply_constraint_expr(e, &mut bounds);
        }
        let mut domains: Vec<(u32, i64, i64)> = Vec::new();
        for (fname, idx, width, signed, lo, hi) in &bounds {
            if plain_rand.contains(fname) && (*lo, *hi) != rand_type_range(*width, *signed) {
                domains.push((*idx, *lo, *hi));
            }
        }
        // Everything else → predicates (inter-variable, !=, inside, implication,
        // dist-field ranges, and unknown-name ranges → loud E3009 in compile).
        let mut preds: Vec<Vec<sim_ir::COp>> = Vec::new();
        for e in constraints {
            self.collect_constraint_preds(e, &map, &plain_rand, false, &mut preds);
        }
        self.randomize_with.push((domains, preds));
        id
    }

    /// N7-REST: intercept `obj.randomize()`. Emits a `ClassRandomize` SysTask that
    /// (at run time) draws the object's `rand` fields per the folded constraint
    /// bounds; if `result` is given, also assigns 1 (success) to it. Returns true
    /// iff this was a randomize() on a class handle we own. `randomize()` takes no
    /// args in Phase B1 (no inline `with {…}` constraint).
    pub(crate) fn try_emit_randomize(
        &mut self,
        b: &mut ProcessBuilder,
        name: &ast::HierPath,
        args: &[ast::Expr],
        constraints: Option<&[ast::Expr]>,
        result: Option<&ast::Lvalue>,
    ) -> bool {
        if name.segments.len() != 2 || name.segments[1].name != "randomize" {
            return false;
        }
        let Some(net) = self.lookup_net_scoped(&name.segments[0].name) else {
            return false;
        };
        if !self.net_class.contains_key(&net) {
            return false; // not a class handle → not our randomize
        }
        if !args.is_empty() {
            self.error(
                MsgCode::ElabUnsupported,
                "randomize() takes no positional arguments (use inline `with {…}` for \
                 per-call constraints)",
            );
            return true;
        }
        // N7-REST B-CRV final: fold any inline `with {…}` constraints into a
        // per-call sidecar entry; the with-id rides a Const arg the engine reads.
        let with_id = constraints.map(|cs| {
            let class_name = self.net_class.get(&net).cloned().unwrap_or_default();
            self.fold_inline_with(&class_name, cs)
        });
        let handle = self.push_expr(ir::Expr::Signal { net, word: None });
        // When the call captures a result (`r = obj.randomize()`), allocate a 32-bit
        // status net and pass it as args[1]; the engine writes the §18.11 verdict
        // (1=success, 0=fail) there, and the result lvalue is assigned FROM it (not a
        // hardcoded 1 — so an unsatisfiable / null randomize reports 0 correctly).
        let mut task_args = vec![handle];
        let status_net = result.map(|_| {
            let sn = self.nets.len() as u32;
            let sname = format!("__rand_status_{sn}");
            self.add_net(
                &sname,
                ir::NetVar {
                    kind: ir::NetKind::Reg,
                    width: 32,
                    msb: 31,
                    lsb: 0,
                    signed: false,
                    array_len: 1,
                    dir: ir::PortDir::Internal,
                    init: default_init(ast::NetVarKind::Reg, 32),
                },
            );
            sn
        });
        if let Some(sn) = status_net {
            let status_sig = self.push_expr(ir::Expr::Signal {
                net: sn,
                word: None,
            });
            task_args.push(status_sig);
        }
        // The inline-`with` id is a Const arg (the engine distinguishes it from the
        // status Signal by expr kind). Absent for a plain randomize() → no extra
        // arg → byte-identical to B1/B2.
        if let Some(wid) = with_id {
            let wid_e = self.const_u32_expr(wid, 32);
            task_args.push(wid_e);
        }
        let sid = self.push_stmt(ir::Stmt::SysTask {
            which: ir::SysTaskId::ClassRandomize,
            fmt: None,
            args: task_args,
        });
        b.push_stmt_id(sid);
        if let (Some(lv), Some(sn)) = (result, status_net) {
            let status_rd = self.push_expr(ir::Expr::Signal {
                net: sn,
                word: None,
            });
            let lvv = self.lower_lvalue(lv);
            self.check_lvalue_kind(&lvv, true);
            let sid2 = self.push_stmt(ir::Stmt::BlockingAssign {
                lhs: lvv,
                rhs: status_rd,
            });
            b.push_stmt_id(sid2);
        }
        true
    }

    /// v7: `x = $random(seed)` special form. The seed must lower to a plain
    /// whole-net Signal (an integral VARIABLE — IEEE 1364 §17.9.1); the rhs
    /// becomes `SysFunc{Random,[seed]}` which the engine intercepts
    /// statement-level (`StmtEffect::SeededRandom` — seed written back in the
    /// WRITE phase). Returns false when the rhs is not a seeded $random.
    /// An intra-assignment delay keeps draw-now/write-later semantics by
    /// riding the SAME desugar as a plain blocking (`tmp = draw; #d; lhs=tmp`)
    /// — the seed updates at the DRAW, like iverilog.
    pub(crate) fn random_seeded_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: Option<&ast::Lvalue>,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        let ast::ExprKind::SysCall { name, args } = &rhs.kind else {
            return false;
        };
        if name.name != "$random" || args.is_empty() {
            return false;
        }
        if args.len() > 1 {
            self.error(MsgCode::ElabUnsupported, "$random takes at most one seed");
            return true;
        }
        let seed_id = self.lower_expr(&args[0]);
        if !matches!(
            self.exprs.get(seed_id as usize),
            Some(ir::Expr::Signal { word: None, .. })
        ) {
            self.error(
                MsgCode::ElabUnsupported,
                "$random seed must be a plain integral variable (v7)",
            );
            return true;
        }
        let rhs_id = self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::Random,
            args: vec![seed_id],
        });
        match lhs {
            Some(lhs) => self.emit_blocking_intercept(b, lhs, delay, rhs_id),
            // Bare statement `$random(seed);` (return discarded): the seed WRITEBACK
            // still fires — `StmtEffect::SeededRandom` is rhs-based (`k_random_seeded_
            // rhs`), so a throwaway assign of the draw advances the seed exactly as
            // iverilog does. Without this the bare form hit `lower_systask` (W3056
            // skip) and silently left the seed unchanged, so a later `$random(seed)`
            // diverged. §4.5.123 sibling; a bare statement has no intra-assign delay.
            None => self.emit_discarded_call(b, rhs_id),
        }
        true
    }

    /// v9 rank 6: `x = $dist_uniform(seed, start, end)` — the seeded-`$random`
    /// family (the engine advances `seed` in the WRITE phase). The seed must
    /// lower to a plain whole-net Signal; `start`/`end` are any expressions.
    pub(crate) fn dist_uniform_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: Option<&ast::Lvalue>,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        let ast::ExprKind::SysCall { name, args } = &rhs.kind else {
            return false;
        };
        // v9 `$dist_uniform` + v19 the non-uniform `$dist_*` siblings: each
        // advances the ref seed VAR (a statement-level intercept, like
        // `$random(seed)`) and returns an int. Name → (id, total arity incl. seed).
        let (which, arity): (ir::SysFuncId, usize) = match name.name.as_str() {
            "$dist_uniform" => (ir::SysFuncId::DistUniform, 3),
            "$dist_normal" => (ir::SysFuncId::DistNormal, 3),
            "$dist_exponential" => (ir::SysFuncId::DistExponential, 2),
            "$dist_poisson" => (ir::SysFuncId::DistPoisson, 2),
            "$dist_chi_square" => (ir::SysFuncId::DistChiSquare, 2),
            "$dist_t" => (ir::SysFuncId::DistT, 2),
            "$dist_erlang" => (ir::SysFuncId::DistErlang, 3),
            _ => return false,
        };
        if args.len() != arity {
            self.error(
                MsgCode::ElabUnsupported,
                "a $dist_* function takes (seed, …) with the distribution's fixed arity",
            );
            return true;
        }
        let seed_id = self.lower_expr(&args[0]);
        let seed_net = match self.exprs.get(seed_id as usize) {
            Some(ir::Expr::Signal { net, word: None }) => Some(*net),
            _ => None,
        };
        let Some(seed_net) = seed_net else {
            self.error(
                MsgCode::ElabUnsupported,
                "a $dist_* seed must be a plain integral variable (v9)",
            );
            return true;
        };
        // iverilog rejects a seed narrower than 32 bits: the 32-bit Annex LCG
        // state would be truncated on write-back, silently corrupting the
        // sequence. Match it with a loud E3009 (review M1).
        if self.nets[seed_net as usize].width < 32 {
            self.error(
                MsgCode::ElabUnsupported,
                "a $dist_* seed variable must be at least 32 bits \
                 (a narrower seed would truncate the RNG state)",
            );
            return true;
        }
        let mut ids = vec![seed_id];
        for a in &args[1..] {
            ids.push(self.lower_expr(a));
        }
        let rhs_id = self.push_expr(ir::Expr::SysFunc { which, args: ids });
        match lhs {
            Some(lhs) => self.emit_blocking_intercept(b, lhs, delay, rhs_id),
            // Bare statement `$dist_*(seed, …);` (return discarded): the seed
            // WRITEBACK still fires — `StmtEffect::SeededDist` is rhs-based
            // (`k_dist_seeded_rhs`), so a throwaway assign advances the seed exactly
            // as iverilog does. Without this the bare form hit `lower_systask` (W3056
            // skip), silently leaving the seed unchanged. §4.5.125 sibling.
            None => self.emit_discarded_call(b, rhs_id),
        }
        true
    }
}
