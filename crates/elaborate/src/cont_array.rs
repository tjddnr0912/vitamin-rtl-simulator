//! A whole fixed-size unpacked array as the target of a continuous assignment (IEEE
//! 1800-2017 §10.3, §7.6): `assign a = b;`, `assign a = '{default: v};` and
//! `assign a = '{e0, …};`.
//!
//! This IR has no whole-array value, so the assignment becomes one continuous assign per
//! element. That is the representation `wire_array_port` already uses for an array port,
//! and it is what `assign a[i] = …;` written once per element produces. The element
//! pairing and the pattern expansions are the procedural array assignment's
//! (`array_assign_special`): position correspondence by `residual_word_offsets`,
//! `flatten_assign_pattern`, `expand_array_default_pattern`, and one §7.6 rule for an array
//! source ([`Elaborator::array_copy_mismatch`]).
//!
//! The lowering is kept only when the `assign` is the array's ONLY writer. With any other
//! writer the element-wise form reaches behaviour this IR does not model: a `wire` array
//! element with two continuous drivers is not resolved (iverilog reads `x` where the last
//! driver wins here), and a variable with a continuous and a procedural writer is refused
//! by iverilog while verilator runs it. [`Elaborator::check_whole_array_sole_writer`] makes
//! that case loud once every writer exists.

use super::*;

/// One whole-array `assign`, recorded for the sole-writer check: its element continuous
/// assigns are `cont_assigns[lo..hi]`.
pub(crate) struct WholeArrayCa {
    pub(crate) net: u32,
    pub(crate) lo: usize,
    pub(crate) hi: usize,
    pub(crate) span: ast::Span,
    pub(crate) prefix: String,
}

/// The value an element continuous assign reads.
enum ElemSource<'a> {
    /// Pattern leaves, one per element in declared order.
    Leaves(Vec<&'a ast::Expr>),
    /// The same position of a whole source array: `(net, word offset per position)`.
    Array(u32, Vec<u32>),
}

