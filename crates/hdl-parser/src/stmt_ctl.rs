//! control-flow statements — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// Desugar `lhs inside { item, … }` to an OR of equality (`lhs == v`) and range
    /// (`lhs >= lo && lhs <= hi`) tests. `lhs` is cloned per item (constraint / `if`
    /// operands are side-effect-free). An empty set never matches (`1'b0`).
    pub(crate) fn parse_inside(&mut self, lhs: Expr) -> Expr {
        let start = lhs.span;
        self.expect(TokenKind::LBrace, "'{' to open an `inside` set");
        let mut terms: Vec<Expr> = Vec::new();
        while self.peek() != Some(TokenKind::RBrace) && self.peek().is_some() {
            let before = self.pos;
            let term = if self.peek() == Some(TokenKind::LBracket) {
                self.bump(); // [
                let lo = self.expr(0);
                self.expect(TokenKind::Colon, "':' in an `inside` range");
                let hi = self.expr(0);
                self.expect(TokenKind::RBracket, "']' to close an `inside` range");
                let ge = mk_bin(BinOp::Ge, lhs.clone(), lo);
                let le = mk_bin(BinOp::Le, lhs.clone(), hi);
                mk_bin(BinOp::LogAnd, ge, le)
            } else {
                let v = self.expr(0);
                mk_bin(BinOp::Eq, lhs.clone(), v)
            };
            terms.push(term);
            if self.peek() == Some(TokenKind::Comma) {
                self.bump();
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "'}' to close an `inside` set");
        let span = start.to(self.prev_span());
        let mut it = terms.into_iter();
        let mut acc = it.next().unwrap_or(Expr {
            kind: ExprKind::IntLit {
                kind: IntLitKind::Sized,
                raw: "1'b0".to_string(),
            },
            span,
        });
        for t in it {
            acc = mk_bin(BinOp::LogOr, acc, t);
        }
        Expr {
            kind: acc.kind,
            span,
        }
    }

    // ─────────────────────── 4. control flow ───────────────────────
    pub(crate) fn parse_if(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // if
        self.expect(TokenKind::LParen, "'(' after 'if'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let then_s = Box::new(self.parse_statement());
        // dangling-else binds EAGERLY to this (nearest) if
        let else_s = if self.eat_kw(Kw::Else) {
            Some(Box::new(self.parse_statement()))
        } else {
            None
        };
        Stmt::If {
            cond,
            then_s,
            else_s,
            span: start.to(self.prev_span()),
        }
    }

    pub(crate) fn parse_case(&mut self, kind: CaseKind) -> Stmt {
        let start = self.cur_span();
        self.bump(); // case/casez/casex
        self.expect(TokenKind::LParen, "'(' after case");
        let scrutinee = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let mut items = Vec::new();
        while !self.at_eof() && !self.at_kw(Kw::Endcase) {
            let before = self.pos;
            items.push(self.parse_case_item());
            if self.pos == before {
                self.bump(); // never spin on a stuck case item
            }
        }
        self.expect(TokenKind::Word(WordKind::Keyword(Kw::Endcase)), "'endcase'");
        Stmt::Case {
            kind,
            scrutinee,
            items,
            span: start.to(self.prev_span()),
        }
    }

    /// `default [:] stmt` | `label {, label} : stmt`.
    pub(crate) fn parse_case_item(&mut self) -> CaseItem {
        let start = self.cur_span();
        if self.eat_kw(Kw::Default) {
            self.eat(TokenKind::Colon); // ':' OPTIONAL after default
            let body = Box::new(self.parse_statement());
            return CaseItem::Default {
                body,
                span: start.to(self.prev_span()),
            };
        }
        let mut labels = vec![self.expr(0)];
        while !self.node_budget_blown && self.eat(TokenKind::Comma) {
            labels.push(self.expr(0));
        }
        self.expect(TokenKind::Colon, "':' in case item");
        let body = Box::new(self.parse_statement());
        CaseItem::Match {
            labels,
            body,
            span: start.to(self.prev_span()),
        }
    }

    // ───────────────────────── SV §11.5 break/continue ─────────────────────
    /// Parse a loop body while tracking `break`/`continue` that target THIS loop.
    /// Pushes a `LoopLabels` (unique by the loop's start offset), parses the body,
    /// then — IF `continue` was used — wraps the body in a synthetic named block
    /// `begin : $continue$<lo> body end` (its exit is the loop's continue point:
    /// before the for-step / at the while back-edge). Returns the (maybe-wrapped)
    /// body and whether `break` was used (the caller wraps the whole loop). A loop
    /// with no break/continue is returned UNWRAPPED ⇒ byte-identical.
    pub(crate) fn parse_loop_body(&mut self, start: Span) -> (Stmt, bool) {
        let lo = start.lo;
        self.loop_labels.push(LoopLabels {
            break_label: format!("$break${lo}"),
            continue_label: format!("$continue${lo}"),
            break_used: false,
            continue_used: false,
        });
        let body = self.parse_statement();
        let lbl = self.loop_labels.pop().expect("pushed above");
        let body = if lbl.continue_used {
            Stmt::Block {
                label: Some(Ident {
                    name: lbl.continue_label,
                    span: start,
                }),
                decls: Vec::new(),
                stmts: vec![body],
                span: start,
            }
        } else {
            body
        };
        (body, lbl.break_used)
    }

    pub(crate) fn parse_for(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // for
        self.expect(TokenKind::LParen, "'(' after 'for'");
        // SV §12.7.1: a TYPED loop-variable declaration in the for-init
        // (`for (int i = 0; …)`). When a type keyword leads the init we parse a
        // local `NetVarDecl` and WRAP the whole For in an (unlabeled) block whose
        // `decls` carries the loop-var decl, so `hoist_block_local_nets` flattens
        // it to a module net just like any other block-local. v1 elaborate has NO
        // per-block scoping (block-locals share the module namespace), so a decl
        // named like an outer variable would be SKIPPED and the loop would alias
        // / clobber that outer var (silent-wrong vs iverilog, where the for-init
        // variable is implicitly local). So — exactly as `parse_foreach` does for
        // its index — we rename the loop variable to a synthetic UNIQUE name and
        // rewrite its references inside the for's init/cond/step/body. A nested
        // for / foreach reusing the same name renames ITS subtree first, so this
        // pass only ever sees its own occurrences (the rename helper's block arm
        // also stops at any inner redeclaration). An unlabeled wrapping block
        // lowers byte-identically to lowering the For alone.
        let typed_init = if self.net_var_kind().is_some() {
            self.parse_for_typed_init()
        } else if let Some(info) = self.peek_block_typedef_decl() {
            // SV §12.7.1 typed for-init using a user-defined type name
            // (`for (my_t i=0; …)`). The `<typedef> <ident>` shape is
            // unambiguously a decl (a plain `i = 0` re-uses an existing var and
            // `peek_block_typedef_decl` returns None for it).
            self.parse_for_typed_init_typedef(info)
        } else {
            None
        };
        let init = Box::new(match &typed_init {
            // The synthesized `i = init` assign (typed init always has one).
            Some((_, init_assign, _)) => init_assign.clone(),
            None => self.parse_for_assign(), // `i = 0`, no trailing ';'
        });
        self.expect(TokenKind::Semi, "';' after for-init");
        let cond = self.expr(0);
        self.expect(TokenKind::Semi, "';' after for-cond");
        let step = Box::new(self.parse_for_assign()); // `i = i+1`, no trailing ';'
        self.expect(TokenKind::RParen, "')'");
        let (body, break_used) = self.parse_loop_body(start);
        let span = start.to(self.prev_span());

        let mut for_stmt = Stmt::For {
            init,
            cond,
            step,
            body: Box::new(body),
            span,
        };

        let built = if let Some((decl, _, orig_name)) = typed_init {
            // Rewrite every reference to the original loop-var name across the
            // whole For → the synthetic name. The For's `init` was synthesized to
            // already carry the synthetic name (no `orig_name` occurrences), so the
            // rename only rebinds cond/step/body. `rename_ident_in_stmt`'s block
            // arm stops at any inner redeclaration, so a nested block/loop that
            // shadows the name keeps its own binding. (The synthetic `$continue$`
            // block has no decls, so the rename descends through it.)
            let synth = decl.names[0].name.name.clone();
            rename_ident_in_stmt(&mut for_stmt, &orig_name.name, &synth);
            Stmt::Block {
                label: None,
                decls: vec![decl],
                stmts: vec![for_stmt],
                span,
            }
        } else {
            for_stmt
        };
        self.wrap_break(built, break_used, start)
    }

    /// SV §12.7.1 typed for-init: `int i = 0` (or `integer` / `byte` /
    /// `logic [3:0]` / …). Parses the loop-variable declaration WITHOUT consuming
    /// the trailing `;` (the for-clause owns that). The declared variable is given
    /// a synthetic UNIQUE name (`__forvar_<name>_<span>`) so it never aliases a
    /// same-named outer var under v1's flat block-local namespace; the original
    /// name is returned so `parse_for` can rewrite the cond/step/body references.
    /// Returns the renamed `NetVarDecl`, the synthesized `i = init` blocking
    /// assign (already pointing at the synthetic name), and the ORIGINAL `Ident`.
    /// `None` when there is no `=` initializer to seed the loop (loud).
    pub(crate) fn parse_for_typed_init(&mut self) -> Option<(NetVarDecl, Stmt, Ident)> {
        let start = self.cur_span();
        let kind = self.net_var_kind().unwrap();
        self.bump(); // the type keyword
        let signed = self.signed_eff(Some(kind));
        let range = self.opt_range();
        let packed = self.opt_packed_dims();
        self.reject_packed_dims_on_nonvector(kind, range.is_some() || !packed.is_empty());
        self.build_for_typed_init(start, kind, signed, range, packed)
    }

    /// Shared tail of typed for-init parsing: read the single loop-variable
    /// declarator + `= init`, rename it to a synthetic unique name (so it never
    /// aliases a same-named outer var under v1's flat block-local namespace), and
    /// synthesize `(renamed decl, `i = init` assign, ORIGINAL ident)`.
    pub(crate) fn build_for_typed_init(
        &mut self,
        start: Span,
        kind: NetVarKind,
        signed: bool,
        range: Option<Range>,
        packed: Vec<Range>,
    ) -> Option<(NetVarDecl, Stmt, Ident)> {
        // SV §12.7.1 allows ONE loop variable; parse a single declarator (no
        // comma-list) so a stray comma stays a loud error rather than being
        // silently swallowed as a second for-init variable.
        let n_start = self.cur_span();
        let orig = self.ident()?;
        let init_expr = if self.eat(TokenKind::Eq) {
            self.expr(0)
        } else {
            // No initializer — the For has no defined start value. Emit loud and
            // bail rather than fabricate a `0`; the caller's `None` arm then falls
            // back to the plain-assign path (which errors on the leftover token).
            self.error("'=' initializer in a for-loop variable declaration");
            return None;
        };
        let synth = Ident {
            name: format!("__forvar_{}_{}", orig.name, start.lo),
            span: orig.span,
        };
        let decl_span = start.to(self.prev_span());
        let decl = NetVarDecl {
            kind,
            signed,
            range,
            packed,
            delay: None,
            names: vec![DeclName {
                name: synth.clone(),
                unpacked: Vec::new(),
                init: None, // seeded via the synthesized init assign below, not a static var-init
                span: n_start.to(self.prev_span()),
            }],
            lifetime: None,
            class_type: None,
            class_args: Vec::new(),
            const_param: false,
            span: decl_span,
        };
        let init_assign = Stmt::Blocking {
            lhs: Lvalue::Ident(HierPath {
                segments: vec![synth.clone()],
                span: synth.span,
            }),
            delay: None,
            event: None,
            rhs: init_expr,
            span: n_start.to(self.prev_span()),
        };
        Some((decl, init_assign, orig))
    }

    /// A single blocking assignment WITHOUT a trailing `;` (for-init / for-step).
    pub(crate) fn parse_for_assign(&mut self) -> Stmt {
        let start = self.cur_span();
        // for-init / for-step may use the SV §11.4 shorthands: `++i`, `i++`,
        // `i += n`. Prefix form first (leads with the operator), then the lvalue
        // forms (postfix / compound), else a plain `i = e`.
        if matches!(
            self.peek(),
            Some(TokenKind::PlusPlus) | Some(TokenKind::MinusMinus)
        ) {
            return self.parse_pre_incdec(start);
        }
        let lhs = self.parse_lvalue();
        if let Some(stmt) = self.try_compound_assign(&lhs, start) {
            return stmt;
        }
        self.expect(TokenKind::Eq, "'=' in for-clause assignment");
        let rhs = self.expr(0);
        let rhs = self.maybe_struct_pattern_rhs(&lhs, rhs);
        Stmt::Blocking {
            lhs,
            delay: None,
            event: None,
            rhs,
            span: start.to(self.prev_span()),
        }
    }

    pub(crate) fn parse_while(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // while
        self.expect(TokenKind::LParen, "'(' after 'while'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let (body, break_used) = self.parse_loop_body(start);
        let loop_stmt = Stmt::While {
            cond,
            body: Box::new(body),
            span: start.to(self.prev_span()),
        };
        self.wrap_break(loop_stmt, break_used, start)
    }

    /// P2-E: `do body while (cond);` desugars at parse to
    /// `begin body; while (cond) body end` — the body runs once before the
    /// first test (body CLONE; loops with side-effecting macro-expanded
    /// bodies are identical either way since both copies are the same AST).
    pub(crate) fn parse_do_while(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // do
                     // break/continue target THIS do-while (not an enclosing loop). The body
                     // is `$continue`-wrapped if needed; BOTH desugar copies (the once-run body
                     // and the while body) carry the same wrap — each is lowered separately so
                     // the disable stack resolves to the right copy's exit. `continue` in the
                     // first body falls through to the `while` (re-tests cond); in the while
                     // body it hits the back-edge. `break` wraps the whole desugar.
        let (body, break_used) = self.parse_loop_body(start);
        if !self.at_kw(Kw::While) {
            self.error("'while' after a do-body");
            return Stmt::Error(start.to(self.prev_span()));
        }
        self.bump(); // while
        self.expect(TokenKind::LParen, "'(' after 'while'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        self.expect(TokenKind::Semi, "';' after do-while");
        let span = start.to(self.prev_span());
        let again = Stmt::While {
            cond,
            body: Box::new(body.clone()),
            span,
        };
        let block = Stmt::Block {
            label: None,
            decls: Vec::new(),
            stmts: vec![body, again],
            span,
        };
        self.wrap_break(block, break_used, start)
    }

    /// v5 ⑥ follow-on, reworked at v6: `foreach (arr[i]) stmt` — PARSE-TIME
    /// desugar to the uniform first/next walk (no new AST/IR node):
    ///   begin : (anon)  integer i; integer __st;
    ///     __st = arr.first(i);
    ///     while (__st == 1) begin stmt  __st = arr.next(i); end
    ///   end
    /// ONE shape serves every dyn kind: elaborate lowers first/next on
    /// dyn/queue handles to the DENSE 0..size-1 walk (synthetic-index gated —
    /// the user surface keeps them assoc-only) and on assoc handles to the
    /// key-order walk (§7.9.4). A status of −1 (key wider than the integer
    /// index — possible on i64/string-keyed assoc) stops the loop with the
    /// engine's W4020 truncation warn. Anything that is not a dyn handle gets
    /// the method-call loud error at elaborate.
    /// Multi-index foreach (`a[i,j]`) is outside the MVP — loud at parse.
    pub(crate) fn parse_foreach(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // foreach
        self.expect(TokenKind::LParen, "'(' after 'foreach'");
        let Some(arr) = self.ident() else {
            self.error("an array name in 'foreach (name[index])'");
            return Stmt::Error(start);
        };
        self.expect(TokenKind::LBracket, "'['");
        let Some(ivar) = self.ident() else {
            self.error("a loop-index name in 'foreach (name[index])'");
            return Stmt::Error(start);
        };
        if self.peek() == Some(TokenKind::Comma) {
            // Multi-dimension foreach (IEEE §12.7.3): `foreach (a[i,j,…])`. `ivar`
            // is the already-parsed first index (dimension 1); collect the rest and
            // build the dimension-tagged nested desugar. (An empty LEADING slot
            // `foreach(a[,j])` is not handled here — `self.ident()` above already
            // loud-rejected the leading comma; that rare form stays honest-loud.)
            return self.parse_multidim_foreach(start, arr, ivar);
        }
        self.expect(TokenKind::RBracket, "']'");
        self.expect(TokenKind::RParen, "')'");
        // break/continue target THIS foreach. `continue` wraps the user body in
        // `$continue`, whose exit falls through to the `__st = next` advance (so
        // the iterator still advances after a continue); `break` wraps the
        // synthesized while below.
        let (mut body, break_used) = self.parse_loop_body(start);
        let span = start.to(self.prev_span());
        // v1 elaborate FLATTENS block-locals into the module namespace (no
        // per-block scoping), so a decl named like an outer variable would be
        // skipped and the loop would CLOBBER the outer one (silent-wrong vs
        // IEEE/iverilog, where the foreach index is implicitly local). Make
        // the index a synthetic unique name and rename its references inside
        // the body instead — correct shadowing with zero scoping support.
        // (A nested foreach reusing the same index name renames ITS body
        // first, so the outer pass only sees its own occurrences.)
        let synth = Ident {
            name: format!("__foreach_{}_{}", ivar.name, start.lo),
            span: ivar.span,
        };
        rename_ident_in_stmt(&mut body, &ivar.name, &synth.name);
        let ivar = synth;
        let one_seg = |id: &Ident| HierPath {
            segments: vec![id.clone()],
            span: id.span,
        };
        let ivar_expr = |id: &Ident| Expr {
            kind: ExprKind::Ident(one_seg(id)),
            span: id.span,
        };
        // synthetic status var (unique like the index — same collision rules).
        let stvar = Ident {
            name: format!("__foreach_st_{}", start.lo),
            span: ivar.span,
        };
        // r18 (Fix G): a foreach over a SoA record dyn-array/queue — the `.first`/`.next`
        // iterator must ride a REAL net, but a SoA record array `arr` has no net named
        // `arr` (only `$unp$arr$field`). Rewrite the iterator receiver to field 0's dyn
        // net (all fields share length). The body's `arr[i].field` already SoA-rewrote to
        // `$unp$arr$field[i]` during its own parse, so only the iterator needs this.
        let iter_arr = self
            .record_soa_vars
            .get(&arr.name)
            .and_then(|ty| self.unpacked_struct_layouts.get(ty))
            .and_then(|ms| ms.first())
            .map(|m0| Ident {
                name: Self::unpacked_member_net(&arr.name, &m0.name.name),
                span: arr.span,
            })
            .unwrap_or_else(|| arr.clone());
        // __st = arr.first(i) / arr.next(i)
        let iter_call = |method: &str| Expr {
            kind: ExprKind::Call {
                name: HierPath {
                    segments: vec![
                        iter_arr.clone(),
                        Ident {
                            name: method.to_string(),
                            span: arr.span,
                        },
                    ],
                    span: arr.span,
                },
                args: vec![ivar_expr(&ivar)],
            },
            span: arr.span,
        };
        let st_assign = |method: &str| Stmt::Blocking {
            lhs: Lvalue::Ident(one_seg(&stvar)),
            delay: None,
            event: None,
            rhs: iter_call(method),
            span,
        };
        // while (__st == 1) — a −1 truncation status stops the walk (W4020
        // already warned at the engine seam).
        let cond = Expr {
            kind: ExprKind::Binary {
                op: BinOp::Eq,
                lhs: Box::new(ivar_expr(&stvar)),
                rhs: Box::new(Self::dec_lit(1, span)),
            },
            span,
        };
        let loop_body = Stmt::Block {
            label: None,
            decls: Vec::new(),
            stmts: vec![body, st_assign("next")],
            span,
        };
        // block-local `integer i; integer __st;` so neither leaks/collides.
        let decl_of = |id: &Ident| NetVarDecl {
            kind: NetVarKind::Integer,
            signed: true,
            range: None,
            packed: Vec::new(),
            delay: None,
            names: vec![DeclName {
                name: id.clone(),
                unpacked: Vec::new(),
                init: None,
                span: id.span,
            }],
            lifetime: None,
            class_type: None,
            class_args: Vec::new(),
            const_param: false,
            span: id.span,
        };
        Stmt::Block {
            label: None, // the synthetic names need no block scope
            decls: vec![decl_of(&ivar), decl_of(&stvar)],
            stmts: vec![
                st_assign("first"),
                self.wrap_break(
                    Stmt::While {
                        cond,
                        body: Box::new(loop_body),
                        span,
                    },
                    break_used,
                    start,
                ),
            ],
            span,
        }
    }

    /// Multi-dimension `foreach (a[i,j,…])` (IEEE §12.7.3). `ivar0` is the already-
    /// parsed first index (dimension 1). Collects the remaining comma-separated
    /// slots (a named index iterates its 1-indexed dimension; an EMPTY slot leaves
    /// that dimension un-iterated), parses the loop body, and builds the nested
    /// per-dimension walk. Row-major: the leftmost named index is the OUTERMOST
    /// loop; `break`/`continue` target the innermost dimension's loop (matching the
    /// iverilog nested-for desugar).
    pub(crate) fn parse_multidim_foreach(&mut self, start: Span, arr: Ident, ivar0: Ident) -> Stmt {
        let mut named: Vec<(Ident, u32)> = vec![(ivar0, 1)];
        let mut dim: u32 = 1;
        while self.peek() == Some(TokenKind::Comma) {
            self.bump();
            dim += 1;
            // An empty slot (next is `,` or `]`) leaves this dimension un-iterated.
            if matches!(
                self.peek(),
                Some(TokenKind::Comma) | Some(TokenKind::RBracket)
            ) {
                continue;
            }
            if let Some(id) = self.ident() {
                // Each foreach index must be a distinct name (it declares an
                // implicit loop variable). A repeat (`foreach(a[i,i])`) is illegal —
                // iverilog rejects it, and silently aliasing both levels to one net
                // would mis-iterate. Loud here.
                if named.iter().any(|(e, _)| e.name == id.name) {
                    self.error_at(
                        id.span,
                        "a distinct name for each foreach index (this one repeats an earlier index)",
                    );
                    return Stmt::Error(start);
                }
                named.push((id, dim));
            } else {
                self.error("a loop-index name in 'foreach (name[index])'");
                return Stmt::Error(start);
            }
        }
        self.expect(TokenKind::RBracket, "']'");
        self.expect(TokenKind::RParen, "')'");
        let (body, break_used) = self.parse_loop_body(start);
        let span = start.to(self.prev_span());
        self.build_multi_foreach(&arr, &named, body, break_used, start, span)
    }

    /// Build the nested per-dimension walk for a multi-dimension `foreach`. Each
    /// named index `(idx, dim)` becomes one `while` level whose status var is driven
    /// by `arr.first/next(idx, dim)` — the 2nd (dimension) argument routes elaborate
    /// to the right unpacked dimension's bounds. Loops nest leftmost-outermost; the
    /// innermost carries `wrap_break` so `break`/`continue` affect only it.
    pub(crate) fn build_multi_foreach(
        &mut self,
        arr: &Ident,
        named: &[(Ident, u32)],
        mut body: Stmt,
        break_used: bool,
        start: Span,
        span: Span,
    ) -> Stmt {
        // Each user index → a unique synthetic name (implicit-local, shadow-correct
        // under v1's flat namespace; a nested foreach reusing a name renamed ITS
        // body first, so this pass only sees its own occurrences).
        let synths: Vec<(Ident, u32)> = named
            .iter()
            .map(|(id, d)| {
                let s = Ident {
                    name: format!("__foreach_{}_{}", id.name, start.lo),
                    span: id.span,
                };
                rename_ident_in_stmt(&mut body, &id.name, &s.name);
                (s, *d)
            })
            .collect();
        let one_seg = |id: &Ident| HierPath {
            segments: vec![id.clone()],
            span: id.span,
        };
        let decl_of = |id: &Ident| NetVarDecl {
            kind: NetVarKind::Integer,
            signed: true,
            range: None,
            packed: Vec::new(),
            delay: None,
            names: vec![DeclName {
                name: id.clone(),
                unpacked: Vec::new(),
                init: None,
                span: id.span,
            }],
            lifetime: None,
            class_type: None,
            class_args: Vec::new(),
            const_param: false,
            span: id.span,
        };
        let mut decls: Vec<NetVarDecl> = Vec::new();
        let mut cur: Stmt = body;
        let n = synths.len();
        for (lvl, (idx, d)) in synths.iter().enumerate().rev() {
            let innermost = lvl + 1 == n;
            let stvar = Ident {
                name: format!("__foreach_st{}_{}", lvl, start.lo),
                span: idx.span,
            };
            // `arr.first/next(idx, dim)` — args[1] tags the dimension for elaborate.
            let iter_call = |method: &str| Expr {
                kind: ExprKind::Call {
                    name: HierPath {
                        segments: vec![
                            arr.clone(),
                            Ident {
                                name: method.to_string(),
                                span: arr.span,
                            },
                        ],
                        span: arr.span,
                    },
                    args: vec![
                        Expr {
                            kind: ExprKind::Ident(one_seg(idx)),
                            span: idx.span,
                        },
                        Self::dec_lit(*d, span),
                    ],
                },
                span: arr.span,
            };
            let st_assign = |method: &str| Stmt::Blocking {
                lhs: Lvalue::Ident(one_seg(&stvar)),
                delay: None,
                event: None,
                rhs: iter_call(method),
                span,
            };
            let cond = Expr {
                kind: ExprKind::Binary {
                    op: BinOp::Eq,
                    lhs: Box::new(Expr {
                        kind: ExprKind::Ident(one_seg(&stvar)),
                        span: stvar.span,
                    }),
                    rhs: Box::new(Self::dec_lit(1, span)),
                },
                span,
            };
            let loop_body = Stmt::Block {
                label: None,
                decls: Vec::new(),
                stmts: vec![cur, st_assign("next")],
                span,
            };
            let while_stmt = Stmt::While {
                cond,
                body: Box::new(loop_body),
                span,
            };
            let wrapped = if innermost {
                self.wrap_break(while_stmt, break_used, start)
            } else {
                while_stmt
            };
            cur = Stmt::Block {
                label: None,
                decls: Vec::new(),
                stmts: vec![st_assign("first"), wrapped],
                span,
            };
            decls.push(decl_of(idx));
            decls.push(decl_of(&stvar));
        }
        Stmt::Block {
            label: None,
            decls,
            stmts: vec![cur],
            span,
        }
    }

    pub(crate) fn parse_repeat(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // repeat
        self.expect(TokenKind::LParen, "'(' after 'repeat'");
        let count = self.expr(0);
        self.expect(TokenKind::RParen, "')'");
        let (body, break_used) = self.parse_loop_body(start);
        let loop_stmt = Stmt::Repeat {
            count,
            body: Box::new(body),
            span: start.to(self.prev_span()),
        };
        self.wrap_break(loop_stmt, break_used, start)
    }

    pub(crate) fn parse_forever(&mut self) -> Stmt {
        let start = self.cur_span();
        self.bump(); // forever — NO parens, NO count
        let (body, break_used) = self.parse_loop_body(start);
        let loop_stmt = Stmt::Forever {
            body: Box::new(body),
            span: start.to(self.prev_span()),
        };
        self.wrap_break(loop_stmt, break_used, start)
    }
}
