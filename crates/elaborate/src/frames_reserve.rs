//! frame layout reservation — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// DUP (round-5): collect every `automatic` block-local declaration
    /// (name → declaring `Stmt::Block` span `(lo, hi)`) from a procedural statement
    /// tree. Only `Stmt::Block` carries a scopeable per-block namespace; `Stmt::Fork`
    /// locals keep the pre-existing flatten (concurrent — different semantics), so
    /// its `decls` are skipped (recursed into only to find nested Blocks). STATIC
    /// (non-`automatic`) block-locals are intentionally OMITTED — they safely
    /// coalesce onto one net when two sequential blocks reuse a temp name (see
    /// `hoist_block_local_nets`), so they need no separate scope.
    pub(crate) fn gather_auto_block_locals(
        s: &ast::Stmt,
        out: &mut BTreeMap<String, Vec<(u32, u32)>>,
    ) {
        match s {
            ast::Stmt::Block {
                decls, stmts, span, ..
            } => {
                for d in decls {
                    if d.lifetime == Some(true) {
                        for n in &d.names {
                            out.entry(n.name.name.clone())
                                .or_default()
                                .push((span.lo, span.hi));
                        }
                    }
                }
                for st in stmts {
                    Self::gather_auto_block_locals(st, out);
                }
            }
            ast::Stmt::Fork { stmts, .. } => {
                for st in stmts {
                    Self::gather_auto_block_locals(st, out);
                }
            }
            ast::Stmt::If { then_s, else_s, .. } => {
                Self::gather_auto_block_locals(then_s, out);
                if let Some(e) = else_s {
                    Self::gather_auto_block_locals(e, out);
                }
            }
            ast::Stmt::Case { items, .. } => {
                for it in items {
                    let inner = match it {
                        ast::CaseItem::Match { body: b, .. } => b,
                        ast::CaseItem::Default { body: b, .. } => b,
                    };
                    Self::gather_auto_block_locals(inner, out);
                }
            }
            ast::Stmt::For { body: b, .. }
            | ast::Stmt::While { body: b, .. }
            | ast::Stmt::Repeat { body: b, .. }
            | ast::Stmt::Forever { body: b, .. } => {
                Self::gather_auto_block_locals(b, out);
            }
            _ => {}
        }
    }

    /// `int tmp;`. Returns the `automatic`-override bits for the freshly reserved
    /// slots. The slot index is read from the live net count so it stays aligned
    /// with the frame's flat slot order even when some decls coalesce.
    /// A multi-dim PACKED frame-local decl (`logic [1:0][7:0] p`) is a flat
    /// `product(packed widths)`-bit vector — return its full `(width, msb, packed
    /// extents)` (else `None` ⇒ use the outer-dim-only `range_to_dims`). The extents are
    /// computed ONCE here (`packed_extents` emits an error for a non-const bound, so it
    /// must not run twice) and handed to [`Self::register_frame_packed`]. Together these
    /// mirror the module-scope decl so an element read `p[i]` is an element-width
    /// part-select, not a 1-bit bit-select on a truncated net.
    #[allow(clippy::type_complexity)]
    pub(crate) fn frame_packed_width(
        &mut self,
        d: &ast::NetVarDecl,
    ) -> Option<(u32, u32, Vec<(u32, u32, bool)>)> {
        if d.packed.is_empty() {
            return None;
        }
        let ext = self.packed_extents(d.range.as_ref(), &d.packed);
        let w = ext
            .iter()
            .fold(1u32, |a, &(_, pw, _)| a.saturating_mul(pw.max(1)));
        Some((w, w.saturating_sub(1), ext))
    }

    /// After reserving a frame-local `net`, register a multi-dim PACKED array's
    /// `packed_dims` (element-select, from the precomputed `ext`) + `dim_desc`
    /// (`$bits`/`$size`/`$dimensions`/`$left`/`$right`) — mirroring the module-scope
    /// decl. Uses `compute_dim_desc` (NOT `record_dim_desc`) to avoid the unconditional
    /// `intro_kind` side effect; the caller keeps its own 2-state `intro_kind` gate.
    pub(crate) fn register_frame_packed(
        &mut self,
        net: u32,
        d: &ast::NetVarDecl,
        unpacked: &[ast::Dim],
        ext: Vec<(u32, u32, bool)>,
    ) {
        self.packed_dims.insert(net, ext);
        let dd = self.compute_dim_desc(d.kind, d.range.as_ref(), &d.packed, unpacked);
        self.dim_desc.insert(net, dd);
    }

    /// Reserve ONE frame-local variable `name` (declared by `d`, with the per-name
    /// unpacked dims `unpacked`) and return its net id. Three cases:
    /// - a SUPPORTED single-dim unpacked array (§13.3 slice, `classify_unpacked_array`
    ///   ⇒ `Ok`): an md-packed `[count][elem_w]` frame slot — the SAME representation
    ///   as an array FORMAL, so `arr[k]` read/write reuses the packed part-select
    ///   machinery (`lower_packed_read` / `frame_write_lvalue`) and per-activity window
    ///   isolation. Registered in `frame_arr_formal_meta` (signed-element `$signed`
    ///   re-stamp + whole-array-slice reject) — NOT in `frame_array_local`, so a
    ///   suspendable-task guard (`frame_task_has_unsafe_construct`) lets it lift.
    /// - an UNSUPPORTED unpacked array (`Err`, e.g. multi-dim / non-zero-based /
    ///   dynamic / non-simple element): a 1-elem net marked `frame_array_local` so
    ///   `mem[k]` element access stays loud (correct-or-loud), as before.
    /// - a scalar / multi-dim PACKED local: the widen + `register_frame_packed` path.
    pub(crate) fn reserve_frame_local_decl(
        &mut self,
        name: &str,
        d: &ast::NetVarDecl,
        unpacked: &[ast::Dim],
    ) -> u32 {
        if !unpacked.is_empty() {
            // V5: a frame-local single-dim DYNAMIC array (`int loc[]`) of a simple
            // bit-vector element gets a real `DynArray` heap handle (like a module
            // dyn-array), so `loc = new[n]` / `loc[i]` / `loc.size()` route to the heap.
            // The per-net heap slot (`dyn_heap[net]`) holds the CURRENT activation's
            // array; the engine frees it at frame exit and fatal-louds on recursive /
            // concurrent reentry (correct-or-loud; a full per-activation stash is a
            // follow-on). Multi-dim / packed / non-bit-vector-element dynamic arrays
            // stay loud (`frame_array_local`).
            // T1-7: a `string` ELEMENT joins the bit-vector ones. The container is the
            // same `DynArray` heap handle; only the element storage differs, and
            // `string_elem_dyn_nets` is what tells the engine to hold these elements as
            // byte strings (and fill `new[]` with ""). Note this is the element type —
            // a frame-local SCALAR `string` is a different thing entirely (a slab-stored
            // `NetKind::String`, §4.5.167) and never reaches here, because it has no
            // unpacked dimension.
            let str_elem = matches!(d.kind, ast::NetVarKind::String);
            if unpacked.len() == 1
                && matches!(unpacked[0], ast::Dim::Dyn)
                && d.packed.is_empty()
                && (ast_kind_is_bit_vector(d.kind) || str_elem)
            {
                let (w, msb, lsb, signed) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
                let net = self.nets.len() as u32;
                self.add_net(
                    name,
                    ir::NetVar {
                        kind: ir::NetKind::DynArray,
                        // A string element has no packed width; the handle carries 0 so
                        // `coerce_dyn_elem` / `resize_keep_sign` take their `is_str`
                        // paths instead of truncating the byte string.
                        width: if str_elem { 0 } else { w.max(1) },
                        msb: if str_elem { 0 } else { msb },
                        lsb: if str_elem { 0 } else { lsb },
                        signed: signed && !str_elem,
                        array_len: 0, // heap handle — elements live in the engine heap
                        dir: ir::PortDir::Internal,
                        init: default_init(d.kind, w.max(1)),
                    },
                );
                if str_elem {
                    self.string_elem_dyn_nets.insert(net);
                }
                // IEEE §7.5.2: a 2-state element defaults to 0 (not X) on `new[]` fill.
                if net_kind_is_two_state(d.kind) {
                    self.two_state_heap_handles.insert(net);
                }
                return net;
            }
            // T1-7: a FIXED frame-local string array (`string s[2]` in a task body) takes
            // the same heap handle, pre-sized at frame ENTRY by `emit_frame_local_inits`
            // (the module-scope twin rides the t0 var-init flush, which a frame local
            // never reaches). Registering it in `fixed_string_dyn` is what makes it
            // behave as FIXED storage: `new[]` on it stays loud, a multi-dim declaration
            // flattens row-major through the same chain walk, and a partial index is
            // rejected — one shared set, so the frame-local and module-scope forms cannot
            // drift apart.
            if str_elem && d.packed.is_empty() {
                if let Some(dims) = self.fixed_string_dims_zero_asc(unpacked) {
                    let net = self.nets.len() as u32;
                    self.add_net(
                        name,
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
                    self.string_elem_dyn_nets.insert(net);
                    self.fixed_string_dyn.insert(net, dims);
                    return net;
                }
            }
            if let Some(Ok(af)) = self.classify_unpacked_array(
                unpacked,
                d.range.as_ref(),
                &d.packed,
                d.signed,
                d.kind,
            ) {
                let (count, elem_w) = (af.count, af.elem_w);
                // §4.5.199: `count` = product of every unpacked dim, and `ext` gets one
                // packed entry per dim (via `af.dims`) so a multi-dim frame-LOCAL array
                // `int m[2][2]` reads/writes `m[i][j]` through the existing N-D packed
                // chain. A single-dim array is byte-identical to `array_formal_ext`.
                let ext = Self::array_formal_ext_dims(&af.dims, elem_w);
                let w = count.saturating_mul(elem_w).max(1);
                let net = self.nets.len() as u32;
                self.add_net(
                    name,
                    ir::NetVar {
                        // Whole md-packed slot is unsigned (`signed:false`); the element's
                        // signedness lives in `frame_arr_formal_meta` (`$signed` re-stamp).
                        // A 2-state element defaults to 0 via `default_init(d.kind, w)`
                        // (an unpacked array of `int` inits to 0, of `logic` to X).
                        kind: frame_local_net_kind(d.kind),
                        width: w,
                        msb: w.saturating_sub(1),
                        lsb: 0,
                        signed: false,
                        array_len: 1,
                        dir: ir::PortDir::Internal,
                        init: default_init(d.kind, w),
                    },
                );
                self.packed_dims.insert(net, ext);
                let dd = self.compute_dim_desc(d.kind, d.range.as_ref(), &[], unpacked);
                self.dim_desc.insert(net, dd);
                self.frame_arr_formal_meta.insert(net, af);
                if net_kind_is_two_state(d.kind) {
                    self.intro_kind.insert(net, d.kind);
                }
                return net;
            }
            // `classify_unpacked_array` returned `Some(Err(..))` — fall through to the
            // loud 1-elem + `frame_array_local` path below.
        }
        let pinfo = self.frame_packed_width(d);
        let (mut w, mut msb, lsb, signed) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
        if let Some((pw, pmsb, _)) = &pinfo {
            w = *pw;
            msb = *pmsb;
        }
        let net = self.nets.len() as u32;
        self.add_net(
            name,
            ir::NetVar {
                kind: frame_local_net_kind(d.kind),
                width: w,
                msb,
                lsb,
                signed,
                array_len: 1,
                dir: ir::PortDir::Internal,
                init: default_init(d.kind, w),
            },
        );
        if let Some((_, _, ext)) = pinfo {
            self.register_frame_packed(net, d, unpacked, ext);
        }
        // EXT2-H guard: an UNSUPPORTED unpacked-array local (1-elem net) — mark it so
        // its `mem[k]` select write stays loud, not mis-lowered as a bit-select.
        if !unpacked.is_empty() {
            self.frame_array_local.insert(net);
        }
        if net_kind_is_two_state(d.kind) {
            self.intro_kind.insert(net, d.kind);
        }
        net
    }

    pub(crate) fn reserve_frame_block_locals(&mut self, body: &ast::Stmt, base_net: u32) -> u64 {
        let mut decls = Vec::new();
        collect_block_local_decls(body, &mut decls);
        let mut auto_override = 0u64;
        for d in &decls {
            for decl in &d.names {
                let fq = self.fq(&decl.name.name);
                if let Some(&existing) = self.symbols.get(&fq) {
                    // EXT2-H guard: a block-local UNPACKED ARRAY that shadows a
                    // same-named outer scalar coalesces onto that (shape-blind) net —
                    // mark the coalesced net so an element write `y[k]=v` stays loud,
                    // not a silent scalar bit-write.
                    if !decl.unpacked.is_empty() {
                        self.frame_array_local.insert(existing);
                    }
                    continue; // coalesce with an already-reserved formal/local
                }
                let slot = self.nets.len() as u32 - base_net;
                self.reserve_frame_local_decl(&decl.name.name, d, &decl.unpacked);
                if d.lifetime == Some(true) && slot < 64 {
                    auto_override |= 1u64 << slot;
                }
            }
        }
        auto_override
    }

    pub(crate) fn reserve_frame_func(&mut self, name: &str, func: &ast::FunctionDef) {
        let fid = self.funcs.len() as u32;
        let base_net = self.nets.len() as u32;
        let n_params = func.ports.len() as u32;
        // Family D (r17): register this per-instance FuncId for a hierarchical call
        // `u1.f(x)` IF the function is HIER-CALLABLE — a plain module function (no
        // `::`), input-only SCALAR formals, non-`string` formals, and a non-`string`
        // return. `cur_prefix` is the current instance path, so the key is the full
        // `<inst>.<fname>` that `hier_resolve` reconstructs from the call's segments. A
        // non-callable function (output/inout/array/string formal or string return) gets
        // NO entry → a hier call to it stays loud at resolution (correct-or-loud).
        if !name.contains("::")
            && !func.ret_string
            && func.ports.iter().all(|p| {
                matches!(p.dir, ast::PortDir::Input)
                    && p.unpacked.is_empty()
                    && !matches!(
                        p.net_or_var.unwrap_or(ast::NetVarKind::Reg),
                        ast::NetVarKind::String
                    )
            })
        {
            let key = if self.cur_prefix.is_empty() {
                name.to_string()
            } else {
                format!("{}.{}", self.cur_prefix, name)
            };
            self.hier_funcs.insert(key, fid);
        }
        // v7: per-formal `string` bitmask — a `string` formal lowers to a 1-bit
        // Wire slot, so the engine needs this to bind a string LITERAL actual as a
        // heap string rather than truncating it (see `FuncMeta.str_params`).
        let mut str_params: u64 = 0;
        for (i, p) in func.ports.iter().enumerate() {
            if matches!(
                p.net_or_var.unwrap_or(ast::NetVarKind::Reg),
                ast::NetVarKind::String
            ) {
                if i < 64 {
                    str_params |= 1u64 << i;
                } else {
                    // The `string`-formal mask is 64-wide; a `string` formal beyond
                    // index 63 could not be marked → its literal actual would
                    // silently truncate to the 1-bit slot. Loud-reject instead
                    // (correct-or-loud); a 65+-param frame function is pathological.
                    self.error_unsupported(
                        p.span,
                        "a `string` formal beyond parameter index 63 is unsupported \
                         (the frame-call string-argument mask is 64 wide)",
                    );
                }
            }
        }
        let (ret_width, ret_signed) = self.func_return_dims(func);
        let scope_seg = format!("$func${name}");
        // The return var is named by the FUNCTION's own name, not the frame-table key
        // — the body assigns / reads it by that name (`f = E;` / `return f`). These are
        // equal for every module frame func (the table key IS the function name), so this
        // is byte-identical there; they DIFFER only for a package-scoped call, whose key
        // is `pkg::name` (distinct scope, to avoid colliding with a same-named module
        // function) while the body still says `name` (round-7).
        let ret_name = func.name.name.clone();
        let auto_override = self.with_scope(&scope_seg, |s| {
            // [0..n_params): input formals, port order.
            for p in &func.ports {
                // §4.5.177: an `input` DYNAMIC-array formal (`int c[]`) in a FRAMED
                // (automatic/recursive, control-flow-body) function is a per-activation
                // `DynArray` heap net — the caller's array is snapshotted into it at the
                // call site (`emit_frame_call` + a `handle_copy` marker). A framed
                // function's `&self` executor can never mutate a dyn-array (`new[]` /
                // element write both need `&mut` heap), so the snapshot is a safe
                // pass-by-value/alias. Reads route to the heap (`read_net` dyn branch) and
                // §4.5.176 makes `foreach(c[i])` / `c.first(k)` work inside the body. A
                // STRAIGHT-LINE dyn-formal function still takes the R2 inline alias path.
                if s.is_input_dyn_array_formal(p) {
                    let kind = p.net_or_var.unwrap_or(ast::NetVarKind::Reg);
                    let (w, msb, lsb, signed) = s.range_to_dims(kind, p.range.as_ref(), p.signed);
                    let net = s.nets.len() as u32;
                    s.add_net(
                        &p.name.name,
                        ir::NetVar {
                            kind: ir::NetKind::DynArray,
                            width: w.max(1),
                            msb,
                            lsb,
                            signed,
                            array_len: 0, // heap handle — elements live in the engine heap
                            dir: ir::PortDir::Internal,
                            init: default_init(kind, w.max(1)),
                        },
                    );
                    if net_kind_is_two_state(kind) {
                        s.two_state_heap_handles.insert(net);
                    }
                    continue;
                }
                // §13.3 UARR: an unpacked-array formal lowers to an md-packed
                // `[count][elem_w]` frame slot so `arr[i]` reuses the md-packed
                // element read/write machinery; the call site packs the actual
                // whole-array into a `count*elem_w` concat. Outside the slice, the
                // call site loud-rejects — reserve a sane scalar so the (then
                // unreachable) body lowering does not panic, keeping 1 slot / formal.
                if let Some(cls) = s.classify_array_formal(p) {
                    match cls {
                        Ok(af) => {
                            let (count, elem_w) = (af.count, af.elem_w);
                            // §4.5.202: N-D packed_dims (one position-indexed entry per
                            // unpacked dim + the elem_w entry). Byte-identical to
                            // `array_formal_ext(count, elem_w)` for a 1-D formal.
                            let ext = Self::array_formal_ext_dims(&af.dims, elem_w);
                            let w = count.saturating_mul(elem_w).max(1);
                            let net = s.nets.len() as u32;
                            s.add_net(
                                &p.name.name,
                                ir::NetVar {
                                    kind: ir::NetKind::Reg,
                                    width: w,
                                    msb: w.saturating_sub(1),
                                    lsb: 0,
                                    signed: false,
                                    array_len: 1,
                                    dir: ir::PortDir::Internal,
                                    init: default_init(ast::NetVarKind::Reg, w),
                                },
                            );
                            s.packed_dims.insert(net, ext);
                            let dd = s.compute_dim_desc(
                                p.net_or_var.unwrap_or(ast::NetVarKind::Reg),
                                p.range.as_ref(),
                                &[],
                                &p.unpacked,
                            );
                            s.dim_desc.insert(net, dd);
                            s.frame_arr_formal_meta.insert(net, af);
                            // A 2-state ELEMENT (`int unsigned`/`byte unsigned`/`bit`/…)
                            // can never hold X/Z (§6.11.3): register the whole md-packed
                            // slot as 2-state so the frame arg-binding coerces X/Z→0. The
                            // coercion is whole-value, which is correct for a uniform
                            // 2-state element array; a 4-state `logic`/`reg` element keeps
                            // X/Z (not registered).
                            let ekind = p.net_or_var.unwrap_or(ast::NetVarKind::Reg);
                            if net_kind_is_two_state(ekind) {
                                s.intro_kind.insert(net, ekind);
                            }
                        }
                        Err(_) => {
                            let (w, msb, lsb, signed) =
                                s.range_to_dims(ast::NetVarKind::Reg, p.range.as_ref(), p.signed);
                            s.add_net(
                                &p.name.name,
                                ir::NetVar {
                                    kind: ir::NetKind::Reg,
                                    width: w,
                                    msb,
                                    lsb,
                                    signed,
                                    array_len: 1,
                                    dir: ir::PortDir::Internal,
                                    init: default_init(ast::NetVarKind::Reg, w),
                                },
                            );
                        }
                    }
                    continue;
                }
                let kind = p.net_or_var.unwrap_or(ast::NetVarKind::Reg);
                let (w, msb, lsb, signed) = s.range_to_dims(kind, p.range.as_ref(), p.signed);
                let net = s.nets.len() as u32;
                s.add_net(
                    &p.name.name,
                    ir::NetVar {
                        // R5-B: an output/inout `string` formal needs a real assignable
                        // `NetKind::String` slot (mirrors reserve_frame_task). For an
                        // INPUT formal `formal_net_kind` delegates to
                        // `map_net_kind_or_wire`, so an input-only function is
                        // byte-identical.
                        kind: formal_net_kind(kind, p.dir),
                        width: w,
                        msb,
                        lsb,
                        signed,
                        array_len: 1,
                        dir: ir::PortDir::Internal,
                        init: default_init(kind, w),
                    },
                );
                // A 2-state formal (byte/int/shortint/longint/bit) can never hold
                // X/Z — register it so the engine coerces frame slot writes (§6.11.3).
                if net_kind_is_two_state(kind) {
                    s.intro_kind.insert(net, kind);
                }
            }
            // [n_params]: the function-named RETURN var (declared range/sign).
            let ret_net = s.nets.len() as u32;
            s.add_net(
                &ret_name,
                ir::NetVar {
                    // R4: a `string` return uses a heap-backed `NetKind::String` slot
                    // (procedurally assignable, the frame body writes it and the copy-out
                    // reads the string Value); `ret_width` stays the Implicit-Reg width 1
                    // (a string bypasses width via `resize_keep_sign`, which keeps is_str).
                    kind: if func.ret_string {
                        ir::NetKind::String
                    } else if ret_width == 32 && ret_signed {
                        ir::NetKind::Integer
                    } else {
                        ir::NetKind::Reg
                    },
                    width: ret_width,
                    msb: ret_width.saturating_sub(1),
                    lsb: 0,
                    signed: ret_signed,
                    array_len: 1,
                    dir: ir::PortDir::Internal,
                    init: default_init(ast::NetVarKind::Reg, ret_width),
                },
            );
            // A 2-state return type can never hold X/Z — register the return slot
            // so `frame_slot_write` coerces the return assignment (§6.11.3). The
            // specific 2-state kind is immaterial to coercion; pick one by width.
            if func.ret_two_state {
                let k = match ret_width {
                    0..=8 => ast::NetVarKind::Byte,
                    9..=16 => ast::NetVarKind::Shortint,
                    17..=32 => ast::NetVarKind::Int,
                    _ => ast::NetVarKind::Longint,
                };
                s.intro_kind.insert(ret_net, k);
            }
            // [n_params+1..]: body_decls, source order (scalars only — a frame-
            // local array/dyn/string is outside the B1 cut and lowers as a 1-elem
            // net here; the body validator rejects any select/array lvalue use).
            // B4: a body_decl declared `automatic` sets its slot's override bit.
            let mut auto_override: u64 = 0;
            let mut slot = n_params + 1; // formals + return var
            for d in &func.body_decls {
                for decl in &d.names {
                    s.reserve_frame_local_decl(&decl.name.name, d, &decl.unpacked);
                    if d.lifetime == Some(true) && slot < 64 {
                        auto_override |= 1u64 << slot;
                    }
                    slot += 1;
                }
            }
            // Block-locals declared inside a `begin … end` in the body (after the
            // top-level body_decls in the flat slot order).
            auto_override |= s.reserve_frame_block_locals(&func.body, base_net);
            auto_override
        });
        let locals_len = self.nets.len() as u32 - base_net;
        self.frame_idx.insert(name.to_string(), fid);
        self.funcs.push(ir::FuncDef {
            entry: 0, // filled by lower_frame_func_body
            n_params,
            locals_len,
            is_task: false,
        });
        self.func_metas.push(FuncMeta {
            base_net,
            n_params,
            return_slot: n_params, // convention: return var right after the formals
            locals_len,
            is_automatic: func.automatic,
            ret_width,
            ret_signed,
            auto_override,
            str_params,
            has_hier_call: false,
            contains_shared_fork: false,
        });
        self.frame_func_names.push(func.name.name.clone()); // %m
    }

    /// B2: reserve a frame TASK — allocate its formal nets (input + output, in
    /// declared order) then body_decls under a `$func$<name>` scope; push a
    /// `FuncDef{is_task:true}` + `FuncMeta` (no return var) and the name→FuncId
    /// task divert.
    pub(crate) fn reserve_frame_task(&mut self, name: &str, task: &ast::TaskDef) {
        let fid = self.funcs.len() as u32;
        let base_net = self.nets.len() as u32;
        let n_params = task.ports.len() as u32;
        // Family D (r18): register this per-instance FuncId for a hierarchical enable
        // `u1.tk(x)` IF the task is HIER-CALLABLE — a plain module task (no `::`) with
        // SCALAR non-`string`, non-array formals of ANY direction (§4.5.201: input/output/
        // inout). An input/inout formal copies IN by value; an output/inout formal copies
        // OUT to the caller lvalue at the task's exit — both via the standard `TaskCallInfo`
        // in/out binds the engine already applies. An array/string formal gets NO entry → a
        // hier enable to it stays loud at resolution (correct-or-loud). `cur_prefix` is the
        // current instance path, so the key is the full `<inst>.<tname>` that `hier_resolve`
        // reconstructs from the enable's segments; `hier_task_port_dirs` records the
        // directions so the resolver can route each arg without the callee def in scope.
        if !name.contains("::")
            && task.ports.iter().all(|p| {
                // A SCALAR non-`string` formal of any direction is hier-callable (§4.5.201
                // in/out/inout binds).
                let scalar_ok = p.unpacked.is_empty()
                    && !matches!(
                        p.net_or_var.unwrap_or(ast::NetVarKind::Reg),
                        ast::NetVarKind::String
                    );
                // §4.5.207 / item 1: a FIXED unpacked-array formal of ANY direction is
                // hier-callable. INPUT packs the actual array into the md-packed slot at
                // resolution; OUTPUT/INOUT additionally UNPACKS the slot back into the caller
                // array at the enable's ret block (deferred copy-out, §13.5.2). A string/dyn
                // formal is still NOT hier-callable, so a task carrying one stays loud. An
                // unsupported array SHAPE (descending non-zero-base, …) gets no
                // `frame_arr_formal_meta` entry → the resolver treats it as a scalar and the
                // whole-array actual is loud there (correct-or-loud).
                let fixed_array_ok = self.is_fixed_unpacked_array_formal(p);
                scalar_ok || fixed_array_ok
            })
        {
            let key = if self.cur_prefix.is_empty() {
                name.to_string()
            } else {
                format!("{}.{}", self.cur_prefix, name)
            };
            self.hier_tasks.insert(key, fid);
            self.hier_task_port_dirs
                .insert(fid, task.ports.iter().map(|p| p.dir).collect());
        }
        // v7: per-formal `string` bitmask (mirror of reserve_frame_func) — a
        // `string` formal lowers to a 1-bit Wire slot, so the `run_task` copy-in
        // needs this to bind a string LITERAL actual as a heap string rather than
        // truncating it (the copy-in only consults INPUT slots; marking an output
        // formal is harmless). See `FuncMeta.str_params`.
        let mut str_params: u64 = 0;
        for (i, p) in task.ports.iter().enumerate() {
            if matches!(
                p.net_or_var.unwrap_or(ast::NetVarKind::Reg),
                ast::NetVarKind::String
            ) {
                if i < 64 {
                    str_params |= 1u64 << i;
                } else {
                    self.error_unsupported(
                        p.span,
                        "a `string` formal beyond parameter index 63 is unsupported \
                         (the frame-call string-argument mask is 64 wide)",
                    );
                }
            }
        }
        let scope_seg = format!("$func${name}");
        let auto_override = self.with_scope(&scope_seg, |s| {
            // [0..n_params): formals (input AND output, declared order).
            for p in &task.ports {
                let kind = p.net_or_var.unwrap_or(ast::NetVarKind::Reg);
                let (w, msb, lsb, signed) = s.range_to_dims(kind, p.range.as_ref(), p.signed);
                let net = s.nets.len() as u32;
                // V2A-frame (§4.5.173): an `input` DYNAMIC-array formal (`byte b[]`) is a
                // per-activation `DynArray` heap handle — the caller's array is DEEP-COPIED
                // into it at frame entry (pass-by-VALUE, IEEE §13.5.1; `enter` snapshot in
                // exec.rs). Reserved exactly like a frame-local dyn-array LOCAL (§4.5.171),
                // so V5's net-range lifecycle (`func_has_dyn_local` → reentry guard +
                // free-at-exit) covers it automatically and `b[i]`/`b.size()` route to the
                // heap (`read_net` frame_local dyn branch). V2B (§4.5.194): an OUTPUT/INOUT
                // dyn formal reserves the SAME `DynArray` net — the body writes it and the
                // engine deep-copies it out to the caller at exit (INOUT also copies in).
                if s.is_input_dyn_array_formal(p) || s.is_output_or_inout_dyn_array_formal(p) {
                    s.add_net(
                        &p.name.name,
                        ir::NetVar {
                            kind: ir::NetKind::DynArray,
                            width: w.max(1),
                            msb,
                            lsb,
                            signed,
                            array_len: 0, // heap handle — elements live in the engine heap
                            dir: ir::PortDir::Internal,
                            init: default_init(kind, w.max(1)),
                        },
                    );
                    // IEEE §7.5.2: a 2-state element defaults to 0 (not X) on the copy-in.
                    if net_kind_is_two_state(kind) {
                        s.two_state_heap_handles.insert(net);
                    }
                    continue;
                }
                // §13.3 UARR (§4.5.188/193): an `input` OR `output` unpacked-FIXED
                // array formal (`byte b[4]`, `logic [63:0] w[80]`) lowers to an
                // md-packed `[count][elem_w]` frame slot — IDENTICAL to the FUNCTION
                // path (`reserve_frame_func`), so `b[i]` reuses the md-packed element
                // read/write machinery. INPUT: the caller packs the whole-array actual
                // into the slot value. OUTPUT (§4.5.193): the body WRITES the slot; the
                // caller copies the whole slot to a packed temp at exit and unpacks it
                // into the caller array elements after the call (`emit_frame_task_call`).
                // The value slot works on BOTH the suspendable-frame and the synchronous
                // `run_task_call` path (no heap). INOUT (r17) reserves the SAME md-packed
                // slot — the caller packs the actual IN at entry and unpacks OUT at exit.
                if matches!(
                    p.dir,
                    ast::PortDir::Input | ast::PortDir::Output | ast::PortDir::Inout
                ) {
                    if let Some(Ok(af)) = s.classify_array_formal(p) {
                        let (count, elem_w) = (af.count, af.elem_w);
                        // §4.5.202: N-D packed_dims for a multi-dim ascending formal
                        // (`int m[2][2]`); byte-identical to `array_formal_ext` for 1-D.
                        let ext = Self::array_formal_ext_dims(&af.dims, elem_w);
                        let w = count.saturating_mul(elem_w).max(1);
                        s.add_net(
                            &p.name.name,
                            ir::NetVar {
                                kind: ir::NetKind::Reg,
                                width: w,
                                msb: w.saturating_sub(1),
                                lsb: 0,
                                signed: false,
                                array_len: 1,
                                dir: ir::PortDir::Internal,
                                init: default_init(ast::NetVarKind::Reg, w),
                            },
                        );
                        s.packed_dims.insert(net, ext);
                        let dd = s.compute_dim_desc(
                            p.net_or_var.unwrap_or(ast::NetVarKind::Reg),
                            p.range.as_ref(),
                            &[],
                            &p.unpacked,
                        );
                        s.dim_desc.insert(net, dd);
                        s.frame_arr_formal_meta.insert(net, af);
                        let ekind = p.net_or_var.unwrap_or(ast::NetVarKind::Reg);
                        if net_kind_is_two_state(ekind) {
                            s.intro_kind.insert(net, ekind);
                        }
                        continue;
                    }
                }
                // N1: a `string` OUTPUT/INOUT formal is a real `NetKind::String` slot
                // (procedurally assignable) instead of the input-only 1-bit Wire — the
                // body writes it and the frame copy-out moves the string to the actual.
                s.add_net(
                    &p.name.name,
                    ir::NetVar {
                        kind: formal_net_kind(kind, p.dir),
                        width: w,
                        msb,
                        lsb,
                        signed,
                        array_len: 1,
                        dir: ir::PortDir::Internal,
                        init: default_init(kind, w),
                    },
                );
                if net_kind_is_two_state(kind) {
                    s.intro_kind.insert(net, kind);
                }
            }
            // [n_params..): body_decls, source order (scalars). B4: a body_decl
            // declared `automatic` sets its slot's override bit.
            let mut auto_override: u64 = 0;
            let mut slot = n_params; // tasks have no return var
            for d in &task.body_decls {
                for decl in &d.names {
                    s.reserve_frame_local_decl(&decl.name.name, d, &decl.unpacked);
                    if d.lifetime == Some(true) && slot < 64 {
                        auto_override |= 1u64 << slot;
                    }
                    slot += 1;
                }
            }
            // Block-locals declared inside a `begin … end` in the body.
            auto_override |= s.reserve_frame_block_locals(&task.body, base_net);
            auto_override
        });
        let locals_len = self.nets.len() as u32 - base_net;
        self.task_frame_idx.insert(name.to_string(), fid);
        self.funcs.push(ir::FuncDef {
            entry: 0, // filled by lower_frame_task_body
            n_params,
            locals_len,
            is_task: true,
        });
        self.func_metas.push(FuncMeta {
            base_net,
            n_params,
            return_slot: 0, // unused for tasks (no func-named return var)
            locals_len,
            is_automatic: task.automatic,
            ret_width: 1,
            ret_signed: false,
            auto_override,
            str_params,
            has_hier_call: false,
            contains_shared_fork: false,
        });
        self.frame_func_names.push(task.name.name.clone()); // %m
    }
}
