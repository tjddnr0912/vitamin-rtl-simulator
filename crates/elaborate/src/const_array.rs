//! §3 ⑤ ⓔ — constant-context consumers of an array-parameter ELEMENT.
//!
//! `array_const_vals` (GAP-G) captures the element VALUES of a 0-based ascending
//! single-dimension constant array parameter, and `const_eval_in_scope`'s `BitSelect`
//! arm answers a whole-element read `A[i]`. Everything that asks for MORE than the
//! value — a select of the element (`A[1][4:0]`, a struct member `S[1].b`, which the
//! parser has already spelled as a part-select), its bits in a concatenation
//! (`{A[0], A[1]}`), its width (`$bits(A[1][4:0])`), or the array's own geometry
//! (`$size(A)`) — needs the element's DECLARED range and the array's extent, which the
//! value table does not carry. This module is that side table and its readers:
//!
//! * [`ArrayConstMeta`], captured next to the values at the two sites that fill
//!   `array_const_vals` / `pkg_array_const_vals` (module scope and package scope), so
//!   the two maps can never disagree about which arrays exist;
//! * [`Elaborator::const_array_elem_read`] — one element as `(value, meta)`, the shape
//!   both the i64 select fold (`const_select_base`) and the wide bit domain
//!   (`wide_name_bits`) ask for;
//! * [`Elaborator::const_dim_query`] — `$size` / `$left` / `$right` / `$low` / `$high` /
//!   `$increment` / `$dimensions` / `$unpacked_dimensions` in a constant context, over a
//!   captured array parameter or a parameter with a DECLARED range.
//!
//! Every reader DECLINES (→ loud at the binding site) rather than guess: a
//! multi-packed-dimension element (`logic [1:0][3:0] A[2]` — `A[1][0]` names a packed
//! nibble, not bit 0), an out-of-range index, an array the capture does not cover
//! (descending / non-zero base / multi-dim unpacked / an element wider than 64 bits),
//! and an untyped parameter (whose width is value-inferred, the row-14 provenance wall).

use super::*;

/// The geometry of a captured constant array parameter — the part of the declaration
/// the element VALUES do not carry.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ArrayConstMeta {
    /// Element count (the array is `[0:count-1]`, by the capture's own rule).
    pub(crate) count: u32,
    /// The element's declared range: `lo`, width, ascending. For an integer atom
    /// (`int`, `byte`, …) the implicit `[w-1:0]`.
    pub(crate) elem_lo: u32,
    pub(crate) elem_w: u32,
    pub(crate) elem_asc: bool,
    pub(crate) elem_signed: bool,
    /// Number of PACKED dimensions of the element: 0 for an unranged `logic`, 1 for a
    /// single range or an integer atom, more for a multi-packed element. Only 1 lets a
    /// select / bit read be answered at `elem_w`.
    pub(crate) packed_dims: u32,
}

/// Which table a constant array parameter lives in — the one resolution
/// `const_array_vals_of_base` performs, made reusable so the meta twin cannot resolve
/// a base to a DIFFERENT array than the value table did.
pub(crate) enum ConstArrayRef {
    Local(String),
    Pkg(String, String),
}

