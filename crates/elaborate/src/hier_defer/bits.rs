//! §2 🆕 M ⓒ: the hierarchical-PARAMETER lanes a select or `$bits` needs — a
//! constant base for `u.K[7:0]` / `u.K[0]` / `u.K[b+:w]`, and the deferred width of
//! `$bits(u.X)` for a net or a parameter. Both were loud at every width before this
//! (`resolve_deferred_hier_sel` resolves nets; `lower_bits_fold` lowered a still-
//! deferred read), where both oracles answer.

use super::*;
use crate::const_wide_num::bp_from_limbs;
use crate::literal::{make_const_i64, make_const_u32};
use crate::net_util::resize_bits;

impl Elaborator<'_> {
    /// The hierarchical parameter at `prefix`/`path` as a `ConstVal` at its declared
    /// width — the value a whole read `u.K` materializes (`patch_expr_param_const_w`
    /// for the i64 lane, the wide side map's own const for the >64-bit lane). `None`
    /// when the name is not a parameter; an UNTYPED (value-sized) parameter has no
    /// declared width to select within and declines after a loud message.
    fn hier_param_const(&mut self, prefix: &str, path: &[String]) -> Option<ir::ConstVal> {
        if let Some(cv) = self.hier_lookup_wide_param(prefix, path) {
            return Some(cv);
        }
        let v = self.hier_lookup_param(prefix, path)?;
        match self.hier_lookup_param_meta(prefix, path) {
            Some((w, signed)) if (1..=64).contains(&w) => Some(make_const_i64(v, w, signed)),
            Some((w, signed)) if w > 64 => Some(ir::ConstVal {
                width: w,
                signed,
                repr: ir::ConstRepr::Numeric,
                bits: resize_bits(&bp_from_limbs(vec![v as u64], 64), 64, w, signed),
            }),
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "a bit / part-select of the hierarchical parameter `{}` needs its \
                         declared width, and this parameter is sized from its value (no \
                         range, type or sized literal)",
                        path.join(".")
                    ),
                );
                None
            }
        }
    }

    /// Build a bit / part-select READ of a hierarchical PARAMETER: a `Select` over the
    /// parameter's constant, with the offset normalized against its declared LSB
    /// (`hier_param_range`, recorded for a non-zero-LSB declaration only — an ascending
    /// declaration declines, as the net twin `build_hier_read_part` does). `None` after
    /// a loud message; `Some(None)`-like "not a parameter" is reported by the caller.
    pub(crate) fn build_hier_param_select(&mut self, d: &DeferredHierSelect) -> Option<u32> {
        let cv = self.hier_param_const(&d.prefix, &d.path)?;
        let path = d.path.join(".");
        let (lo, asc) = self
            .hier_resolve(&d.prefix, &d.path, &self.hier_param_range)
            .map_or((0, false), |(lo, _, asc)| (lo, asc));
        if asc {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "a select of the hierarchical parameter `{path}` declared ascending \
                     (`[lo:hi]`) is unsupported — read the whole parameter"
                ),
            );
            return None;
        }
        let (raw_off, width, kind) = match d.part {
            Some(p) => {
                if !d.idx_eids.is_empty() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!("too many indices in hierarchical part-select of `{path}`"),
                    );
                    return None;
                }
                (p.raw_off, p.width, p.kind)
            }
            None => {
                if d.idx_eids.len() != 1 {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "too many indices in hierarchical read of `{path}` (a parameter \
                             takes a single bit-select)"
                        ),
                    );
                    return None;
                }
                let one = self.const_u32_expr(1, 32);
                (d.idx_eids[0], one, ir::SelKind::Bit)
            }
        };
        let cid = self.intern_const(cv);
        let base = self.push_expr(ir::Expr::Const { val: cid });
        let offset = if lo == 0 {
            raw_off
        } else {
            self.norm_sub_k(raw_off, lo as i32)
        };
        Some(self.push_expr(ir::Expr::Select {
            base,
            offset,
            width,
            kind,
        }))
    }

    /// Patch every `$bits(u.X)` placeholder with the width of the hierarchical net
    /// (× its unpacked element count, as the local `$bits(arr)` reads) or parameter
    /// (its declared width; 32 for a value-sized one, as the local `$bits(P)` reads;
    /// the wide side map's width past 64). A string net has no static width and is
    /// loud, as the local twin is; an unresolved name is loud.
    pub(crate) fn resolve_deferred_hier_bits(&mut self) {
        let pending = std::mem::take(&mut self.deferred_hier_bits);
        let ambient = self.cur_span;
        for d in pending {
            self.cur_span = d.span.or(ambient);
            let w: Option<u32> = if let Some(net) = self.hier_lookup(&d.prefix, &d.path) {
                let nv = &self.nets[net as usize];
                if nv.kind == ir::NetKind::String {
                    None
                } else {
                    u32::try_from(u64::from(nv.width.max(1)) * u64::from(nv.array_len.max(1))).ok()
                }
            } else if let Some((w, _)) = self.hier_lookup_param_meta(&d.prefix, &d.path) {
                Some(w)
            } else if self.hier_lookup_param(&d.prefix, &d.path).is_some() {
                Some(32)
            } else {
                self.hier_lookup_wide_param(&d.prefix, &d.path)
                    .map(|cv| cv.width)
            };
            match w {
                Some(w) => {
                    let cid = self.intern_const(make_const_u32(w, 32));
                    if let Some(slot) = self.exprs.get_mut(d.eid as usize) {
                        *slot = ir::Expr::Const { val: cid };
                    }
                }
                None => self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "$bits of hierarchical `{}`: not a net or parameter with a static \
                         width (a string has none; an unknown name is one)",
                        d.path.join(".")
                    ),
                ),
            }
        }
        self.cur_span = ambient;
    }
}
