//! system tasks — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// The sub-slice of a system task/func's args that are READ inputs — the write
/// DEST args of the string-building / scanf / file-read / `$cast` / `$value$plusargs`
/// families are excluded (a string populated by e.g. `$sformat`/`$sscanf`/`$fgets`/
/// `$cast`/`$value$plusargs` and never otherwise read is write-only, so must not trip
/// the coalesce read-gate). Every other task reads all its args. This must cover EVERY
/// sysfunc vita supports as a direct-rhs writer, else its write-only dest is misread.
pub(crate) fn syscall_read_args<'a>(task: &str, args: &'a [ast::Expr]) -> &'a [ast::Expr] {
    match task {
        // dest is arg 0; the remaining args are read inputs (`$cast(dst, src)` writes
        // dst=arg0, reads src=arg1)
        "$sformat" | "$swrite" | "$swriteb" | "$swriteh" | "$swriteo" | "$fgets" | "$fread"
        | "$cast" => args.get(1..).unwrap_or(&[]),
        // source + fmt (args 0,1) are the only reads; trailing args are write dests
        "$sscanf" | "$fscanf" => &args[..2.min(args.len())],
        // `$value$plusargs(fmt, dest)`: the format (arg 0) is read, dest (arg 1) written
        "$value$plusargs" => &args[..1.min(args.len())],
        _ => args,
    }
}

