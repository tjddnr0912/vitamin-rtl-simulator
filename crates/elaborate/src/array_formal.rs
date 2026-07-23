//! array formals — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// §13.3 UARR: the classified shape of a SUPPORTED unpacked-array subroutine
/// formal (`classify_array_formal`) or frame-LOCAL array. Lowered to an md-packed
/// `[count][elem_w]` frame slot; `ascending` is the FIRST unpacked dim's declared
/// direction, which a formal ACTUAL must match (§7.6 positional copy).
#[derive(Clone)]
pub(crate) struct ArrayFormal {
    /// Total element count = product of every unpacked-dim size (== the single dim for
    /// a 1-D array/formal).
    pub(crate) count: u32,
    pub(crate) elem_w: u32,
    pub(crate) ascending: bool,
    /// G3: the element type is SIGNED (`byte`/`int`/`shortint`/`longint`/`logic
    /// signed [..]`). The md-packed slot is whole-`signed:false`; a whole-element read
    /// `arr[i]` re-stamps `$signed` (`lower_packed_read`) so a negative element reads
    /// as negative instead of zero-extended (silent-wrong). Unsigned ⇒ `false`.
    pub(crate) elem_signed: bool,
    /// §4.5.199/206: per-unpacked-dim `(lo, size, ascending)`, OUTER→INNER. `[(lo, count,
    /// ascending)]` for a 1-D array/formal; N entries for a multi-dim frame-LOCAL array OR a
    /// multi-dim FORMAL (§4.5.202/205). The md-packed slot's `packed_dims` is `(lo, size,
    /// false)` per dim + a trailing `(0, elem_w, false)`, so `flatten_word` maps `m[i][j]` to
    /// bit offset `(i-lo0)*∏inner + (j-lo1)*elem_w` — §4.5.206 threads a NON-ZERO base `lo`
    /// (`m[1:4]`) so the element index normalizes (zero-based ⇒ `lo=0` ⇒ no `Sub`, byte-
    /// identical). A FORMAL's actual must match these dims' lo + size + direction
    /// (`lower_array_actual_packed`); the direction is handled consistently on both sides, so
    /// any (ascending / descending / mixed / non-zero-base, any direction) matching pair copies
    /// element-for-element (§4.5.211: non-zero-base DESCENDING is supported too — `flatten_word`
    /// normalizes `idx-lo` consistently on the pack and the read, so `m[k] == a[k]`; a direction/
    /// base MISMATCH between actual and formal is the real §7.6 guard and stays loud).
    pub(crate) dims: Vec<(u32, u32, bool)>,
}

