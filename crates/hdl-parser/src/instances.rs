//! instances / overrides — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// `defparam hier.path = expr [, hier.path = expr]* ;` (IEEE §23.10.1) — a
    /// hierarchical parameter override. Each LHS is a hierarchical path whose last
    /// segment names the parameter and whose prefix names the target instance.
    pub(crate) fn parse_defparam(&mut self) -> Option<DefparamItem> {
        let start = self.cur_span();
        self.bump(); // 'defparam'
        let mut assigns = Vec::new();
        loop {
            let Some(path) = self.hier_path() else {
                self.error("a hierarchical parameter path after `defparam`");
                break;
            };
            self.expect(TokenKind::Eq, "'=' in defparam");
            let value = self.expr(0);
            assigns.push((path, value));
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semi, "';' after defparam");
        Some(DefparamItem {
            assigns,
            span: start.to(self.prev_span()),
        })
    }

    // ─────────────────────── module instantiation ───────────────────────
    /// Round-9: `bind <target> <checker> <inst> (.p(sig), …);`. After the
    /// contextual `bind` and the target-module ident, `<checker> <inst> (…);` is
    /// an ORDINARY module instantiation, so `parse_module_instance` (which also
    /// consumes the trailing `;`) is reused wholesale — no bind-specific port
    /// parsing. `None` on a malformed target/checker ident (caller recovers).
    /// Is the cursor on a §23.11 `bind` directive (as opposed to an instantiation of a
    /// module literally NAMED `bind`)? `bind` is a contextual keyword — the lexer has
    /// no `Kw::Bind` — so at MODULE-ITEM position, where a bare ident legitimately
    /// starts an instantiation, the spelling alone does not decide. The two shapes
    /// separate at the THIRD token: a directive is `bind TARGET CHECKER …`, so token 2
    /// is a name (or the `.` of an instance-target path), while an instantiation is
    /// `bind u (…)` / `bind #(…) u (…)` / `bind cg = new;`, where token 1 is `#` or
    /// token 2 is `(`/`=`/`,`/`;`. At SOURCE-UNIT position no such ambiguity exists
    /// (a bare ident there is always an error), which is why the top-level arm still
    /// matches on the keyword alone.
    pub(crate) fn at_bind_directive(&self) -> bool {
        self.at_ident_kw("bind")
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
            )
            && matches!(
                self.peek_at(2),
                Some(TokenKind::Word(WordKind::Ident))
                    | Some(TokenKind::EscapedIdent)
                    | Some(TokenKind::Dot)
            )
    }

    pub(crate) fn parse_bind_decl(&mut self) -> Option<BindDecl> {
        let start = self.cur_span();
        self.bump(); // contextual 'bind'
        let target = self.ident()?;
        // §23.11 `bind_target_instance_list`: `bind top.u1 chk c (…)` names an
        // INSTANCE, not a module type. vita's bind table is keyed by target MODULE
        // name (elaborate wires the checker into every instance of it), so a
        // per-instance target would need a different key and is unimplemented. It used
        // to reach `self.ident()` below and report "expected identifier, found '.'"
        // plus two follow-on lines — a spelling error, for a construct vita simply
        // does not have. Report the real reason once and skip the directive.
        if matches!(self.peek(), Some(TokenKind::Dot) | Some(TokenKind::Colon)) {
            self.error(
                "a target MODULE name — vita does not support a `bind` instance target \
                 (`bind top.u1 chk c(…)` / `bind m : u1 chk c(…)`, IEEE 1800-2017 \
                 §23.11 bind_target_instance_list); a bind on the target MODULE name \
                 attaches the checker to every instance of it",
            );
            return None;
        }
        let checker = self.ident()?;
        let inst = self.parse_module_instance(checker);
        Some(BindDecl {
            target,
            inst,
            span: start.to(self.prev_span()),
        })
    }

    /// Parse a module instantiation, given the already-consumed `module_name`.
    /// Grammar:  module_name [ #(param_overrides) ] inst_body {, inst_body} ;
    /// where     inst_body = inst_name [unpacked_dims] ( port_connections )
    ///
    /// Disambiguation: the caller reaches a bare ident at module-item position
    /// only after every keyword-led item is ruled out; in V2005 module scope a
    /// leading bare identifier can ONLY start an instantiation, so no lookahead
    /// is needed to decide. Gate primitives (and/or/not …) are NOT special-cased
    /// here — they lex as plain idents and so flow through this path; a true
    /// gate-primitive instance has no module body for elaborate to find and is a
    /// DEFERRED limitation (it still recovers as an ordinary instance shape).
    /// Always returns a `ModuleInstance` (recovery is internal: sync via the
    /// terminal `expect(Semi)` + per-list forward-progress guards).
    pub(crate) fn parse_module_instance(&mut self, module_name: Ident) -> ModuleInstance {
        let start = module_name.span;

        // optional parameter override list  #( … )
        let param_overrides = if self.peek() == Some(TokenKind::Hash) {
            self.bump(); // '#'
            self.parse_param_overrides()
        } else {
            Vec::new()
        };

        // one-or-more instance bodies, comma-separated
        let mut instances = Vec::new();
        loop {
            let before = self.pos;
            instances.push(self.parse_instance_item());
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        self.expect(TokenKind::Semi, "';' after instantiation");
        ModuleInstance {
            module_name,
            param_overrides,
            instances,
            span: start.to(self.prev_span()),
        }
    }

    /// Parse `( param_overrides )` after a consumed `#`.
    /// `.NAME(expr)` ⇒ ParamConn::Named ; bare `expr` ⇒ ParamConn::Positional.
    /// The first token being `Dot` selects the named form for the whole list.
    /// An empty `#()` is legal (yields an empty Vec).
    pub(crate) fn parse_param_overrides(&mut self) -> Vec<ParamConn> {
        let mut out = Vec::new();
        if !self.expect(TokenKind::LParen, "'(' after '#'") {
            return out;
        }
        if self.peek() == Some(TokenKind::RParen) {
            self.bump(); // empty `#()`
            return out;
        }
        let named = self.peek() == Some(TokenKind::Dot);
        loop {
            let before = self.pos;
            if named {
                self.parse_named_param_conn_into(&mut out);
            } else if !self.push_type_param_override(&mut out, None, self.cur_span()) {
                // positional override: a single const-expr (never empty)
                out.push(ParamConn::Positional(self.expr(0)));
            }
            if self.pos == before {
                self.bump(); // progress guard
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "')' closing parameter overrides");
        out
    }

    /// `.NAME(expr)` | `.NAME()`  → ParamConn::Named { name, value, span } into
    /// `out` — a TYPE value (§4.5.437, `.T(logic [15:0])`) pushes the two value
    /// overrides a type parameter desugars to.
    pub(crate) fn parse_named_param_conn_into(&mut self, out: &mut Vec<ParamConn>) {
        let start = self.cur_span();
        self.expect(TokenKind::Dot, "'.' in named parameter override");
        let name = self.ident().unwrap_or(Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        self.expect(TokenKind::LParen, "'(' after parameter name");
        if self.push_type_param_override(out, Some(&name), start) {
            self.expect(TokenKind::RParen, "')' after parameter value");
            return;
        }
        let value = if self.peek() == Some(TokenKind::RParen) {
            None // `.W()` — explicitly-empty override
        } else {
            Some(self.expr(0))
        };
        self.expect(TokenKind::RParen, "')' after parameter value");
        out.push(ParamConn::Named {
            name,
            value,
            span: start.to(self.prev_span()),
        });
    }

    /// One instance: inst_name [unpacked_dims] ( port_connections )
    pub(crate) fn parse_instance_item(&mut self) -> InstanceItem {
        let start = self.cur_span();
        let name = self.ident().unwrap_or(Ident {
            name: String::new(),
            span: self.cur_span(),
        });

        // optional instance-array dims: `u_x [3:0] (...)` / `u_x [4] (...)`
        let mut unpacked = Vec::new();
        while self.at_dim_start() {
            match self.parse_dim() {
                Some(d) => unpacked.push(d),
                None => break,
            }
        }

        let conns = self.parse_port_conns();
        InstanceItem {
            name,
            unpacked,
            conns,
            span: start.to(self.prev_span()),
        }
    }

    /// `( … )` port-connection list.
    ///   first element `.NAME(...)`      ⇒ Named
    ///   first element `.*`               ⇒ implicit (DEFERRED: stub → empty Named)
    ///   first element bare expr / empty  ⇒ Positional (empty `()` ⇒ Positional([]))
    pub(crate) fn parse_port_conns(&mut self) -> PortConnList {
        if !self.expect(TokenKind::LParen, "'(' before port connections") {
            // recovered with no '(' — synthesize an empty positional list
            return PortConnList::Positional(Vec::new());
        }
        // empty `()` ⇒ zero-arity positional
        if self.peek() == Some(TokenKind::RParen) {
            self.bump();
            return PortConnList::Positional(Vec::new());
        }
        // named iff the first connection starts with a dot (covers `.p(e)`, the
        // `.p` shorthand, and the `.*` wildcard — all begin with `.`).
        let named = self.peek() == Some(TokenKind::Dot);
        if named {
            let mut conns = Vec::new();
            let mut wildcard = false;
            loop {
                // `.*` wildcard item (Dot then Star — there is no DotStar token).
                if self.peek() == Some(TokenKind::Dot)
                    && self.toks.get(self.pos + 1).map(|t| t.kind) == Some(TokenKind::Star)
                {
                    if wildcard {
                        self.error("a single `.*` per port connection list");
                    }
                    wildcard = true;
                    self.bump(); // '.'
                    self.bump(); // '*'
                } else {
                    let before = self.pos;
                    conns.push(self.parse_named_port_conn());
                    if self.pos == before {
                        self.bump();
                    }
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')' closing port connections");
            PortConnList::Named(conns, wildcard)
        } else {
            // positional: each element is `expr` OR empty (a skipped port → None).
            let mut conns: Vec<Option<Expr>> = Vec::new();
            loop {
                match self.peek() {
                    // an empty slot: `,` or `)` where an expr would start
                    Some(TokenKind::Comma) | Some(TokenKind::RParen) => conns.push(None),
                    _ => conns.push(Some(self.expr(0))),
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')' closing port connections");
            PortConnList::Positional(conns)
        }
    }

    /// `.PORT(expr)` | `.PORT()` | `.PORT`  → PortConn { name, value, span }.
    /// `.PORT()` (explicitly-unconnected) ⇒ value = None.
    /// `.PORT` (no parens, IEEE §23.3.2.3 implicit-named shorthand) ⇒
    /// `.PORT(PORT)` — the port binds to a same-named signal in this scope.
    pub(crate) fn parse_named_port_conn(&mut self) -> PortConn {
        let start = self.cur_span();
        self.expect(TokenKind::Dot, "'.' in named port connection");
        let name = self.ident().unwrap_or(Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        // `.name` shorthand: no `(` ⇒ desugar to `.name(name)`. The synthesized
        // identifier flows through the ordinary named-connection path, so a
        // missing same-named signal becomes a normal (loud) bind error — exactly
        // as iverilog reports it. A bare `.name(...)` is unchanged.
        if self.peek() != Some(TokenKind::LParen) {
            let value = (!name.name.is_empty()).then(|| Expr {
                span: name.span,
                kind: ExprKind::Ident(HierPath {
                    segments: vec![name.clone()],
                    span: name.span,
                }),
            });
            return PortConn {
                name,
                value,
                span: start.to(self.prev_span()),
                implicit_name: true,
            };
        }
        self.expect(TokenKind::LParen, "'(' after port name");
        let value = if self.peek() == Some(TokenKind::RParen) {
            None // `.clk()` — explicitly unconnected
        } else {
            Some(self.expr(0))
        };
        self.expect(TokenKind::RParen, "')' after port expression");
        PortConn {
            name,
            value,
            span: start.to(self.prev_span()),
            implicit_name: false,
        }
    }

    /// Parse a parameterized class handle's `#( expr, expr, … )` specialization
    /// arguments (ⓑ-breadth, §8.25). Positional value args only (named `.W(16)`
    /// is a v1 loud-reject left to elaborate). The leading `#` is at the cursor.
    pub(crate) fn parse_param_override_args(&mut self) -> Vec<Expr> {
        self.bump(); // `#`
        let mut args = Vec::new();
        if self.expect(TokenKind::LParen, "'(' after `#` in a class specialization") {
            if self.peek() != Some(TokenKind::RParen) {
                loop {
                    args.push(self.expr(0));
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RParen, "')' to close class specialization args");
        }
        args
    }
}
