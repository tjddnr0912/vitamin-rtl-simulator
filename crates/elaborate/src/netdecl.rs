//! net & variable declarations — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// P1-8: emit `ElabMultidriver` for any net whose continuous-assign drivers
    /// OVERLAP at the bit level. Whole-net targets count as `[0, width)`; a
    /// static part/bit-select as `[off, off+w)`. DYNAMIC (non-const offset)
    /// selects and array-element writes are not counted (the conservative cut —
    /// a false positive on a disjoint dynamic split would reject legal code).
    /// Deterministic: nets in ascending id, intervals sorted, one report per net.
    pub(crate) fn check_whole_net_multidriver(&mut self) {
        let mut per_net: BTreeMap<u32, Vec<(u64, u64)>> = BTreeMap::new();
        // MULTI-DRIVER: a net all of whose cont-assign drivers are WHOLE-NET and
        // non-delayed is RESOLVED by 4-state wire resolution at settle time (the
        // sim-engine `md_nets`), so its overlap is LEGAL — not an error. Mirror
        // the engine's eligibility exactly: a net is `not_md` (keeps E3001 on
        // overlap) if ANY driver is delayed, multi-chunk, array-element, or a
        // partial/bit select.
        let mut not_md: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        for ca in &self.cont_assigns {
            let whole_ok = ca.delay.is_none() && ca.lhs.chunks.len() == 1 && {
                let c = &ca.lhs.chunks[0];
                c.word.is_none() && c.offset.is_none() && c.width.is_none()
            };
            if !whole_ok {
                for c in &ca.lhs.chunks {
                    not_md.insert(c.net);
                }
            }
            for c in &ca.lhs.chunks {
                if c.word.is_some() {
                    continue; // array-element write: not counted (v1)
                }
                let Some(nv) = self.nets.get(c.net as usize) else {
                    continue;
                };
                let iv = match (c.offset, c.width) {
                    (None, None) => Some((0u64, nv.width.max(1) as u64)),
                    (Some(off_e), w_e) => {
                        let off = self.const_expr_u64(off_e);
                        let w = match w_e {
                            Some(we) => self.const_expr_u64(we),
                            None => Some(1), // bit-select
                        };
                        match (off, w) {
                            (Some(o), Some(w)) => Some((o, o.saturating_add(w.max(1)))),
                            _ => None, // dynamic select: skip
                        }
                    }
                    (None, Some(_)) => None, // not produced by collect_lval_chunks
                };
                if let Some(iv) = iv {
                    per_net.entry(c.net).or_default().push(iv);
                }
            }
        }
        for (net, mut ivs) in per_net {
            if ivs.len() < 2 {
                continue;
            }
            // Whole-net multi-driver (all drivers resolvable): legal, the engine
            // resolves it. Keep E3001 only for overlaps the engine does NOT model
            // (a partial/dynamic/array/delayed driver is in the mix).
            if !not_md.contains(&net) {
                continue;
            }
            ivs.sort_unstable();
            let overlap = ivs.windows(2).any(|p| p[1].0 < p[0].1);
            if overlap {
                let name = self
                    .symbols
                    .iter()
                    .find(|(_, &id)| id == net)
                    .map(|(n, _)| n.clone())
                    .unwrap_or_else(|| format!("#{net}"));
                self.error(
                    MsgCode::ElabMultidriver,
                    &format!(
                        "net `{name}` driven by multiple overlapping continuous \
                         assignments"
                    ),
                );
            }
        }
    }

    pub(crate) fn elaborate_netvar_decl(
        &mut self,
        d: &ast::NetVarDecl,
        ports: &ast::PortList,
        body: &[ast::ModuleItem],
        // A `string s = expr;` initializer is supported only in the module body,
        // whose `collect_var_init_drivers` pass emits the t0 assignment. In other
        // scopes (block-local, interface, generate) the init is not collected, so
        // it stays a LOUD reject here rather than a silently-dropped initializer.
        allow_string_init: bool,
    ) {
        // §4.5.249: anchor this declaration's diagnostics at the declaration.
        let saved_span = self.cur_span.replace(d.span);
        self.elaborate_netvar_decl_inner(d, ports, body, allow_string_init);
        self.cur_span = saved_span;
    }

    fn elaborate_netvar_decl_inner(
        &mut self,
        d: &ast::NetVarDecl,
        ports: &ast::PortList,
        body: &[ast::ModuleItem],
        allow_string_init: bool,
    ) {
        // ⓑ-breadth (§25.9): a `virtual INTERFACE vif;` handle is NOT a net — it is
        // a STATIC ALIAS resolved in the post-instance pass (`resolve_virtual_ifaces`),
        // AFTER the bound interface instances are flattened. No net here.
        if d.kind == ast::NetVarKind::VirtualIface {
            return;
        }
        if !net_kind_supported(d.kind) {
            self.error(MsgCode::ElabUnsupported, "unsupported net/var kind (v1)");
            // still emit a Wire-shaped net per name so references resolve.
        }
        // Multi-dim PACKED array (`logic [3:0][7:0]`): the net is a flat vector of
        // `product(packed widths)` bits; a select `m[i]` is a bit-slice. Computed once
        // per decl (shared by all names; unpacked dims are per-name).
        let packed_ext = self.packed_extents(d.range.as_ref(), &d.packed);
        // a named event carries NO range/init/array surface (IEEE §6.17) —
        // loud, then the bare counter net is still created so refs resolve.
        if matches!(d.kind, ast::NetVarKind::Event)
            && (d.range.is_some()
                || !d.packed.is_empty()
                || d.names
                    .iter()
                    .any(|n| n.init.is_some() || !n.unpacked.is_empty()))
        {
            self.error(
                MsgCode::ElabUnsupported,
                "a named event takes no range, initializer or array dimensions",
            );
        }
        // GAP-G: capture const-array element values (covers a generate-LOCAL array
        // param, whose net is created only here; a module-scope one is also
        // captured earlier in the decl-order body-param walk so a `localparam X =
        // ROT[i]` folds). Idempotent.
        self.capture_const_array_vals(d);
        for decl in &d.names {
            // A plain net DECLARATION is the one place that both sizes the net and can
            // record its declared low bound, so it is the one place that opts in to a
            // NEGATIVE one (`logic [3:-2] x` — 6 bits, IEEE §7.4.2). Every other
            // `range_to_dims` caller keeps the warn-and-clamp: a port/formal/class-property
            // with a negative bound stays LOUD rather than becoming a wide net whose
            // selects nobody normalizes.
            // …and only for a net that reaches the RECORD 550 lines below. A
            // dyn/queue/assoc element net is built on an early-`continue` path that
            // never does, so opting it in makes the net WIDE with nobody normalizing
            // its selects: `logic [-3:0] q[$]` read `q[0][-3]` as internal bit 6 and
            // printed a silent `x`, where the clamp at least warned. Adversarial
            // review, and the exact ladder descent this opt-in exists to prevent.
            let dyn_storage = decl
                .unpacked
                .iter()
                .any(|x| matches!(x, ast::Dim::Dyn | ast::Dim::Queue(_) | ast::Dim::Assoc(_)));
            let neg_lsb = self
                .declared_neg_lsb(d.range.as_ref())
                .filter(|_| !dyn_storage);
            let asc_lsb = self
                .declared_asc_lsb(d.range.as_ref())
                .filter(|_| !dyn_storage);
            let (mut width, mut msb, lsb, signed) = self.range_to_dims_opt(
                d.kind,
                d.range.as_ref(),
                d.signed,
                neg_lsb.is_some() || asc_lsb.is_some(),
            );
            if !d.packed.is_empty() {
                width = packed_ext
                    .iter()
                    .fold(1u32, |a, &(_, w, _)| a.saturating_mul(w.max(1)));
                msb = width.saturating_sub(1);
            }
            // ── v7 P2-C: `string s` — heap-handle declaration ──
            // width 0 / array_len 0; the engine heap holds the bytes (dyn
            // precedent). Reads materialize is_str packed values; writes
            // strip leading NULs through the funnel.
            if matches!(d.kind, ast::NetVarKind::String) {
                // N6: a FIXED `string` ARRAY (`string files[0:1]` / `string f[4]`) — a
                // single fixed unpacked dim, no packed/range — desugars to N scalar
                // `NetKind::String` element nets (`<name>$sae$<i>`); `files[K]` (CONST K)
                // resolves to the K-th net. A dynamic (`string s[]`) / multi-dim /
                // init-pattern array stays loud (correct-or-loud) via the reject below.
                if d.range.is_none() && d.packed.is_empty() && !decl.unpacked.is_empty() {
                    // T1: a ZERO-BASED ASCENDING fixed string array routes to the
                    // DYNAMIC representation instead of N per-element nets. Measured
                    // capability parity (23 shapes, fixed vs dyn, iverilog-differential)
                    // shows dyn is a strict SUPERSET — it additionally supports a runtime
                    // index, `foreach`, a runtime element write and `.size()`, and since
                    // §4.5.220 fixed the dyn element BYTE select it no longer loses
                    // anything fixed had. Unifying is therefore a climb, not a trade.
                    //
                    // Restricted to `allow_string_init` scopes (module body + block-local)
                    // because the routed net needs a `new[n]` pre-size driven from the t0
                    // var-init flush, which only those scopes run. Generate / package /
                    // interface / port scopes keep the per-element-net path verbatim.
                    //
                    // T1-5: MULTI-dim (`string s[2][3]`) rides the same route — one FLAT
                    // row-major container, with `s[i][j]` flattened at each access. Every
                    // dimension must qualify (`fixed_string_geom`), because the flat
                    // container has no map for an axis whose bounds are not constant.
                    if allow_string_init {
                        if let Some(geom) = self.fixed_string_geom(&decl.unpacked) {
                            if self.route_fixed_string_array(decl, &geom, ports, body) {
                                continue;
                            }
                        }
                    }
                    if decl.unpacked.len() != 1 {
                        // Multi-dim that did NOT qualify for the route (a non-constant or
                        // negative bound, too many elements, or a scope that does not run
                        // the var-init flush): the per-element-net path below speaks one
                        // dimension only, so fall through to the dimension reject rather
                        // than silently building storage for the first dimension alone.
                    } else if let Some((min, max)) = self.fixed_dim_bounds(&decl.unpacked[0]) {
                        // r19: a `'{…}` decl-init EXPANDS to per-element assignments in
                        // the t0 var-init pre-sweep (`fixed_string_array_init_pairs`,
                        // collected by `collect_var_init_drivers` / the block-local
                        // hoist). Accept it here so the element nets below still get
                        // created; anything the expansion cannot express — a wrong
                        // element count (iverilog rejects that too), a non-`'{…}` init,
                        // or a scope that does not run the flush — stays LOUD rather
                        // than being silently dropped.
                        if let Some(init) = &decl.init {
                            let ok = allow_string_init
                                && self
                                    .string_array_init_pairs(&decl.name, &decl.unpacked, init)
                                    .is_some();
                            if !ok {
                                self.error(
                                    MsgCode::ElabUnsupported,
                                    "a string-array initializer is supported only as a \
                                     `'{…}` pattern with one element per declared index, \
                                     at module or block scope (else assign elements in an \
                                     initial block)",
                                );
                                continue;
                            }
                        }
                        let dir = self.dir_for_name(&decl.name.name, ports, body);
                        if dir != ir::PortDir::Internal {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a string array cannot be a port (outside the v7 scope)",
                            );
                            continue;
                        }
                        // CHECKED + capped: `max - min` overflows i64 on a pathological
                        // declaration (`string s[9223372036854775807:-9223372036854775807]`
                        // — iverilog asserts on it too) and the bare subtraction PANICKED,
                        // which is below loud on the accuracy ladder. Element count also
                        // has to respect the array cap before it is used as a `Vec`
                        // capacity.
                        let Some(n) = max
                            .checked_sub(min)
                            .and_then(|d| u64::try_from(d).ok())
                            .and_then(|d| d.checked_add(1))
                            .filter(|&d| d <= MAX_ARRAY_LEN)
                            .map(|d| d as usize)
                        else {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "string array `{}` declares more than the v1 element \
                                     cap ({MAX_ARRAY_LEN})",
                                    decl.name.name
                                ),
                            );
                            continue;
                        };
                        let mut elem_ids = Vec::with_capacity(n);
                        for i in 0..n {
                            let ename = format!("{}$sae${i}", decl.name.name);
                            let net = self.nets.len() as u32;
                            self.add_net(
                                &ename,
                                ir::NetVar {
                                    kind: ir::NetKind::String,
                                    width: 0,
                                    msb: 0,
                                    lsb: 0,
                                    signed: false,
                                    array_len: 0,
                                    dir: ir::PortDir::Internal,
                                    init: default_init(ast::NetVarKind::Reg, 1),
                                },
                            );
                            elem_ids.push(net);
                        }
                        let key = self.fq(&decl.name.name);
                        self.string_array_elems.insert(key, (min, max, elem_ids));
                        continue;
                    }
                }
                // N3 Phase 2: `string s[]` — a DYNAMIC ARRAY of strings. One DynArray
                // handle net; the string elements live in the engine heap as `is_str`
                // Values (`string_elem_dyn_nets` marker). A decl-init `'{…}` rides the
                // shared dyn-array `'{…}` flush like any dyn array. A FIXED string array
                // (`string s[0:1]`) took the element-net path above; a queue/assoc/multi-
                // dim string container stays loud.
                // T1-4: `string q[$]` joins it — an UNBOUNDED queue of strings. One
                // handle net and the same `string_elem_dyn_nets` marker (the engine's
                // `dyn_str_elem` is a per-NET flag, not a DynArray-only one); only the
                // `NetKind` differs.
                //
                // Admitting the dimension is NOT sufficient on its own, and the first
                // attempt at this shipped nothing: `q.size()` was right while every
                // element read back EMPTY, because the queue push/insert paths did their
                // own `.resize(w)` — width 0 → 1 → the byte string truncated to a bit.
                // They now share `coerce_dyn_elem` with the dyn-array element write.
                //
                // T1-6: a BOUNDED queue (`string q[$:N]`) joins it. The bound is enforced
                // in the ENGINE, keyed on the net and blind to the element type — the
                // integral twin `int q[$:1]` already ran and dropped its tail — so the
                // only thing that kept `string q[$:3]` loud was this pattern admitting
                // `Queue(None)` alone. It shares `queue_dim_bound` with that twin.
                let str_container_kind = match decl.unpacked.first() {
                    Some(ast::Dim::Dyn) if decl.unpacked.len() == 1 => Some(ir::NetKind::DynArray),
                    Some(ast::Dim::Queue(_)) if decl.unpacked.len() == 1 => {
                        Some(ir::NetKind::Queue)
                    }
                    _ => None,
                };
                if d.range.is_none() && d.packed.is_empty() && str_container_kind.is_some() {
                    let kind = str_container_kind.expect("guarded by is_some above");
                    let Ok(queue_bound) = self.queue_dim_bound(&decl.unpacked[0]) else {
                        continue;
                    };
                    let dir = self.dir_for_name(&decl.name.name, ports, body);
                    if dir != ir::PortDir::Internal {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "a string dynamic array cannot be a port (outside the v7 scope)",
                        );
                        continue;
                    }
                    let next_id = self.nets.len() as u32;
                    self.add_net(
                        &decl.name.name,
                        ir::NetVar {
                            kind,
                            width: 0,
                            msb: 0,
                            lsb: 0,
                            signed: false,
                            array_len: 0,
                            dir: ir::PortDir::Internal,
                            init: default_init(ast::NetVarKind::Reg, 1),
                        },
                    );
                    if self.nets.len() as u32 > next_id {
                        self.string_elem_dyn_nets.insert(next_id);
                        if let Some(b) = queue_bound {
                            self.queue_bounds.insert(next_id, b);
                        }
                    }
                    // A `'{…}` decl-init rides the var-init flush (`new[N]` + element
                    // writes), collected in `collect_var_init_drivers`; a whole-value /
                    // non-pattern init has no surface → loud. `allow_string_init` gates
                    // it to the scopes that run the flush (module body / block-local).
                    if let Some(init) = &decl.init {
                        if !allow_string_init
                            || !matches!(init.kind, ast::ExprKind::AssignPattern(_))
                        {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a `string s[]` initializer is supported only as a `'{…}` \
                                 pattern at module scope (else use `new[]` / element writes)",
                            );
                        }
                    }
                    continue;
                }
                if d.range.is_some() || !d.packed.is_empty() || !decl.unpacked.is_empty() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a string variable takes no packed/unpacked dimensions (v7)",
                    );
                    continue;
                }
                // A `string s = expr;` initializer is a one-time t0 assignment,
                // equivalent to `initial s = expr;`. In the module body the net is
                // registered here and the init is collected (in declaration order)
                // by `collect_var_init_drivers`, then drained into the synthesized
                // pre-sweep `initial` (§6.8); the string heap holds no foldable
                // `init` field, so it always rides that path. A scope that does not
                // pass `allow_string_init` keeps STRING inits a loud reject —
                // correct-or-loud, never a silently-dropped init. (Generate and
                // interface bodies DO now run the array/non-const pre-sweep, but
                // their string decl-init remains a documented follow-on, so they
                // still pass `false` here.)
                if decl.init.is_some() && !allow_string_init {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a string declaration initializer is supported only at \
                         module scope (assign in an initial block here)",
                    );
                    continue;
                }
                let dir = self.dir_for_name(&decl.name.name, ports, body);
                if dir != ir::PortDir::Internal {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a string variable cannot be a port (outside the v7 scope)",
                    );
                    continue;
                }
                self.add_net(
                    &decl.name.name,
                    ir::NetVar {
                        kind: ir::NetKind::String,
                        width: 0,
                        msb: 0,
                        lsb: 0,
                        signed: false,
                        array_len: 0,
                        dir: ir::PortDir::Internal,
                        init: default_init(ast::NetVarKind::Reg, 1),
                    },
                );
                continue;
            }
            // ── v5 ⑥: dynamic-storage HANDLE declaration ──
            // `integer d[]` / `logic [7:0] q[$]` / `integer a[integer]`:
            // one dyn dim → a handle net (element width/signedness,
            // `array_len 0`, heap-backed). Engine slices ③④⑤ are the
            // storage; this is the front door.
            if let Some(handle_kind) = self.dyn_dim_kind(&decl.unpacked) {
                if decl.unpacked.len() != 1 {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a dynamic dimension cannot be mixed with other unpacked dimensions (nested dynamic storage is outside the MVP)",
                    );
                    continue;
                }
                // v6 ③: bounded queue `[$:N]` — fold N (a non-negative
                // const) into the sidecar; the engine truncates the TAIL of
                // any op that exceeds size N+1 (iverilog live).
                let Ok(queue_bound) = self.queue_dim_bound(&decl.unpacked[0]) else {
                    continue;
                };
                if let Some(init) = &decl.init {
                    // A queue / dynamic-array `'{…}` (or `{…}`) initializer is EXPANDED to
                    // runtime ops at t0 by the var-init flush (a push_back sequence /
                    // `new[N]` + element writes); allow it and register the handle.
                    // `allow_string_init` gates this to the scopes that pass it (module body
                    // / block-local). Generate and interface bodies now run the flush for
                    // ARRAY/non-const inits, but the QUEUE/dyn-array decl-init path there is
                    // a documented follow-on (the handle-net creation + in-scope expand is
                    // unverified), so they still pass `false` → a queue `'{…}` decl-init in
                    // those scopes stays a LOUD reject (correct-or-loud), never a silent
                    // drop. A `{…}` unpacked concatenation (§10.10) is the same element list
                    // as `'{…}` for a scalar-element queue/dyn array, so it rides the same
                    // path — EXCEPT a STRING-element array, whose `'{…}` path is a separate
                    // follow-on, so `string s[] = {…}` stays loud (never silent-empty). Any
                    // other init (a whole-value copy, a non-pattern expr) or an assoc array
                    // stays loud too — those use runtime methods.
                    let init_is_pattern = match &init.kind {
                        ast::ExprKind::AssignPattern(_) => true,
                        ast::ExprKind::Concat { .. } => !matches!(d.kind, ast::NetVarKind::String),
                        _ => false,
                    };
                    // §7.5.1: `byte b[] = new[20];` — a sizing constructor is a legal
                    // dynamic-array declaration initializer, and it needs no expansion at
                    // all: the generic flush emits `b = new[20]`, the very statement the
                    // split-decl workaround wrote by hand. Rejecting it here `continue`d
                    // past the handle registration, so ONE declaration produced eight
                    // diagnostics — including two "undeclared `t.aa_key`" for a name
                    // declared on the line above. A `new[n](src)` copy-constructor rides
                    // the same statement path. Queues/assoc take no `new[]` (loud below).
                    let init_is_new = matches!(init.kind, ast::ExprKind::New { .. })
                        && handle_kind == ir::NetKind::DynArray;
                    let desugared = init_is_new
                        || (allow_string_init
                            && init_is_pattern
                            && matches!(handle_kind, ir::NetKind::Queue | ir::NetKind::DynArray));
                    if !desugared {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "a dynamic-storage handle takes no initializer except a `'{…}` pattern on a queue/dynamic array (whole-value copy/other inits use `new[]`/methods)",
                        );
                        continue;
                    }
                }
                // N3 Phase 2: a `real r[]` DYNAMIC ARRAY is supported (element-real
                // heap). A real/realtime element in a QUEUE/ASSOC, and `event`
                // anywhere, stay loud (correct-or-loud — not yet plumbed).
                let is_real_dyn =
                    matches!(d.kind, ast::NetVarKind::Real | ast::NetVarKind::Realtime)
                        && handle_kind == ir::NetKind::DynArray;
                if matches!(d.kind, ast::NetVarKind::Event)
                    || (matches!(d.kind, ast::NetVarKind::Real | ast::NetVarKind::Realtime)
                        && !is_real_dyn)
                {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "real/event elements in dynamic storage are outside the MVP",
                    );
                    continue;
                }
                if !net_is_variable(d.kind) && !is_real_dyn {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "dynamic storage must be a VARIABLE kind (reg/logic/integer/time), not a net",
                    );
                    continue;
                }
                let dir = self.dir_for_name(&decl.name.name, ports, body);
                if dir != ir::PortDir::Internal {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a dynamic-storage handle cannot be a port (outside the MVP)",
                    );
                    continue;
                }
                let next_id = self.nets.len() as u32;
                self.add_net(
                    &decl.name.name,
                    ir::NetVar {
                        kind: handle_kind,
                        width,
                        msb,
                        lsb,
                        signed,
                        array_len: 0, // the handle marker — elements live in the engine heap
                        dir,
                        init: default_init(d.kind, width),
                    },
                );
                if self.nets.len() as u32 > next_id {
                    if let Some(b) = queue_bound {
                        self.queue_bounds.insert(next_id, b);
                    }
                    // IEEE §7.5.2: a 2-state element type defaults to 0, not X.
                    // The handle skips `record_dim_desc`, so flag it here so the
                    // `two_state_nets` sidecar reaches the engine's `new[]` fill.
                    if net_kind_is_two_state(d.kind) {
                        self.two_state_heap_handles.insert(next_id);
                    }
                    // N3 Phase 2: a `real r[]` element-real dyn array — the engine
                    // flags the net `is_real` and fills `new[]` with 0.0.
                    if is_real_dyn {
                        self.real_elem_dyn_nets.insert(next_id);
                    }
                }
                continue;
            }
            // ── N7: class-handle declaration (`Packet p;`) ──
            // A scalar 32-bit object-id net (0 = null), recorded in
            // `class_handle_nets` (engine routing) + `net_class` (static type for
            // field/method resolution). The object lives in the engine heap.
            if matches!(d.kind, ast::NetVarKind::ClassHandle) {
                if d.range.is_some() || !d.packed.is_empty() || !decl.unpacked.is_empty() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a class handle takes no packed/unpacked dimensions (N7 MVP)",
                    );
                    continue;
                }
                if decl.init.is_some() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a class-handle declaration initializer is outside the N7 MVP \
                         (assign `new`/`null` in a procedural block)",
                    );
                    continue;
                }
                let cname = match &d.class_type {
                    Some(c) if self.class_table.contains_key(&c.name) => c.name.clone(),
                    Some(c) => {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!("`{}` does not name a class", c.name),
                        );
                        continue;
                    }
                    None => {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "class handle without a class type",
                        );
                        continue;
                    }
                };
                let dir = self.dir_for_name(&decl.name.name, ports, body);
                if dir != ir::PortDir::Internal {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a class handle cannot be a port (N7 MVP)",
                    );
                    continue;
                }
                let next_id = self.nets.len() as u32;
                self.add_net(
                    &decl.name.name,
                    ir::NetVar {
                        kind: ir::NetKind::Integer,
                        width: 32,
                        msb: 31,
                        lsb: 0,
                        signed: false,
                        array_len: 1,
                        dir: ir::PortDir::Internal,
                        init: default_init(ast::NetVarKind::ClassHandle, 32),
                    },
                );
                if self.nets.len() as u32 > next_id {
                    self.class_handle_nets.insert(next_id);
                    self.net_class.insert(next_id, cname);
                }
                continue;
            }
            let dim_extents = self.array_dim_extents(&decl.unpacked);
            let array_len = dim_extents
                .iter()
                .fold(1u32, |acc, &(_, n)| acc.saturating_mul(n.max(1)));
            // P2-6: cap the unpacked element count like MAX_NET_WIDTH caps the
            // vector width — `reg [7:0] m [0:2147483647]` would otherwise try a
            // multi-GB allocation at t0 (OS OOM kill, no diagnostic).
            if (array_len as u64) > MAX_ARRAY_LEN {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "unpacked array `{}` has {} elements (cap {MAX_ARRAY_LEN})",
                        decl.name.name, array_len
                    ),
                );
                continue;
            }
            let dir = self.dir_for_name(&decl.name.name, ports, body);
            // init: const-fold the initializer (§6.8: a ONE-TIME value at creation).
            // A literal folds directly; a non-literal CONSTANT (param / constant
            // expression, e.g. `int x = P;` or `reg [31:0] y = A+B;`) is const-eval'd
            // — previously only literals folded, so a param/expr initializer silently
            // defaulted to X/0 (a now-fixed pre-existing silent-wrong, and the value
            // that the 2-state `is_var` change relies on instead of a cont-assign).
            // A non-foldable init defaults here and rides the §6.8 pre-sweep
            // instead: the module body collects it via `collect_var_init_drivers`,
            // and a PACKAGE body does the same (A2b — `elaborate_pkg_netvar`
            // collects, `elaborate_package` flushes its own pre-sweep initial
            // whose ProcId precedes every module process).
            // §4.5.257: the DEFAULT only. A declaration initializer — constant or not — is
            // an ASSIGNMENT that happens in the static-initialization phase, in scope
            // order, and pre-applying a constant here took it out of that order: a
            // generate initializer reading a module `int mm = 77;` saw 77, while the same
            // read of a NON-constant `int mm = f();` correctly saw 0. vita disagreed with
            // itself depending on whether the initializer happened to fold. The value now
            // always rides the pre-sweep, which the engine runs before arming anything, so
            // it still produces no event and still lands before every user process.
            let init = default_init(d.kind, width);
            self.add_net(
                &decl.name.name,
                ir::NetVar {
                    kind: map_net_kind_or_wire(d.kind),
                    width,
                    msb,
                    lsb,
                    signed,
                    array_len,
                    dir,
                    init,
                },
            );
            if matches!(d.kind, ast::NetVarKind::Event) {
                let key = self.fq(&decl.name.name);
                if let Some(&id) = self.symbols.get(&key) {
                    self.event_nets.insert(id);
                }
            }
            // WAND/WOR: record the wired-AND / wired-OR resolution kind for the
            // engine (looked up post-add, robust to a duplicate-decl skip). The
            // net itself is a plain Wire in the IR; the sidecar drives multi-driver
            // resolution only.
            if matches!(d.kind, ast::NetVarKind::Wand | ast::NetVarKind::Wor) {
                let key = self.fq(&decl.name.name);
                if let Some(&id) = self.symbols.get(&key) {
                    if matches!(d.kind, ast::NetVarKind::Wand) {
                        self.wired_and_nets.insert(id);
                    } else {
                        self.wired_or_nets.insert(id);
                    }
                }
            }
            // A2a: a DESUGARED array parameter — register the net as an
            // elaboration constant so every later write stays loud (looked up
            // post-add like the sidecars above). Elaborate-local only.
            if d.const_param {
                // A non-Internal dir means the name collided with a port decl
                // (`module m(R); input R; localparam int R[…] = …;`) — the
                // port copy-in would drive the net through a deny-free
                // ContAssign (adversarial find), so reject the merge itself.
                if dir != ir::PortDir::Internal {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!("an array parameter `{}` cannot be a port", decl.name.name),
                    );
                }
                let key = self.fq(&decl.name.name);
                if let Some(&id) = self.symbols.get(&key) {
                    self.const_param_nets.insert(id, decl.name.name.clone());
                }
            }
            // Record per-dim extents when addressing is NOT plain 0-based: any
            // MULTI-dim array, OR a 1-D array with a non-zero lower bound (`mem[4:7]`).
            // A plain 0-based 1-D array stays ABSENT so its lowering falls back to
            // `[(0, array_len)]` — byte-identical to the long-standing golden IR.
            // Keyed by the just-assigned NetId (looked up post-add so a duplicate-skip
            // does not mis-key). (a)-flattening: no frozen-IR field added.
            // Twin of the extents record above, for a NEGATIVE packed low bound: the net
            // was sized `|msb-lsb|+1` and stored normalized as `[w-1:0]`, so this is what
            // lets a bit/part select address the declared numbering. Recorded post-add for
            // the same reason (a duplicate-decl skip must not mis-key).
            if let Some(l) = neg_lsb.or(asc_lsb) {
                let key = self.fq(&decl.name.name);
                if let Some(&id) = self.symbols.get(&key) {
                    // Direction picks the map: the two normalizations are mirror
                    // images (`i - l` vs `l - i`), so one map cannot serve both.
                    if neg_lsb.is_some() {
                        self.net_decl_neg_lsb.insert(id, l);
                    } else {
                        self.net_decl_asc_lsb.insert(id, l);
                    }
                    // …and the full declared pair, for the VCD `$var` label (the stored
                    // range is normalized, so without this a waveform viewer numbers the
                    // bits `[5:0]` where iverilog numbers them `[3:-2]`).
                    if let Some(m) = d
                        .range
                        .as_ref()
                        .and_then(|r| self.const_range_bound_fold(&r.msb))
                    {
                        self.net_decl_range.insert(id, (m, l));
                    }
                }
            }
            if dim_extents.len() >= 2 || dim_extents.iter().any(|&(lo, _)| lo != 0) {
                let key = self.fq(&decl.name.name);
                if let Some(&id) = self.symbols.get(&key) {
                    self.array_dims.insert(id, dim_extents);
                }
            }
            // Declared per-dim DIRECTION for array-assignment correspondence
            // (sparse: only when some dim is descending, e.g. `mem[3:0]`).
            let desc: Vec<bool> = decl
                .unpacked
                .iter()
                .map(|d| match d {
                    ast::Dim::Range(r) => {
                        let msb = self.const_range_bound_fold(&r.msb);
                        let lsb = self.const_range_bound_fold(&r.lsb);
                        matches!((msb, lsb), (Some(m), Some(l)) if m > l)
                    }
                    _ => false,
                })
                .collect();
            if desc.iter().any(|&d| d) {
                let key = self.fq(&decl.name.name);
                if let Some(&id) = self.symbols.get(&key) {
                    self.array_dim_desc.insert(id, desc);
                }
            }
            // SYS-INTRO dimension descriptor for $size/$left/.../$dimensions.
            if let Some(&id) = self.symbols.get(&self.fq(&decl.name.name)) {
                self.record_dim_desc(id, d.kind, d.range.as_ref(), &d.packed, &decl.unpacked);
            }
            // Declared array-ness — covers `[0:0]` (1-element) arrays that
            // `array_len > 1` cannot distinguish from scalars.
            if !decl.unpacked.is_empty() {
                let key = self.fq(&decl.name.name);
                if let Some(&id) = self.symbols.get(&key) {
                    self.unpacked_array_nets.insert(id);
                }
            }
            // Record packed-dim extents for a multi-dim packed net so a select can be
            // lowered to the right bit-slice.
            if !d.packed.is_empty() {
                let key = self.fq(&decl.name.name);
                if let Some(&id) = self.symbols.get(&key) {
                    self.packed_dims.insert(id, packed_ext.clone());
                }
            }
        }
    }

    /// The rand-field name of a pure `field OP const` / `const OP field` comparison
    /// (OP ∈ {<,<=,>,>=,==}) — the form captured by the `[lo,hi]` sampling domain.
    /// `None` for `!=` (not a range narrowing) and for inter-variable `field OP
    /// field` (the OTHER side must const-evaluate), which both need a predicate.
    pub(crate) fn single_range_field(&self, e: &ast::Expr) -> Option<String> {
        if let ast::ExprKind::Binary { op, lhs, rhs } = &e.kind {
            if matches!(
                op,
                ast::BinOp::Lt | ast::BinOp::Le | ast::BinOp::Gt | ast::BinOp::Ge | ast::BinOp::Eq
            ) {
                if let Some(f) = rand_field_ident(lhs) {
                    if self.const_eval_in_scope(rhs).is_some() {
                        return Some(f);
                    }
                }
                if let Some(f) = rand_field_ident(rhs) {
                    if self.const_eval_in_scope(lhs).is_some() {
                        return Some(f);
                    }
                }
            }
        }
        None
    }

    pub(crate) fn elaborate_cont_assign(&mut self, ca: &ast::ContinuousAssign) {
        // Delay: hdl-ast Delay values are exprs; sim-ir delay is Option<u32>.
        // The FROZEN `ContAssign.delay` keeps `Some(rise)` (values[0]) so the
        // "has delay" check and the uniform fast-path are untouched. S1: a
        // distinct rise/fall/turnoff rides the `ca_delays` sidecar.
        let (delay, rft) = self.fold_ca_delay(ca.delay.as_ref());
        for (lv, rhs) in &ca.assigns {
            let lhs = self.lower_lvalue(lv);
            // P1-9 (E3018): a user `assign` may not drive a Reg/Integer/Real
            // variable (SV `logic` admits one continuous driver — passes). Port
            // bindings / decl-inits are NOT routed here (IEEE 1800 var-port and
            // legacy `reg r = init` forms stay accepted).
            self.check_lvalue_kind(&lhs, false);
            let rhs_id = self.lower_expr(rhs);
            let rhs_id = self.resize_rhs_for_lvalue(rhs, rhs_id, &lhs);
            // The index of THIS cont-assign is the len BEFORE the push.
            let idx = self.cont_assigns.len() as u32;
            if let Some(rft) = rft {
                self.ca_delays.insert(idx, rft);
            }
            // R14: `assign` — the user wrote it, so the identity is the
            // statement's own span.
            self.push_cont_assign(
                ir::ContAssign {
                    lhs,
                    rhs: rhs_id,
                    delay,
                },
                "assign",
                Some(ca.span),
            );
        }
    }
}
