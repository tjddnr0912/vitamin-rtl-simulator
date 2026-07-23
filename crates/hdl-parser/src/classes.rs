//! classes / constraints — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// Parse `value dist { item, … }` into `ExprKind::Dist`. Each item is a single
    /// value or a `[lo:hi]` range, optionally followed by `:= weight` (per-value) or
    /// `:/ weight` (weight spread over the range); the default weight is `:= 1`.
    pub(crate) fn parse_dist(&mut self, lhs: Expr) -> Expr {
        let start = lhs.span;
        self.expect(TokenKind::LBrace, "'{' to open a `dist` set");
        let mut items: Vec<DistItem> = Vec::new();
        while self.peek() != Some(TokenKind::RBrace) && self.peek().is_some() {
            let before = self.pos;
            let (lo, hi) = if self.peek() == Some(TokenKind::LBracket) {
                self.bump(); // [
                let lo = self.expr(0);
                self.expect(TokenKind::Colon, "':' in a `dist` range");
                let hi = self.expr(0);
                self.expect(TokenKind::RBracket, "']' to close a `dist` range");
                (lo, Some(Box::new(hi)))
            } else {
                (self.expr(0), None)
            };
            // weight: `:= w` (per-value) or `:/ w` (spread); default `:= 1`.
            let (weight, per_range) = if self.peek() == Some(TokenKind::Colon) {
                self.bump(); // :
                let per_range = if self.eat(TokenKind::Slash) {
                    true
                } else {
                    self.expect(TokenKind::Eq, "'=' or '/' after ':' in a `dist` weight");
                    false
                };
                (Box::new(self.expr(0)), per_range)
            } else {
                (
                    Box::new(Expr {
                        kind: ExprKind::IntLit {
                            kind: IntLitKind::Decimal,
                            raw: "1".to_string(),
                        },
                        span: self.cur_span(),
                    }),
                    false,
                )
            };
            items.push(DistItem {
                lo: Box::new(lo),
                hi,
                weight,
                per_range,
            });
            if self.peek() == Some(TokenKind::Comma) {
                self.bump();
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "'}' to close a `dist` set");
        let span = start.to(self.prev_span());
        Expr {
            kind: ExprKind::Dist {
                value: Box::new(lhs),
                items,
            },
            span,
        }
    }

    /// G8: parse a chained method call `<recv>.method(args)` where `recv` is a call /
    /// method result (cursor at the `.`).
    pub(crate) fn parse_method_chain(&mut self, recv: Expr) -> Expr {
        let start = recv.span;
        self.bump(); // '.'
        let method = self.ident().unwrap_or_else(|| Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        let args = self.call_args();
        let args = self.expand_struct_call_args(args); // R5: struct actual → members
        Expr {
            kind: ExprKind::MethodCall {
                recv: Box::new(recv),
                method,
                args,
            },
            span: start.to(self.prev_span()),
        }
    }

    // ─────────────────────── 1b. function / task definitions ───────────────────────
    /// `function [automatic] [signed] [range] [ret_type] name [(tf_ports)] ;
    ///    {body_decl} body_stmt endfunction`
    /// V2005: return width = `signed` + `range`; `ret_type` is one of
    /// ParamType::{Implicit,Integer,Real,Realtime,Time} (a `reg [N]` return maps to
    /// Implicit + range — ParamType has no Reg/Logic). Ports may be ANSI (in the
    /// paren list) or non-ANSI (input/output decls in the body prefix, hoisted).
    /// N7: register every `class NAME` in the token stream as a class-typed
    /// alias so `NAME var;` parses (forward-reference safe; any nesting).
    pub(crate) fn prescan_class_names(&mut self) {
        let mut names: Vec<String> = Vec::new();
        for i in 0..self.toks.len() {
            if matches!(
                self.toks[i].kind,
                TokenKind::Word(WordKind::Keyword(Kw::Class))
            ) {
                if let Some(t) = self.toks.get(i + 1) {
                    if matches!(t.kind, TokenKind::Word(WordKind::Ident)) {
                        names.push(self.src[t.span.clone()].to_string());
                    }
                }
            }
        }
        for n in names {
            self.typedefs.entry(n.clone()).or_insert(TypeInfo {
                kind: NetVarKind::ClassHandle,
                signed: false,
                range: None,
                packed: Vec::new(),
                class_name: Some(n.clone()),
            });
        }
    }

    /// `class NAME [extends BASE] ; { class_item } endclass [: NAME]` (N7).
    /// Parameterized classes (`class C #(…)`) and `virtual class` (abstract) are
    /// loud-deferred at elaborate; here we parse the plain single-inheritance
    /// form. Returns `None` only on a missing class name.
    /// `virtual [interface] IFACE name [, name2];` (ⓑ-breadth, §25.9) — a virtual
    /// interface handle. The interface type name rides `class_type`; elaborate
    /// resolves the static binding alias.
    pub(crate) fn parse_virtual_iface_decl(&mut self) -> Option<NetVarDecl> {
        let start = self.cur_span();
        self.bump(); // `virtual`
        let _ = self.eat_kw(Kw::Interface); // optional `interface` keyword
        let iface = self.ident()?;
        let mut names = Vec::new();
        loop {
            let Some(name) = self.ident() else { break };
            names.push(DeclName {
                span: name.span,
                name,
                unpacked: Vec::new(),
                init: None,
            });
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semi, "';' after a virtual interface declaration");
        Some(NetVarDecl {
            kind: NetVarKind::VirtualIface,
            signed: false,
            range: None,
            packed: Vec::new(),
            delay: None,
            names,
            lifetime: None,
            class_type: Some(iface),
            class_args: Vec::new(),
            const_param: false,
            span: start.to(self.prev_span()),
        })
    }

    pub(crate) fn parse_class_decl(&mut self) -> Option<ClassDecl> {
        let start = self.cur_span();
        self.bump(); // 'class'
        let name = self.ident()?;
        // ⓑ-breadth (§8.25): `class C #(int W = 8, …)` value parameter list.
        let params = if self.peek() == Some(TokenKind::Hash) {
            self.parse_class_param_list()
        } else {
            Vec::new()
        };
        let extends = if self.eat_kw(Kw::Extends) {
            self.ident()
        } else {
            None
        };
        self.expect(TokenKind::Semi, "';' after class header");
        let mut items = Vec::new();
        while !self.at_eof() && !self.at_kw(Kw::Endclass) {
            let before = self.pos;
            if let Some(it) = self.parse_class_item() {
                items.push(it);
            }
            if self.pos == before {
                self.bump(); // guarantee forward progress
            }
        }
        self.expect(
            TokenKind::Word(WordKind::Keyword(Kw::Endclass)),
            "'endclass'",
        );
        self.opt_block_label(); // optional `: name`
        Some(ClassDecl {
            name,
            params,
            extends,
            items,
            span: start.to(self.prev_span()),
        })
    }

    /// Parse `#( [parameter] [type] NAME [= DEFAULT], … )` class value parameters
    /// (ⓑ-breadth, §8.25). The optional `parameter` keyword and a leading type
    /// (`int`/`logic [W]`/…) are accepted and ignored for layout (the value is what
    /// matters); a missing default is allowed (the spec must then override it).
    pub(crate) fn parse_class_param_list(&mut self) -> Vec<ClassParam> {
        self.bump(); // `#`
        let mut params = Vec::new();
        if !self.expect(TokenKind::LParen, "'(' after `#` in a class parameter list") {
            return params;
        }
        if self.peek() != Some(TokenKind::RParen) {
            loop {
                let _ = self.eat_kw(Kw::Parameter); // optional `parameter`
                                                    // optional leading type: a net/var kind keyword + optional range.
                if let Some(k) = self.net_var_kind() {
                    self.bump();
                    let _ = self.opt_signed();
                    let r = self.opt_range();
                    // §4.5.156 (§3 全 site): a non-vector class value-param type may not carry
                    // a packed range (`class C #(int [3:0] X)`).
                    self.reject_packed_dims_on_nonvector(k, r.is_some());
                }
                let Some(name) = self.ident() else { break };
                let default = if self.eat(TokenKind::Eq) {
                    Some(self.expr(0))
                } else {
                    None
                };
                params.push(ClassParam { name, default });
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "')' to close the class parameter list");
        params
    }

    /// One class member: `[virtual] function/task …`, a data member `T name;`,
    /// or a loud-rejected deferred qualifier (`rand`/`static`/…). An optional
    /// leading `local`/`protected` access qualifier (IEEE §8.18) rides as the
    /// member `Visibility` (the default is public).
    pub(crate) fn parse_class_item(&mut self) -> Option<ClassItem> {
        if self.at_lex_error() {
            let s = self.cur_span();
            self.bump();
            return Some(ClassItem::Error(s));
        }
        // IEEE §8.18 access control: an optional leading `local`/`protected`
        // qualifier (contextual idents, not lexer keywords). Consume it once; it
        // may precede a `[virtual] function/task` or a plain data member. A
        // duplicate (`local protected x`) is loud — never silently take the last.
        let vis = if self.eat_ident_kw("local") {
            Visibility::Local
        } else if self.eat_ident_kw("protected") {
            Visibility::Protected
        } else {
            Visibility::Public
        };
        if vis != Visibility::Public && (self.at_ident_kw("local") || self.at_ident_kw("protected"))
        {
            self.error("a single `local`/`protected` access qualifier on a class member");
            let s = self.cur_span();
            self.skip_class_item_recover();
            return Some(ClassItem::Error(s));
        }
        let is_virtual = self.eat_kw(Kw::Virtual);
        if self.at_kw(Kw::Function) {
            return Some(ClassItem::Func {
                is_virtual,
                vis,
                def: self.parse_function_def().0,
            });
        }
        if self.at_kw(Kw::Task) {
            return Some(ClassItem::Task {
                is_virtual,
                vis,
                def: self.parse_task_def(),
            });
        }
        if is_virtual {
            self.error("`function` or `task` after `virtual` in a class body");
            let s = self.cur_span();
            self.skip_class_item_recover();
            return Some(ClassItem::Error(s));
        }
        // A `local`/`protected` qualifier combined with `rand`/`randc`/`constraint`
        // is outside this slice (access-controlled randomization) — loud, never a
        // silent drop of the visibility OR the rand-ness.
        if vis != Visibility::Public
            && (self.at_ident_kw("rand")
                || self.at_ident_kw("randc")
                || self.at_ident_kw("constraint"))
        {
            self.error(
                "a plain data member or method after `local`/`protected` \
                 (access-controlled rand/constraint members are outside this slice)",
            );
            let s = self.cur_span();
            self.skip_class_item_recover();
            return Some(ClassItem::Error(s));
        }
        // N7-REST: `rand`/`randc` data member — consume the qualifier, parse the
        // member declaration, and tag it for `randomize()`.
        let randc = self.at_ident_kw("randc");
        if randc || self.at_ident_kw("rand") {
            self.bump(); // the rand/randc qualifier (an Ident, not a lexer keyword)
            let decl = if self.net_var_kind().is_some() {
                self.parse_net_var(false) // class data member: no net delay
            } else if let Some(info) = self.peek_typedef_name() {
                self.parse_typed_decl(info)
            } else {
                self.error("a data member declaration after `rand`/`randc`");
                let s = self.cur_span();
                self.skip_class_item_recover();
                return Some(ClassItem::Error(s));
            };
            return Some(match decl {
                Some(d) => ClassItem::RandProperty { randc, decl: d },
                None => ClassItem::Error(self.prev_span()),
            });
        }
        // N7-REST: `constraint NAME { expr; … }` block.
        if self.at_ident_kw("constraint") {
            return self.parse_constraint();
        }
        // Loud-reject the remaining deferred member qualifiers so they never
        // silently parse as a net type name (N7 MVP: plain data members + methods;
        // `local`/`protected` are handled above, the rest stay deferred — and a
        // `local static x` combo is still loud here since `static` is caught).
        for kw in ["static", "const", "pure", "extern"] {
            if self.at_ident_kw(kw) {
                self.error(
                    "a plain data member or method (N7 MVP does not support \
                     rand/randc/static/const/constraint/pure/extern class members)",
                );
                let s = self.cur_span();
                self.skip_class_item_recover();
                return Some(ClassItem::Error(s));
            }
        }
        // Data member: a net/var declaration, a typedef-name, or a class-typed
        // handle (registered as a `NetVarKind::Class` alias in the prescan). The
        // leading `local`/`protected` access qualifier rides as `vis`.
        if self.net_var_kind().is_some() {
            return self
                .parse_net_var(false) // class property: no net delay
                .map(|d| ClassItem::Property(vis, d));
        }
        if let Some(info) = self.peek_typedef_name() {
            return self
                .parse_typed_decl(info)
                .map(|d| ClassItem::Property(vis, d));
        }
        self.error("class member (data member or `function`/`task` method)");
        let s = self.cur_span();
        self.skip_class_item_recover();
        Some(ClassItem::Error(s))
    }

    /// `constraint NAME { constraint_expr ; … }` (N7-REST). The `constraint`
    /// qualifier is the current token. Each body item is a boolean expression
    /// terminated by `;`; unsupported forms (`inside`/`dist`/`->`) parse-fail loud,
    /// and elaborate loud-rejects any expr it cannot fold to a per-field bound.
    pub(crate) fn parse_constraint(&mut self) -> Option<ClassItem> {
        let start = self.cur_span();
        self.bump(); // `constraint` (an Ident)
        let Some(name) = self.ident() else {
            self.error("a constraint name after `constraint`");
            let s = self.cur_span();
            self.skip_class_item_recover();
            return Some(ClassItem::Error(s));
        };
        self.expect(TokenKind::LBrace, "'{' to open a constraint block");
        let mut exprs = Vec::new();
        let mut soft = Vec::new();
        while self.peek() != Some(TokenKind::RBrace) && self.peek().is_some() {
            let before = self.pos;
            // optional `soft` qualifier (IEEE §18.5.14) before a constraint expr.
            let is_soft = self.eat_ident_kw("soft");
            let e = self.expr(0);
            exprs.push(e);
            soft.push(is_soft);
            self.expect(TokenKind::Semi, "';' after a constraint expression");
            // Guard against a non-advancing error loop.
            if self.pos == before {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "'}' to close a constraint block");
        Some(ClassItem::Constraint(ConstraintDecl {
            name,
            exprs,
            soft,
            span: start.to(self.prev_span()),
        }))
    }

    /// `obj.randomize() with { … }` postfix: consume `with`, parse the inline
    /// constraint block, and wrap the Call `lhs` into `ExprKind::RandomizeWith`.
    /// `#[inline(never)]` so its locals never inflate the recursive `expr_capped`
    /// frame (the expr-depth cap depends on a small hot frame).
    #[inline(never)]
    pub(crate) fn parse_randomize_with_postfix(&mut self, lhs: Expr) -> Expr {
        self.bump(); // `with`
        let constraints = self.parse_with_constraints();
        let span = lhs.span.to(self.prev_span());
        let (name, args) = match lhs.kind {
            ExprKind::Call { name, args } => (name, args),
            _ => unreachable!("caller gates on ExprKind::Call"),
        };
        Expr {
            kind: ExprKind::RandomizeWith(Box::new(RandomizeWithExpr {
                name,
                args,
                constraints,
            })),
            span,
        }
    }

    /// Dispatch a `with` postfix on a method call: brace ⇒ inline `randomize()
    /// with {…}` (§18.7), paren ⇒ array-method `with (expr)` iterator (§7.12).
    /// `#[inline(never)]` so the hot recursive `expr` frame stays small (the
    /// caller is guarded to brace/paren lookahead).
    #[inline(never)]
    pub(crate) fn parse_with_postfix(&mut self, lhs: Expr) -> Expr {
        if self.peek_at(1) == Some(TokenKind::LBrace) {
            self.parse_randomize_with_postfix(lhs)
        } else {
            self.parse_array_with_postfix(lhs)
        }
    }

    /// Parse `arr.method(opt_iter_var) with (expr)` (IEEE §7.12). `#[inline(never)]`
    /// so it does not enlarge the hot recursive `expr` frame (same depth-cap
    /// discipline as `parse_randomize_with_postfix`). The receiver method call has
    /// already been parsed into `lhs` (an `ExprKind::Call`); we split its path into
    /// receiver + method and capture the optional single bare-ident iterator var.
    #[inline(never)]
    pub(crate) fn parse_array_with_postfix(&mut self, lhs: Expr) -> Expr {
        self.bump(); // `with`
        self.expect(TokenKind::LParen, "'(' to open a `with` iterator clause");
        let with_expr = self.expr(0);
        self.expect(TokenKind::RParen, "')' to close a `with` iterator clause");
        let span = lhs.span.to(self.prev_span());
        let (mut path, args) = match lhs.kind {
            ExprKind::Call { name, args } => (name, args),
            _ => unreachable!("caller gates on ExprKind::Call"),
        };
        // method = last path segment; receiver = the rest.
        let method = path.segments.pop().unwrap_or(Ident {
            name: String::new(),
            span,
        });
        let recv = HierPath {
            span,
            segments: path.segments,
        };
        // A single bare-identifier method arg is the named iterator variable
        // (`find(x) with (x>2)`); anything else means the default `item`.
        let iter_var = match args.as_slice() {
            [Expr {
                kind: ExprKind::Ident(p),
                ..
            }] if p.segments.len() == 1 => Some(p.segments[0].clone()),
            _ => None,
        };
        Expr {
            kind: ExprKind::ArrayMethodWith(Box::new(ArrayMethodWithExpr {
                recv,
                method,
                iter_var,
                with_expr,
            })),
            span,
        }
    }

    /// Parse `{ (constraint_expr ;)* }` after `with` for inline `randomize() with`
    /// (IEEE §18.7). Mirrors the constraint-block body. `soft` inside an inline
    /// `with` is a v1 loud reject (the per-call sidecar carries hard predicates
    /// only — class-level `soft` is unaffected).
    pub(crate) fn parse_with_constraints(&mut self) -> Vec<Expr> {
        self.expect(
            TokenKind::LBrace,
            "'{' to open an inline `with` constraint block",
        );
        let mut exprs = Vec::new();
        while self.peek() != Some(TokenKind::RBrace) && self.peek().is_some() {
            let before = self.pos;
            if self.at_ident_kw("soft") {
                self.error(
                    "`soft` inside inline `randomize() with` is unsupported \
                     (v1: hard constraints only)",
                );
                self.bump();
            }
            let e = self.expr(0);
            exprs.push(e);
            self.expect(TokenKind::Semi, "';' after an inline constraint expression");
            if self.pos == before {
                self.bump();
            }
        }
        self.expect(
            TokenKind::RBrace,
            "'}' to close an inline `with` constraint block",
        );
        exprs
    }

    /// Recover from a malformed class item by skipping to the next `;` or
    /// `endclass` (consuming the `;`), without re-reporting.
    pub(crate) fn skip_class_item_recover(&mut self) {
        while !self.at_eof() && !self.at_kw(Kw::Endclass) {
            let semi = self.peek() == Some(TokenKind::Semi);
            self.bump();
            if semi {
                break;
            }
        }
    }
}
