//! generate constructs — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// `genvar i, j;` → `ModuleItem::Genvar{names, span}`. The `genvar` keyword is
    /// already at `peek()`. An empty/garbled name list still terminates at `;`.
    pub(crate) fn parse_genvar_decl(&mut self) -> ModuleItem {
        let start = self.cur_span();
        self.bump(); // `genvar`
        let mut names = Vec::new();
        if let Some(id) = self.ident() {
            names.push(id);
            while self.eat(TokenKind::Comma) {
                match self.ident() {
                    Some(id) => names.push(id),
                    None => break, // diagnosed by ident(); stop the list
                }
            }
        }
        self.expect(TokenKind::Semi, "';' after genvar declaration");
        // §3 ⑤ ⓓ (review B-3): a genvar is a declaration — it shadows a same-named
        // imported / outer constant, which must not fold a generate index in its
        // place (`import p::*` exporting `gi`, `genvar gi; … gg[gi]` folded p's 2).
        for id in &names {
            self.unbind_struct_enum_name(&id.name);
        }
        ModuleItem::Genvar {
            names,
            span: start.to(self.prev_span()),
        }
    }

    /// `generate <gen_items> endgenerate`. Dispatch only calls this on the
    /// `generate` keyword; the SV bare-`if`/`for`/`case`-at-module-scope form is a
    /// DEFERRED variant.
    pub(crate) fn parse_generate_construct(&mut self) -> GenerateConstruct {
        let start = self.cur_span();
        self.bump(); // `generate`
        let items = self.parse_gen_items_until(&|p| p.at_kw(Kw::Endgenerate) || p.at_eof());
        self.expect(
            TokenKind::Word(WordKind::Keyword(Kw::Endgenerate)),
            "'endgenerate'",
        );
        GenerateConstruct {
            items,
            span: start.to(self.prev_span()),
        }
    }

    /// Parse `GenItem`s until `stop` is true (or EOF). Shared by the construct
    /// body, gen-blocks (`begin … end`), and case-item bodies. Forward-progress
    /// guarded.
    pub(crate) fn parse_gen_items_until(&mut self, stop: &dyn Fn(&Self) -> bool) -> Vec<GenItem> {
        let mut items = Vec::new();
        // The `pending` disjunct keeps the loop alive to drain a body-param
        // comma-list continuation that is the LAST item before the stop token.
        while !self.at_eof() && (!self.pending_module_items.is_empty() || !stop(self)) {
            // Emit queued body-param comma-list continuations (same generate scope,
            // wrapped as plain gen items) before the forward-progress guard below.
            if !self.pending_module_items.is_empty() {
                items.push(GenItem::Item(Box::new(self.pending_module_items.remove(0))));
                continue;
            }
            let before = self.pos;
            if let Some(it) = self.parse_gen_item() {
                items.push(it);
            }
            if self.pos == before {
                self.bump(); // never spin on a stuck gen-item
            }
        }
        items
    }

    /// One generate item: `for` / `if` / `case` / `begin…end` block / genvar decl
    /// / a plain module-item (instance, cont-assign, net, procedural block). A
    /// stray `;` (empty item) is consumed and yields nothing.
    pub(crate) fn parse_gen_item(&mut self) -> Option<GenItem> {
        if self.eat(TokenKind::Semi) {
            return None; // empty generate item
        }
        if self.at_kw(Kw::For) {
            return Some(self.parse_gen_for());
        }
        if self.at_kw(Kw::If) {
            return Some(self.parse_gen_if());
        }
        if self.at_kw(Kw::Case) {
            return Some(self.parse_gen_case());
        }
        if self.at_kw(Kw::Begin) {
            return Some(self.parse_gen_block());
        }
        // genvar decls inside generate are legal — keep them wrapped so elaborate's
        // no-op handler ignores them (they never become nets).
        if self.at_kw(Kw::Genvar) {
            return Some(GenItem::Item(Box::new(self.parse_genvar_decl())));
        }
        // anything else → a plain module-item (instance / assign / net / proc / …).
        // `parse_module_item` returns None only after recording an error; wrap a
        // real item, else propagate None (the caller's progress guard syncs).
        self.parse_module_item()
            .map(|mi| GenItem::Item(Box::new(mi)))
    }

    /// `for ( genvar_id = e ; cond ; genvar_id = e ) gen_block`. A `begin : label`
    /// hoists its label onto the For node (see `parse_gen_branch`).
    pub(crate) fn parse_gen_for(&mut self) -> GenItem {
        let start = self.cur_span();
        self.bump(); // `for`
        self.expect(TokenKind::LParen, "'(' after generate 'for'");
        // IEEE 1800-2017 §27.4 `genvar_initialization`: the loop variable may be
        // DECLARED in the header (`for (genvar i = 0; …)`). vita required it outside,
        // which widens the name's scope to the whole module and hands collision
        // management back to the author. The elaborator already binds `init.lvalue`
        // as the loop's genvar whether or not a declaration preceded it, so the
        // keyword is consumed and nothing else changes.
        let _ = self.eat_kw(Kw::Genvar);
        let init = self.parse_gen_assign(false);
        // The loop variable shadows a same-named constant for the LOOP only (§27.4
        // — a header genvar's scope is the loop; review A k03: `localparam int i`
        // read as `g[i].x` after the loop must still fold to the localparam).
        let shadowed = self.const_locals.remove(&init.lvalue.name);
        self.expect(TokenKind::Semi, "';' after generate-for init");
        let cond = self.expr(0);
        self.expect(TokenKind::Semi, "';' after generate-for cond");
        let step = self.parse_gen_assign(true);
        self.expect(TokenKind::RParen, "')' after generate-for header");
        let (label, body) = self.parse_gen_branch();
        if let Some(v) = shadowed {
            self.const_locals.insert(init.lvalue.name.clone(), v);
        }
        GenItem::For {
            init,
            cond,
            step,
            label,
            body,
            span: start.to(self.prev_span()),
        }
    }

    /// `genvar_iteration` for a generate-for init/step (no trailing `;`). LHS is a
    /// single genvar identifier (the LRM restricts it — not a general lvalue). The
    /// STEP (`is_step`) also accepts the IEEE §27.4 `++g`/`g++`/`--g`/`g--`/`g op= expr`
    /// forms, desugared to `g = g <op> operand` (byte-identical to the explicit
    /// `g = g + …` it produces, reusing the procedural for-step operators); the INIT
    /// is `g = expr` only (`for (g++; …)` stays a loud `=`-expected error).
    pub(crate) fn parse_gen_assign(&mut self, is_step: bool) -> GenAssign {
        let start = self.cur_span();
        // STEP prefix `++g` / `--g` (inc_or_dec_operator genvar_identifier).
        if is_step {
            if let Some(t @ (TokenKind::PlusPlus | TokenKind::MinusMinus)) = self.peek() {
                let op = Self::compound_assign_binop(t).expect("++/-- is a compound op");
                self.bump(); // ++ / --
                let lvalue = self.gen_step_ident(start);
                return self.gen_step_assign(lvalue, op, None, start);
            }
        }
        let lvalue = self.gen_step_ident(start);
        // STEP postfix `g++` / `g--` or compound `g op= expr`.
        if is_step {
            if let Some(t) = self.peek() {
                if let Some(op) = Self::compound_assign_binop(t) {
                    let is_incdec = matches!(t, TokenKind::PlusPlus | TokenKind::MinusMinus);
                    self.bump(); // the `++`/`--`/`op=` operator
                    let operand = if is_incdec { None } else { Some(self.expr(0)) };
                    return self.gen_step_assign(lvalue, op, operand, start);
                }
            }
        }
        self.expect(TokenKind::Eq, "'=' in generate-for assignment");
        let value = self.expr(0);
        GenAssign {
            lvalue,
            value,
            span: start.to(self.prev_span()),
        }
    }

    /// A single genvar identifier for a generate-for init/step (empty-name recovery).
    pub(crate) fn gen_step_ident(&mut self, start: Span) -> Ident {
        self.ident().unwrap_or(Ident {
            name: String::new(),
            span: start,
        })
    }

    /// Build a generate-for STEP `g = g <op> operand` (operand `None` ⇒ literal `1`
    /// for `++`/`--`). Shared by the prefix/postfix/`op=` desugars — the resulting
    /// `GenAssign` is byte-identical to the explicit `g = g <op> …` form.
    pub(crate) fn gen_step_assign(
        &self,
        lvalue: Ident,
        op: BinOp,
        operand: Option<Expr>,
        start: Span,
    ) -> GenAssign {
        let operand = operand.unwrap_or(Expr {
            span: self.prev_span(),
            kind: ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw: "1".to_string(),
            },
        });
        let lhs_expr = Expr {
            span: start,
            kind: ExprKind::Ident(HierPath {
                segments: vec![lvalue.clone()],
                span: start,
            }),
        };
        let span = start.to(self.prev_span());
        GenAssign {
            lvalue,
            value: Expr {
                span,
                kind: ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs_expr),
                    rhs: Box::new(operand),
                },
            },
            span,
        }
    }

    /// `if ( cond ) gen_item [ else gen_item ]`. Dangling-else binds EAGERLY to the
    /// nearest `if` (same rule as the procedural parser).
    pub(crate) fn parse_gen_if(&mut self) -> GenItem {
        let start = self.cur_span();
        self.bump(); // `if`
        self.expect(TokenKind::LParen, "'(' after generate 'if'");
        let cond = self.expr(0);
        self.expect(TokenKind::RParen, "')' after generate-if condition");
        let (label, then_b) = self.parse_gen_branch();
        let else_b = if self.eat_kw(Kw::Else) {
            self.parse_gen_branch().1
        } else {
            Vec::new()
        };
        GenItem::If {
            cond,
            then_b,
            else_b,
            label,
            span: start.to(self.prev_span()),
        }
    }

    /// `case ( e ) { label{,label}: gen_item | default[:] gen_item } endcase`.
    pub(crate) fn parse_gen_case(&mut self) -> GenItem {
        let start = self.cur_span();
        self.bump(); // `case`
        self.expect(TokenKind::LParen, "'(' after generate 'case'");
        let scrutinee = self.expr(0);
        self.expect(TokenKind::RParen, "')' after generate-case scrutinee");
        let mut items = Vec::new();
        while !self.at_eof() && !self.at_kw(Kw::Endcase) {
            let before = self.pos;
            items.push(self.parse_gen_case_item());
            if self.pos == before {
                self.bump(); // never spin on a stuck case item
            }
        }
        self.expect(
            TokenKind::Word(WordKind::Keyword(Kw::Endcase)),
            "'endcase' for generate-case",
        );
        GenItem::Case {
            scrutinee,
            items,
            span: start.to(self.prev_span()),
        }
    }

    /// One generate-case item: `default [:] gen_item` | `label {, label} : gen_item`.
    pub(crate) fn parse_gen_case_item(&mut self) -> GenCaseItem {
        let start = self.cur_span();
        if self.eat_kw(Kw::Default) {
            self.eat(TokenKind::Colon); // ':' OPTIONAL after default
            let body = self.gen_case_body(start);
            return GenCaseItem::Default {
                body,
                span: start.to(self.prev_span()),
            };
        }
        let mut labels = vec![self.expr(0)];
        while !self.node_budget_blown && self.eat(TokenKind::Comma) {
            labels.push(self.expr(0));
        }
        self.expect(TokenKind::Colon, "':' in generate-case item");
        let body = self.gen_case_body(start);
        GenCaseItem::Match {
            labels,
            body,
            span: start.to(self.prev_span()),
        }
    }

    /// A generate-case item's body, KEEPING the `begin : label` name.
    ///
    /// ⚠️ Both arms above used to call `parse_gen_branch().1`, which takes the items
    /// and throws away `.0` — the label — while the `if` and `for` arms bind it
    /// (`let (label, body) = self.parse_gen_branch()`). A named generate-case block
    /// therefore minted NO scope at all: its members landed in the ENCLOSING scope.
    /// MEASURED on `case (1) 1: begin : g logic [7:0] x; end`: vita's VCD shows
    /// scopes `tb u` where the `if`/`begin` spellings of the same design show
    /// `tb u g[0]`, so `u.g.x` and `u.g[0].x` were BOTH E3010 — the one generate
    /// kind unreachable by either spelling. Worse than unreachable when the name
    /// collides: a parent with its own `x` gets E3009 "redeclared" on a design both
    /// iverilog and verilator accept (they print the parent's `x`).
    ///
    /// The fix re-wraps rather than extending the AST: `GenCaseItem` has no `label`
    /// slot, and adding one would flip the `hdl-ast` SchemaHash for a name the
    /// EXISTING `GenItem::Block` arm already knows how to scope. Wrapping hands the
    /// labelled body to `elaborate_gen_scoped` through that arm, so the `label[0]`
    /// naming is the one the other kinds use — one spelling, not a second copy.
    /// An UNLABELLED body is passed through untouched, preserving the transparent
    /// (`unlabeled_is_scope`) behaviour the case arm already had.
    fn gen_case_body(&mut self, start: Span) -> Vec<GenItem> {
        let (label, items) = self.parse_gen_branch();
        match label {
            Some(l) => vec![GenItem::Block {
                label: Some(l),
                items,
                span: start.to(self.prev_span()),
            }],
            None => items,
        }
    }

    /// `begin [: label] gen_items end [: label]` → a `GenItem::Block`.
    pub(crate) fn parse_gen_block(&mut self) -> GenItem {
        let start = self.cur_span();
        self.bump(); // `begin`
        let label = self.opt_block_label(); // reuse PR2 helper (`: name` or None)
                                            // A generate block is a scope (IEEE §27.2): its typedefs, struct/enum name
                                            // bindings and IMPORTS end with it. Without the snapshot an `import r::P;`
                                            // inside the block dropped the module-scope binding of `P` (§4.5.410
                                            // review: q's value read through r's layout at module scope).
        let snap = self.snapshot_scope();
        let items = self.parse_gen_items_until(&|p| p.at_kw(Kw::End) || p.at_eof());
        self.restore_scope(snap);
        self.expect(TokenKind::Word(WordKind::Keyword(Kw::End)), "'end'");
        self.opt_block_label(); // optional `: end_label` (no AST slot → discard)
        GenItem::Block {
            label,
            items,
            span: start.to(self.prev_span()),
        }
    }

    /// Parse a control-structure BRANCH body and HOIST a `begin:label` label out of
    /// it. Returns `(label, items)`:
    /// - `begin [: lbl] … end` → `(lbl, inner_items)` (the begin/end is unwrapped so
    ///   the For/If node carries the label directly — elaborate's `label[idx]`
    ///   prefixing expects the loop/if to OWN the label).
    /// - any other single gen-item → `(None, vec![item])`.
    pub(crate) fn parse_gen_branch(&mut self) -> (Option<Ident>, Vec<GenItem>) {
        if self.at_kw(Kw::Begin) {
            match self.parse_gen_block() {
                GenItem::Block { label, items, .. } => (label, items),
                other => (None, vec![other]), // unreachable; defensive
            }
        } else {
            // A single (unbracketed) gen item. A body param comma-list here
            // (`if (c) localparam A=1, B=2;`) emits >1 item from ONE construct;
            // collect the queued continuations into THIS branch so they stay scoped
            // to it rather than leaking to the enclosing scope.
            let mut items = Vec::new();
            if let Some(it) = self.parse_gen_item() {
                items.push(it);
            }
            while !self.pending_module_items.is_empty() {
                items.push(GenItem::Item(Box::new(self.pending_module_items.remove(0))));
            }
            (None, items)
        }
    }
}
