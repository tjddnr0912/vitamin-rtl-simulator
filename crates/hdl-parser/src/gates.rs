//! gate primitives — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    // ───────────────────────── GATE: gate-level primitives ─────────────────
    /// Classify the current token as a gate-primitive keyword, if any.
    pub(crate) fn gate_kind(&self) -> Option<GateKind> {
        use GateKind::*;
        match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(k))) => Some(match k {
                Kw::And => And,
                Kw::Or => Or,
                Kw::Nand => Nand,
                Kw::Nor => Nor,
                Kw::Xor => Xor,
                Kw::Xnor => Xnor,
                Kw::Buf => Buf,
                Kw::Not => Not,
                Kw::Bufif0 => Bufif0,
                Kw::Bufif1 => Bufif1,
                Kw::Notif0 => Notif0,
                Kw::Notif1 => Notif1,
                _ => return None,
            }),
            _ => None,
        }
    }

    /// `gate_type [#delay] inst {, inst} ;` where `inst = [name] ( terminals )`.
    /// Desugared to continuous assigns (IEEE 1364 §7): multi-input gates reduce
    /// inputs with the gate op (first terminal = output); buf/not pass/invert the
    /// LAST terminal to every preceding output; bufif/notif drive `out` with the
    /// (inverted) data when the control matches, else `1'bz`. Drive strength is
    /// not modelled (a strength spec parses as terminals ⇒ a natural loud error).
    pub(crate) fn parse_gate_primitive(&mut self, gate: GateKind) -> Option<ContinuousAssign> {
        let start = self.cur_span();
        self.bump(); // gate keyword
        let delay = if self.peek() == Some(TokenKind::Hash) {
            self.parse_delay()
        } else {
            None
        };
        let mut assigns: Vec<(Lvalue, Expr)> = Vec::new();
        loop {
            // optional instance name precedes the terminal list `(`.
            if self.is_ident() {
                let _name = self.ident();
            }
            self.expect(TokenKind::LParen, "'(' before gate terminals");
            let mut terms: Vec<Expr> = Vec::new();
            loop {
                let before = self.pos;
                terms.push(self.expr(0));
                if self.pos == before {
                    self.bump(); // forward-progress guard
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')' after gate terminals");
            self.gate_desugar(gate, &terms, &mut assigns);
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semi, "';' after gate instantiation");
        Some(ContinuousAssign {
            delay,
            assigns,
            span: start.to(self.prev_span()),
            from_gate: true,
        })
    }

    /// Lower one gate instance's terminals into `(output_lvalue, rhs_expr)` pairs.
    pub(crate) fn gate_desugar(
        &mut self,
        gate: GateKind,
        terms: &[Expr],
        out: &mut Vec<(Lvalue, Expr)>,
    ) {
        use GateKind::*;
        let sp = terms
            .first()
            .map(|e| e.span)
            .unwrap_or_else(|| self.cur_span());
        let bin = |op: BinOp, l: Expr, r: Expr| Expr {
            span: l.span.to(r.span),
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(l),
                rhs: Box::new(r),
            },
        };
        let inv = |e: Expr| Expr {
            span: e.span,
            kind: ExprKind::Unary {
                op: UnOp::BitNot,
                operand: Box::new(e),
            },
        };
        // z→x coercion via double bitwise-not: `~~v` is identity for 0/1/x and maps
        // z→x (a gate input's z becomes x, IEEE 1364 §7.3/§7.4). Needed only on the
        // NON-inverting pass-through paths (buf, bufif data); the inverting paths
        // (not/notif via `~`) and the multi-input operator folds already coerce z→x.
        let zx = |e: Expr| inv(inv(e));
        let hi_z = || Expr {
            span: sp,
            kind: ExprKind::IntLit {
                kind: IntLitKind::Sized,
                raw: "1'bz".to_string(),
            },
        };
        match gate {
            And | Or | Nand | Nor | Xor | Xnor => {
                // first terminal = output; the rest fold with the gate operator.
                if terms.len() < 2 {
                    self.error("gate needs an output and at least one input");
                    return;
                }
                let op = match gate {
                    And | Nand => BinOp::BitAnd,
                    Or | Nor => BinOp::BitOr,
                    _ => BinOp::BitXor, // Xor | Xnor
                };
                // 2+ inputs fold through the operator (z→x naturally); a single
                // input has no operator, so coerce its z→x explicitly.
                let mut acc = if terms.len() == 2 {
                    zx(terms[1].clone())
                } else {
                    terms[1].clone()
                };
                for t in &terms[2..] {
                    acc = bin(op, acc, t.clone());
                }
                let rhs = if matches!(gate, Nand | Nor | Xnor) {
                    inv(acc)
                } else {
                    acc
                };
                out.push((self.expr_to_lvalue(terms[0].clone()), rhs));
            }
            Buf | Not => {
                // LAST terminal = input; every preceding terminal is an output.
                if terms.len() < 2 {
                    self.error("buf/not needs at least one output and one input");
                    return;
                }
                let input = terms.last().unwrap().clone();
                for o in &terms[..terms.len() - 1] {
                    let rhs = if matches!(gate, Not) {
                        inv(input.clone()) // ~ already maps z→x
                    } else {
                        zx(input.clone()) // buf pass-through: coerce z→x
                    };
                    out.push((self.expr_to_lvalue(o.clone()), rhs));
                }
            }
            Bufif0 | Bufif1 | Notif0 | Notif1 => {
                // ( out, data, control ): drive (inverted) data when control
                // matches the gate's active level, else high-Z.
                if terms.len() != 3 {
                    self.error("bufif/notif needs exactly (output, data, control)");
                    return;
                }
                let data = if matches!(gate, Notif0 | Notif1) {
                    inv(terms[1].clone()) // ~ already maps z→x
                } else {
                    zx(terms[1].clone()) // bufif data pass-through: coerce z→x
                };
                let ctrl = terms[2].clone();
                // bufif1/notif1 drive on control==1 (then=data, else=Z);
                // bufif0/notif0 drive on control==0 (then=Z, else=data).
                let (then_e, else_e) = if matches!(gate, Bufif1 | Notif1) {
                    (data, hi_z())
                } else {
                    (hi_z(), data)
                };
                let rhs = Expr {
                    span: sp,
                    kind: ExprKind::Ternary {
                        cond: Box::new(ctrl),
                        then_e: Box::new(then_e),
                        else_e: Box::new(else_e),
                    },
                };
                out.push((self.expr_to_lvalue(terms[0].clone()), rhs));
            }
        }
    }
}