impl Elaborator<'_> {
    /// IEEE 1800 §7.6: the source of an unpacked-array assignment must have the target's
    /// unpacked dimensions and an equivalent element type (width, realness and signedness).
    /// `Some(message)` names the first mismatch. Shared by the procedural array assignment
    /// and the continuous one so the two cannot drift.
    pub(crate) fn array_copy_mismatch(
        &self,
        t_net: u32,
        t_res: &[(i64, u32)],
        s_net: u32,
        s_res: &[(i64, u32)],
    ) -> Option<&'static str> {
        if t_res.len() != s_res.len()
            || t_res.iter().zip(s_res).any(|(&(_, ts), &(_, ss))| ts != ss)
        {
            return Some(
                "unpacked-array assignment requires the same number of dimensions \
                 and the same size per dimension (IEEE 1800 §7.6)",
            );
        }
        let (tw, tk, tsg) = {
            let nv = &self.nets[t_net as usize];
            (nv.width, nv.kind, nv.signed)
        };
        let (sw, sk, ssg) = {
            let nv = &self.nets[s_net as usize];
            (nv.width, nv.kind, nv.signed)
        };
        // §6.22.2 equivalent element types: width, realness AND signedness
        // (a raw word copy would be bit-correct either way, but accepting a
        // signed/unsigned mix would silently diverge from conformant tools).
        if tw != sw || (tk == ir::NetKind::Real) != (sk == ir::NetKind::Real) || tsg != ssg {
            return Some(
                "unpacked-array assignment requires identical element types \
                 (IEEE 1800 §7.6)",
            );
        }
        None
    }

    /// `assign lv = rhs;` where `lv` names a WHOLE fixed-size unpacked array and `rhs` is a
    /// positional pattern, `'{default: v}`, or another whole array. Returns `true` when the
    /// assignment was consumed (lowered element by element, or refused with the procedural
    /// twin's message), `false` to leave it to the scalar funnel, whose E3009 is the answer
    /// every other shape had before: a sub-array target (`a[1] = …` on a 2-D array), a
    /// sub-array source, any other right-hand side, a `real` element, and a target that is
    /// also the source.
    pub(crate) fn cont_assign_whole_array(
        &mut self,
        lv: &ast::Lvalue,
        rhs: &ast::Expr,
        span: ast::Span,
    ) -> bool {
        const ARRAY_CONT_UNROLL_CAP: u64 = 4096;
        let Some((t_net, t_lead)) = self.lval_array_view(lv) else {
            return false;
        };
        if !t_lead.is_empty() {
            return false;
        }
        let s_array = match &rhs.kind {
            ast::ExprKind::AssignPattern(_) | ast::ExprKind::AssignPatternKeyed(_) => None,
            _ => match self.expr_array_view(rhs) {
                Some((s_net, s_lead)) if s_lead.is_empty() && s_net != t_net => Some(s_net),
                _ => return false,
            },
        };
        // Excluded before any side effect, so the scalar funnel answers exactly as
        // before: a `real` element; a net declared with a delay (`wire #2 w [2];`),
        // which this IR applies only to a net-declaration assignment; an element type
        // not written in the array's own declaration (`inline_elem_type`: an enum, a
        // struct, a typedef or a type parameter, whose 2-state-ness and enum identity
        // this IR does not keep — `enum bit [7:0]` elements read `x` where both oracles
        // read 0, and an enum array copied to a vector is refused by iverilog); and a
        // copy between a 2-state and a 4-state element type, which §7.6 does not call
        // equivalent (both oracles refuse `logic [7:0] d [2]; bit [7:0] s [2]; assign
        // d = s;`).
        if self.nets[t_net as usize].kind == ir::NetKind::Real
            || self.delayed_decl_nets.contains(&t_net)
            || !self.inline_elem_arrays.contains(&t_net)
        {
            return false;
        }
        if let Some(s_net) = s_array {
            if !self.inline_elem_arrays.contains(&s_net)
                || self.net_is_two_state(t_net) != self.net_is_two_state(s_net)
            {
                return false;
            }
        }
        // The element chunks are built by hand, bypassing `collect_lval_chunks` — re-run
        // its two write rules here, as `array_assign_special` does.
        if let Some(path) = lval_root_path(lv) {
            let path = path.clone();
            self.check_modport_write(&path);
        }
        self.deny_readonly_write(t_net, "assign to");
        let t_dims = self.net_dim_extents(t_net);
        let n = t_dims
            .iter()
            .fold(1u64, |a, &(_, s)| a.saturating_mul(s as u64));
        if n > ARRAY_CONT_UNROLL_CAP {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "a whole-array continuous assignment of {n} elements (v1 cap {ARRAY_CONT_UNROLL_CAP})"
                ),
            );
            return true;
        }
        let t_desc: Vec<bool> = self.array_dim_desc.get(&t_net).cloned().unwrap_or_default();
        let t_offs = Self::residual_word_offsets(&t_dims, &t_desc);
        let elem_width = self.nets[t_net as usize].width;
        let expanded: Vec<ast::Expr>;
        let source = match (&rhs.kind, s_array) {
            (ast::ExprKind::AssignPattern(elems), _) => {
                let Some(leaves) = self.flatten_assign_pattern(elems, &t_dims) else {
                    return true; // count mismatch / missing nested pattern — error emitted
                };
                ElemSource::Leaves(leaves)
            }
            (ast::ExprKind::AssignPatternKeyed(keyed), _) => {
                let Some(e) = self.expand_array_default_pattern(keyed, &t_dims) else {
                    return true; // error already emitted
                };
                expanded = e;
                let Some(leaves) = self.flatten_assign_pattern(&expanded, &t_dims) else {
                    return true;
                };
                ElemSource::Leaves(leaves)
            }
            (_, Some(s_net)) => {
                let s_dims = self.net_dim_extents(s_net);
                if let Some(msg) = self.array_copy_mismatch(t_net, &t_dims, s_net, &s_dims) {
                    self.error(MsgCode::ElabUnsupported, msg);
                    return true;
                }
                let s_desc: Vec<bool> =
                    self.array_dim_desc.get(&s_net).cloned().unwrap_or_default();
                ElemSource::Array(s_net, Self::residual_word_offsets(&s_dims, &s_desc))
            }
            (_, None) => return false,
        };
        let lo = self.cont_assigns.len();
        for (k, &t_off) in t_offs.iter().enumerate() {
            let t_word = self.const_u32_expr(t_off, 32);
            let lhs = ir::Lvalue {
                chunks: vec![ir::LvalChunk {
                    net: t_net,
                    word: Some(t_word),
                    offset: None,
                    width: None,
                    kind: ir::SelKind::Bit, // neutral tag; offset/width None ⇒ the whole element
                }],
            };
            if k == 0 {
                self.check_lvalue_kind(&lhs, false); // E3018 once (same net throughout)
            }
            // Each item is evaluated as an assignment to its element (§10.9.1), sized as
            // the procedural pattern path sizes it: a fill grows to the element, and a
            // streaming item is not left-justified the way a stream that is the whole
            // right-hand side of a scalar `assign` is (verilator: `'{{<<{n}}}` into 8-bit
            // elements reads `08`, `assign s = {<<{n}}` reads `80`).
            let rhs_id = match &source {
                ElemSource::Leaves(leaves) => self.lower_ctx_or_plain(leaves[k], elem_width),
                ElemSource::Array(s_net, s_offs) => {
                    let s_word = self.const_u32_expr(s_offs[k], 32);
                    self.push_expr(ir::Expr::Signal {
                        net: *s_net,
                        word: Some(s_word),
                    })
                }
            };
            // R14: every element row carries the statement's span — the source holds one
            // `assign`, and the split into rows is vita's (see `wire_array_port`).
            self.push_cont_assign(
                ir::ContAssign {
                    lhs,
                    rhs: rhs_id,
                    delay: None,
                },
                "assign",
                Some(span),
            );
        }
        self.whole_array_cas.push(WholeArrayCa {
            net: t_net,
            lo,
            hi: self.cont_assigns.len(),
            span,
            prefix: self.cur_prefix.clone(),
        });
        true
    }

    /// Refuse every whole-array `assign` whose array has another writer. Runs after the
    /// last lowering and the deferred hierarchical writes, so every writer exists. The
    /// census is every channel that stores to a net: the continuous assigns (another
    /// `assign`, an output port, a declaration initializer), the statements (procedural
    /// writes, `force`/`release`, task and function bodies, hierarchical writes), a system
    /// task or function that may write an argument, a frame call's copy-out, a clocking
    /// block's output commit, and an `inout` port connection, whose drive back from the
    /// child no IR carries. A system call is classified by an explicit READ-ONLY list, so
    /// an unlisted one counts as a writer; `$sformat` / `$swrite*` write only `args[0]`.
    /// A written chunk naming no net of this design (a hierarchical placeholder no pass
    /// resolved) is a writer of every array.
    pub(crate) fn check_whole_array_sole_writer(&mut self) {
        if self.whole_array_cas.is_empty() {
            return;
        }
        let targets: BTreeSet<u32> = self.whole_array_cas.iter().map(|w| w.net).collect();
        // net → the continuous assigns of each whole-array group that owns it.
        let mut own: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
        for w in &self.whole_array_cas {
            own.entry(w.net).or_default().push((w.lo, w.hi));
        }
        let mut written: BTreeSet<u32> = BTreeSet::new();
        let mut unknown = false;
        // A group sees every other group on the same array as another writer.
        for (&net, groups) in &own {
            if groups.len() > 1 {
                written.insert(net);
            }
        }
        for (i, ca) in self.cont_assigns.iter().enumerate() {
            for c in &ca.lhs.chunks {
                if targets.contains(&c.net)
                    && !own[&c.net].iter().any(|&(lo, hi)| (lo..hi).contains(&i))
                {
                    written.insert(c.net);
                }
                unknown |= c.net as usize >= self.nets.len();
            }
        }
        for st in &self.stmts {
            match st {
                ir::Stmt::BlockingAssign { lhs, .. }
                | ir::Stmt::NonblockingAssign { lhs, .. }
                | ir::Stmt::Force { lhs, .. }
                | ir::Stmt::Release { lhs } => {
                    self.lvalue_writes(lhs, &targets, &mut written, &mut unknown);
                }
                ir::Stmt::SysTask { which, args, .. } => {
                    let dest = match which {
                        ir::SysTaskId::Sformat => &args[..args.len().min(1)],
                        w if systask_is_read_only(*w) => &[][..],
                        _ => &args[..],
                    };
                    for &a in dest {
                        self.ir_expr_nets_in(a, &targets, &mut written);
                    }
                }
                ir::Stmt::Disable { .. } => {}
            }
        }
        for e in &self.exprs {
            if let ir::Expr::SysFunc { which, args } = e {
                if !sysfunc_is_read_only(*which) {
                    for &a in args {
                        self.ir_expr_nets_in(a, &targets, &mut written);
                    }
                }
            }
        }
        let out_binds = self
            .task_calls_proc
            .values()
            .chain(self.task_calls_func.values())
            .chain(self.pending_task_calls.iter().map(|(_, info)| info))
            .flat_map(|info| info.out_binds.iter());
        for (_, lv) in out_binds {
            self.lvalue_writes(lv, &targets, &mut written, &mut unknown);
        }
        for pairs in self.clocking_outputs.values() {
            written.extend(
                pairs
                    .iter()
                    .map(|&(src, _)| src)
                    .filter(|n| targets.contains(n)),
            );
        }
        for &eid in &self.inout_actual_exprs {
            self.ir_expr_nets_in(eid, &targets, &mut written);
        }
        if unknown {
            written = targets;
        }
        if written.is_empty() {
            return;
        }
        let refused: Vec<(ast::Span, String)> = self
            .whole_array_cas
            .iter()
            .filter(|w| written.contains(&w.net))
            .map(|w| (w.span, w.prefix.clone()))
            .collect();
        let saved = std::mem::take(&mut self.cur_prefix);
        for (span, prefix) in refused {
            self.cur_prefix = prefix;
            self.error_at(
                MsgCode::ElabUnsupported,
                span,
                "a whole unpacked array driven by a continuous `assign` has another writer \
                 (another `assign`, a port, a procedural or clocking-block write, or a system \
                 task); v1 lowers a whole-array `assign` only when it is the array's only writer",
            );
        }
        self.cur_prefix = saved;
    }

    /// Record in `written` each chunk of `lv` on a net of `targets`; set `unknown` for a
    /// chunk naming no net of this design.
    fn lvalue_writes(
        &self,
        lv: &ir::Lvalue,
        targets: &BTreeSet<u32>,
        written: &mut BTreeSet<u32>,
        unknown: &mut bool,
    ) {
        for c in &lv.chunks {
            if targets.contains(&c.net) {
                written.insert(c.net);
            }
            *unknown |= c.net as usize >= self.nets.len();
        }
    }

    /// Is `net` declared with a 2-state type (`bit`, `byte`, `int`, …)?
    fn net_is_two_state(&self, net: u32) -> bool {
        self.intro_kind
            .get(&net)
            .is_some_and(|k| net_kind_is_two_state(*k))
    }

    /// Add to `out` every net of `nets` that expression `eid` reads anywhere in its tree.
    fn ir_expr_nets_in(&self, eid: u32, nets: &BTreeSet<u32>, out: &mut BTreeSet<u32>) {
        let mut stack = vec![eid];
        while let Some(id) = stack.pop() {
            let Some(e) = self.exprs.get(id as usize) else {
                continue;
            };
            match e {
                ir::Expr::Const { .. } | ir::Expr::ArrayItem { .. } => {}
                ir::Expr::Signal { net, word } => {
                    if nets.contains(net) {
                        out.insert(*net);
                    }
                    stack.extend(word.iter().copied());
                }
                ir::Expr::Select {
                    base,
                    offset,
                    width,
                    ..
                } => stack.extend([*base, *offset, *width]),
                ir::Expr::Concat { parts } => stack.extend(parts.iter().copied()),
                ir::Expr::Replicate { count, value } => stack.extend([*count, *value]),
                ir::Expr::Unary { operand, .. } => stack.push(*operand),
                ir::Expr::Binary { lhs, rhs, .. } => stack.extend([*lhs, *rhs]),
                ir::Expr::Ternary {
                    cond,
                    then_e,
                    else_e,
                } => stack.extend([*cond, *then_e, *else_e]),
                ir::Expr::SysFunc { args, .. } | ir::Expr::Call { args, .. } => {
                    stack.extend(args.iter().copied())
                }
            }
        }
    }
}

