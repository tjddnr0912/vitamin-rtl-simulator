//! ENG-1b: the TYPE of a net vita derives to hold a value on the design's behalf.
//!
//! An intra-assignment capture temp (`lhs = #d rhs`, `= @(e)`, `<= @(e)`, the
//! `$random` / `$dist_*` / `$cast` intercept, `$value$plusargs`), the string-queue
//! self-pattern snapshot, the `@(ra[1])` select holder, the SVA previous-value
//! register and the clocking sample / drive holders each copy a value that lives in
//! some STORAGE location — the destination the capture feeds, or the source the
//! holder copies. The holder takes that storage's type (kind, width, signedness,
//! unpacked shape), read by ONE classifier, [`Elaborator::held_of_storage`],
//! exhaustive over `NetKind` with no catch-all. A storage the holder cannot carry (a
//! whole unpacked array, a queue, a dynamic or associative array) gets no holder and
//! the producer refuses it by name.
//!
//! Before this, every holder was an unsigned integral `reg` at some width: `r = #1 i`
//! with `int i = -5` stored 1.8446744073709552e19 into a `real` (verilator `-5`, and
//! vita's own `r = i` printed `-5`); a string capture came back empty; `@(ra[1])` woke
//! on `round()` of the element; `$past` of an `int` read back unsigned.

use super::*;

/// What a derived holder holds, read from one storage location.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Held {
    /// Integral bits — the producer's integral holder, at the storage's width and
    /// signedness (a string's byte is one).
    Integral,
    /// A `real` / `realtime`: a `NetKind::Real` holder (64 bits, `+0.0` initial).
    Real,
    /// A `string`: a `NetKind::String` holder.
    String,
    /// No holder can carry it; the text names the storage for the refusal.
    NoHolder(&'static str),
    /// The storage does not exist yet: a deferred hierarchical write sentinel or read
    /// placeholder (or `POISON_NET`, already reported). The producer records it and the
    /// resolution verifies it — never read as integral.
    Deferred,
}

/// A clocking item that got no holder: its `cb.x` alias is not created, and a use of
/// the name is refused with `msg`.
pub(crate) struct ClkRefused {
    /// The alias's fully-qualified name (`top.cb.q`).
    pub(crate) alias: String,
    pub(crate) msg: String,
    /// A clocking INPUT (read-only, §14.3), not an output.
    pub(crate) input: bool,
    /// Spans already refused, one line per span: two bare uses in one statement share
    /// the statement's span and print once; each select (`cb.m[0]`) has its own span
    /// and prints its own line.
    pub(crate) reported: Vec<(u32, u32)>,
}

/// A capture temp whose destination is a deferred hierarchical write: retyped to the
/// destination's storage when the write resolves ([`Elaborator::verify_deferred_captures`]).
pub(crate) struct DeferredCapture {
    pub(crate) tmp: u32,
    pub(crate) sentinel: u32,
    /// The destination as written, for the refusal.
    pub(crate) dest: String,
}

