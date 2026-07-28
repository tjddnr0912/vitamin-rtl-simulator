//! string_array_route — T1: route a fixed `string` array to the DYNAMIC-array
//! representation, so it gains a runtime index / `foreach` / runtime element write /
//! `.size()` that the per-element-net form cannot express.
//!
//! Why routing rather than extending the element-net path: a fixed string array is N
//! distinct `NetKind::String` nets, so a RUNTIME index would have to select among nets
//! — there is no such operation. The dynamic form is one `DynArray` handle whose
//! elements live in the engine heap, where an index is an ordinary runtime value.
//!
//! The container numbers its elements `0..n-1`, which is NOT the declared index space
//! for `string s[1:3]` / `s[3:1]` / `s[2][2]`. The first cut of this therefore routed
//! only zero-based ascending 1-D declarations. What lifted that restriction was not new
//! machinery but APPLYING the map instead of assuming it: `flatten_word` already
//! normalizes `idx - lo` per dim with row-major strides, and `lower_fixed_foreach_step`
//! already walks declared bounds in the declared direction. So the geometry
//! (`StrArrayGeom`) is recorded at the declaration and consulted at every access and at
//! `foreach` — and the zero-based 1-D case still lowers to exactly the same IR it did
//! before (`lo == 0` emits no `Sub`, stride 1 emits no `Mul`).
//!
//! Why it is safe to unify (the ladder only goes UP): measured capability parity across
//! 23 shapes, fixed vs dyn, against iverilog — decl-init, const index, byte select,
//! element `.len()`/`.getc()`/`.toupper()`/`.substr()`, element-to-element copy, function
//! argument, ternary, `$sformatf`, `case`, compare, concat, empty read — agree on every
//! one, and dyn additionally answers runtime index, `foreach`, runtime write and
//! `.size()`. Nothing fixed could do is lost. (Before §4.5.220 that was NOT true: the
//! dyn element byte select `d[0][0]` answered a silent 0 where fixed answered 119, so
//! routing then would have been a trade of one silent-wrong for another.)

use super::*;

/// T1: the declared GEOMETRY of a routed fixed string array — everything the flat
/// container needs to speak the DECLARED index space rather than its own `0..n-1`.
///
/// The container is one row-major block of `product(size)` elements, so every access
/// must map the declared index to a word. That map is exactly `flatten_word`'s: per
/// dim, `word_coord = idx - lo`, strides = suffix products. `desc` never enters the
/// map (any bijection is a valid storage order); it drives only the `foreach` WALK
/// DIRECTION, because IEEE §12.7.3 traverses the declared bounds left-to-right and
/// `string s[3:1]` must yield 3, 2, 1.
///
/// Keeping both in one record is deliberate: they are derived from the same
/// declaration in one pass, and a net that has extents but no direction (or the
/// reverse) is the exact shape of a silent renumbering.
#[derive(Clone, Debug)]
pub(crate) struct StrArrayGeom {
    /// Per unpacked dim, `(lo, size)` — `lo` is the dim's MINIMUM declared index
    /// (`[3:1]` and `[1:3]` both give `(1, 3)`), which is the convention
    /// `flatten_word` normalizes against.
    pub(crate) extents: Vec<(i64, u32)>,
    /// Per unpacked dim: the declaration is DESCENDING (`[3:1]`), so `foreach` walks
    /// that dim from `hi` down to `lo`.
    pub(crate) desc: Vec<bool>,
}

impl StrArrayGeom {
    /// Total element count of the flat container. Cannot overflow — the element cap
    /// in `fixed_string_geom` is applied before a geometry is ever built.
    pub(crate) fn elem_count(&self) -> u64 {
        self.extents.iter().map(|&(_, n)| u64::from(n)).product()
    }
}