/// Is an element type written in its declaration itself — `logic [7:0] a [2]`, not a
/// typedef, an enum, a struct or a type parameter? Every packed dimension must lie in the
/// declaration's own text between its start `lo` and its first name `first`. A typedef's
/// dimensions are the typedef's text (the parser clones them; a struct's is synthesized
/// with an empty span), and a typedef cannot reach another compilation unit's parse, so a
/// span in that window is the declaration's own. The element type then says what it is:
/// an enum declared `enum bit [7:0]` is recorded as a 4-state `logic` vector, which no
/// check here could tell from `logic [7:0]`.
pub(crate) fn inline_elem_type(
    lo: u32,
    first: u32,
    range: Option<&ast::Range>,
    packed: &[ast::Range],
    shape_param: bool,
) -> bool {
    let within = |r: &ast::Range| lo <= r.span.lo && r.span.hi <= first;
    !shape_param && range.is_some_and(within) && packed.iter().all(within)
}

/// A system task that reads its arguments and writes no net. Anything not listed may
/// write one (`$readmem*`, `$cast`, `$sformat`, the in-place array and container methods).
fn systask_is_read_only(which: ir::SysTaskId) -> bool {
    use ir::SysTaskId as T;
    matches!(
        which,
        T::Display
            | T::Write
            | T::Monitor
            | T::Strobe
            | T::Finish
            | T::Stop
            | T::DumpFile
            | T::DumpVars
            | T::DumpOn
            | T::DumpOff
            | T::DumpAll
            | T::DumpFlush
            | T::DumpLimit
            | T::Fclose
            | T::Fdisplay
            | T::Fwrite
            | T::WritememB
            | T::WritememH
            | T::MonitorOn
            | T::MonitorOff
    )
}

