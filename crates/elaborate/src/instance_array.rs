//! instance arrays — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// Unroll an instance array `child u[L:R](conns)` (IEEE 1364-2005
    /// §12.1.2-3) into N = |L−R|+1 children named `u[idx]`, idx walking the
    /// DECLARED order L→R. Connection rule (iverilog-pinned live 2026-06-12):
    /// a conn whose width equals the child port width P fans out to every
    /// instance; width N×P slices into P-bit chunks with the FIRST-named
    /// index taking the MOST significant chunk (both `[3:0]` and `[0:3]`);
    /// any other width is a loud error.
    ///
    /// v1 cuts (all loud E3009): exactly one constant range; ANSI-port child
    /// (port widths must fold under the instance's param overrides);
    /// non-interface ports; sliced/shared conns must be plain identifiers
    /// (part-select synthesis needs the parent net's declared [msb:lsb]).
    pub(crate) fn elaborate_instance_array(
        &mut self,
        child: &ast::ModuleDecl,
        item: &ast::InstanceItem,
        overrides: &[ResolvedOverride],
        parent_inst: u32,
        map: &ModuleMap<'_>,
    ) {
        const INST_ARRAY_CAP: u64 = 4096;
        let iname = item.name.name.clone();
        // ── range: exactly one constant [msb:lsb] ──
        let range = match item.unpacked.as_slice() {
            [ast::Dim::Range(r)] => r,
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!("instance array `{iname}` needs exactly one [msb:lsb] range"),
                );
                return;
            }
        };
        let (Some(left), Some(right)) = (
            self.const_eval_in_scope(&range.msb),
            self.const_eval_in_scope(&range.lsb),
        ) else {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("instance array `{iname}` range is not a constant"),
            );
            return;
        };
        let n64 = left.abs_diff(right) + 1;
        if n64 > INST_ARRAY_CAP {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("instance array `{iname}` has {n64} elements (cap {INST_ARRAY_CAP})"),
            );
            return;
        }
        let n = n64 as u32;

        // ── child port widths, folded in the CHILD param scope (Fix-1 recipe) ──
        let ast::PortList::Ansi(ports) = &child.ports else {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "instance array `{iname}`: child `{}` has non-ANSI ports (v1: ANSI only)",
                    child.name.name
                ),
            );
            return;
        };
        if ports.iter().any(|p| p.iface.is_some()) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("instance array `{iname}`: interface-typed ports are unsupported"),
            );
            return;
        }
        let saved = self.bind_params(child, overrides);
        let mut port_widths: Vec<(String, u32)> = Vec::with_capacity(ports.len());
        let mut widths_ok = true;
        for p in ports {
            let mut w: u64 = match &p.range {
                Some(r) => {
                    match (
                        self.const_eval_in_scope(&r.msb),
                        self.const_eval_in_scope(&r.lsb),
                    ) {
                        (Some(m), Some(l)) => m.abs_diff(l) + 1,
                        _ => {
                            widths_ok = false;
                            0
                        }
                    }
                }
                None => 1,
            };
            for r in &p.packed {
                match (
                    self.const_eval_in_scope(&r.msb),
                    self.const_eval_in_scope(&r.lsb),
                ) {
                    (Some(m), Some(l)) => w = w.saturating_mul(m.abs_diff(l) + 1),
                    _ => widths_ok = false,
                }
            }
            port_widths.push((p.name.name.clone(), w.min(u32::MAX as u64) as u32));
        }
        self.restore_params(saved);
        if !widths_ok {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("instance array `{iname}`: a child port width does not const-fold"),
            );
            return;
        }
        let port_w = |name: &str| -> Option<u32> {
            port_widths.iter().find(|(n, _)| n == name).map(|x| x.1)
        };

        // ── per-connection slice plan ──
        // Shared = same expr to every instance; Slice carries the parent net's
        // declared (msb, lsb) so chunk part-selects follow its direction.
        enum Plan {
            Open,
            Shared,
            Slice { p: u32, msb: i64, lsb: i64 },
        }
        let conns: Vec<(ast::Ident, Option<&ast::Expr>, ast::Span)> = match &item.conns {
            ast::PortConnList::Named(v, wc) => {
                if *wc {
                    // v1: `.*` across an instance array would need per-element
                    // same-name resolution (and possibly slicing) — keep loud.
                    self.error(
                        MsgCode::ElabUnsupported,
                        "`.*` wildcard on an instance array is not yet supported",
                    );
                }
                v.iter()
                    .map(|pc| (pc.name.clone(), pc.value.as_ref(), pc.span))
                    .collect()
            }
            ast::PortConnList::Positional(v) => v
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let pname = ports.get(i).map(|p| p.name.clone()).unwrap_or(ast::Ident {
                        name: format!("<positional {i}>"),
                        span: item.span,
                    });
                    (pname, e.as_ref(), item.span)
                })
                .collect(),
        };
        let mut plans: Vec<Plan> = Vec::with_capacity(conns.len());
        for (pname, value, _) in &conns {
            let Some(expr) = value else {
                plans.push(Plan::Open);
                continue;
            };
            let Some(p) = port_w(&pname.name) else {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "instance array `{iname}`: no port `{}` on child",
                        pname.name
                    ),
                );
                return;
            };
            // Width the connection ACTUALLY has: plain identifiers only (the
            // slice must be a part-select of a declared net).
            let net_id = match &expr.kind {
                ast::ExprKind::Ident(hp) if hp.segments.len() == 1 => {
                    match self.lookup_net_scoped(&hp.segments[0].name) {
                        Some(id) => id,
                        None => {
                            self.error(
                                MsgCode::ElabUnresolvedName,
                                &format!(
                                    "undeclared net/variable `{}`",
                                    self.fq(&hp.segments[0].name)
                                ),
                            );
                            return;
                        }
                    }
                }
                _ => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "instance array `{iname}`: connection for port `{}` must be a \
                             plain identifier (v1)",
                            pname.name
                        ),
                    );
                    return;
                }
            };
            let nv = &self.nets[net_id as usize];
            let (w, nmsb, nlsb) = (nv.width, nv.msb as i64, nv.lsb as i64);
            if w == p {
                plans.push(Plan::Shared);
            } else if (w as u64) == (n as u64) * (p as u64) {
                plans.push(Plan::Slice {
                    p,
                    msb: nmsb,
                    lsb: nlsb,
                });
            } else {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "instance array `{iname}`: port `{}` width {p} expects a connection \
                         of width {p} or {}, got {w}",
                        pname.name,
                        (n as u64) * (p as u64)
                    ),
                );
                return;
            }
        }

        // ── unroll, slicing MSB-first in declared order ──
        let dec_lit = |v: i64, span: ast::Span| ast::Expr {
            kind: ast::ExprKind::IntLit {
                kind: ast::IntLitKind::Decimal,
                raw: v.to_string(),
            },
            span,
        };
        for k in 0..n {
            let idx = if left >= right {
                left - k as i64
            } else {
                left + k as i64
            };
            let inst_conns: Vec<ast::PortConn> = conns
                .iter()
                .zip(&plans)
                .map(|((pname, value, span), plan)| {
                    let value = match plan {
                        Plan::Open => None,
                        Plan::Shared => value.map(|e| (*e).clone()),
                        Plan::Slice { p, msb, lsb } => {
                            let e = value.expect("Slice plan implies a conn expr");
                            // Chunk k (MSB-first): walk from the net's declared
                            // MSB end toward the LSB end, P bits at a time.
                            let (hi, lo) = if msb >= lsb {
                                let hi = msb - (k as i64) * (*p as i64);
                                (hi, hi - (*p as i64 - 1))
                            } else {
                                // ascending [0:7]: the LEFT index is the MSB.
                                let hi = msb + (k as i64) * (*p as i64);
                                (hi, hi + (*p as i64 - 1))
                            };
                            Some(ast::Expr {
                                kind: ast::ExprKind::PartSelect {
                                    base: Box::new((*e).clone()),
                                    msb: Box::new(dec_lit(hi, *span)),
                                    lsb: Box::new(dec_lit(lo, *span)),
                                },
                                span: *span,
                            })
                        }
                    };
                    ast::PortConn {
                        name: pname.clone(),
                        value,
                        span: *span,
                    }
                })
                .collect();
            let child_path = self.child_prefix(&format!("{iname}[{idx}]"));
            self.elaborate_instance(
                child,
                &child_path,
                Some(parent_inst),
                overrides,
                PortBinding::Named(&inst_conns, false),
                map,
            );
        }
    }

    /// Resolve a `$` (type extreme) / constant range endpoint to its integer value.
    /// `$` clamps to the coverpoint domain extreme (signed: ±2^(W-1); unsigned:
    /// 0..2^W-1, clamped to the i64 domain at W≥63). A non-constant `Val` → `None`.
    pub(crate) fn resolve_range_end(
        &self,
        end: &ast::RangeEnd,
        w: u32,
        signed: bool,
        is_hi: bool,
    ) -> Option<i64> {
        match end {
            ast::RangeEnd::TypeExtreme => {
                let ww = w.min(63);
                Some(if signed {
                    let bits = ww.saturating_sub(1);
                    if is_hi {
                        (1i64 << bits) - 1
                    } else {
                        -(1i64 << bits)
                    }
                } else if is_hi {
                    if ww >= 63 {
                        i64::MAX
                    } else {
                        (1i64 << ww) - 1
                    }
                } else {
                    0
                })
            }
            ast::RangeEnd::Val(e) => self.const_eval_in_scope(e),
        }
    }
}