/// T1: true iff this declaration is a DYNAMIC string CONTAINER with a `'{…}` decl-init
/// that the t0 var-init flush expands — `string s[] = '{…}` (`new[N]` + element writes)
/// or `string q[$] = '{…}` (one `push_back` per element).
///
/// ONE predicate shared by BOTH decl-init collectors (module-scope
/// `collect_var_init_drivers` and the block-local hoist) so they cannot drift. They
/// drifted once before and silently emptied a block-local `string s[] = '{…}`, and
/// admitting `string q[$]` without updating both did it again: the int-queue twin
/// initialised fine while the string queue came out size 0 with no diagnostic.
///
/// A `{…}` §10.10 unpacked CONCAT is deliberately NOT admitted here even though the
/// flush's `dyn_pattern_elems` accepts one: a string-element `{…}` is loud at the decl
/// gate, and widening this would route it to a silently-empty container instead.
pub(crate) fn is_dyn_string_container_init(unpacked: &[ast::Dim], init: &ast::Expr) -> bool {
    unpacked.len() == 1
        && matches!(unpacked[0], ast::Dim::Dyn | ast::Dim::Queue(_))
        && matches!(init.kind, ast::ExprKind::AssignPattern(_))
}

impl Elaborator<'_> {
    /// T1: `Some((lo, size, descending))` iff `dim` declares a CONSTANT fixed unpacked
    /// dimension — `string s[n]`, `s[1:3]`, `s[3:1]`, `s[-1:1]`.
    ///
    /// **Positive precondition (when it is safe to route):** the routed container is a
    /// flat `0..n-1` block, so a declaration may be routed exactly when its index space
    /// is a known constant interval that can be MAPPED onto that block — every access
    /// normalizing `idx - lo`, and `foreach` walking the declared direction. Both are
    /// existing machinery (`flatten_word`, `lower_fixed_foreach_step`), which is why
    /// the base and direction no longer have to be zero/ascending: the map is applied
    /// rather than assumed.
    ///
    /// A NEGATIVE low bound used to be declined here because `flatten_word` carried its
    /// `lo` as `u32`; it carries `i64` now, so the same "apply the map" argument covers
    /// it and the decline is gone. Only a NON-CONSTANT bound still fails closed — it
    /// cannot be mapped at all.
    fn fixed_string_dim_extent(&self, dim: &ast::Dim) -> Option<(i64, u32, bool)> {
        match dim {
            // `[N]` is the `[0:N-1]` shorthand (IEEE §7.4.2) — always ascending.
            ast::Dim::Size(e) => {
                let n = self.const_eval_in_scope(e)?;
                (n >= 1).then_some(())?;
                Some((0, u32::try_from(n).ok()?, false))
            }
            ast::Dim::Range(r) => {
                let m = self.const_eval_in_scope(&r.msb)?;
                let l = self.const_eval_in_scope(&r.lsb)?;
                let (lo, hi) = if m <= l { (m, l) } else { (l, m) };
                // CHECKED: `hi - lo` overflows i64 on a pathological declaration such as
                // `[4611686018427387904:-4611686018427387905]`. The old negative-bound
                // decline masked that (either endpoint negative → returned early); with
                // negatives admitted, the span itself is what has to fail closed.
                let size = u32::try_from(hi.checked_sub(lo)?.checked_add(1)?).ok()?;
                Some((lo, size, m > l))
            }
            _ => None,
        }
    }

    /// T1-5: the whole declared geometry of a fixed string array — `string s[2]` →
    /// one `(0,2)` dim, `string s[1:2][2:1]` → `[(1,2), (1,2)]` with `desc = [false, true]`.
    ///
    /// A multi-dim array is stored FLAT (row-major) in one container, so the whole
    /// declaration has to qualify: one non-constant dimension and the flat map is
    /// undefined for that axis. `None` ⇒ not routed, and the declaration keeps whatever
    /// it does today (per-element nets for 1-D, loud for multi-dim).
    ///
    /// The element count is capped for the same reason the `'{…}` unroll is: a
    /// declaration like `string s[65536][65536]` would otherwise try to pre-size a
    /// container of 2^32 elements. Declining leaves it loud, not truncated.
    pub(crate) fn fixed_string_geom(&self, unpacked: &[ast::Dim]) -> Option<StrArrayGeom> {
        const MAX_ROUTED_STR_ELEMS: u64 = 1 << 20;
        if unpacked.is_empty() {
            return None;
        }
        let mut extents = Vec::with_capacity(unpacked.len());
        let mut desc = Vec::with_capacity(unpacked.len());
        let mut total: u64 = 1;
        for d in unpacked {
            let (lo, size, dn) = self.fixed_string_dim_extent(d)?;
            total = total.checked_mul(u64::from(size))?;
            if total > MAX_ROUTED_STR_ELEMS {
                return None;
            }
            extents.push((lo, size));
            desc.push(dn);
        }
        Some(StrArrayGeom { extents, desc })
    }

    /// T1: true iff the decl pass created FIXED string-ARRAY storage for `name` in the
    /// current scope — either the per-element nets (`string_array_elems`) or the
    /// ROUTED dynamic-array handle (`fixed_string_dyn`).
    ///
    /// BOTH decl-init collectors (module-scope `collect_var_init_drivers` and the
    /// block-local hoist) gate on this ONE predicate. Keying them off the decl's own
    /// output — rather than each re-deciding the shape — is what keeps the two scopes
    /// from drifting apart, which is exactly how a block-local `string s[] = '{…}`
    /// once ended up silently EMPTY while the identical module-scope decl worked.
    pub(crate) fn has_fixed_string_array_storage(&self, name: &str) -> bool {
        self.string_array_elems.contains_key(&self.fq(name))
            || self
                .dyn_handle(name)
                .is_some_and(|(n, _)| self.fixed_string_dyn.contains_key(&n))
    }

    /// Route one `string <name> [n]` / `[0:n-1]` declaration to a `DynArray` handle.
    ///
    /// Returns `true` when the declaration is fully handled here (routed, or rejected
    /// with a diagnostic); `false` to DECLINE, leaving the caller on the unchanged
    /// per-element-net path. Declining is always safe — it is exactly today's behaviour.
    pub(crate) fn route_fixed_string_array(
        &mut self,
        decl: &ast::DeclName,
        geom: &StrArrayGeom,
        ports: &ast::PortList,
        body: &[ast::ModuleItem],
    ) -> bool {
        // Row-major FLAT storage: `string s[2][3]` is one 6-element container, and
        // `s[i][j]` flattens to `s[i*3+j]` at every access. The product cannot overflow
        // — `fixed_string_geom` caps the element count before this is reached.
        let n: u64 = geom.elem_count();
        // Mirrors the `string s[]` branch: a string container cannot be a port.
        let dir = self.dir_for_name(&decl.name.name, ports, body);
        if dir != ir::PortDir::Internal {
            self.error(
                MsgCode::ElabUnsupported,
                "a string array cannot be a port (outside the v7 scope)",
            );
            return true;
        }
        // Validate the init HERE and let the collectors expand it, which is exactly the
        // division of labour the per-element-net path already uses. The element COUNT
        // check lives inside `fixed_string_array_init_pairs`: without it a
        // `string s[3] = '{"a","b"}` would silently produce a 2-element array (iverilog
        // rejects the mismatch), which is precisely the silent-wrong this routing must
        // not introduce.
        //
        // The pairs are deliberately NOT pushed here. The collectors route them to the
        // list their SCOPE requires — a block-local string init lands in the deferred
        // list so it runs after the module-scope string inits it may read — and pushing
        // from here would both duplicate the writes and flatten that ordering.
        //
        // T1-4: a MULTI-dim declaration takes the SAME expansion. `string_array_init_pairs`
        // descends the nested `'{'{…},'{…}}` against the declared dims and emits fully
        // indexed `s[i][j] = …` writes, so the pattern and the accesses agree on the
        // row-major order by construction rather than by a second implementation.
        if let Some(init) = &decl.init {
            if self
                .string_array_init_pairs(&decl.name, &decl.unpacked, init)
                .is_none()
            {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a string-array initializer is supported only as a \
                     `'{…}` pattern with one element per declared index, \
                     at module or block scope (else assign elements in an \
                     initial block)",
                );
                return true;
            }
        }

        // MANGLED, never the declared name — see `fixed_string_dyn_key`. The bare name
        // must stay free in the module namespace or a block-local of the same name
        // collides with this net instead of getting its own storage.
        let mangled = format!("{}$sad", decl.name.name);
        let next_id = self.nets.len() as u32;
        self.add_net(
            &mangled,
            ir::NetVar {
                kind: ir::NetKind::DynArray,
                width: 0,
                msb: 0,
                lsb: 0,
                signed: false,
                array_len: 0,
                dir: ir::PortDir::Internal,
                init: default_init(ast::NetVarKind::Reg, 1),
            },
        );
        if (self.nets.len() as u32) <= next_id {
            // The name was already taken (a re-declaration); no net was added, so there
            // is nothing to route. Decline rather than mark a net we do not own.
            return false;
        }
        // `string_elem_dyn_nets` is what makes the engine hold the elements as byte
        // strings and fill `new[]` with "" — the routed net MUST join it, or every
        // element would degrade to a bit-vector 0/X.
        self.string_elem_dyn_nets.insert(next_id);
        self.fixed_string_dyn.insert(next_id, geom.clone());
        let key = self.fq(&decl.name.name);
        self.fixed_string_dyn_key.insert(key, next_id);

        // Pre-size to the declared length. This rides the t0 var-init pre-sweep rather
        // than being synthesized at the net, because a `DynArray` net carries no length.
        // It is recorded HERE, at the declaration, so it always precedes the element
        // writes the collectors push later — the decl pass runs before
        // `collect_var_init_drivers`, and a block-local's writes are appended after the
        // module-scope list entirely.
        //
        // Keyed by the DECLARING scope's prefix (see `pending_scoped_presize`): the lvalue
        // below is a bare name, and it must be lowered while that scope is active. Pushing
        // it straight into `pending_var_inits` resolved a generate-scope declaration
        // against the module namespace.
        let span = decl.name.span;
        let path = ast::HierPath {
            segments: vec![decl.name.clone()],
            span,
        };
        let lhs = ast::Lvalue::Ident(path);
        let rhs = ast::Expr {
            kind: ast::ExprKind::New {
                size: Box::new(ast::Expr {
                    kind: ast::ExprKind::IntLit {
                        kind: ast::IntLitKind::Decimal,
                        raw: n.to_string(),
                    },
                    span,
                }),
                src: None,
            },
            span,
        };
        // §4.5.255: inside a `$blk$` scope the pre-size joins the block-local list rather
        // than the per-scope pre-size map, because that list is what orders and replays a
        // block-local scope. It carries the declaration's own offset and is pushed HERE, at
        // the declaration — before `collect_block_local_decl_inits` pushes the element
        // writes, which share that offset — so the stable sort keeps `new[n]` ahead of
        // them. The pre-size map has no flush point of its own for such a scope, and a
        // pre-size left in it is an array that silently stays length 0.
        if self.in_block_local_scope() {
            let lo = decl.name.span.lo;
            self.push_block_local_init(None, lo, lhs, rhs);
        } else {
            let scope = self.cur_prefix.clone();
            let in_gen = self.in_generate_body;
            self.pending_scoped_presize
                .entry(scope)
                .or_default()
                .push((in_gen, lhs, rhs));
        }
        true
    }

    /// r19/T1-4: expand a fixed string-array `'{…}` decl-init into per-element
    /// `(lvalue, expr)` assignments, for ANY number of declared dimensions.
    ///
    /// One dim: `string s[3] = '{"a","b","c"}` → `s[0]="a"; s[1]="b"; s[2]="c"`.
    /// Two dims: `string s[2][2] = '{'{"a","b"},'{"c","d"}}` → `s[0][0]="a"` … — the
    /// nesting is walked in lockstep with the declared dims, so the pattern and the
    /// row-major accesses cannot disagree.
    ///
    /// Pattern element k targets the declared index walking from the LEFT bound toward
    /// the right (IEEE §10.9.1), per dim — so a DESCENDING declaration is NOT `min+k`:
    /// `string s[3:1] = '{"a1","b2","c3"}` fills s[3]="a1", s[2]="b2", s[1]="c3"
    /// (iverilog-confirmed).
    ///
    /// The returned pairs are ordered by ASCENDING NORMALIZED (row-major, `idx-min` per
    /// dim) position regardless of the declaration direction, because iverilog performs
    /// the element assignments in that order and an element initializer may READ a
    /// sibling element mid-fill (`string s[3:1] = '{"AA", peek(), "CC"}` where `peek()`
    /// returns `s[3]`). Only the index↔value mapping follows the pattern order; emitting
    /// the WRITES in pattern order was a silent-wrong on descending bounds.
    ///
    /// `None` ⇒ not an expandable pattern init (wrong element count at some level, a
    /// non-pattern init or a non-pattern at an inner level, a non-constant bound, or a
    /// shape whose element writes this cannot express) and the caller stays LOUD. A
    /// count mismatch is loud on purpose: iverilog rejects it too ("Unpacked array
    /// assignment pattern expects N element(s)").
    pub(crate) fn string_array_init_pairs(
        &self,
        name: &ast::Ident,
        unpacked: &[ast::Dim],
        init: &ast::Expr,
    ) -> Option<Vec<(ast::Lvalue, ast::Expr)>> {
        // Mirrors `ARRAY_PATTERN_UNROLL_CAP`; a larger pattern declines here and louds.
        const FIXED_STR_ARRAY_INIT_CAP: usize = 4096;
        if unpacked.is_empty() {
            return None;
        }
        // The DECLARED bounds per dim, direction preserved (`fixed_dim_bounds`
        // normalizes to (min,max) and would lose the fill order). Signed i64 throughout
        // so the per-element-net path keeps accepting a negative bound (`string s[-1:1]`),
        // which the routed path declines for a different reason (`flatten_word`'s u32 lo).
        let mut bounds: Vec<(i64, i64)> = Vec::with_capacity(unpacked.len());
        let mut total: usize = 1;
        for d in unpacked {
            let (left, right) = match d {
                ast::Dim::Size(e) => {
                    let n = self.const_eval_in_scope(e)?;
                    (n >= 1).then_some((0, n - 1))?
                }
                ast::Dim::Range(r) => (
                    self.const_eval_in_scope(&r.msb)?,
                    self.const_eval_in_scope(&r.lsb)?,
                ),
                _ => return None,
            };
            // Checked: a pathological pair of bounds (`[i64::MAX : i64::MIN]`) overflows
            // a bare subtraction and would PANIC where the decl used to emit a
            // diagnostic.
            let count = usize::try_from(left.checked_sub(right)?.unsigned_abs()).ok()? + 1;
            total = total.checked_mul(count)?;
            if total > FIXED_STR_ARRAY_INIT_CAP {
                return None;
            }
            bounds.push((left, right));
        }
        // A HIERARCHICAL element initializer (`'{u.p, "b2"}`) reads a CHILD instance's
        // string, which is still empty during the parent's t0 pre-sweep — it would
        // render "" with no error. That ordering bug is pre-existing (a scalar
        // `string z = u.p;` is silently empty on this path too, ROADMAP §2), but this
        // expansion must not widen its reach from a loud reject to a silent wrong
        // value, so decline the whole init and let the caller loud. Checked on the
        // WHOLE init tree (`expr_mentions_hier_path` recurses through `AssignPattern`),
        // so an inner nesting level cannot smuggle one in.
        if self.expr_mentions_hier_path(init) {
            return None;
        }
        // (normalized row-major position, index list) per leaf, so the sort below is
        // exactly the 1-D "ascending declared index" rule generalized.
        let mut leaves: Vec<(u64, Vec<i64>)> = Vec::with_capacity(total);
        let mut vals: Vec<&ast::Expr> = Vec::with_capacity(total);
        Self::walk_string_init_pattern(&bounds, init, &mut Vec::new(), 0, &mut leaves, &mut vals)?;
        debug_assert_eq!(leaves.len(), vals.len());
        let mut order: Vec<usize> = (0..leaves.len()).collect();
        order.sort_by_key(|&i| leaves[i].0);
        Some(
            order
                .into_iter()
                .map(|i| (self.indexed_lvalue(name, &leaves[i].1), vals[i].clone()))
                .collect(),
        )
    }

    /// Recursive worker for [`Self::string_array_init_pairs`]: descend `pat` one
    /// declared dim at a time, collecting one leaf per element.
    ///
    /// Every level must be an `AssignPattern` with EXACTLY the declared element count —
    /// a short or long level declines the whole init (iverilog rejects it too), and a
    /// non-pattern where a dim remains declines rather than assigning a row from a
    /// scalar. Only when every dim is consumed is the remaining expression taken as an
    /// element VALUE, so `'{"a","b"}` on a 2-D array cannot fill the first row and drop
    /// the rest.
    #[allow(clippy::too_many_arguments)]
    fn walk_string_init_pattern<'a>(
        bounds: &[(i64, i64)],
        pat: &'a ast::Expr,
        idxs: &mut Vec<i64>,
        depth: usize,
        leaves: &mut Vec<(u64, Vec<i64>)>,
        vals: &mut Vec<&'a ast::Expr>,
    ) -> Option<()> {
        if depth == bounds.len() {
            // Row-major normalized position: `((i0-min0) * n1 + (i1-min1)) * n2 + …`.
            let mut pos: u64 = 0;
            for (k, &(left, right)) in bounds.iter().enumerate() {
                let (min, max) = if left <= right {
                    (left, right)
                } else {
                    (right, left)
                };
                let size = (max - min) as u64 + 1;
                pos = pos * size + (idxs[k] - min) as u64;
            }
            leaves.push((pos, idxs.clone()));
            vals.push(pat);
            return Some(());
        }
        let ast::ExprKind::AssignPattern(elems) = &pat.kind else {
            return None;
        };
        let (left, right) = bounds[depth];
        let count = usize::try_from((left - right).unsigned_abs()).ok()? + 1;
        if elems.len() != count {
            return None;
        }
        let step: i64 = if left <= right { 1 } else { -1 };
        for (k, e) in elems.iter().enumerate() {
            idxs.push(left + step * (k as i64));
            Self::walk_string_init_pattern(bounds, e, idxs, depth + 1, leaves, vals)?;
            idxs.pop();
        }
        Some(())
    }

    /// `name[i0][i1]…` as an `Lvalue`, with each index spelled as a literal.
    ///
    /// A NEGATIVE index must be `-<lit>`, not a decimal literal whose raw text starts
    /// with '-': `parse_int_literal` rejects a non-digit byte, so that raw never folds
    /// and the element write louds with a misleading "requires a constant index".
    fn indexed_lvalue(&self, name: &ast::Ident, idxs: &[i64]) -> ast::Lvalue {
        let span = name.span;
        let mut lv = ast::Lvalue::Ident(ast::HierPath {
            segments: vec![name.clone()],
            span,
        });
        for &idx in idxs {
            let mag = ast::Expr {
                kind: ast::ExprKind::IntLit {
                    kind: ast::IntLitKind::Decimal,
                    raw: idx.unsigned_abs().to_string(),
                },
                span,
            };
            let index = if idx < 0 {
                ast::Expr {
                    kind: ast::ExprKind::Unary {
                        op: ast::UnOp::Minus,
                        operand: Box::new(mag),
                    },
                    span,
                }
            } else {
                mag
            };
            lv = ast::Lvalue::BitSelect {
                base: Box::new(lv),
                index: Box::new(index),
                span,
            };
        }
        lv
    }

    /// T1: true iff `net` is a routed fixed string array — fixed-size storage that
    /// merely happens to be dyn-backed, so the resize operations stay LOUD.
    pub(crate) fn is_fixed_string_dyn(&self, net: u32) -> bool {
        self.fixed_string_dyn.contains_key(&net)
    }

    /// T1-5: the declared geometry of a routed string array, or `None` if `net` is not
    /// one. A 1-D array has a single dim; the container is always FLAT (row-major)
    /// regardless, so a multi-dim `s[i][j]` is `s[(i-lo0)*n1 + (j-lo1)]`.
    pub(crate) fn fixed_string_dyn_geom(&self, net: u32) -> Option<&StrArrayGeom> {
        self.fixed_string_dyn.get(&net)
    }

    /// T1-1/2/3/5: the `(lo, size, descending)` of one unpacked dim of a routed string
    /// array — what `lower_fixed_foreach_step` needs to walk it.
    ///
    /// A routed array is a `DynArray` net, so it is invisible to `net_dim_extents` /
    /// `array_dim_desc` (those describe the flat WORD store). Rather than register it
    /// there — which would also change what the engine-facing `net_dims` table says
    /// about a handle — the walk asks THIS, and the two sources stay disjoint by
    /// construction: a net is either a routed string array or a static array, never both.
    pub(crate) fn routed_foreach_dim(&self, net: u32, dim: u32) -> Option<(i64, u32, bool)> {
        let g = self.fixed_string_dyn.get(&net)?;
        let k = (dim.max(1) - 1) as usize;
        let &(lo, size) = g.extents.get(k)?;
        Some((lo, size, g.desc.get(k).copied().unwrap_or(false)))
    }

    /// T1-5: reject a PARTIAL index of a routed multi-dim string array (`s[0]` where `s`
    /// is `string s[2][2]`). Returns true when it fired.
    ///
    /// Both element funnels reach their single-index arm for this shape, and the flat
    /// container would happily accept the row number as an element number — a silent
    /// read of the wrong element (measured: it printed an empty string at exit 0 while
    /// iverilog rejects the source). There is no value surface for a whole row, so the
    /// honest answer is a reject, and it must be raised at BOTH funnels or a partial
    /// write stays silent while the read is loud.
    pub(crate) fn reject_partial_md_string_index(&mut self, net: u32) -> bool {
        let multi = self
            .fixed_string_dyn_geom(net)
            .is_some_and(|g| g.extents.len() > 1);
        if multi {
            self.error(
                MsgCode::ElabUnsupported,
                "a partial index of a multi-dimensional string array selects a whole \
                 row, which has no value surface — index every dimension (`s[i][j]`)",
            );
        }
        multi
    }

    /// T1: map an index CHAIN on a routed string array to `(handle net, flat word)`.
    ///
    /// ONE funnel for every routed shape — 1-D or multi-dim, zero-based or not,
    /// ascending or descending — because the declared→flat map is a single expression
    /// (`flatten_word` over the declared extents) and splitting it per shape is how the
    /// index space silently renumbers on one path but not the other.
    ///
    /// `idxs` arrives outermost-first (`s[i][j]` collects `j` then `i`) and is reversed
    /// here, so both callers can walk their own chain spelling and hand over the raw
    /// list.
    ///
    /// Fail-closed twice, each otherwise a silent-wrong: a root that is not a routed
    /// string array declines (the caller's own logic runs, unchanged), and an index
    /// COUNT that does not match the dimension count declines — a partial index would
    /// otherwise be read as a flat index into the wrong element.
    ///
    /// Byte-identical for the common case: with one dim, `lo == 0` and stride 1,
    /// `flatten_word` emits neither the `Sub` nor the `Mul`, so `s[k]` on a plain
    /// `string s[N]` lowers to exactly the `lower_index_expr(k)` it did before.
    fn routed_string_word(&mut self, net: u32, mut idxs: Vec<&ast::Expr>) -> Option<u32> {
        let extents = self.fixed_string_dyn_geom(net)?.extents.clone();
        if idxs.len() != extents.len() {
            return None;
        }
        idxs.reverse();
        // `ascending` is EMPTY on purpose. It is `flatten_word`'s PACKED-dim convention
        // (`coord = hi - idx`, little-endian bit numbering); an UNPACKED array has no
        // such convention — any bijection is a legal storage order — so every dim
        // normalizes as `idx - lo` and the declaration's direction is carried by
        // `desc`, which drives the `foreach` walk alone.
        Some(self.flatten_word(&extents, &idxs, &[]))
    }

    /// T1: the WRITE twin of `routed_string_elem`. Same walk over the `Lvalue` spelling
    /// of the chain, into the same `routed_string_word` funnel — a read that flattened
    /// while its write did not (or the reverse) would put `s[i][j]` on two different
    /// elements.
    pub(crate) fn routed_string_lval(
        &mut self,
        base: &ast::Lvalue,
        index: &ast::Expr,
    ) -> Option<(u32, u32)> {
        let mut idxs: Vec<&ast::Expr> = vec![index];
        let mut cur = base;
        loop {
            match cur {
                ast::Lvalue::BitSelect {
                    base: b, index: i, ..
                } => {
                    idxs.push(i);
                    cur = b;
                }
                ast::Lvalue::Ident(p) if p.segments.len() == 1 => {
                    let (net, _) = self.dyn_handle(&p.segments[0].name)?;
                    let word = self.routed_string_word(net, idxs)?;
                    return Some((net, word));
                }
                _ => return None,
            }
        }
    }

    /// T1: resolve `base[index]` when it is an element of a routed string array,
    /// returning the handle net and the FLAT (row-major) word index.
    pub(crate) fn routed_string_elem(
        &mut self,
        base: &ast::Expr,
        index: &ast::Expr,
    ) -> Option<(u32, u32)> {
        // Walk outward-to-inward, collecting indices: `s[i][j]` visits `j` first.
        let mut idxs: Vec<&ast::Expr> = vec![index];
        let mut cur = base;
        loop {
            match &cur.kind {
                ast::ExprKind::BitSelect { base: b, index: i } => {
                    idxs.push(i);
                    cur = b;
                }
                ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                    let (net, _) = self.dyn_handle_read(&p.segments[0].name)?;
                    let word = self.routed_string_word(net, idxs)?;
                    return Some((net, word));
                }
                _ => return None,
            }
        }
    }
}