/// A system function that writes no argument. Anything not listed may write one (the
/// scanf / `$fgets` / `$fread` / `$value$plusargs` / `$cast` destinations, the seed of
/// `$random` and `$dist_*`, the index of an associative-array traversal, a queue pop).
fn sysfunc_is_read_only(which: ir::SysFuncId) -> bool {
    use ir::SysFuncId as F;
    matches!(
        which,
        F::Time
            | F::Realtime
            | F::Signed
            | F::Unsigned
            | F::Clog2
            | F::Rtoi
            | F::Itor
            | F::RealToBits
            | F::BitsToReal
            | F::DynSize
            | F::AssocExists
            | F::AssocNum
            | F::Urandom
            | F::UrandomRange
            | F::CountOnes
            | F::OneHot
            | F::OneHot0
            | F::IsUnknown
            | F::Stime
            | F::Fopen
            | F::Sformatf
            | F::TestPlusargs
            | F::StrLen
            | F::StrGetC
            | F::StrSubstr
            | F::StrToUpper
            | F::StrToLower
            | F::StrCmp
            | F::Feof
            | F::Fgetc
            | F::ArrSum
            | F::ArrProduct
            | F::ArrAnd
            | F::ArrOr
            | F::ArrXor
            | F::StrAtoi
            | F::StrAtohex
            | F::StrAtooct
            | F::StrAtobin
            | F::StrAtoreal
            | F::Ln
            | F::Log10
            | F::Exp
            | F::Sqrt
            | F::Pow
            | F::Floor
            | F::Ceil
            | F::Sin
            | F::Cos
            | F::Tan
            | F::Asin
            | F::Acos
            | F::Atan
            | F::Atan2
            | F::Hypot
            | F::Sinh
            | F::Cosh
            | F::Tanh
            | F::Asinh
            | F::Acosh
            | F::Atanh
            | F::RealToInt
            | F::TwoState
    )
}
