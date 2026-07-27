//! struct-of-arrays rewrites — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// N3 SoA: transpose a record-array decl-init `'{ '{a,b}, '{c,d} }` into per-FIELD
    /// unpacked-array inits — field 0 → `'{a,c}`, field 1 → `'{b,d}`. Each field's dyn
    /// array then inits independently via the existing dyn-array `'{…}` decl-init flush.
    /// `None` when the init is not the `'{ '{…},… }` shape with exactly `nfields`-arg
    /// elements (the caller emits a loud error — never a silent partial init).
    pub(crate) fn soa_field_inits(&self, nfields: usize, init: &Expr) -> Option<Vec<Expr>> {
        let ExprKind::AssignPattern(elems) = &init.kind else {
            return None;
        };
        let mut per_field: Vec<Vec<Expr>> = vec![Vec::new(); nfields];
        for el in elems {
            let ExprKind::AssignPattern(args) = &el.kind else {
                return None; // an element that is not an inner `'{…}` record pattern
            };
            if args.len() != nfields {
                return None; // wrong field count
            }
            for (j, a) in args.iter().enumerate() {
                per_field[j].push(a.clone());
            }
        }
        Some(
            per_field
                .into_iter()
                .map(|args| Expr {
                    kind: ExprKind::AssignPattern(args),
                    span: init.span,
                })
                .collect(),
        )
    }

    /// N3 SoA: if `var` is a SoA record array and `field` is a valid member, return the
    /// member's own dyn-array net name `$unp$var$field`; else `None`. Drives the
    /// `arr[i].field` → `$unp$arr$field[i]` rewrite (a native, correctly-typed dyn
    /// element access — no packed offset / `$signed` / dbase machinery needed).
    pub(crate) fn soa_member_field(&self, var: &str, field: &str) -> Option<String> {
        let ty = self.record_soa_vars.get(var)?;
        let members = self.unpacked_struct_layouts.get(ty)?;
        members
            .iter()
            .any(|m| m.name.name == field)
            .then(|| Self::unpacked_member_net(var, field))
    }

    /// N3 SoA: rewrite the receiver of a whole-array method call `arr.size()` /
    /// `arr.delete()` on a SoA record array to field 0's dyn array
    /// (`$unp$arr$field0.size()`) — every field net has equal length, so any field
    /// answers `.size()`/`.num()`; `.delete()` on field 0 is not enough on its own,
    /// so a bare `arr.delete()` stays loud (handled as a whole-array op elsewhere).
    /// A non-SoA path (or ≠2 segments) is returned unchanged.
    pub(crate) fn soa_rewrite_method_recv(&self, path: HierPath) -> HierPath {
        // Only READ-ONLY length queries (`size`/`num`) map to field 0 — every field net
        // has equal length. A MUTATING method (`push_back`/`pop_*`/`insert`/`delete`) must
        // NOT collapse to field 0 (it would desync the fields); those fan out per field at
        // the statement / assignment site (`try_soa_queue_method_stmt`, `try_soa_assign`),
        // so leave the receiver `arr` alone here (it reaches those paths intact).
        if path.segments.len() == 2 && matches!(path.segments[1].name.as_str(), "size" | "num") {
            if let Some(ty) = self.record_soa_vars.get(&path.segments[0].name) {
                if let Some(m0) = self
                    .unpacked_struct_layouts
                    .get(ty)
                    .and_then(|ms| ms.first())
                {
                    let mnet = Self::unpacked_member_net(&path.segments[0].name, &m0.name.name);
                    let mut segs = path.segments;
                    segs[0].name = mnet;
                    return HierPath {
                        segments: segs,
                        span: path.span,
                    };
                }
            }
        }
        path
    }

    /// r18 (Fix A): the per-field values for a record actual passed to a SoA queue
    /// `push_back`/`push_front`/`insert`. The args have ALREADY been struct-expanded
    /// (`expand_struct_call_args`), so a record var `p` is its `nfields` per-member nets
    /// in field order; a `'{…}` pattern actual stays a single arg whose `nfields` elements
    /// are the per-field values. `None` (with a loud error) on any other shape — never a
    /// silent partial push that would desync the field queues.
    pub(crate) fn soa_record_arg_values(
        &mut self,
        args: &[Expr],
        nfields: usize,
        span: Span,
    ) -> Option<Vec<Expr>> {
        if args.len() == nfields {
            return Some(args.to_vec());
        }
        if args.len() == 1 {
            if let ExprKind::AssignPattern(elems) = &args[0].kind {
                if elems.len() == nfields {
                    return Some(elems.clone());
                }
            }
        }
        self.error_at(
            span,
            "a record-queue push/insert value must be a same-type record variable or a \
             `'{…}` pattern listing every field",
        );
        None
    }

    /// r18 (Fix A): fan out a queue METHOD statement on a SoA record queue
    /// (`q.push_back(p)`, `q.push_front(p)`, `q.insert(i,p)`, `q.delete()`/`q.delete(i)`)
    /// into per-field native queue ops. `path` = `[q, method]`; `args` are already
    /// struct-expanded. `None` for a non-SoA-queue receiver or an unhandled method → the
    /// caller builds the generic call (loud in elaborate). All-or-loud: a shape mismatch
    /// emits a loud error, never a silent partial fan-out that desyncs the field queues.
    pub(crate) fn try_soa_queue_method_stmt(
        &mut self,
        path: &HierPath,
        args: &[Expr],
        span: Span,
    ) -> Option<Stmt> {
        if path.segments.len() != 2 {
            return None;
        }
        let var = path.segments[0].name.clone();
        let ty = self.record_soa_vars.get(&var)?.clone();
        let members = self.unpacked_struct_layouts.get(&ty)?.clone();
        let method = path.segments[1].name.clone();
        let nf = members.len();
        let call = |field: &str, margs: Vec<Expr>| Stmt::UserTaskCall {
            name: HierPath {
                segments: vec![
                    Ident {
                        name: Self::unpacked_member_net(&var, field),
                        span,
                    },
                    Ident {
                        name: method.clone(),
                        span,
                    },
                ],
                span,
            },
            args: margs,
            span,
        };
        let stmts: Vec<Stmt> = match method.as_str() {
            "push_back" | "push_front" => {
                let vals = self.soa_record_arg_values(args, nf, span)?;
                members
                    .iter()
                    .zip(vals)
                    .map(|(m, v)| call(&m.name.name, vec![v]))
                    .collect()
            }
            "insert" => {
                // `q.insert(i, p)` → args = [index, field0, …] (index not expanded).
                let vals = self.soa_record_arg_values(&args[1.min(args.len())..], nf, span)?;
                let idx = args.first().cloned()?;
                members
                    .iter()
                    .zip(vals)
                    .map(|(m, v)| call(&m.name.name, vec![idx.clone(), v]))
                    .collect()
            }
            "delete" => members
                .iter()
                .map(|m| call(&m.name.name, args.to_vec()))
                .collect(),
            _ => return None,
        };
        Some(Stmt::Block {
            label: None,
            decls: Vec::new(),
            stmts,
            span,
        })
    }

    /// N3 SoA: expand a whole-array / element assignment of a SoA record array into a
    /// `Block` of per-field NATIVE dyn ops. Handles `arr = new[N]` / `new[N](src)`,
    /// `arr = other` (same-type SoA copy), `arr[i] = '{…}` (element pattern write), and
    /// (r18) `rec = q.pop_front()`/`pop_back()` (per-field pop into a record var).
    /// Returns `None` for any other shape → the caller builds the ordinary assignment,
    /// which resolves the virtual `arr` to no net (loud) — never a silent partial write.
    pub(crate) fn try_soa_assign(
        &mut self,
        lhs: &Lvalue,
        rhs: &Expr,
        blocking: bool,
        span: Span,
    ) -> Option<Stmt> {
        // r18 (Fix A): `rec = q.pop_front()`/`pop_back()` — pop a record off a SoA queue
        // into a scalar record var, per field (`$unp$rec$f = $unp$q$f.pop_*()`). The lhs is
        // a same-type per-member record var (`var_unpacked_struct`); the rhs pops on a SoA
        // record queue. Placed first because the whole-array branch below early-returns
        // `None` when the lhs is not itself a SoA array.
        if let (Lvalue::Ident(lp), ExprKind::Call { name, args }) = (lhs, &rhs.kind) {
            if lp.segments.len() == 1
                && name.segments.len() == 2
                && args.is_empty()
                && matches!(name.segments[1].name.as_str(), "pop_front" | "pop_back")
            {
                let lvar = lp.segments[0].name.clone();
                let qvar = name.segments[0].name.clone();
                if let Some(qty) = self.record_soa_vars.get(&qvar).cloned() {
                    // The destination must be a same-type record var.
                    if self.var_unpacked_struct.get(&lvar) == Some(&qty) {
                        let members = self.unpacked_struct_layouts.get(&qty)?.clone();
                        let pop = name.segments[1].name.clone();
                        let stmts = members
                            .iter()
                            .map(|m| {
                                let dst = Self::unpacked_member_net(&lvar, &m.name.name);
                                let srcq = Self::unpacked_member_net(&qvar, &m.name.name);
                                let pop_rhs = Expr {
                                    kind: ExprKind::Call {
                                        name: HierPath {
                                            segments: vec![
                                                Ident { name: srcq, span },
                                                Ident {
                                                    name: pop.clone(),
                                                    span,
                                                },
                                            ],
                                            span,
                                        },
                                        args: Vec::new(),
                                    },
                                    span,
                                };
                                Self::assign_stmt(
                                    Self::ident_lval(&dst, span),
                                    pop_rhs,
                                    blocking,
                                    span,
                                )
                            })
                            .collect();
                        return Some(Stmt::Block {
                            label: None,
                            decls: Vec::new(),
                            stmts,
                            span,
                        });
                    }
                }
            }
        }
        // T1-11: whole-ELEMENT read `rec = q[i]` / `rec = arr[i]` — a SoA record
        // container element copied into a record var, per field
        // (`$unp$rec$f = $unp$q$f[i]`).
        //
        // The per-FIELD element access (`q[0].len`) and the whole-element read on a
        // PACKABLE record queue both already worked; only the non-packable (SoA) form had
        // no surface, because a SoA container has no net under its own name at all — the
        // bare `q[i]` resolved to "undeclared net/variable `q`" downstream.
        //
        // Same-TYPE gate, identical in force to the scalar whole-copy below: same declared
        // type ⇒ the two member lists came from one `unpacked_struct_layouts` entry, so
        // the per-field correspondence is by construction rather than by position.
        //
        // The index expression is CLONED per field, so it is evaluated once per member.
        // That is sound only because vita's subset cannot express a side-effecting index
        // — a function that writes anything outside itself is loud at the frame gate, and
        // `q[k++]` does not parse. The pre-existing `arr[i] = '{…}` fan-out below rests on
        // the same invariant; if the frame subset ever widens, BOTH need a replay guard.
        if let (Lvalue::Ident(lp), ExprKind::BitSelect { base, index }) = (lhs, &rhs.kind) {
            if let (1, ExprKind::Ident(bp)) = (lp.segments.len(), &base.kind) {
                if bp.segments.len() == 1 {
                    let lvar = lp.segments[0].name.clone();
                    let qvar = bp.segments[0].name.clone();
                    if let Some(qty) = self.record_soa_vars.get(&qvar).cloned() {
                        if self.var_unpacked_struct.get(&lvar) == Some(&qty) {
                            let members = self.unpacked_struct_layouts.get(&qty)?.clone();
                            let stmts = members
                                .iter()
                                .map(|m| {
                                    let dst = Self::unpacked_member_net(&lvar, &m.name.name);
                                    let srcq = Self::unpacked_member_net(&qvar, &m.name.name);
                                    let elem = Expr {
                                        kind: ExprKind::BitSelect {
                                            base: Box::new(Self::ident_expr(&srcq, span)),
                                            index: index.clone(),
                                        },
                                        span,
                                    };
                                    Self::assign_stmt(
                                        Self::ident_lval(&dst, span),
                                        elem,
                                        blocking,
                                        span,
                                    )
                                })
                                .collect();
                            return Some(Stmt::Block {
                                label: None,
                                decls: Vec::new(),
                                stmts,
                                span,
                            });
                        }
                    }
                }
            }
        }
        // Q (round-19): scalar-record whole-copy `a = b` / `a <= b` where BOTH `a`
        // and `b` are scalar `var_unpacked_struct` records of the SAME type name →
        // per-member fan-out `$unp$a$f <op> $unp$b$f` for every field. The record
        // net, queue, `size()`/`pop_front()`, and field access already work via the
        // per-member SoA representation (`parse_unpacked_struct_decl`); only the
        // whole-value COPY had no surface — a scalar non-packable record has no
        // flat net of its own, so a bare `a = b` would otherwise resolve `a` to
        // "undeclared" downstream.
        //
        // Correctness gate: SAME type name is the whole guarantee. Same declared
        // type ⇒ `$unp$a$f` / `$unp$b$f` were built from the IDENTICAL member list
        // (`unpacked_struct_layouts[ty]`), giving identical per-field width/sign/
        // kind by construction — including a PARAMETER-width member
        // (`[ADDR_W-1:0]`) whose raw range resolves per-instance at elaborate (this
        // path never calls `packable_record_layout`, so no parse-time frozen width
        // is baked in — safe under `#(.ADDR_W())` override). A cross-type copy is
        // refused (`None` here → falls through to the ordinary assignment, loud
        // downstream) because two DIFFERENT record types carry no guaranteed
        // per-field correspondence (different member lists / order / widths) — a
        // silent field-by-position copy would silently truncate or reorder fields.
        // All-or-loud: every member is copied, or the whole `Block` is skipped —
        // never a partial fan-out that would desync fields.
        if let (Lvalue::Ident(lp), ExprKind::Ident(rp)) = (lhs, &rhs.kind) {
            if lp.segments.len() == 1 && rp.segments.len() == 1 {
                let lvar = lp.segments[0].name.clone();
                let rvar = rp.segments[0].name.clone();
                if let Some(lty) = self.var_unpacked_struct.get(&lvar).cloned() {
                    if self.var_unpacked_struct.get(&rvar) == Some(&lty) {
                        let members = self.unpacked_struct_layouts.get(&lty)?.clone();
                        let stmts = members
                            .iter()
                            .map(|m| {
                                let dst = Self::unpacked_member_net(&lvar, &m.name.name);
                                let srcn = Self::unpacked_member_net(&rvar, &m.name.name);
                                Self::assign_stmt(
                                    Self::ident_lval(&dst, span),
                                    Self::ident_expr(&srcn, span),
                                    blocking,
                                    span,
                                )
                            })
                            .collect();
                        return Some(Stmt::Block {
                            label: None,
                            decls: Vec::new(),
                            stmts,
                            span,
                        });
                    }
                }
            }
        }
        // Whole-array: `arr = new[N]` / `arr = new[N](src)` / `arr = other`.
        if let Lvalue::Ident(p) = lhs {
            if p.segments.len() == 1 {
                let var = p.segments[0].name.clone();
                let ty = self.record_soa_vars.get(&var)?.clone();
                let members = self.unpacked_struct_layouts.get(&ty)?.clone();
                let stmts: Vec<Stmt> = match &rhs.kind {
                    ExprKind::New { size, src } => {
                        // Per-field `new[N]`; a SoA src `other` maps to `$unp$other$f`.
                        let src_var = match src.as_deref().map(|e| &e.kind) {
                            Some(ExprKind::Ident(rp)) if rp.segments.len() == 1 => {
                                Some(rp.segments[0].name.clone())
                            }
                            _ => None,
                        };
                        members
                            .iter()
                            .map(|m| {
                                let mnet = Self::unpacked_member_net(&var, &m.name.name);
                                let field_src = src_var.as_ref().and_then(|s| {
                                    self.record_soa_vars.contains_key(s).then(|| {
                                        Box::new(Self::ident_expr(
                                            &Self::unpacked_member_net(s, &m.name.name),
                                            span,
                                        ))
                                    })
                                });
                                let new_rhs = Expr {
                                    kind: ExprKind::New {
                                        size: size.clone(),
                                        src: field_src.or_else(|| src.clone()),
                                    },
                                    span,
                                };
                                Self::assign_stmt(
                                    Self::ident_lval(&mnet, span),
                                    new_rhs,
                                    blocking,
                                    span,
                                )
                            })
                            .collect()
                    }
                    ExprKind::Ident(rp)
                        if rp.segments.len() == 1
                            && self.record_soa_vars.get(&rp.segments[0].name) == Some(&ty) =>
                    {
                        // `arr = other` — same-type SoA whole-array copy (field by field).
                        let other = rp.segments[0].name.clone();
                        members
                            .iter()
                            .map(|m| {
                                let dst = Self::unpacked_member_net(&var, &m.name.name);
                                let srcn = Self::unpacked_member_net(&other, &m.name.name);
                                Self::assign_stmt(
                                    Self::ident_lval(&dst, span),
                                    Self::ident_expr(&srcn, span),
                                    blocking,
                                    span,
                                )
                            })
                            .collect()
                    }
                    _ => return None,
                };
                return Some(Stmt::Block {
                    label: None,
                    decls: Vec::new(),
                    stmts,
                    span,
                });
            }
        }
        // Element pattern write: `arr[i] = '{v0,…,vM}`.
        if let Lvalue::BitSelect { base, index, .. } = lhs {
            if let Lvalue::Ident(p) = base.as_ref() {
                if p.segments.len() == 1 {
                    let var = p.segments[0].name.clone();
                    let ty = self.record_soa_vars.get(&var)?.clone();
                    let members = self.unpacked_struct_layouts.get(&ty)?.clone();
                    // T1-11: the WRITE twin of the whole-element read above —
                    // `q[i] = rec` from a same-type record var, fanned out per field.
                    // Emitted here, in the element-write branch, so the two element
                    // rhs forms (`'{…}` pattern and whole record) share one place and
                    // one index-clone rule.
                    if let ExprKind::Ident(rp) = &rhs.kind {
                        if rp.segments.len() == 1
                            && self.var_unpacked_struct.get(&rp.segments[0].name) == Some(&ty)
                        {
                            let rvar = rp.segments[0].name.clone();
                            let stmts = members
                                .iter()
                                .map(|m| {
                                    let mnet = Self::unpacked_member_net(&var, &m.name.name);
                                    let srcn = Self::unpacked_member_net(&rvar, &m.name.name);
                                    let elem = Lvalue::BitSelect {
                                        base: Box::new(Self::ident_lval(&mnet, span)),
                                        index: index.clone(),
                                        span,
                                    };
                                    Self::assign_stmt(
                                        elem,
                                        Self::ident_expr(&srcn, span),
                                        blocking,
                                        span,
                                    )
                                })
                                .collect();
                            return Some(Stmt::Block {
                                label: None,
                                decls: Vec::new(),
                                stmts,
                                span,
                            });
                        }
                    }
                    let ExprKind::AssignPattern(vals) = &rhs.kind else {
                        return None; // a non-pattern element rhs → loud fallthrough
                    };
                    if vals.len() != members.len() {
                        self.error_at(span, "a record-array element pattern must list every field");
                        // Emit an empty block: the error already fails compilation.
                        return Some(Stmt::Block {
                            label: None,
                            decls: Vec::new(),
                            stmts: Vec::new(),
                            span,
                        });
                    }
                    let stmts = members
                        .iter()
                        .zip(vals)
                        .map(|(m, v)| {
                            let mnet = Self::unpacked_member_net(&var, &m.name.name);
                            let elem = Lvalue::BitSelect {
                                base: Box::new(Self::ident_lval(&mnet, span)),
                                index: index.clone(),
                                span,
                            };
                            Self::assign_stmt(elem, v.clone(), blocking, span)
                        })
                        .collect();
                    return Some(Stmt::Block {
                        label: None,
                        decls: Vec::new(),
                        stmts,
                        span,
                    });
                }
            }
        }
        None
    }
}
