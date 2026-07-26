//! string_array_route — T1: route a ZERO-BASED ASCENDING fixed `string` array to the
//! DYNAMIC-array representation, so it gains a runtime index / `foreach` / runtime
//! element write / `.size()` that the per-element-net form cannot express.
//!
//! Why routing rather than extending the element-net path: a fixed string array is N
//! distinct `NetKind::String` nets, so a RUNTIME index would have to select among nets
//! — there is no such operation. The dynamic form is one `DynArray` handle whose
//! elements live in the engine heap, where an index is an ordinary runtime value.
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
        && matches!(unpacked[0], ast::Dim::Dyn | ast::Dim::Queue(None))
        && matches!(init.kind, ast::ExprKind::AssignPattern(_))
}

impl Elaborator<'_> {
    /// T1: `Some(n)` iff `dim` declares a ZERO-BASED ASCENDING fixed unpacked
    /// dimension of `n` elements — `string s[n]` or `string s[0:n-1]`.
    ///
    /// **Positive precondition (when it is safe to route):** only this shape may be
    /// routed to the DYNAMIC-array representation, because that representation
    /// numbers its elements `0..n-1` and `foreach` walks them in exactly that order.
    /// A declaration with a NON-ZERO base or a DESCENDING range denotes a different
    /// index space — measured against iverilog, `foreach` over `int a[1:3]` yields
    /// 1,2,3 and over `int a[3:1]` yields 3,2,1 — so routing those would silently
    /// RENUMBER (and, descending, re-order) the index space. They stay on the
    /// per-element-net path, where a runtime index remains loud.
    ///
    /// Fail-closed: a dim this cannot PROVE zero-based-ascending is not routed, so a
    /// non-constant or exotic bound simply keeps today's behaviour.
    pub(crate) fn fixed_string_dim_zero_asc(&mut self, dim: &ast::Dim) -> Option<i64> {
        match dim {
            ast::Dim::Size(e) => {
                let n = self.const_eval_in_scope(e)?;
                (n >= 1).then_some(n)
            }
            ast::Dim::Range(r) => {
                let m = self.const_eval_in_scope(&r.msb)?;
                let l = self.const_eval_in_scope(&r.lsb)?;
                // `[0:n-1]` only. `[n-1:0]` (descending) and any non-zero base decline.
                (m == 0 && l >= 0).then_some(l + 1)
            }
            _ => None,
        }
    }

    /// T1-5: every dimension of a fixed string array, when they are ALL zero-based
    /// ascending — `string s[2]` → `[2]`, `string s[2][3]` → `[2, 3]`.
    ///
    /// A multi-dim array is stored FLAT (row-major) in one container, so the whole
    /// declaration has to qualify: routing with one non-conforming dimension would
    /// renumber that axis silently. `None` ⇒ not routed, and the declaration keeps
    /// whatever it does today (per-element nets for 1-D, loud for multi-dim).
    ///
    /// The element count is capped for the same reason the `'{…}` unroll is: a
    /// declaration like `string s[65536][65536]` would otherwise try to pre-size a
    /// container of 2^32 elements. Declining leaves it loud, not truncated.
    pub(crate) fn fixed_string_dims_zero_asc(&mut self, unpacked: &[ast::Dim]) -> Option<Vec<u32>> {
        const MAX_ROUTED_STR_ELEMS: i64 = 1 << 20;
        if unpacked.is_empty() {
            return None;
        }
        let mut dims = Vec::with_capacity(unpacked.len());
        let mut total: i64 = 1;
        for d in unpacked {
            let n = self.fixed_string_dim_zero_asc(d)?;
            total = total.checked_mul(n)?;
            if total > MAX_ROUTED_STR_ELEMS {
                return None;
            }
            dims.push(u32::try_from(n).ok()?);
        }
        Some(dims)
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
        dims: &[u32],
        ports: &ast::PortList,
        body: &[ast::ModuleItem],
    ) -> bool {
        // Row-major FLAT storage: `string s[2][3]` is one 6-element container, and
        // `s[i][j]` flattens to `s[i*3+j]` at every access. The product cannot overflow
        // — `fixed_string_dims_zero_asc` caps the element count before this is reached.
        let n: i64 = dims.iter().map(|&d| i64::from(d)).product();
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
        // A MULTI-dim declaration takes the `dims.len() > 1` arm unconditionally: the
        // expansion only speaks one dimension, and a nested `'{'{…},'{…}}` would have to
        // be flattened in the same row-major order the accesses use. Leaving it loud is
        // correct-or-loud; silently filling the first N elements would not be.
        if let Some(init) = &decl.init {
            if dims.len() > 1
                || self
                    .fixed_string_array_init_pairs(&decl.name, &decl.unpacked[0], init)
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
        self.fixed_string_dyn.insert(next_id, dims.to_vec());
        let key = self.fq(&decl.name.name);
        self.fixed_string_dyn_key.insert(key, next_id);

        // Pre-size to the declared length. This rides `pending_var_inits` (the t0
        // var-init pre-sweep) rather than being synthesized at the net, because a
        // `DynArray` net carries no length. It is pushed HERE, at the declaration, so
        // it always precedes the element writes the collectors push later — the decl
        // pass runs before `collect_var_init_drivers`, and a block-local's writes are
        // appended after the module-scope list entirely.
        let span = decl.name.span;
        let path = ast::HierPath {
            segments: vec![decl.name.clone()],
            span,
        };
        self.pending_var_inits.push((
            ast::Lvalue::Ident(path),
            ast::Expr {
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
            },
        ));
        true
    }

    /// T1: true iff `net` is a routed fixed string array — fixed-size storage that
    /// merely happens to be dyn-backed, so the resize operations stay LOUD.
    pub(crate) fn is_fixed_string_dyn(&self, net: u32) -> bool {
        self.fixed_string_dyn.contains_key(&net)
    }

    /// T1-5: the per-dimension extents of a routed string array, or `None` if `net` is
    /// not one. A 1-D array has a single entry; the container is always FLAT (row-major)
    /// regardless, so a multi-dim `s[i][j]` is `s[i*n1 + j]`.
    pub(crate) fn fixed_string_dyn_dims(&self, net: u32) -> Option<&[u32]> {
        self.fixed_string_dyn.get(&net).map(|v| v.as_slice())
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
            .fixed_string_dyn_dims(net)
            .is_some_and(|dims| dims.len() > 1);
        if multi {
            self.error(
                MsgCode::ElabUnsupported,
                "a partial index of a multi-dimensional string array selects a whole \
                 row, which has no value surface — index every dimension (`s[i][j]`)",
            );
        }
        multi
    }

    /// T1-5: resolve `base[index]` when it is an element of a routed MULTI-dimensional
    /// string array, returning the handle net and the FLAT (row-major) word index.
    ///
    /// The read and write funnels both bottom out on an `Ident` base, which is all a 1-D
    /// container needs. A multi-dim access nests (`s[i][j]` has the BitSelect `s[i]` as
    /// its base), so this walks the chain to its root and flattens the whole index list
    /// at once — the dimensions are only meaningful together.
    ///
    /// Fail-closed in three ways, each of which would otherwise be a silent-wrong:
    /// a chain whose root is not a routed string array declines (the caller's own logic
    /// runs, unchanged); a PARTIAL index (`s[i]` on a 2-D array) declines rather than
    /// being read as a flat index into the wrong element; and a 1-D array declines here
    /// so it keeps taking the existing single-index path byte-identically.
    /// T1-5: the WRITE twin of `routed_md_string_elem`. Same walk over the `Lvalue`
    /// spelling of the chain, and deliberately the same decline conditions — a read that
    /// flattened while its write did not (or the reverse) would put `s[i][j]` on two
    /// different elements.
    pub(crate) fn routed_md_string_lval(
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
                    let dims = self.fixed_string_dyn_dims(net)?.to_vec();
                    if dims.len() < 2 || idxs.len() != dims.len() {
                        return None;
                    }
                    idxs.reverse();
                    let extents: Vec<(u32, u32)> = dims.iter().map(|&n| (0, n)).collect();
                    let ascending = vec![true; dims.len()];
                    let word = self.flatten_word(&extents, &idxs, &ascending);
                    return Some((net, word));
                }
                _ => return None,
            }
        }
    }

    pub(crate) fn routed_md_string_elem(
        &mut self,
        base: &ast::Expr,
        index: &ast::Expr,
    ) -> Option<(u32, u32)> {
        // Walk outward-to-inward, collecting indices, then reverse: `s[i][j]` visits
        // `j` first.
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
                    let dims = self.fixed_string_dyn_dims(net)?.to_vec();
                    // Only the MULTI-dim shape belongs here, and only a FULL index.
                    if dims.len() < 2 || idxs.len() != dims.len() {
                        return None;
                    }
                    idxs.reverse();
                    // Zero-based ascending by construction — that is the only shape the
                    // declaration routes — so the extents are `(0, size)` and every
                    // dimension counts upward.
                    let extents: Vec<(u32, u32)> = dims.iter().map(|&n| (0, n)).collect();
                    let ascending = vec![true; dims.len()];
                    let word = self.flatten_word(&extents, &idxs, &ascending);
                    return Some((net, word));
                }
                _ => return None,
            }
        }
    }
}