impl Elaborator<'_> {
    /// §13.3 UARR: lower an unpacked-array ACTUAL (`pick(tbl, …)`) into the packed
    /// `count*elem_w` value bound to an md-packed array FORMAL slot. The actual must
    /// be a bare whole-array net of matching element width and length; anything else
    /// (a select, literal, package-scoped ref, or a shape mismatch) is loud-rejected
    /// (correct-or-loud). Packs `{net[count-1], …, net[0]}` so element index `i`
    /// lands at bit offset `i*elem_w` — the md-packed formal's `[count-1:0][elem_w-1:0]`
    /// layout (element 0 at the LSB).
    pub(crate) fn lower_array_actual_packed(&mut self, a: &ast::Expr, af: &ArrayFormal) -> u32 {
        let (count, elem_w) = (af.count, af.elem_w);
        let net_opt = match &a.kind {
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.lookup_net_scoped(&p.segments[0].name)
            }
            _ => None,
        };
        // §13.3 UARR2 (round-7): the actual is the CALLER's OWN unpacked-array formal
        // (`f(a, i)` forwarding `f`'s formal `a` on to `g`). Such a formal is a single
        // md-packed frame net (array_len 1), NOT a static array, so it does not resolve
        // through the static-array path below. When the caller and callee formals have
        // the SAME shape (count × elem_w) and unpacked-dim direction, the caller
        // formal's whole md-packed value IS exactly the callee slot value (both store
        // element 0 at the LSB), so pass it through as a whole-net read — no per-element
        // repack needed. A shape / direction mismatch is loud (§7.6 copies by position,
        // so opposite directions would silently reverse the elements).
        if let Some(net) = net_opt {
            if let Some(caller_af) = self.frame_arr_formal_meta.get(&net).cloned() {
                // §4.5.202: forward the caller's OWN md-packed array formal as a whole-net
                // value when the per-dim shape (`dims`: size + direction, outer→inner) and
                // element width MATCH — then both slots share the identical
                // `array_formal_ext_dims` flat layout, so the whole value is bit-identical
                // and means the same in the callee. Covers 1-D (byte-identical to the old
                // count/ascending check) AND a matching multi-dim forward; any shape /
                // direction / dim-count mismatch falls to the loud error below.
                if caller_af.dims == af.dims && caller_af.elem_w == af.elem_w {
                    return self.push_expr(ir::Expr::Signal { net, word: None });
                }
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "unpacked-array actual formal ({} × {}b, {}) does not match the \
                         callee formal ({} × {}b, {})",
                        caller_af.count,
                        caller_af.elem_w,
                        if caller_af.ascending {
                            "[0:N-1]"
                        } else {
                            "[N-1:0]"
                        },
                        af.count,
                        af.elem_w,
                        if af.ascending { "[0:N-1]" } else { "[N-1:0]" },
                    ),
                );
                return self.placeholder_expr();
            }
        }
        let Some(net) = net_opt.filter(|&n| self.net_is_static_array(n)) else {
            self.error(
                MsgCode::ElabUnsupported,
                "an unpacked-array formal requires a whole-array actual (a bare array \
                 name; a select, literal, or package-scoped array is unsupported)",
            );
            return self.placeholder_expr();
        };
        let nv = &self.nets[net as usize];
        if nv.width != elem_w || nv.array_len != count {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "unpacked-array actual shape ({} x {}b) does not match the formal \
                     ({} x {}b)",
                    nv.array_len, nv.width, count, elem_w
                ),
            );
            return self.placeholder_expr();
        }
        // The actual must be a SINGLE-dimension unpacked array whose declared
        // direction MATCHES the formal's. `dim_desc` holds `(msb,lsb)` per dim +
        // the unpacked-dim count. A 2-D actual with the same total element count
        // (`m[0:1][0:1]` for `a[0:3]`) is a shape mismatch; a direction mismatch
        // (formal `[3:0]`, actual `[0:3]`) would make the index-based md-packed read
        // disagree with §7.6's POSITIONAL copy (a silent value reversal). Both loud.
        // §4.5.202/206: the actual's per-dim shape (base + size + direction, outer→inner)
        // must MATCH the formal's `dims`. §7.6 copies by POSITION into the md-packed slot, so
        // a differing size, opposite direction (`[0:N-1]` vs `[N-1:0]` — silently reverses a
        // dim), differing base (`[1:4]` vs `[0:3]` — the read normalizes `idx-lo` per side),
        // or dim-count would mis-map elements. For a 1-D formal this is the long-standing
        // direction check; for a multi-dim formal every dim must line up. Read-only, so clone
        // the small dim table out before the error path (which needs `&mut self`).
        let actual_dd = self.dim_desc.get(&net).cloned();
        let shape_ok = match &actual_dd {
            Some((dims, unpacked_n))
                if *unpacked_n == af.dims.len() && dims.len() >= af.dims.len() =>
            {
                af.dims.iter().enumerate().all(|(k, &(flo, fsize, fasc))| {
                    let (m, l) = dims[k];
                    let actual_lo = m.min(l).max(0) as u32;
                    let actual_asc = m < l;
                    let actual_size = (m.abs_diff(l) + 1).min(u32::MAX as u64) as u32;
                    actual_lo == flo && actual_size == fsize && actual_asc == fasc
                })
            }
            _ => false,
        };
        if !shape_ok {
            self.error(
                MsgCode::ElabUnsupported,
                "an unpacked-array actual must match the formal's per-dimension shape and \
                 direction (a differing size, direction, or dimension count would silently \
                 mis-map elements under §7.6 positional copy)",
            );
            return self.placeholder_expr();
        }
        // `Signal.word` is an ExprId (the flat word OFFSET expr), NOT a literal
        // index — read flat position `k` via a `Const k` word expr. Flat words are
        // 0-based POSITIONS (direction/base-independent), so this packs the actual's
        // elements in position order; the md-packed formal reads them back with the
        // same position→offset map. Concat parts[0] is the MSB, so emit the highest
        // position first → position 0 lands at the LSB (`arr[0]` = `[0 +: elem_w]`).
        let parts: Vec<u32> = (0..count)
            .rev()
            .map(|k| {
                let word = self.const_u32_expr(k, 32);
                self.push_expr(ir::Expr::Signal {
                    net,
                    word: Some(word),
                })
            })
            .collect();
        self.push_expr(ir::Expr::Concat { parts })
    }

    /// §4.5.207 / §4.5.210: does the bare whole-array actual `net` MATCH the callee array formal
    /// `af` — same element width, element count, and per-dim base + size + direction? Accepts two
    /// actual kinds: a STATIC array (`byte a[4]`, checked against `dim_desc` + `array_len`) and a
    /// caller's OWN md-packed array FORMAL (`frame_arr_formal_meta`, §4.5.210: a frame task/func
    /// forwards its array formal into a nested hier enable) — a forwarded formal's per-dim `dims`
    /// and `elem_w` must equal the callee's, so the two share the identical `array_formal_ext_dims`
    /// flat layout and the whole md-packed value means the same in the callee. The shared shape
    /// gate for the hierarchical-task copy-IN (`pack_hier_array_actual`) AND the OUTPUT/INOUT
    /// copy-OUT (item 1), so the directions can never disagree on "matching". `&self`.
    pub(crate) fn hier_array_shape_ok(&self, net: u32, af: &ArrayFormal) -> bool {
        // §4.5.210: a forwarded frame ARRAY FORMAL (not a static array) — the whole md-packed
        // net value is the slot value iff the per-dim shape + element width match exactly.
        if let Some(caller_af) = self.frame_arr_formal_meta.get(&net) {
            return caller_af.dims == af.dims && caller_af.elem_w == af.elem_w;
        }
        if !self.net_is_static_array(net) {
            return false;
        }
        let nv = &self.nets[net as usize];
        if nv.width != af.elem_w || nv.array_len != af.count {
            return false;
        }
        match self.dim_desc.get(&net) {
            Some((dims, unpacked_n))
                if *unpacked_n == af.dims.len() && dims.len() >= af.dims.len() =>
            {
                af.dims.iter().enumerate().all(|(k, &(flo, fsize, fasc))| {
                    let (m, l) = dims[k];
                    let actual_lo = m.min(l).max(0) as u32;
                    let actual_asc = m < l;
                    let actual_size = (m.abs_diff(l) + 1).min(u32::MAX as u64) as u32;
                    actual_lo == flo && actual_size == fsize && actual_asc == fasc
                })
            }
            _ => false,
        }
    }

    /// §4.5.207 / §4.5.210: build the callee INPUT array formal's md-packed slot value from a bare
    /// whole-array actual `net` (resolved in the CALLER scope at defer time) — the resolve-time
    /// twin of [`Self::lower_array_actual_packed`]. Two sources, gated by `hier_array_shape_ok`:
    /// a STATIC array is packed element-by-element into a Concat (position 0 at the LSB, matching
    /// the formal's md-packed read); a forwarded frame ARRAY FORMAL (§4.5.210) is passed through
    /// as its WHOLE md-packed net value (both slots share the identical `array_formal_ext_dims`
    /// layout — no repack needed, like `lower_array_actual_packed`'s UARR2 forwarding). Returns
    /// `None` on any shape mismatch — the caller emits the loud diagnostic.
    pub(crate) fn pack_hier_array_actual(&mut self, net: u32, af: &ArrayFormal) -> Option<u32> {
        if !self.hier_array_shape_ok(net, af) {
            return None;
        }
        // §4.5.210: a forwarded frame array formal — the whole md-packed net IS the slot value.
        if !self.net_is_static_array(net) {
            return Some(self.push_expr(ir::Expr::Signal { net, word: None }));
        }
        let parts: Vec<u32> = (0..af.count)
            .rev()
            .map(|k| {
                let word = self.const_u32_expr(k, 32);
                self.push_expr(ir::Expr::Signal {
                    net,
                    word: Some(word),
                })
            })
            .collect();
        Some(self.push_expr(ir::Expr::Concat { parts }))
    }

    /// IEEE §13.3 unpacked-array subroutine formal classification (the md-packed
    /// frame slice). Returns:
    /// - `None` — NOT an array formal (empty `unpacked`); a scalar/vector formal.
    /// - `Some(Ok((count, elem_w, elem_signed)))` — a SUPPORTED single-dim
    ///   zero-based unpacked array of a simple zero-LSB vector (or scalar) element.
    ///   Lowered as an md-packed `[count][elem_w]` frame slot: `arr[i]` reuses the
    ///   md-packed element read/write machinery (§4.5.82/97), the call site packs
    ///   the actual whole-array into a `count*elem_w` concat.
    /// - `Some(Err(reason))` — an array formal OUTSIDE the slice (multi-dim,
    ///   non-zero-based, dynamic/queue/assoc dim, non-const bound, or a non-simple
    ///   element) → loud (correct-or-loud; iverilog also rejects unpacked tf-ports).
    pub(crate) fn classify_array_formal(
        &mut self,
        p: &ast::TfPort,
    ) -> Option<Result<ArrayFormal, &'static str>> {
        // A TfPort has no `packed` field (a formal element is a single vector range),
        // so pass `&[]` — the multi-dim-packed-element reject never fires for formals.
        let cls = self.classify_unpacked_array(
            &p.unpacked,
            p.range.as_ref(),
            &[],
            p.signed,
            p.net_or_var.unwrap_or(ast::NetVarKind::Reg),
        );
        // §4.5.202 (+§4.5.205): a MULTI-DIM unpacked-array FORMAL (`int m[2][2]`,
        // `m[1:0][2:0]`, mixed directions) is supported. Element access `m[i][j]` routes
        // through the N-D md-packed slot (`array_formal_ext_dims` + `flatten_word`), and the
        // call-site pack (`lower_array_actual_packed`) requires the actual's per-dim (size +
        // direction) to MATCH the formal's, so a declared index maps to the same logical
        // element on both sides — forward pass-by-value `m[i][j] = a[i][j]` (§13.5.1), the
        // same as the long-standing 1-D formal. (§4.5.202 first gated DESCENDING multi-dim
        // loud on a mistaken "index-major vs declaration-major reversal" worry; §4.5.205
        // verified against a distinct-value reference — descending / mixed / non-square /
        // 3-D all forward, and a direction MISMATCH is loud in `lower_array_actual_packed`
        // — so the extra gate was removed. iverilog rejects the whole construct, so this is
        // hand-IEEE.) Non-zero-based / dynamic / non-simple-element shapes are still rejected
        // upstream in `classify_unpacked_array`.
        cls
    }

    /// §13.3 UARR slice classifier, shared by array FORMALS and frame-LOCAL unpacked
    /// arrays (a frame local reuses the same md-packed `[count][elem_w]` lowering). The
    /// caller supplies the raw fields (`classify_array_formal` from a `TfPort`, the
    /// frame-local reservation from a `NetVarDecl` + per-name unpacked dims). `packed`
    /// is the ADDITIONAL packed dims of a local (`logic [3:0][7:0] arr[]` ⇒ `[[7:0]]`);
    /// a formal passes `&[]`. See the SUPPORTED/`Err` contract on `reserve_frame_func`.
    pub(crate) fn classify_unpacked_array(
        &mut self,
        unpacked: &[ast::Dim],
        range: Option<&ast::Range>,
        packed: &[ast::Range],
        signed: bool,
        kind: ast::NetVarKind,
    ) -> Option<Result<ArrayFormal, &'static str>> {
        if unpacked.is_empty() {
            return None;
        }
        // A multi-dim PACKED element (`logic [3:0][7:0] arr[0:2]`) is out of the
        // simple-element slice: the md-packed slot encodes only `[count][elem_w]`, so a
        // sub-element select `arr[k][j]` would mis-lower to a bit select (silent-wrong).
        // Loud-reject (a formal never reaches here — its `packed` is always `&[]`).
        if !packed.is_empty() {
            return Some(Err(
                "a multi-dimensional packed element combined with an unpacked array",
            ));
        }
        // §4.5.199: EVERY unpacked dim must be a zero-based CONSTANT (`[N]` or
        // `[0:N-1]`/`[N-1:0]`), so each index i_k maps to a fixed stride in the md-packed
        // slot. A single dim is the common (formal + 1-D local) case; 2+ dims are a
        // multi-dim frame-LOCAL array — `array_formal_ext_dims` emits one `packed_dims`
        // entry per dim + the elem_w entry, so `flatten_word` handles `m[i][j]` with the
        // existing N-D offset math. A multi-dim FORMAL is loud-rejected downstream
        // (`classify_array_formal`); the formal binding packs a single dimension.
        let mut dims: Vec<(u32, u32, bool)> = Vec::with_capacity(unpacked.len());
        for dim in unpacked {
            let (lo, sz, asc) = match dim {
                // `[N]` == `[0:N-1]` (zero-based ascending by construction).
                ast::Dim::Size(e) => match self.const_eval_in_scope(e) {
                    Some(n) if n >= 1 => (0u32, n as u32, true),
                    _ => return Some(Err("a non-constant or non-positive unpacked-array size")),
                },
                ast::Dim::Range(r) => {
                    let (m, l) = match (
                        self.const_eval_in_scope(&r.msb),
                        self.const_eval_in_scope(&r.lsb),
                    ) {
                        (Some(m), Some(l)) => (m, l),
                        _ => return Some(Err("a non-constant unpacked-array bound")),
                    };
                    // §4.5.206/211: negative bounds are always loud. A NON-ZERO base is supported
                    // for ANY direction — `array_formal_ext_dims` sets the packed dim's `lo` so
                    // `flatten_word` normalizes `idx-lo` on BOTH the actual pack (`lower_array_
                    // actual_packed` reads flat words in position order) and the formal read (`m[k]`
                    // → `k-lo`), so `m[k] == a[k]` for a same-range formal (§13.5.1 positional
                    // copy) regardless of direction. §4.5.206 gated non-zero-base DESCENDING (`[4:1]`)
                    // out of caution ("base+direction not cleanly verifiable"), but §4.5.211 verified
                    // the machinery it built already handles it: distinct-digit differentials (1-D
                    // `[4:1]`=4321, non-square 2×3 `[2:1][3:1]`=654321, mixed `[1:2][3:1]`, offset
                    // `[5:2]`, signed, output/inout, hier, frame-local, forward) are all forward. A
                    // direction / base MISMATCH between actual and formal stays loud in
                    // `lower_array_actual_packed` (the real §7.6 positional-copy guard). ZERO-based
                    // (any direction) keeps `lo=0` → no `Sub`, byte-identical.
                    if m < 0 || l < 0 {
                        return Some(Err("a negative unpacked-array bound"));
                    }
                    let lo = m.min(l) as u32;
                    let ascending = m < l;
                    (lo, (m.abs_diff(l) + 1) as u32, ascending)
                }
                ast::Dim::Dyn | ast::Dim::Queue(_) | ast::Dim::Assoc(_) => {
                    return Some(Err("a dynamic / queue / associative formal dimension"))
                }
            };
            dims.push((lo, sz, asc));
        }
        // `count` = total elements (product); saturating so a pathological product cannot
        // panic (the width guard below caps the slot). `ascending` = the FIRST dim's
        // direction (the §7.6 positional-copy check consumes it for 1-D formals only).
        let count: u32 = dims.iter().fold(1u32, |a, &(_, s, _)| a.saturating_mul(s));
        let ascending = dims[0].2;
        // G3: a signed element is supported — the md-packed element read is a part-select
        // (unsigned per §11.5.1), so `lower_packed_read` re-stamps `$signed` on a whole-
        // element read when `elem_signed` is set. `signed` reflects the element type's
        // declared signedness (`byte`→signed, `byte unsigned`→unsigned).
        let elem_signed = signed;
        // The element must be a simple zero-LSB vector (`[W-1:0]`), an integral atom
        // (`int unsigned`/`byte unsigned`/…), or a scalar. A non-zero-LSB element
        // (`[15:8]`) needs per-element offset normalization (§4.5.103) — out of slice.
        let elem_w = match range {
            // No explicit vector range: an atom (`int`/`byte`/…, implicit zero-LSB
            // `[W-1:0]`) or a 1-bit scalar. A real/string/event/handle element is not
            // a bit-vector and cannot be md-packed → loud. The atom's width comes from
            // `range_to_dims` (int→32, byte→8, …); bare signed atoms already louded
            // above via `signed`.
            None => {
                if !ast_kind_is_bit_vector(kind) {
                    return Some(Err(
                        "a real / string / event / class-handle unpacked-array element",
                    ));
                }
                let (w, _, _, _) = self.range_to_dims(kind, None, signed);
                w
            }
            Some(r) => {
                let (m, l) = match (
                    self.const_eval_in_scope(&r.msb),
                    self.const_eval_in_scope(&r.lsb),
                ) {
                    (Some(m), Some(l)) => (m, l),
                    _ => return Some(Err("a non-constant element width")),
                };
                if l != 0 || m < 0 {
                    return Some(Err("a non-zero-LSB element range (`[hi:0]` only)"));
                }
                (m.abs_diff(l) + 1) as u32
            }
        };
        Some(Ok(ArrayFormal {
            count,
            elem_w,
            ascending,
            elem_signed,
            dims,
        }))
    }

    /// §4.5.199/206: the md-packed `packed_dims` extents for an N-D frame-local array OR a
    /// multi-dim array FORMAL (§4.5.202) — one `(lo, size, false)` entry per unpacked dim +
    /// the trailing element bit-width entry. `ascending=false` (raw index, so the declared
    /// direction never enters the element offset — matching directions align by construction);
    /// `lo` is the dim's base (§4.5.206: `m[1:4]` → `lo=1`, so `flatten_word` normalizes the
    /// index `idx-lo`; a zero-based dim keeps `lo=0` ⇒ no `Sub`). `flatten_word` over these
    /// dims maps `m[i][j]` (row-major) to bit offset `(i-lo0)*(∏inner sizes)*elem_w +
    /// (j-lo1)*elem_w`. For a single zero-based dim this yields `[(0, count, false), (0,
    /// elem_w, false)]` — the long-standing byte-identical 1-D formal layout.
    pub(crate) fn array_formal_ext_dims(
        dims: &[(u32, u32, bool)],
        elem_w: u32,
    ) -> Vec<(u32, u32, bool)> {
        dims.iter()
            .map(|&(lo, sz, _)| (lo, sz, false))
            .chain(std::iter::once((0u32, elem_w, false)))
            .collect()
    }

    /// True if `base` is the WHOLE array formal directly — a 1-seg `Ident`
    /// resolving to an md-packed array-formal frame net. A part / indexed-select of
    /// it (`arr[hi:lo]`, `arr[c+:w]`) is an UNPACKED-array SLICE, not a packed
    /// value; the md-packed internal representation would otherwise silently return
    /// a byte-concatenation of elements. Loud-reject (only element reads `arr[i]`
    /// are supported). An ELEMENT part-select `arr[i][hi:lo]` has a `BitSelect`
    /// base (not a bare `Ident`), so it is NOT caught here — it stays supported.
    pub(crate) fn is_array_formal_whole(&self, base: &ast::Expr) -> bool {
        if let ast::ExprKind::Ident(p) = &base.kind {
            if p.segments.len() == 1 {
                if let Some(net) = self.lookup_net_scoped(&p.segments[0].name) {
                    return self.frame_arr_formal_meta.contains_key(&net);
                }
            }
        }
        false
    }

    /// R2: is `p` a read-only `input` single-dim DYNAMIC-array formal of a simple
    /// integral element (`byte b[]` / `int b[]` / `logic [W:0] b[]`)? Only this shape
    /// is aliased (queue/assoc/multi-dim, and real/string/handle elements, are not).
    pub(crate) fn is_input_dyn_array_formal(&self, p: &ast::TfPort) -> bool {
        matches!(p.dir, ast::PortDir::Input)
            && p.unpacked.len() == 1
            && matches!(p.unpacked[0], ast::Dim::Dyn)
            && !matches!(
                p.net_or_var,
                Some(
                    ast::NetVarKind::String
                        | ast::NetVarKind::Real
                        | ast::NetVarKind::ClassHandle
                        | ast::NetVarKind::Event
                )
            )
    }

    /// V2B (§4.5.194): the OUTPUT/INOUT twin of `is_input_dyn_array_formal` — an
    /// `output`/`inout` dynamic-array formal (`output byte b[]`). Reserved as a `DynArray`
    /// net like an input formal (§4.5.171 lifecycle); the body writes it (`new[]` / element),
    /// and the engine DEEP-COPIES the formal's heap array OUT to the caller's array at the
    /// subroutine's exit (INOUT also snapshots the caller IN at entry).
    pub(crate) fn is_output_or_inout_dyn_array_formal(&self, p: &ast::TfPort) -> bool {
        matches!(p.dir, ast::PortDir::Output | ast::PortDir::Inout)
            && p.unpacked.len() == 1
            && matches!(p.unpacked[0], ast::Dim::Dyn)
            && !matches!(
                p.net_or_var,
                Some(
                    ast::NetVarKind::String
                        | ast::NetVarKind::Real
                        | ast::NetVarKind::ClassHandle
                        | ast::NetVarKind::Event
                )
            )
    }

    /// §4.5.203: a FIXED unpacked-array formal (`byte b[4]`, `int m[2][2]`) — every unpacked
    /// dim is a constant `[N]` / `[lo:hi]` (never dyn/queue/assoc, which are matched by the
    /// dyn predicates above). Such a formal is only supported on the FRAME path (an md-packed
    /// value slot, §4.5.188 input / §4.5.193 output-inout / §4.5.202 multi-dim); the inline
    /// (static-task) binding path has no slot. So a task carrying one is FORCE-FRAMED
    /// (`build_task_frame_set`) — proven safe since frame ⊇ inline (§4.5.198/199). A supported
    /// shape then binds via `classify_array_formal`; a rejected shape (descending, output/inout
    /// multi-dim, non-zero-based, …) stays loud on the frame path (correct-or-loud). Direction /
    /// base / element are NOT inspected here — that is the classifier's job at reserve time.
    pub(crate) fn is_fixed_unpacked_array_formal(&self, p: &ast::TfPort) -> bool {
        !p.unpacked.is_empty()
            && p.unpacked
                .iter()
                .all(|d| matches!(d, ast::Dim::Size(_) | ast::Dim::Range(_)))
    }
}
