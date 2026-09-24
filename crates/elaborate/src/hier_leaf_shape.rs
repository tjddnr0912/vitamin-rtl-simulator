//! The declared shape of a hierarchical PLACEHOLDER, recorded when it is created.
//!
//! A cross-instance read `u.x` lowers to `Signal{net: POISON_NET}` and a
//! cross-instance call `u.f(a)` to `Call{func: POISON_FID}`; both are patched only
//! after every instance exists (`resolve_deferred_hier` /
//! `resolve_deferred_hier_call`). Until then the arena node carries no width, no
//! sign and no real-ness, so every store and bind rule that needs a trusted width
//! kept its verbatim tail: a static function `function [3:0] f4; f4 = u.lv;` with
//! `logic [7:0] lv = 8'hf0` returned `f0` where iverilog and verilator print `0`
//! (s15 matrix cell m_u8 f4).
//!
//! The same downward declaration walk the size-cast lane already asks
//! (`hier_leaf_net` / `hier_leaf_real` / `hier_leaf_func`, see `expr_size_hier.rs`)
//! answers the leaf at creation time. Its answer is kept here, keyed by the
//! placeholder's ExprId, and the shared width/sign/real-ness mirrors
//! (`ir_bits_of`, `expr_self_signed`, `canonical_self_width`, `expr_is_real`)
//! read it the way they read `class_field_widths`. Where the walk declines
//! (generate, upward/absolute paths, instance arrays, `bind`, `defparam`, typed
//! parameters, an ambiguous name), or where the declared width's fold is not
//! exact by construction (`expr_size_hier_exact.rs`), no entry is recorded and
//! the placeholder stays unknown, exactly as before.
//!
//! The resolution passes VERIFY an entry against the net / function they patch
//! in; a disagreement is refused loudly (`verify_hier_net_shape` /
//! `verify_hier_call_shape`), so an entry is never a silent guess.

use super::*;

/// The declared shape of one hierarchical placeholder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HierLeafShape {
    pub(crate) width: u32,
    pub(crate) signed: bool,
    /// A `real` / `realtime` variable (64 bits, signed, real domain).
    pub(crate) real: bool,
}