impl Elaborator<'_> {
    /// The element geometry of one constant array declarator, under the SAME shape
    /// rule as `const_array_elem_vals` (the caller records both or neither).
    pub(crate) fn const_array_elem_meta(
        &mut self,
        d: &ast::NetVarDecl,
        decl: &ast::DeclName,
    ) -> ArrayConstMeta {
        let (w, msb, lsb, signed) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
        let (elem_w, _) = self.const_array_elem_geom(d);
        let has_range = d.range.is_some()
            || matches!(
                d.kind,
                ast::NetVarKind::Integer
                    | ast::NetVarKind::Byte
                    | ast::NetVarKind::Shortint
                    | ast::NetVarKind::Int
                    | ast::NetVarKind::Longint
                    | ast::NetVarKind::Time
            );
        let packed_dims = d.packed.len() as u32 + u32::from(has_range);
        let _ = w;
        ArrayConstMeta {
            count: self.const_array_elem_count(decl),
            elem_lo: msb.min(lsb),
            elem_w,
            elem_asc: msb < lsb,
            elem_signed: signed,
            packed_dims,
        }
    }

    /// Resolve the base of a constant element read to its table entry — the same
    /// three routes, in the same local-wins order, as `const_array_vals_of_base`.
    pub(crate) fn const_array_ref_of_base(&self, base: &ast::Expr) -> Option<ConstArrayRef> {
        match &base.kind {
            ast::ExprKind::Ident(path) if path.segments.len() == 1 => {
                let n = path.segments[0].name.as_str();
                // An inline-function formal or a task output formal SHADOWS the array —
                // the value would resolve to the substitution (`const_select_base`'s rule).
                if self.subst_lookup(n).is_some() || self.out_subst_lookup(n).is_some() {
                    return None;
                }
                // The INNERMOST binding of the name over the combined set the value
                // lookups walk (a constant array is also a net in `symbols`; a
                // generate-scope scalar is in `params`), and it must BE the array —
                // `generate if (1) begin : g localparam int ROT = 99; … ROT[1]` names
                // the inner scalar's bit, not the outer array's element (review B B1:
                // the unguarded walk answered the outer array to the wide and `$size`
                // consumers; the value arm carried the same hole, recorded as GAP-G's
                // root in `const_select_base`, closed here for every consumer).
                if let Some(key) = self.walk_scopes_key(n, |k| {
                    self.array_const_vals.contains_key(k)
                        || self.params.contains_key(k)
                        || self.symbols.contains_key(k)
                }) {
                    if self.array_const_vals.contains_key(&key) {
                        return Some(ConstArrayRef::Local(key));
                    }
                    // A wildcard / explicit import binds a var-ALIAS symbol for the
                    // name; that is the import route below, not a shadowing local.
                    if !self.pkg_var_aliases.contains_key(&key) {
                        return None;
                    }
                }
                if self.local_decl_names.contains(n) || self.lookup_scoped(n).is_some() {
                    return None;
                }
                let akey = self.walk_scopes_key(n, |k| self.pkg_var_aliases.contains_key(k))?;
                let (pkg, _) = self.pkg_var_aliases.get(&akey)?;
                self.pkg_array_const_vals.get(pkg)?.get(n)?;
                Some(ConstArrayRef::Pkg(pkg.clone(), n.to_string()))
            }
            ast::ExprKind::PkgScoped { pkg, name } => {
                self.pkg_array_const_vals.get(&pkg.name)?.get(&name.name)?;
                Some(ConstArrayRef::Pkg(pkg.name.clone(), name.name.clone()))
            }
            _ => None,
        }
    }

    pub(crate) fn const_array_vals_of_ref(&self, r: &ConstArrayRef) -> Option<&Vec<i64>> {
        match r {
            ConstArrayRef::Local(k) => self.array_const_vals.get(k),
            ConstArrayRef::Pkg(p, n) => self.pkg_array_const_vals.get(p)?.get(n),
        }
    }

    fn const_array_meta_of_ref(&self, r: &ConstArrayRef) -> Option<ArrayConstMeta> {
        match r {
            ConstArrayRef::Local(k) => self.array_const_meta.get(k).copied(),
            ConstArrayRef::Pkg(p, n) => self.pkg_array_const_meta.get(p)?.get(n).copied(),
        }
    }

    /// `e` is `A[i]` over a captured constant array → the element's value (as the
    /// capture coerced it: truncated / sign-extended to the element type) and the
    /// array's meta. The index folds the way the value arm folds it. `None` for any
    /// other base, an unfoldable / negative / out-of-range index.
    pub(crate) fn const_array_elem_read(&self, e: &ast::Expr) -> Option<(i64, ArrayConstMeta)> {
        let ast::ExprKind::BitSelect { base, index } = &e.kind else {
            return None;
        };
        let r = self.const_array_ref_of_base(base)?;
        let idx = self.const_eval_in_scope(index)?;
        if idx < 0 {
            return None;
        }
        let v = self
            .const_array_vals_of_ref(&r)?
            .get(idx as usize)
            .copied()?;
        let m = self.const_array_meta_of_ref(&r)?;
        Some((v, m))
    }

    /// An element's bits at its DECLARED width, for the wide bit domain — under the
    /// same three declines as `narrow_param_bits` (a non-zero LSB or an ascending
    /// element reads backwards in a domain that indexes from 0 and carries no
    /// direction), plus a multi-packed element (its `[i]` is a packed slice, not a bit).
    pub(crate) fn const_array_elem_bits(&self, e: &ast::Expr) -> Option<WideBits> {
        let (v, m) = self.const_array_elem_read(e)?;
        if m.packed_dims != 1 || m.elem_lo != 0 || m.elem_asc || !(1..=64).contains(&m.elem_w) {
            return None;
        }
        let cv = ir::ConstVal {
            width: 64,
            signed: m.elem_signed,
            repr: ir::ConstRepr::Numeric,
            bits: ir::BitPacked {
                val: vec![v as u64],
                unk: vec![0],
            },
        };
        Some((
            resize_bits(&cv.bits, 64, m.elem_w, m.elem_signed),
            m.elem_w,
            m.elem_signed,
        ))
    }

    /// §20.7 array query functions in a CONSTANT context. `dims` is the declared
    /// dimension list outermost-first as `(left, right)` pairs, exactly the runtime
    /// `net_dims_desc` order: the unpacked dimension first, then the packed one.
    ///
    /// Answered for: a captured constant array parameter (unpacked `[0:count-1]`,
    /// then its single packed range when it has exactly one); a parameter with a
    /// DECLARED range (`param_sel_range` — provenance-filtered, so an untyped
    /// parameter's value-inferred 32 never becomes a `$size`). Everything else —
    /// a variable, an element, a multi-packed element's inner dimensions — declines.
    /// A dimension index outside the list declines too (the runtime prints `x`; a
    /// constant cannot).
    pub(crate) fn const_dim_query(&self, name: &str, args: &[ast::Expr]) -> Option<i64> {
        let with_dim = matches!(
            name,
            "$size" | "$left" | "$right" | "$low" | "$high" | "$increment"
        );
        let no_dim = matches!(name, "$dimensions" | "$unpacked_dimensions");
        if !(with_dim || no_dim) || args.is_empty() || args.len() > 2 {
            return None;
        }
        let arg = Self::peel_parens(&args[0]);
        // (dims outermost-first, total packed dims, unpacked dims)
        let (dims, packed, unpacked): (Vec<(i64, i64)>, i64, i64) =
            if let Some(r) = self.const_array_ref_of_base(arg) {
                let m = self.const_array_meta_of_ref(&r)?;
                let mut dims = vec![(0i64, i64::from(m.count) - 1)];
                if m.packed_dims == 1 {
                    let (lo, hi) = (
                        i64::from(m.elem_lo),
                        i64::from(m.elem_lo) + i64::from(m.elem_w) - 1,
                    );
                    dims.push(if m.elem_asc { (lo, hi) } else { (hi, lo) });
                }
                (dims, i64::from(m.packed_dims), 1)
            } else if matches!(
                arg.kind,
                ast::ExprKind::Ident(_) | ast::ExprKind::PkgScoped { .. }
            ) {
                let (lo, w, asc) = self.param_sel_range(arg)?;
                let (lo, hi) = (lo, lo + i64::from(w) - 1);
                (vec![if asc { (lo, hi) } else { (hi, lo) }], 1, 0)
            } else {
                return None;
            };
        if name == "$dimensions" {
            return (args.len() == 1).then_some(packed + unpacked);
        }
        if name == "$unpacked_dimensions" {
            return (args.len() == 1).then_some(unpacked);
        }
        let d = if args.len() == 2 {
            self.const_int_selfdet(&args[1])?
        } else {
            1
        };
        if d < 1 || (d as usize) > dims.len() {
            return None;
        }
        let (left, right) = dims[(d - 1) as usize];
        Some(match name {
            "$left" => left,
            "$right" => right,
            "$low" => left.min(right),
            "$high" => left.max(right),
            "$size" => (left - right).abs() + 1,
            "$increment" => {
                if left >= right {
                    1
                } else {
                    -1
                }
            }
            _ => return None,
        })
    }
}

