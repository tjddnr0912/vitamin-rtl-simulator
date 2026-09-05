//! special expression forms — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// The bounding `[lo,hi]` interval of the SINGLE rand field constrained by `e`
    /// (an over-approximation: OR ⇒ union, AND ⇒ intersect, leaf comparison ⇒
    /// half/point/range). `None` if `e` constrains more than one field or none —
    /// then no single-field domain narrowing applies. Used to size the sampling
    /// domain for `inside` / disjunctive single-field constraints.
    pub(crate) fn expr_field_interval(&self, e: &ast::Expr) -> Option<(String, i64, i64)> {
        use ast::{BinOp, ExprKind};
        match &e.kind {
            ExprKind::Paren { inner } => self.expr_field_interval(inner),
            ExprKind::Binary {
                op: BinOp::LogOr,
                lhs,
                rhs,
            } => {
                let (fl, llo, lhi) = self.expr_field_interval(lhs)?;
                let (fr, rlo, rhi) = self.expr_field_interval(rhs)?;
                if fl != fr {
                    return None;
                }
                Some((fl, llo.min(rlo), lhi.max(rhi)))
            }
            ExprKind::Binary {
                op: BinOp::LogAnd,
                lhs,
                rhs,
            } => {
                let (fl, llo, lhi) = self.expr_field_interval(lhs)?;
                let (fr, rlo, rhi) = self.expr_field_interval(rhs)?;
                if fl != fr {
                    return None;
                }
                Some((fl, llo.max(rlo), lhi.min(rhi)))
            }
            ExprKind::Binary { op, lhs, rhs }
                if matches!(
                    op,
                    BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq
                ) =>
            {
                let (f, c, op) = if let Some(f) = rand_field_ident(lhs) {
                    (f, self.const_eval_in_scope(rhs)?, *op)
                } else if let Some(f) = rand_field_ident(rhs) {
                    (f, self.const_eval_in_scope(lhs)?, flip_cmp(*op))
                } else {
                    return None;
                };
                let (lo, hi) = match op {
                    BinOp::Lt => (i64::MIN, c.saturating_sub(1)),
                    BinOp::Le => (i64::MIN, c),
                    BinOp::Gt => (c.saturating_add(1), i64::MAX),
                    BinOp::Ge => (c, i64::MAX),
                    BinOp::Eq => (c, c),
                    _ => return None,
                };
                Some((f, lo, hi))
            }
            _ => None,
        }
    }

    pub(crate) fn lower_int_literal(&mut self, kind: ast::IntLitKind, raw: &str) -> u32 {
        let cv = match parse_int_literal(raw, kind) {
            Some(cv) => cv,
            None => {
                // Truncate the echoed lexeme: a digit-cap-rejected decimal can be
                // hundreds of thousands of chars, and echoing it verbatim would be
                // unbounded stderr (a DoS in its own right).
                let shown: String = if raw.chars().count() > 64 {
                    format!(
                        "{}…({} chars)",
                        raw.chars().take(64).collect::<String>(),
                        raw.len()
                    )
                } else {
                    raw.to_string()
                };
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!("malformed integer literal `{shown}`"),
                );
                make_const_u32(0, 32)
            }
        };
        // P0-10: an unsized literal grows to hold its value (IEEE §3.5.1); cap the
        // result at MAX_NET_WIDTH like a declared net so a pathological wide
        // literal is rejected loud instead of interning a giant const (this also
        // makes the `bits.len() as u32` width casts in literal.rs unreachable past
        // u32). A sized over-cap literal is rejected for the same reason a
        // same-width net is.
        if cv.width as u64 > MAX_NET_WIDTH {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "integer literal width {} exceeds the v1 cap ({MAX_NET_WIDTH})",
                    cv.width
                ),
            );
            return self.intern_const(make_const_u32(0, 32));
        }
        self.intern_const(cv)
    }

    /// ⓑ-breadth (v17): lower the iterator `with`-expression with the iterator
    /// name (`item` or named) bound to `Expr::ArrayItem`, restoring the prior
    /// context afterward (nested with-clauses bind their own iterator). `elem` is
    /// the iterated array's element (width, signed) so a bare `item` sizes right.
    pub(crate) fn lower_with_expr(
        &mut self,
        amw: &ast::ArrayMethodWithExpr,
        elem: Option<(u32, bool)>,
    ) -> u32 {
        self.lower_with_expr_based(amw, elem, 0)
    }

    /// [`Self::lower_with_expr`] plus the DECLARED low index of the iterated
    /// array (V34-4). The engine hands `item.index` the flat slot number, which
    /// is the §7.12.3 index only for a 0-based array; `idx_base` shifts it so
    /// `int a[-1:1]` iterates -1, 0, 1. Every dynamic-storage receiver passes 0
    /// (a handle has no declared bounds), so those lower byte-for-byte as before.
    pub(crate) fn lower_with_expr_based(
        &mut self,
        amw: &ast::ArrayMethodWithExpr,
        elem: Option<(u32, bool)>,
        idx_base: i64,
    ) -> u32 {
        let name = amw
            .iter_var
            .as_ref()
            .map(|i| i.name.clone())
            .unwrap_or_else(|| "item".to_string());
        let saved = self.array_iter.replace(name);
        let saved_elem = self.array_iter_elem.take();
        let saved_base = self.array_iter_index_base;
        self.array_iter_elem = elem;
        self.array_iter_index_base = idx_base;
        let eid = self.lower_expr(&amw.with_expr);
        self.array_iter = saved;
        self.array_iter_elem = saved_elem;
        self.array_iter_index_base = saved_base;
        eid
    }

    /// ⓑ-breadth (v17): `arr.sum() with (expr)` (and product/and/or/xor) — a
    /// reduction with a per-element expression. Element-typed scalar result; the
    /// with-expr is folded instead of the raw element. A locator method here is a
    /// statement-only form (loud).
    pub(crate) fn lower_reduction_with_expr(&mut self, amw: &ast::ArrayMethodWithExpr) -> u32 {
        let which = match amw.method.name.as_str() {
            "sum" => ir::SysFuncId::ArrSum,
            "product" => ir::SysFuncId::ArrProduct,
            "and" => ir::SysFuncId::ArrAnd,
            "or" => ir::SysFuncId::ArrOr,
            "xor" => ir::SysFuncId::ArrXor,
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a locator method with `with` (find*/min/max/unique) must be the \
                     direct rhs of a blocking assign to a queue (`q2 = q.find() with (…);`)",
                );
                return self.placeholder_expr();
            }
        };
        if amw.recv.segments.len() != 1 {
            self.error(
                MsgCode::ElabUnsupported,
                "array `with`-method receiver must be a simple handle name",
            );
            return self.placeholder_expr();
        }
        // V34-4: the receiver is EITHER a dynamic-storage handle (as always) or a
        // 1-D fixed-size unpacked array — IEEE §7.12 applies the §7.12.3 reduction
        // methods to both, and the emitted IR is identical (`Signal{net,word:None}`
        // + `SysFunc`), so nothing about the frozen sim-ir shape changes.
        //
        // `idx_base` is the array's DECLARED low index. The engine iterates FLAT
        // slots, so `item.index` over `int a[-1:1]` would read 0,1,2 where §7.12.3
        // says -1,0,1; `lower_with_expr` rebases it. A handle is always 0-based.
        let (net, idx_base) = match self.dyn_handle(&amw.recv.segments[0].name) {
            Some((net, kind)) => {
                if !matches!(
                    kind,
                    ir::NetKind::DynArray
                        | ir::NetKind::Queue
                        | ir::NetKind::Assoc
                        | ir::NetKind::AssocStr
                ) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "array reduction on a non-array handle",
                    );
                    return self.placeholder_expr();
                }
                (net, 0i64)
            }
            None => match self.static_array_recv(&amw.recv.segments[0].name) {
                StaticArrayRecv::Integral(net, lo) => (net, lo),
                StaticArrayRecv::Unsupported(msg) => {
                    self.error(MsgCode::ElabUnsupported, msg);
                    return self.placeholder_expr();
                }
                StaticArrayRecv::No => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "array reduction `with` applies to a dynamic array / queue / assoc \
                         handle or a 1-D fixed-size unpacked array",
                    );
                    return self.placeholder_expr();
                }
            },
        };
        let elem = self.handle_elem_type(net);
        let handle = self.push_expr(ir::Expr::Signal { net, word: None });
        let with_eid = self.lower_with_expr_based(amw, elem, idx_base);
        self.push_expr(ir::Expr::SysFunc {
            which,
            args: vec![handle, with_eid],
        })
    }

    /// ⓑ-breadth (v17): `dst = src.LOCATOR()[ with (pred)]` — a queue-returning
    /// locator (IEEE §7.12.1), lowered to the statement-level `ArrLocator` SysTask.
    /// `dst` must be a queue handle (the locator result type). Returns `true` once
    /// it has handled (or loud-rejected) the form.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn lower_locator_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        delay: Option<&ast::Delay>,
        src_net: u32,
        src_kind: ir::NetKind,
        method: &str,
        amw: Option<&ast::ArrayMethodWithExpr>,
    ) -> bool {
        let (code, needs_pred) = match method {
            "min" => (0u32, false),
            "max" => (1, false),
            "unique" => (2, false),
            "find" => (3, true),
            "find_index" => (4, true),
            "find_first" => (5, true),
            "find_last" => (6, true),
            "find_first_index" => (7, true),
            "find_last_index" => (8, true),
            "unique_index" => (9, false),
            _ => return false,
        };
        if delay.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "a delayed locator assignment is outside the MVP",
            );
            return true;
        }
        if !matches!(
            src_kind,
            ir::NetKind::DynArray | ir::NetKind::Queue | ir::NetKind::Assoc | ir::NetKind::AssocStr
        ) {
            self.error(
                MsgCode::ElabUnsupported,
                "a locator method applies to a dynamic array / queue / assoc handle",
            );
            return true;
        }
        let dst = match lhs {
            ast::Lvalue::Ident(p) if p.segments.len() == 1 => self.dyn_handle(&p.segments[0].name),
            _ => None,
        };
        let Some((dst_net, ir::NetKind::Queue)) = dst else {
            self.error(
                MsgCode::ElabUnsupported,
                "a locator result must be assigned to a queue handle (`int r[$]; r = q.min();`)",
            );
            return true;
        };
        if needs_pred && amw.is_none() {
            self.error(
                MsgCode::ElabUnsupported,
                "find* locators require a `with (condition)` clause",
            );
            return true;
        }
        let dst_h = self.push_expr(ir::Expr::Signal {
            net: dst_net,
            word: None,
        });
        let src_h = self.push_expr(ir::Expr::Signal {
            net: src_net,
            word: None,
        });
        let elem = self.handle_elem_type(src_net);
        let kc = self.const_u32_expr(code, 32);
        let mut args = vec![dst_h, src_h, kc];
        if let Some(amw) = amw {
            if needs_pred {
                let pred = self.lower_with_expr(amw, elem);
                args.push(pred);
            } else {
                self.error(
                    MsgCode::ElabUnsupported,
                    "min/max/unique with a `with` key-expression is outside v1",
                );
                return true;
            }
        }
        let sid = self.push_stmt(ir::Stmt::SysTask {
            which: ir::SysTaskId::ArrLocator,
            fmt: None,
            args,
        });
        b.push_stmt_id(sid);
        true
    }

    /// v6: `st = h.first(k);` / next/last/prev — the iteration special form
    /// (the third BLOCKING-assign special). The key is a REF argument: it must
    /// be a plain whole VARIABLE; the engine writes it on a hit. On dyn/queue
    /// handles the dense walk is an INTERNAL desugar target only — gated to
    /// the synthetic `__foreach_*` index so the user surface stays assoc-only
    /// (IEEE defines first/next on associative arrays alone).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn lower_iter_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        delay: Option<&ast::Delay>,
        net: u32,
        kind: ir::NetKind,
        method: &str,
        args: &[ast::Expr],
    ) -> bool {
        if delay.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "a delayed iteration assignment is outside the MVP",
            );
            return true;
        }
        if args.len() != 1 {
            self.error(
                MsgCode::ElabUnsupported,
                "first/next/last/prev take exactly the key variable",
            );
            return true;
        }
        let key_net = match &args[0].kind {
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.lookup_net_scoped(&p.segments[0].name)
            }
            _ => None,
        };
        let Some(knet) = key_net else {
            self.error(
                MsgCode::ElabUnsupported,
                "the iteration key must be a plain variable (`st = a.first(k);`)",
            );
            return true;
        };
        let kkind = self.nets.get(knet as usize).map(|n| n.kind);
        if !matches!(
            kkind,
            Some(ir::NetKind::Reg | ir::NetKind::Logic | ir::NetKind::Integer)
        ) {
            self.error(
                MsgCode::ElabUnsupported,
                "the iteration key must be an integral VARIABLE (reg/logic/integer family)",
            );
            return true;
        }
        // A2a: the engine WRITES the found key into `knet` — a bare Signal that
        // never passes lower_lvalue (adversarial find: `aa.first(R)` silently
        // mutated a desugared array parameter).
        self.deny_readonly_write(knet, "write the iteration key into");
        if !matches!(kind, ir::NetKind::Assoc | ir::NetKind::AssocStr) {
            // The dense dyn/queue walk exists only for the foreach desugar.
            let synthetic = matches!(
                &args[0].kind,
                ast::ExprKind::Ident(p) if p.segments[0].name.starts_with("__foreach_")
            );
            if !synthetic {
                self.error(
                    MsgCode::ElabUnsupported,
                    "first/next/last/prev are associative-array methods (use `foreach` to walk a dyn array/queue)",
                );
                return true;
            }
        }
        let which = match method {
            "first" => ir::SysFuncId::AssocFirst,
            "next" => ir::SysFuncId::AssocNext,
            "last" => ir::SysFuncId::AssocLast,
            _ => ir::SysFuncId::AssocPrev,
        };
        let handle = self.push_expr(ir::Expr::Signal { net, word: None });
        let key = self.push_expr(ir::Expr::Signal {
            net: knet,
            word: None,
        });
        let rhs = self.push_expr(ir::Expr::SysFunc {
            which,
            args: vec![handle, key],
        });
        let lv = self.lower_lvalue(lhs);
        self.check_lvalue_kind(&lv, true);
        let sid = self.push_stmt(ir::Stmt::BlockingAssign { lhs: lv, rhs });
        b.push_stmt_id(sid);
        true
    }

    // ── $countbits desugar (SYS-INTRO잔여, IR-0) ───────────────────
    /// `$countbits(expr, ctrl1, …)` → 32-bit count of the bits of `expr` that
    /// 4-state-match ANY control bit. Each control arg is reduced to its bit 0
    /// (a 1-bit 0/1/x/z constant). The result is `Σ_i ((expr[i]===c0)||…) ? 1 : 0`
    /// — pure IR-0 over Select/CaseEq/Ternary/Add (iverilog 13.0 crashes ⇒ hand-
    /// IEEE §20.9). At least one control bit is required (E3009 otherwise).
    pub(crate) fn lower_countbits(&mut self, args: &[ast::Expr]) -> u32 {
        if args.len() < 2 {
            self.error(
                MsgCode::ElabUnsupported,
                "$countbits needs an expression and at least one control bit \
                 (e.g. $countbits(v, 1) / $countbits(v, 1'bx))",
            );
            return self.placeholder_expr();
        }
        let base = self.lower_expr(&args[0]);
        // Integral-only operand (IEEE §20.9) — a real would silently popcount its
        // IEEE-754 storage. Mirror the sibling $countones guard (loud E3009).
        if self.expr_is_real(base) {
            self.error(
                MsgCode::ElabUnsupported,
                "real operand is not legal for a bit-vector system function",
            );
            return self.placeholder_expr();
        }
        let Some(w) = self.ir_bits_of(base).filter(|&w| w > 0) else {
            self.error(
                MsgCode::ElabUnsupported,
                "$countbits operand has an unresolved width",
            );
            return self.placeholder_expr();
        };
        // `Select.offset`/`.width` are const-expr EDGES (ExprIds), not literals.
        let one_w = self.const_u32_expr(1, 32); // width-1 edge (shared)
        let off0 = self.const_u32_expr(0, 32); // bit-0 offset edge (shared)
                                               // Reduce each control arg to its bit 0 (a 1-bit 4-state constant).
        let ctrls: Vec<u32> = args[1..]
            .iter()
            .map(|c| {
                let cid = self.lower_expr(c);
                self.push_expr(ir::Expr::Select {
                    base: cid,
                    offset: off0,
                    width: one_w,
                    kind: ir::SelKind::Bit,
                })
            })
            .collect();
        let one = self.const_u32_expr(1, 32);
        let zero = self.const_u32_expr(0, 32);
        let mut acc = zero;
        for i in 0..w {
            let off_i = self.const_u32_expr(i, 32);
            let bit = self.push_expr(ir::Expr::Select {
                base,
                offset: off_i,
                width: one_w,
                kind: ir::SelKind::Bit,
            });
            // OR of (bit === ctrl_j) across all control bits.
            let mut matches: Option<u32> = None;
            for &c in &ctrls {
                let eq = self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::CaseEq,
                    lhs: bit,
                    rhs: c,
                });
                matches = Some(match matches {
                    None => eq,
                    Some(m) => self.push_expr(ir::Expr::Binary {
                        op: ir::BinOp::BitOr,
                        lhs: m,
                        rhs: eq,
                    }),
                });
            }
            let cond = matches.unwrap(); // ≥1 control guaranteed above
            let term = self.push_expr(ir::Expr::Ternary {
                cond,
                then_e: one,
                else_e: zero,
            });
            acc = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Add,
                lhs: acc,
                rhs: term,
            });
        }
        acc
    }

    // ── $bits const-fold (v7, IR-0) ────────────────────────────────
    /// `$bits(arg)` → 32-bit Const. The argument is a TYPE reference, never
    /// evaluated; unsupported shapes are LOUD (E3009), never a silent 0.
    pub(crate) fn lower_bits_fold(&mut self, arg: &ast::Expr) -> u32 {
        // N7: `$bits(obj.field)` = the FIELD width. The field Signal's net is the
        // 32-bit handle, so the generic `ir_bits_of` would wrongly report 32.
        if let ast::ExprKind::Ident(p) = &arg.kind {
            if let Some((_, class, field)) = self.resolve_class_member(p) {
                if let Some((_, f)) = self.class_field_id(&class, &field) {
                    return self.const_u32_expr(f.width, 32);
                }
            }
            // §2 🆕 M ⓒ: `$bits(u.X)` of a HIERARCHICAL net or parameter — its width
            // is known only once the sibling instance exists, so defer (as a whole
            // read `u.X` does) and patch the placeholder in
            // `resolve_deferred_hier_bits`. An interface-member alias (`ifc.sig`, a
            // local net named with a dot) keeps the local path.
            if p.segments.len() >= 2 {
                let joined = p
                    .segments
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                if self.lookup_net_scoped(&joined).is_none() {
                    let eid = self.const_u32_expr(32, 32);
                    self.deferred_hier_bits.push(DeferredHierBits {
                        eid,
                        prefix: self.cur_prefix.clone(),
                        path: p.segments.iter().map(|s| s.name.clone()).collect(),
                        span: Some(arg.span),
                    });
                    return eid;
                }
            }
        }
        let n = self
            .bits_of_view(arg, false)
            .or_else(|| {
                // General expression: lower it (dead arena nodes — the arg is
                // not evaluated at runtime) and fold its self-determined width.
                let eid = self.lower_expr(arg);
                self.ir_bits_of(eid)
            })
            .filter(|&n| n > 0);
        match n {
            Some(n) => self.const_u32_expr(n, 32),
            None => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "$bits argument shape unsupported (nets, array views, and \
                     self-determined expressions fold; v7)",
                );
                self.placeholder_expr()
            }
        }
    }

    // ── SYS-INTRO array-query / dimension const-folds (Medium bundle rank 2, IR-0) ──
    /// Fold `$size`/`$left`/`$right`/`$low`/`$high`/`$increment`/`$dimensions`/
    /// `$unpacked_dimensions`/`$isunbounded` to a 32-bit Const at elaborate (the
    /// argument is a TYPE/net reference, never evaluated — IEEE 1800 §20.6.2/§20.7).
    /// Returns `Some(const_eid)` if `name` is one of these AND the net resolves;
    /// `None` falls through (unknown func → the normal loud path). Dimension order
    /// = unpacked dims (decl order) then packed dims; `$increment = left>=right ? 1
    /// : -1`; a 0-dimension scalar with a dim query folds to 32-bit X (iverilog).
    pub(crate) fn try_introspect_fold(&mut self, name: &str, args: &[ast::Expr]) -> Option<u32> {
        // $isunbounded(x): 1 iff x is the `$` token, else 0 (hand-IEEE: iverilog
        // rejects; we const-fold). No unbounded ranges in v1 ⇒ almost always 0.
        if name == "$isunbounded" && args.len() == 1 {
            let v = matches!(args[0].kind, ast::ExprKind::Dollar) as i64;
            return Some(self.const_param_expr(v));
        }
        // SYS-INTRO잔여: `$typename(net)` const-folds to a packed-ASCII string of
        // vita's canonical type spelling (hand-IEEE — iverilog rejects $typename).
        // A net arg resolves; a type-literal / indexed / expression arg stays loud.
        if name == "$typename" && args.len() == 1 {
            let net = self.resolve_intro_net(&args[0])?;
            let s = self.typename_string(net);
            // A SYNTHESIZED string, not source text: build it from bytes rather
            // than re-quoting and re-escaping it. A type spelling that ever
            // contained a backslash would otherwise be re-interpreted here, and
            // with the Table 5-1 escapes in place it would also report an escape
            // warning about text no user wrote.
            let cid = self.intern_const(crate::literal::str_const_from_bytes(s.as_bytes()));
            return Some(self.push_expr(ir::Expr::Const { val: cid }));
        }
        let with_dim = matches!(
            name,
            "$size" | "$left" | "$right" | "$low" | "$high" | "$increment"
        );
        let no_dim = matches!(name, "$dimensions" | "$unpacked_dimensions");
        if !(with_dim || no_dim) || args.is_empty() {
            return None;
        }
        let net = self.resolve_intro_net(&args[0])?;
        let (dims, unpacked) = self.net_dims_desc(net)?;
        if name == "$dimensions" {
            return Some(self.const_param_expr(dims.len() as i64));
        }
        if name == "$unpacked_dimensions" {
            return Some(self.const_param_expr(unpacked as i64));
        }
        // 1-based dimension index, default 1 (the outermost dimension).
        let d = if args.len() >= 2 {
            self.const_eval_in_scope(&args[1])?
        } else {
            1
        };
        if d < 1 || (d as usize) > dims.len() {
            return Some(self.const_x32_expr()); // 0-dim / out-of-range ⇒ X (iverilog)
        }
        let (left, right) = dims[(d - 1) as usize];
        let val = match name {
            "$left" => left,
            "$right" => right,
            "$low" => left.min(right),
            "$high" => left.max(right),
            "$size" => (left - right).abs() + 1,
            "$increment" => {
                if left >= right {
                    1
                } else {
                    -1
                }
            }
            _ => return None,
        };
        Some(self.const_param_expr(val))
    }

    /// Width of the shapes `$bits` can take WITHOUT lowering: a static-array
    /// view (whole array = total bits, partial chain = remaining slice), a
    /// plain net ident, or a bound param (unsized i64 domain → 32,
    /// iverilog-pinned). Inline-subst formals fall through to the lowering
    /// path (the subst maps them to the actual's expr).
    ///
    /// `prescan_first`: in CONST contexts (param binding (3b), range specs)
    /// the current module's nets are not in the real table yet — the
    /// decl-order prescan is the authority there. At runtime the real table
    /// resolves first (it sees generate-scoped shadows the prescan doesn't).
    /// `$bits` of a SELF-DETERMINED expression, in the `&self` constant domain.
    ///
    /// The runtime path (`lower_bits_fold`) answers these by lowering the argument and
    /// reading the resulting node's width; the constant domain cannot, because lowering
    /// needs `&mut self` and would leave arena nodes behind at elaborate time. So this is
    /// the `&self` twin, and it is deliberately narrow: literals carry their own width,
    /// concatenation sums its parts, replication multiplies (§11.4.12), and EVERY other
    /// leaf — a name, an array view — is handed to `bits_of_view`, the same resolver the
    /// already-working `$bits(<name>)` case uses. Routing leaves anywhere else would let
    /// this walk and the lowering disagree about what a name means, which is the shape
    /// `classifier-must-match-its-lowering-resolver` names.
    ///
    /// ⚠️ NOT `const_self_width`: that walk resolves a bare name through `param_meta`,
    /// whose widths are a mix of declared and value-inferred, and §4.5.373 measured that
    /// a parameter's recorded value is not canonical at its recorded width. Reusing it
    /// here would import that unsoundness into a width that declares nets.
    pub(crate) fn bits_of_selfdet(&self, e: &ast::Expr) -> Option<u32> {
        match &e.kind {
            ast::ExprKind::Paren { inner } => self.bits_of_selfdet(inner),
            // §5.7.1: a fill in a self-determined position is ONE bit — `$bits('1)` is
            // 1 in both oracles; the literal parser's 32 is the i64 container (🆕 C).
            ast::ExprKind::IntLit { kind, raw } if crate::literal::is_fill_literal(raw, *kind) => {
                Some(1)
            }
            ast::ExprKind::IntLit { kind, raw } => {
                crate::literal::parse_int_literal(raw, *kind).map(|c| c.width)
            }
            ast::ExprKind::Concat { parts } => parts
                .iter()
                .try_fold(0u32, |acc, p| acc.checked_add(self.bits_of_selfdet(p)?)),
            ast::ExprKind::Replicate { count, value } => {
                let n = const_eval_u32(count)?;
                let one = value
                    .iter()
                    .try_fold(0u32, |acc, p| acc.checked_add(self.bits_of_selfdet(p)?))?;
                n.checked_mul(one)
            }
            _ => self.bits_of_view(e, true),
        }
        .filter(|&w| w > 0)
    }

    pub(crate) fn bits_of_view(&self, e: &ast::Expr, prescan_first: bool) -> Option<u32> {
        if let ast::ExprKind::Paren { inner } = &e.kind {
            return self.bits_of_view(inner, prescan_first);
        }
        let from_prescan = |me: &Self| -> Option<u32> {
            let (root, depth) = ident_index_chain(e)?;
            // mirror the lower_expr Ident resolution priority: a bound
            // formal/param shadows a same-named decl.
            if me.subst_lookup(root).is_some()
                || me.out_subst_lookup(root).is_some()
                || me.lookup_scoped(root).is_some()
            {
                return None;
            }
            let (elem, dims) = me.bits_prescan.get(root)?;
            if depth > dims.len() {
                return None; // indexing into packed space → lowering path
            }
            let rem: u64 = dims[depth..].iter().product();
            u32::try_from(elem.saturating_mul(rem)).ok()
        };
        let from_table = |me: &Self| -> Option<u32> {
            if let Some((net, idxs)) = me.expr_array_view(e) {
                let nv = me.nets.get(net as usize)?;
                let w = nv.width.max(1) as u64;
                if idxs.is_empty() {
                    return u32::try_from(w * nv.array_len.max(1) as u64).ok();
                }
                let dims = me.array_dims.get(&net)?;
                if idxs.len() > dims.len() {
                    return None; // trailing packed selects → lowering path
                }
                // `array_dims` stores `(lo, SIZE)`, not `(lo, hi)` — reading the second
                // field as an upper bound made `$bits` of a partially-indexed array view
                // off by one on every 0-based dim (`int a[2][3]` → `$bits(a[0])` counted
                // 4 elements) and arbitrarily wrong on a non-0-based one. iverilog-pinned.
                let rem: u64 = dims[idxs.len()..]
                    .iter()
                    .map(|&(_, size)| u64::from(size))
                    .product();
                return u32::try_from(rem * w).ok();
            }
            if let ast::ExprKind::Ident(p) = &e.kind {
                if let [seg] = p.segments.as_slice() {
                    let name = seg.name.as_str();
                    if me.subst_lookup(name).is_some() || me.out_subst_lookup(name).is_some() {
                        return None; // formal — resolve via the lowering path
                    }
                    // §2 🆕 L ⓝ (§4.5.430): a BLOCK-local (or function-local) variable
                    // that shadows a constant of the same name — a wildcard-imported
                    // package parameter binds at the module key the flattened local
                    // also uses — is the object `$bits` names; the value read already
                    // prefers it (`bare_ident_route`'s shadow test, mirrored here).
                    let local_shadows = me
                        .walk_scopes_key(name, |k| {
                            me.params.contains_key(k) || me.symbols.contains_key(k)
                        })
                        .is_some_and(|key| {
                            me.symbols.contains_key(&key)
                                && (me.block_local_declared_at(&key, e.span)
                                    || (!me.params.contains_key(&key)
                                        && me.block_local_covers(&key, e.span)))
                        });
                    if !local_shadows && me.lookup_scoped(name).is_some() {
                        // A TYPED param/localparam has a declared width (`logic
                        // [11:0] P` ⇒ 12); `$bits` must report THAT, not the
                        // value-inferred 32. An untyped param / genvar (no recorded
                        // width) stays 32 (the width-less i64 domain).
                        if let Some((w, _)) = me.walk_scopes(name, &me.param_meta) {
                            return Some(w);
                        }
                        return Some(32);
                    }
                    if let Some(net) = me.lookup_net_scoped(name) {
                        let nv = me.nets.get(net as usize)?;
                        if nv.kind == ir::NetKind::String {
                            return None; // dynamic length — loud at the site
                        }
                        // An unpacked array is its element width × its element count
                        // (`$bits(Q)` for `logic [11:0] Q [2]` is 24, both oracles) —
                        // the same product the array-view arm above computes; a
                        // block-local that shadows a constant reaches this arm instead.
                        return u32::try_from(
                            u64::from(nv.width.max(1)) * u64::from(nv.array_len.max(1)),
                        )
                        .ok();
                    }
                }
            }
            None
        };
        if prescan_first {
            from_prescan(self).or_else(|| from_table(self))
        } else {
            from_table(self).or_else(|| from_prescan(self))
        }
    }

    /// Per-label equality test for a case arm. Plain `case` is the exact 4-state
    /// `scrut === label`.
    ///
    /// `casez`/`casex` lower to the dedicated v7 match ops: a bit position is
    /// don't-care iff EITHER side (label or RUNTIME scrutinee) has z there
    /// (`CasezEq`) or x-or-z (`CasexEq`); every remaining position compares
    /// 4-state exact. This replaces the v1 `redor(scrut^label) !== 1` formula,
    /// which was exact for casex but over-lenient for casez (it wildcarded x
    /// too — `casez(1x10)` falsely matched `1010`; iverilog-pinned strict).
    /// §11.4.6 `==?`/`!=?` wildcard equality: the RHS PATTERN's x/z bits are
    /// don't-care; every other bit compares like plain `==`/`!=` (an LHS x/z in
    /// a compared position propagates x — UNLIKE `CasexEq`, which wildcards
    /// EITHER side, so mapping there would be a silent-wrong). Lowered as
    /// `(lhs & mask) ==/!= cleaned` with mask/cleaned computed from the CONSTANT
    /// pattern at elaborate time: mask = the pattern's known bits, 1-filled
    /// through the comparison width (the pattern zero-extends, so extension
    /// bits stay COMPARED against 0 — iverilog-pinned); cleaned = pattern with
    /// its wildcard bits cleared. The compare inherits vita's oracle-pinned
    /// `Eq`/`Ne` 4-state semantics. A NON-constant pattern would need a runtime
    /// known-bit mask (no frozen-IR primitive exposes the unk plane) →
    /// honest-loud; iverilog supports it, recorded as a follow-on.
    pub(crate) fn lower_wildcard_eq(&mut self, lhs: &ast::Expr, rhs: &ast::Expr, ne: bool) -> u32 {
        let lhs_id = self.lower_expr(lhs);
        // A fill pattern (`'1`/`'x`/…) sizes to the LHS width, like a case
        // label (§11.6 — mirrors `lower_case_label`).
        let rhs_id = if expr_contains_fill(rhs) {
            let w = self.ir_bits_of(lhs_id).unwrap_or(32);
            self.lower_expr_ctx(rhs, w)
        } else {
            self.lower_expr(rhs)
        };
        if self.expr_is_real(lhs_id) || self.expr_is_real(rhs_id) {
            self.error(
                MsgCode::ElabUnsupported,
                "wildcard equality (==?/!=?) is not defined on a real operand",
            );
            return self.placeholder_expr();
        }
        let cv = match self.exprs.get(rhs_id as usize) {
            Some(ir::Expr::Const { val }) => self.consts.get(*val as usize).cloned(),
            _ => None,
        };
        // Numeric AND string-literal patterns are fine (a string has packed
        // known bytes and no x/z, so its mask is all-ones — iverilog accepts
        // `"ab" ==? "ab"`). Real is guarded above; a COMPOUND const expression
        // (`{2'b1?,2'b1?}`, `(P|1)`) lowers to a non-Const node and stays loud
        // (a const-fold walker is a recorded follow-on — honest, iverilog folds).
        let Some(cv) = cv.filter(|c| c.repr != ir::ConstRepr::Real) else {
            self.error(
                MsgCode::ElabUnsupported,
                "wildcard equality (==?/!=?) needs a constant right-hand pattern \
                 (a runtime pattern's x/z mask has no IR primitive; a compound \
                 const expression is not folded yet — use a literal or parameter)",
            );
            return self.placeholder_expr();
        };
        // Comparison width = max(operands) (§11.8.2): the pattern zero-extends,
        // so mask bits ABOVE the pattern width stay 1 (known-0 must match). An
        // UNSIZABLE lhs is loud: falling back to the pattern width would build
        // a too-narrow mask whose zero-extension ANDs the lhs's high bits away
        // — every high bit would silently "match" (the engine widens the And
        // to max(lhs, mask), so the mask MUST cover the lhs).
        let Some(aw) = self.ir_bits_of(lhs_id) else {
            self.error(
                MsgCode::ElabUnsupported,
                "wildcard equality (==?/!=?) on a left operand of unsizable \
                 width is unsupported (the pattern mask must cover it)",
            );
            return self.placeholder_expr();
        };
        let w = aw.max(cv.width).max(1);
        let nwords = ((w as usize) + 63) / 64;
        let mut mask = vec![0u64; nwords];
        let mut clean = vec![0u64; nwords];
        for wi in 0..nwords {
            let cvv = cv.bits.val.get(wi).copied().unwrap_or(0);
            let cvu = cv.bits.unk.get(wi).copied().unwrap_or(0);
            mask[wi] = !cvu; // pattern x/z ⇒ 0 (don't-care); known/extension ⇒ 1
            clean[wi] = cvv & !cvu; // wildcard positions cleared to 0
        }
        let top = w % 64;
        if top != 0 {
            let m = (1u64 << top) - 1;
            mask[nwords - 1] &= m;
            clean[nwords - 1] &= m;
        }
        let push_const = |el: &mut Self, bits: Vec<u64>| -> u32 {
            let cid = el.intern_const(ir::ConstVal {
                width: w,
                signed: false,
                repr: ir::ConstRepr::Numeric,
                bits: ir::BitPacked {
                    val: bits,
                    unk: vec![0u64; nwords],
                },
            });
            el.push_expr(ir::Expr::Const { val: cid })
        };
        let m_id = push_const(self, mask);
        let c_id = push_const(self, clean);
        let anded = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::BitAnd,
            lhs: lhs_id,
            rhs: m_id,
        });
        self.push_expr(ir::Expr::Binary {
            op: if ne { ir::BinOp::Ne } else { ir::BinOp::Eq },
            lhs: anded,
            rhs: c_id,
        })
    }

    /// Fill-aware lowering of ONE case label to its IR expr id. §11.6: a case
    /// label is sized to the case-expression width, so a fill label grows to it
    /// (`case(x8) '1:` ⇒ the label is 8'hFF, not 32 bits — otherwise it never
    /// matches). Non-fill labels lower byte-identically. Split from `case_cmp`
    /// so `lower_case` can lower every label ONCE, derive the case's COLLECTIVE
    /// signedness from the label ids, and reuse them in the compare cascade (a
    /// second lowering would bloat the expr arena and churn the goldens).
    pub(crate) fn lower_case_label(&mut self, scrut_id: u32, label: &ast::Expr) -> u32 {
        // ⚠️ `sibling_ctx`, not a bare `ir_bits_of`: a REAL selector has no bit
        // width to lend (§6.12) and `ir_bits_of` answers its storage 64, so
        // `case (r) '1:` sized the label to 64 bits and fell through to `default`
        // where BOTH oracles match — and where `case (r) 1'b1:` already matched.
        let w = self.sibling_ctx(0, scrut_id);
        self.lower_case_label_at(w, label)
    }

    /// `lower_case_label` with the §12.5 common width supplied directly.
    ///
    /// `lower_case` uses this for the second pass, once the selector and every item
    /// have been seen and the common maximum is actually known — the first pass can
    /// only offer the selector's own width, which is not the rule when some OTHER item
    /// is wider (§4.5.356).
    pub(crate) fn lower_case_label_at(&mut self, w: u32, label: &ast::Expr) -> u32 {
        if expr_contains_fill(label) {
            self.lower_expr_ctx(label, w)
        } else {
            self.lower_expr(label)
        }
    }

    /// Per-label 4-state equality node from a PRE-LOWERED label id. Plain `case`
    /// is the exact 4-state `scrut === label`; casez/casex map to the dedicated
    /// v7 wildcard match ops.
    pub(crate) fn case_cmp(&mut self, scrut_id: u32, lbl_id: u32, kind: ast::CaseKind) -> u32 {
        let op = match kind {
            ast::CaseKind::Case => ir::BinOp::CaseEq,
            ast::CaseKind::Casez => ir::BinOp::CasezEq,
            ast::CaseKind::Casex => ir::BinOp::CasexEq,
        };
        self.push_expr(ir::Expr::Binary {
            op,
            lhs: scrut_id,
            rhs: lbl_id,
        })
    }
}
