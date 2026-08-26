//! enum methods — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// `var == val` (logical equality), the condition of an enum-method ternary.
    pub(crate) fn enum_eq(var: &Expr, val: i64, span: Span) -> Expr {
        Expr {
            kind: ExprKind::Binary {
                op: BinOp::Eq,
                lhs: Box::new(var.clone()),
                rhs: Box::new(Self::i64_lit(val, span)),
            },
            span,
        }
    }

    /// `.next`/`.prev` → a ternary chain over the ordered label values. `next`
    /// maps vᵢ→vᵢ₊₁ and wraps the last→first; `prev` maps vᵢ→vᵢ₋₁ and wraps the
    /// first→last (§6.19.5). The default (an out-of-range value) takes the wrap
    /// target, matching the boundary case.
    pub(crate) fn enum_step_chain(
        var: &Expr,
        labels: &[(String, i64)],
        span: Span,
        is_next: bool,
    ) -> Expr {
        let vals: Vec<i64> = labels.iter().map(|(_, v)| *v).collect();
        let n = vals.len();
        // (match_value, result_value) pairs + a default result for any other value.
        let (pairs, default): (Vec<(i64, i64)>, i64) = if is_next {
            (
                (0..n.saturating_sub(1))
                    .map(|i| (vals[i], vals[i + 1]))
                    .collect(),
                vals[0],
            )
        } else {
            let mut p = vec![(vals[0], vals[n - 1])];
            p.extend((1..n).map(|i| (vals[i], vals[i - 1])));
            (p, vals[n - 1])
        };
        pairs
            .iter()
            .rev()
            .fold(Self::i64_lit(default, span), |else_e, (m, r)| Expr {
                kind: ExprKind::Ternary {
                    cond: Box::new(Self::enum_eq(var, *m, span)),
                    then_e: Box::new(Self::i64_lit(*r, span)),
                    else_e: Box::new(else_e),
                },
                span,
            })
    }

    /// If `path` is `var.method` where `var` is a (literal-foldable) enum variable
    /// and `method` ∈ {first,last,num,next,prev,name}, build the §6.19.5 desugar
    /// (constants for first/last/num; ternary chains for next/prev/name). `None`
    /// when it is not such an access — the caller falls through to its normal
    /// path (so `var.bar` on an enum stays a loud undeclared-name error).
    pub(crate) fn enum_method_expr(&mut self, path: &HierPath) -> Option<Expr> {
        if path.segments.len() != 2 {
            return None;
        }
        // Clone the (small) label list up front so the `name` arm can mutate
        // `self.pending_enum_name_fns` without holding a borrow of `self.enum_defs`.
        let ename = self.var_enum.get(&path.segments[0].name)?.clone();
        let labels = self.enum_defs.get(&ename)?.clone();
        if labels.is_empty() {
            return None;
        }
        let span = path.span;
        let var = Expr {
            kind: ExprKind::Ident(HierPath {
                segments: vec![path.segments[0].clone()],
                span,
            }),
            span,
        };
        Some(match path.segments[1].name.as_str() {
            "first" => Self::i64_lit(labels[0].1, span),
            "last" => Self::i64_lit(labels[labels.len() - 1].1, span),
            "num" => Self::i64_lit(labels.len() as i64, span),
            "next" => Self::enum_step_chain(&var, &labels, span, true),
            "prev" => Self::enum_step_chain(&var, &labels, span, false),
            // `.name`/`.name()` → a call to a synthetic string-returning
            // `case(x)` function (generated once per enum type, injected into this
            // container's body at the end). A `string` function returns the EXACT
            // label length in EVERY context (assign AND `$display("%s", …)`); a
            // packed string-literal ternary would pad shorter labels to the widest
            // label's width (a silent-wrong vs iverilog's exact-length dynamic string).
            "name" => {
                let fname = format!("$enum_name${ename}");
                self.pending_enum_name_fns
                    .entry(ename)
                    .or_insert_with(|| Self::build_enum_name_fn(&fname, &labels, span));
                Expr {
                    kind: ExprKind::Call {
                        name: HierPath {
                            segments: vec![Ident { name: fname, span }],
                            span,
                        },
                        args: vec![var],
                    },
                    span,
                }
            }
            _ => return None,
        })
    }

    /// SV §6.19.5 `x.next(N)` / `x.prev(N)` with a CONSTANT step `n_steps`. Builds an
    /// N-step ternary chain over the enum labels — each label maps to the member `N`
    /// positions ahead (`next`) or behind (`prev`), wrapping around the ordered set
    /// (`prev(N)` ≡ a forward step of `len − N`). `None` when `x` is not a
    /// literal-foldable enum variable (the caller then emits a generic Call that
    /// loud-rejects — the same fate as a NON-constant step, correct-or-loud). Kept
    /// separate from the arg-less `enum_method_expr` so the common `.next()` desugar
    /// stays byte-identical.
    pub(crate) fn enum_step_n_expr(
        &self,
        path: &HierPath,
        is_next: bool,
        n_steps: i64,
    ) -> Option<Expr> {
        if path.segments.len() != 2 {
            return None;
        }
        let ename = self.var_enum.get(&path.segments[0].name)?;
        let labels = self.enum_defs.get(ename)?;
        if labels.is_empty() {
            return None;
        }
        let span = path.span;
        let var = Expr {
            kind: ExprKind::Ident(HierPath {
                segments: vec![path.segments[0].clone()],
                span,
            }),
            span,
        };
        let vals: Vec<i64> = labels.iter().map(|(_, v)| *v).collect();
        let n = vals.len() as i64;
        // Forward offset into the ordered label set (0..n). `prev` reverses direction.
        // `rem_euclid` normalizes N ≥ n, N == 0 (identity), and any sign.
        let offset = if is_next {
            n_steps.rem_euclid(n)
        } else {
            (-n_steps).rem_euclid(n)
        };
        // Every valid member maps to `vals[(i+offset) mod n]`; the default (an
        // out-of-set value) mirrors the arg-less chain's wrap target.
        let default = vals[offset as usize];
        Some(
            (0..n)
                .map(|i| (vals[i as usize], vals[(i + offset).rem_euclid(n) as usize]))
                .collect::<Vec<(i64, i64)>>()
                .iter()
                .rev()
                .fold(Self::i64_lit(default, span), |else_e, (m, r)| Expr {
                    kind: ExprKind::Ternary {
                        cond: Box::new(Self::enum_eq(&var, *m, span)),
                        then_e: Box::new(Self::i64_lit(*r, span)),
                        else_e: Box::new(else_e),
                    },
                    span,
                }),
        )
    }

    /// Build the synthetic `function string <fname>(input signed? [63:0] x)` whose
    /// body is `case(x) <val>: return "<label>"; … default: return ""; endcase`.
    /// A 64-bit port holds any `i64` enum value; the sign follows the labels (any
    /// negative ⇒ signed), so a negative label compares correctly.
    pub(crate) fn build_enum_name_fn(
        fname: &str,
        labels: &[(String, i64)],
        span: Span,
    ) -> FunctionDef {
        let port_signed = labels.iter().any(|(_, v)| *v < 0);
        let mk_ident = |n: &str| Ident {
            name: n.to_string(),
            span,
        };
        let x_ref = |s: Span| Expr {
            kind: ExprKind::Ident(HierPath {
                segments: vec![Ident {
                    name: "x".to_string(),
                    span: s,
                }],
                span: s,
            }),
            span: s,
        };
        let ret_str = |raw: String| {
            Box::new(Stmt::Return {
                value: Some(Expr {
                    kind: ExprKind::StrLit {
                        raw: format!("\"{raw}\""),
                    },
                    span,
                }),
                span,
            })
        };
        let mut items: Vec<CaseItem> = labels
            .iter()
            .map(|(lname, v)| CaseItem::Match {
                labels: vec![Self::i64_lit(*v, span)],
                body: ret_str(lname.clone()),
                span,
            })
            .collect();
        items.push(CaseItem::Default {
            body: ret_str(String::new()),
            span,
        });
        FunctionDef {
            automatic: false,
            signed: false,
            range: None,
            ret_type: ParamType::Implicit,
            ret_two_state: false,
            ret_string: true,
            name: mk_ident(fname),
            ports: vec![TfPort {
                dir: PortDir::Input,
                dir_spelling: TfDirSpelling::Declared,
                net_or_var: None,
                signed: port_signed,
                range: Some(Range {
                    msb: Self::i64_lit(63, span),
                    lsb: Self::i64_lit(0, span),
                    span,
                }),
                name: mk_ident("x"),
                unpacked: Vec::new(),
                default: None,
                span,
            }],
            body_decls: Vec::new(),
            body_enums: Vec::new(),
            body: Box::new(Stmt::Case {
                kind: CaseKind::Case,
                scrutinee: x_ref(span),
                items,
                span,
            }),
            span,
        }
    }
}