impl Elaborator<'_> {
    /// The type of one storage location: a whole net (`word == false`) or one element
    /// of it (`word == true`).
    pub(crate) fn held_of_storage(&self, net: u32, word: bool) -> Held {
        let Some(nv) = self.nets.get(net as usize) else {
            return Held::Deferred;
        };
        let elem = |n: u32| {
            if self.string_elem_dyn_nets.contains(&n) {
                Held::String
            } else if self.real_elem_dyn_nets.contains(&n) {
                Held::Real
            } else {
                Held::Integral
            }
        };
        match nv.kind {
            ir::NetKind::Real if !word && nv.array_len > 1 => {
                Held::NoHolder("an unpacked array of `real`")
            }
            ir::NetKind::Real => Held::Real,
            ir::NetKind::String if word => Held::Integral,
            ir::NetKind::String => Held::String,
            ir::NetKind::DynArray if !word => Held::NoHolder("a dynamic array"),
            ir::NetKind::Queue if !word => Held::NoHolder("a queue"),
            ir::NetKind::Assoc | ir::NetKind::AssocStr if !word => {
                Held::NoHolder("an associative array")
            }
            ir::NetKind::DynArray
            | ir::NetKind::Queue
            | ir::NetKind::Assoc
            | ir::NetKind::AssocStr => elem(net),
            ir::NetKind::Wire | ir::NetKind::Reg | ir::NetKind::Logic | ir::NetKind::Integer
                if !word && nv.array_len > 1 =>
            {
                Held::NoHolder("an unpacked array")
            }
            ir::NetKind::Wire | ir::NetKind::Reg | ir::NetKind::Logic | ir::NetKind::Integer => {
                Held::Integral
            }
        }
    }

    /// The type of an assignment DESTINATION: one whole chunk (a net, or one element)
    /// is that storage; a bit / part select or a concatenation is integral bits at the
    /// lvalue width.
    pub(crate) fn held_of_lvalue(&self, lv: &ir::Lvalue) -> Held {
        match lv.chunks.as_slice() {
            [c] if c.offset.is_none() && c.width.is_none() => {
                self.held_of_storage(c.net, c.word.is_some())
            }
            _ => Held::Integral,
        }
    }

    /// The net of a `real` or `string` holder (`Integral` / `NoHolder` / `Deferred` have
    /// no typed form and are the caller's): a `real` starts at `+0.0`, a `string` empty.
    fn typed_holder_net(held: Held) -> Option<ir::NetVar> {
        match held {
            Held::Real => Some(ir::NetVar {
                kind: ir::NetKind::Real,
                width: 64,
                msb: 63,
                lsb: 0,
                signed: true,
                array_len: 1,
                dir: ir::PortDir::Internal,
                init: default_init(ast::NetVarKind::Real, 64),
            }),
            Held::String => Some(ir::NetVar {
                kind: ir::NetKind::String,
                width: 0,
                msb: 0,
                lsb: 0,
                signed: false,
                array_len: 0,
                dir: ir::PortDir::Internal,
                init: default_init(ast::NetVarKind::Reg, 1),
            }),
            Held::Integral | Held::NoHolder(_) | Held::Deferred => None,
        }
    }

    /// A `real` or `string` holder named `name`; `None` for any other `held`, or when the
    /// arena refused it.
    pub(crate) fn add_typed_holder(&mut self, name: &str, held: Held) -> Option<u32> {
        let nv = Self::typed_holder_net(held)?;
        let id = self.nets.len() as u32;
        self.add_net(name, nv);
        (self.nets.len() as u32 > id).then_some(id)
    }

    /// The capture temp of an intra-assignment timing control into `lv`: it takes the
    /// DESTINATION's type, so `tmp = rhs` is the same assignment (same context, same
    /// conversion) as `lv = rhs` and `lv = tmp` a same-type copy (IEEE 1800-2017 §9.4.5,
    /// Table 9-3). A select or concatenation destination keeps the integral temp at
    /// the lvalue width (`ia_capture_width`).
    pub(crate) fn fresh_capture_for(&mut self, lv: &ir::Lvalue, rhs_id: u32) -> u32 {
        match self.held_of_lvalue(lv) {
            h @ (Held::Real | Held::String) => {
                let name = format!("$ia_tmp${}", self.nets.len());
                self.add_typed_holder(&name, h).unwrap_or(POISON_NET)
            }
            Held::NoHolder(what) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "an intra-assignment delay or event control into {what} is \
                         unsupported: vita's capture holds one value, not this type"
                    ),
                );
                self.fresh_ia_tmp(1)
            }
            Held::Integral => {
                let w = self.ia_capture_width(lv, rhs_id);
                self.fresh_ia_tmp(w)
            }
            Held::Deferred => {
                let w = self.ia_capture_width(lv, rhs_id);
                let tmp = self.fresh_ia_tmp(w);
                if let [c] = lv.chunks.as_slice() {
                    self.record_deferred_capture(tmp, c.net);
                }
                tmp
            }
        }
    }

    /// Record a capture whose destination chunk is a deferred hierarchical write
    /// (`u.r = #1 x`): the temp is integral until the write resolves.
    fn record_deferred_capture(&mut self, tmp: u32, sentinel: u32) {
        let dest = if (HIER_WRITE_SENTINEL_BASE..POISON_NET).contains(&sentinel) {
            self.deferred_hier_write
                .get((sentinel - HIER_WRITE_SENTINEL_BASE) as usize)
                .map(|d| d.path.join("."))
        } else if (HIER_SEL_WRITE_SENTINEL_BASE..HIER_WRITE_SENTINEL_BASE).contains(&sentinel) {
            self.deferred_hier_sel_write
                .get((sentinel - HIER_SEL_WRITE_SENTINEL_BASE) as usize)
                .map(|d| format!("{}[…]", d.path.join(".")))
        } else {
            None
        };
        // POISON_NET (already reported) or a sentinel range exhausted: nothing to verify.
        if let Some(dest) = dest {
            self.deferred_captures.push(DeferredCapture {
                tmp,
                sentinel,
                dest,
            });
        }
    }

    /// Verify every capture recorded by [`Self::fresh_capture_for`] against the
    /// destination its hierarchical write resolved to: a `real` / `string`
    /// destination retypes the temp (nothing between the capture's lowering and here
    /// reads the temp's type: the engine converts at the assignment by the lvalue's
    /// kind, which is the temp's), a whole unpacked / dynamic destination is refused
    /// by name. Runs after both write resolvers.
    pub(crate) fn verify_deferred_captures(&mut self) {
        let recs = std::mem::take(&mut self.deferred_captures);
        for d in recs {
            let Some(c) = self.hier_resolved_chunk.get(&d.sentinel).cloned() else {
                continue; // never resolved: the resolver reported it
            };
            let held = self.held_of_lvalue(&ir::Lvalue { chunks: vec![c] });
            if let Held::NoHolder(what) = held {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "an intra-assignment delay or event control into `{}`, {what}, \
                         is unsupported: vita's capture holds one value, not this type",
                        d.dest
                    ),
                );
                continue;
            }
            // `Integral` keeps the temp; `Deferred` here is POISON_NET, already reported.
            let Some(nv) = Self::typed_holder_net(held) else {
                continue;
            };
            if let Some(slot) = self.nets.get_mut(d.tmp as usize) {
                *slot = nv;
            }
        }
    }

    /// The clocking item that got no holder named by `path` (or by a leading part of
    /// it: `cb.q.size`, `cb.s.len`) written in scope `prefix`. Found with the walk
    /// `lookup_net_scoped` finds a `cb.x` alias with (innermost wins, outward through a
    /// generate block, a function or task body, a named block), over the symbols that
    /// exist at lowering and the refused items together, so a use inside a function or
    /// generate block names the item, and an inner name the walk can see wins. The walk
    /// does not see a generate-local instance, interface or generate block named `cb`
    /// (its nets come after lowering, bound by the deferred hierarchical resolver) nor a
    /// generate-local clocking block (it gets no alias): a use meant for that inner
    /// object is refused naming the outer item (Q-CLKSHADOW).
    pub(crate) fn clk_refused_at(&self, prefix: &str, path: &[String]) -> Option<usize> {
        if self.clk_refused.is_empty() {
            return None;
        }
        let refused = |k: &str| self.clk_refused.iter().position(|r| r.alias == k);
        (2..=path.len()).find_map(|k| {
            let head = path[..k].join(".");
            let key = self.walk_scopes_key_from(prefix, &head, |k| {
                self.symbols.contains_key(k) || refused(k).is_some()
            })?;
            refused(&key)
        })
    }

    /// [`Self::clk_refused_at`] for a path written in the current scope.
    pub(crate) fn clk_refused_path(&self, path: &ast::HierPath) -> Option<usize> {
        let names: Vec<String> = path.segments.iter().map(|s| s.name.clone()).collect();
        self.clk_refused_at(&self.cur_prefix, &names)
    }

    /// Refuse a use of the refused clocking item `i`, at `at` (else the current span),
    /// in the scope `prefix` the use was written in: once per span, so two bare uses in
    /// one statement print one line and each select prints its own.
    pub(crate) fn report_clk_refused(&mut self, i: usize, at: Option<ast::Span>, prefix: &str) {
        let sp = at.or(self.cur_span);
        let Some(r) = self.clk_refused.get_mut(i) else {
            return;
        };
        if let Some(s) = sp {
            if r.reported.contains(&(s.lo, s.hi)) {
                return;
            }
            r.reported.push((s.lo, s.hi));
        }
        let msg = r.msg.clone();
        let saved = std::mem::replace(&mut self.cur_prefix, prefix.to_string());
        match sp {
            Some(s) => self.error_at(MsgCode::ElabUnsupported, s, &msg),
            None => self.error(MsgCode::ElabUnsupported, &msg),
        }
        self.cur_prefix = saved;
    }

    /// The written root of an assignment target (`cb.q`, `cb.q[0]`, `cb.v[3:0]`).
    pub(crate) fn clk_refused_lvalue(&self, lv: &ast::Lvalue) -> Option<usize> {
        match lv {
            ast::Lvalue::Ident(p) => self.clk_refused_path(p),
            ast::Lvalue::BitSelect { base, .. }
            | ast::Lvalue::PartSelect { base, .. }
            | ast::Lvalue::IndexedPart { base, .. } => self.clk_refused_lvalue(base),
            ast::Lvalue::Concat { parts, .. } => {
                parts.iter().find_map(|p| self.clk_refused_lvalue(p))
            }
            ast::Lvalue::Error(_) => None,
        }
    }
}