/// Count the ARG-CONSUMING conversion specifiers in a `$display`-family format
/// literal (`$sformatf`-hoist gate, §4.5.127). Scans the raw literal (quotes +
/// deferred escapes): a `\`-escape skips two bytes (so `\%`/`\"` never start a
/// spec), `%%` is a literal percent, and a `%` followed by optional flags/width/
/// precision then a known consumer char (`h x d o b c s t e f g v p u z`) counts
/// one. `%m` (scope) and `%l` (library) consume NO arg and are excluded; any
/// unknown/non-alphabetic conversion is NOT counted. `count >= value_args` gates
/// the hoist as a "no surplus value arg" test (a surplus string arg falls into
/// vita's pre-existing numeric-render-of-a-string gap, a silent-wrong the hoist
/// must not expose). SOUNDNESS: this char-set is exactly the one vita's own
/// `render_template` (sim-engine) consumes, so for WELL-FORMED formats the count
/// equals real consumption ⇒ no surplus is admitted. The flag-skip here is broader
/// than `render_template`'s (it also skips ` `/`#`/misordered flags), so on
/// MALFORMED formats it can OVERcount relative to render — but iverilog itself
/// warns ("unknown/invalid format") on exactly those, so an over-counted surplus
/// never lands on iverilog-clean code (correct-or-loud holds: such a case is a
/// category-b iverilog-warned divergence, not a silent-wrong on valid code).
/// Escaped/octal `%` (`\045`) UNDERcounts → stricter → loud (safe).
pub(crate) fn count_arg_specs(raw: &str) -> usize {
    let b = raw.as_bytes();
    let mut i = 0;
    let mut n = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2, // an escape sequence — skip it (\n, \", \%, …)
            b'%' => {
                i += 1;
                if i >= b.len() {
                    break; // trailing '%'
                }
                if b[i] == b'%' {
                    i += 1; // literal "%%"
                    continue;
                }
                // skip flags / field width / precision
                while i < b.len() && matches!(b[i], b'-' | b'+' | b' ' | b'#' | b'.' | b'0'..=b'9')
                {
                    i += 1;
                }
                if i >= b.len() {
                    break;
                }
                let c = b[i].to_ascii_lowercase();
                if matches!(
                    c,
                    b'h' | b'x'
                        | b'd'
                        | b'o'
                        | b'b'
                        | b'c'
                        | b's'
                        | b't'
                        | b'e'
                        | b'f'
                        | b'g'
                        | b'v'
                        | b'p'
                        | b'u'
                        | b'z'
                ) {
                    n += 1;
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    n
}

/// `$display`→Display … `$dumpall`→DumpAll. `name` retains the leading `$`
/// (parser keeps it, parallel to `map_sysfunc`). Unknown → None.
/// `$monitoron`/`$monitoroff`/`$timeformat` etc. are DEFERRED.
/// Severity-task name → [`SeverityKind`] (P1-1). These do NOT map to a frozen
/// `SysTaskId`; they lower as `Display` + an out-of-band severity entry.
pub(crate) fn map_severity(dollar_name: &str) -> Option<SeverityKind> {
    match dollar_name {
        "$fatal" => Some(SeverityKind::Fatal),
        "$error" => Some(SeverityKind::Error),
        "$warning" => Some(SeverityKind::Warning),
        "$info" => Some(SeverityKind::Info),
        // Parser-synthesized `unique`/`priority` violation report — same
        // machinery as the severity tasks, its own diagnostic code. The name is
        // shared with the producer as a constant so the two cannot drift.
        ast::UNIQUE_VIOLATION_TASK => Some(SeverityKind::UniqueViolation),
        _ => None,
    }
}

/// b/o/h print-task variant → its default radix (P1-5). Exact-match (so
/// `$monitoron`/`$monitoroff` never alias `$monitoro` + a stray suffix).
pub(crate) fn radix_of_systask(dollar_name: &str) -> Option<u8> {
    match dollar_name {
        "$displayb" | "$writeb" | "$strobeb" | "$monitorb" | "$fdisplayb" | "$fwriteb"
        | "$swriteb" => Some(2),
        "$displayo" | "$writeo" | "$strobeo" | "$monitoro" | "$fdisplayo" | "$fwriteo"
        | "$swriteo" => Some(8),
        "$displayh" | "$writeh" | "$strobeh" | "$monitorh" | "$fdisplayh" | "$fwriteh"
        | "$swriteh" => Some(16),
        _ => None,
    }
}

/// True for the FILE-directed `$monitor`/`$strobe` twins, whose first argument is a
/// descriptor. They share the frozen `Monitor`/`Strobe` ids, so this name test is the
/// only thing that distinguishes them — used BOTH by the fmt/args split and by the
/// sidecar record, so the two cannot disagree about which argument is the fd.
pub(crate) fn is_file_monitor_strobe(dollar_name: &str) -> bool {
    matches!(
        dollar_name,
        "$fmonitor"
            | "$fmonitorb"
            | "$fmonitoro"
            | "$fmonitorh"
            | "$fstrobe"
            | "$fstrobeb"
            | "$fstrobeo"
            | "$fstrobeh"
    )
}

pub(crate) fn map_systask(dollar_name: &str) -> Option<ir::SysTaskId> {
    match dollar_name {
        "$display" | "$displayb" | "$displayo" | "$displayh" => Some(ir::SysTaskId::Display),
        "$write" | "$writeb" | "$writeo" | "$writeh" => Some(ir::SysTaskId::Write),
        "$monitor" | "$monitorb" | "$monitoro" | "$monitorh" => Some(ir::SysTaskId::Monitor),
        "$strobe" | "$strobeb" | "$strobeo" | "$strobeh" => Some(ir::SysTaskId::Strobe),
        "$finish" => Some(ir::SysTaskId::Finish),
        // SYS-INTRO잔여: `$exit` waits for program blocks then ends; with no
        // `program` constructs in vita it is exactly `$finish` (IR-0, no new id).
        "$exit" => Some(ir::SysTaskId::Finish),
        "$stop" => Some(ir::SysTaskId::Stop),
        "$dumpfile" => Some(ir::SysTaskId::DumpFile),
        "$dumpvars" => Some(ir::SysTaskId::DumpVars),
        "$dumpon" => Some(ir::SysTaskId::DumpOn),
        "$dumpoff" => Some(ir::SysTaskId::DumpOff),
        "$dumpall" => Some(ir::SysTaskId::DumpAll),
        "$dumpflush" => Some(ir::SysTaskId::DumpFlush),
        "$dumplimit" => Some(ir::SysTaskId::DumpLimit),
        // v7 file I/O ($fopen is a special form — it returns the fd).
        "$readmemb" => Some(ir::SysTaskId::ReadmemB),
        "$readmemh" => Some(ir::SysTaskId::ReadmemH),
        // v9: the write-side mirror of $readmem* (Medium-bundle rank 5).
        "$writememb" => Some(ir::SysTaskId::WritememB),
        "$writememh" => Some(ir::SysTaskId::WritememH),
        "$fclose" => Some(ir::SysTaskId::Fclose),
        // $swrite* — "$write to a string": same engine as $sformat (dest = args[0],
        // a leading string-literal is the format, every other arg renders $write-
        // style via `format_args_str`). The b/o/h variants set the default radix of
        // unformatted args through `radix_of_systask`. IEEE 1364-2005 §21.3.3.
        "$sformat" | "$swrite" | "$swriteb" | "$swriteo" | "$swriteh" => {
            Some(ir::SysTaskId::Sformat)
        }
        "$fdisplay" | "$fdisplayb" | "$fdisplayo" | "$fdisplayh" => Some(ir::SysTaskId::Fdisplay),
        "$fwrite" | "$fwriteb" | "$fwriteo" | "$fwriteh" => Some(ir::SysTaskId::Fwrite),
        // `$fmonitor`/`$fstrobe` are the FILE-directed twins of `$monitor`/`$strobe` —
        // identical postponed semantics, output routed to `args[0]`'s descriptor instead
        // of stdout. They reuse the FROZEN `Monitor`/`Strobe` ids rather than adding
        // variants (which would flip the SimIr schema hash and re-pin every golden); the
        // fd-ness rides the `file_directed_stmts` sidecar, keyed by StmtId.
        "$fmonitor" | "$fmonitorb" | "$fmonitoro" | "$fmonitorh" => Some(ir::SysTaskId::Monitor),
        "$fstrobe" | "$fstrobeb" | "$fstrobeo" | "$fstrobeh" => Some(ir::SysTaskId::Strobe),
        // v9 rank 6: monitor enable/disable + the $cast TASK form (`$cast(d, s);`
        // as a statement — the func form `ok = $cast(d, s)` is a direct-rhs
        // intercept, see `cast_special`).
        "$monitoron" => Some(ir::SysTaskId::MonitorOn),
        "$monitoroff" => Some(ir::SysTaskId::MonitorOff),
        "$cast" => Some(ir::SysTaskId::Cast),
        _ => None,
    }
}

/// `$time`→Time, `$realtime`→Realtime, `$signed`→Signed, `$unsigned`→Unsigned,
/// `$clog2`→Clog2. `name` retains the leading `$` (verdict M6).
pub(crate) fn map_sysfunc(dollar_name: &str) -> Option<ir::SysFuncId> {
    match dollar_name {
        "$time" => Some(ir::SysFuncId::Time),
        "$realtime" => Some(ir::SysFuncId::Realtime),
        "$signed" => Some(ir::SysFuncId::Signed),
        "$unsigned" => Some(ir::SysFuncId::Unsigned),
        "$clog2" => Some(ir::SysFuncId::Clog2),
        "$rtoi" => Some(ir::SysFuncId::Rtoi),
        "$itor" => Some(ir::SysFuncId::Itor),
        "$realtobits" => Some(ir::SysFuncId::RealToBits),
        "$bitstoreal" => Some(ir::SysFuncId::BitsToReal),
        // v7 bit-vector predicates ($bits never reaches here — const-folded).
        "$countones" => Some(ir::SysFuncId::CountOnes),
        "$onehot" => Some(ir::SysFuncId::OneHot),
        "$onehot0" => Some(ir::SysFuncId::OneHot0),
        "$isunknown" => Some(ir::SysFuncId::IsUnknown),
        // v7 random + time + plusarg probe ($value$plusargs is a special
        // form — it writes its ref var, never mapped here).
        "$test$plusargs" => Some(ir::SysFuncId::TestPlusargs),
        "$random" => Some(ir::SysFuncId::Random),
        "$urandom" => Some(ir::SysFuncId::Urandom),
        "$urandom_range" => Some(ir::SysFuncId::UrandomRange),
        "$stime" => Some(ir::SysFuncId::Stime),
        // Round-9 FIO: $feof is the ONLY PURE file function — it reads the fd's
        // EOF flag with no state mutation, so it is allowed in any expression /
        // condition context (`while (!$feof(fd))`). The fd-ADVANCING reads
        // ($fgetc/$fgets/$fread/$fscanf/$sscanf/$ungetc) are NOT mapped here — they
        // stay direct-rhs-only (a second evaluation under unspecified expression
        // order would double-advance the fd). The direct-rhs `e = $feof(fd)` form
        // is still intercepted first by `file_read_int_special` (byte-identical),
        // so this mapping is reached only for expression-context $feof.
        "$feof" => Some(ir::SysFuncId::Feof),
        // v19: N6 real-math (IEEE §20.8.2) — pure value functions, lowered like
        // any other SysFunc. The non-uniform $dist_* are NOT here — they advance
        // the ref seed and route through `dist_seeded_special` (direct-rhs only).
        "$ln" => Some(ir::SysFuncId::Ln),
        "$log10" => Some(ir::SysFuncId::Log10),
        "$exp" => Some(ir::SysFuncId::Exp),
        "$sqrt" => Some(ir::SysFuncId::Sqrt),
        "$pow" => Some(ir::SysFuncId::Pow),
        "$floor" => Some(ir::SysFuncId::Floor),
        "$ceil" => Some(ir::SysFuncId::Ceil),
        "$sin" => Some(ir::SysFuncId::Sin),
        "$cos" => Some(ir::SysFuncId::Cos),
        "$tan" => Some(ir::SysFuncId::Tan),
        "$asin" => Some(ir::SysFuncId::Asin),
        "$acos" => Some(ir::SysFuncId::Acos),
        "$atan" => Some(ir::SysFuncId::Atan),
        "$atan2" => Some(ir::SysFuncId::Atan2),
        "$hypot" => Some(ir::SysFuncId::Hypot),
        "$sinh" => Some(ir::SysFuncId::Sinh),
        "$cosh" => Some(ir::SysFuncId::Cosh),
        "$tanh" => Some(ir::SysFuncId::Tanh),
        "$asinh" => Some(ir::SysFuncId::Asinh),
        "$acosh" => Some(ir::SysFuncId::Acosh),
        "$atanh" => Some(ir::SysFuncId::Atanh),
        _ => None,
    }
}

/// The N6 real-math system functions (IEEE §20.8.2) — all return `real`. Their
/// declared arity: `$pow`/`$atan2`/`$hypot` take 2 args, the rest take 1.
pub(crate) fn real_math_arity(which: ir::SysFuncId) -> Option<usize> {
    use ir::SysFuncId::*;
    match which {
        Pow | Atan2 | Hypot => Some(2),
        Ln | Log10 | Exp | Sqrt | Floor | Ceil | Sin | Cos | Tan | Asin | Acos | Atan | Sinh
        | Cosh | Tanh | Asinh | Acosh | Atanh => Some(1),
        _ => None,
    }
}

impl Elaborator<'_> {
    /// Per-bit X/Z→0 coercion for a 2-state cast target (§6.11.3). Bitwise ops
    /// propagate X, but case-equality RESOLVES it: bit `i` becomes `(e[i] === 1'b1)`
    /// (1 iff known-1, else 0). Re-assembled MSB-first into a `tw`-bit value.
    pub(crate) fn coerce_two_state(&mut self, e: u32, tw: u32) -> u32 {
        let one = self.const_u32_expr(1, 1);
        let mut parts: Vec<u32> = Vec::with_capacity(tw as usize);
        for i in (0..tw).rev() {
            let off = self.const_u32_expr(i, 32);
            let wid = self.const_u32_expr(1, 32);
            let bit = self.push_expr(ir::Expr::Select {
                base: e,
                offset: off,
                width: wid,
                kind: ir::SelKind::Bit,
            });
            let known = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::CaseEq,
                lhs: bit,
                rhs: one,
            });
            parts.push(known);
        }
        self.push_expr(ir::Expr::Concat { parts })
    }

    /// Emit the write for a SYS-READ special form (`$fscanf`/`$sscanf`/`$fgets`/
    /// `$fread`). As an assignment rhs (`Some(lhs)`) the returned count is written
    /// to `lhs`. As a BARE statement (`None`) the count is DISCARDED — but the
    /// destination writes (the side-effect of EVALUATING the `SysFunc`) must still
    /// happen, so the SysFunc is evaluated via `emit_discarded_call` (a throwaway
    /// assign to a fresh net). Without this a bare `$sscanf(str,fmt,a);` silently
    /// never wrote `a` (iverilog writes it regardless of the return being used).
    pub(crate) fn emit_sysread_write(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: Option<&ast::Lvalue>,
        rhs_id: u32,
    ) {
        match lhs {
            Some(lhs) => {
                let lv = self.lower_lvalue(lhs);
                self.check_lvalue_kind(&lv, true);
                let sid = self.push_stmt(ir::Stmt::BlockingAssign {
                    lhs: lv,
                    rhs: rhs_id,
                });
                b.push_stmt_id(sid);
            }
            None => self.emit_discarded_call(b, rhs_id),
        }
    }

    // ── $systask lowering (SysTaskId map + fmt/args split) ─────────
    /// `$display(...)` etc. → `ir::Stmt::SysTask` appended to `self.stmts`;
    /// returns its StmtId. Unknown `$task` → `ElabUnsupported`, `None` (skip).
    /// fmt/args split: for the print family the FIRST arg, IF it is a string
    /// literal, becomes `fmt`; the rest are value args. Non-print tasks
    /// ($finish/$dumpfile/...) carry `fmt: None`, every arg in `args`.
    pub(crate) fn lower_systask(&mut self, name: &ast::Ident, args: &[ast::Expr]) -> Option<u32> {
        // ⚠️ NOT a value fix — vita and iverilog agree here, byte for byte.
        // `$display(n == 1 ? "[PASS] …" : "[FAIL] …")` prints a large decimal number
        // in BOTH, because IEEE 1800 §5.9 makes a string literal a packed integral,
        // the ternary puts both arms in an integral context, and a single non-format
        // argument prints as decimal. The user's expectation is what is wrong.
        //
        // It is still worth saying: this exact shape is always a mistake (nobody wants
        // the number), it is the ordinary way to write a PASS/FAIL line, and it made a
        // whole test log unreadable. Narrow on purpose — ONE argument, a ternary, a
        // string LITERAL on both arms — so nothing else can trip it.
        if matches!(name.name.as_str(), "$display" | "$write") {
            if let [only] = args {
                if let ast::ExprKind::Ternary { then_e, else_e, .. } = &only.kind {
                    let lit = |e: &ast::Expr| {
                        matches!(
                            &e.kind,
                            ast::ExprKind::StrLit { .. }
                                | ast::ExprKind::Paren { .. } if Self::param_str_literal(e).is_some()
                        )
                    };
                    if lit(then_e) && lit(else_e) {
                        self.warn_code(
                            MsgCode::ElabStrTernaryNumeric,
                            &format!(
                                "`{}` was given ONE argument that is a ternary of string \
                             literals — IEEE 1800 §5.9 makes those packed integers, so \
                             this prints a decimal number, not text (use `if`/`else`, \
                             or `{}(\"%s\", cond ? \"a\" : \"b\")`)",
                                name.name, name.name
                            ),
                        );
                    }
                }
            }
        }
        // SVA-REST `$assertoff`/`$asserton`/`$assertkill` (IEEE 1800 §20.11): runtime
        // assertion control. Lowered to a no-op `Display` (no fmt/args) whose StmtId is
        // recorded in `assert_ctl`; the engine flips the global assertion-enable when it
        // reaches the stmt and suppresses gated fires (`assert_fire`) while disabled. A
        // hierarchical/`level` argument is accepted-and-ignored (global subset).
        if let Some(kind) = match name.name.as_str() {
            "$assertoff" => Some(0u8),
            "$asserton" => Some(1u8),
            "$assertkill" => Some(2u8),
            _ => None,
        } {
            // Only the GLOBAL no-argument form is supported. A `levels`/`scope_list`
            // argument (`$assertoff(1, top.cA)`, IEEE 1800 §20.12) would restrict the
            // control to named scopes — vita has no per-scope assertion grouping, so
            // accept-and-ignoring the scope would SILENTLY over-disable (suppress a
            // sibling scope's legitimate violation → false PASS). Loud-reject instead.
            if !args.is_empty() {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "`{}` with a levels/scope argument is unsupported (only the global \
                         no-argument form is supported; a scoped control would silently \
                         over-disable)",
                        name.name
                    ),
                );
                return None;
            }
            let sid = self.push_stmt(ir::Stmt::SysTask {
                which: ir::SysTaskId::Display,
                fmt: None,
                args: Vec::new(),
            });
            self.assert_ctl.insert(sid, kind);
            return Some(sid);
        }
        // §21.3.2 `$timeformat[(units, precision, suffix, min_width)]` — a no-op
        // `Display` stmt + a `timeformat_stmts` side entry (the assert_ctl /
        // severity pattern; the frozen SysTaskId gains no variant). Args stay
        // RUNTIME expressions — the engine evaluates them at execution time
        // (variable units/suffix are legal, iverilog-pinned). Arity is 0 or 4
        // exactly (iverilog: "$timeformat requires zero or four arguments").
        if name.name == "$timeformat" {
            if !(args.is_empty() || args.len() == 4) {
                self.error(
                    MsgCode::ElabUnsupported,
                    "$timeformat requires zero or four arguments (units, precision, \
                     suffix, min_field_width)",
                );
                return None;
            }
            // A `$timeformat` as a DEFERRED-assert action would be captured by the
            // §16.4 push_stmt hook (every SysTask under `cur_defer` lands in
            // `defer_acts`) and the engine's `try_defer` intercepts BEFORE the
            // timeformat check — the call would silently print its args at
            // maturation instead of updating the `%t` state. Loud-reject the
            // combination (soundness review Q1); use a plain statement instead.
            if self.cur_defer.is_some() {
                self.error(
                    MsgCode::ElabUnsupported,
                    "$timeformat as a deferred-assertion action is unsupported \
                     (it would be captured for maturation instead of updating the \
                     %t format state) — call it as a plain statement",
                );
                return None;
            }
            let arg_ids: Vec<u32> = args.iter().map(|a| self.lower_expr(a)).collect();
            let sid = self.push_stmt(ir::Stmt::SysTask {
                which: ir::SysTaskId::Display,
                fmt: None,
                args: arg_ids,
            });
            self.timeformat_stmts.insert(sid);
            return Some(sid);
        }
        // OBS-3: `$vita_stage("label", v0, v1, …)` — a vendor stage-trace task. Lowers
        // to a no-op `Display` (never prints) + a `stage_stmts` StmtId sidecar (the
        // $timeformat pattern; the frozen SysTaskId gains no variant). The engine
        // intercepts by StmtId and, when `+STAGE_TRACE` is set, appends a `stage.jsonl`
        // line `{v,t,kind:"stage",label,idx,vals[]}`; without the plusarg it is a pure
        // no-op. Args stay RUNTIME expressions. Requires ≥1 arg (the label). One-shot
        // `vita` only (velab loud-rejects a staged design with `$vita_stage`).
        if name.name == "$vita_stage" {
            if args.is_empty() {
                self.error(
                    MsgCode::ElabUnsupported,
                    "$vita_stage requires at least a label argument \
                     (`$vita_stage(\"label\"[, values…])`)",
                );
                return None;
            }
            if self.cur_defer.is_some() {
                self.error(
                    MsgCode::ElabUnsupported,
                    "$vita_stage as a deferred-assertion action is unsupported \
                     — call it as a plain statement",
                );
                return None;
            }
            let arg_ids: Vec<u32> = args.iter().map(|a| self.lower_expr(a)).collect();
            let sid = self.push_stmt(ir::Stmt::SysTask {
                which: ir::SysTaskId::Display,
                fmt: None,
                args: arg_ids,
            });
            self.stage_stmts.insert(sid);
            return Some(sid);
        }
        // P1-1: `$fatal`/`$error`/`$warning`/`$info` lower as `Display` stmts plus
        // an out-of-band SeverityTable entry (the frozen SysTaskId has no severity
        // variants; the engine intercepts by StmtId and routes to the diag stream).
        if let Some(sev) = map_severity(&name.name) {
            return Some(self.lower_severity_task(sev, args));
        }
        // §21.3.5 `$fflush[(fd|mcd)]` — flush a stdio buffer. vita holds open
        // files as raw UNBUFFERED `std::fs::File`s (each `$fwrite` `write_all`s
        // straight to the OS) and captures `$display`/STDOUT through a
        // deterministic sink, so there is NOTHING to flush: `$fflush` is a
        // provably-correct no-op, and a same-sim reopen-read already sees every
        // prior write. iverilog prints no diagnostic for it either, so accept-
        // and-drop SILENTLY (no Stmt, no warning) — matching iverilog's
        // observable behaviour — rather than the misleading "unsupported system
        // task skipped" the map_systask fallback would emit. (If vita ever adds
        // userspace file buffering, $fflush must become a real flush — see the
        // ROADMAP §3 `$fstrobe`/`$fmonitor` note, which shares the file path.)
        if name.name == "$fflush" {
            return None;
        }
        let which = match map_systask(&name.name) {
            Some(w) => w,
            None => {
                // M-D: unknown $task ($timeformat/$monitoron/$readmemh/...) is a
                // WARN + skip (no Stmt emitted), NOT an IR-killing error. The
                // testbench survives.
                self.warn(&format!(
                    "unsupported system task `{}` skipped (v2)",
                    name.name
                ));
                return None;
            }
        };
        // v9 rank 6: the $cast TASK form `$cast(dst, src);` must carry the SAME
        // loud guards as the func form (cast_special) — a non-plain destination
        // (memory element / select / concat) or wrong arity is E3009, NOT a silent
        // dropped no-op in the engine's cast_task (review H1).
        if matches!(which, ir::SysTaskId::Cast) {
            if args.len() != 2 {
                self.error(MsgCode::ElabUnsupported, "$cast takes (dest, source)");
                return None;
            }
            let dst_id = self.lower_expr(&args[0]);
            if !matches!(
                self.exprs.get(dst_id as usize),
                Some(ir::Expr::Signal { word: None, .. })
            ) {
                self.error(
                    MsgCode::ElabUnsupported,
                    "$cast destination must be a plain integral variable (v9 subset)",
                );
                return None;
            }
        }
        // $sformat / $swrite* write the rendered text into args[0], which must be a
        // whole register or SV string. The engine's Sformat handler writes ONLY a
        // `Signal{word:None}` dest and silently drops anything else (a part-select,
        // memory element, or concat) — iverilog loud-rejects those ("first argument
        // must be a register or SV string"), so match the oracle instead of the
        // silent no-op. (Mirrors the $cast dest guard above; the check is exactly the
        // engine's accepted shape, so no destination it CAN write is rejected.)
        if matches!(which, ir::SysTaskId::Sformat) {
            let ok = args.first().is_some_and(|a| {
                let d = self.lower_expr(a);
                matches!(
                    self.exprs.get(d as usize),
                    Some(ir::Expr::Signal { word: None, .. })
                )
            });
            if !ok {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "{}'s first argument must be a whole register or SV string",
                        name.name
                    ),
                );
                return None;
            }
        }
        let takes_fmt = matches!(
            which,
            ir::SysTaskId::Display
                | ir::SysTaskId::Write
                | ir::SysTaskId::Monitor
                | ir::SysTaskId::Strobe
        );
        // M-D: $dumpvars(level, scope...) passes a scope/module name, not a net.
        // Lowering a scope ident through lower_expr would resolve_net → fatal
        // unresolved-name. For the dump family, drop any non-net/non-const arg
        // with a warning instead of resolving it.
        let dump_family = matches!(
            which,
            ir::SysTaskId::DumpVars
                | ir::SysTaskId::DumpFile
                | ir::SysTaskId::DumpOn
                | ir::SysTaskId::DumpOff
                | ir::SysTaskId::DumpAll
        );
        let mut fmt_raw: Option<String> = None;
        // v7 file print family: args[0] is the DESCRIPTOR; the format (when a
        // string literal) is args[1]. Stmt args stay [fd, value-args…].
        // Keyed on `which` for the ids that are unambiguously file-directed, PLUS the
        // name for `$fmonitor`/`$fstrobe`, whose `which` is the shared `Monitor`/`Strobe`
        // id. Getting this wrong would treat the descriptor as a value argument and print
        // it — so the two places that decide fd-ness (this split and the sidecar record
        // below) both read `is_file_monitor_strobe`.
        let file_fmt = matches!(
            which,
            ir::SysTaskId::Fdisplay | ir::SysTaskId::Fwrite | ir::SysTaskId::Sformat
        ) || is_file_monitor_strobe(&name.name);
        let mut file_args_buf: Vec<ast::Expr> = Vec::new();
        let (fmt, value_args): (Option<u32>, &[ast::Expr]) = if file_fmt {
            match args.get(1) {
                Some(ast::Expr {
                    kind: ast::ExprKind::StrLit { raw },
                    span,
                }) => {
                    fmt_raw = Some(parse_str_literal_text(raw));
                    let cid = self.intern_str_literal(raw, *span);
                    let fmt_expr = self.push_expr(ir::Expr::Const { val: cid });
                    file_args_buf.push(args[0].clone());
                    file_args_buf.extend(args.iter().skip(2).cloned());
                    (Some(fmt_expr), file_args_buf.as_slice())
                }
                _ => (None, args),
            }
        } else if takes_fmt {
            match args.first() {
                Some(ast::Expr {
                    kind: ast::ExprKind::StrLit { raw },
                    span,
                }) => {
                    fmt_raw = Some(parse_str_literal_text(raw));
                    let cid = self.intern_str_literal(raw, *span);
                    let fmt_expr = self.push_expr(ir::Expr::Const { val: cid });
                    (Some(fmt_expr), &args[1..])
                }
                _ => (None, args),
            }
        } else {
            (None, args)
        };
        // r19/B3: `$readmem*`/`$writemem*` take `(file, mem, start, end)` — args 2
        // and 3 are ADDRESSES and must be integral. This lowering site handles every
        // system-task argument, so gating it wholesale would false-loud
        // `$display("%f", R)`; gate the two positions that are addresses instead. A
        // real param reached the engine here as an f64 BIT PATTERN read as an
        // address (`address 4607182418800017408 outside the load range`), which
        // silently loaded nothing and, on the write side, silently wrote no file.
        let addr_positions = matches!(
            which,
            ir::SysTaskId::ReadmemB
                | ir::SysTaskId::ReadmemH
                | ir::SysTaskId::WritememB
                | ir::SysTaskId::WritememH
        );
        let arg_ids: Vec<u32> = value_args
            .iter()
            .enumerate()
            .filter_map(|(argi, a)| {
                if addr_positions && argi >= 2 {
                    return Some(self.lower_index_expr(a));
                }
                // `$dumpvars(level, scope)` — the level const and a scope/module
                // ident. v1 dumps ALL signals (a valid superset of any requested
                // depth/scope), so a scope ident is silently dropped here rather
                // than warned: scope/depth-SELECTIVE dumping is a refinement, but
                // the common `$dumpvars(0, top)` idiom must not spew a warning.
                if dump_family && !self.is_net_or_const_arg(a) {
                    // ⑤b: a scope/module arg encodes as a SYNTHETIC string
                    // const carrying two candidates `fq\x01raw` — the runtime
                    // filter tries the elaborate-scope-resolved FQ first, then
                    // the raw text as a root-absolute path. Non-ident args
                    // keep the historical silent drop.
                    if let ast::ExprKind::Ident(p) = &a.kind {
                        let joined = p
                            .segments
                            .iter()
                            .map(|s| s.name.as_str())
                            .collect::<Vec<_>>()
                            .join(".");
                        let fq = self.fq(&joined);
                        let enc = format!("{fq}\u{0001}{joined}");
                        let cid =
                            self.intern_const(crate::literal::str_const_from_bytes(enc.as_bytes()));
                        Some(self.push_expr(ir::Expr::Const { val: cid }))
                    } else {
                        None
                    }
                } else {
                    // Item-⑤ status quo: a whole-array `$dumpvars(1, mem)` arg
                    // keeps its historical word-0 surface (doc-01 known v1
                    // simplification: v1 dumps ALL signals anyway) — the
                    // Phase-1.x ② whole-array loud check must not fire here.
                    // v7: $readmem's MEMORY argument is the same whole-array
                    // Signal shape (the engine writes elements via the funnel).
                    let readmem_family = matches!(
                        which,
                        ir::SysTaskId::ReadmemB
                            | ir::SysTaskId::ReadmemH
                            | ir::SysTaskId::WritememB
                            | ir::SysTaskId::WritememH
                    );
                    if dump_family || readmem_family {
                        if let Some((net, lead)) = self.expr_array_view(a) {
                            if lead.is_empty() {
                                // A2a: $readmemb/h WRITE the memory — a desugared
                                // array-parameter target is loud ($writemem/$dump
                                // only READ, so they pass).
                                if matches!(
                                    which,
                                    ir::SysTaskId::ReadmemB | ir::SysTaskId::ReadmemH
                                ) {
                                    self.deny_const_param_write(net, "$readmem into");
                                }
                                return Some(self.push_expr(ir::Expr::Signal { net, word: None }));
                            }
                        }
                    }
                    Some(self.lower_expr(a))
                }
            })
            .collect();
        // §4.1a STATIC gate: a `%b/%h/%o/%x` conversion specifier paired with a
        // real-typed argument is illegal (real has no radix form; use $realtobits).
        if let Some(fmt_str) = &fmt_raw {
            self.check_format_real_radix(fmt_str, &arg_ids);
        }
        let sid = self.push_stmt(ir::Stmt::SysTask {
            which,
            fmt,
            args: arg_ids,
        });
        // P1-5: the b/o/h print variants change the DEFAULT radix of unformatted
        // args — record it out-of-band (frozen SysTaskId has no radix variants).
        if let Some(r) = radix_of_systask(&name.name) {
            self.radixes.insert(sid, r);
        }
        // Family D (r17): a GENUINE `$display`/`$write` print (every special Display —
        // severity/timeformat/stage/assert-ctl — returned early above, so reaching here
        // with a Display/Write `which` is a real print). Record it so
        // `classify_frame_body` admits it in a subset function/task body and the `&self`
        // executors render it.
        if matches!(which, ir::SysTaskId::Display | ir::SysTaskId::Write) {
            self.frame_print_stmts.insert(sid);
        }
        // `$fmonitor`/`$fstrobe`: mark this call site file-directed so the engine reads
        // `args[0]` as a descriptor and routes the postponed render through `file_write`.
        if is_file_monitor_strobe(&name.name) {
            self.file_directed_stmts.insert(sid);
        }
        Some(sid)
    }

    /// P1-1: lower `$fatal([finish_number][, fmt, args…])` / `$error`/`$warning`/
    /// `$info([fmt, args…])` to a `SysTaskId::Display` stmt + a [`SeverityTable`]
    /// entry keyed by its StmtId. `$fatal`'s leading INTEGER LITERAL is the IEEE
    /// finish_number — consumed and ignored (like `$finish(n)`), never printed.
    /// The fmt/args split mirrors the print family (first string literal = fmt).
    pub(crate) fn lower_severity_task(&mut self, sev: SeverityKind, args: &[ast::Expr]) -> u32 {
        let args: &[ast::Expr] = if sev == SeverityKind::Fatal
            && matches!(
                args.first().map(|e| &e.kind),
                Some(ast::ExprKind::IntLit { .. })
            ) {
            &args[1..]
        } else {
            args
        };
        let mut fmt_raw: Option<String> = None;
        let (fmt, value_args): (Option<u32>, &[ast::Expr]) = match args.first() {
            Some(ast::Expr {
                kind: ast::ExprKind::StrLit { raw },
                span,
            }) => {
                fmt_raw = Some(parse_str_literal_text(raw));
                let cid = self.intern_str_literal(raw, *span);
                let fmt_expr = self.push_expr(ir::Expr::Const { val: cid });
                (Some(fmt_expr), &args[1..])
            }
            _ => (None, args),
        };
        let arg_ids: Vec<u32> = value_args.iter().map(|a| self.lower_expr(a)).collect();
        if let Some(fmt_str) = &fmt_raw {
            self.check_format_real_radix(fmt_str, &arg_ids);
        }
        let sid = self.push_stmt(ir::Stmt::SysTask {
            which: ir::SysTaskId::Display,
            fmt,
            args: arg_ids,
        });
        self.severities.insert(sid, sev);
        // SVA-REST: a fire `$error` lowered while a checker body is being synthesized
        // is an ASSERTION fire — record its StmtId so `$assertoff`/`$assertkill` can
        // suppress it at runtime.
        if self.in_assert_synth {
            self.assert_fire.insert(sid);
        }
        sid
    }
}
