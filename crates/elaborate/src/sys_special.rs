//! file/plusargs system-call specials — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// v7: `ok = $value$plusargs(fmt, var)` special form (the seeded-$random
    /// family — the engine writes `var` in the WRITE phase). The fmt must be
    /// a string LITERAL with at most one conversion spec from the supported
    /// set (%d/%h/%x/%o/%b/%s — %e/%f/%g real conversions are loud-deferred);
    /// `var` must lower to a plain whole-net Signal.
    /// Validate `$value$plusargs(format, var)` args and build the
    /// `SysFunc{ValuePlusargs}` ExprId (which writes `var` and returns 1/0 when
    /// the engine evaluates it). `None` (after emitting a diagnostic) on a bad
    /// arity / non-literal format / unsupported spec / non-plain-variable target.
    /// Shared by the statement form (`r = $value$plusargs(…)`) and the
    /// if-condition form (`if ($value$plusargs(…)) …`, B1).
    pub(crate) fn value_plusargs_rhs(&mut self, args: &[ast::Expr]) -> Option<u32> {
        if args.len() != 2 {
            self.error(
                MsgCode::ElabUnsupported,
                "$value$plusargs takes (format, variable)",
            );
            return None;
        }
        let ast::ExprKind::StrLit { .. } = &args[0].kind else {
            self.error(
                MsgCode::ElabUnsupported,
                "$value$plusargs needs a string-literal format (v7)",
            );
            return None;
        };
        let fmt_id = self.lower_expr(&args[0]);
        // validate the conversion set on the DECODED text (the const pool
        // holds the unescaped bytes the engine will see).
        if let Some(ir::Expr::Const { val }) = self.exprs.get(fmt_id as usize) {
            let c = &self.consts[*val as usize];
            let mut bytes = Vec::new();
            let nbytes = (c.width as usize).div_ceil(8);
            for bi in (0..nbytes).rev() {
                let bit = bi * 8;
                let w = bit / 64;
                let sh = bit % 64;
                bytes.push((c.bits.val.get(w).copied().unwrap_or(0) >> sh) as u8);
            }
            let text: String = String::from_utf8_lossy(&bytes).into_owned();
            let specs: Vec<char> = text
                .match_indices('%')
                .filter_map(|(i, _)| text[i + 1..].chars().next())
                .collect();
            if specs.len() > 1
                || specs.first().is_some_and(|c| {
                    !matches!(
                        c,
                        'd' | 'D' | 'h' | 'H' | 'x' | 'X' | 'o' | 'O' | 'b' | 'B' | 's' | 'S'
                    )
                })
            {
                self.error(
                    MsgCode::ElabUnsupported,
                    "$value$plusargs format supports one %d/%h/%x/%o/%b/%s spec (v7)",
                );
                return None;
            }
        }
        let var_id = self.lower_expr(&args[1]);
        let Some(ir::Expr::Signal { net, word: None }) = self.exprs.get(var_id as usize) else {
            self.error(
                MsgCode::ElabUnsupported,
                "$value$plusargs target must be a plain variable (v7)",
            );
            return None;
        };
        // A2a: $value$plusargs WRITES the dest.
        let net = *net;
        if net == POISON_NET && self.is_deferred_hier_sel_dest(var_id) {
            self.error(
                MsgCode::ElabUnsupported,
                "a $value$plusargs target cannot be a hierarchical element select \
                 (v7) — read into a local variable",
            );
            return None;
        }
        self.deny_const_param_write(net, "$value$plusargs into");
        Some(self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::ValuePlusargs,
            args: vec![fmt_id, var_id],
        }))
    }

    pub(crate) fn value_plusargs_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: Option<&ast::Lvalue>,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        let ast::ExprKind::SysCall { name, args } = &rhs.kind else {
            return false;
        };
        if name.name != "$value$plusargs" {
            return false;
        }
        let Some(rhs_id) = self.value_plusargs_rhs(args) else {
            return true; // a diagnostic was already emitted
        };
        match (lhs, delay) {
            (Some(lhs), Some(d)) => {
                // capture-now/write-later (the shared intra-assignment desugar,
                // assignment form only) — the plusarg search and var write happen
                // at the CAPTURE.
                let lv = self.lower_lvalue(lhs);
                self.check_lvalue_kind(&lv, true);
                let w = self.ir_lvalue_width(&lv);
                let tmp = self.fresh_ia_tmp(w);
                let cap = self.push_stmt(ir::Stmt::BlockingAssign {
                    lhs: whole_net_lvalue(tmp),
                    rhs: rhs_id,
                });
                b.push_stmt_id(cap);
                let (amount, region) = self.lower_delay(d);
                let resume = b.new_block();
                b.end_block_with(ir::Terminator::Delay {
                    amount,
                    region,
                    resume: resume.raw(),
                });
                b.start_block(resume);
                let tmp_read = self.push_expr(ir::Expr::Signal {
                    net: tmp,
                    word: None,
                });
                let wr = self.push_stmt(ir::Stmt::BlockingAssign {
                    lhs: lv,
                    rhs: tmp_read,
                });
                b.push_stmt_id(wr);
            }
            // Assignment form (no delay) → BlockingAssign(lhs); a BARE statement
            // (`None`, no intra-delay possible) → evaluate the SysFunc for its
            // ref-var write side-effect and discard the returned OK flag.
            (lhs, _) => self.emit_sysread_write(b, lhs, rhs_id),
        }
        true
    }

    /// v7: `fd = $fopen(name[, mode])` special form — the open mutates the
    /// engine file table (WRITE phase). Both arguments must be string
    /// LITERALS (a runtime filename needs the P2-C string type).
    pub(crate) fn fopen_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        let ast::ExprKind::SysCall { name, args } = &rhs.kind else {
            return false;
        };
        if name.name != "$fopen" {
            return false;
        }
        if args.is_empty() || args.len() > 2 {
            self.error(MsgCode::ElabUnsupported, "$fopen takes (name[, mode])");
            return true;
        }
        // §21.3: the filename/mode may be a string LITERAL, a runtime `string`
        // value (variable / concatenation), or a packed reg holding ASCII — the
        // engine's `k_fopen` resolves all three (Const → is_str → packed-chars).
        // Previously literal-only (v7); relaxed so file-driven testbenches that
        // build a path in a `string` variable open the file (iverilog parity).
        let arg_ids: Vec<u32> = args.iter().map(|a| self.lower_expr(a)).collect();
        let rhs_id = self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::Fopen,
            args: arg_ids,
        });
        let lv = self.lower_lvalue(lhs);
        self.check_lvalue_kind(&lv, true);
        if delay.is_some() {
            // exotic; keep the contract narrow + loud rather than guessing
            // open-now/assign-later semantics nobody writes.
            self.error(
                MsgCode::ElabUnsupported,
                "intra-assignment delay on $fopen is unsupported (v7)",
            );
            return true;
        }
        let sid = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: lv,
            rhs: rhs_id,
        });
        b.push_stmt_id(sid);
        true
    }

    /// v9 file-READ int-returning special forms: `c = $fgetc(fd)`,
    /// `e = $feof(fd)`, `r = $ungetc(c, fd)`. Each reads/advances the fd read
    /// state, so it is a statement-level effect (WRITE phase) like `$fopen` —
    /// legal ONLY as the direct rhs of a blocking assign; the int result is
    /// assigned to `lhs`. Returns false (unhandled) when `rhs` is some other
    /// SysCall. This recognizer + the lower_expr loud-reject guard MUST stay in
    /// sync (the guard catches any non-direct-rhs placement of these names).
    pub(crate) fn file_read_int_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: Option<&ast::Lvalue>,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        let ast::ExprKind::SysCall { name, args } = &rhs.kind else {
            return false;
        };
        let (which, arity, sig) = match name.name.as_str() {
            "$fgetc" => (ir::SysFuncId::Fgetc, 1usize, "$fgetc(fd)"),
            "$feof" => (ir::SysFuncId::Feof, 1, "$feof(fd)"),
            "$ungetc" => (ir::SysFuncId::Ungetc, 2, "$ungetc(c, fd)"),
            _ => return false,
        };
        if args.len() != arity {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("{sig} takes {arity} argument(s)"),
            );
            return true;
        }
        if delay.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("intra-assignment delay on {sig} is unsupported (v9)"),
            );
            return true;
        }
        let arg_ids: Vec<u32> = args.iter().map(|a| self.lower_expr(a)).collect();
        let rhs_id = self.push_expr(ir::Expr::SysFunc {
            which,
            args: arg_ids,
        });
        self.emit_sysread_write(b, lhs, rhs_id);
        true
    }

    /// v9 `$fgets(str, fd)` special form: reads a line, ADVANCING the fd and
    /// WRITING the str destination (arg 0, a whole-net Signal) — a statement-
    /// level effect (WRITE phase) in the `$value$plusargs` family. Legal ONLY
    /// as the direct rhs of a blocking assign; the byte count is assigned to
    /// `lhs`. Returns false when `rhs` is some other SysCall. Stays in sync with
    /// the lower_expr loud-reject guard.
    pub(crate) fn fgets_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: Option<&ast::Lvalue>,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        let ast::ExprKind::SysCall { name, args } = &rhs.kind else {
            return false;
        };
        if name.name != "$fgets" {
            return false;
        }
        if args.len() != 2 {
            self.error(MsgCode::ElabUnsupported, "$fgets takes (str, fd)");
            return true;
        }
        if delay.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "intra-assignment delay on $fgets is unsupported (v9)",
            );
            return true;
        }
        let str_id = self.lower_expr(&args[0]);
        let Some(ir::Expr::Signal { net, word: None }) = self.exprs.get(str_id as usize) else {
            self.error(
                MsgCode::ElabUnsupported,
                "$fgets target must be a plain variable (v9)",
            );
            return true;
        };
        // A2a: $fgets WRITES the dest.
        let net = *net;
        if net == POISON_NET && self.is_deferred_hier_sel_dest(str_id) {
            self.error(
                MsgCode::ElabUnsupported,
                "a $fgets target cannot be a hierarchical element select (v9) — \
                 read into a local variable",
            );
            return true;
        }
        self.deny_const_param_write(net, "$fgets into");
        let fd_id = self.lower_expr(&args[1]);
        let rhs_id = self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::Fgets,
            args: vec![str_id, fd_id],
        });
        self.emit_sysread_write(b, lhs, rhs_id);
        true
    }

    /// v9 `$fread(target, fd[, start[, count]])` special form: binary-reads into
    /// the target (arg 0 = a single reg/vector OR a WHOLE memory; an element
    /// select like `mem[i]` is loud, matching iverilog) AND advances the fd — a
    /// statement-level effect (WRITE phase) in the `$value$plusargs` family.
    /// Legal ONLY as the direct rhs of a blocking assign; the byte count is
    /// assigned to `lhs`. Stays in sync with the lower_expr loud-reject guard.
    pub(crate) fn fread_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: Option<&ast::Lvalue>,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        let ast::ExprKind::SysCall { name, args } = &rhs.kind else {
            return false;
        };
        if name.name != "$fread" {
            return false;
        }
        if !(2..=4).contains(&args.len()) {
            self.error(
                MsgCode::ElabUnsupported,
                "$fread takes (target, fd[, start[, count]])",
            );
            return true;
        }
        if delay.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "intra-assignment delay on $fread is unsupported (v9)",
            );
            return true;
        }
        // target: a WHOLE memory (array view, no trailing index) or a single
        // reg/vector. An element select (mem[i]) is loud — iverilog: "$fread's
        // first argument must be an integral variable or memory".
        let target_id = if let Some((net, lead)) = self.expr_array_view(&args[0]) {
            if !lead.is_empty() {
                self.error(
                    MsgCode::ElabUnsupported,
                    "$fread target must be a whole memory or a variable, not an element select (v9)",
                );
                return true;
            }
            // A2a: $fread WRITES the whole memory — a desugared array
            // parameter target is loud.
            self.deny_const_param_write(net, "$fread into");
            self.push_expr(ir::Expr::Signal { net, word: None })
        } else {
            let id = self.lower_expr(&args[0]);
            let Some(ir::Expr::Signal { net, word: None }) = self.exprs.get(id as usize) else {
                self.error(
                    MsgCode::ElabUnsupported,
                    "$fread target must be an integral variable or memory (v9)",
                );
                return true;
            };
            let net = *net;
            if net == POISON_NET && self.is_deferred_hier_sel_dest(id) {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a $fread target cannot be a hierarchical element select (v9) — \
                     read into a local variable",
                );
                return true;
            }
            self.deny_const_param_write(net, "$fread into");
            id
        };
        let mut sf_args = vec![target_id, self.lower_expr(&args[1])];
        // r19/S4: `$fread(mem, fd, start, count)` — args 2/3 are ADDRESSES, the same
        // shape as `$readmem*`. Gating one address-taking task family and not the
        // other left this one reading the f64 bit pattern as a start address
        // ("start argument (4607182418800017408) is outside the memory range"),
        // loading nothing at exit 0.
        if let Some(a) = args.get(2) {
            sf_args.push(self.lower_index_expr(a));
        }
        if let Some(a) = args.get(3) {
            sf_args.push(self.lower_index_expr(a));
        }
        let rhs_id = self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::Fread,
            args: sf_args,
        });
        self.emit_sysread_write(b, lhs, rhs_id);
        true
    }

    /// v9 `$fscanf(fd, fmt, args...)` / `$sscanf(str, fmt, args...)` special
    /// form: the scanf parser WRITES every matched ref arg (a whole-net Signal)
    /// AND, for `$fscanf`, advances the fd — a statement-level effect (WRITE
    /// phase) in the `$value$plusargs` family, and the FIRST multi-ref-write
    /// intercept. Legal ONLY as the direct rhs of a blocking assign; the
    /// conversion count is assigned to `lhs`. Stays in sync with the lower_expr
    /// loud-reject guard.
    pub(crate) fn scanf_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: Option<&ast::Lvalue>,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        let ast::ExprKind::SysCall { name, args } = &rhs.kind else {
            return false;
        };
        let which = match name.name.as_str() {
            "$fscanf" => ir::SysFuncId::Fscanf,
            "$sscanf" => ir::SysFuncId::Sscanf,
            _ => return false,
        };
        if args.len() < 2 {
            self.error(
                MsgCode::ElabUnsupported,
                "$fscanf/$sscanf take (source, format, args...)",
            );
            return true;
        }
        if delay.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "intra-assignment delay on $fscanf/$sscanf is unsupported (v9)",
            );
            return true;
        }
        if !matches!(args[1].kind, ast::ExprKind::StrLit { .. }) {
            self.error(
                MsgCode::ElabUnsupported,
                "$fscanf/$sscanf need a string-literal format (v9)",
            );
            return true;
        }
        let src_id = self.lower_expr(&args[0]);
        let fmt_id = self.lower_expr(&args[1]);
        let mut sf_args = vec![src_id, fmt_id];
        for a in &args[2..] {
            let id = self.lower_expr(a);
            let Some(ir::Expr::Signal { net, word: None }) = self.exprs.get(id as usize) else {
                self.error(
                    MsgCode::ElabUnsupported,
                    "$fscanf/$sscanf destination arguments must be plain variables (v9)",
                );
                return true;
            };
            // A2a: the scanf parser WRITES every dest arg.
            let net = *net;
            if net == POISON_NET && self.is_deferred_hier_sel_dest(id) {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a $fscanf/$sscanf destination cannot be a hierarchical element \
                     select (v9) — read into a local variable",
                );
                return true;
            }
            self.deny_const_param_write(net, "$fscanf/$sscanf into");
            sf_args.push(id);
        }
        let rhs_id = self.push_expr(ir::Expr::SysFunc {
            which,
            args: sf_args,
        });
        self.emit_sysread_write(b, lhs, rhs_id);
        true
    }

    /// v7 P2-C: `dest = $sformatf(fmt, args…)` special form. The format
    /// must be a string LITERAL; rendering runs kernel-side (WRITE phase,
    /// `StmtEffect::Sformatf`) and the result is a string-domain value the
    /// funnel converts per the destination (§6.16).
    pub(crate) fn sformatf_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        let ast::ExprKind::SysCall { name, args } = &rhs.kind else {
            return false;
        };
        if name.name != "$sformatf" {
            return false;
        }
        let Some(ast::ExprKind::StrLit { .. }) = args.first().map(|a| &a.kind) else {
            self.error(
                MsgCode::ElabUnsupported,
                "$sformatf needs a string-literal format (v7)",
            );
            return true;
        };
        if delay.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "intra-assignment delay on $sformatf is unsupported (v7)",
            );
            return true;
        }
        let arg_ids: Vec<u32> = args.iter().map(|a| self.lower_expr(a)).collect();
        let rhs_id = self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::Sformatf,
            args: arg_ids,
        });
        let lv = self.lower_lvalue(lhs);
        self.check_lvalue_kind(&lv, true);
        let sid = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: lv,
            rhs: rhs_id,
        });
        b.push_stmt_id(sid);
        true
    }
}