/// The names [`Elaborator::const_dim_query`] answers — shared with the diagnostic
/// that explains a declined fold, so the two lists cannot drift.
pub(crate) fn is_dim_query_name(name: &str) -> bool {
    matches!(
        name,
        "$size"
            | "$left"
            | "$right"
            | "$low"
            | "$high"
            | "$increment"
            | "$dimensions"
            | "$unpacked_dimensions"
    )
}

impl Elaborator<'_> {
    /// The `(width, signed)` an UNTYPED parameter takes from a select initializer
    /// (§6.20.2 through §11.5.1): an element read of a constant array parameter is
    /// the element's declared type; any other bit-select is one unsigned bit; a
    /// part-select is `|msb-lsb|+1` and an indexed part-select its width, both
    /// unsigned, both from the bounds alone (`select_span`, the one place a select's
    /// endpoints are folded). `None` for a non-select or an unfoldable bound — the
    /// caller then keeps its value-inferred tail, exactly as before.
    pub(crate) fn select_init_meta(&self, e: &ast::Expr) -> Option<(u32, bool)> {
        use crate::const_select::SelParts;
        let parts = match &e.kind {
            ast::ExprKind::BitSelect { base, index } => {
                if let Some((_, m)) = self.const_array_elem_read(e) {
                    let _ = base;
                    return (m.elem_w > 0).then_some((m.elem_w, m.elem_signed));
                }
                SelParts::Bit(index)
            }
            ast::ExprKind::PartSelect { msb, lsb, .. } => SelParts::Range(msb, lsb),
            ast::ExprKind::IndexedPart {
                offset, width, dir, ..
            } => SelParts::Indexed(offset, width, *dir),
            _ => return None,
        };
        let (a, b, _) = self.select_span(
            parts,
            &std::collections::BTreeMap::new(),
            &crate::const_fn_width::ConstWidths::new(),
            0,
        )?;
        let w = u32::try_from(a.abs_diff(b)).ok()?.checked_add(1)?;
        Some((w, false))
    }
}

impl Elaborator<'_> {
    /// Is an override expression a SELECT of an array-parameter element
    /// (`A[0][3:0]`, `A[0][7]`, `p::A[1][7-:4]`, parenthesised)? A whole element
    /// `A[0]` is not (its base is the array, not an element). See
    /// `ResolvedOverride::elem_select`.
    pub(crate) fn override_is_elem_select(&self, e: &ast::Expr) -> bool {
        let e = Self::peel_parens(e);
        let base = match &e.kind {
            ast::ExprKind::BitSelect { base, .. }
            | ast::ExprKind::PartSelect { base, .. }
            | ast::ExprKind::IndexedPart { base, .. } => Self::peel_parens(base),
            _ => return false,
        };
        self.const_array_elem_read(base).is_some()
    }
}
