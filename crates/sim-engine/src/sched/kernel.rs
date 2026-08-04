//! split part of `sched` (mechanical move).

use super::*;

/// [P7b] `Scheduler` is the interpreter's implementation of the body↔kernel ABI:
/// each method forwards to the inherent method of the same purpose (the `k_*` prefix
/// keeps the trait surface distinct from the inherent one, so there is no shadowing).
/// The statement executor (`exec::compute_effect`/`apply_effect`) drives the
/// interpreter through exactly this surface, so the existing suite already exercises
/// the seam byte-identically — and a Stage-C compiled body will call the same methods.
impl Kernel for Scheduler<'_, '_> {
    #[cfg(feature = "jit")]
    fn k_nets(&self) -> &dyn crate::eval::NetReader {
        self.st
    }
    fn k_eval_for_lvalue(&self, lhs: &Lvalue, rhs: u32) -> Value {
        self.eval_for_lvalue(lhs, rhs)
    }
    fn k_eval_native(&self, prog: &crate::native_eval::NativeProg) -> Value {
        self.eval_native(prog)
    }
    fn k_resolve_lvalue_offsets(&self, lhs: &Lvalue) -> Offsets {
        self.resolve_lvalue_offsets(lhs)
    }
    fn k_force(&mut self, lhs: &Lvalue, value: Value, rhs: u32, sid: u32) {
        // The multiplier is snapshot at registration so a `$time` in the RHS
        // keeps rendering with the right scale on later re-evals (C7 lesson).
        let net = lhs.chunks[0].net;
        let mult = self.st.cur_time_mult;
        let weak = self.st.assign_ranks.contains(&sid);
        if weak {
            // §9.3.1 proc-assign: an active FORCE keeps priority — park the
            // assign as latent (it takes control at release). Otherwise (re)pin
            // at assign rank (a second assign overrides the first).
            if matches!(self.st.active_forces.get(&net), Some((.., false))) {
                self.st.latent_assigns.insert(net, (lhs.clone(), rhs, mult));
                return;
            }
            self.st.latent_assigns.remove(&net);
        } else if let Some((plv, prhs, pmult, true)) = self.st.active_forces.get(&net).cloned() {
            // real force displacing an active assign: park it for release.
            self.st.latent_assigns.insert(net, (plv, prhs, pmult));
        }
        self.st.force_write(lhs, value);
        // Register for continuous re-evaluation (IEEE §9.3.2 / §9.3.1).
        self.st
            .active_forces
            .insert(net, (lhs.clone(), rhs, mult, weak));
        // C-FORCE-REEVAL-p2: refresh this force's net→forces sensitivity (or
        // mark it always-reeval if volatile / zero-net) so the per-delta reeval
        // can skip forces whose inputs are unchanged.
        self.st.register_force_sensitivity(net, rhs);
    }
    fn k_release(&mut self, lhs: &Lvalue, sid: u32) {
        let net = lhs.chunks[0].net;
        if self.st.assign_ranks.contains(&sid) {
            // `deassign`: drop the assign wherever it lives. An active STRONG
            // force is untouched; an active assign unpins (the variable HOLDS
            // its value, §9.3.1); a latent assign is just forgotten.
            self.st.latent_assigns.remove(&net);
            if matches!(self.st.active_forces.get(&net), Some((.., true))) {
                self.st.active_forces.remove(&net);
                self.st.unregister_force_sensitivity(net);
                self.st.release(lhs);
            }
            return;
        }
        // `release`: removes the FORCE. A parked proc-assign resumes control
        // (re-pin + re-evaluate NOW, §9.3.1); an active assign is NOT a force
        // and keeps control; otherwise plain unpin.
        match self.st.active_forces.get(&net) {
            Some((.., true)) => {} // assign active, no force: release is a no-op
            _ => {
                self.st.active_forces.remove(&net);
                self.st.unregister_force_sensitivity(net);
                self.st.release(lhs);
                if let Some((alv, arhs, amult)) = self.st.latent_assigns.remove(&net) {
                    let saved = self.st.cur_time_mult;
                    self.st.cur_time_mult = amult;
                    let v = self.eval_for_lvalue(&alv, arhs);
                    self.st.force_write(&alv, v);
                    self.st.cur_time_mult = saved;
                    self.st.active_forces.insert(net, (alv, arhs, amult, true));
                    // Re-register the resumed latent assign's sensitivity.
                    self.st.register_force_sensitivity(net, arhs);
                }
            }
        }
    }
    fn k_write_lvalue(&mut self, lhs: &Lvalue, value: Value, offsets: &Offsets) {
        self.st.write_lvalue(lhs, value, offsets);
    }
    fn k_schedule_nba(&mut self, lhs: &Lvalue, value: Value) {
        self.schedule_nba(lhs, value);
    }
    fn k_schedule_nba_scalar(&mut self, lhs: &Lvalue, value: Value) {
        self.schedule_nba_scalar(lhs, value);
    }
    fn k_write_scalar(&mut self, lhs: &Lvalue, net: u32, value: Value) {
        self.st.write_scalar(lhs, net, value);
    }
    fn k_delay_ticks(&self, eid: u32) -> u64 {
        self.delay_ticks(eid)
    }
    fn k_schedule_nba_at(&mut self, lhs: &Lvalue, value: Value, ticks: u64) {
        self.schedule_nba_at(lhs, value, ticks);
    }
    fn k_dispatch_systask(
        &mut self,
        which: sim_ir::SysTaskId,
        fmt: Option<u32>,
        args: &[u32],
        sid: u32,
    ) -> crate::builtins::Ctl {
        crate::builtins::dispatch(self, which, fmt, args, sid)
    }
    fn k_queue_pop_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::queue_pop_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_random_seeded_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::random_seeded_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_random_seeded(&mut self, rhs: u32) -> Value {
        // shape guaranteed by `k_random_seeded_rhs` + elaborate's whole-net
        // seed contract; everything below defends a hand-built IR.
        let seed_net = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) => {
                args.first()
                    .and_then(|&a| match self.st.ir.exprs.get(a as usize) {
                        Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    })
            }
            _ => None,
        };
        let Some(net) = seed_net else {
            return Value::xs(32, true);
        };
        // seed in: low 32 bits of the variable; X/Z reads as 0 (then the
        // Annex zero-substitution applies, like an uninitialized iverilog reg).
        let cur = self.st.read_net(net, None);
        let mut s = if cur.has_xz() {
            0
        } else {
            (cur.to_u64().unwrap_or(0) & 0xffff_ffff) as u32
        };
        let r = crate::rng::annex_n_random(&mut s);
        // write the updated seed back through the normal lvalue funnel
        // (resizes to the variable's width like any blocking assign).
        let lv = Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        let sv = Value::from_i128(s as i32 as i128, 32, true);
        let off = self.resolve_lvalue_offsets(&lv);
        self.k_write_lvalue(&lv, sv, &off);
        Value::from_i128(r as i128, 32, true)
    }
    fn k_dist_seeded_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::dist_seeded_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_dist_seeded(&mut self, rhs: u32) -> Value {
        // shape guaranteed by `k_dist_seeded_rhs` + elaborate's whole-net seed
        // contract; everything below defends a hand-built IR.
        let (which, seed_net, params) = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { which, args }) if !args.is_empty() => {
                let net = match self.st.ir.exprs.get(args[0] as usize) {
                    Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                    _ => None,
                };
                (*which, net, args[1..].to_vec())
            }
            _ => return Value::xs(32, true),
        };
        let Some(net) = seed_net else {
            return Value::xs(32, true);
        };
        // dist params are `integer` (signed 32-bit); X/Z reads as 0.
        let p: Vec<i32> = params
            .iter()
            .map(|&a| self.eval(a).to_u64().unwrap_or(0) as u32 as i32)
            .collect();
        // seed in: low 32 bits; X/Z → 0 (uninitialized-reg parity). The dist
        // kernels advance it via the Annex `69069*s+1` integer LCG.
        let cur = self.st.read_net(net, None);
        let mut s = if cur.has_xz() {
            0
        } else {
            (cur.to_u64().unwrap_or(0) & 0xffff_ffff) as u32
        };
        let r = match which {
            sim_ir::SysFuncId::DistUniform => {
                crate::rng::dist_uniform(&mut s, *p.first().unwrap_or(&0), *p.get(1).unwrap_or(&0))
            }
            sim_ir::SysFuncId::DistNormal => {
                crate::rng::dist_normal(&mut s, *p.first().unwrap_or(&0), *p.get(1).unwrap_or(&0))
            }
            sim_ir::SysFuncId::DistExponential => {
                crate::rng::dist_exponential(&mut s, *p.first().unwrap_or(&0))
            }
            sim_ir::SysFuncId::DistPoisson => {
                crate::rng::dist_poisson(&mut s, *p.first().unwrap_or(&0))
            }
            sim_ir::SysFuncId::DistChiSquare => {
                crate::rng::dist_chi_square(&mut s, *p.first().unwrap_or(&0))
            }
            sim_ir::SysFuncId::DistT => crate::rng::dist_t(&mut s, *p.first().unwrap_or(&0)),
            sim_ir::SysFuncId::DistErlang => {
                crate::rng::dist_erlang(&mut s, *p.first().unwrap_or(&0), *p.get(1).unwrap_or(&0))
            }
            _ => 0,
        };
        let lv = Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        let sv = Value::from_i128(s as i32 as i128, 32, true);
        let off = self.resolve_lvalue_offsets(&lv);
        self.k_write_lvalue(&lv, sv, &off);
        Value::from_i128(r as i128, 32, true)
    }
    fn k_cast_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::cast_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_cast(&mut self, rhs: u32) -> Value {
        // func-form `ok = $cast(dst, src)`: write the resized `src` into the `dst`
        // ref arg and return 1. iverilog 13.0 does NOT support $cast (no oracle):
        // hand-IEEE §6.24.2 — an integral assignment always succeeds in this
        // class-free subset, so the status is always 1 (failure=0 needs class /
        // strict-enum range checks vita does not model).
        let (dst_net, src_arg) = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() == 2 => {
                let net = match self.st.ir.exprs.get(args[0] as usize) {
                    Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                    _ => None,
                };
                (net, args[1])
            }
            _ => return Value::from_i128(0, 32, true),
        };
        let Some(net) = dst_net else {
            return Value::from_i128(0, 32, true);
        };
        let lv = Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        // context-size `src` to the dst width, then write through the funnel.
        let v = self.eval_for_lvalue(&lv, src_arg);
        let off = self.resolve_lvalue_offsets(&lv);
        self.k_write_lvalue(&lv, v, &off);
        Value::from_i128(1, 32, true)
    }
    fn k_value_plusargs_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::value_plusargs_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_value_plusargs(&mut self, rhs: u32) -> Value {
        // args = [fmt string-literal Const, ref-var whole-net Signal] —
        // elaborate's contract; defend a hand-built IR by returning 0.
        let (fmt_eid, var_net) = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() == 2 => {
                let var = match self.st.ir.exprs.get(args[1] as usize) {
                    Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                    _ => None,
                };
                (args[0], var)
            }
            _ => (u32::MAX, None),
        };
        let fmt = match self.st.ir.exprs.get(fmt_eid as usize) {
            Some(sim_ir::Expr::Const { val }) => crate::builtins::const_string(self.st.ir, *val),
            _ => return Value::from_i128(0, 32, true),
        };
        let Some(net) = var_net else {
            return Value::from_i128(0, 32, true);
        };
        // split "prefix%C" — elaborate validated exactly one supported spec.
        let Some(pct) = fmt.find('%') else {
            // degenerate no-spec format: a pure test probe, nothing written.
            let hit = self.st.plusargs.iter().any(|p| p.starts_with(&fmt));
            return Value::from_i128(hit as i128, 32, true);
        };
        let prefix = &fmt[..pct];
        let conv = fmt[pct + 1..].chars().next().unwrap_or('d');
        let Some(rest) = self
            .st
            .plusargs
            .iter()
            .find_map(|p| p.strip_prefix(prefix).map(|r| r.to_string()))
        else {
            return Value::from_i128(0, 32, true); // MISS: var untouched
        };
        let radix = match conv {
            'd' | 'D' => 10,
            'h' | 'H' | 'x' | 'X' => 16,
            'o' | 'O' => 8,
            'b' | 'B' => 2,
            _ => 0, // %s
        };
        let value = if radix == 0 {
            // %s: pack the raw bytes MSB-first (IEEE §5.9 string packing).
            let bytes = rest.as_bytes();
            let w = (bytes.len() as u32 * 8).max(8);
            let mut v = Value::zeros(w, false);
            for (i, &by) in bytes.iter().rev().enumerate() {
                let bit = i * 8;
                v.val[bit / 64] |= (by as u64) << (bit % 64);
            }
            v
        } else {
            // scanf-style: optional sign, then leading digits of the radix.
            let (neg, digits) = match rest.strip_prefix('-') {
                Some(d) => (true, d),
                None => (false, rest.as_str()),
            };
            let lead: String = digits.chars().take_while(|c| c.is_digit(radix)).collect();
            let mag = u64::from_str_radix(&lead, radix).unwrap_or(0);
            let raw = if neg {
                (mag as i64).wrapping_neg() as u64
            } else {
                mag
            };
            let mut v = Value::zeros(64, false);
            v.val[0] = raw;
            v
        };
        let lv = Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        let off = self.resolve_lvalue_offsets(&lv);
        self.k_write_lvalue(&lv, value, &off);
        Value::from_i128(1, 32, true)
    }
    fn k_fopen_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fopen_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_fopen(&mut self, rhs: u32) -> Value {
        // args = [name (, mode)] — each is a string LITERAL, a runtime `string`
        // value, or a packed reg holding ASCII (elaborate's relaxed contract).
        let args = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) => args.clone(),
            _ => return Value::from_i128(0, 32, true),
        };
        // Resolve a name/mode arg to text: a Const{StrUtf8} literal decodes
        // directly; a runtime STRING value (`is_str`) renders its exact bytes;
        // any other packed value is treated as ASCII in a reg (NUL-stripped) —
        // all three are valid $fopen argument forms (§21.3, iverilog parity).
        let resolve = |st: &SimState<'_>, a: u32| -> String {
            if let Some(sim_ir::Expr::Const { val }) = st.ir.exprs.get(a as usize) {
                crate::builtins::const_string(st.ir, *val)
            } else {
                let v = st.eval_expr(a);
                if v.is_str {
                    String::from_utf8_lossy(&v.to_str_bytes()).into_owned()
                } else {
                    crate::builtins::fmt_packed_chars_min(&v)
                }
            }
        };
        let name = match args.first() {
            Some(&a) => resolve(self.st, a),
            None => return Value::from_i128(0, 32, true),
        };
        let mode = args.get(1).map(|&a| resolve(self.st, a));
        let open = |mode: &str| -> std::io::Result<std::fs::File> {
            let mut o = std::fs::OpenOptions::new();
            // a '+' mode (r+/w+/a+) is read-AND-write; plain w/a are write-only.
            let plus = mode.contains('+');
            match mode.trim_end_matches('b') {
                "r" | "r+" => o.read(true).write(plus),
                "a" | "a+" => o.create(true).append(true).read(plus),
                // "w"/"w+" and anything unrecognized: truncate-write (the
                // overwhelmingly common TB mode; unknown modes behave as "w").
                _ => o.create(true).write(true).truncate(true).read(plus),
            };
            o.open(&name)
        };
        let fd = match mode {
            Some(m) => match open(&m) {
                Ok(f) => {
                    let n = self.st.next_fd;
                    self.st.next_fd = self.st.next_fd.saturating_add(1); // FD-RECLAIM: no wrap
                    let fd = 0x8000_0000 | n;
                    self.st.files.insert(fd, f);
                    // v9 SYS-READ: a mode with 'r' or '+' is read-capable
                    // (r/r+/w+/a+); plain "w"/"a" stays write-only and absent.
                    if m.contains('r') || m.contains('+') {
                        self.st.readable_fds.insert(fd);
                    }
                    fd
                }
                Err(_) => 0, // IEEE: $fopen failure returns 0
            },
            None => match open("w") {
                Ok(f) => {
                    // MCD-RECLAIM: hand out the LOWEST channel bit not currently
                    // open (bit 0 = stdout, reserved), so a $fclose'd bit is
                    // reused (iverilog reclaims). Byte-identical to the old
                    // monotonic counter when nothing has been freed.
                    match (1..31u32).find(|b| !self.st.mcd_files.contains_key(b)) {
                        Some(bit) => {
                            self.st.mcd_files.insert(bit, f);
                            1u32 << bit
                        }
                        None => return Value::from_i128(0, 32, true), // space full
                    }
                }
                Err(_) => 0,
            },
        };
        let mut v = Value::zeros(32, true);
        v.val[0] = fd as u64;
        v
    }
    // ── v9 SYS-READ: file-read int functions ($fgetc/$feof/$ungetc) ──
    fn k_fgetc_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fgetc_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_fgetc(&mut self, rhs: u32) -> Value {
        let fd_arg = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if !args.is_empty() => args[0],
            _ => return Value::from_i128(-1, 32, true),
        };
        let fdv = self.eval(fd_arg);
        if fdv.has_xz() {
            return Value::from_i128(-1, 32, true);
        }
        let fd = fdv.to_u64().unwrap_or(0) as u32;
        match crate::builtins::file_read_byte(self, fd) {
            Some(b) => Value::from_i128(b as i128, 32, true),
            None => Value::from_i128(-1, 32, true),
        }
    }
    fn k_feof_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::feof_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_feof(&mut self, rhs: u32) -> Value {
        let fd_arg = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if !args.is_empty() => args[0],
            _ => return Value::from_i128(-1, 32, true),
        };
        let fdv = self.eval(fd_arg);
        if fdv.has_xz() {
            return Value::from_i128(-1, 32, true);
        }
        let fd = fdv.to_u64().unwrap_or(0) as u32;
        // §21.3.4: the pre-opened descriptors are always-valid, never-EOF fds
        // (iverilog-pinned: `$feof(STDOUT)` = 0, no warning — mirroring the
        // write-only-fd rule, whose failed `$fgetc` never latches EOF).
        if (0x8000_0000..=0x8000_0002).contains(&fd) {
            return Value::from_i128(0, 32, true);
        }
        // a bad/closed fd → −1 (iverilog parity, NOT 0); an open fd that has
        // not yet hit EOF → 0.
        if fd & 0x8000_0000 == 0 || !self.st.files.contains_key(&fd) {
            crate::builtins::bad_fd_warn(self, fd);
            return Value::from_i128(-1, 32, true);
        }
        let eof = self.st.read_state.get(&fd).map(|s| s.eof).unwrap_or(false);
        Value::from_i128(if eof { 1 } else { 0 }, 32, true)
    }
    fn k_ungetc_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::ungetc_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_ungetc(&mut self, rhs: u32) -> Value {
        let (c_arg, fd_arg) = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => (args[0], args[1]),
            _ => return Value::from_i128(-1, 32, true),
        };
        let cv = self.eval(c_arg);
        let fdv = self.eval(fd_arg);
        if fdv.has_xz() {
            return Value::from_i128(-1, 32, true);
        }
        let fd = fdv.to_u64().unwrap_or(0) as u32;
        // The EOF sentinel is ONLY the exact int −1 (0xffff_ffff, fully known).
        // iverilog treats every other c — INCLUDING a value with x/z bits — as
        // a normal char and pushes its low byte (x/z bits coerced to 0).
        if !cv.has_xz() && (cv.to_u64().unwrap_or(0) as u32) == 0xffff_ffff {
            return Value::from_i128(-1, 32, true);
        }
        // §21.3.4: the pre-opened STDOUT/STDERR follow the write-only rule —
        // −1, no warning. STDIN pushback is part of the deferred stdin-read
        // feature → −1 quietly too (nothing to push back into).
        if (0x8000_0000..=0x8000_0002).contains(&fd) {
            return Value::from_i128(-1, 32, true);
        }
        // a bad/closed fd warns + returns −1; a valid but write-only ("w"/"a")
        // fd returns −1 WITHOUT a warning (iverilog: a write stream is not
        // pushable and never becomes readable). Only a read-capable fd accepts
        // a pushback.
        if fd & 0x8000_0000 == 0 || !self.st.files.contains_key(&fd) {
            crate::builtins::bad_fd_warn(self, fd);
            return Value::from_i128(-1, 32, true);
        }
        if !self.st.readable_fds.contains(&fd) {
            return Value::from_i128(-1, 32, true);
        }
        // the pushed byte = the low 8 bits with x/z bits coerced to 0.
        let mut byte = 0u8;
        for i in 0..8 {
            let (v, u) = cv.get_vu(i);
            if u == 0 && v != 0 {
                byte |= 1 << i;
            }
        }
        // LIFO push (iverilog retains every pushed byte); pushing clears EOF
        // (there is data to read again).
        let st = self.st.read_state.entry(fd).or_default();
        st.pushback.push(byte);
        st.eof = false;
        Value::from_i128(0, 32, true)
    }
    fn k_fgets_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fgets_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_fgets(&mut self, rhs: u32) -> Value {
        // args = [str-dest whole-net Signal, fd] — elaborate's contract.
        let (dest_net, fd_arg) = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => {
                let net = match self.st.ir.exprs.get(args[0] as usize) {
                    Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                    _ => None,
                };
                (net, args[1])
            }
            _ => (None, u32::MAX),
        };
        let Some(net) = dest_net else {
            return Value::from_i128(0, 32, true);
        };
        let fdv = self.eval(fd_arg);
        if fdv.has_xz() {
            return Value::from_i128(0, 32, true);
        }
        let fd = fdv.to_u64().unwrap_or(0) as u32;
        let whole_net = |net: u32| Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        // v7: a SystemVerilog `string` dest is a dynamic HANDLE (NetKind::String,
        // net width 0). It has NO byte capacity, so it must NOT fall into the
        // sub-byte (width < 8) reg branch below, which would clear the dest and
        // return 0 (the silent-wrong this fixes). Read the WHOLE line uncapped
        // (through a retained newline, else to EOF), pack it MSB-first, and write
        // it via the same string lvalue path as `s = "..."` (§6.16 byte-strip).
        if self.st.ir.nets[net as usize].kind == sim_ir::NetKind::String {
            let mut raw: Vec<u8> = Vec::new();
            while let Some(b) = crate::builtins::file_read_byte(self, fd) {
                raw.push(b);
                if b == b'\n' {
                    break;
                }
            }
            if raw.is_empty() {
                // genuine EOF / bad-fd / write-only: dest UNCHANGED, count 0.
                return Value::from_i128(0, 32, true);
            }
            // C-string semantics (iverilog parity, same as the reg path below):
            // the STORED string and the returned count stop at the first NUL,
            // even though the whole line was already consumed from the stream.
            // (n == raw.len() when there is no embedded NUL.) A leading NUL gives
            // n = 0 → the dest is set to the empty string (distinct from the EOF
            // arm above, which leaves it UNCHANGED).
            let n = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
            let lv = whole_net(net);
            let off = self.resolve_lvalue_offsets(&lv);
            self.k_write_lvalue(&lv, Value::from_str_bytes(&raw[..n]), &off);
            return Value::from_i128(n as i128, 32, true);
        }
        // capacity = the dest in whole bytes (iverilog reads the FULL width N,
        // not C's N-1 — no NUL is reserved).
        let width = self.st.ir.nets[net as usize].width.max(1);
        let max_bytes = (width / 8) as usize;
        if max_bytes == 0 {
            // sub-byte dest (width < 8): iverilog reads NO stream byte but
            // CLEARS the dest to 0 (C fgets into a too-small buffer => empty
            // string written), returning 0.
            let lv = whole_net(net);
            let off = self.resolve_lvalue_offsets(&lv);
            self.k_write_lvalue(&lv, Value::zeros(width, false), &off);
            return Value::from_i128(0, 32, true);
        }
        // read the line: up to max_bytes OR through a newline (retained).
        let mut raw: Vec<u8> = Vec::new();
        let mut any_read = false;
        while raw.len() < max_bytes {
            match crate::builtins::file_read_byte(self, fd) {
                Some(b) => {
                    any_read = true;
                    raw.push(b);
                    if b == b'\n' {
                        break;
                    }
                }
                None => break,
            }
        }
        if !any_read {
            // genuine EOF / bad-fd / write-only: dest UNCHANGED, count 0.
            return Value::from_i128(0, 32, true);
        }
        // the RETURNED string stops at the first NUL (C string semantics); the
        // bytes after it were still consumed from the stream above, so the file
        // position matches iverilog.
        let n = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        // pack the first n bytes right-justified MSB-first (first byte = most
        // significant) into a width-wide value; n == 0 (leading NUL) leaves it
        // all-zero, which CLEARS the dest — iverilog writes 0, not the prior
        // value. n*8 <= width because n <= max_bytes = width / 8.
        let mut v = Value::zeros(width, false);
        for (i, &by) in raw[..n].iter().rev().enumerate() {
            let bit = i * 8;
            v.val[bit / 64] |= (by as u64) << (bit % 64);
        }
        let lv = whole_net(net);
        let off = self.resolve_lvalue_offsets(&lv);
        self.k_write_lvalue(&lv, v, &off);
        Value::from_i128(n as i128, 32, true)
    }
    fn k_fread_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fread_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_fread(&mut self, rhs: u32) -> Value {
        // args = [target whole-net Signal, fd, start?, count?] — elaborate's
        // contract (a single reg OR a whole memory; element-select is loud).
        let (net, fd_arg, start_arg, count_arg) = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => {
                let net = match self.st.ir.exprs.get(args[0] as usize) {
                    Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                    _ => None,
                };
                (net, args[1], args.get(2).copied(), args.get(3).copied())
            }
            _ => (None, u32::MAX, None, None),
        };
        let Some(net) = net else {
            return Value::from_i128(0, 32, true);
        };
        let fdv = self.eval(fd_arg);
        if fdv.has_xz() {
            return Value::from_i128(0, 32, true);
        }
        let fd = fdv.to_u64().unwrap_or(0) as u32;
        let (w, alen) = {
            let nv = &self.st.ir.nets[net as usize];
            (nv.width.max(1), nv.array_len.max(1) as u64)
        };
        let cap_per = ((w + 7) / 8) as usize; // bytes per element/reg
        if alen <= 1 {
            // ── single reg/vector: read ceil(w/8) bytes, MSB-slot fill ──
            let mut got: Vec<u8> = Vec::new();
            for _ in 0..cap_per {
                match crate::builtins::file_read_byte(self, fd) {
                    Some(b) => got.push(b),
                    None => break,
                }
            }
            if got.is_empty() {
                return Value::from_i128(0, 32, true); // EOF: dest UNCHANGED
            }
            let prior = self.st.read_net(net, None);
            let v = fill_reg_slots(&prior, w, &got);
            let lv = Lvalue {
                chunks: vec![sim_ir::LvalChunk {
                    net,
                    word: None,
                    offset: None,
                    width: None,
                    kind: sim_ir::SelKind::Bit,
                }],
            };
            let off = self.resolve_lvalue_offsets(&lv);
            self.k_write_lvalue(&lv, v, &off);
            return Value::from_i128(got.len() as i128, 32, true);
        }
        // ── memory: fill elements ascending from `start` (declared index) ──
        let Some(base) = crate::builtins::declared_array_base(&self.st.net_dims, net) else {
            self.st
                .sink
                .emit(diag::LogEvent::Diagnostic(diag::Diagnostic {
                    severity: diag::Severity::Warning,
                    code: diag::MsgCode::RunReadmem,
                    message: "$fread into a memory with a NEGATIVE declared base \
                              (e.g. `reg m[-1:1]`) is not supported; no elements read"
                        .to_string(),
                    location: None,
                    context: Vec::new(),
                    sim_time: Some(diag::TimeStamp { ticks: self.st.now }),
                }));
            return Value::from_i128(0, 32, true);
        };
        let last = base + alen - 1;
        // iverilog evaluates the start/count operands by coercing each x/z bit
        // to 0 (NOT by treating an x/z operand as absent), so a present operand
        // always counts — `4'b001x` => 2, `3'bxxx` => 0 (an explicit count 0).
        let coerce_xz0 = |v: &Value| -> u64 {
            v.val.first().copied().unwrap_or(0) & !v.unk.first().copied().unwrap_or(0)
        };
        let start = match start_arg {
            Some(a) => {
                let v = self.eval(a);
                coerce_xz0(&v)
            }
            None => base,
        };
        let count = count_arg.map(|a| {
            let v = self.eval(a);
            coerce_xz0(&v)
        });
        let warn = |sched: &mut Scheduler, msg: String| {
            sched
                .st
                .sink
                .emit(diag::LogEvent::Diagnostic(diag::Diagnostic {
                    severity: diag::Severity::Warning,
                    code: diag::MsgCode::RunReadmem,
                    message: msg,
                    location: None,
                    context: Vec::new(),
                    sim_time: Some(diag::TimeStamp {
                        ticks: sched.st.now,
                    }),
                }));
        };
        if start < base || start > last {
            warn(
                self,
                format!(
                    "$fread start argument ({start}) is outside the memory range [{base}:{last}]"
                ),
            );
            return Value::from_i128(0, 32, true);
        }
        let avail = last - start + 1;
        let n_elems = match count {
            Some(c) if c > avail => {
                warn(
                    self,
                    format!("$fread count argument ({c}) is too large for start ({start}) and the memory range [{base}:{last}]; clamped"),
                );
                avail
            }
            Some(c) => c,
            None => avail,
        };
        let mut rc: u64 = 0;
        for e in 0..n_elems {
            let word = (start + e - base) as u32;
            let mut got: Vec<u8> = Vec::new();
            let mut hit_eof = false;
            for _ in 0..cap_per {
                match crate::builtins::file_read_byte(self, fd) {
                    Some(b) => got.push(b),
                    None => {
                        hit_eof = true;
                        break;
                    }
                }
            }
            if got.is_empty() {
                break; // no more data — leave this and later elements untouched
            }
            rc += got.len() as u64;
            let prior = self.st.read_net(net, Some(word));
            let v = fill_reg_slots(&prior, w, &got);
            let lv = Lvalue {
                chunks: vec![sim_ir::LvalChunk {
                    net,
                    word: Some(0),
                    offset: None,
                    width: None,
                    kind: sim_ir::SelKind::Bit,
                }],
            };
            let off = crate::exec::Offsets::Inline {
                buf: [(0, word), (0, 0)],
                len: 1,
            };
            self.st.write_lvalue(&lv, v, &off);
            if hit_eof {
                break;
            }
        }
        Value::from_i128(rc as i128, 32, true)
    }
    fn k_fscanf_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fscanf_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_fscanf(&mut self, rhs: u32) -> Value {
        // args = [fd, fmt strconst, dst0, dst1, ...] — elaborate's contract.
        let args: Vec<u32> = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => args.clone(),
            _ => return Value::from_i128(-1, 32, true),
        };
        let fdv = self.eval(args[0]);
        if fdv.has_xz() {
            return Value::from_i128(-1, 32, true);
        }
        let fd = fdv.to_u64().unwrap_or(0) as u32;
        let fmt: Vec<u8> = match self.st.ir.exprs.get(args[1] as usize) {
            Some(sim_ir::Expr::Const { val }) => {
                crate::builtins::const_string(self.st.ir, *val).into_bytes()
            }
            _ => return Value::from_i128(-1, 32, true),
        };
        let dsts: Vec<u32> = args[2..]
            .iter()
            .filter_map(|&a| match self.st.ir.exprs.get(a as usize) {
                Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                _ => None,
            })
            .collect();
        scan_run(self, Some(fd), &[], &fmt, &dsts)
    }
    fn k_sscanf_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::sscanf_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_sscanf(&mut self, rhs: u32) -> Value {
        // args = [source string-VALUE, fmt strconst, dst0, ...].
        let args: Vec<u32> = match self.st.ir.exprs.get(rhs as usize) {
            Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() >= 2 => args.clone(),
            _ => return Value::from_i128(-1, 32, true),
        };
        let src: Vec<u8> = self.eval(args[0]).to_str_bytes();
        let fmt: Vec<u8> = match self.st.ir.exprs.get(args[1] as usize) {
            Some(sim_ir::Expr::Const { val }) => {
                crate::builtins::const_string(self.st.ir, *val).into_bytes()
            }
            _ => return Value::from_i128(-1, 32, true),
        };
        let dsts: Vec<u32> = args[2..]
            .iter()
            .filter_map(|&a| match self.st.ir.exprs.get(a as usize) {
                Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                _ => None,
            })
            .collect();
        scan_run(self, None, &src, &fmt, &dsts)
    }
    fn k_sformatf_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::sformatf_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_rhs_is_stmt_effect_family(&self, rhs: u32) -> bool {
        sim_ir::rhs_is_stmt_effect(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_sformatf(&mut self, rhs: u32) -> Value {
        // args = [fmt string-literal Const, value args…] (elaborate contract).
        let Some(sim_ir::Expr::SysFunc { args, .. }) = self.st.ir.exprs.get(rhs as usize) else {
            return Value::from_str_bytes(&[]);
        };
        let (fmt, rest) = (args.first().copied(), args.get(1..).unwrap_or(&[]).to_vec());
        let text = crate::builtins::format_args_str(&*self.st, fmt, &rest, None);
        Value::from_str_bytes(text.as_bytes())
    }
    fn k_disable_fork(&mut self) {
        // IEEE §9.6.3: terminate every ACTIVE DESCENDANT of the calling
        // process. Transitive walk: barriers parented by the kill set spread
        // to their children. The arena is append-only and the walk is
        // index-ordered — deterministic. Stale resume entries are dropped at
        // the `run_body` choke.
        let mut kill: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        kill.insert(self.cur_aid);
        loop {
            let mut grew = false;
            for (aid, a) in self.activities.iter().enumerate() {
                if a.dead || a.reported || kill.contains(&(aid as u32)) {
                    continue;
                }
                let Some(jr) = a.join_ref else { continue };
                let parent = self.barriers[jr as usize].parent;
                if kill.contains(&parent) {
                    kill.insert(aid as u32);
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        kill.remove(&self.cur_aid); // the caller itself lives on
                                    // §16.4: a deferred report pending in a KILLED process is cancelled (the
                                    // action never matures). Drop by `(aid, gen)` of the LIVE killed
                                    // activities so a recycled slot's COMPLETED predecessor report (a
                                    // different gen under the same aid) is NOT also cancelled.
        let mut kill_keys: std::collections::BTreeSet<(u32, u32)> =
            std::collections::BTreeSet::new();
        for &aid in &kill {
            self.activities[aid as usize].dead = true;
            kill_keys.insert((aid, self.activities[aid as usize].gen));
        }
        if !self.st.postponed.deferred_observed.is_empty() {
            self.st
                .postponed
                .deferred_observed
                .retain(|&(_, aid, gen), _| !kill_keys.contains(&(aid, gen)));
        }
        if !self.st.postponed.deferred_reactive.is_empty() {
            self.st
                .postponed
                .deferred_reactive
                .retain(|&(_, aid, gen), _| !kill_keys.contains(&(aid, gen)));
        }
    }
    fn k_queue_pop(&mut self, lhs: &Lvalue, rhs: u32) -> Value {
        // `k_queue_pop_rhs` guaranteed the shape; everything below is
        // defensive against a hand-built IR — degrade, never panic.
        let Some(sim_ir::Expr::SysFunc { which, args }) = self.st.ir.exprs.get(rhs as usize) else {
            return Value::xs(1, false);
        };
        let front = matches!(which, sim_ir::SysFuncId::QPopFront);
        let net = args
            .first()
            .and_then(|&a| match self.st.ir.exprs.get(a as usize) {
                Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
                _ => None,
            });
        let popped = match net {
            Some(n) => {
                let (w, signed) = self
                    .st
                    .ir
                    .nets
                    .get(n as usize)
                    .map(|nv| (nv.width.max(1), nv.signed))
                    .unwrap_or((1, false));
                // Scope the `borrow_mut` to the pop; the empty/non-queue warn
                // runs after it releases (§C6 — no heap guard across the warn).
                let popped_opt = {
                    let mut heap = self.st.dyn_heap.borrow_mut();
                    match heap.get_mut(n as usize).and_then(|o| o.as_mut()) {
                        Some(crate::state::DynObj::Queue { elems }) if !elems.is_empty() => {
                            let v = if front {
                                elems.pop_front()
                            } else {
                                elems.pop_back()
                            };
                            Some(v.unwrap_or_else(|| Value::xs(w, signed)))
                        }
                        _ => None,
                    }
                };
                match popped_opt {
                    Some(v) => v,
                    _ => {
                        // empty (a missing entry IS the empty queue) or a
                        // non-queue object: element-width X + warn-once
                        // (iverilog live: per-call warning + x; our once-latch
                        // is the established anti-spam policy).
                        self.st.dyn_warn_once_at(n, "pop on an empty queue (X)");
                        Value::xs(w, signed)
                    }
                }
            }
            None => Value::xs(1, false),
        };
        // Context-size EXACTLY as `k_eval_for_lvalue` sizes an rhs: width =
        // max(lhs width, pop self-width), extension driven by the pop's
        // self-signedness (= the ELEMENT's, via the width table).
        let lw = self.st.lvalue_width(lhs);
        let sw = self.st.wt.get(rhs);
        popped.resize_keep_sign(lw.max(sw.width), sw.signed)
    }
    fn k_assoc_iter_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::assoc_iter_rhs(self.st.ir.exprs.as_slice(), rhs)
    }
    fn k_assoc_iter(&mut self, lhs: &Lvalue, rhs: u32) -> Value {
        let status = self.assoc_iter_step(rhs);
        // Context-size the int status exactly as `k_queue_pop` sizes its
        // result (self-width of the rhs = 32 signed via the width table).
        let mut v = Value::zeros(32, true);
        v.val[0] = (status as u32) as u64;
        let lw = self.st.lvalue_width(lhs);
        let sw = self.st.wt.get(rhs);
        v.resize_keep_sign(lw.max(sw.width), sw.signed)
    }

    // ── terminator / control surface (C1) — pure forwarders ──
    fn k_truthy(&self, eid: u32) -> bool {
        self.truthy(eid)
    }
    fn k_truthy_value(&self, v: &Value) -> bool {
        self.truthy_value(v)
    }
    fn k_rearm(&mut self, proc: u32) {
        self.rearm(proc);
    }
    fn k_enter_body(&mut self, tmpl: u32) {
        crate::exec::enter_body(self.st, tmpl as usize);
    }
    fn k_now(&self) -> u64 {
        self.st.now
    }
    fn k_delta_budget(&self) -> u64 {
        self.max_deltas
    }
    fn k_time_limit(&self) -> Option<u64> {
        self.time_limit
    }
    fn k_schedule_resume(&mut self, proc: u32, block: u32, tick: u64, inactive: bool) {
        self.schedule_resume(proc, block, tick, inactive);
    }
    fn k_call_fatal(&self) -> bool {
        self.st.call_fatal.get()
    }
    fn k_drain_diags(&mut self) {
        // Nothing to drain: this store emits at the access (`warn_run_range` is
        // called from `read_net` and from the write funnel). A no-op here is the
        // honest answer, not a stub — see the trait doc.
    }
    fn k_max_deltas(&self) -> u64 {
        self.max_deltas_guard()
    }
    fn k_mark_fatal(&mut self) {
        self.mark_fatal();
    }
    fn k_class_new_site(&self, sid: u32) -> Option<u32> {
        self.st.class_new_sites.get(&sid).copied()
    }
    fn k_class_alloc(&mut self, class_id: u32) -> Value {
        let id = self.st.class_alloc(class_id);
        // CLASS-HEAP-CAP: the heap is never garbage-collected, so an unbounded
        // `new()` in a loop would grow without limit. Bound it to a loud fatal
        // (graceful $finish) instead of an OOM — this is the single allocation
        // chokepoint (`class_alloc`'s only caller).
        if self.st.class_heap.borrow().len() as u64 > self.st.max_class_objs {
            self.st.fatal_class_limit();
        }
        // The handle holds the object-id as a 32-bit unsigned integer (0 = null).
        Value::from_i128(id as i128, 32, false)
    }
}