impl Elaborator<'_> {
    /// Record the declared shape of the hierarchical NET read `path` for the
    /// placeholder `eid`. Only a whole scalar/vector net (no unpacked dimension)
    /// or a scalar real is recorded.
    pub(crate) fn record_hier_net_shape(&mut self, eid: u32, path: &ast::HierPath) {
        let names: Vec<&str> = path.segments.iter().map(|s| s.name.as_str()).collect();
        let shape = match self.hier_leaf_net(path) {
            // A declared width is recorded only when its fold is exact by
            // construction (`expr_size_hier_exact.rs`).
            Some(_) if !self.hier_leaf_width_exact(&names, false) => None,
            Some(h) if h.dims == 0 => Some(HierLeafShape {
                width: h.width.max(1),
                signed: h.signed,
                real: false,
            }),
            Some(_) => None,
            None if self.hier_leaf_real(path) => Some(HierLeafShape {
                width: 64,
                signed: true,
                real: true,
            }),
            None => None,
        };
        if let Some(s) = shape {
            self.hier_placeholder_shape.insert(eid, s);
        }
    }

    /// Record the declared return shape of the hierarchical CALL `path` for the
    /// placeholder `eid`. `hier_leaf_func` answers bit-vector returns only; a
    /// `real` return stays unrecorded.
    pub(crate) fn record_hier_call_shape(&mut self, eid: u32, path: &ast::HierPath) {
        let names: Vec<&str> = path.segments.iter().map(|s| s.name.as_str()).collect();
        if !self.hier_leaf_width_exact(&names, true) {
            return;
        }
        if let Some((width, signed)) = self.hier_leaf_func(path) {
            self.hier_placeholder_shape.insert(
                eid,
                HierLeafShape {
                    width: width.max(1),
                    signed,
                    real: false,
                },
            );
        }
    }

    /// Record the shape of a hierarchical SELECT placeholder (`u.v[3]`,
    /// `u.ia[0]`, `u.v[7:4]`, `u.v[b+:4]`) where it is a fact: `n_idx` index
    /// selects on `path`, then a part-select of width-expression `part` (if any).
    /// A bit of a vector is 1 unsigned bit; an element of a one-dimensional
    /// unpacked array has the declared element shape; a part-select of a vector or
    /// of such an element has its constant width, unsigned. Everything else
    /// (multi-dimensional, dynamic, a width that does not fold, a path the
    /// declaration walk declines) stays unrecorded.
    pub(crate) fn record_hier_sel_shape(
        &mut self,
        eid: u32,
        path: &[String],
        n_idx: usize,
        part: Option<u32>,
    ) {
        let Some(h) = self.hier_leaf_net_names(path) else {
            return;
        };
        let shape = match (part, n_idx, h.dims) {
            (Some(wid), 0, 0) | (Some(wid), 1, 1) => match self.width_edge_u32(wid) {
                Some(w) if w > 0 => (w, false),
                _ => return,
            },
            (None, 1, 0) => (1, false),
            (None, 1, 1) => {
                let names: Vec<&str> = path.iter().map(String::as_str).collect();
                if !self.hier_leaf_width_exact(&names, false) {
                    return;
                }
                (h.width.max(1), h.signed)
            }
            _ => return,
        };
        self.hier_placeholder_shape.insert(
            eid,
            HierLeafShape {
                width: shape.0,
                signed: shape.1,
                real: false,
            },
        );
    }

    /// The node a deferred hierarchical SELECT resolved to must have the shape
    /// recorded for its placeholder; checked on the built node, before it is
    /// copied into the placeholder slot.
    pub(crate) fn verify_hier_sel_shape(&mut self, eid: u32, built: u32, path: &[String]) {
        let Some(s) = self.hier_shape(eid) else {
            return;
        };
        let w = self.ir_bits_of(built);
        let sg = self.expr_self_signed(built);
        if w != Some(s.width) || sg != s.signed {
            let got = match w {
                Some(w) => shape_words(w, sg),
                None => "a value of unknown width".to_string(),
            };
            self.error_hier_shape_mismatch(path, &shape_text(s), &got);
        }
    }

    /// The recorded shape of a placeholder, if any.
    pub(crate) fn hier_shape(&self, eid: u32) -> Option<HierLeafShape> {
        self.hier_placeholder_shape.get(&eid).copied()
    }

    /// Called by `lower_size_cast` on its operand `e` before it reads any width.
    ///
    /// A WIDENING cast of a bare hierarchical CALL whose recorded return is signed
    /// needs a sign fill, and `extend_to`'s fill names the call a second time, so
    /// `cast_extend_signed` withholds it (ROADMAP §2, `16'(f())`). With the
    /// recorded width that route zero-fills: `32'(u.hs(3))` printed `000000fd`
    /// where PRE and both oracles print `fffffffd` (s15 c_call2 c). So such a call
    /// drops its record and keeps the route it had before, for every consumer of
    /// the node (s15 c_call5: every cell byte-identical to PRE).
    pub(crate) fn release_hier_call_for_widening_cast(&mut self, e: u32, n: u32) {
        if let Some(h) = self.hier_shape(e) {
            if h.signed
                && n > h.width
                && matches!(self.exprs.get(e as usize), Some(ir::Expr::Call { .. }))
            {
                self.withdraw_hier_shape(e);
            }
        }
    }

    /// `(any hierarchical placeholder, any recorded as real)` in `eid`'s subtree
    /// — for a consumer that keeps its pre-record route for such a leaf.
    pub(crate) fn subtree_hier_leaves(&self, eid: u32) -> (bool, bool) {
        let (mut any, mut real) = (false, false);
        let mut stack = vec![eid];
        let mut seen = BTreeSet::new();
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            // A recorded slot is hierarchical even after nothing else marks it.
            if let Some(h) = self.hier_placeholder_shape.get(&id) {
                any = true;
                real |= h.real;
                continue;
            }
            match self.exprs.get(id as usize) {
                Some(e) if crate::packed::is_expr_placeholder(e) => any = true,
                Some(ir::Expr::Const { .. }) => {}
                Some(ir::Expr::Signal { word, .. }) => stack.extend(word.iter().copied()),
                Some(ir::Expr::Select {
                    base,
                    offset,
                    width,
                    ..
                }) => stack.extend([*base, *offset, *width]),
                Some(ir::Expr::Concat { parts }) => stack.extend(parts.iter().copied()),
                Some(ir::Expr::Replicate { count, value }) => stack.extend([*count, *value]),
                Some(ir::Expr::Unary { operand, .. }) => stack.push(*operand),
                Some(ir::Expr::Binary { lhs, rhs, .. }) => stack.extend([*lhs, *rhs]),
                Some(ir::Expr::Ternary {
                    cond,
                    then_e,
                    else_e,
                }) => stack.extend([*cond, *then_e, *else_e]),
                Some(ir::Expr::SysFunc { args, .. }) | Some(ir::Expr::Call { args, .. }) => {
                    stack.extend(args.iter().copied())
                }
                // An element-iterator leaf, or an id past the arena: nothing hierarchical.
                Some(ir::Expr::ArrayItem { .. }) | None => {}
            }
        }
        (any, real)
    }

    /// Drop a recorded shape, so the placeholder is unknown again to every
    /// consumer, as before the record existed. The canonical self-width cache may
    /// hold an answer read from the record, so it is cut back to below `eid` and
    /// its placeholder scan restarts there.
    pub(crate) fn withdraw_hier_shape(&mut self, eid: u32) {
        if self.hier_placeholder_shape.remove(&eid).is_some() {
            self.selfw_cache.truncate(eid as usize);
            self.selfw_scan = self.selfw_scan.min(eid);
        }
    }

    /// A hierarchical placeholder whose shape is NOT recorded: the one kind of
    /// node the canonical self-width rule cannot answer yet.
    pub(crate) fn is_unshaped_placeholder(&self, eid: u32, e: &ir::Expr) -> bool {
        crate::packed::is_expr_placeholder(e) && !self.hier_placeholder_shape.contains_key(&eid)
    }

    /// The patched-in net must have the shape recorded at creation; the width,
    /// sign and real-ness consumers already answered from the record.
    pub(crate) fn verify_hier_net_shape(&mut self, eid: u32, net: u32, path: &[String]) {
        let Some(s) = self.hier_shape(eid) else {
            return;
        };
        let Some(nv) = self.nets.get(net as usize) else {
            return;
        };
        let real = matches!(nv.kind, ir::NetKind::Real);
        let ok = if s.real {
            real
        } else {
            !real && nv.width.max(1) == s.width && nv.signed == s.signed
        };
        if !ok {
            let got = if real {
                "real".to_string()
            } else {
                shape_words(nv.width.max(1), nv.signed)
            };
            self.error_hier_shape_mismatch(path, &shape_text(s), &got);
        }
    }

    /// The patched-in function must return the shape recorded at creation.
    pub(crate) fn verify_hier_call_shape(&mut self, eid: u32, fid: u32, path: &[String]) {
        let Some(s) = self.hier_shape(eid) else {
            return;
        };
        let Some(m) = self.func_metas.get(fid as usize) else {
            return;
        };
        let (w, sg) = (m.ret_width.max(1), m.ret_signed);
        if w != s.width || sg != s.signed {
            self.error_hier_shape_mismatch(path, &shape_text(s), &shape_words(w, sg));
        }
    }

    /// A hierarchical read whose recorded shape resolved to something the record
    /// cannot describe (`got`: a parameter, nothing, a dynamic container element).
    pub(crate) fn verify_hier_shape_not_net(&mut self, eid: u32, path: &[String], got: &str) {
        if let Some(s) = self.hier_shape(eid) {
            self.error_hier_shape_mismatch(path, &shape_text(s), got);
        }
    }

    fn error_hier_shape_mismatch(&mut self, path: &[String], declared: &str, got: &str) {
        self.error(
            MsgCode::ElabUnsupported,
            &format!(
                "hierarchical reference `{}`: the declaration walk read it as {declared}, \
                 but it resolved to {got}; the width and sign already used for it would be \
                 wrong, so it is refused",
                path.join(".")
            ),
        );
    }
}

fn shape_words(width: u32, signed: bool) -> String {
    format!(
        "a {width}-bit {} value",
        if signed { "signed" } else { "unsigned" }
    )
}

fn shape_text(s: HierLeafShape) -> String {
    if s.real {
        "a real".to_string()
    } else {
        shape_words(s.width, s.signed)
    }
}
