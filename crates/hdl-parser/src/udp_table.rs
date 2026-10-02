//! UDP table synthesis — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// Combinational-UDP desugar (the original YELLOW #1 path), factored out so the
    /// sequential path can early-return to it after the shared header parse.
    pub(crate) fn build_comb_udp(
        &mut self,
        span: Span,
        name: Ident,
        port_names: Vec<Ident>,
        ordered_inputs: &[Ident],
        rows: &[(Vec<UdpLevSym>, UdpOutSym)],
    ) -> Option<ModuleDecl> {
        let mk_eq = |raw: &str, lhs: Expr| Expr {
            span,
            kind: ExprKind::Binary {
                op: BinOp::CaseEq,
                lhs: Box::new(lhs),
                rhs: Box::new(Expr {
                    span,
                    kind: ExprKind::IntLit {
                        kind: IntLitKind::Sized,
                        raw: raw.to_string(),
                    },
                }),
            },
        };
        let row_cond = |in_syms: &[UdpLevSym]| -> Expr {
            let mut conds: Vec<Expr> = Vec::new();
            for (i, s) in in_syms.iter().enumerate() {
                let in_e = Expr {
                    span,
                    kind: ExprKind::Ident(HierPath {
                        segments: vec![ordered_inputs[i].clone()],
                        span,
                    }),
                };
                match s {
                    UdpLevSym::Zero => conds.push(mk_eq("1'b0", in_e)),
                    UdpLevSym::One => conds.push(mk_eq("1'b1", in_e)),
                    UdpLevSym::X => conds.push(Expr {
                        span,
                        kind: ExprKind::Binary {
                            op: BinOp::LogOr,
                            lhs: Box::new(mk_eq("1'bx", in_e.clone())),
                            rhs: Box::new(mk_eq("1'bz", in_e)),
                        },
                    }),
                    UdpLevSym::Q => {}
                    UdpLevSym::B => conds.push(Expr {
                        span,
                        kind: ExprKind::Binary {
                            op: BinOp::LogOr,
                            lhs: Box::new(mk_eq("1'b0", in_e.clone())),
                            rhs: Box::new(mk_eq("1'b1", in_e)),
                        },
                    }),
                }
            }
            conds
                .into_iter()
                .reduce(|a, b| Expr {
                    span,
                    kind: ExprKind::Binary {
                        op: BinOp::LogAnd,
                        lhs: Box::new(a),
                        rhs: Box::new(b),
                    },
                })
                .unwrap_or(Expr {
                    span,
                    kind: ExprKind::IntLit {
                        kind: IntLitKind::Sized,
                        raw: "1'b1".to_string(),
                    },
                })
        };
        let or_fold = |conds: Vec<Expr>| -> Option<Expr> {
            conds.into_iter().reduce(|a, b| Expr {
                span,
                kind: ExprKind::Binary {
                    op: BinOp::LogOr,
                    lhs: Box::new(a),
                    rhs: Box::new(b),
                },
            })
        };
        let mut zero_conds: Vec<Expr> = Vec::new();
        let mut one_conds: Vec<Expr> = Vec::new();
        for (in_syms, out) in rows {
            match out {
                UdpOutSym::Zero => zero_conds.push(row_cond(in_syms)),
                UdpOutSym::One => one_conds.push(row_cond(in_syms)),
                UdpOutSym::X => {}
            }
        }
        let out_lval = Lvalue::Ident(HierPath {
            segments: vec![port_names[0].clone()],
            span,
        });
        let assign = |raw: &str| Stmt::Blocking {
            lhs: out_lval.clone(),
            delay: None,
            event: None,
            rhs: Expr {
                span,
                kind: ExprKind::IntLit {
                    kind: IntLitKind::Sized,
                    raw: raw.to_string(),
                },
            },
            span,
        };
        let mut inner = assign("1'bx");
        if let Some(any1) = or_fold(one_conds) {
            inner = Stmt::If {
                cond: any1,
                then_s: Box::new(assign("1'b1")),
                else_s: Some(Box::new(inner)),
                span,
            };
        }
        if let Some(any0) = or_fold(zero_conds) {
            inner = Stmt::If {
                cond: any0,
                then_s: Box::new(assign("1'b0")),
                else_s: Some(Box::new(inner)),
                span,
            };
        }
        // A UDP is a PRIMITIVE (IEEE 1364 §29): like a gate, it has an output from the
        // start. So the table is evaluated ONCE in the first time-0 batch, as an
        // `initial`, and again on every input change, as `always @(in1 or …)` — the
        // sequential desugar's shape (`udp.rs`). Not `always @*`: that has no time-0
        // run, and a table whose inputs never change produced `x` (caught by
        // `udp_comb::udp_qmark_matches_z`). Not `always_comb` either, since
        // §4.5.584: its implicit time-0 pass waits for the first `#0` promotion, so a
        // table fed only by a declaration initializer (directly, through an assign,
        // through a port) read `x` in every reader before it — `logic k = 1'b0; inv
        // u(o, k);` read by an `always @(t)` that `initial t = 1;` wakes: iverilog
        // 13.0 and verilator 5.052 print `o=1`, that `always_comb` printed `o=x`.
        // The `initial` runs where the `always_comb` used to (first batch, its
        // process slot). The level block is armed at seeding, so the time-0 settle
        // of an input also wakes it, once, at the front of the batch after the
        // first; it reads the inputs as they are then — unchanged unless the first
        // batch wrote one, when it computes (and its change is the event of) what
        // that write's own wake would compute. Measured, not proven per delta: every
        // UDP cell and UDP test prints the value the `always_comb` desugar printed
        // before §4.5.584, including `initial k = 1;` writing an input the settle
        // had woken (review r2 R2-N2).
        let init = ProceduralBlock {
            kind: ProcKind::Initial,
            sensitivity: None,
            body: Box::new(inner.clone()),
            span,
        };
        let sens: Vec<EventExpr> = ordered_inputs
            .iter()
            .map(|inp| EventExpr {
                edge: Edge::NoEdge,
                expr: Expr {
                    span,
                    kind: ExprKind::Ident(HierPath {
                        segments: vec![inp.clone()],
                        span,
                    }),
                },
                iff: None,
                span,
            })
            .collect();
        let always = ProceduralBlock {
            kind: ProcKind::Always,
            sensitivity: Some(Sensitivity::List(sens)),
            body: Box::new(Stmt::Block {
                label: None,
                decls: Vec::new(),
                stmts: vec![inner],
                span,
            }),
            span,
        };
        let mut ports: Vec<AnsiPort> = Vec::with_capacity(port_names.len());
        ports.push(AnsiPort {
            integral_typedef: false,
            pattern_members: Vec::new(),
            shape_param: None,
            dir: PortDir::Output,
            net_or_var: Some(NetVarKind::Reg),
            signed: false,
            range: None,
            packed: Vec::new(),
            name: port_names[0].clone(),
            unpacked: Vec::new(),
            default: None,
            iface: None,
            span,
        });
        for inp in ordered_inputs {
            ports.push(AnsiPort {
                integral_typedef: false,
                pattern_members: Vec::new(),
                shape_param: None,
                dir: PortDir::Input,
                net_or_var: None,
                signed: false,
                range: None,
                packed: Vec::new(),
                name: inp.clone(),
                unpacked: Vec::new(),
                default: None,
                iface: None,
                span,
            });
        }
        Some(ModuleDecl {
            is_macromodule: false,
            name,
            params: Vec::new(),
            ports: PortList::Ansi(ports),
            body: vec![ModuleItem::Proc(init), ModuleItem::Proc(always)],
            span,
            // Overwritten by the driver from `resolve_module_nettype`; the parser cannot
            // see the stripped directive, so it writes the IEEE default (`wire`).
            nettype_none: false,
        })
    }

    /// Scan a UDP input FIELD (raw concatenated token text) into `n_in` columns.
    /// Each column is one level symbol (`0 1 x ? b`) or one edge spec (`r f p n *`
    /// or a parenthesised `(vw)` pair). Whitespace is ignored.
    pub(crate) fn scan_udp_input_cols(
        field: &str,
        n_in: usize,
    ) -> Result<Vec<UdpInCol>, &'static str> {
        let chars: Vec<char> = field.chars().filter(|c| !c.is_whitespace()).collect();
        let mut cols: Vec<UdpInCol> = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            match c {
                '0' => {
                    cols.push(UdpInCol::Lev(UdpLevSym::Zero));
                    i += 1;
                }
                '1' => {
                    cols.push(UdpInCol::Lev(UdpLevSym::One));
                    i += 1;
                }
                'x' | 'X' => {
                    cols.push(UdpInCol::Lev(UdpLevSym::X));
                    i += 1;
                }
                '?' => {
                    cols.push(UdpInCol::Lev(UdpLevSym::Q));
                    i += 1;
                }
                'b' | 'B' => {
                    cols.push(UdpInCol::Lev(UdpLevSym::B));
                    i += 1;
                }
                'r' | 'R' => {
                    cols.push(UdpInCol::Edge(vec![(UdpEnd::Zero, UdpEnd::One)]));
                    i += 1;
                }
                'f' | 'F' => {
                    cols.push(UdpInCol::Edge(vec![(UdpEnd::One, UdpEnd::Zero)]));
                    i += 1;
                }
                'p' | 'P' => {
                    cols.push(UdpInCol::Edge(vec![
                        (UdpEnd::Zero, UdpEnd::One),
                        (UdpEnd::Zero, UdpEnd::X),
                        (UdpEnd::X, UdpEnd::One),
                    ]));
                    i += 1;
                }
                'n' | 'N' => {
                    cols.push(UdpInCol::Edge(vec![
                        (UdpEnd::One, UdpEnd::Zero),
                        (UdpEnd::One, UdpEnd::X),
                        (UdpEnd::X, UdpEnd::Zero),
                    ]));
                    i += 1;
                }
                '*' => {
                    // `*` = (??) = any change.
                    cols.push(UdpInCol::Edge(vec![(UdpEnd::Q, UdpEnd::Q)]));
                    i += 1;
                }
                '(' => {
                    // parse exactly TWO endpoint chars then ')'.
                    let from = match chars.get(i + 1) {
                        Some(&fc) => Self::udp_endpoint(fc)?,
                        None => return Err("a complete edge pair (vw) in a UDP row"),
                    };
                    let to = match chars.get(i + 2) {
                        Some(&tc) => Self::udp_endpoint(tc)?,
                        None => return Err("a complete edge pair (vw) in a UDP row"),
                    };
                    if chars.get(i + 3) != Some(&')') {
                        return Err("a two-symbol edge pair (vw) closed by ')'");
                    }
                    cols.push(UdpInCol::Edge(vec![(from, to)]));
                    i += 4;
                }
                _ => return Err("a UDP input symbol (0 1 x ? b r f p n * or (vw))"),
            }
        }
        if cols.len() != n_in {
            return Err("a UDP table row with one symbol per input port");
        }
        Ok(cols)
    }

    pub(crate) fn udp_endpoint(c: char) -> Result<UdpEnd, &'static str> {
        match c {
            '0' => Ok(UdpEnd::Zero),
            '1' => Ok(UdpEnd::One),
            'x' | 'X' => Ok(UdpEnd::X),
            '?' => Ok(UdpEnd::Q),
            // `b`, `z`, `*` etc. are NOT legal edge endpoints — loud.
            _ => Err("a UDP edge endpoint (0 1 x ?)"),
        }
    }

    /// Scan the NEXT-state field into a symbol (`0 1 x` or `-`=hold).
    pub(crate) fn scan_udp_next(field: &str) -> Result<UdpNextSym, &'static str> {
        let chars: Vec<char> = field.chars().filter(|c| !c.is_whitespace()).collect();
        match chars.as_slice() {
            ['0'] => Ok(UdpNextSym::Zero),
            ['1'] => Ok(UdpNextSym::One),
            ['x'] | ['X'] => Ok(UdpNextSym::X),
            ['-'] => Ok(UdpNextSym::Hold),
            _ => Err("a UDP next-state symbol (0 1 x -)"),
        }
    }
}