impl Elaborator<'_> {
    /// Record a clocking item that gets no holder: `cb.item` binds the signal `src`,
    /// whose type `held` no clocking holder carries. Nothing is sampled or driven for
    /// it and no `cb.item` alias is created, so an unused item is harmless (IEEE
    /// 1800-2017 §14.3 declares it; nothing reads it) and a use of the name is
    /// refused with this text, at the use.
    pub(crate) fn refuse_clocking_item(
        &mut self,
        cb: &str,
        item: &str,
        src: &str,
        held: Held,
        input: bool,
    ) {
        let what = match held {
            Held::String => "a string",
            Held::NoHolder(what) => what,
            Held::Deferred => "a signal not resolved here",
            Held::Integral | Held::Real => return,
        };
        let msg = if input {
            format!(
                "clocking input `{cb}.{item}` samples `{src}`, {what}: vita's clocking \
                 sample holds one integral or `real` value, not this type (follow-on)"
            )
        } else {
            format!(
                "clocking output `{cb}.{item}` drives `{src}`, {what}: vita's clocking \
                 drive holds one integral or `real` value, not this type (follow-on)"
            )
        };
        let alias = self.fq(&format!("{cb}.{item}"));
        self.clk_refused.push(ClkRefused {
            alias,
            msg,
            input,
            reported: Vec::new(),
        });
    }

    /// `$readmem*` into a clocking INPUT that has no holder: the read-only refusal
    /// keeps precedence over the by-name one (the item's type is moot — a clocking
    /// input is never written), and it is the only error for the argument.
    pub(crate) fn deny_refused_clocking_input_arg(&mut self, e: &ast::Expr, how: &str) -> bool {
        let mut inner = e;
        while let ast::ExprKind::Paren { inner: i } = &inner.kind {
            inner = i;
        }
        let ast::ExprKind::Ident(path) = &inner.kind else {
            return false;
        };
        let joined: Vec<&str> = path.segments.iter().map(|s| s.name.as_str()).collect();
        if !self
            .clk_refused_path(path)
            .is_some_and(|i| self.clk_refused[i].input)
        {
            return false;
        }
        self.error(
            MsgCode::ElabUnsupported,
            &format!(
                "cannot {how} a clocking INPUT (`{}` is read-only, §14.3)",
                joined.join(".")
            ),
        );
        true
    }
}
